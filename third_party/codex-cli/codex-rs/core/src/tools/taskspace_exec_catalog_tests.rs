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
    assert!(sequence_descriptions["initialize_and_work"].contains("required non-empty `tools`"));
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
    let initialize_branch = parameters["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["properties"]["type"]["enum"][0] == "initialize_and_work")
        .unwrap();
    let initialize_map = &initialize_branch["properties"]["initialize_map"];
    assert_eq!(initialize_map["type"], "object");
    assert!(initialize_map.get("$ref").is_none());
    assert!(parameters["$defs"].get("initialize_map_input").is_none());
    assert!(parameters["$defs"]["tool_action"].is_object());
    assert_eq!(
        parameters["$defs"]["tool_action"]["anyOf"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    for branch in parameters["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|branch| {
            matches!(
                branch["properties"]["type"]["enum"][0].as_str(),
                Some("initialize_and_work" | "work" | "update_and_work" | "reopen_update_and_work")
            )
        })
    {
        assert!(
            branch["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|name| name == "tools")
        );
        assert_eq!(branch["properties"]["tools"]["minItems"], 1);
    }
    let rendered = declaration.to_string();
    for name in [
        "initialize_and_work",
        "update_and_work",
        "update_and_finish",
        "reopen_update_and_work",
        "read_file",
        "apply_patch",
    ] {
        assert!(rendered.contains(name), "missing {name}");
    }
    for forbidden in ["hosted_bindings", "client_work", "hosted_work", "\"calls\""] {
        assert!(!rendered.contains(forbidden), "found {forbidden}");
    }
    assert!(!rendered.contains("update_plan"));
    assert!(!rendered.contains("web_search"));
    assert!(!rendered.contains("image_generation"));
    assert!(!rendered.contains("\"exec\""));
    assert!(!rendered.contains("\"wait\""));
    assert!(
        declaration["description"]
            .as_str()
            .unwrap()
            .contains("only client Tool actions")
    );
    assert!(
        declaration["description"]
            .as_str()
            .unwrap()
            .contains("emit exactly one `taskspace_exec` Function Call")
    );
    assert!(
        declaration["description"]
            .as_str()
            .unwrap()
            .contains("never emit sibling `taskspace_exec` calls")
    );
    assert!(
        declaration["description"]
            .as_str()
            .unwrap()
            .contains("do not also patch that owner to `in_flight`")
    );
    let description = declaration["description"].as_str().unwrap();
    assert_eq!(
        description.matches("Node state-machine contract:").count(),
        1
    );
    for required in [
        "Root stays `in_flight` while the Map is open",
        "`waiting` means at least one non-Root parent is incomplete",
        "`ready` means every non-Root parent is `completed`",
        "`in_flight` means the Agent has started work",
        "`completed` means the Agent has explicitly recorded completion",
        "changing parents may rederive a not-started node between `waiting` and `ready`",
        "only `ready -> in_flight`, `ready -> completed`, or `in_flight -> completed`",
        "No other explicit state transition is accepted",
        "Tool success, failure, or cancellation records an outcome but never completes the owner",
        "the sequence's Map operation is applied before its Tool actions",
        "Map patches are applied in declared array order",
        "Any invalid patch rejects the whole sequence with no commit",
        "Tool outcomes do not unlock descendants",
        "Only `finish_map` may change ready Finish and open Root to `completed`",
        "completed Work nodes remain completed",
    ] {
        assert!(description.contains(required), "missing {required}");
    }
    assert!(!description.contains("TaskSpacePendingProviderActionsR8V1"));
    assert!(!description.contains("assign_pending_actions"));
    let first_turn = description
        .split_once("First-turn initialization and work example:")
        .unwrap()
        .1
        .split_once("Parent completion and direct-child work example:")
        .unwrap()
        .0;
    for required in [
        "\"type\":\"initialize_and_work\"",
        "\"tool\":\"exec_command\"",
    ] {
        assert!(first_turn.contains(required), "missing {required}");
    }
    assert!(!first_turn.contains("\"tool\":\"web_search\""));
    assert!(!first_turn.contains("\"execution\":\"already_executed\""));
    for forbidden in [
        "emit both",
        "native top-level Provider Tool item",
        "Never emit either side alone",
    ] {
        assert!(!description.contains(forbidden), "found {forbidden}");
    }
    assert!(!description.contains("\"type\":\"work\",\"tools\":[{\"tool\":\"web_search\""));
    assert!(!description.contains("single top-level entry point"));
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
    ] {
        assert!(
            catalog.decode_plan(&old.to_string()).is_err(),
            "accepted {old}"
        );
    }
    for missing_client_work in [
        json!({
            "type": "initialize_and_work",
            "initialize_map": canonical_first_turn_example()["initialize_map"].clone()
        }),
        json!({"type": "work"}),
        json!({"type": "work", "tools": []}),
    ] {
        assert!(
            catalog
                .decode_plan(&missing_client_work.to_string())
                .is_err(),
            "accepted work sequence without client work: {missing_client_work}"
        );
    }
}

#[test]
fn client_tools_decode_without_provider_wire_overlap() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let plan = catalog
        .decode_plan(
            &json!({
                "type": "work",
                "tools": [
                    client_tool(),
                    {"tool": "apply_patch", "node_id": "fix", "input": "*** Begin Patch"},
                    {"tool": "lookup", "namespace": "mcp__sample__", "node_id": "work", "input": {"value": "b"}}
                ]
            })
            .to_string(),
        )
        .unwrap();
    assert_eq!(plan.sequence_type, "work");
    assert_eq!(plan.tools.len(), 3);
    let namespace = &plan.tools[2];
    assert_eq!(
        namespace.tool_name,
        ToolName::namespaced("mcp__sample__", "lookup")
    );
}

#[test]
fn tool_shapes_are_exact_and_namespace_identity_is_lossless() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    for invalid in [
        json!({"type": "work", "tools": [{"tool": "missing", "node_id": "n", "input": {}}]}),
        json!({"type": "work", "tools": [{"tool": "mcp__sample__lookup", "node_id": "n", "input": {"value": "v"}}]}),
        json!({"type": "work", "tools": [{"tool": "lookup", "node_id": "n", "input": {"value": "v"}}]}),
        json!({"type": "work", "tools": [{"tool": "web_search", "node_ids": ["n"]}]}),
        json!({"type": "work", "tools": [{"tool": "web_search", "execution": "requested", "node_ids": ["n"]}]}),
        json!({"type": "work", "tools": [{"tool": "web_search", "execution": "already_executed", "node_ids": ["n"], "input": {"queries": ["x"]}}]}),
        json!({"type": "work", "tools": [{"tool": "web_search", "execution": "already_executed", "node_ids": []}]}),
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
                &json!({"type": "work", "tools": [{"tool": "web_search", "execution": "already_executed", "node_ids": ["n"]}]})
                    .to_string()
            )
            .is_err()
    );
}
