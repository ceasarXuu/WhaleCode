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
use super::ExecCall;
use super::HostedOutputFact;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedHostedBinding {
    pub(crate) output_index: usize,
    pub(crate) provider_id: String,
    pub(crate) tool: String,
    pub(crate) outcome: rooted_dag::ActionOutcome,
    pub(crate) node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecPreflightResult {
    pub(crate) candidate_map: Option<TaskSpaceMap>,
    pub(crate) read_maps: Vec<(usize, TaskSpaceMapView)>,
    pub(crate) client_calls: Vec<PreparedClientCall>,
    pub(crate) hosted_bindings: Vec<PreparedHostedBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecPreflightError {
    RequestContext(TaskSpaceExecEnvelopeError),
    InvalidMapBoundary {
        index: usize,
        operation: &'static str,
    },
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
    LifecycleRequiresWork {
        index: usize,
        operation: &'static str,
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
    HostedCountMismatch {
        actual: usize,
        declared: usize,
    },
    HostedFactInvalid {
        output_index: usize,
        reason: &'static str,
    },
    HostedToolMismatch {
        binding_index: usize,
        actual: String,
        declared: String,
    },
    HostedNodeInvalid {
        binding_index: usize,
        node_id: String,
        reason: &'static str,
    },
}

pub(crate) fn preflight_taskspace_exec(
    envelope: &TaskSpaceExecEnvelope,
    current_map: Option<&TaskSpaceMap>,
    hosted_facts: &[HostedOutputFact],
) -> Result<TaskSpaceExecPreflightResult, TaskSpaceExecPreflightError> {
    envelope
        .request()
        .validate_current_map(current_map)
        .map_err(TaskSpaceExecPreflightError::RequestContext)?;

    let mut candidate_map = current_map.cloned();
    let mut read_maps = Vec::new();
    let mut client_calls = Vec::new();
    let mut patch_indices = Vec::new();
    let calls = &envelope.plan().calls;
    let has_hosted_work = !hosted_facts.is_empty();

    for (index, call) in calls.iter().enumerate() {
        match call {
            ExecCall::Map(operation) => {
                validate_map_boundary(index, calls, operation, has_hosted_work)?;
                if operation.is_noop_update() {
                    return Err(TaskSpaceExecPreflightError::NoEffectMapUpdate { index });
                }
                let effect = apply_map_operation(
                    candidate_map.as_ref(),
                    envelope.request().map_id(),
                    operation.clone(),
                )
                .map_err(|error| {
                    TaskSpaceExecPreflightError::MapOperationRejected { index, error }
                })?;
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
                    MapOperationEffect::Candidate(map) => candidate_map = Some(map),
                }
            }
            ExecCall::Client(call) => {
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
        }
    }
    if patch_indices.len() > 1 {
        return Err(TaskSpaceExecPreflightError::PatchLimitExceeded {
            indices: patch_indices,
        });
    }
    let hosted_bindings = validate_hosted_bindings(envelope, candidate_map.as_ref(), hosted_facts)?;
    Ok(TaskSpaceExecPreflightResult {
        candidate_map,
        read_maps,
        client_calls,
        hosted_bindings,
    })
}

fn validate_map_boundary(
    index: usize,
    calls: &[ExecCall],
    operation: &super::MapOperation,
    has_hosted_work: bool,
) -> Result<(), TaskSpaceExecPreflightError> {
    if (operation.is_initialize() || operation.is_reopen()) && index != 0 {
        return Err(TaskSpaceExecPreflightError::InvalidMapBoundary {
            index,
            operation: operation.name(),
        });
    }
    if operation.is_read() && (calls.len() != 1 || has_hosted_work) {
        return Err(TaskSpaceExecPreflightError::InvalidMapBoundary {
            index,
            operation: operation.name(),
        });
    }
    if operation.is_finish() && index + 1 != calls.len() {
        return Err(TaskSpaceExecPreflightError::InvalidMapBoundary {
            index,
            operation: operation.name(),
        });
    }
    if operation.is_initialize() || operation.is_reopen() {
        let has_later_work = calls[index + 1..]
            .iter()
            .any(|call| matches!(call, ExecCall::Client(_)));
        if !has_later_work && !has_hosted_work {
            return Err(TaskSpaceExecPreflightError::LifecycleRequiresWork {
                index,
                operation: operation.name(),
            });
        }
    } else if operation.completes_work_node() {
        let has_later_work_or_finish = calls[index + 1..].iter().any(|call| {
            matches!(call, ExecCall::Client(_))
                || matches!(call, ExecCall::Map(operation) if operation.is_finish())
        });
        if !has_later_work_or_finish && !has_hosted_work {
            return Err(TaskSpaceExecPreflightError::LifecycleRequiresWork {
                index,
                operation: operation.name(),
            });
        }
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

fn validate_hosted_bindings(
    envelope: &TaskSpaceExecEnvelope,
    candidate_map: Option<&TaskSpaceMap>,
    hosted_facts: &[HostedOutputFact],
) -> Result<Vec<PreparedHostedBinding>, TaskSpaceExecPreflightError> {
    let bindings = &envelope.plan().hosted_bindings;
    if hosted_facts.len() != bindings.len() {
        return Err(TaskSpaceExecPreflightError::HostedCountMismatch {
            actual: hosted_facts.len(),
            declared: bindings.len(),
        });
    }
    let mut facts = hosted_facts.to_vec();
    facts.sort_by_key(|fact| fact.output_index);
    let mut output_indices = BTreeSet::new();
    let mut provider_ids = BTreeSet::new();
    for fact in &facts {
        if !output_indices.insert(fact.output_index) {
            return Err(TaskSpaceExecPreflightError::HostedFactInvalid {
                output_index: fact.output_index,
                reason: "duplicate_output_index",
            });
        }
        if fact.provider_id.trim().is_empty() || !provider_ids.insert(fact.provider_id.as_str()) {
            return Err(TaskSpaceExecPreflightError::HostedFactInvalid {
                output_index: fact.output_index,
                reason: "missing_or_duplicate_provider_id",
            });
        }
    }

    facts
        .into_iter()
        .zip(bindings)
        .enumerate()
        .map(|(binding_index, (fact, binding))| {
            if fact.tool != binding.tool {
                return Err(TaskSpaceExecPreflightError::HostedToolMismatch {
                    binding_index,
                    actual: fact.tool,
                    declared: binding.tool.clone(),
                });
            }
            let mut node_ids = BTreeSet::new();
            for node_id in &binding.node_ids {
                if node_id.trim().is_empty() || !node_ids.insert(node_id.as_str()) {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        binding_index,
                        node_id: node_id.clone(),
                        reason: "empty_or_duplicate_node",
                    });
                }
                if candidate_map.is_none_or(|map| rooted_dag::node(map, node_id).is_none()) {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        binding_index,
                        node_id: node_id.clone(),
                        reason: "unknown_node",
                    });
                }
                if candidate_map.and_then(|map| rooted_dag::node_role(map, node_id))
                    != Some(NodeRole::Work)
                {
                    return Err(TaskSpaceExecPreflightError::HostedNodeInvalid {
                        binding_index,
                        node_id: node_id.clone(),
                        reason: "boundary_node",
                    });
                }
            }
            Ok(PreparedHostedBinding {
                output_index: fact.output_index,
                provider_id: fact.provider_id,
                tool: fact.tool,
                outcome: fact.outcome,
                node_ids: binding.node_ids.clone(),
            })
        })
        .collect()
}
