use super::*;
use crate::create_apply_patch_freeform_tool;
use crate::create_list_dir_tool;

const SELECTED_CONTROL_V2: &str = include_str!(
    "../../../../../benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json"
);

#[test]
fn provider_visible_schema_matches_the_selected_r7_contract() {
    let selected: serde_json::Value =
        serde_json::from_str(SELECTED_CONTROL_V2).expect("selected control contract");
    let actual = serde_json::to_value(create_taskspace_control_tool(&[
        create_list_dir_tool(),
        create_apply_patch_freeform_tool(),
    ]))
    .expect("serialize provider tool");

    assert_eq!(
        actual["description"],
        selected["provider_tool"]["function"]["description"]
    );
    assert_eq!(
        actual["parameters"],
        selected["provider_tool"]["function"]["parameters"]
    );
}

#[test]
fn hidden_patch_profile_only_removes_patch_from_next_call_enums() {
    let selected: serde_json::Value =
        serde_json::from_str(SELECTED_CONTROL_V2).expect("selected control contract");
    let mut expected = selected["provider_tool"]["function"]["parameters"].clone();
    remove_patch_capability(&mut expected);
    let actual = serde_json::to_value(create_taskspace_control_tool(&[create_list_dir_tool()]))
        .expect("serialize provider tool");

    assert_eq!(actual["parameters"], expected);
}

fn remove_patch_capability(schema: &mut serde_json::Value) {
    match schema {
        serde_json::Value::Array(values) => {
            for value in values {
                remove_patch_capability(value);
            }
        }
        serde_json::Value::Object(fields) => {
            if let Some(required_next_call) = fields.get_mut("required_next_call")
                && let Some(values) = required_next_call["enum"].as_array_mut()
            {
                values.retain(|value| value != "apply_patch");
            }
            for value in fields.values_mut() {
                remove_patch_capability(value);
            }
        }
        _ => {}
    }
}

#[test]
fn lifecycle_schema_includes_initialization_with_required_next_call() {
    let list_dir = create_list_dir_tool();
    let value =
        serde_json::to_value(create_taskspace_control_tool(&[list_dir])).expect("serialize");
    assert!(
        value["description"]
            .as_str()
            .is_some_and(|description| description.contains("canonical TaskSpace Map"))
    );
    let description = value["description"].as_str().expect("tool description");
    assert!(description.contains("never chooses nodes, repairs arguments, or decides"));
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
            "bind_node",
            "block_node",
            "unblock_node",
            "rework_node",
            "complete_then_continue",
            "complete_then_end",
            "finish_end",
            "expand_nodes",
            "read_map",
            "read_output_ref",
            "read_output_ref",
            "read_output_ref",
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
            "required_next_call"
        ])
    );
    assert!(variants[0]["properties"].get("current_node_id").is_none());
    assert_eq!(
        variants[0]["properties"]["initial_work_node"]["properties"]["goal"]["description"],
        "The first coherent Work goal."
    );
    assert_eq!(
        variants[0]["properties"]["additional_work_nodes"]["items"]["properties"]["goal"]["description"],
        "A coherent Work goal."
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
    assert!(
        variants[0]["properties"]["finish_identity"]
            .get("description")
            .is_none()
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
    assert!(actions.contains(&"bind_node"));
    assert!(actions.contains(&"block_node"));
    assert!(actions.contains(&"unblock_node"));
    assert!(actions.contains(&"rework_node"));
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
        .find(|variant| variant["properties"]["action"]["enum"][0] == json!("bind_node"))
        .expect("bind variant");
    assert_eq!(
        bind["required"],
        json!([
            "action",
            "expected_revision",
            "node_id",
            "required_next_call"
        ])
    );
    assert!(bind["properties"].get("required_next_call").is_some());

    for action in ["block_node", "unblock_node", "rework_node"] {
        let transition = value["parameters"]["anyOf"]
            .as_array()
            .expect("variants")
            .iter()
            .find(|variant| variant["properties"]["action"]["enum"][0] == json!(action))
            .expect("direct transition variant");
        assert_eq!(
            transition["required"],
            json!(["action", "expected_revision", "node_id"])
        );
        assert!(transition["properties"].get("required_next_call").is_none());
        assert!(transition["properties"].get("transition").is_none());
    }
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

    let read_variants = value["parameters"]["anyOf"]
        .as_array()
        .expect("variants")
        .iter()
        .filter(|variant| variant["properties"]["action"]["enum"][0] == json!("read_output_ref"))
        .collect::<Vec<_>>();
    assert_eq!(read_variants.len(), 4);
    assert_eq!(
        read_variants
            .iter()
            .map(|variant| variant["properties"]["mode"]["enum"][0].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["head", "tail", "line_range", "grep"]
    );
    assert_eq!(read_variants[0]["properties"]["max_bytes"]["minimum"], 1);
    assert_eq!(
        read_variants[2]["required"],
        json!([
            "action",
            "output_ref",
            "mode",
            "start_line",
            "end_line",
            "max_bytes"
        ])
    );
    assert!(read_variants[0]["properties"].get("pattern").is_none());
    assert!(read_variants[3]["properties"].get("start_line").is_none());
}
