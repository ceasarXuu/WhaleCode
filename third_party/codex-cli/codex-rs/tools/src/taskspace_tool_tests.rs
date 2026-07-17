use super::*;
use crate::create_apply_patch_freeform_tool;
use crate::create_list_dir_tool;

#[test]
fn lifecycle_schema_includes_initialization_with_required_continuation() {
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
    assert_eq!(
        action_names,
        vec![
            "initialize_map",
            "mutate_graph",
            "transition_node",
            "transition_node",
            "finish_end",
            "expand_nodes",
            "read_output_ref"
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
            "continuation"
        ])
    );
    assert!(variants[0]["properties"].get("current_node_id").is_none());
    assert_eq!(
        variants[0]["properties"]["initial_work_node"]["description"],
        "Agent-selected initial Work node. Define it only here, not in additional_work_nodes. Declared edges must make it Ready at initialization; Runtime binds it before continuation actions execute."
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
    let ordinary = &value["parameters"]["$defs"]["ordinaryAction"]["anyOf"][0];
    assert_eq!(
        ordinary["properties"]["tool_name"]["enum"],
        json!(["list_dir"])
    );
    assert_eq!(ordinary["properties"]["arguments"]["type"], "object");
    assert_eq!(ordinary["properties"]["arguments"]["properties"], json!({}));
    assert_ne!(
        ordinary["properties"]["arguments"],
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
fn lifecycle_schema_exposes_active_actions_without_continuation_fields() {
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
    assert!(actions.contains(&"finish_end"));
    assert!(actions.contains(&"expand_nodes"));
    assert!(actions.contains(&"read_output_ref"));
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
            "continuation"
        ])
    );
    assert!(bind["properties"].get("continuation").is_some());

    let transition = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .find(|variant| {
            variant["properties"]["action"]["enum"][0] == json!("transition_node")
                && variant["properties"]["transition"]["enum"]
                    == json!(["complete", "block", "unblock", "rework"])
        })
        .expect("non-bind transition variant");
    assert_eq!(
        transition["required"],
        json!(["action", "expected_revision", "node_id", "transition"])
    );
    assert!(transition["properties"].get("continuation").is_none());
    assert_eq!(
        transition["properties"]["transition"]["enum"],
        json!(["complete", "block", "unblock", "rework"])
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
    assert!(mutation["properties"].get("continuation").is_some());
    assert!(
        !mutation["required"]
            .as_array()
            .expect("mutation required")
            .contains(&json!("continuation"))
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
