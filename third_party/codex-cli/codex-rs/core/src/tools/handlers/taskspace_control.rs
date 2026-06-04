use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::action_map::ActionMapCognitiveClaimInput;
use crate::action_map::ActionMapEvidenceRefInput;
use crate::action_map::ActionMapNextNodeDraft;
use crate::action_map::NodeKind;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;

pub struct TaskSpaceControlHandler;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum TaskSpaceControlArgs {
    StartTask {
        task_title: String,
        #[serde(default)]
        task_objective: String,
        #[serde(default)]
        node_kind: String,
        node_title: String,
        node_context_summary: String,
        #[serde(default)]
        bind_current: bool,
    },
    RouteTask {
        task_id: String,
    },
    CreateNode {
        kind: String,
        title: String,
        context_summary: String,
        #[serde(default)]
        dependency_node_ids: Vec<String>,
        #[serde(default)]
        bind_current: bool,
    },
    BindNode {
        node_id: String,
    },
    FinishNode {
        node_id: String,
        result_summary: String,
        #[serde(default)]
        next_node_id: Option<String>,
        #[serde(default)]
        next_node_kind: Option<String>,
        #[serde(default)]
        next_node_title: Option<String>,
        #[serde(default)]
        next_node_context_summary: Option<String>,
        #[serde(default)]
        next_dependency_node_ids: Vec<String>,
    },
    BlockNode {
        node_id: String,
        blocker_summary: String,
    },
    RecordOutputContract {
        output_contract_id: String,
        kind: String,
        description: String,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    RecordFactSource {
        fact_source_id: String,
        provenance: String,
        description: String,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    RecordFact {
        claim_id: String,
        statement: String,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    MarkResultValidity {
        result_id: String,
        validity: String,
        validity_reason: String,
        #[serde(default)]
        claims: Vec<TaskSpaceCognitiveClaimArgs>,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
        #[serde(default)]
        changed_artifacts: Vec<String>,
        #[serde(default)]
        validator_refs: Vec<String>,
        #[serde(default)]
        remaining_uncertainty: Vec<String>,
    },
}

#[derive(Debug, Default, Deserialize)]
struct TaskSpaceEvidenceRefArgs {
    #[serde(default)]
    result_id: Option<String>,
    #[serde(default)]
    claim_id: Option<String>,
    #[serde(default)]
    fact_source_id: Option<String>,
    #[serde(default)]
    trace_event_id: Option<String>,
    #[serde(default)]
    artifact_ref: Option<String>,
    #[serde(default)]
    validator_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceCognitiveClaimArgs {
    claim_id: String,
    statement: String,
    #[serde(default)]
    evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
}

pub struct TaskSpaceControlOutput {
    message: String,
}

impl ToolOutput for TaskSpaceControlOutput {
    fn log_preview(&self) -> String {
        self.message.clone()
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, _payload: &ToolPayload) -> ResponseInputItem {
        let mut output = FunctionCallOutputPayload::from_text(self.message.clone());
        output.success = Some(true);
        ResponseInputItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output,
        }
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        JsonValue::String(self.message.clone())
    }
}

impl ToolHandler for TaskSpaceControlHandler {
    type Output = TaskSpaceControlOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "taskspace_control handler received unsupported payload".to_string(),
                ));
            }
        };
        let args: TaskSpaceControlArgs = parse_arguments(&arguments)?;
        let message = match args {
            TaskSpaceControlArgs::StartTask {
                task_title,
                task_objective,
                node_kind,
                node_title,
                node_context_summary,
                bind_current,
            } => {
                let node_kind = parse_node_kind("node_kind", &node_kind)?;
                let (task_id, map_id, node_id) = session
                    .start_action_map_task_for_main_with_kind(
                        &turn,
                        node_kind,
                        task_title,
                        task_objective,
                        node_title,
                        node_context_summary,
                        bind_current,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                if bind_current {
                    format!(
                        "TaskSpace task started and bound: task={task_id} map={map_id} node={node_id}"
                    )
                } else {
                    format!("TaskSpace task started: task={task_id} map={map_id} node={node_id}")
                }
            }
            TaskSpaceControlArgs::RouteTask { task_id } => {
                session
                    .route_action_map_task_for_main(&turn, &task_id)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace task routed: {task_id}")
            }
            TaskSpaceControlArgs::CreateNode {
                kind,
                title,
                context_summary,
                dependency_node_ids,
                bind_current,
            } => {
                let kind = parse_node_kind("kind", &kind)?;
                let node_id = session
                    .create_action_map_node_for_main_with_kind(
                        &turn,
                        kind,
                        title,
                        context_summary,
                        dependency_node_ids,
                        bind_current,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                if bind_current {
                    format!("TaskSpace node created and bound: {node_id}")
                } else {
                    format!("TaskSpace node created: {node_id}")
                }
            }
            TaskSpaceControlArgs::BindNode { node_id } => {
                session
                    .bind_action_map_main_node(&turn, &node_id)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace main node bound: {node_id}")
            }
            TaskSpaceControlArgs::FinishNode {
                node_id,
                result_summary,
                next_node_id,
                next_node_kind,
                next_node_title,
                next_node_context_summary,
                next_dependency_node_ids,
            } => {
                let next_node_draft = build_next_node_draft(
                    next_node_kind,
                    next_node_title,
                    next_node_context_summary,
                    next_dependency_node_ids,
                )?;
                let outcome = session
                    .finish_action_map_main_node_with_next(
                        &turn,
                        &node_id,
                        result_summary,
                        next_node_id,
                        next_node_draft,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                if let Some(next_node_id) = outcome.next_node_id {
                    format!(
                        "TaskSpace node finished: {node_id} result {}. Next node created and bound: {next_node_id}",
                        outcome.result_id
                    )
                } else {
                    format!(
                        "TaskSpace node finished: {node_id} result {}",
                        outcome.result_id
                    )
                }
            }
            TaskSpaceControlArgs::BlockNode {
                node_id,
                blocker_summary,
            } => {
                let result_id = session
                    .block_action_map_main_node(&turn, &node_id, blocker_summary)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace node blocked: {node_id} result {result_id}")
            }
            TaskSpaceControlArgs::RecordOutputContract {
                output_contract_id,
                kind,
                description,
                evidence_refs,
            } => {
                session
                    .record_action_map_output_contract(
                        &turn,
                        &output_contract_id,
                        &kind,
                        description,
                        convert_evidence_refs(evidence_refs),
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace output contract recorded: {output_contract_id}")
            }
            TaskSpaceControlArgs::RecordFactSource {
                fact_source_id,
                provenance,
                description,
                evidence_refs,
            } => {
                session
                    .record_action_map_fact_source(
                        &turn,
                        &fact_source_id,
                        &provenance,
                        description,
                        convert_evidence_refs(evidence_refs),
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace fact source recorded: {fact_source_id}")
            }
            TaskSpaceControlArgs::RecordFact {
                claim_id,
                statement,
                evidence_refs,
            } => {
                session
                    .record_action_map_fact(
                        &turn,
                        &claim_id,
                        statement,
                        convert_evidence_refs(evidence_refs),
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace fact recorded: {claim_id}")
            }
            TaskSpaceControlArgs::MarkResultValidity {
                result_id,
                validity,
                validity_reason,
                claims,
                evidence_refs,
                changed_artifacts,
                validator_refs,
                remaining_uncertainty,
            } => {
                session
                    .mark_action_map_result_validity(
                        &turn,
                        &result_id,
                        &validity,
                        validity_reason,
                        convert_claims(claims),
                        convert_evidence_refs(evidence_refs),
                        changed_artifacts,
                        validator_refs,
                        remaining_uncertainty,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace result validity recorded: {result_id} validity={validity}")
            }
        };
        Ok(TaskSpaceControlOutput { message })
    }
}

fn parse_node_kind(field: &str, value: &str) -> Result<NodeKind, FunctionCallError> {
    NodeKind::from_str(value).ok_or_else(|| {
        FunctionCallError::RespondToModel(format!(
            "taskspace_control {field} must be one of: inspect_code_context, implement_solution, smoke_test, regression_test, final_synthesis."
        ))
    })
}

fn build_next_node_draft(
    next_node_kind: Option<String>,
    next_node_title: Option<String>,
    next_node_context_summary: Option<String>,
    next_dependency_node_ids: Vec<String>,
) -> Result<Option<ActionMapNextNodeDraft>, FunctionCallError> {
    let has_any = next_node_kind
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || next_node_title
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || next_node_context_summary
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || !next_dependency_node_ids.is_empty();
    if !has_any {
        return Ok(None);
    }

    let kind = parse_node_kind("next_node_kind", next_node_kind.as_deref().unwrap_or(""))?;
    let title = next_node_title.unwrap_or_default();
    let context_summary = next_node_context_summary.unwrap_or_default();
    if title.trim().is_empty() || context_summary.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "taskspace_control finish_node next node draft requires next_node_kind, next_node_title, and next_node_context_summary."
                .to_string(),
        ));
    }
    Ok(Some(ActionMapNextNodeDraft {
        kind,
        title,
        context_summary,
        dependency_node_ids: next_dependency_node_ids,
    }))
}

fn convert_evidence_refs(inputs: Vec<TaskSpaceEvidenceRefArgs>) -> Vec<ActionMapEvidenceRefInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapEvidenceRefInput {
            result_id: input.result_id,
            claim_id: input.claim_id,
            fact_source_id: input.fact_source_id,
            trace_event_id: input.trace_event_id,
            artifact_ref: input.artifact_ref,
            validator_ref: input.validator_ref,
        })
        .collect()
}

fn convert_claims(inputs: Vec<TaskSpaceCognitiveClaimArgs>) -> Vec<ActionMapCognitiveClaimInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapCognitiveClaimInput {
            id: input.claim_id,
            statement: input.statement,
            evidence_refs: convert_evidence_refs(input.evidence_refs),
        })
        .collect()
}
