use super::*;
use crate::create_apply_patch_freeform_tool;
use crate::create_list_dir_tool;

#[test]
fn bootstrap_schema_requires_initialization_with_continuation() {
    let list_dir = create_list_dir_tool();
    let list_dir_value = serde_json::to_value(&list_dir).expect("serialize list_dir");
    let value =
        serde_json::to_value(create_taskspace_control_tool(&[list_dir])).expect("serialize");
    assert_eq!(value["parameters"]["type"], json!("object"));
    let variants = value["parameters"]["anyOf"].as_array().expect("variants");
    let action_names = variants
        .iter()
        .filter_map(|variant| variant["properties"]["action"]["enum"][0].as_str())
        .collect::<Vec<_>>();
    assert_eq!(action_names, vec!["initialize_map"]);
    assert_eq!(
        variants[0]["required"],
        json!([
            "action",
            "root",
            "finish",
            "work_nodes",
            "edges",
            "current_node_id",
            "continuation"
        ])
    );
    let text = value.to_string();
    assert!(!text.contains("initialize_then_actions"));
    assert!(!text.contains("initial_nodes"));
    assert!(!text.contains("dependency_node_ids"));
    assert!(!text.contains("task_goal"));
    assert!(!text.contains("task_title"));
    assert!(!text.contains("task_objective"));
    assert!(!text.contains("context_summary"));
    assert_eq!(
        value["parameters"]["$defs"]["ordinaryAction"]["anyOf"][0]["properties"]["arguments"],
        list_dir_value["parameters"]
    );
    let continuation = variants[0]["properties"]["continuation"]["anyOf"]
        .as_array()
        .expect("continuation variants");
    assert_eq!(continuation.len(), 1);
    assert_eq!(continuation[0]["properties"]["kind"]["enum"][0], "actions");
    assert!(value["parameters"]["$defs"].get("patchAction").is_none());
}

#[test]
fn bootstrap_schema_exposes_one_patch_slot_outside_ordinary_actions() {
    let value = serde_json::to_value(create_taskspace_control_tool(&[
        create_list_dir_tool(),
        create_apply_patch_freeform_tool(),
    ]))
    .expect("serialize");
    let definitions = value["parameters"]["$defs"]
        .as_object()
        .expect("definitions");
    assert_eq!(
        definitions["patchAction"]["properties"]["tool_name"]["enum"][0],
        "apply_patch"
    );
    assert!(
        !definitions["ordinaryAction"]
            .to_string()
            .contains("apply_patch")
    );

    let continuation = value["parameters"]["anyOf"][0]["properties"]["continuation"]["anyOf"]
        .as_array()
        .expect("continuation variants");
    assert_eq!(continuation.len(), 2);
    let patch = continuation
        .iter()
        .find(|variant| variant["properties"]["kind"]["enum"][0] == "patch_then_actions")
        .expect("patch continuation");
    assert_eq!(patch["required"], json!(["kind", "patch"]));
    assert_eq!(patch["additionalProperties"], false);
}

#[test]
fn active_schema_contains_no_ordinary_tool_expression() {
    let value = serde_json::to_value(create_taskspace_active_control_tool()).expect("serialize");
    let text = value.to_string();
    let actions = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .map(|variant| variant["properties"]["action"]["enum"][0].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"mutate_graph"));
    assert!(actions.contains(&"transition_node"));
    assert!(actions.contains(&"finish_end"));
    assert!(actions.contains(&"expand_nodes"));
    assert!(actions.contains(&"read_output_ref"));
    assert!(!actions.contains(&"create_node"));
    assert!(!actions.contains(&"finish_nodes"));
    assert!(!actions.contains(&"finish_then_end"));
    assert!(!text.contains("ordinaryAction"));
    assert!(!text.contains("tool_name"));
    assert!(!text.contains("arguments"));
    assert!(!text.contains("\"next\""));
    assert!(!text.contains("\"existing\""));
    assert!(!text.contains("\"create\""));
    assert!(!text.contains("finish_node_ids"));
    assert!(!text.contains("NodeKind"));
    assert!(!text.contains("dependency_node_ids"));
    assert!(!text.contains("terminal_node_id"));
    assert!(!text.contains("preceding_finishes"));
    assert!(!text.contains("next_node_goal"));
    assert!(!text.contains("terminal_finish"));
    assert!(!text.contains("result_summary"));
    assert!(!text.contains("blocker_summary"));
    assert!(!text.contains("task_title"));
    assert!(!text.contains("context_summary"));
    let transition = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("transition_node"))
        .expect("transition variant");
    assert_eq!(
        transition["required"],
        json!(["action", "expected_revision", "node_id", "transition"])
    );
    assert_eq!(
        transition["properties"]["transition"]["enum"],
        json!(["bind", "complete", "block", "unblock"])
    );
    let mutation = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("mutate_graph"))
        .expect("mutation variant");
    assert_eq!(
        mutation["required"],
        json!([
            "action",
            "expected_revision",
            "add_nodes",
            "add_edges",
            "remove_edges"
        ])
    );
    let terminal = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("finish_end"))
        .expect("terminal variant");
    assert_eq!(
        terminal["required"],
        json!(["action", "expected_revision", "final_summary"])
    );
    let expand = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("expand_nodes"))
        .expect("expand variant");
    assert_eq!(expand["required"], json!(["action", "node_ids"]));
    assert_eq!(expand["properties"]["node_ids"]["minItems"], 1);
    assert_eq!(expand["additionalProperties"], false);
}
