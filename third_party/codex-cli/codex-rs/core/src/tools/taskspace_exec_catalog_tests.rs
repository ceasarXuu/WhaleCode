use std::collections::BTreeMap;

use codex_tools::AdditionalProperties;
use codex_tools::FreeformTool;
use codex_tools::FreeformToolFormat;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiNamespace;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use super::*;

fn function(name: &str) -> ResponsesApiTool {
    let (property, description) = if name == "exec_command" {
        ("cmd", "Run a shell command.")
    } else {
        ("value", "Run the Tool.")
    };
    ResponsesApiTool {
        name: name.into(),
        description: description.into(),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([(property.into(), JsonSchema::string(None))]),
            Some(vec![property.into()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
        defer_loading: None,
    }
}

fn deferred_function(name: &str) -> ResponsesApiTool {
    ResponsesApiTool {
        defer_loading: Some(true),
        ..function(name)
    }
}

fn specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec::Function(function("exec_command")),
        ToolSpec::Function(function("read_file")),
        ToolSpec::Freeform(FreeformTool {
            name: "apply_patch".into(),
            description: "Apply one patch.".into(),
            format: FreeformToolFormat {
                r#type: "grammar".into(),
                syntax: "lark".into(),
                definition: "start: /.+/".into(),
            },
        }),
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "mcp__sample__".into(),
            description: "Sample namespace.".into(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("lookup"))],
        }),
        ToolSpec::ToolSearch {
            execution: "client".into(),
            description: "Search deferred tools.".into(),
            parameters: JsonSchema::object(
                BTreeMap::from([("query".into(), JsonSchema::string(None))]),
                Some(vec!["query".into()]),
                Some(AdditionalProperties::Boolean(false)),
            ),
        },
        ToolSpec::WebSearch {
            external_web_access: Some(true),
            filters: None,
            user_location: None,
            search_context_size: None,
            search_content_types: None,
        },
        ToolSpec::ImageGeneration {
            output_format: "png".into(),
        },
        ToolSpec::Function(function("exec")),
        ToolSpec::Function(function("wait")),
        ToolSpec::Function(function("update_plan")),
    ]
}

fn update_input() -> Value {
    json!({
        "add_work_nodes": [],
        "node_patches": [{"node_id": "work", "state": "completed"}]
    })
}

fn client_tool() -> Value {
    json!({"tool": "read_file", "node_id": "work", "input": {"value": "a"}})
}

#[test]
fn declaration_is_deterministic_and_exposes_one_closed_contract() {
    let first = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let second = TaskSpaceExecCatalog::build(&specs()).unwrap();
    assert_eq!(
        serde_json::to_vec(first.declaration()).unwrap(),
        serde_json::to_vec(second.declaration()).unwrap()
    );
    assert_eq!(first.capability_identity(), second.capability_identity());

    let declaration = serde_json::to_value(first.declaration()).unwrap();
    let parameters = &declaration["parameters"];
    assert_eq!(parameters["anyOf"].as_array().unwrap().len(), 8);
    let sequence_descriptions = parameters["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|branch| {
            (
                branch["properties"]["type"]["enum"][0].as_str().unwrap(),
                branch["description"].as_str().unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(sequence_descriptions.len(), 8);
    assert!(sequence_descriptions["initialize_and_work"].contains("Map is blank"));
    assert!(sequence_descriptions["work"].contains("already Ready or InFlight"));
    assert!(sequence_descriptions["work"].contains("does not complete its owner"));
    assert!(sequence_descriptions["work"].contains("use update_and_work instead"));
    assert!(sequence_descriptions["update_map"].contains("without Tool work"));
    assert!(sequence_descriptions["update_and_work"].contains("Update the Map first"));
    assert!(sequence_descriptions["update_and_work"].contains("direct dependents"));
    assert!(sequence_descriptions["update_and_work"].contains("do not unlock descendants"));
    assert!(sequence_descriptions["update_and_finish"].contains("Finish node Ready"));
    assert!(sequence_descriptions["read_map"].contains("without changing it"));
    assert!(sequence_descriptions["reopen_update_and_work"].contains("user feedback"));
    assert!(sequence_descriptions["finish_map"].contains("already Ready"));
    assert!(parameters["$defs"]["tool_action"].is_object());
    assert_eq!(
        parameters["$defs"]["tool_action"]["anyOf"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    let rendered = declaration.to_string();
    for name in [
        "initialize_and_work",
        "update_and_work",
        "update_and_finish",
        "reopen_update_and_work",
        "read_file",
        "apply_patch",
        "web_search",
        "image_generation",
    ] {
        assert!(rendered.contains(name), "missing {name}");
    }
    for forbidden in ["hosted_bindings", "client_work", "hosted_work", "\"calls\""] {
        assert!(!rendered.contains(forbidden), "found {forbidden}");
    }
    assert!(!rendered.contains("update_plan"));
    assert!(!rendered.contains("\"exec\""));
    assert!(!rendered.contains("\"wait\""));
    assert!(
        declaration["description"]
            .as_str()
            .unwrap()
            .contains("single `tools` array")
    );
    assert!(
        declaration["description"]
            .as_str()
            .unwrap()
            .contains("do not also patch that owner to `in_flight`")
    );
}

#[test]
fn all_eight_legal_sequences_decode_and_old_wire_is_rejected() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let legal = [
        canonical_first_turn_example(),
        json!({"type": "work", "tools": [client_tool()]}),
        json!({"type": "update_map", "update_map": update_input()}),
        canonical_handoff_example(),
        canonical_finish_example(),
        canonical_read_example(),
        json!({
            "type": "reopen_update_and_work",
            "reopen_map": {},
            "update_map": update_input(),
            "tools": [client_tool()]
        }),
        json!({"type": "finish_map", "finish_map": {"content": "done"}}),
    ];
    for value in legal {
        catalog
            .decode_plan(&value.to_string())
            .unwrap_or_else(|error| panic!("rejected {value}: {error:?}"));
    }
    for old in [
        json!({"calls": []}),
        json!({"hosted_bindings": [{"tool": "web_search", "node_ids": ["work"]}]}),
        json!({"type": "custom", "tools": [client_tool()]}),
        json!({"type": "work", "tools": []}),
    ] {
        assert!(
            catalog.decode_plan(&old.to_string()).is_err(),
            "accepted {old}"
        );
    }
}

#[test]
fn unified_tools_preserve_client_freeform_namespace_and_hosted_actions() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let plan = catalog
        .decode_plan(
            &json!({
                "type": "work",
                "tools": [
                    client_tool(),
                    {"tool": "apply_patch", "node_id": "fix", "input": "*** Begin Patch"},
                    {"tool": "lookup", "namespace": "mcp__sample__", "node_id": "work", "input": {"value": "b"}},
                    {"tool": "web_search", "node_ids": ["work", "fix"]},
                    {"tool": "image_generation", "node_ids": ["work"]}
                ]
            })
            .to_string(),
        )
        .unwrap();
    assert_eq!(plan.sequence_type, "work");
    assert_eq!(plan.tools.len(), 5);
    let ToolAction::Client(namespace) = &plan.tools[2] else {
        panic!("expected namespaced client Tool")
    };
    assert_eq!(
        namespace.tool_name,
        ToolName::namespaced("mcp__sample__", "lookup")
    );
    let ToolAction::Hosted(hosted) = &plan.tools[3] else {
        panic!("expected hosted Tool")
    };
    assert_eq!(hosted.node_ids, vec!["work", "fix"]);
}

#[test]
fn tool_shapes_are_exact_and_namespace_identity_is_lossless() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    for invalid in [
        json!({"type": "work", "tools": [{"tool": "missing", "node_id": "n", "input": {}}]}),
        json!({"type": "work", "tools": [{"tool": "mcp__sample__lookup", "node_id": "n", "input": {"value": "v"}}]}),
        json!({"type": "work", "tools": [{"tool": "lookup", "node_id": "n", "input": {"value": "v"}}]}),
        json!({"type": "work", "tools": [{"tool": "web_search", "node_ids": []}]}),
        json!({"type": "work", "tools": [{"tool": "read_file", "node_id": "n", "input": {"value": "v"}, "revision": 1}]}),
    ] {
        assert!(
            catalog.decode_plan(&invalid.to_string()).is_err(),
            "accepted {invalid}"
        );
    }
}

#[test]
fn deferred_capabilities_are_hidden_until_loaded() {
    let specs = vec![
        ToolSpec::Function(function("always_visible")),
        ToolSpec::Function(deferred_function("deferred_plain")),
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "deferred_namespace".into(),
            description: "Deferred namespace.".into(),
            tools: vec![ResponsesApiNamespaceTool::Function(deferred_function(
                "selected_child",
            ))],
        }),
    ];
    let initial = TaskSpaceExecCatalog::build(&specs).unwrap();
    let loaded = TaskSpaceExecCatalog::build_with_loaded_deferred(
        &specs,
        &[ToolSpec::Namespace(ResponsesApiNamespace {
            name: "deferred_namespace".into(),
            description: "Deferred namespace.".into(),
            tools: vec![ResponsesApiNamespaceTool::Function(deferred_function(
                "selected_child",
            ))],
        })],
    )
    .unwrap();
    let initial_json = serde_json::to_string(initial.declaration()).unwrap();
    let loaded_json = serde_json::to_string(loaded.declaration()).unwrap();
    assert!(initial_json.contains("always_visible"));
    assert!(!initial_json.contains("deferred_plain"));
    assert!(!initial_json.contains("selected_child"));
    assert!(loaded_json.contains("selected_child"));
    assert_ne!(initial.capability_identity(), loaded.capability_identity());
}

#[test]
fn catalog_rejects_collisions_and_is_source_order_independent() {
    assert_eq!(
        TaskSpaceExecCatalog::build(&[ToolSpec::Function(function("read_map"))]).unwrap_err(),
        TaskSpaceExecCatalogError::MapCapabilityCollision {
            public_name: "read_map".into()
        }
    );
    let duplicate = vec![
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "same".into(),
            description: String::new(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("lookup"))],
        }),
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "same".into(),
            description: String::new(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("lookup"))],
        }),
    ];
    assert!(matches!(
        TaskSpaceExecCatalog::build(&duplicate),
        Err(TaskSpaceExecCatalogError::DuplicateCapability { .. })
    ));

    let mut reversed = specs();
    reversed.reverse();
    assert_eq!(
        serde_json::to_vec(TaskSpaceExecCatalog::build(&specs()).unwrap().declaration()).unwrap(),
        serde_json::to_vec(
            TaskSpaceExecCatalog::build(&reversed)
                .unwrap()
                .declaration()
        )
        .unwrap()
    );
}

#[test]
fn declaration_without_hosted_tools_has_no_hosted_variant() {
    let catalog =
        TaskSpaceExecCatalog::build(&[ToolSpec::Function(function("read_file"))]).unwrap();
    let declaration = serde_json::to_string(catalog.declaration()).unwrap();
    assert!(!declaration.contains("web_search"));
    assert!(
        catalog
            .decode_plan(
                &json!({"type": "work", "tools": [{"tool": "web_search", "node_ids": ["n"]}]})
                    .to_string()
            )
            .is_err()
    );
}
