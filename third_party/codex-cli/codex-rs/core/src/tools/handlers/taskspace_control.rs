use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::action_map::ActionMapCognitiveClaimInput;
use crate::action_map::ActionMapEvidenceRefInput;
use crate::action_map::ActionMapLedgerDecisionInput;
use crate::action_map::ActionMapNextNodeDraft;
use crate::action_map::ActionMapResultAdoptionInput;
use crate::action_map::ActionMapStateCommitBlockerInput;
use crate::action_map::ActionMapStateCommitFactSourceInput;
use crate::action_map::ActionMapStateCommitFinishNodeInput;
use crate::action_map::ActionMapStateCommitInput;
use crate::action_map::ActionMapStateCommitNextBestActionInput;
use crate::action_map::ActionMapStateCommitNodeInput;
use crate::action_map::ActionMapStateCommitOutputContractInput;
use crate::action_map::ActionMapStateCommitResultValidityInput;
use crate::action_map::ActionMapSubagentPlanInput;
use crate::action_map::ActionMapSuccessCriterionInput;
use crate::action_map::NodeKind;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::output_reference::OUTPUT_SLICE_MAX_BYTES;
use crate::tools::output_reference::OutputSliceMode;
use crate::tools::output_reference::OutputSliceRequest;
use crate::tools::output_reference::read_output_artifact_slice;
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
        #[serde(default)]
        initial_success_criteria: Vec<TaskSpaceSuccessCriterionArgs>,
        #[serde(default)]
        initial_output_contracts: Vec<TaskSpaceOutputContractArgs>,
        #[serde(default)]
        initial_fact_sources: Vec<TaskSpaceFactSourceArgs>,
        node_title: String,
        node_context_summary: String,
        #[serde(default)]
        bind_current: Option<bool>,
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
        #[serde(alias = "kind")]
        output_contract_kind: String,
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
    RecordSuccessCriteria {
        criteria: Vec<TaskSpaceSuccessCriterionArgs>,
    },
    RecordOpenQuestion {
        question_id: String,
        question: String,
        reason: String,
        #[serde(default)]
        blocking: bool,
        #[serde(default)]
        opened_by_node_id: Option<String>,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    CloseOpenQuestion {
        question_id: String,
        resolution: String,
        #[serde(default)]
        closed_by_result_id: Option<String>,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    RecordDecision {
        decision_id: String,
        decision_kind: String,
        decision: String,
        rationale: String,
        #[serde(default)]
        depends_on_results: Vec<String>,
        #[serde(default)]
        depends_on_facts: Vec<String>,
        #[serde(default)]
        resolves_questions: Vec<String>,
        #[serde(default)]
        supports_criteria: Vec<String>,
        #[serde(default)]
        risks: Vec<String>,
    },
    RecordNextBestAction {
        #[serde(default)]
        node_id: Option<String>,
        action_summary: String,
        reason: String,
        #[serde(default)]
        expected_artifact: Option<String>,
        #[serde(default)]
        blocked_by: Vec<String>,
    },
    RecordSubagentPlan {
        parent_node_id: String,
        why_parallelizable: String,
        expected_artifact: String,
        acceptance_check: String,
        max_scope: String,
        #[serde(default)]
        supports_questions: Vec<String>,
        #[serde(default)]
        tests_hypotheses: Vec<String>,
        #[serde(default)]
        depends_on_results: Vec<String>,
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
    AdoptResult {
        result_id: String,
        #[serde(default)]
        adopted_by_facts: Vec<String>,
        #[serde(default)]
        adopted_by_hypotheses: Vec<String>,
        #[serde(default)]
        adopted_by_decisions: Vec<String>,
        #[serde(default)]
        adopted_by_criteria: Vec<String>,
        #[serde(default)]
        adopted_by_nodes: Vec<String>,
    },
    ReadOutputRef {
        output_ref: String,
        mode: String,
        #[serde(default)]
        start_line: Option<usize>,
        #[serde(default)]
        end_line: Option<usize>,
        #[serde(default)]
        pattern: Option<String>,
        #[serde(default)]
        max_bytes: Option<usize>,
    },
    StateCommit {
        #[serde(default)]
        commit_id: Option<String>,
        #[serde(default)]
        schema_version: Option<String>,
        #[serde(default)]
        dry_run: bool,
        #[serde(default)]
        active_node_id: Option<String>,
        #[serde(default)]
        nodes: Vec<TaskSpaceStateCommitNodeArgs>,
        #[serde(default)]
        finished_nodes: Vec<TaskSpaceStateCommitFinishNodeArgs>,
        #[serde(default)]
        blockers: Vec<TaskSpaceStateCommitBlockerArgs>,
        #[serde(default)]
        result_validities: Vec<TaskSpaceStateCommitResultValidityArgs>,
        #[serde(default)]
        result_adoptions: Vec<TaskSpaceStateCommitResultAdoptionArgs>,
        #[serde(default)]
        success_criteria: Vec<TaskSpaceSuccessCriterionArgs>,
        #[serde(default)]
        output_contracts: Vec<TaskSpaceOutputContractArgs>,
        #[serde(default)]
        fact_sources: Vec<TaskSpaceFactSourceArgs>,
        #[serde(default)]
        facts: Vec<TaskSpaceCognitiveClaimArgs>,
        #[serde(default)]
        decisions: Vec<TaskSpaceDecisionArgs>,
        #[serde(default)]
        next_best_action: Option<TaskSpaceNextBestActionArgs>,
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

#[derive(Debug, Deserialize)]
struct TaskSpaceSuccessCriterionArgs {
    #[serde(alias = "criterion_id")]
    id: String,
    kind: String,
    description: String,
    #[serde(default = "default_success_criterion_status")]
    status: String,
    #[serde(default)]
    evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceFactSourceArgs {
    #[serde(alias = "fact_source_id")]
    id: String,
    provenance: String,
    description: String,
    #[serde(default)]
    evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceOutputContractArgs {
    #[serde(alias = "output_contract_id")]
    id: String,
    #[serde(alias = "output_contract_kind", alias = "kind")]
    kind: String,
    description: String,
    #[serde(default)]
    evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceDecisionArgs {
    #[serde(alias = "decision_id")]
    id: String,
    decision_kind: String,
    decision: String,
    rationale: String,
    #[serde(default)]
    depends_on_results: Vec<String>,
    #[serde(default)]
    depends_on_facts: Vec<String>,
    #[serde(default)]
    resolves_questions: Vec<String>,
    #[serde(default)]
    supports_criteria: Vec<String>,
    #[serde(default)]
    risks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceNextBestActionArgs {
    #[serde(default)]
    node_id: Option<String>,
    action_summary: String,
    reason: String,
    #[serde(default)]
    expected_artifact: Option<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceStateCommitNodeArgs {
    kind: String,
    title: String,
    context_summary: String,
    #[serde(default)]
    dependency_node_ids: Vec<String>,
    #[serde(default)]
    bind_current: bool,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceStateCommitFinishNodeArgs {
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
}

#[derive(Debug, Deserialize)]
struct TaskSpaceStateCommitBlockerArgs {
    node_id: String,
    blocker_summary: String,
}

#[derive(Debug, Deserialize)]
struct TaskSpaceStateCommitResultValidityArgs {
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
}

#[derive(Debug, Deserialize)]
struct TaskSpaceStateCommitResultAdoptionArgs {
    result_id: String,
    #[serde(default)]
    adopted_by_facts: Vec<String>,
    #[serde(default)]
    adopted_by_hypotheses: Vec<String>,
    #[serde(default)]
    adopted_by_decisions: Vec<String>,
    #[serde(default)]
    adopted_by_criteria: Vec<String>,
    #[serde(default)]
    adopted_by_nodes: Vec<String>,
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
        let normalized_arguments = normalize_taskspace_arguments(&arguments)?;
        let args: TaskSpaceControlArgs = parse_arguments(&normalized_arguments)?;
        if let Some(action) = legacy_state_action_name(&args) {
            let _ = session
                .record_action_map_legacy_state_action_attempt(
                    &turn,
                    action,
                    true,
                    false,
                    "active_profile_requires_state_commit",
                )
                .await;
            return Err(legacy_state_action_rejection(action));
        }
        let message = match args {
            TaskSpaceControlArgs::StartTask {
                task_title,
                task_objective,
                node_kind,
                initial_success_criteria,
                initial_output_contracts,
                initial_fact_sources,
                node_title,
                node_context_summary,
                bind_current,
            } => {
                let bind_current = bind_current.unwrap_or(true);
                let node_kind = parse_node_kind("node_kind", &node_kind)?;
                let (task_id, map_id, node_id) = session
                    .start_action_map_task_for_main_with_kind_and_criteria(
                        &turn,
                        node_kind,
                        task_title,
                        task_objective,
                        convert_success_criteria(initial_success_criteria),
                        convert_state_commit_output_contracts(initial_output_contracts),
                        convert_fact_sources(initial_fact_sources),
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
                let auto_accept_handoff = matches!(
                    next_node_kind.as_deref(),
                    Some("implement_solution") | Some("smoke_test") | Some("regression_test")
                );
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
                if auto_accept_handoff {
                    let evidence_ref = ActionMapEvidenceRefInput {
                        result_id: Some(outcome.result_id.clone()),
                        ..ActionMapEvidenceRefInput::default()
                    };
                    session
                        .mark_action_map_result_validity(
                            &turn,
                            &outcome.result_id,
                            "accepted",
                            "main-path node handoff accepted by taskspace_control".to_string(),
                            vec![ActionMapCognitiveClaimInput {
                                id: format!("claim-{}-handoff", outcome.result_id),
                                statement:
                                    "Main-path node result is sufficient to continue into the next task phase."
                                        .to_string(),
                                evidence_refs: vec![evidence_ref.clone()],
                            }],
                            vec![evidence_ref],
                            Vec::new(),
                            Vec::new(),
                            Vec::new(),
                        )
                        .await
                        .map_err(FunctionCallError::RespondToModel)?;
                }
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
                output_contract_kind,
                description,
                evidence_refs,
            } => {
                session
                    .record_action_map_output_contract(
                        &turn,
                        &output_contract_id,
                        &output_contract_kind,
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
            TaskSpaceControlArgs::RecordSuccessCriteria { criteria } => {
                let count = criteria.len();
                session
                    .record_action_map_success_criteria(&turn, convert_success_criteria(criteria))
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace success criteria recorded: {count}")
            }
            TaskSpaceControlArgs::RecordOpenQuestion {
                question_id,
                question,
                reason,
                blocking,
                opened_by_node_id,
                evidence_refs,
            } => {
                session
                    .record_action_map_open_question(
                        &turn,
                        &question_id,
                        question,
                        reason,
                        blocking,
                        opened_by_node_id,
                        convert_evidence_refs(evidence_refs),
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace open question recorded: {question_id}")
            }
            TaskSpaceControlArgs::CloseOpenQuestion {
                question_id,
                resolution,
                closed_by_result_id,
                evidence_refs,
            } => {
                session
                    .close_action_map_open_question(
                        &turn,
                        &question_id,
                        resolution,
                        closed_by_result_id,
                        convert_evidence_refs(evidence_refs),
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace open question closed: {question_id}")
            }
            TaskSpaceControlArgs::RecordDecision {
                decision_id,
                decision_kind,
                decision,
                rationale,
                depends_on_results,
                depends_on_facts,
                resolves_questions,
                supports_criteria,
                risks,
            } => {
                session
                    .record_action_map_decision(
                        &turn,
                        ActionMapLedgerDecisionInput {
                            id: decision_id.clone(),
                            decision_kind,
                            decision,
                            rationale,
                            depends_on_results,
                            depends_on_facts,
                            resolves_questions,
                            supports_criteria,
                            risks,
                        },
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace decision recorded: {decision_id}")
            }
            TaskSpaceControlArgs::RecordNextBestAction {
                node_id,
                action_summary,
                reason,
                expected_artifact,
                blocked_by,
            } => {
                session
                    .record_action_map_next_best_action(
                        &turn,
                        node_id,
                        action_summary,
                        reason,
                        expected_artifact,
                        blocked_by,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                "TaskSpace next best action recorded".to_string()
            }
            TaskSpaceControlArgs::RecordSubagentPlan {
                parent_node_id,
                why_parallelizable,
                expected_artifact,
                acceptance_check,
                max_scope,
                supports_questions,
                tests_hypotheses,
                depends_on_results,
            } => {
                let plan_id = session
                    .record_action_map_subagent_plan(
                        &turn,
                        ActionMapSubagentPlanInput {
                            parent_node_id,
                            why_parallelizable,
                            expected_artifact,
                            acceptance_check,
                            max_scope,
                            supports_questions,
                            tests_hypotheses,
                            depends_on_results,
                        },
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace subagent plan recorded: {plan_id}")
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
            TaskSpaceControlArgs::AdoptResult {
                result_id,
                adopted_by_facts,
                adopted_by_hypotheses,
                adopted_by_decisions,
                adopted_by_criteria,
                adopted_by_nodes,
            } => {
                session
                    .adopt_action_map_result(
                        &turn,
                        ActionMapResultAdoptionInput {
                            result_id: result_id.clone(),
                            adopted_by_facts,
                            adopted_by_hypotheses,
                            adopted_by_decisions,
                            adopted_by_criteria,
                            adopted_by_nodes,
                        },
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace result adoption recorded: {result_id}")
            }
            TaskSpaceControlArgs::ReadOutputRef {
                output_ref,
                mode,
                start_line,
                end_line,
                pattern,
                max_bytes,
            } => {
                let mode_tag = format!("mode:{mode}");
                let request = OutputSliceRequest {
                    mode: parse_output_slice_mode(&mode, start_line, end_line, pattern)?,
                    max_bytes: max_bytes.unwrap_or(OUTPUT_SLICE_MAX_BYTES),
                };
                let rollout_path = session
                    .current_rollout_path()
                    .await
                    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
                let slice =
                    read_output_artifact_slice(rollout_path.as_deref(), &output_ref, request)
                        .await
                        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
                session
                    .record_action_map_output_ref_trace_event(
                        &turn,
                        "output_ref.slice_read",
                        None,
                        output_ref,
                        vec![
                            "output_ref".to_string(),
                            "slice_read".to_string(),
                            mode_tag,
                            format!("bytes:{}", slice.len()),
                        ],
                    )
                    .await;
                slice
            }
            TaskSpaceControlArgs::StateCommit {
                commit_id,
                schema_version,
                dry_run,
                active_node_id,
                nodes,
                finished_nodes,
                blockers,
                result_validities,
                result_adoptions,
                success_criteria,
                output_contracts,
                fact_sources,
                facts,
                decisions,
                next_best_action,
            } => {
                validate_state_commit_schema(schema_version.as_deref())?;
                let commit_id = commit_id
                    .unwrap_or_else(|| auto_state_commit_id_from_arguments(&normalized_arguments));
                let outcome = session
                    .state_commit_action_map(
                        &turn,
                        ActionMapStateCommitInput {
                            commit_id: commit_id.clone(),
                            dry_run,
                            active_node_id,
                            nodes: convert_state_commit_nodes(nodes)?,
                            finished_nodes: convert_state_commit_finished_nodes(finished_nodes)?,
                            blockers: convert_state_commit_blockers(blockers),
                            result_validities: convert_state_commit_result_validities(
                                result_validities,
                            ),
                            result_adoptions: convert_state_commit_result_adoptions(
                                result_adoptions,
                            ),
                            success_criteria: convert_success_criteria(success_criteria),
                            output_contracts: convert_state_commit_output_contracts(
                                output_contracts,
                            ),
                            fact_sources: convert_fact_sources(fact_sources),
                            facts: convert_claims(facts),
                            decisions: convert_decisions(decisions),
                            next_best_action: next_best_action.map(convert_next_best_action),
                        },
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!(
                    "TaskSpace state_commit {}: status={} dry_run={} replayed={} accepted_sections=[{}] rejected_sections=[{}]",
                    outcome.commit_id,
                    outcome.status.as_str(),
                    outcome.dry_run,
                    outcome.replayed,
                    outcome.accepted_sections.join(","),
                    outcome
                        .rejected_sections
                        .iter()
                        .map(|error| format!("{}: {}", error.section, error.message))
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            }
        };
        Ok(TaskSpaceControlOutput { message })
    }
}

fn legacy_state_action_name(args: &TaskSpaceControlArgs) -> Option<&'static str> {
    match args {
        TaskSpaceControlArgs::RecordOutputContract { .. } => Some("record_output_contract"),
        TaskSpaceControlArgs::RecordFactSource { .. } => Some("record_fact_source"),
        TaskSpaceControlArgs::RecordFact { .. } => Some("record_fact"),
        TaskSpaceControlArgs::RecordSuccessCriteria { .. } => Some("record_success_criteria"),
        TaskSpaceControlArgs::RecordOpenQuestion { .. } => Some("record_open_question"),
        TaskSpaceControlArgs::CloseOpenQuestion { .. } => Some("close_open_question"),
        TaskSpaceControlArgs::RecordDecision { .. } => Some("record_decision"),
        TaskSpaceControlArgs::RecordNextBestAction { .. } => Some("record_next_best_action"),
        TaskSpaceControlArgs::MarkResultValidity { .. } => Some("mark_result_validity"),
        TaskSpaceControlArgs::AdoptResult { .. } => Some("adopt_result"),
        _ => None,
    }
}

#[cfg(test)]
fn reject_legacy_state_action_for_active_profile(
    args: &TaskSpaceControlArgs,
) -> Result<(), FunctionCallError> {
    let Some(action) = legacy_state_action_name(args) else {
        return Ok(());
    };
    Err(legacy_state_action_rejection(action))
}

fn legacy_state_action_rejection(action: &str) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!(
        "TaskSpace active profile blocks legacy state action `{action}`. Use taskspace_control(action=state_commit, schema_version=taskspace-state-commit-v1) to batch state changes; start_task initial_* fields remain allowed for new-task setup."
    ))
}

fn parse_output_slice_mode(
    mode: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    pattern: Option<String>,
) -> Result<OutputSliceMode, FunctionCallError> {
    match mode {
        "head" => Ok(OutputSliceMode::Head),
        "tail" => Ok(OutputSliceMode::Tail),
        "line_range" => Ok(OutputSliceMode::LineRange {
            start_line: start_line.ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "taskspace_control read_output_ref line_range requires start_line.".to_string(),
                )
            })?,
            end_line: end_line.ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "taskspace_control read_output_ref line_range requires end_line.".to_string(),
                )
            })?,
        }),
        "grep" => Ok(OutputSliceMode::Grep {
            pattern: pattern.ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "taskspace_control read_output_ref grep requires pattern.".to_string(),
                )
            })?,
        }),
        _ => Err(FunctionCallError::RespondToModel(
            "taskspace_control read_output_ref mode must be one of: head, tail, line_range, grep."
                .to_string(),
        )),
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

fn auto_state_commit_id_from_arguments(arguments: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in arguments.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("auto-{hash:016x}")
}

fn normalize_taskspace_arguments(arguments: &str) -> Result<String, FunctionCallError> {
    let mut value: JsonValue = parse_arguments(arguments)?;
    let Some(root) = value.as_object_mut() else {
        return Ok(arguments.to_string());
    };
    if let Some(payload) = root.remove("payload") {
        let JsonValue::Object(payload) = payload else {
            return Err(FunctionCallError::RespondToModel(
                "taskspace_control payload must be a JSON object when provided.".to_string(),
            ));
        };

        for (key, value) in payload {
            root.entry(key).or_insert(value);
        }
    }
    normalize_taskspace_argument_aliases(root);

    serde_json::to_string(&value).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "failed to normalize taskspace_control arguments: {err}"
        ))
    })
}

fn normalize_taskspace_argument_aliases(root: &mut serde_json::Map<String, JsonValue>) {
    move_alias(root, "control_action", "action");
    move_alias(root, "control_type", "action");
    move_alias(root, "action_name", "action");
    move_alias(root, "command", "action");
    let action = root
        .get("action")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string();
    match action.as_str() {
        "start_task" => {
            move_alias(root, "task_name", "task_title");
            move_alias(root, "task_description", "task_objective");
            move_alias(root, "summary", "task_objective");
            move_alias(root, "first_node", "node_title");
            move_alias(root, "first_node_kind", "node_kind");
            move_alias(root, "initial_node_kind", "node_kind");
            move_alias(root, "first_node_id", "node_title");
            move_alias(root, "first_node_title", "node_title");
            move_alias(root, "first_node_description", "node_context_summary");
            move_alias(root, "first_node_context", "node_context_summary");
            move_alias(root, "initial_node_context", "node_context_summary");
            move_alias(root, "description", "node_context_summary");
            move_alias(root, "success_criteria", "initial_success_criteria");
            move_alias(root, "initial_criteria", "initial_success_criteria");
            move_alias(root, "criteria", "initial_success_criteria");
            move_alias(root, "output_contracts", "initial_output_contracts");
            move_alias(root, "initial_contracts", "initial_output_contracts");
            move_alias(root, "contracts", "initial_output_contracts");
            move_alias(root, "fact_sources", "initial_fact_sources");
            root.entry("node_kind".to_string())
                .or_insert_with(|| JsonValue::String("inspect_code_context".to_string()));
            root.entry("task_title".to_string())
                .or_insert_with(|| JsonValue::String("TaskSpace task".to_string()));
            root.entry("node_title".to_string())
                .or_insert_with(|| JsonValue::String("Inspect code context".to_string()));
            root.entry("node_context_summary".to_string())
                .or_insert_with(|| {
                    JsonValue::String(
                        "Inspect the repository context for the user request.".to_string(),
                    )
                });
            normalize_start_task_initial_sections(root);
        }
        "create_node" => {
            move_alias(root, "node_kind", "kind");
            move_alias(root, "node_title", "title");
            move_alias(root, "node_context_summary", "context_summary");
        }
        "finish_node" => {
            root.entry("result_summary".to_string()).or_insert_with(|| {
                JsonValue::String(
                    "TaskSpace node completed with the inspected evidence.".to_string(),
                )
            });
        }
        "record_success_criteria" => {
            move_alias(root, "success_criteria", "criteria");
            if let Some(criteria) = root.get_mut("criteria") {
                normalize_success_criteria_value(criteria);
            }
        }
        "record_output_contract" => {
            if !root.contains_key("output_contract_id") {
                if let Some(description) = summarize_string_array(root.remove("output_contracts")) {
                    root.insert(
                        "output_contract_id".to_string(),
                        JsonValue::String("output-contract-1".to_string()),
                    );
                    root.insert(
                        "output_contract_kind".to_string(),
                        JsonValue::String("artifact".to_string()),
                    );
                    root.insert("description".to_string(), JsonValue::String(description));
                }
            }
            normalize_evidence_refs(root);
        }
        "record_fact_source" => {
            if !root.contains_key("fact_source_id") {
                if let Some(description) = summarize_string_array(root.remove("fact_sources")) {
                    root.insert(
                        "fact_source_id".to_string(),
                        JsonValue::String("fact-source-1".to_string()),
                    );
                    root.insert(
                        "provenance".to_string(),
                        JsonValue::String("observed_from_environment".to_string()),
                    );
                    root.insert("description".to_string(), JsonValue::String(description));
                }
            }
            normalize_fact_source_provenance(root);
            normalize_evidence_refs(root);
            normalize_fact_source_inline_artifact_refs(root);
        }
        "state_commit" => {
            normalize_state_commit_sections(root);
        }
        _ => {}
    }
}

fn move_alias(root: &mut serde_json::Map<String, JsonValue>, alias: &str, target: &str) {
    if root.contains_key(target) {
        return;
    }
    if let Some(value) = root.remove(alias) {
        root.insert(target.to_string(), value);
    }
}

fn summarize_string_array(value: Option<JsonValue>) -> Option<String> {
    let JsonValue::Array(items) = value? else {
        return None;
    };
    let parts: Vec<String> = items
        .into_iter()
        .filter_map(|item| item.as_str().map(str::trim).map(str::to_string))
        .filter(|item| !item.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

fn normalize_success_criteria_value(value: &mut JsonValue) {
    let JsonValue::Array(items) = value else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        match item {
            JsonValue::String(text) => {
                let description = text.trim().to_string();
                *item = serde_json::json!({
                    "id": format!("criterion-{}", index + 1),
                    "kind": "test",
                    "description": description,
                });
            }
            JsonValue::Object(object) => {
                object
                    .entry("id".to_string())
                    .or_insert_with(|| JsonValue::String(format!("criterion-{}", index + 1)));
                object
                    .entry("kind".to_string())
                    .or_insert_with(|| JsonValue::String("test".to_string()));
                if let Some(kind) = object.get("kind").and_then(JsonValue::as_str) {
                    let normalized = match kind {
                        "test_pass" | "command_pass" => Some("test"),
                        _ => None,
                    };
                    if let Some(normalized) = normalized {
                        object.insert(
                            "kind".to_string(),
                            JsonValue::String(normalized.to_string()),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn normalize_fact_source_provenance(root: &mut serde_json::Map<String, JsonValue>) {
    let Some(value) = root.get_mut("provenance") else {
        return;
    };
    let Some(provenance) = value.as_str() else {
        return;
    };
    let normalized = match provenance {
        "file" | "filesystem" | "repo" => Some("observed_from_environment"),
        _ => None,
    };
    if let Some(normalized) = normalized {
        *value = JsonValue::String(normalized.to_string());
    }
}

fn normalize_evidence_refs(root: &mut serde_json::Map<String, JsonValue>) {
    if root.contains_key("evidence_refs") {
        return;
    }
    let Some(refs) = root.remove("refs") else {
        return;
    };
    let JsonValue::Array(items) = refs else {
        return;
    };
    let evidence_refs: Vec<JsonValue> = items
        .into_iter()
        .filter_map(|item| item.as_str().map(str::trim).map(str::to_string))
        .filter(|item| !item.is_empty())
        .map(|artifact_ref| serde_json::json!({ "artifact_ref": artifact_ref }))
        .collect();
    if !evidence_refs.is_empty() {
        root.insert("evidence_refs".to_string(), JsonValue::Array(evidence_refs));
    }
}

fn normalize_state_commit_sections(root: &mut serde_json::Map<String, JsonValue>) {
    if let Some(criteria) = root.get_mut("success_criteria") {
        normalize_success_criteria_map(criteria);
        normalize_success_criteria_value(criteria);
        normalize_success_criteria_objects(criteria);
    }
    normalize_state_commit_result_validities(root);
    normalize_state_commit_decisions(root);
    normalize_state_commit_facts(root);
    normalize_output_contract_array_inline_artifact_refs(root, "output_contracts");
    normalize_state_commit_evidence_array(root, "output_contracts");
    normalize_fact_source_array_inline_artifact_refs(root, "fact_sources");
    normalize_state_commit_evidence_array(root, "fact_sources");
    normalize_state_commit_described_objects(root, "output_contracts", "artifact");
    normalize_state_commit_described_objects(root, "fact_sources", "observed_from_environment");
    if let Some(JsonValue::Array(items)) = root.get_mut("fact_sources") {
        for item in items {
            let JsonValue::Object(object) = item else {
                continue;
            };
            normalize_fact_source_provenance(object);
        }
    }
}

fn normalize_success_criteria_map(value: &mut JsonValue) {
    let JsonValue::Object(items) = value else {
        return;
    };
    let criteria = items
        .iter()
        .filter_map(|(id, status)| {
            let status = status.as_str()?.trim();
            if status.is_empty() {
                return None;
            }
            let status = normalize_success_criterion_status_alias(status);
            Some(serde_json::json!({
                "id": id,
                "kind": "test",
                "description": format!("{id} status {status}"),
                "status": status,
                "evidence_refs": [{ "artifact_ref": "user-request" }],
            }))
        })
        .collect::<Vec<_>>();
    *value = JsonValue::Array(criteria);
}

fn normalize_success_criteria_objects(value: &mut JsonValue) {
    let JsonValue::Array(items) = value else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        let JsonValue::Object(object) = item else {
            continue;
        };
        let id = object
            .entry("id".to_string())
            .or_insert_with(|| JsonValue::String(format!("criterion-{}", index + 1)))
            .as_str()
            .unwrap_or("criterion")
            .to_string();
        let kind = object
            .entry("kind".to_string())
            .or_insert_with(|| JsonValue::String("test".to_string()))
            .as_str()
            .unwrap_or("test")
            .to_string();
        let status = object
            .entry("status".to_string())
            .or_insert_with(|| JsonValue::String(default_success_criterion_status()))
            .as_str()
            .unwrap_or("open")
            .to_string();
        let status = normalize_success_criterion_status_alias(&status);
        object.insert("status".to_string(), JsonValue::String(status.clone()));
        object
            .entry("description".to_string())
            .or_insert_with(|| JsonValue::String(format!("{kind} {id} is {status}")));
        if !object.contains_key("evidence_refs") {
            object.insert(
                "evidence_refs".to_string(),
                JsonValue::Array(vec![serde_json::json!({ "artifact_ref": "user-request" })]),
            );
        }
    }
}

fn normalize_state_commit_result_validities(root: &mut serde_json::Map<String, JsonValue>) {
    let Some(value) = root.get_mut("result_validities") else {
        return;
    };
    match value {
        JsonValue::Object(items) => {
            let validities = items
                .iter()
                .filter_map(|(result_id, validity)| {
                    let validity = validity.as_str()?.trim();
                    if validity.is_empty() {
                        return None;
                    }
                    let validity = normalize_result_validity_alias(validity);
                    Some(serde_json::json!({
                        "result_id": result_id,
                        "validity": validity,
                        "validity_reason": "accepted by compact state_commit normalization",
                        "claims": [{
                            "claim_id": format!("claim-{result_id}-compact-validity"),
                            "statement": format!("{result_id} validity recorded as {validity}"),
                            "evidence_refs": [{ "result_id": result_id }]
                        }],
                        "evidence_refs": [{ "result_id": result_id }]
                    }))
                })
                .collect::<Vec<_>>();
            *value = JsonValue::Array(validities);
        }
        JsonValue::Array(items) => {
            for item in items {
                let JsonValue::Object(object) = item else {
                    continue;
                };
                if !object.contains_key("validity_reason") {
                    object.insert(
                        "validity_reason".to_string(),
                        JsonValue::String(
                            "accepted by compact state_commit normalization".to_string(),
                        ),
                    );
                }
                if let Some(validity) = object.get("validity").and_then(JsonValue::as_str) {
                    object.insert(
                        "validity".to_string(),
                        JsonValue::String(normalize_result_validity_alias(validity)),
                    );
                }
                if !object.contains_key("evidence_refs")
                    && let Some(result_id) = object.get("result_id").and_then(JsonValue::as_str)
                {
                    object.insert(
                        "evidence_refs".to_string(),
                        JsonValue::Array(vec![serde_json::json!({ "result_id": result_id })]),
                    );
                }
            }
        }
        _ => {}
    }
}

fn normalize_result_validity_alias(value: &str) -> String {
    match value.trim().replace('-', "_").to_ascii_lowercase().as_str() {
        "pass" | "passed" | "success" | "succeeded" | "valid" => "accepted".to_string(),
        "fail"
        | "failed"
        | "failure"
        | "error"
        | "rejected"
        | "invalid_infrastructure_failure"
        | "infrastructure_failure"
        | "infra_failure"
        | "invalid_infra_failure" => "invalid".to_string(),
        "uncertain" | "needs_review" | "warning" => "questioned".to_string(),
        other => other.to_string(),
    }
}

fn normalize_success_criterion_status_alias(value: &str) -> String {
    match value.trim().replace('-', "_").to_ascii_lowercase().as_str() {
        "pass" | "passed" | "success" | "succeeded" | "accepted" => "satisfied".to_string(),
        "fail" | "failed" | "failure" | "invalid" | "error" | "needs_review" => {
            "questioned".to_string()
        }
        other => other.to_string(),
    }
}

fn normalize_state_commit_decisions(root: &mut serde_json::Map<String, JsonValue>) {
    let Some(JsonValue::Array(items)) = root.get_mut("decisions") else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        match item {
            JsonValue::String(text) => {
                let decision = text.trim().to_string();
                if !decision.is_empty() {
                    *item = serde_json::json!({
                        "id": format!("decision-{}", index + 1),
                        "decision_kind": "validation",
                        "decision": decision,
                        "rationale": "recorded by compact state_commit normalization"
                    });
                }
            }
            JsonValue::Object(object) => {
                object
                    .entry("id".to_string())
                    .or_insert_with(|| JsonValue::String(format!("decision-{}", index + 1)));
                object
                    .entry("decision_kind".to_string())
                    .or_insert_with(|| JsonValue::String("validation".to_string()));
                if !object.contains_key("decision")
                    && let Some(summary) = object.remove("summary")
                {
                    object.insert("decision".to_string(), summary);
                }
                object.entry("decision".to_string()).or_insert_with(|| {
                    JsonValue::String("TaskSpace state_commit decision".to_string())
                });
                object.entry("rationale".to_string()).or_insert_with(|| {
                    JsonValue::String("recorded by compact state_commit normalization".to_string())
                });
            }
            _ => {}
        }
    }
}

fn normalize_state_commit_facts(root: &mut serde_json::Map<String, JsonValue>) {
    let Some(JsonValue::Array(items)) = root.get_mut("facts") else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        match item {
            JsonValue::String(text) => {
                let statement = text.trim().to_string();
                if !statement.is_empty() {
                    *item = serde_json::json!({
                        "claim_id": format!("fact-{}", index + 1),
                        "statement": statement,
                        "evidence_refs": [{ "artifact_ref": "taskspace-state-commit" }]
                    });
                }
            }
            JsonValue::Object(object) => {
                object
                    .entry("claim_id".to_string())
                    .or_insert_with(|| JsonValue::String(format!("fact-{}", index + 1)));
                if !object.contains_key("statement")
                    && let Some(summary) = object.remove("summary")
                {
                    object.insert("statement".to_string(), summary);
                }
                object.entry("statement".to_string()).or_insert_with(|| {
                    JsonValue::String("TaskSpace state_commit fact".to_string())
                });
                if !object.contains_key("evidence_refs") {
                    object.insert(
                        "evidence_refs".to_string(),
                        JsonValue::Array(vec![
                            serde_json::json!({ "artifact_ref": "taskspace-state-commit" }),
                        ]),
                    );
                }
            }
            _ => {}
        }
    }
}

fn normalize_state_commit_described_objects(
    root: &mut serde_json::Map<String, JsonValue>,
    field: &str,
    default_kind_or_provenance: &str,
) {
    let Some(JsonValue::Array(items)) = root.get_mut(field) else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        let JsonValue::Object(object) = item else {
            continue;
        };
        let id = object
            .get("id")
            .or_else(|| object.get("output_contract_id"))
            .or_else(|| object.get("fact_source_id"))
            .and_then(JsonValue::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                let id = format!("{}-{}", field.replace('_', "-"), index + 1);
                object.insert("id".to_string(), JsonValue::String(id.clone()));
                id
            });
        let default_key = if field == "fact_sources" {
            "provenance"
        } else {
            "kind"
        };
        let has_kind_alias =
            field == "output_contracts" && object.contains_key("output_contract_kind");
        if !has_kind_alias {
            object
                .entry(default_key.to_string())
                .or_insert_with(|| JsonValue::String(default_kind_or_provenance.to_string()));
        }
        object
            .entry("description".to_string())
            .or_insert_with(|| JsonValue::String(format!("{field} {id}")));
    }
}

fn normalize_start_task_initial_sections(root: &mut serde_json::Map<String, JsonValue>) {
    normalize_string_or_array_field(root, "initial_success_criteria");
    normalize_string_or_array_field(root, "initial_output_contracts");
    normalize_string_or_array_field(root, "initial_fact_sources");
    if let Some(criteria) = root.get_mut("initial_success_criteria") {
        normalize_success_criteria_value(criteria);
        normalize_success_criteria_objects(criteria);
    }
    normalize_output_contract_array(root, "initial_output_contracts");
    normalize_fact_source_array(root, "initial_fact_sources");
    if let Some(JsonValue::Array(items)) = root.get_mut("initial_fact_sources") {
        for item in items {
            let JsonValue::Object(object) = item else {
                continue;
            };
            normalize_fact_source_provenance(object);
            normalize_fact_source_inline_artifact_refs(object);
        }
    }
}

fn normalize_string_or_array_field(root: &mut serde_json::Map<String, JsonValue>, field: &str) {
    let Some(value) = root.get_mut(field) else {
        return;
    };
    if value.is_string() {
        let text = value.as_str().unwrap_or_default().trim().to_string();
        *value = if text.is_empty() {
            JsonValue::Array(Vec::new())
        } else {
            JsonValue::Array(vec![JsonValue::String(text)])
        };
    } else if value.is_object() {
        let object = std::mem::replace(value, JsonValue::Null);
        *value = JsonValue::Array(vec![object]);
    }
}

fn normalize_output_contract_array(root: &mut serde_json::Map<String, JsonValue>, field: &str) {
    let Some(JsonValue::Array(items)) = root.get_mut(field) else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        match item {
            JsonValue::String(text) => {
                let description = text.trim().to_string();
                let evidence_refs = string_output_contract_artifact_ref(&description)
                    .map(|artifact_ref| {
                        JsonValue::Array(vec![serde_json::json!({ "artifact_ref": artifact_ref })])
                    })
                    .unwrap_or_else(|| {
                        JsonValue::Array(vec![
                            serde_json::json!({ "artifact_ref": "user-request" }),
                        ])
                    });
                *item = serde_json::json!({
                    "id": format!("output-contract-{}", index + 1),
                    "kind": "artifact",
                    "description": description,
                    "evidence_refs": evidence_refs,
                });
            }
            JsonValue::Object(object) => {
                object
                    .entry("id".to_string())
                    .or_insert_with(|| JsonValue::String(format!("output-contract-{}", index + 1)));
                object
                    .entry("kind".to_string())
                    .or_insert_with(|| JsonValue::String("artifact".to_string()));
                object.entry("description".to_string()).or_insert_with(|| {
                    JsonValue::String(format!(
                        "initial_output_contracts output-contract-{}",
                        index + 1
                    ))
                });
                normalize_output_contract_inline_artifact_refs(object);
                let needs_default = match object.get("evidence_refs") {
                    Some(JsonValue::Array(existing)) => existing.is_empty(),
                    Some(_) => false,
                    None => true,
                };
                if needs_default {
                    object.insert(
                        "evidence_refs".to_string(),
                        JsonValue::Array(vec![
                            serde_json::json!({ "artifact_ref": "user-request" }),
                        ]),
                    );
                }
            }
            _ => {}
        }
    }
}

fn string_output_contract_artifact_ref(description: &str) -> Option<String> {
    let value = description.trim();
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return None;
    }
    let file_name = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .trim_matches('.');
    if value.contains('/') || value.contains('\\') || file_name.contains('.') {
        Some(value.to_string())
    } else {
        None
    }
}

fn normalize_fact_source_array(root: &mut serde_json::Map<String, JsonValue>, field: &str) {
    let Some(JsonValue::Array(items)) = root.get_mut(field) else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        match item {
            JsonValue::String(text) => {
                let description = text.trim().to_string();
                *item = serde_json::json!({
                    "id": format!("fact-source-{}", index + 1),
                    "provenance": "observed_from_environment",
                    "description": description,
                    "evidence_refs": [{ "artifact_ref": "user-request" }],
                });
            }
            JsonValue::Object(object) => {
                object
                    .entry("id".to_string())
                    .or_insert_with(|| JsonValue::String(format!("fact-source-{}", index + 1)));
                object
                    .entry("provenance".to_string())
                    .or_insert_with(|| JsonValue::String("observed_from_environment".to_string()));
                object.entry("description".to_string()).or_insert_with(|| {
                    JsonValue::String(format!("initial_fact_sources fact-source-{}", index + 1))
                });
                normalize_fact_source_inline_artifact_refs(object);
                let needs_default = match object.get("evidence_refs") {
                    Some(JsonValue::Array(existing)) => existing.is_empty(),
                    Some(_) => false,
                    None => true,
                };
                if needs_default {
                    object.insert(
                        "evidence_refs".to_string(),
                        JsonValue::Array(vec![
                            serde_json::json!({ "artifact_ref": "user-request" }),
                        ]),
                    );
                }
            }
            _ => {}
        }
    }
}

fn normalize_state_commit_evidence_array(
    root: &mut serde_json::Map<String, JsonValue>,
    field: &str,
) {
    let Some(JsonValue::Array(items)) = root.get_mut(field) else {
        return;
    };
    for item in items {
        let JsonValue::Object(object) = item else {
            continue;
        };
        let needs_default = match object.get("evidence_refs") {
            Some(JsonValue::Array(existing)) => existing.is_empty(),
            Some(_) => false,
            None => true,
        };
        if needs_default {
            object.insert(
                "evidence_refs".to_string(),
                JsonValue::Array(vec![serde_json::json!({ "artifact_ref": "user-request" })]),
            );
        }
    }
}

fn normalize_output_contract_array_inline_artifact_refs(
    root: &mut serde_json::Map<String, JsonValue>,
    field: &str,
) {
    let Some(JsonValue::Array(items)) = root.get_mut(field) else {
        return;
    };
    for item in items {
        let JsonValue::Object(object) = item else {
            continue;
        };
        normalize_output_contract_inline_artifact_refs(object);
    }
}

fn normalize_output_contract_inline_artifact_refs(object: &mut serde_json::Map<String, JsonValue>) {
    let artifact_refs = inline_output_contract_artifact_refs(object);
    append_inline_artifact_refs_to_evidence_refs(object, artifact_refs);
}

fn normalize_fact_source_array_inline_artifact_refs(
    root: &mut serde_json::Map<String, JsonValue>,
    field: &str,
) {
    let Some(JsonValue::Array(items)) = root.get_mut(field) else {
        return;
    };
    for item in items {
        let JsonValue::Object(object) = item else {
            continue;
        };
        normalize_fact_source_inline_artifact_refs(object);
    }
}

fn normalize_fact_source_inline_artifact_refs(object: &mut serde_json::Map<String, JsonValue>) {
    let artifact_refs = inline_fact_source_artifact_refs(object);
    append_inline_artifact_refs_to_evidence_refs(object, artifact_refs);
}

fn append_inline_artifact_refs_to_evidence_refs(
    object: &mut serde_json::Map<String, JsonValue>,
    artifact_refs: Vec<String>,
) {
    if artifact_refs.is_empty() {
        return;
    }
    let entry = object
        .entry("evidence_refs".to_string())
        .or_insert_with(|| JsonValue::Array(Vec::new()));
    let JsonValue::Array(evidence_refs) = entry else {
        return;
    };
    for artifact_ref in artifact_refs {
        let already_present = evidence_refs.iter().any(|evidence_ref| {
            evidence_ref
                .get("artifact_ref")
                .and_then(JsonValue::as_str)
                .is_some_and(|existing| existing == artifact_ref)
        });
        if !already_present {
            evidence_refs.push(serde_json::json!({ "artifact_ref": artifact_ref }));
        }
    }
}

fn inline_output_contract_artifact_refs(
    object: &serde_json::Map<String, JsonValue>,
) -> Vec<String> {
    let mut artifact_refs = Vec::new();
    for key in [
        "path",
        "artifact_ref",
        "artifactRef",
        "artifact_path",
        "artifactPath",
        "output_path",
        "outputPath",
        "target_path",
        "targetPath",
    ] {
        push_inline_artifact_ref(object.get(key), &mut artifact_refs);
    }
    for key in [
        "paths",
        "artifact_refs",
        "artifactRefs",
        "artifact_paths",
        "artifactPaths",
        "output_paths",
        "outputPaths",
        "target_paths",
        "targetPaths",
    ] {
        push_inline_artifact_refs(object.get(key), &mut artifact_refs);
    }
    artifact_refs
}

fn inline_fact_source_artifact_refs(object: &serde_json::Map<String, JsonValue>) -> Vec<String> {
    let mut artifact_refs = Vec::new();
    for key in ["path", "artifact_ref", "artifact_path", "source_path"] {
        push_inline_artifact_ref(object.get(key), &mut artifact_refs);
    }
    for key in ["paths", "artifact_refs", "artifact_paths", "source_paths"] {
        push_inline_artifact_refs(object.get(key), &mut artifact_refs);
    }
    artifact_refs
}

fn push_inline_artifact_ref(value: Option<&JsonValue>, artifact_refs: &mut Vec<String>) {
    let Some(artifact_ref) = value.and_then(JsonValue::as_str).map(str::trim) else {
        return;
    };
    if artifact_ref.is_empty()
        || artifact_refs
            .iter()
            .any(|existing| existing == artifact_ref)
    {
        return;
    }
    artifact_refs.push(artifact_ref.to_string());
}

fn push_inline_artifact_refs(value: Option<&JsonValue>, artifact_refs: &mut Vec<String>) {
    let Some(JsonValue::Array(items)) = value else {
        return;
    };
    for item in items {
        push_inline_artifact_ref(Some(item), artifact_refs);
    }
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

fn convert_success_criteria(
    inputs: Vec<TaskSpaceSuccessCriterionArgs>,
) -> Vec<ActionMapSuccessCriterionInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapSuccessCriterionInput {
            id: input.id,
            kind: input.kind,
            description: input.description,
            status: input.status,
            evidence_refs: convert_evidence_refs(input.evidence_refs),
        })
        .collect()
}

fn convert_fact_sources(
    inputs: Vec<TaskSpaceFactSourceArgs>,
) -> Vec<ActionMapStateCommitFactSourceInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapStateCommitFactSourceInput {
            id: input.id,
            provenance: input.provenance,
            description: input.description,
            evidence_refs: convert_evidence_refs(input.evidence_refs),
        })
        .collect()
}

fn convert_state_commit_output_contracts(
    inputs: Vec<TaskSpaceOutputContractArgs>,
) -> Vec<ActionMapStateCommitOutputContractInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapStateCommitOutputContractInput {
            id: input.id,
            kind: input.kind,
            description: input.description,
            evidence_refs: convert_evidence_refs(input.evidence_refs),
        })
        .collect()
}

fn convert_decisions(inputs: Vec<TaskSpaceDecisionArgs>) -> Vec<ActionMapLedgerDecisionInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapLedgerDecisionInput {
            id: input.id,
            decision_kind: input.decision_kind,
            decision: input.decision,
            rationale: input.rationale,
            depends_on_results: input.depends_on_results,
            depends_on_facts: input.depends_on_facts,
            resolves_questions: input.resolves_questions,
            supports_criteria: input.supports_criteria,
            risks: input.risks,
        })
        .collect()
}

fn convert_next_best_action(
    input: TaskSpaceNextBestActionArgs,
) -> ActionMapStateCommitNextBestActionInput {
    ActionMapStateCommitNextBestActionInput {
        node_id: input.node_id,
        action_summary: input.action_summary,
        reason: input.reason,
        expected_artifact: input.expected_artifact,
        blocked_by: input.blocked_by,
    }
}

fn convert_state_commit_nodes(
    inputs: Vec<TaskSpaceStateCommitNodeArgs>,
) -> Result<Vec<ActionMapStateCommitNodeInput>, FunctionCallError> {
    inputs
        .into_iter()
        .map(|input| {
            Ok(ActionMapStateCommitNodeInput {
                kind: parse_node_kind("nodes.kind", &input.kind)?,
                title: input.title,
                context_summary: input.context_summary,
                dependency_node_ids: input.dependency_node_ids,
                bind_current: input.bind_current,
            })
        })
        .collect()
}

fn convert_state_commit_finished_nodes(
    inputs: Vec<TaskSpaceStateCommitFinishNodeArgs>,
) -> Result<Vec<ActionMapStateCommitFinishNodeInput>, FunctionCallError> {
    inputs
        .into_iter()
        .map(|input| {
            Ok(ActionMapStateCommitFinishNodeInput {
                node_id: input.node_id,
                result_summary: input.result_summary,
                next_node_id: input.next_node_id,
                next_node_draft: build_next_node_draft(
                    input.next_node_kind,
                    input.next_node_title,
                    input.next_node_context_summary,
                    input.next_dependency_node_ids,
                )?,
            })
        })
        .collect()
}

fn convert_state_commit_blockers(
    inputs: Vec<TaskSpaceStateCommitBlockerArgs>,
) -> Vec<ActionMapStateCommitBlockerInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapStateCommitBlockerInput {
            node_id: input.node_id,
            blocker_summary: input.blocker_summary,
        })
        .collect()
}

fn convert_state_commit_result_validities(
    inputs: Vec<TaskSpaceStateCommitResultValidityArgs>,
) -> Vec<ActionMapStateCommitResultValidityInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapStateCommitResultValidityInput {
            result_id: input.result_id,
            validity: input.validity,
            validity_reason: input.validity_reason,
            claims: convert_claims(input.claims),
            evidence_refs: convert_evidence_refs(input.evidence_refs),
            changed_artifacts: input.changed_artifacts,
            validator_refs: input.validator_refs,
            remaining_uncertainty: input.remaining_uncertainty,
        })
        .collect()
}

fn convert_state_commit_result_adoptions(
    inputs: Vec<TaskSpaceStateCommitResultAdoptionArgs>,
) -> Vec<ActionMapResultAdoptionInput> {
    inputs
        .into_iter()
        .map(|input| ActionMapResultAdoptionInput {
            result_id: input.result_id,
            adopted_by_facts: input.adopted_by_facts,
            adopted_by_hypotheses: input.adopted_by_hypotheses,
            adopted_by_decisions: input.adopted_by_decisions,
            adopted_by_criteria: input.adopted_by_criteria,
            adopted_by_nodes: input.adopted_by_nodes,
        })
        .collect()
}

fn validate_state_commit_schema(schema_version: Option<&str>) -> Result<(), FunctionCallError> {
    if schema_version
        .map(|value| value.trim().is_empty() || value == "taskspace-state-commit-v1")
        .unwrap_or(true)
    {
        return Ok(());
    }
    Err(FunctionCallError::RespondToModel(
        "taskspace_control state_commit schema_version must be taskspace-state-commit-v1."
            .to_string(),
    ))
}

fn default_success_criterion_status() -> String {
    "open".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_task_accepts_missing_initial_success_criteria_for_gate_recovery() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "start_task",
            "task_title": "Audit",
            "task_objective": "Audit the codebase",
            "node_kind": "inspect_code_context",
            "node_title": "Inspect",
            "node_context_summary": "Read the project shape",
            "bind_current": true
        }))
        .expect("start_task args parse");

        match args {
            TaskSpaceControlArgs::StartTask {
                initial_success_criteria,
                ..
            } => assert!(initial_success_criteria.is_empty()),
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_defaults_missing_bind_current_for_main_path() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "start_task",
            "task_title": "Fix test",
            "task_objective": "Fix the failing test",
            "node_kind": "inspect_code_context",
            "node_title": "Inspect",
            "node_context_summary": "Read the README and tests"
        }))
        .expect("start_task args parse");

        match args {
            TaskSpaceControlArgs::StartTask { bind_current, .. } => {
                assert_eq!(bind_current.unwrap_or(true), true)
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_parses_initial_success_criteria_when_present() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "start_task",
            "task_title": "Audit",
            "task_objective": "Audit the codebase",
            "node_kind": "inspect_code_context",
            "initial_success_criteria": [{
                "id": "sc-1",
                "kind": "validator",
                "description": "Public validator exits 0",
                "evidence_refs": [{"artifact_ref": "user-request"}]
            }],
            "node_title": "Inspect",
            "node_context_summary": "Read the project shape"
        }))
        .expect("start_task criteria parse");

        match args {
            TaskSpaceControlArgs::StartTask {
                initial_success_criteria,
                ..
            } => {
                assert_eq!(initial_success_criteria.len(), 1);
                assert_eq!(initial_success_criteria[0].id, "sc-1");
                assert_eq!(initial_success_criteria[0].status, "open");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_normalizes_initial_scaffold_aliases() {
        let raw = serde_json::json!({
            "action": "start_task",
            "task_title": "Audit",
            "task_objective": "Audit the codebase",
            "success_criteria": ["Validator passes"],
            "output_contracts": [{
                "id": "oc-1",
                "kind": "validator",
                "description": "Final answer reports validator status"
            }],
            "fact_sources": [{
                "id": "fs-1",
                "provenance": "repo",
                "description": "Repository state at task start",
                "evidence_refs": []
            }],
            "node_title": "Inspect",
            "node_context_summary": "Read the project shape"
        })
        .to_string();

        let normalized =
            normalize_taskspace_arguments(&raw).expect("start_task scaffold normalizes");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task scaffold parses");

        match args {
            TaskSpaceControlArgs::StartTask {
                initial_success_criteria,
                initial_output_contracts,
                initial_fact_sources,
                ..
            } => {
                assert_eq!(initial_success_criteria.len(), 1);
                assert_eq!(initial_success_criteria[0].id, "criterion-1");
                assert_eq!(initial_output_contracts.len(), 1);
                assert_eq!(
                    initial_output_contracts[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("user-request")
                );
                assert_eq!(initial_fact_sources.len(), 1);
                assert_eq!(
                    initial_fact_sources[0].provenance,
                    "observed_from_environment"
                );
                assert_eq!(
                    initial_fact_sources[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("user-request")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn record_output_contract_prefers_specific_kind_field_and_keeps_alias() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "record_output_contract",
            "output_contract_id": "contract-1",
            "output_contract_kind": "validator",
            "description": "Public validator exits 0",
            "evidence_refs": [{"artifact_ref": "user-request"}]
        }))
        .expect("record_output_contract args parse");

        match args {
            TaskSpaceControlArgs::RecordOutputContract {
                output_contract_kind,
                ..
            } => assert_eq!(output_contract_kind, "validator"),
            other => panic!("unexpected args: {other:?}"),
        }

        let legacy_args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "record_output_contract",
            "output_contract_id": "contract-1",
            "kind": "validator",
            "description": "Public validator exits 0",
            "evidence_refs": [{"artifact_ref": "user-request"}]
        }))
        .expect("record_output_contract legacy args parse");

        match legacy_args {
            TaskSpaceControlArgs::RecordOutputContract {
                output_contract_kind,
                ..
            } => assert_eq!(output_contract_kind, "validator"),
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn adopt_result_parses_all_adoption_refs() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "adopt_result",
            "result_id": "result-1",
            "adopted_by_facts": ["fact-1"],
            "adopted_by_hypotheses": ["hyp-1"],
            "adopted_by_decisions": ["decision-1"],
            "adopted_by_criteria": ["sc-1"],
            "adopted_by_nodes": ["node-1"]
        }))
        .expect("adopt_result args parse");

        match args {
            TaskSpaceControlArgs::AdoptResult {
                result_id,
                adopted_by_facts,
                adopted_by_hypotheses,
                adopted_by_decisions,
                adopted_by_criteria,
                adopted_by_nodes,
            } => {
                assert_eq!(result_id, "result-1");
                assert_eq!(adopted_by_facts, vec!["fact-1"]);
                assert_eq!(adopted_by_hypotheses, vec!["hyp-1"]);
                assert_eq!(adopted_by_decisions, vec!["decision-1"]);
                assert_eq!(adopted_by_criteria, vec!["sc-1"]);
                assert_eq!(adopted_by_nodes, vec!["node-1"]);
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_parses_batch_sections() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "state_commit",
            "commit_id": "commit-1",
            "schema_version": "taskspace-state-commit-v1",
            "dry_run": true,
            "active_node_id": "node-1",
            "nodes": [{
                "kind": "smoke_test",
                "title": "Smoke",
                "context_summary": "Run smoke checks",
                "dependency_node_ids": ["node-1"]
            }],
            "finished_nodes": [{
                "node_id": "node-1",
                "result_summary": "inspection complete",
                "next_node_kind": "smoke_test",
                "next_node_title": "Smoke",
                "next_node_context_summary": "Run smoke checks"
            }],
            "blockers": [{
                "node_id": "node-2",
                "blocker_summary": "environment missing"
            }],
            "success_criteria": [{
                "id": "sc-1",
                "kind": "validator",
                "description": "self-test passes",
                "evidence_refs": [{"artifact_ref": "user-request"}]
            }],
            "output_contracts": [{
                "output_contract_id": "oc-1",
                "output_contract_kind": "artifact",
                "description": "updated source file",
                "evidence_refs": [{"artifact_ref": "user-request"}]
            }],
            "fact_sources": [{
                "fact_source_id": "source-1",
                "provenance": "provided_by_user",
                "description": "The user requested v0.0.5 completion",
                "evidence_refs": [{"artifact_ref": "user-request"}]
            }],
            "facts": [{
                "claim_id": "fact-1",
                "statement": "Phase 1 needs transactional state_commit",
                "evidence_refs": [{"fact_source_id": "source-1"}]
            }],
            "decisions": [{
                "decision_id": "decision-1",
                "decision_kind": "implementation",
                "decision": "reuse existing ledger structures",
                "rationale": "prevents parallel state",
                "depends_on_facts": ["fact-1"]
            }],
            "result_validities": [{
                "result_id": "result-1",
                "validity": "accepted",
                "validity_reason": "validator passed",
                "claims": [{"claim_id": "claim-1", "statement": "result is valid"}],
                "evidence_refs": [{"result_id": "result-1"}],
                "changed_artifacts": ["src/lib.rs"]
            }],
            "result_adoptions": [{
                "result_id": "result-1",
                "adopted_by_facts": ["fact-1"]
            }],
            "next_best_action": {
                "node_id": "node-1",
                "action_summary": "run focused tests",
                "reason": "validate the commit path"
            }
        }))
        .expect("state_commit args parse");

        match args {
            TaskSpaceControlArgs::StateCommit {
                commit_id,
                dry_run,
                active_node_id,
                nodes,
                finished_nodes,
                blockers,
                success_criteria,
                output_contracts,
                fact_sources,
                facts,
                decisions,
                result_validities,
                result_adoptions,
                next_best_action,
                ..
            } => {
                assert_eq!(commit_id.as_deref(), Some("commit-1"));
                assert!(dry_run);
                assert_eq!(active_node_id.as_deref(), Some("node-1"));
                assert_eq!(nodes[0].kind, "smoke_test");
                assert_eq!(
                    finished_nodes[0].next_node_kind.as_deref(),
                    Some("smoke_test")
                );
                assert_eq!(blockers[0].node_id, "node-2");
                assert_eq!(success_criteria[0].id, "sc-1");
                assert_eq!(output_contracts[0].id, "oc-1");
                assert_eq!(fact_sources[0].id, "source-1");
                assert_eq!(facts[0].claim_id, "fact-1");
                assert_eq!(decisions[0].id, "decision-1");
                assert_eq!(result_validities[0].result_id, "result-1");
                assert_eq!(result_adoptions[0].adopted_by_facts, vec!["fact-1"]);
                assert_eq!(
                    next_best_action.expect("next action").action_summary,
                    "run focused tests"
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_fact_source_path_normalizes_to_artifact_ref() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "commit_id": "commit-1",
            "schema_version": "taskspace-state-commit-v1",
            "fact_sources": [{
                "path": "projects.csv",
                "description": "Input CSV with project data"
            }]
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit fact-source paths parse");

        match args {
            TaskSpaceControlArgs::StateCommit { fact_sources, .. } => {
                assert_eq!(fact_sources.len(), 1);
                assert_eq!(fact_sources[0].id, "fact-sources-1");
                assert_eq!(fact_sources[0].provenance, "observed_from_environment");
                assert_eq!(
                    fact_sources[0].evidence_refs[0].artifact_ref.as_deref(),
                    Some("projects.csv")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_output_contract_path_normalizes_to_artifact_ref() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "commit_id": "commit-1",
            "schema_version": "taskspace-state-commit-v1",
            "output_contracts": [{
                "path": "merged_users.parquet",
                "description": "Merged user dataset"
            }]
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit output-contract paths parse");

        match args {
            TaskSpaceControlArgs::StateCommit {
                output_contracts, ..
            } => {
                assert_eq!(output_contracts.len(), 1);
                assert_eq!(output_contracts[0].id, "output-contracts-1");
                assert_eq!(output_contracts[0].kind, "artifact");
                assert_eq!(
                    output_contracts[0].evidence_refs[0].artifact_ref.as_deref(),
                    Some("merged_users.parquet")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn active_profile_rejects_direct_legacy_state_action() {
        let raw = serde_json::json!({
            "action": "record_fact",
            "claim_id": "claim-1",
            "statement": "Legacy state update should be displaced",
            "evidence_refs": [{"artifact_ref": "artifact://request"}]
        });
        let args: TaskSpaceControlArgs =
            serde_json::from_value(raw).expect("record_fact args parse");

        let err = reject_legacy_state_action_for_active_profile(&args)
            .expect_err("legacy state action should be blocked");

        assert!(
            err.to_string()
                .contains("active profile blocks legacy state action `record_fact`")
        );
    }

    #[test]
    fn active_profile_allows_state_commit_action() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "schema_version": "taskspace-state-commit-v1",
            "active_node_id": "node-1",
            "facts": [{
                "claim_id": "claim-1",
                "statement": "State commit remains the active profile path",
                "evidence_refs": [{"artifact_ref": "artifact://request"}]
            }]
        });
        let args: TaskSpaceControlArgs =
            serde_json::from_value(raw).expect("state_commit args parse");

        reject_legacy_state_action_for_active_profile(&args)
            .expect("state_commit should remain allowed");
    }

    #[test]
    fn state_commit_accepts_missing_commit_id_for_recovery() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "schema_version": "taskspace-state-commit-v1",
            "success_criteria": [{
                "id": "sc-1",
                "kind": "validator",
                "description": "self-test passes",
                "evidence_refs": [{"artifact_ref": "user-request"}]
            }]
        });
        let arguments = raw.to_string();
        let args: TaskSpaceControlArgs =
            serde_json::from_value(raw).expect("state_commit args parse");

        match args {
            TaskSpaceControlArgs::StateCommit {
                commit_id,
                success_criteria,
                ..
            } => {
                assert!(commit_id.is_none());
                assert_eq!(success_criteria[0].id, "sc-1");
                assert!(auto_state_commit_id_from_arguments(&arguments).starts_with("auto-"));
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn compact_payload_normalizes_to_legacy_arguments() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "payload": {
                "schema_version": "taskspace-state-commit-v1",
                "success_criteria": [{
                    "id": "sc-1",
                    "kind": "test",
                    "description": "compact payload parses",
                    "status": "open",
                    "evidence_refs": []
                }]
            }
        });
        let normalized =
            normalize_taskspace_arguments(&raw.to_string()).expect("payload normalizes");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit args parse");

        match args {
            TaskSpaceControlArgs::StateCommit {
                schema_version,
                success_criteria,
                ..
            } => {
                assert_eq!(schema_version.as_deref(), Some("taskspace-state-commit-v1"));
                assert_eq!(success_criteria.len(), 1);
                assert_eq!(success_criteria[0].id, "sc-1");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn smoke_style_start_task_aliases_normalize() {
        let raw = serde_json::json!({
            "action": "start_task",
            "payload": {
                "task_name": "fix-failing-test",
                "first_node": "diagnose-and-inspect",
                "description": "Run diagnostic command, inspect README and tests."
            }
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task aliases parse");

        match args {
            TaskSpaceControlArgs::StartTask {
                task_title,
                node_kind,
                node_title,
                node_context_summary,
                ..
            } => {
                assert_eq!(task_title, "fix-failing-test");
                assert_eq!(node_kind, "inspect_code_context");
                assert_eq!(node_title, "diagnose-and-inspect");
                assert!(node_context_summary.contains("diagnostic command"));
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_accepts_action_name_alias() {
        let raw = serde_json::json!({
            "action_name": "start_task",
            "first_node_id": "inspect_context",
            "first_node_kind": "inspect_code_context",
            "initial_success_criteria": "Tax calculation tests pass",
            "initial_fact_sources": ["README", "test files", "source files"]
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let value: JsonValue = serde_json::from_str(&normalized).expect("json");
        assert_eq!(value["action"], "start_task");
        assert!(value.get("action_name").is_none());

        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task action_name alias parses");
        match args {
            TaskSpaceControlArgs::StartTask {
                node_kind,
                node_title,
                initial_success_criteria,
                initial_fact_sources,
                ..
            } => {
                assert_eq!(node_kind, "inspect_code_context");
                assert_eq!(node_title, "inspect_context");
                assert_eq!(initial_success_criteria[0].id, "criterion-1");
                assert_eq!(initial_fact_sources.len(), 3);
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_accepts_natural_task_payload_aliases() {
        let raw = serde_json::json!({
            "action": "start_task",
            "task_description": "Create a JSON processor that transforms departments.csv, employees.csv, and projects.csv into organization.json following schema.json.",
            "initial_criteria": [
                "Read schema.json",
                "Verify organization.json structure matches schema"
            ],
            "initial_contracts": [
                "organization.json file with correct structure and data",
                "Code for JSON processor that can reproduce the output"
            ],
            "initial_fact_sources": [
                "schema.json",
                "departments.csv",
                "employees.csv",
                "projects.csv"
            ],
            "first_node_kind": "inspect_code_context",
            "first_node_description": "Explore the provided CSV files and schema.json."
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let value: JsonValue = serde_json::from_str(&normalized).expect("json");
        assert_eq!(
            value["task_objective"],
            "Create a JSON processor that transforms departments.csv, employees.csv, and projects.csv into organization.json following schema.json."
        );
        assert!(value.get("task_description").is_none());
        assert!(value.get("initial_criteria").is_none());
        assert!(value.get("initial_contracts").is_none());
        assert_eq!(value["node_kind"], "inspect_code_context");

        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("natural start task aliases parse");
        match args {
            TaskSpaceControlArgs::StartTask {
                task_objective,
                node_kind,
                initial_success_criteria,
                initial_output_contracts,
                initial_fact_sources,
                node_context_summary,
                ..
            } => {
                assert!(task_objective.contains("organization.json"));
                assert_eq!(node_kind, "inspect_code_context");
                assert_eq!(initial_success_criteria.len(), 2);
                assert_eq!(initial_output_contracts.len(), 2);
                assert_eq!(initial_fact_sources.len(), 4);
                assert_eq!(initial_fact_sources[0].description, "schema.json");
                assert!(node_context_summary.contains("CSV files"));
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_wraps_single_initial_section_objects() {
        let raw = serde_json::json!({
            "action": "start_task",
            "task_description": "Validate organization.json with schema.json",
            "initial_contracts": {
                "description": "organization.json follows schema.json"
            },
            "initial_fact_sources": {
                "path": "schema.json"
            },
            "first_node_kind": "inspect_code_context"
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("single object sections parse");
        match args {
            TaskSpaceControlArgs::StartTask {
                initial_output_contracts,
                initial_fact_sources,
                ..
            } => {
                assert_eq!(initial_output_contracts.len(), 1);
                assert_eq!(
                    initial_output_contracts[0].description,
                    "organization.json follows schema.json"
                );
                assert_eq!(initial_fact_sources.len(), 1);
                assert_eq!(
                    initial_fact_sources[0].description,
                    "initial_fact_sources fact-source-1"
                );
                assert_eq!(
                    initial_fact_sources[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("schema.json")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_output_contract_path_normalizes_to_artifact_ref() {
        let raw = serde_json::json!({
            "action": "start_task",
            "task_description": "Merge users and output merged_users.parquet",
            "initial_output_contracts": [{
                "path": "merged_users.parquet",
                "description": "Merged user dataset with required columns"
            }],
            "first_node_kind": "inspect_code_context"
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task output-contract path parses");
        match args {
            TaskSpaceControlArgs::StartTask {
                initial_output_contracts,
                ..
            } => {
                assert_eq!(initial_output_contracts.len(), 1);
                assert_eq!(
                    initial_output_contracts[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("merged_users.parquet")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_string_output_contract_paths_normalize_to_artifact_refs() {
        let raw = serde_json::json!({
            "action": "start_task",
            "task_description": "Merge users and produce declared files",
            "initial_output_contracts": [
                "/app/merged_users.parquet",
                "/app/conflicts.json",
                "Fixed implementation"
            ],
            "first_node_kind": "inspect_code_context"
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task string path contracts parse");
        match args {
            TaskSpaceControlArgs::StartTask {
                initial_output_contracts,
                ..
            } => {
                assert_eq!(initial_output_contracts.len(), 3);
                assert_eq!(
                    initial_output_contracts[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("/app/merged_users.parquet")
                );
                assert_eq!(
                    initial_output_contracts[1].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("/app/conflicts.json")
                );
                assert_eq!(
                    initial_output_contracts[2].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("user-request")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_fact_source_path_normalizes_to_artifact_ref() {
        let raw = serde_json::json!({
            "action": "start_task",
            "node_kind": "inspect_code_context",
            "initial_fact_sources": [{
                "path": "schema.json",
                "description": "Defines the expected organization.json schema"
            }, {
                "path": "employees.csv",
                "description": "Input CSV with employee data"
            }]
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task fact-source paths parse");

        match args {
            TaskSpaceControlArgs::StartTask {
                initial_fact_sources,
                ..
            } => {
                assert_eq!(initial_fact_sources.len(), 2);
                assert_eq!(initial_fact_sources[0].id, "fact-source-1");
                assert_eq!(
                    initial_fact_sources[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("schema.json")
                );
                assert_eq!(
                    initial_fact_sources[1].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("employees.csv")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn finish_node_accepts_command_alias() {
        let raw = serde_json::json!({
            "command": "finish_node",
            "node_id": "node-1",
            "next_node_kind": "implement_solution",
            "next_node_title": "Apply fix",
            "next_node_context_summary": "Patch src/tax_calc.py based on tests."
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let value: JsonValue = serde_json::from_str(&normalized).expect("json");
        assert_eq!(value["action"], "finish_node");
        assert!(value.get("command").is_none());

        let args: TaskSpaceControlArgs =
            parse_arguments(&normalized).expect("finish_node command alias parses");
        match args {
            TaskSpaceControlArgs::FinishNode {
                result_summary,
                next_node_kind,
                ..
            } => {
                assert!(result_summary.contains("TaskSpace node completed"));
                assert_eq!(next_node_kind.as_deref(), Some("implement_solution"));
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn finish_node_missing_result_summary_normalizes() {
        let raw = serde_json::json!({
            "action": "finish_node",
            "node_id": "node-1",
            "next_node_kind": "implement_solution",
            "next_node_title": "Apply inspected fix",
            "next_node_context_summary": "Patch the inspected file"
        });

        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            parse_arguments(&normalized).expect("finish_node args parse");

        match args {
            TaskSpaceControlArgs::FinishNode { result_summary, .. } => {
                assert!(result_summary.contains("TaskSpace node completed"));
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn start_task_direct_string_sections_normalize() {
        let raw = serde_json::json!({
            "action": "start_task",
            "node_kind": "inspect_code_context",
            "initial_success_criteria": "All tax tests pass",
            "initial_output_contracts": "Fixed implementation",
            "initial_fact_sources": ["README", "tests"],
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("start_task direct strings parse");

        match args {
            TaskSpaceControlArgs::StartTask {
                task_title,
                node_title,
                node_context_summary,
                initial_success_criteria,
                initial_output_contracts,
                initial_fact_sources,
                ..
            } => {
                assert!(!task_title.is_empty());
                assert!(!node_title.is_empty());
                assert!(!node_context_summary.is_empty());
                assert_eq!(initial_success_criteria[0].id, "criterion-1");
                assert_eq!(initial_success_criteria[0].status, "open");
                assert_eq!(initial_output_contracts[0].id, "output-contract-1");
                assert_eq!(
                    initial_output_contracts[0].evidence_refs[0]
                        .artifact_ref
                        .as_deref(),
                    Some("user-request")
                );
                assert_eq!(initial_fact_sources[0].id, "fact-source-1");
                assert_eq!(
                    initial_fact_sources[0].provenance,
                    "observed_from_environment"
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_compact_result_validity_maps_normalize() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "result_validities": {
                "result-1": "accepted"
            },
            "success_criteria": {
                "criterion-1": "accepted"
            }
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("compact state_commit parses");

        match args {
            TaskSpaceControlArgs::StateCommit {
                result_validities,
                success_criteria,
                ..
            } => {
                assert_eq!(result_validities[0].result_id, "result-1");
                assert_eq!(result_validities[0].validity, "accepted");
                assert!(!result_validities[0].validity_reason.is_empty());
                assert_eq!(success_criteria[0].id, "criterion-1");
                assert_eq!(success_criteria[0].status, "satisfied");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_compact_failed_validation_aliases_normalize() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "result_validities": {
                "result-13": "failed"
            },
            "success_criteria": {
                "criterion-1": "failed"
            },
            "decisions": [
                "Local validation failure is infrastructure-specific."
            ]
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("compact failed state_commit parses");

        match args {
            TaskSpaceControlArgs::StateCommit {
                result_validities,
                success_criteria,
                decisions,
                ..
            } => {
                assert_eq!(result_validities[0].result_id, "result-13");
                assert_eq!(result_validities[0].validity, "invalid");
                assert_eq!(success_criteria[0].id, "criterion-1");
                assert_eq!(success_criteria[0].status, "questioned");
                assert_eq!(decisions[0].id, "decision-1");
                assert_eq!(decisions[0].decision_kind, "validation");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_compact_local_infra_fact_string_normalizes() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "result_validities": {
                "result-13": "invalid_infrastructure_failure"
            },
            "facts": [
                "E_ACCESSDENIED indicates Bash/Service is not available on this Windows host."
            ],
            "next_outline": "Stop retrying bash diagnostics and close as local validation infra."
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("compact local infra state_commit parses");

        match args {
            TaskSpaceControlArgs::StateCommit {
                result_validities,
                facts,
                ..
            } => {
                assert_eq!(result_validities[0].result_id, "result-13");
                assert_eq!(result_validities[0].validity, "invalid");
                assert_eq!(facts[0].claim_id, "fact-1");
                assert!(facts[0].statement.contains("E_ACCESSDENIED"));
                assert_eq!(
                    facts[0].evidence_refs[0].artifact_ref.as_deref(),
                    Some("taskspace-state-commit")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_result_validity_array_missing_reason_normalizes() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "result_validities": [{
                "result_id": "result-1",
                "validity": "accepted"
            }]
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit parses");

        match args {
            TaskSpaceControlArgs::StateCommit {
                result_validities, ..
            } => {
                assert_eq!(result_validities[0].result_id, "result-1");
                assert!(!result_validities[0].validity_reason.is_empty());
                assert_eq!(
                    result_validities[0].evidence_refs[0].result_id.as_deref(),
                    Some("result-1")
                );
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_object_sections_missing_descriptions_normalize() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "success_criteria": [{
                "id": "sc-validation",
                "kind": "test",
                "status": "satisfied"
            }],
            "output_contracts": [{
                "id": "oc-result"
            }],
            "fact_sources": [{
                "id": "fs-observed"
            }]
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit object sections parse");

        match args {
            TaskSpaceControlArgs::StateCommit {
                success_criteria,
                output_contracts,
                fact_sources,
                ..
            } => {
                assert_eq!(
                    success_criteria[0].description,
                    "test sc-validation is satisfied"
                );
                assert_eq!(
                    output_contracts[0].description,
                    "output_contracts oc-result"
                );
                assert_eq!(fact_sources[0].description, "fact_sources fs-observed");
                assert_eq!(fact_sources[0].provenance, "observed_from_environment");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_success_criterion_object_without_status_defaults_open() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "schema_version": "taskspace-state-commit-v1",
            "success_criteria": [{
                "id": "sc-new",
                "kind": "test",
                "description": "New validation criterion"
            }]
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit success criterion parses");

        match args {
            TaskSpaceControlArgs::StateCommit {
                success_criteria, ..
            } => {
                assert_eq!(success_criteria.len(), 1);
                assert_eq!(success_criteria[0].id, "sc-new");
                assert_eq!(success_criteria[0].status, "open");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn smoke_style_success_criteria_aliases_normalize() {
        let raw = serde_json::json!({
            "action": "record_success_criteria",
            "payload": {
                "success_criteria": [
                    "All tests pass",
                    {"description": "Diagnostic script runs without error"},
                    {"id": "diag-runs", "kind": "command_pass", "description": "Diagnostic runs"}
                ]
            }
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("success criteria aliases parse");

        match args {
            TaskSpaceControlArgs::RecordSuccessCriteria { criteria } => {
                assert_eq!(criteria.len(), 3);
                assert_eq!(criteria[0].id, "criterion-1");
                assert_eq!(criteria[0].kind, "test");
                assert_eq!(criteria[1].id, "criterion-2");
                assert_eq!(criteria[1].kind, "test");
                assert_eq!(criteria[2].id, "diag-runs");
                assert_eq!(criteria[2].kind, "test");
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn smoke_style_contract_and_fact_source_aliases_normalize() {
        let contract = serde_json::json!({
            "action": "record_output_contract",
            "payload": {
                "output_contracts": ["Fixed source file", "Passing test suite"],
                "refs": ["README.md", "tests/test_large_output_demo.py"]
            }
        });
        let normalized_contract =
            normalize_taskspace_arguments(&contract.to_string()).expect("normalize contract");
        let contract_args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized_contract).expect("contract aliases parse");
        match contract_args {
            TaskSpaceControlArgs::RecordOutputContract {
                output_contract_id,
                output_contract_kind,
                description,
                evidence_refs,
            } => {
                assert_eq!(output_contract_id, "output-contract-1");
                assert_eq!(output_contract_kind, "artifact");
                assert!(description.contains("Passing test suite"));
                assert_eq!(evidence_refs.len(), 2);
            }
            other => panic!("unexpected args: {other:?}"),
        }

        let fact_source = serde_json::json!({
            "action": "record_fact_source",
            "payload": {
                "fact_sources": ["README.md", "tests/"],
                "provenance": "file",
                "refs": ["README.md"]
            }
        });
        let normalized_fact_source =
            normalize_taskspace_arguments(&fact_source.to_string()).expect("normalize source");
        let fact_source_args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized_fact_source).expect("source aliases parse");
        match fact_source_args {
            TaskSpaceControlArgs::RecordFactSource {
                fact_source_id,
                provenance,
                description,
                evidence_refs,
            } => {
                assert_eq!(fact_source_id, "fact-source-1");
                assert_eq!(provenance, "observed_from_environment");
                assert!(description.contains("README.md"));
                assert_eq!(evidence_refs.len(), 1);
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn state_commit_contract_and_source_evidence_defaults_normalize() {
        let raw = serde_json::json!({
            "action": "state_commit",
            "schema_version": "taskspace-state-commit-v1",
            "output_contracts": [{
                "output_contract_id": "oc-1",
                "output_contract_kind": "artifact",
                "description": "Modified implementation files"
            }],
            "fact_sources": [{
                "fact_source_id": "fs-1",
                "provenance": "file",
                "description": "README and tests"
            }]
        });
        let normalized = normalize_taskspace_arguments(&raw.to_string()).expect("normalize");
        let args: TaskSpaceControlArgs =
            serde_json::from_str(&normalized).expect("state_commit normalized args parse");

        match args {
            TaskSpaceControlArgs::StateCommit {
                output_contracts,
                fact_sources,
                ..
            } => {
                assert_eq!(output_contracts.len(), 1);
                assert_eq!(output_contracts[0].evidence_refs.len(), 1);
                assert_eq!(
                    output_contracts[0].evidence_refs[0].artifact_ref.as_deref(),
                    Some("user-request")
                );
                assert_eq!(fact_sources.len(), 1);
                assert_eq!(fact_sources[0].provenance, "observed_from_environment");
                assert_eq!(fact_sources[0].evidence_refs.len(), 1);
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }

    #[test]
    fn read_output_ref_parses_grep_request() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "read_output_ref",
            "output_ref": "output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "mode": "grep",
            "pattern": "needle",
            "max_bytes": 128
        }))
        .expect("read_output_ref args parse");

        match args {
            TaskSpaceControlArgs::ReadOutputRef {
                output_ref,
                mode,
                pattern,
                max_bytes,
                ..
            } => {
                assert!(output_ref.starts_with("output-ref://sha256/"));
                assert_eq!(mode, "grep");
                assert_eq!(pattern.as_deref(), Some("needle"));
                assert_eq!(max_bytes, Some(128));
            }
            other => panic!("unexpected args: {other:?}"),
        }
    }
}
