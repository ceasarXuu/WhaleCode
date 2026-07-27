use std::collections::BTreeSet;

use super::*;

fn function_tool() -> ResponsesApiTool {
    let ToolSpec::Function(tool) = create_taskspace_control_tool() else {
        panic!("taskspace_control must be a function tool");
    };
    tool
}

fn variants(tool: &ResponsesApiTool) -> &[JsonSchema] {
    tool.parameters.any_of.as_deref().expect("control variants")
}

fn action_name(variant: &JsonSchema) -> &str {
    variant.properties.as_ref().expect("properties")["action"]
        .enum_values
        .as_ref()
        .expect("action enum")[0]
        .as_str()
        .expect("action string")
}

fn required_fields(variant: &JsonSchema) -> Vec<&str> {
    variant
        .required
        .as_ref()
        .expect("required fields")
        .iter()
        .map(String::as_str)
        .collect()
}

fn variant<'a>(tool: &'a ResponsesApiTool, action: &str) -> &'a JsonSchema {
    variants(tool)
        .iter()
        .find(|variant| action_name(variant) == action)
        .unwrap_or_else(|| panic!("missing {action}"))
}

#[test]
fn control_exposes_exactly_five_top_level_actions() {
    let tool = function_tool();
    let actions = variants(&tool)
        .iter()
        .map(action_name)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actions,
        BTreeSet::from([
            "execute",
            "finish_map",
            "initialize_and_execute",
            "read_map",
            "read_output_ref",
        ])
    );
}

#[test]
fn initialize_and_execute_keeps_roles_and_action_manifest_separate() {
    let tool = function_tool();
    let initialize = variant(&tool, "initialize_and_execute");
    assert_eq!(
        required_fields(initialize),
        ["action", "root", "work_nodes", "finish", "edges", "actions"]
    );
    let properties = initialize.properties.as_ref().expect("properties");
    assert_eq!(properties["work_nodes"].min_items, Some(1));
    assert_eq!(properties["actions"].min_items, Some(1));
    let action_item = properties["actions"].items.as_deref().expect("action item");
    assert_eq!(required_fields(action_item), ["node_id", "tool"]);
    assert_eq!(
        action_item
            .properties
            .as_ref()
            .expect("action properties")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["node_id", "tool"]
    );
}

#[test]
fn execute_exposes_all_nonterminal_mutations_and_requires_actions() {
    let tool = function_tool();
    let execute = variant(&tool, "execute");
    assert_eq!(
        required_fields(execute),
        ["action", "expected_revision", "mutations", "actions"]
    );
    let properties = execute.properties.as_ref().expect("properties");
    assert_eq!(properties["actions"].min_items, Some(1));
    assert_eq!(properties["mutations"].min_items, None);

    let mutation = properties["mutations"]
        .items
        .as_deref()
        .expect("mutation item");
    let mutation_actions = mutation
        .any_of
        .as_ref()
        .expect("mutation variants")
        .iter()
        .map(action_name)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        mutation_actions,
        BTreeSet::from([
            "add_edges",
            "add_work_nodes",
            "block_node",
            "complete_node",
            "remove_edges",
            "rework_node",
            "unblock_node",
        ])
    );
}

#[test]
fn finish_map_uses_the_explicit_finish_identity_contract() {
    let tool = function_tool();
    let finish = variant(&tool, "finish_map");
    assert_eq!(
        required_fields(finish),
        [
            "action",
            "expected_revision",
            "finish_node_id",
            "exact_summary",
        ]
    );
    let properties = finish.properties.as_ref().expect("properties");
    assert!(!properties.contains_key("terminal_node_id"));
    assert!(!properties.contains_key("final_summary"));
}

#[test]
fn schema_wire_shape_matches_the_b1x_golden() {
    let tool = function_tool();
    let summary = variants(&tool)
        .iter()
        .map(|variant| {
            json!({
                "action": action_name(variant),
                "required": variant.required,
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(
        summary,
        vec![
            json!({"action":"initialize_and_execute","required":["action","root","work_nodes","finish","edges","actions"]}),
            json!({"action":"execute","required":["action","expected_revision","mutations","actions"]}),
            json!({"action":"read_map","required":["action"]}),
            json!({"action":"read_output_ref","required":["action","output_ref","mode","max_bytes"]}),
            json!({"action":"read_output_ref","required":["action","output_ref","mode","max_bytes"]}),
            json!({"action":"read_output_ref","required":["action","output_ref","mode","start_line","end_line","max_bytes"]}),
            json!({"action":"read_output_ref","required":["action","output_ref","mode","pattern","max_bytes"]}),
            json!({"action":"finish_map","required":["action","expected_revision","finish_node_id","exact_summary"]}),
        ]
    );
}

#[test]
fn schema_contains_no_superseded_carrier_or_lifecycle_actions() {
    let serialized =
        serde_json::to_string(&function_tool()).expect("serialize taskspace_control schema");
    for removed in [
        "taskspace_binding",
        "initialize_map",
        "mutate_graph",
        "bind_node",
        "complete_then_continue",
        "expand_nodes",
        "terminal_node_id",
        "final_summary",
    ] {
        assert!(
            !serialized.contains(removed),
            "schema still contains {removed}"
        );
    }
    for removed_action in ["active", "after_boundary"] {
        assert!(
            !serialized.contains(&format!("\"{removed_action}\"")),
            "schema still exposes {removed_action}"
        );
    }
}
