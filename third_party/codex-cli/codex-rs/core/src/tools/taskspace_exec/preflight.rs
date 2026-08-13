use std::collections::BTreeMap;
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
use super::HostedToolFact;
use super::MapOperationApplyError;
use super::MapOperationEffect;
use super::TaskSpaceExecEnvelope;
use super::TaskSpaceExecEnvelopeError;
use super::TaskSpaceExecInternalCallId;
use super::ToolAction;
use super::apply_map_operation;
use super::schema_validation::validate_json_schema;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PreparedClientCall {
    pub(crate) identity: TaskSpaceExecInternalCallId,
    pub(crate) call: ClientCall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedProviderAction {
    pub(crate) tool_index: usize,
    pub(crate) identity: TaskSpaceExecInternalCallId,
    pub(crate) tool: String,
    pub(crate) outcome: rooted_dag::ActionOutcome,
    pub(crate) node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecPreflightResult {
    pub(crate) candidate_map: Option<TaskSpaceMap>,
    pub(crate) read_maps: Vec<(usize, TaskSpaceMapView)>,
    pub(crate) client_calls: Vec<PreparedClientCall>,
    pub(crate) provider_actions: Vec<PreparedProviderAction>,
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
    HostedToolSetMismatch {
        actual: Vec<String>,
        declared: Vec<String>,
    },
    HostedToolDuplicate {
        tool_index: usize,
        tool: String,
    },
    HostedFactDuplicate {
        tool: String,
    },
    HostedNodeInvalid {
        tool_index: usize,
        node_id: String,
        reason: &'static str,
    },
}

pub(crate) fn preflight_taskspace_exec(
    envelope: &TaskSpaceExecEnvelope,
    current_map: Option<&TaskSpaceMap>,
    hosted_facts: &[HostedToolFact],
) -> Result<TaskSpaceExecPreflightResult, TaskSpaceExecPreflightError> {
    envelope
        .request()
        .validate_current_map(current_map)
        .map_err(TaskSpaceExecPreflightError::RequestContext)?;

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

    let provider_actions = envelope
        .plan()
        .tools
        .iter()
        .enumerate()
        .filter_map(|(index, action)| match action {
            ToolAction::Hosted(action) => Some((index, action.clone())),
            ToolAction::Client(_) => None,
        })
        .collect::<Vec<_>>();
    let hosted_facts_by_tool = match_provider_actions(&provider_actions, hosted_facts)?;

    for (index, action) in envelope.plan().tools.iter().enumerate() {
        match action {
            ToolAction::Client(call) => {
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
            ToolAction::Hosted(_) => {}
        }
    }
    if patch_indices.len() > 1 {
        return Err(TaskSpaceExecPreflightError::PatchLimitExceeded {
            indices: patch_indices,
        });
    }
    let provider_actions = reconcile_provider_actions(
        &provider_actions,
        candidate_map.as_ref(),
        &hosted_facts_by_tool,
        envelope,
    )?;
    candidate_map = activate_ready_tool_nodes(candidate_map, &client_calls, &provider_actions)?;

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
        provider_actions,
    })
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

fn match_provider_actions<'a>(
    actions: &[(usize, super::plan::ProviderAction)],
    hosted_facts: &'a [HostedToolFact],
) -> Result<BTreeMap<&'a str, &'a HostedToolFact>, TaskSpaceExecPreflightError> {
    let mut facts_by_tool = BTreeMap::new();
    for fact in hosted_facts {
        if facts_by_tool.insert(fact.tool.as_str(), fact).is_some() {
            return Err(TaskSpaceExecPreflightError::HostedFactDuplicate {
                tool: fact.tool.clone(),
            });
        }
    }
    let mut actions_by_tool = BTreeMap::new();
    for (tool_index, action) in actions {
        if actions_by_tool
            .insert(action.tool.as_str(), (*tool_index, action))
            .is_some()
        {
            return Err(TaskSpaceExecPreflightError::HostedToolDuplicate {
                tool_index: *tool_index,
                tool: action.tool.clone(),
            });
        }
    }
    let actual = facts_by_tool
        .keys()
        .map(|tool| (*tool).to_string())
        .collect::<Vec<_>>();
    let declared = actions_by_tool
        .keys()
        .map(|tool| (*tool).to_string())
        .collect::<Vec<_>>();
    if actual != declared {
        return Err(TaskSpaceExecPreflightError::HostedToolSetMismatch { actual, declared });
    }

    Ok(facts_by_tool)
}

fn reconcile_provider_actions(
    actions: &[(usize, super::plan::ProviderAction)],
    candidate_map: Option<&TaskSpaceMap>,
    facts_by_tool: &BTreeMap<&str, &HostedToolFact>,
    envelope: &TaskSpaceExecEnvelope,
) -> Result<Vec<PreparedProviderAction>, TaskSpaceExecPreflightError> {
    actions
        .iter()
        .map(|(tool_index, action)| {
            let fact = facts_by_tool
                .get(action.tool.as_str())
                .expect("Hosted Tool sets were checked above");
            let mut node_ids = BTreeSet::new();
            for node_id in &action.node_ids {
                if node_id.trim().is_empty() || !node_ids.insert(node_id.as_str()) {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        tool_index: *tool_index,
                        node_id: node_id.clone(),
                        reason: "empty_or_duplicate_node",
                    });
                }
                if candidate_map.is_none_or(|map| rooted_dag::node(map, node_id).is_none()) {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        tool_index: *tool_index,
                        node_id: node_id.clone(),
                        reason: "unknown_node",
                    });
                }
                if candidate_map.and_then(|map| rooted_dag::node_role(map, node_id))
                    != Some(NodeRole::Work)
                {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        tool_index: *tool_index,
                        node_id: node_id.clone(),
                        reason: "boundary_node",
                    });
                }
                if candidate_map
                    .and_then(|map| rooted_dag::node(map, node_id))
                    .is_none_or(|node| {
                        !matches!(node.state, NodeState::Ready | NodeState::InFlight)
                    })
                {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        tool_index: *tool_index,
                        node_id: node_id.clone(),
                        reason: "non_executable_state",
                    });
                }
            }
            Ok(PreparedProviderAction {
                tool_index: *tool_index,
                identity: envelope
                    .internal_call_id(*tool_index)
                    .map_err(TaskSpaceExecPreflightError::RequestContext)?,
                tool: fact.tool.clone(),
                outcome: fact.outcome,
                node_ids: action.node_ids.clone(),
            })
        })
        .collect()
}

fn activate_ready_tool_nodes(
    candidate_map: Option<TaskSpaceMap>,
    clients: &[PreparedClientCall],
    hosted: &[PreparedProviderAction],
) -> Result<Option<TaskSpaceMap>, TaskSpaceExecPreflightError> {
    let Some(map) = candidate_map else {
        return Ok(None);
    };
    let owner_ids = clients
        .iter()
        .map(|client| client.call.node_id.as_str())
        .chain(
            hosted
                .iter()
                .flat_map(|binding| binding.node_ids.iter().map(String::as_str)),
        )
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
