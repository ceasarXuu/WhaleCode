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

pub fn create_taskspace_control_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "action".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("start_task"),
                    json!("route_task"),
                    json!("create_node"),
                    json!("bind_node"),
                    json!("finish_node"),
                    json!("block_node"),
                    json!("record_output_contract"),
                    json!("record_fact_source"),
                    json!("record_fact"),
                    json!("mark_result_validity"),
                ],
                Some(
                "One of: start_task, route_task, create_node, bind_node, finish_node, block_node, record_output_contract, record_fact_source, record_fact, mark_result_validity. Use only for TaskSpace runtime control."
                    .to_string(),
                ),
            ),
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
            "kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some("Required for create_node. Runtime kind for this node.".to_string()),
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
            "claim_id".to_string(),
            JsonSchema::string(Some("Required for record_fact.".to_string())),
        ),
        (
            "statement".to_string(),
            JsonSchema::string(Some("Required for record_fact.".to_string())),
        ),
        (
            "result_id".to_string(),
            JsonSchema::string(Some("Required for mark_result_validity.".to_string())),
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
                Some("Required for mark_result_validity.".to_string()),
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
                JsonSchema::string(Some("Artifact path, id, or stable reference.".to_string())),
                Some("Optional for mark_result_validity.".to_string()),
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
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".to_string(),
        description: r#"Internal TaskSpace control tool.

Use this only when TaskSpace is enabled and you need to update task-map structure before ordinary work.

Supported actions:
- `start_task`: create a new semantic task, its active task path, and the first concrete node. Use this when the current user request does not belong to an existing task in the TaskSpace task inventory. Must include `node_kind`.
- `route_task`: switch the active task path to an existing task chosen by the agent from the TaskSpace task inventory. Runtime validates the id but does not perform semantic matching.
- `create_node`: create a concrete node in the active task path. This requires an existing active task path; use `start_task` first when the current request starts a new semantic task. Must include `kind`. BaseMap candidate nodes are guidance, not automatic graph nodes.
- `bind_node`: bind the main agent's next ordinary action to an existing ready or blocked node that is not held by a subagent.
- `finish_node`: record the current main node's result, mark it completed, and optionally bind an existing next node with `next_node_id` or create and bind a new next node with `next_node_kind`, `next_node_title`, and `next_node_context_summary`.
- `block_node`: record why the current main node cannot proceed and mark it blocked.
- `record_output_contract`: record a task-level output contract with stable `output_contract_id`, `kind`, `description`, and `evidence_refs`.
- `record_fact_source`: record a task-level data source with stable `fact_source_id`, `provenance`, `description`, and `evidence_refs`.
- `record_fact`: record an active task fact with stable `claim_id`, `statement`, and `evidence_refs`. Runtime only accepts facts supported by an accepted result or observed/provided fact source.
- `mark_result_validity`: update an existing node result's evidence package. `accepted` requires claims and evidence refs.

Node kind selection:
- Use `inspect_code_context` for read-only investigation and subagent investigation nodes.
- Use `implement_solution` for code, test, configuration, or documentation edits.
- Use `smoke_test` or `regression_test` before running test/build/lint commands.
- If validation fails and edits are needed, record the test result, switch to `implement_solution` for the fix, then switch back to a test node for the rerun.
- Use `final_synthesis` only for final wrap-up.
- `custom` is reserved for restored legacy nodes and is not valid for live node creation.

Do not expose this tool's internal map/node terminology to the user unless debugging TaskSpace itself.
"#
        .to_string(),
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
                "mark_result_validity",
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
            "claims",
            "evidence_refs",
            "result_id",
            "validity",
            "validity_reason",
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
    }
}
