use super::*;

fn action_names(spec: ToolSpec) -> Vec<String> {
    let ToolSpec::Function(tool) = spec else {
        panic!("taskspace_control must be a function tool");
    };
    tool.parameters
        .any_of
        .expect("control variants")
        .into_iter()
        .map(|variant| {
            variant.properties.expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                .as_str()
                .expect("action string")
                .to_string()
        })
        .collect()
}

#[test]
fn provider_tool_matches_the_fla9_authority_artifact() {
    let authority: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../benchmarks/taskspace/r7/five-layer-taskspace-control-v3.schema.json"
    ))
    .expect("FLA-9 authority schema");
    let ToolSpec::Function(tool) = create_taskspace_control_tool() else {
        panic!("taskspace_control must be a function tool");
    };
    let actual = json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
        }
    });
    if let Ok(path) = std::env::var("R7_TASKSPACE_CONTROL_TOOL_OUT") {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&actual).expect("serialize provider tool"),
        )
        .expect("write provider tool fixture");
    }

    assert_eq!(actual, authority["provider_tool"]);
}

#[test]
fn control_exposes_boundary_and_standalone_actions_once() {
    let actions = action_names(create_taskspace_control_tool());

    for action in [
        "initialize_map",
        "mutate_graph",
        "bind_node",
        "complete_then_continue",
        "block_node",
        "unblock_node",
        "rework_node",
        "finish_map",
        "expand_nodes",
        "read_map",
        "read_output_ref",
    ] {
        assert!(actions.contains(&action.to_string()), "missing {action}");
    }
    assert!(!actions.contains(&"finish_end".to_string()));
    assert!(!actions.contains(&"complete_then_end".to_string()));
    assert!(!actions.contains(&"complete_active_work_then_end".to_string()));
    assert!(!actions.contains(&"close_ready_finish".to_string()));
    assert!(!actions.contains(&"complete_last_running_work_then_end".to_string()));
    assert!(!actions.contains(&"close_finish_with_no_active_work".to_string()));
}

#[test]
fn finish_map_exposes_one_branch_free_terminal_contract() {
    let ToolSpec::Function(tool) = create_taskspace_control_tool() else {
        panic!("taskspace_control must be a function tool");
    };
    let variants = tool.parameters.any_of.as_ref().expect("control variants");
    let closure = variants
        .iter()
        .find(|variant| {
            variant.properties.as_ref().expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                == json!("finish_map")
        })
        .expect("unified Map closure");

    assert_eq!(
        closure.required.as_ref().expect("required"),
        &[
            "action",
            "expected_revision",
            "terminal_node_id",
            "final_summary",
        ]
    );
    let properties = closure.properties.as_ref().expect("properties");
    assert!(properties["terminal_node_id"].enum_values.is_none());
    for removed in [
        "terminal_state",
        "incomplete_work_node_ids",
        "finish_node_id",
        "finish_status",
    ] {
        assert!(!properties.contains_key(removed), "unexpected {removed}");
    }
}

#[test]
fn boundary_actions_expose_exact_payloads_on_control() {
    let ToolSpec::Function(tool) = create_taskspace_control_tool() else {
        panic!("taskspace_control must be a function tool");
    };
    let variants = tool.parameters.any_of.as_ref().expect("control variants");
    let initialize = variants
        .iter()
        .find(|variant| {
            variant.properties.as_ref().expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                == json!("initialize_map")
        })
        .expect("initialize_map");
    let initialize_properties = initialize.properties.as_ref().expect("properties");
    assert!(initialize_properties.contains_key("root"));
    assert!(initialize_properties.contains_key("finish_identity"));
    assert!(
        !serde_json::to_string(&tool)
            .expect("serialize")
            .contains("continue_current")
    );
}
