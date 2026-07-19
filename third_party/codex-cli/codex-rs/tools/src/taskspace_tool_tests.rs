use super::*;
use crate::create_apply_patch_freeform_tool;
use crate::create_list_dir_tool;

#[test]
fn lifecycle_schema_includes_initialization_with_required_next_call() {
    let list_dir = create_list_dir_tool();
    let value =
        serde_json::to_value(create_taskspace_control_tool(&[list_dir])).expect("serialize");
    assert!(
        value["description"]
            .as_str()
            .is_some_and(|description| description.contains(
                "the first top-level tool call MUST be taskspace_control with action=initialize_map"
            ))
    );
    assert_eq!(value["parameters"]["type"], json!("object"));
    let variants = value["parameters"]["anyOf"].as_array().expect("variants");
    let action_names = variants
        .iter()
        .filter_map(|variant| variant["properties"]["action"]["enum"][0].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        action_names,
        vec![
            "initialize_map",
            "mutate_graph",
            "transition_node",
            "transition_node",
            "complete_then_continue",
            "complete_then_end",
            "finish_end",
            "expand_nodes",
            "read_output_ref",
            "read_map"
        ]
    );
    assert_eq!(
        variants[0]["required"],
        json!([
            "action",
            "root",
            "initial_work_node",
            "finish_identity",
            "additional_work_nodes",
            "edges",
            "required_next_call"
        ])
    );
    assert!(variants[0]["properties"].get("current_node_id").is_none());
    assert_eq!(
        variants[0]["properties"]["initial_work_node"]["description"],
        "Agent-selected initial Work node. Define it only here, not in additional_work_nodes. Declared edges must make it Ready at initialization; Runtime binds it before the required next top-level call executes."
    );
    assert_eq!(
        variants[0]["properties"]["additional_work_nodes"]["description"],
        "Zero or more Work nodes other than initial_work_node. Node IDs must be distinct across the entire graph."
    );
    assert_eq!(
        variants[0]["properties"]["finish_identity"]["required"],
        json!(["id"])
    );
    assert_eq!(
        variants[0]["properties"]["finish_identity"]["additionalProperties"],
        false
    );
    assert!(
        variants[0]["properties"]["finish_identity"]["properties"]
            .get("goal")
            .is_none()
    );
    assert_eq!(
        variants[0]["properties"]["finish_identity"]["description"],
        "The unique terminal graph node identity. Reference id as the graph's only sink in edges; every node must reach it. All executable work, including validation, belongs to Work nodes."
    );
    assert!(variants[0]["properties"].get("finish").is_none());
    assert!(variants[0]["properties"].get("current_work_node").is_none());
    assert!(variants[0]["properties"].get("work_nodes").is_none());
    let text = value.to_string();
    assert!(!text.contains("initialize_then_actions"));
    assert!(!text.contains("initial_nodes"));
    assert!(!text.contains("dependency_node_ids"));
    assert!(!text.contains("task_goal"));
    assert!(!text.contains("task_title"));
    assert!(!text.contains("task_objective"));
    assert!(!text.contains("context_summary"));
    assert!(value["parameters"].get("$defs").is_none());
    assert_eq!(
        variants[0]["properties"]["required_next_call"]["enum"],
        json!(["ordinary_tool"])
    );
}

#[test]
fn bootstrap_schema_declares_top_level_patch_without_nested_tool_payloads() {
    let value = serde_json::to_value(create_taskspace_control_tool(&[
        create_list_dir_tool(),
        create_apply_patch_freeform_tool(),
    ]))
    .expect("serialize");
    assert!(value["parameters"].get("$defs").is_none());
    assert_eq!(
        value["parameters"]["anyOf"][0]["properties"]["required_next_call"]["enum"],
        json!(["ordinary_tool", "apply_patch"])
    );
    let text = value.to_string();
    assert!(!text.contains("ordinaryAction"));
    assert!(!text.contains("patchAction"));
    assert!(!text.contains("patch_then_actions"));
    assert!(!text.contains("tool_name"));
}

#[test]
fn lifecycle_schema_requires_atomic_completion_handoffs() {
    let value = serde_json::to_value(create_taskspace_control_tool(&[create_list_dir_tool()]))
        .expect("serialize");
    let actions = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .map(|variant| variant["properties"]["action"]["enum"][0].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"mutate_graph"));
    assert!(actions.contains(&"transition_node"));
    assert!(actions.contains(&"complete_then_continue"));
    assert!(actions.contains(&"complete_then_end"));
    assert!(actions.contains(&"finish_end"));
    assert!(actions.contains(&"expand_nodes"));
    assert!(actions.contains(&"read_output_ref"));
    assert!(actions.contains(&"read_map"));
    assert!(!actions.contains(&"create_node"));
    assert!(!actions.contains(&"finish_nodes"));
    assert!(!actions.contains(&"finish_then_end"));
    let bind = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| {
            variant["properties"]["action"]["enum"][0] == json!("transition_node")
                && variant["properties"]["transition"]["enum"] == json!(["bind"])
        })
        .expect("bind variant");
    assert_eq!(
        bind["required"],
        json!([
            "action",
            "expected_revision",
            "node_id",
            "transition",
            "required_next_call"
        ])
    );
    assert!(bind["properties"].get("required_next_call").is_some());

    let transition = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| {
            variant["properties"]["action"]["enum"][0] == json!("transition_node")
                && variant["properties"]["transition"]["enum"]
                    == json!(["block", "unblock", "rework"])
        })
        .expect("non-bind transition variant");
    assert_eq!(
        transition["required"],
        json!(["action", "expected_revision", "node_id", "transition"])
    );
    assert!(transition["properties"].get("required_next_call").is_none());
    assert_eq!(
        transition["properties"]["transition"]["enum"],
        json!(["block", "unblock", "rework"])
    );
    assert!(!value.to_string().contains("\"complete\""));
    let handoff = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| {
            variant["properties"]["action"]["enum"][0] == json!("complete_then_continue")
        })
        .expect("atomic handoff variant");
    assert_eq!(
        handoff["required"],
        json!([
            "action",
            "expected_revision",
            "current_node_id",
            "next_node_id",
            "required_next_call"
        ])
    );
    assert!(handoff["properties"].get("required_next_call").is_some());
    let complete_end = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("complete_then_end"))
        .expect("atomic terminal variant");
    assert_eq!(
        complete_end["required"],
        json!([
            "action",
            "expected_revision",
            "current_node_id",
            "final_summary"
        ])
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
    assert!(mutation["properties"].get("required_next_call").is_some());
    assert!(
        !mutation["required"]
            .as_array()
            .expect("mutation required")
            .contains(&json!("required_next_call"))
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
    let read_map = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("read_map"))
        .expect("shared read_map variant");
    assert_eq!(read_map["required"], json!(["action"]));
    assert_eq!(read_map["additionalProperties"], false);
}
