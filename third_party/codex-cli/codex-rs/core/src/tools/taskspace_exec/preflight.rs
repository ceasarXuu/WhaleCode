use std::collections::BTreeSet;

use codex_tools::ToolSpecCapabilityInput;

use crate::action_map::TaskSpaceMapView;
use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::NodeRole;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::TaskSpaceMap;
use crate::action_map::taskspace_map_view;

use super::ClientCall;
use super::ClientCallInput;
use super::MapOperationApplyError;
use super::MapOperationEffect;
use super::TaskSpaceExecEnvelope;
use super::TaskSpaceExecEnvelopeError;
use super::TaskSpaceExecInternalCallId;
use super::apply_map_operation;
use super::schema_validation::validate_json_schema;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PreparedClientCall {
    pub(crate) identity: TaskSpaceExecInternalCallId,
    pub(crate) call: ClientCall,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecPreflightResult {
    pub(crate) candidate_map: Option<TaskSpaceMap>,
    pub(crate) read_maps: Vec<(usize, TaskSpaceMapView)>,
    pub(crate) client_calls: Vec<PreparedClientCall>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecPreflightError {
    RequestContext(TaskSpaceExecEnvelopeError),
    NoEffectMapUpdate {
        index: usize,
    },
    MapOperationRejected {
        index: usize,
        error: MapOperationApplyError,
    },
    ReadMapViewFailed {
        index: usize,
        reason: String,
    },
    ToolActivationRejected {
        error: rooted_dag::Rejection,
    },
    ClientNodeMissing {
        index: usize,
        node_id: String,
    },
    ClientNodeNotExecutable {
        index: usize,
        node_id: String,
        state: NodeState,
        incomplete_parent_ids: Vec<String>,
    },
    ClientNodeNotWork {
        index: usize,
        node_id: String,
        role: NodeRole,
    },
    ClientArgumentsInvalid {
        index: usize,
        path: String,
        reason: String,
    },
    ClientCapabilityMismatch {
        index: usize,
        tool: String,
    },
    PatchLimitExceeded {
        indices: Vec<usize>,
    },
    ResponseWorkMissing {
        sequence_type: String,
    },
}

pub(crate) fn preflight_taskspace_exec(
    envelope: &TaskSpaceExecEnvelope,
    current_map: Option<&TaskSpaceMap>,
    has_provider_work: bool,
) -> Result<TaskSpaceExecPreflightResult, TaskSpaceExecPreflightError> {
    envelope
        .request()
        .validate_current_map(current_map)
        .map_err(TaskSpaceExecPreflightError::RequestContext)?;
    if sequence_requires_response_work(&envelope.plan().sequence_type)
        && envelope.plan().tools.is_empty()
        && !has_provider_work
    {
        return Err(TaskSpaceExecPreflightError::ResponseWorkMissing {
            sequence_type: envelope.plan().sequence_type.clone(),
        });
    }

    let mut candidate_map = current_map.cloned();
    let mut read_maps = Vec::new();
    let mut client_calls = Vec::new();
    let mut patch_indices = Vec::new();
    for (index, operation) in envelope.plan().pre_map.iter().enumerate() {
        apply_map_stage(
            index,
            operation,
            envelope,
            &mut candidate_map,
            &mut read_maps,
        )?;
    }

    for (index, call) in envelope.plan().tools.iter().enumerate() {
        validate_client_call(index, call, candidate_map.as_ref(), envelope)?;
        if is_apply_patch(call) {
            patch_indices.push(index);
        }
        client_calls.push(PreparedClientCall {
            identity: envelope
                .internal_call_id(index)
                .map_err(TaskSpaceExecPreflightError::RequestContext)?,
            call: call.clone(),
        });
    }
    if patch_indices.len() > 1 {
        return Err(TaskSpaceExecPreflightError::PatchLimitExceeded {
            indices: patch_indices,
        });
    }
    candidate_map = activate_ready_tool_nodes(candidate_map, &client_calls)?;

    if let Some(operation) = envelope.plan().terminal_map.as_ref() {
        apply_map_stage(
            envelope.plan().pre_map.len(),
            operation,
            envelope,
            &mut candidate_map,
            &mut read_maps,
        )?;
    }
    Ok(TaskSpaceExecPreflightResult {
        candidate_map,
        read_maps,
        client_calls,
    })
}

fn sequence_requires_response_work(sequence_type: &str) -> bool {
    matches!(
        sequence_type,
        "initialize_and_work" | "work" | "update_and_work" | "reopen_update_and_work"
    )
}

fn apply_map_stage(
    index: usize,
    operation: &super::MapOperation,
    envelope: &TaskSpaceExecEnvelope,
    candidate_map: &mut Option<TaskSpaceMap>,
    read_maps: &mut Vec<(usize, TaskSpaceMapView)>,
) -> Result<(), TaskSpaceExecPreflightError> {
    if operation.is_noop_update() {
        return Err(TaskSpaceExecPreflightError::NoEffectMapUpdate { index });
    }
    let effect = apply_map_operation(
        candidate_map.as_ref(),
        envelope.request().map_id(),
        operation.clone(),
    )
    .map_err(|error| TaskSpaceExecPreflightError::MapOperationRejected { index, error })?;
    match effect {
        MapOperationEffect::Read(map) => {
            let view = taskspace_map_view(&map).map_err(|error| {
                TaskSpaceExecPreflightError::ReadMapViewFailed {
                    index,
                    reason: error.to_string(),
                }
            })?;
            read_maps.push((index, view));
        }
        MapOperationEffect::Candidate(map) => *candidate_map = Some(map),
    }
    Ok(())
}

fn validate_client_call(
    index: usize,
    call: &ClientCall,
    candidate_map: Option<&TaskSpaceMap>,
    envelope: &TaskSpaceExecEnvelope,
) -> Result<(), TaskSpaceExecPreflightError> {
    let map = candidate_map.ok_or_else(|| TaskSpaceExecPreflightError::ClientNodeMissing {
        index,
        node_id: call.node_id.clone(),
    })?;
    let node = rooted_dag::node(map, &call.node_id).ok_or_else(|| {
        TaskSpaceExecPreflightError::ClientNodeMissing {
            index,
            node_id: call.node_id.clone(),
        }
    })?;
    let role = rooted_dag::node_role(map, &call.node_id).ok_or_else(|| {
        TaskSpaceExecPreflightError::ClientNodeMissing {
            index,
            node_id: call.node_id.clone(),
        }
    })?;
    if role != NodeRole::Work {
        return Err(TaskSpaceExecPreflightError::ClientNodeNotWork {
            index,
            node_id: node.node_id.clone(),
            role,
        });
    }
    if !matches!(node.state, NodeState::Ready | NodeState::InFlight) {
        let incomplete_parent_ids = node
            .parents
            .iter()
            .filter(|parent_id| {
                *parent_id != &map.root.node_id
                    && rooted_dag::node(map, parent_id)
                        .is_none_or(|parent| parent.state != NodeState::Completed)
            })
            .cloned()
            .collect();
        return Err(TaskSpaceExecPreflightError::ClientNodeNotExecutable {
            index,
            node_id: node.node_id.clone(),
            state: node.state,
            incomplete_parent_ids,
        });
    }
    let capability = envelope
        .request()
        .catalog()
        .client_capability(&call.tool_name)
        .ok_or_else(|| TaskSpaceExecPreflightError::ClientCapabilityMismatch {
            index,
            tool: call.display_name.clone(),
        })?;
    match (&call.input, &capability.capability.input) {
        (ClientCallInput::Function(value), ToolSpecCapabilityInput::Function(schema)) => {
            validate_json_schema(value, schema).map_err(|violation| {
                TaskSpaceExecPreflightError::ClientArgumentsInvalid {
                    index,
                    path: violation.path,
                    reason: violation.reason,
                }
            })
        }
        (ClientCallInput::Freeform(_), ToolSpecCapabilityInput::Freeform(_)) => Ok(()),
        _ => Err(TaskSpaceExecPreflightError::ClientCapabilityMismatch {
            index,
            tool: call.display_name.clone(),
        }),
    }
}

fn is_apply_patch(call: &ClientCall) -> bool {
    call.tool_name.namespace.is_none() && call.tool_name.name == "apply_patch"
}

fn activate_ready_tool_nodes(
    candidate_map: Option<TaskSpaceMap>,
    clients: &[PreparedClientCall],
) -> Result<Option<TaskSpaceMap>, TaskSpaceExecPreflightError> {
    let Some(map) = candidate_map else {
        return Ok(None);
    };
    let owner_ids = clients
        .iter()
        .map(|client| client.call.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let patches = owner_ids
        .into_iter()
        .filter(|node_id| {
            rooted_dag::node(&map, node_id).is_some_and(|node| node.state == NodeState::Ready)
        })
        .map(|node_id| rooted_dag::NodePatch {
            node_id: node_id.to_string(),
            state: Some(NodeState::InFlight),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if patches.is_empty() {
        return Ok(Some(map));
    }
    rooted_dag::execute(
        &map,
        rooted_dag::ExecuteTransaction {
            request_revision: map.revision,
            add_work_nodes: Vec::new(),
            patches,
        },
    )
    .map(|commit| Some(commit.map))
    .map_err(|error| TaskSpaceExecPreflightError::ToolActivationRejected { error })
}
