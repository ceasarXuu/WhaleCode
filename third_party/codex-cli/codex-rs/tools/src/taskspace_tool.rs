use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;

fn node_kind_values() -> Vec<serde_json::Value> {
    vec![
        json!("inspect_code_context"),
        json!("implement_solution"),
        json!("smoke_test"),
        json!("regression_test"),
        json!("final_synthesis"),
    ]
}

fn output_contract_kind_values() -> Vec<serde_json::Value> {
    vec![
        json!("artifact"),
        json!("format"),
        json!("encoding"),
        json!("schema"),
        json!("validator"),
        json!("non_goal"),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSpaceControlToolProfile {
    Full,
    Compact,
}

fn action_values(profile: TaskSpaceControlToolProfile) -> Vec<serde_json::Value> {
    match profile {
        TaskSpaceControlToolProfile::Full => vec![
            json!("start_task"),
            json!("route_task"),
            json!("create_node"),
            json!("bind_node"),
            json!("finish_node"),
            json!("block_node"),
            json!("record_output_contract"),
            json!("record_fact_source"),
            json!("record_fact"),
            json!("record_success_criteria"),
            json!("record_open_question"),
            json!("close_open_question"),
            json!("record_decision"),
            json!("record_next_best_action"),
            json!("mark_result_validity"),
            json!("adopt_result"),
            json!("read_output_ref"),
            json!("state_commit"),
        ],
        TaskSpaceControlToolProfile::Compact => vec![
            json!("start_task"),
            json!("route_task"),
            json!("create_node"),
            json!("bind_node"),
            json!("finish_node"),
            json!("block_node"),
            json!("read_output_ref"),
        ],
    }
}

fn action_description(profile: TaskSpaceControlToolProfile) -> &'static str {
    match profile {
        TaskSpaceControlToolProfile::Full => {
            "One of: start_task, route_task, create_node, bind_node, finish_node, block_node, record_output_contract, record_fact_source, record_fact, record_success_criteria, record_open_question, close_open_question, record_decision, record_next_best_action, mark_result_validity, adopt_result, read_output_ref, state_commit. Use only for TaskSpace runtime control."
        }
        TaskSpaceControlToolProfile::Compact => {
            "Mechanical TaskSpace map actions: start_task, route_task, create_node, bind_node, finish_node, block_node, read_output_ref."
        }
    }
}

fn compact_top_level_fields() -> &'static [&'static str] {
    &[
        "action",
        "task_id",
        "task_title",
        "task_objective",
        "node_kind",
        "node_title",
        "node_context_summary",
        "kind",
        "title",
        "context_summary",
        "dependency_node_ids",
        "bind_current",
        "node_id",
        "result_summary",
        "next_node_id",
        "next_node_kind",
        "next_node_title",
        "next_node_context_summary",
        "next_dependency_node_ids",
        "blocker_summary",
        "output_ref",
        "mode",
        "start_line",
        "end_line",
        "pattern",
        "max_bytes",
    ]
}

fn evidence_ref_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "result_id".to_string(),
                JsonSchema::string(Some("Existing node result id.".to_string())),
            ),
            (
                "claim_id".to_string(),
                JsonSchema::string(Some("Existing or related claim id.".to_string())),
            ),
            (
                "fact_source_id".to_string(),
                JsonSchema::string(Some("Existing fact source id.".to_string())),
            ),
            (
                "trace_event_id".to_string(),
                JsonSchema::string(Some("Existing TaskSpace trace event id.".to_string())),
            ),
            (
                "artifact_ref".to_string(),
                JsonSchema::string(Some("Artifact path, id, or stable reference.".to_string())),
            ),
            (
                "validator_ref".to_string(),
                JsonSchema::string(Some(
                    "Validator, test, build, or check reference.".to_string(),
                )),
            ),
        ]),
        None,
        Some(false.into()),
    )
}

fn evidence_refs_schema(description: &str) -> JsonSchema {
    JsonSchema::array(evidence_ref_schema(), Some(description.to_string()))
}

fn claims_schema() -> JsonSchema {
    JsonSchema::array(
        JsonSchema::object(
            BTreeMap::from([
                (
                    "claim_id".to_string(),
                    JsonSchema::string(Some("Stable claim id.".to_string())),
                ),
                (
                    "statement".to_string(),
                    JsonSchema::string(Some("Concise claim statement.".to_string())),
                ),
                (
                    "evidence_refs".to_string(),
                    evidence_refs_schema("Evidence refs supporting this claim."),
                ),
            ]),
            Some(vec![
                "claim_id".to_string(),
                "statement".to_string(),
                "evidence_refs".to_string(),
            ]),
            Some(false.into()),
        ),
        Some("Claims carried by a result evidence package.".to_string()),
    )
}

fn success_criteria_schema(description: &str) -> JsonSchema {
    JsonSchema::array(
        JsonSchema::object(
            BTreeMap::from([
                (
                    "id".to_string(),
                    JsonSchema::string(Some("Stable criterion id such as sc-1.".to_string())),
                ),
                (
                    "kind".to_string(),
                    JsonSchema::string_enum(
                        vec![
                            json!("artifact"),
                            json!("behavior"),
                            json!("test"),
                            json!("validator"),
                            json!("compatibility"),
                            json!("performance"),
                            json!("user_visible_output"),
                        ],
                        Some("Criterion type.".to_string()),
                    ),
                ),
                (
                    "description".to_string(),
                    JsonSchema::string(Some("Concrete completion standard.".to_string())),
                ),
                (
                    "status".to_string(),
                    JsonSchema::string_enum(
                        vec![
                            json!("open"),
                            json!("satisfied"),
                            json!("questioned"),
                            json!("waived"),
                        ],
                        Some("Criterion status; use open when unsure.".to_string()),
                    ),
                ),
                (
                    "evidence_refs".to_string(),
                    evidence_refs_schema("Evidence refs supporting this criterion."),
                ),
            ]),
            Some(vec![
                "id".to_string(),
                "kind".to_string(),
                "description".to_string(),
            ]),
            Some(false.into()),
        ),
        Some(description.to_string()),
    )
}

fn state_commit_node_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "kind".to_string(),
                JsonSchema::string_enum(
                    node_kind_values(),
                    Some("Runtime node kind to create.".to_string()),
                ),
            ),
            (
                "title".to_string(),
                JsonSchema::string(Some("Human-readable node title.".to_string())),
            ),
            (
                "context_summary".to_string(),
                JsonSchema::string(Some("Concise context the node should carry.".to_string())),
            ),
            (
                "dependency_node_ids".to_string(),
                JsonSchema::array(
                    JsonSchema::string(Some("Existing upstream node id.".to_string())),
                    Some("Optional dependency node ids.".to_string()),
                ),
            ),
            (
                "bind_current".to_string(),
                JsonSchema::boolean(Some("Bind the main agent to the created node.".to_string())),
            ),
        ]),
        Some(vec![
            "kind".to_string(),
            "title".to_string(),
            "context_summary".to_string(),
        ]),
        Some(false.into()),
    )
}

fn state_commit_finished_node_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".to_string(),
                JsonSchema::string(Some("Existing current node id to finish.".to_string())),
            ),
            (
                "result_summary".to_string(),
                JsonSchema::string(Some("Concise result summary.".to_string())),
            ),
            (
                "next_node_id".to_string(),
                JsonSchema::string(Some("Optional existing next node to bind.".to_string())),
            ),
            (
                "next_node_kind".to_string(),
                JsonSchema::string_enum(
                    node_kind_values(),
                    Some("Optional kind for atomically creating the next node.".to_string()),
                ),
            ),
            (
                "next_node_title".to_string(),
                JsonSchema::string(Some("Optional title for the next node.".to_string())),
            ),
            (
                "next_node_context_summary".to_string(),
                JsonSchema::string(Some("Optional context for the next node.".to_string())),
            ),
            (
                "next_dependency_node_ids".to_string(),
                JsonSchema::array(
                    JsonSchema::string(Some("Existing upstream node id.".to_string())),
                    Some("Optional dependencies for the next node.".to_string()),
                ),
            ),
        ]),
        Some(vec!["node_id".to_string(), "result_summary".to_string()]),
        Some(false.into()),
    )
}

fn state_commit_blocker_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".to_string(),
                JsonSchema::string(Some("Existing current node id to block.".to_string())),
            ),
            (
                "blocker_summary".to_string(),
                JsonSchema::string(Some("Concrete blocker summary.".to_string())),
            ),
        ]),
        Some(vec!["node_id".to_string(), "blocker_summary".to_string()]),
        Some(false.into()),
    )
}

fn state_commit_result_validity_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "result_id".to_string(),
                JsonSchema::string(Some("Existing result id to review.".to_string())),
            ),
            (
                "validity".to_string(),
                JsonSchema::string_enum(
                    vec![
                        json!("unreviewed"),
                        json!("accepted"),
                        json!("questioned"),
                        json!("invalid"),
                    ],
                    Some("Validity state for the result evidence package.".to_string()),
                ),
            ),
            (
                "validity_reason".to_string(),
                JsonSchema::string(Some("Concise validity reason.".to_string())),
            ),
            ("claims".to_string(), claims_schema()),
            (
                "evidence_refs".to_string(),
                evidence_refs_schema("Evidence refs supporting this validity."),
            ),
            (
                "changed_artifacts".to_string(),
                JsonSchema::array(
                    JsonSchema::string(Some(
                        "Modified artifact path or output artifact.".to_string(),
                    )),
                    Some("Changed artifacts for accepted implementation results.".to_string()),
                ),
            ),
            (
                "validator_refs".to_string(),
                JsonSchema::array(
                    JsonSchema::string(Some(
                        "Validator, test, build, or check reference.".to_string(),
                    )),
                    Some("Validator refs for validation results.".to_string()),
                ),
            ),
            (
                "remaining_uncertainty".to_string(),
                JsonSchema::array(
                    JsonSchema::string(Some("Remaining uncertainty or caveat.".to_string())),
                    Some("Optional remaining uncertainty.".to_string()),
                ),
            ),
        ]),
        Some(vec![
            "result_id".to_string(),
            "validity".to_string(),
            "validity_reason".to_string(),
        ]),
        Some(false.into()),
    )
}

fn state_commit_result_adoption_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "result_id".to_string(),
                JsonSchema::string(Some("Existing result id to adopt.".to_string())),
            ),
            (
                "adopted_by_facts".to_string(),
                JsonSchema::array(JsonSchema::string(None), Some("Fact ids.".to_string())),
            ),
            (
                "adopted_by_hypotheses".to_string(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Hypothesis ids.".to_string()),
                ),
            ),
            (
                "adopted_by_decisions".to_string(),
                JsonSchema::array(JsonSchema::string(None), Some("Decision ids.".to_string())),
            ),
            (
                "adopted_by_criteria".to_string(),
                JsonSchema::array(JsonSchema::string(None), Some("Criterion ids.".to_string())),
            ),
            (
                "adopted_by_nodes".to_string(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Follow-up node ids.".to_string()),
                ),
            ),
        ]),
        Some(vec!["result_id".to_string()]),
        Some(false.into()),
    )
}

fn fact_sources_schema(description: &str) -> JsonSchema {
    JsonSchema::array(
        JsonSchema::object(
            BTreeMap::from([
                (
                    "fact_source_id".to_string(),
                    JsonSchema::string(Some("Stable fact source id.".to_string())),
                ),
                (
                    "provenance".to_string(),
                    JsonSchema::string_enum(
                        vec![
                            json!("observed_from_environment"),
                            json!("provided_by_user"),
                            json!("generated_for_test_only"),
                            json!("inferred"),
                            json!("unknown"),
                        ],
                        Some("Fact source provenance.".to_string()),
                    ),
                ),
                (
                    "description".to_string(),
                    JsonSchema::string(Some("Concrete source description.".to_string())),
                ),
                (
                    "evidence_refs".to_string(),
                    evidence_refs_schema("Evidence refs for this source."),
                ),
            ]),
            Some(vec![
                "fact_source_id".to_string(),
                "provenance".to_string(),
                "description".to_string(),
                "evidence_refs".to_string(),
            ]),
            Some(false.into()),
        ),
        Some(description.to_string()),
    )
}

fn output_contracts_schema(description: &str) -> JsonSchema {
    JsonSchema::array(
        JsonSchema::object(
            BTreeMap::from([
                (
                    "output_contract_id".to_string(),
                    JsonSchema::string(Some("Stable output contract id.".to_string())),
                ),
                (
                    "kind".to_string(),
                    JsonSchema::string_enum(
                        output_contract_kind_values(),
                        Some("Output contract kind.".to_string()),
                    ),
                ),
                (
                    "description".to_string(),
                    JsonSchema::string(Some("Concrete output contract description.".to_string())),
                ),
                (
                    "evidence_refs".to_string(),
                    evidence_refs_schema("Evidence refs for this contract."),
                ),
            ]),
            Some(vec![
                "output_contract_id".to_string(),
                "kind".to_string(),
                "description".to_string(),
                "evidence_refs".to_string(),
            ]),
            Some(false.into()),
        ),
        Some(description.to_string()),
    )
}

fn decisions_schema(description: &str) -> JsonSchema {
    JsonSchema::array(
        JsonSchema::object(
            BTreeMap::from([
                (
                    "decision_id".to_string(),
                    JsonSchema::string(Some("Stable decision id.".to_string())),
                ),
                (
                    "decision_kind".to_string(),
                    JsonSchema::string(Some("Decision kind.".to_string())),
                ),
                (
                    "decision".to_string(),
                    JsonSchema::string(Some("Decision statement.".to_string())),
                ),
                (
                    "rationale".to_string(),
                    JsonSchema::string(Some("Decision rationale.".to_string())),
                ),
                (
                    "depends_on_results".to_string(),
                    JsonSchema::array(JsonSchema::string(None), Some("Result ids.".to_string())),
                ),
                (
                    "depends_on_facts".to_string(),
                    JsonSchema::array(JsonSchema::string(None), Some("Fact ids.".to_string())),
                ),
                (
                    "resolves_questions".to_string(),
                    JsonSchema::array(JsonSchema::string(None), Some("Question ids.".to_string())),
                ),
                (
                    "supports_criteria".to_string(),
                    JsonSchema::array(JsonSchema::string(None), Some("Criterion ids.".to_string())),
                ),
                (
                    "risks".to_string(),
                    JsonSchema::array(
                        JsonSchema::string(None),
                        Some("Risk summaries.".to_string()),
                    ),
                ),
            ]),
            Some(vec![
                "decision_id".to_string(),
                "decision_kind".to_string(),
                "decision".to_string(),
                "rationale".to_string(),
            ]),
            Some(false.into()),
        ),
        Some(description.to_string()),
    )
}

fn next_best_action_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "action_summary".to_string(),
                JsonSchema::string(Some("Minimal next high-value action.".to_string())),
            ),
            (
                "reason".to_string(),
                JsonSchema::string(Some("Why this action is next.".to_string())),
            ),
            (
                "expected_artifact".to_string(),
                JsonSchema::string(Some("Expected artifact from this action.".to_string())),
            ),
            (
                "blocked_by".to_string(),
                JsonSchema::array(JsonSchema::string(None), Some("Blocker ids.".to_string())),
            ),
        ]),
        Some(vec!["action_summary".to_string(), "reason".to_string()]),
        Some(false.into()),
    )
}

pub fn create_taskspace_control_tool() -> ToolSpec {
    create_taskspace_control_tool_with_profile(TaskSpaceControlToolProfile::Full)
}

pub fn create_taskspace_control_tool_with_profile(
    profile: TaskSpaceControlToolProfile,
) -> ToolSpec {
    let mut properties = BTreeMap::from([
        (
            "action".to_string(),
            JsonSchema::string_enum(
                action_values(profile),
                Some(action_description(profile).to_string()),
            ),
        ),
        (
            "commit_id".to_string(),
            JsonSchema::string(Some(
                "Optional for state_commit. Stable checkpoint id; runtime auto-generates one when omitted."
                    .to_string(),
            )),
        ),
        (
            "schema_version".to_string(),
            JsonSchema::string_enum(
                vec![json!("taskspace-state-commit-v1")],
                Some("Required for state_commit: taskspace-state-commit-v1.".to_string()),
            ),
        ),
        (
            "active_node_id".to_string(),
            JsonSchema::string(Some(
                "Optional for state_commit. Current node id this checkpoint is about."
                    .to_string(),
            )),
        ),
        (
            "nodes".to_string(),
            JsonSchema::array(
                state_commit_node_schema(),
                Some("state_commit section: nodes to create.".to_string()),
            ),
        ),
        (
            "finished_nodes".to_string(),
            JsonSchema::array(
                state_commit_finished_node_schema(),
                Some("state_commit section: current nodes to finish.".to_string()),
            ),
        ),
        (
            "blockers".to_string(),
            JsonSchema::array(
                state_commit_blocker_schema(),
                Some("state_commit section: nodes to block.".to_string()),
            ),
        ),
        (
            "result_validities".to_string(),
            JsonSchema::array(
                state_commit_result_validity_schema(),
                Some("state_commit section: result validity reviews.".to_string()),
            ),
        ),
        (
            "result_adoptions".to_string(),
            JsonSchema::array(
                state_commit_result_adoption_schema(),
                Some("state_commit section: result adoption links.".to_string()),
            ),
        ),
        (
            "success_criteria".to_string(),
            success_criteria_schema("state_commit section: success criteria updates."),
        ),
        (
            "output_contracts".to_string(),
            output_contracts_schema("state_commit section: output contract updates."),
        ),
        (
            "fact_sources".to_string(),
            fact_sources_schema("state_commit section: fact source updates."),
        ),
        (
            "facts".to_string(),
            claims_schema(),
        ),
        (
            "decisions".to_string(),
            decisions_schema("state_commit section: decision records."),
        ),
        (
            "next_best_action".to_string(),
            next_best_action_schema(),
        ),
        (
            "task_id".to_string(),
            JsonSchema::string(Some(
                "Required for route_task. Existing task id from the TaskSpace task inventory."
                    .to_string(),
            )),
        ),
        (
            "task_title".to_string(),
            JsonSchema::string(Some(
                "Required for start_task. Human-readable title for the new semantic task."
                    .to_string(),
            )),
        ),
        (
            "task_objective".to_string(),
            JsonSchema::string(Some(
                "Optional for start_task. Concise objective for the new semantic task."
                    .to_string(),
            )),
        ),
        (
            "node_kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some(
                    "Required for start_task. Runtime kind for the first node."
                        .to_string(),
                ),
            ),
        ),
        (
            "node_title".to_string(),
            JsonSchema::string(Some(
                "Required for start_task. Human-readable title for the first concrete node."
                    .to_string(),
            )),
        ),
        (
            "node_context_summary".to_string(),
            JsonSchema::string(Some(
                "Required for start_task. Concise context the first node should carry."
                    .to_string(),
            )),
        ),
        (
            "initial_success_criteria".to_string(),
            success_criteria_schema(
                "Optional for start_task. Provide initial explicit completion standards when available; ordinary work is blocked until criteria exist.",
            ),
        ),
        (
            "kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some("Required only for create_node. Runtime kind for this node.".to_string()),
            ),
        ),
        (
            "title".to_string(),
            JsonSchema::string(Some(
                "Required for create_node. Human-readable node title.".to_string(),
            )),
        ),
        (
            "context_summary".to_string(),
            JsonSchema::string(Some(
                "Required for create_node. Concise context the node should carry.".to_string(),
            )),
        ),
        (
            "dependency_node_ids".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing upstream node id.".to_string())),
                Some("Optional dependency node ids for create_node.".to_string()),
            ),
        ),
        (
            "bind_current".to_string(),
            JsonSchema::boolean(Some(
                "For start_task or create_node, bind the main agent to the new node immediately."
                    .to_string(),
            )),
        ),
        (
            "node_id".to_string(),
            JsonSchema::string(Some(
                "Required for bind_node, finish_node, and block_node. Existing node id."
                    .to_string(),
            )),
        ),
        (
            "result_summary".to_string(),
            JsonSchema::string(Some(
                "Required for finish_node. Concise result summary that should stay in the node context."
                    .to_string(),
            )),
        ),
        (
            "next_node_id".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node. Existing node id to bind after the result is recorded."
                    .to_string(),
            )),
        ),
        (
            "next_node_kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some(
                    "Optional for finish_node. When provided with next_node_title and next_node_context_summary, create and bind a new next node atomically."
                        .to_string(),
                ),
            ),
        ),
        (
            "next_node_title".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node with next_node_kind. Human-readable title for the new next node."
                    .to_string(),
            )),
        ),
        (
            "next_node_context_summary".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node with next_node_kind. Concise context for the new next node."
                    .to_string(),
            )),
        ),
        (
            "next_dependency_node_ids".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing upstream node id.".to_string())),
                Some(
                    "Optional for finish_node with next_node_kind. Dependency node ids for the new next node."
                        .to_string(),
                ),
            ),
        ),
        (
            "blocker_summary".to_string(),
            JsonSchema::string(Some(
                "Required for block_node. Concise blocker summary that should stay in the node context."
                    .to_string(),
            )),
        ),
        (
            "output_contract_id".to_string(),
            JsonSchema::string(Some(
                "Required for record_output_contract. Stable output contract id.".to_string(),
            )),
        ),
        (
            "fact_source_id".to_string(),
            JsonSchema::string(Some(
                "Required for record_fact_source and evidence refs that cite a fact source."
                    .to_string(),
            )),
        ),
        (
            "provenance".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("observed_from_environment"),
                    json!("provided_by_user"),
                    json!("generated_for_test_only"),
                    json!("inferred"),
                    json!("unknown"),
                ],
                Some("Required for record_fact_source.".to_string()),
            ),
        ),
        (
            "description".to_string(),
            JsonSchema::string(Some(
                "Required for record_output_contract and record_fact_source.".to_string(),
            )),
        ),
        (
            "output_contract_kind".to_string(),
            JsonSchema::string_enum(
                output_contract_kind_values(),
                Some(
                    "Required for record_output_contract. One of: artifact, format, encoding, schema, validator, non_goal."
                        .to_string(),
                ),
            ),
        ),
        (
            "claim_id".to_string(),
            JsonSchema::string(Some("Required for record_fact.".to_string())),
        ),
        (
            "statement".to_string(),
            JsonSchema::string(Some("Required for record_fact.".to_string())),
        ),
        (
            "criteria".to_string(),
            success_criteria_schema("Required for record_success_criteria."),
        ),
        (
            "question_id".to_string(),
            JsonSchema::string(Some(
                "Required for record_open_question and close_open_question.".to_string(),
            )),
        ),
        (
            "question".to_string(),
            JsonSchema::string(Some(
                "Required for record_open_question. Concrete unresolved question."
                    .to_string(),
            )),
        ),
        (
            "reason".to_string(),
            JsonSchema::string(Some(
                "Required for record_open_question and record_next_best_action.".to_string(),
            )),
        ),
        (
            "blocking".to_string(),
            JsonSchema::boolean(Some(
                "Optional for record_open_question. True if this must be resolved before final synthesis."
                    .to_string(),
            )),
        ),
        (
            "opened_by_node_id".to_string(),
            JsonSchema::string(Some(
                "Optional for record_open_question. Node that surfaced this question."
                    .to_string(),
            )),
        ),
        (
            "resolution".to_string(),
            JsonSchema::string(Some(
                "Required for close_open_question. Evidence-backed resolution.".to_string(),
            )),
        ),
        (
            "closed_by_result_id".to_string(),
            JsonSchema::string(Some(
                "Optional for close_open_question. Result that resolved this question."
                    .to_string(),
            )),
        ),
        (
            "decision_id".to_string(),
            JsonSchema::string(Some(
                "Required for record_decision. Stable decision id such as d-1.".to_string(),
            )),
        ),
        (
            "decision_kind".to_string(),
            JsonSchema::string(Some(
                "Required for record_decision, for example design, patch, validation, or synthesis."
                    .to_string(),
            )),
        ),
        (
            "decision".to_string(),
            JsonSchema::string(Some("Required for record_decision.".to_string())),
        ),
        (
            "rationale".to_string(),
            JsonSchema::string(Some("Required for record_decision.".to_string())),
        ),
        (
            "depends_on_results".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing result id.".to_string())),
                Some("Optional for record_decision.".to_string()),
            ),
        ),
        (
            "depends_on_facts".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing ledger fact id.".to_string())),
                Some("Optional for record_decision.".to_string()),
            ),
        ),
        (
            "resolves_questions".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing open question id.".to_string())),
                Some("Optional for record_decision.".to_string()),
            ),
        ),
        (
            "supports_criteria".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing success criterion id.".to_string())),
                Some("Optional for record_decision.".to_string()),
            ),
        ),
        (
            "risks".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Risk summary or risk id.".to_string())),
                Some("Optional for record_decision.".to_string()),
            ),
        ),
        (
            "action_summary".to_string(),
            JsonSchema::string(Some(
                "Required for record_next_best_action. Minimal next high-value action."
                    .to_string(),
            )),
        ),
        (
            "expected_artifact".to_string(),
            JsonSchema::string(Some(
                "Optional for record_next_best_action. Expected artifact from this action."
                    .to_string(),
            )),
        ),
        (
            "blocked_by".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Question, blocker, or dependency id.".to_string())),
                Some("Optional for record_next_best_action.".to_string()),
            ),
        ),
        (
            "result_id".to_string(),
            JsonSchema::string(Some(
                "Required for mark_result_validity and adopt_result.".to_string(),
            )),
        ),
        (
            "validity".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("unreviewed"),
                    json!("accepted"),
                    json!("questioned"),
                    json!("invalid"),
                ],
                Some(
                    "Required for mark_result_validity. Use accepted only when claims and evidence_refs are non-empty; use unreviewed or questioned when no validation or source evidence is available."
                        .to_string(),
                ),
            ),
        ),
        (
            "validity_reason".to_string(),
            JsonSchema::string(Some(
                "Required for mark_result_validity. Concise reason for the validity state."
                    .to_string(),
            )),
        ),
        (
            "claims".to_string(),
            claims_schema(),
        ),
        (
            "evidence_refs".to_string(),
            evidence_refs_schema(
                "Required for record_output_contract, record_fact_source, record_fact, and mark_result_validity.",
            ),
        ),
        (
            "changed_artifacts".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Modified artifact path or stable output artifact reference.".to_string())),
                Some("For accepted implementation results, list every modified file or final output artifact here. evidence_refs alone are treated as supporting/source evidence, not as final changed artifacts.".to_string()),
            ),
        ),
        (
            "validator_refs".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Validator, test, build, or check reference.".to_string())),
                Some("Optional for mark_result_validity.".to_string()),
            ),
        ),
        (
            "remaining_uncertainty".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Remaining uncertainty or caveat.".to_string())),
                Some("Optional for mark_result_validity.".to_string()),
            ),
        ),
        (
            "adopted_by_facts".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing fact id.".to_string())),
                Some("Optional for adopt_result.".to_string()),
            ),
        ),
        (
            "adopted_by_hypotheses".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing hypothesis id.".to_string())),
                Some("Optional for adopt_result.".to_string()),
            ),
        ),
        (
            "adopted_by_decisions".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing decision id.".to_string())),
                Some("Optional for adopt_result.".to_string()),
            ),
        ),
        (
            "adopted_by_criteria".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing success criterion id.".to_string())),
                Some("Optional for adopt_result.".to_string()),
            ),
        ),
        (
            "adopted_by_nodes".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing node id.".to_string())),
                Some("Optional for adopt_result.".to_string()),
            ),
        ),
        (
            "output_ref".to_string(),
            JsonSchema::string(Some(
                "Required for read_output_ref. Must use output-ref://sha256/<64 hex> from OutputReferenceV1 artifact_ref."
                    .to_string(),
            )),
        ),
        (
            "mode".to_string(),
            JsonSchema::string_enum(
                vec![json!("head"), json!("tail"), json!("line_range"), json!("grep")],
                Some(
                    "Required for read_output_ref. Choose a bounded slice mode."
                        .to_string(),
                ),
            ),
        ),
        (
            "start_line".to_string(),
            JsonSchema::integer(Some(
                "Required for read_output_ref when mode=line_range. 1-based inclusive start line."
                    .to_string(),
            )),
        ),
        (
            "end_line".to_string(),
            JsonSchema::integer(Some(
                "Required for read_output_ref when mode=line_range. 1-based inclusive end line."
                    .to_string(),
            )),
        ),
        (
            "pattern".to_string(),
            JsonSchema::string(Some(
                "Required for read_output_ref when mode=grep. Literal substring to match."
                    .to_string(),
            )),
        ),
        (
            "max_bytes".to_string(),
            JsonSchema::integer(Some(
                "Optional for read_output_ref. Returned slice byte budget, clamped to a 16KB hard maximum."
                    .to_string(),
            )),
        ),
    ]);
    if profile == TaskSpaceControlToolProfile::Compact {
        let allowed = compact_top_level_fields();
        properties.retain(|key, _| allowed.contains(&key.as_str()));
    }

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".to_string(),
        description: if profile == TaskSpaceControlToolProfile::Compact {
            r#"Mandatory mechanical task-map tool used only while TaskSpace is enabled.

TaskSpace reorganizes the ordinary conversation into a task map. The Agent owns all task semantics and decides when and how work advances. Runtime only validates map ids, node status, bindings, leases, tool/result pairing, and other hard protocol state.

Actions:
- `start_task`: create a task map and first current node when no task exists. Include `task_title`, `task_objective`, `node_kind`, `node_title`, and `node_context_summary`.
- `route_task`: select an existing task by `task_id`.
- `create_node`: create a node with `kind`, `title`, `context_summary`, optional `dependency_node_ids`, and optional `bind_current`.
- `bind_node`: bind the Agent to an existing node by `node_id`.
- `finish_node`: record `result_summary` and complete the current node. It may also bind `next_node_id`, or atomically create and bind a next node with `next_node_kind`, `next_node_title`, `next_node_context_summary`, and optional `next_dependency_node_ids`.
- `block_node`: record `blocker_summary` and block the current node.
- `read_output_ref`: read a bounded part of `output_ref` using `head`, `tail`, `line_range`, or `grep`.

Ordinary tools require a current node binding. A final response requires no running current node. The runtime reports hard state errors exactly and does not choose the Agent's next action, infer task strategy, or reinterpret tool feedback."#
                .to_string()
        } else {
            r#"Internal TaskSpace control tool.

Use this only when TaskSpace is enabled and you need to update task-map structure or cognitive state before ordinary work.

The main agent is the TaskSpace problem-state and model manager. Use this tool to keep the task's current model explicit: route or create the semantic task, keep work bound to concrete nodes, record success criteria, record open questions, record decisions, update next best action, record output contracts, record fact sources, record active facts, mark result validity before relying on node or subagent output, and record result adoption when accepted evidence is used by facts, decisions, criteria, or follow-up nodes.

Runtime preflight blocks ordinary tools and spawn_agent until the active task has at least one success criterion, one output contract, and one fact source. After finish_node, block_node, or subagent completion records a node-level result, runtime blocks further ordinary tools and spawn_agent until mark_result_validity records whether that result is accepted, questioned, or invalid.

Supported actions:
- `start_task`: create a new semantic task, its active task path, and the first concrete node. Use this when the current user request does not belong to an existing task in the TaskSpace task inventory. Must include `node_kind`. Include `initial_success_criteria` whenever the user request gives enough information to define completion; if omitted, runtime will block ordinary work until `record_success_criteria` is called.
- `route_task`: switch the active task path to an existing task chosen by the agent from the TaskSpace task inventory. Runtime validates the id but does not perform semantic matching.
- `create_node`: create a concrete node in the active task path. This requires an existing active task path; use `start_task` first when the current request starts a new semantic task. Must include `kind` with one of: inspect_code_context, implement_solution, smoke_test, regression_test, final_synthesis. BaseMap candidate nodes are guidance, not automatic graph nodes.
- `bind_node`: bind the main agent's next ordinary action to an existing ready or blocked node that is not held by a subagent.
- `finish_node`: record the current main node's result, mark it completed, and optionally bind an existing next node with `next_node_id` or create and bind a new next node with `next_node_kind`, `next_node_title`, and `next_node_context_summary`.
- `block_node`: record why the current main node cannot proceed and mark it blocked.
- `record_output_contract`: record a task-level output contract with stable `output_contract_id`, `output_contract_kind`, `description`, and `evidence_refs`. Use one of: artifact, format, encoding, schema, validator, non_goal. Do not use node kinds here.
- `record_fact_source`: record a task-level data source with stable `fact_source_id`, `provenance`, `description`, and `evidence_refs`.
- `record_fact`: record an active task fact with stable `claim_id`, `statement`, and `evidence_refs`. Runtime only accepts facts supported by an accepted result or observed/provided fact source.
- `record_success_criteria`: record explicit task completion standards. Ordinary work is blocked until at least one success criterion exists.
- `record_open_question`: record an unresolved problem-state gap, with `blocking=true` if it must be answered before final synthesis.
- `close_open_question`: close an open question with a concrete resolution and evidence refs.
- `record_decision`: record a design, patch, validation, or synthesis decision with the facts, results, questions, and criteria it depends on.
- `record_next_best_action`: record the current smallest high-value action implied by the problem state.
- `mark_result_validity`: update an existing node result's evidence package. `accepted` requires non-empty claims and evidence refs. If no validation/source evidence was produced, use `unreviewed` or `questioned`; never call `accepted` with empty claims.
- `adopt_result`: record how an accepted or questioned result is actually used by facts, decisions, criteria, hypotheses, or follow-up nodes. `record_fact` and `record_decision` auto-adopt referenced results; use this action for criteria/node adoption or to repair missing adoption links.
- `read_output_ref`: retrieve a bounded slice from an `OutputReferenceV1` artifact_ref. Use this instead of asking for a large raw stdout/stderr body to be replayed. Supports `mode=head`, `mode=tail`, `mode=line_range` with `start_line` and `end_line`, and `mode=grep` with `pattern`. `max_bytes` is clamped to a 16KB hard maximum.
- `state_commit`: batch related lifecycle and cognitive updates in one checkpoint. Must include `schema_version=taskspace-state-commit-v1`. Use top-level section fields: `nodes`, `finished_nodes`, `blockers`, `result_validities`, `result_adoptions`, `success_criteria`, `output_contracts`, `fact_sources`, `facts`, `decisions`, and `next_best_action`. Prefer this over several single-record actions when closing an inspect/implement/test phase.

Cognitive-state rules:
- After start_task or route_task, record user-stated acceptance criteria, output format, schema, validator, artifact, and non-goal requirements as success criteria and output contracts before ordinary work. Use evidence_refs where available; artifact_ref may cite the current user request, README/spec/test/source path, or expected artifact.
- Use open questions for real unknowns that affect the task model. Use decisions before patching or final synthesis so the task path records why this direction is justified.
- Record user-provided facts, observed environment facts, and validator/test outputs as fact sources before turning them into active facts. Use non-empty evidence_refs; artifact_ref or validator_ref is acceptable before any node result exists.
- `generated_for_test_only`, `inferred`, and `unknown` provenance may guide investigation but must not anchor active facts or final user claims unless rechecked against observed or user-provided evidence.
- Treat subagent and node results as evidence packages, not final truth. After a node-level result is recorded, capture claims, evidence refs, changed artifacts, validator refs, and remaining uncertainty through `mark_result_validity` before using it, spawning follow-up work, running ordinary tools, or answering the user. For accepted implementation results, put modified files in `changed_artifacts`; `evidence_refs` alone mean supporting/source evidence.
- Decisions must cite at least one concrete dependency. Do not base a decision on invalid or unreviewed results. A questioned result cannot be the only dependency for a decision.
- Direct trace events are internal audit records. Do not expose TaskSpace, task, map, node, lease, or subagent protocol terms to the user unless the user is explicitly debugging TaskSpace.

Node kind selection:
- Use `inspect_code_context` for read-only investigation and subagent investigation nodes.
- Use `implement_solution` for code, test, configuration, or documentation edits.
- Use `smoke_test` or `regression_test` before running test/build/lint commands.
- If validation fails and edits are needed, record the test result, switch to `implement_solution` for the fix, then switch back to a test node for the rerun.
- Use `final_synthesis` only for answer-only final wrap-up after accepted validation; do not edit, test, build, spawn agents, or call ordinary tools from final_synthesis. Final user-facing text must not expose internal TaskSpace orchestration terms such as task, map, node, subagent, spawn, lease, final_synthesis, or taskspace_control unless the user explicitly asks to inspect TaskSpace internals. If the user asks how work was organized, describe visible phases, files, tests, and outcomes only; never mention hidden execution roles or words such as subagent, explorer, agent, delegated, parallel, evidence track, fan-out, or spawn.
- `custom` is reserved for restored legacy nodes and is not valid for live node creation.

Do not expose this tool's internal map/node terminology to the user unless debugging TaskSpace itself.
"#
            .to_string()
        },
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["action".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskspace_control_exposes_mvp_cognitive_actions_without_promotion() {
        let value = serde_json::to_value(create_taskspace_control_tool())
            .expect("taskspace tool serializes");
        let action_enum = value["parameters"]["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum");
        let actions = action_enum
            .iter()
            .map(|value| value.as_str().expect("string enum"))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![
                "start_task",
                "route_task",
                "create_node",
                "bind_node",
                "finish_node",
                "block_node",
                "record_output_contract",
                "record_fact_source",
                "record_fact",
                "record_success_criteria",
                "record_open_question",
                "close_open_question",
                "record_decision",
                "record_next_best_action",
                "mark_result_validity",
                "adopt_result",
                "read_output_ref",
                "state_commit",
            ]
        );
        assert!(!actions.contains(&"promote_taskspace"));
    }

    #[test]
    fn taskspace_control_exposes_mvp_cognitive_fields() {
        let value = serde_json::to_value(create_taskspace_control_tool())
            .expect("taskspace tool serializes");
        let properties = value["parameters"]["properties"]
            .as_object()
            .expect("properties object");

        for present_field in [
            "output_contract_id",
            "fact_source_id",
            "provenance",
            "claim_id",
            "statement",
            "initial_success_criteria",
            "criteria",
            "question_id",
            "question",
            "blocking",
            "decision_id",
            "decision_kind",
            "depends_on_results",
            "supports_criteria",
            "action_summary",
            "expected_artifact",
            "output_contract_kind",
            "claims",
            "evidence_refs",
            "result_id",
            "validity",
            "validity_reason",
            "adopted_by_facts",
            "adopted_by_decisions",
            "adopted_by_criteria",
            "adopted_by_nodes",
            "output_ref",
            "mode",
            "start_line",
            "end_line",
            "pattern",
            "max_bytes",
            "commit_id",
            "schema_version",
            "active_node_id",
            "nodes",
            "finished_nodes",
            "blockers",
            "result_validities",
            "result_adoptions",
            "success_criteria",
            "output_contracts",
            "fact_sources",
            "facts",
            "decisions",
            "next_best_action",
        ] {
            assert!(
                properties.contains_key(present_field),
                "{present_field} should be exposed by the MVP cognitive control schema"
            );
        }
        assert!(!properties.contains_key("promotion_not_in_mvp"));
        assert!(!properties.contains_key("promote_taskspace"));
        assert_eq!(
            properties["evidence_refs"]["items"]["properties"]["fact_source_id"]["type"],
            "string"
        );
        let description = value["description"]
            .as_str()
            .expect("tool description is exposed");
        assert!(description.contains("problem-state and model manager"));
        assert!(description.contains("record success criteria"));
        assert!(description.contains("Runtime preflight blocks ordinary tools"));
        assert!(description.contains("Treat subagent and node results as evidence packages"));
        assert!(description.contains("record result adoption"));
        assert!(description.contains("read_output_ref"));
        assert!(description.contains("OutputReferenceV1"));
        assert!(description.contains("Decisions must cite at least one concrete dependency"));
        assert!(description.contains("generated_for_test_only"));
        assert!(description.contains("Do not use node kinds here"));
        assert!(description.contains("never call `accepted` with empty claims"));
        assert!(description.contains("state_commit"));
        assert!(description.contains("finished_nodes"));
        assert!(description.contains("result_validities"));
        assert_eq!(
            properties["schema_version"]["enum"][0],
            "taskspace-state-commit-v1"
        );
        assert_eq!(
            properties["finished_nodes"]["items"]["required"],
            serde_json::json!(["node_id", "result_summary"])
        );
    }

    #[test]
    fn compact_taskspace_control_schema_is_map_lifecycle_only() {
        let value = serde_json::to_value(create_taskspace_control_tool_with_profile(
            TaskSpaceControlToolProfile::Compact,
        ))
        .expect("taskspace tool serializes");
        let action_enum = value["parameters"]["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum");
        let actions = action_enum
            .iter()
            .map(|value| value.as_str().expect("string enum"))
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            vec![
                "start_task",
                "route_task",
                "create_node",
                "bind_node",
                "finish_node",
                "block_node",
                "read_output_ref",
            ]
        );

        let properties = value["parameters"]["properties"]
            .as_object()
            .expect("properties object");
        for removed in [
            "schema_version",
            "nodes",
            "finished_nodes",
            "result_validities",
            "success_criteria",
            "output_contracts",
            "fact_sources",
            "facts",
            "decisions",
            "next_best_action",
            "initial_success_criteria",
        ] {
            assert!(!properties.contains_key(removed), "unexpected `{removed}`");
        }
        for kept in [
            "task_id",
            "task_title",
            "task_objective",
            "node_id",
            "kind",
            "title",
            "context_summary",
            "result_summary",
            "blocker_summary",
            "output_ref",
        ] {
            assert!(properties.contains_key(kept), "missing `{kept}`");
        }

        let description = value["description"].as_str().expect("description");
        assert!(description.contains("final response requires no running current node"));
        assert!(description.contains("does not choose the Agent's next action"));
        assert!(!description.contains("cognitive"));
        assert!(!description.contains("state_commit"));
    }
}
