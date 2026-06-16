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
        let args: TaskSpaceControlArgs = parse_arguments(&arguments)?;
        let message = match args {
            TaskSpaceControlArgs::StartTask {
                task_title,
                task_objective,
                node_kind,
                initial_success_criteria,
                node_title,
                node_context_summary,
                bind_current,
            } => {
                let node_kind = parse_node_kind("node_kind", &node_kind)?;
                let (task_id, map_id, node_id) = session
                    .start_action_map_task_for_main_with_kind_and_criteria(
                        &turn,
                        node_kind,
                        task_title,
                        task_objective,
                        convert_success_criteria(initial_success_criteria),
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
                let commit_id =
                    commit_id.unwrap_or_else(|| auto_state_commit_id_from_arguments(&arguments));
                let outcome = session
                    .state_commit_action_map(
                        &turn,
                        ActionMapStateCommitInput {
                            commit_id: commit_id.clone(),
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
                    "TaskSpace state_commit {}: status={} accepted_sections=[{}] rejected_sections=[{}]",
                    outcome.commit_id,
                    outcome.status.as_str(),
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
