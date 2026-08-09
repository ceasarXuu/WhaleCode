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
use serde_json::json;

use super::*;

fn function(name: &str) -> ResponsesApiTool {
    ResponsesApiTool {
        name: name.into(),
        description: format!("Run {name}."),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([("value".into(), JsonSchema::string(None))]),
            Some(vec!["value".into()]),
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

fn exec_command() -> ResponsesApiTool {
    ResponsesApiTool {
        name: "exec_command".into(),
        description: "Run a shell command.".into(),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([("cmd".into(), JsonSchema::string(None))]),
            Some(vec!["cmd".into()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
        defer_loading: None,
    }
}

fn specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec::Function(exec_command()),
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
    ]
}

#[test]
fn declaration_is_deterministic_and_exposes_each_contract_once() {
    let first = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let second = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let first_json = serde_json::to_vec(first.declaration()).unwrap();
    let second_json = serde_json::to_vec(second.declaration()).unwrap();
    assert_eq!(first_json, second_json);
    assert_eq!(first.capability_identity(), second.capability_identity());
    assert_eq!(first.capability_identity().len(), 64);

    let value = serde_json::to_value(first.declaration()).unwrap();
    assert_eq!(value["name"], "taskspace_exec");
    let rendered = value.to_string();
    for name in [
        "initialize_map",
        "update_map",
        "read_map",
        "reopen_map",
        "finish_map",
        "read_file",
        "apply_patch",
        "mcp__sample__",
        "lookup",
        "tool_search",
    ] {
        assert!(rendered.contains(name), "missing {name} from {rendered}");
    }
    assert!(!rendered.contains("\"exec\""));
    assert!(!rendered.contains("\"wait\""));
    assert!(!rendered.contains("mcp__sample__lookup"));
    assert!(!rendered.contains("version"));
    assert!(!rendered.contains("capability_id"));
    assert!(!rendered.contains("revision"));

    let description = value["description"].as_str().unwrap();
    assert!(description.contains("single top-level entry point"));
    assert!(description.contains("First-turn initialization and work example"));
    assert!(description.contains("node_id` is TaskSpace ownership metadata"));
    assert!(description.contains("The Runtime does not add, infer, reorder, or repair"));
    assert_eq!(description.matches("First-turn initialization").count(), 1);
    assert!(!description.contains(r#"{\"tool\""#));
}

#[test]
fn rendered_first_turn_example_uses_the_same_catalog_contract() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let example = canonical_first_turn_example();
    let plan = catalog.decode_plan(&example.to_string()).unwrap();

    assert_eq!(plan.calls.len(), 2);
    assert!(matches!(plan.calls[0], ExecCall::Map(_)));
    let ExecCall::Client(client) = &plan.calls[1] else {
        panic!("expected client work after map initialization")
    };
    assert_eq!(client.display_name, "exec_command");
    assert_eq!(client.node_id, "inspect");
    assert_eq!(
        client.input,
        ClientCallInput::Function(json!({"cmd": "pwd"}))
    );
}

#[test]
fn capability_identity_changes_with_dispatch_or_hosted_semantics() {
    let baseline = TaskSpaceExecCatalog::build(&specs()).unwrap();

    let mut changed_output = specs();
    let ToolSpec::Function(tool) = &mut changed_output[0] else {
        panic!("expected function")
    };
    tool.output_schema = Some(json!({"type": "string"}));
    let changed_output = TaskSpaceExecCatalog::build(&changed_output).unwrap();
    assert_ne!(
        baseline.capability_identity(),
        changed_output.capability_identity()
    );

    let mut changed_hosted = specs();
    let ToolSpec::ImageGeneration { output_format } = changed_hosted
        .iter_mut()
        .find(|spec| matches!(spec, ToolSpec::ImageGeneration { .. }))
        .unwrap()
    else {
        panic!("expected image generation")
    };
    *output_format = "jpeg".into();
    let changed_hosted = TaskSpaceExecCatalog::build(&changed_hosted).unwrap();
    assert_ne!(
        baseline.capability_identity(),
        changed_hosted.capability_identity()
    );
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
    let initial_json = serde_json::to_string(initial.declaration()).unwrap();
    assert!(initial_json.contains("always_visible"));
    assert!(!initial_json.contains("deferred_plain"));
    assert!(!initial_json.contains("selected_child"));

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
    let loaded_json = serde_json::to_string(loaded.declaration()).unwrap();
    assert!(loaded_json.contains("always_visible"));
    assert!(loaded_json.contains("selected_child"));
    assert!(!loaded_json.contains("deferred_plain"));
    assert_ne!(initial.capability_identity(), loaded.capability_identity());
}

#[test]
fn decoder_preserves_mixed_map_function_freeform_and_namespace_calls() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let plan = catalog
        .decode_plan(
            &json!({
                "calls": [
                    {"tool": "read_map", "arguments": {}},
                    {"tool": "read_file", "node_id": "inspect", "arguments": {"value": "a"}},
                    {"tool": "apply_patch", "node_id": "fix", "input": "*** Begin Patch"},
                    {"tool": "lookup", "namespace": "mcp__sample__", "node_id": "inspect", "arguments": {"value": "b"}}
                ],
                "hosted_bindings": [
                    {"tool": "web_search", "node_ids": ["inspect", "fix"]},
                    {"tool": "image_generation", "node_ids": ["inspect"]}
                ]
            })
            .to_string(),
        )
        .unwrap();

    assert!(matches!(plan.calls[0], ExecCall::Map(_)));
    let ExecCall::Client(function) = &plan.calls[1] else {
        panic!("expected function call")
    };
    assert_eq!(function.display_name, "read_file");
    assert_eq!(function.node_id, "inspect");
    assert_eq!(
        function.input,
        ClientCallInput::Function(json!({"value": "a"}))
    );
    let ExecCall::Client(freeform) = &plan.calls[2] else {
        panic!("expected freeform call")
    };
    assert_eq!(
        freeform.input,
        ClientCallInput::Freeform("*** Begin Patch".into())
    );
    let ExecCall::Client(namespace) = &plan.calls[3] else {
        panic!("expected namespace call")
    };
    assert_eq!(
        namespace.tool_name.namespace.as_deref(),
        Some("mcp__sample__")
    );
    assert_eq!(namespace.tool_name.name, "lookup");
    assert_eq!(plan.hosted_bindings[0].node_ids, vec!["inspect", "fix"]);
}

#[test]
fn native_identity_allows_same_leaf_across_plain_and_namespaced_tools() {
    let catalog = TaskSpaceExecCatalog::build(&[
        ToolSpec::Function(function("lookup")),
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "alpha".into(),
            description: "Alpha namespace.".into(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("lookup"))],
        }),
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "beta".into(),
            description: "Beta namespace.".into(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("lookup"))],
        }),
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "map_tools".into(),
            description: "Map-named namespace child.".into(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("read_map"))],
        }),
    ])
    .unwrap();

    let plan = catalog
        .decode_plan(
            &json!({
                "calls": [
                    {"tool": "lookup", "node_id": "plain", "arguments": {"value": "p"}},
                    {"tool": "lookup", "namespace": "alpha", "node_id": "a", "arguments": {"value": "a"}},
                    {"tool": "lookup", "namespace": "beta", "node_id": "b", "arguments": {"value": "b"}},
                    {"tool": "read_map", "namespace": "map_tools", "node_id": "m", "arguments": {"value": "m"}}
                ],
                "hosted_bindings": []
            })
            .to_string(),
        )
        .unwrap();

    let identities = plan
        .calls
        .into_iter()
        .map(|call| match call {
            ExecCall::Client(call) => call.tool_name,
            ExecCall::Map(_) => panic!("namespaced read_map must remain a client Tool"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        identities,
        vec![
            ToolName::plain("lookup"),
            ToolName::namespaced("alpha", "lookup"),
            ToolName::namespaced("beta", "lookup"),
            ToolName::namespaced("map_tools", "read_map"),
        ]
    );
}

#[test]
fn decoder_rejects_flattened_namespace_alias_and_inexact_namespace_shape() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let flattened = catalog
        .decode_plan(
            r#"{"calls":[{"tool":"mcp__sample__lookup","node_id":"n","arguments":{"value":"v"}}],"hosted_bindings":[]}"#,
        )
        .unwrap_err();
    assert_eq!(
        flattened,
        TaskSpaceExecPlanDecodeError::UnknownTool {
            index: 0,
            tool: "mcp__sample__lookup".into(),
        }
    );

    for invalid in [
        r#"{"calls":[{"tool":"lookup","node_id":"n","arguments":{"value":"v"}}],"hosted_bindings":[]}"#,
        r#"{"calls":[{"tool":"lookup","namespace":"wrong","node_id":"n","arguments":{"value":"v"}}],"hosted_bindings":[]}"#,
        r#"{"calls":[{"tool":"read_file","namespace":null,"node_id":"n","arguments":{"value":"v"}}],"hosted_bindings":[]}"#,
    ] {
        assert!(catalog.decode_plan(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn decoder_preserves_client_tool_search_as_a_native_identity() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let plan = catalog
        .decode_plan(
            r#"{"calls":[{"tool":"tool_search","node_id":"inspect","arguments":{"query":"calendar"}}],"hosted_bindings":[]}"#,
        )
        .unwrap();

    let ExecCall::Client(call) = &plan.calls[0] else {
        panic!("expected client Tool Search call")
    };
    assert_eq!(call.tool_name, codex_tools::ToolName::plain("tool_search"));
    assert_eq!(
        call.input,
        ClientCallInput::Function(json!({"query": "calendar"}))
    );
}

#[test]
fn hosted_only_is_valid_but_completely_empty_plan_is_rejected() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let hosted = catalog
        .decode_plan(
            &json!({
                "calls": [],
                "hosted_bindings": [{"tool": "web_search", "node_ids": ["research"]}]
            })
            .to_string(),
        )
        .unwrap();
    assert!(hosted.calls.is_empty());
    assert_eq!(hosted.hosted_bindings.len(), 1);
    assert_eq!(
        catalog.decode_plan(r#"{"calls":[],"hosted_bindings":[]}"#),
        Err(TaskSpaceExecPlanDecodeError::EmptyPlan)
    );
}

#[test]
fn decoder_rejects_unknown_tools_and_shape_drift() {
    let catalog = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let unknown = catalog
        .decode_plan(
            r#"{"calls":[{"tool":"missing","node_id":"n","arguments":{}}],"hosted_bindings":[]}"#,
        )
        .unwrap_err();
    assert_eq!(
        unknown,
        TaskSpaceExecPlanDecodeError::UnknownTool {
            index: 0,
            tool: "missing".into()
        }
    );

    for invalid in [
        r#"{"calls":[{"tool":"read_file","node_id":"n","arguments":{},"revision":1}],"hosted_bindings":[]}"#,
        r#"{"calls":[{"tool":"apply_patch","node_id":"n","arguments":{}}],"hosted_bindings":[]}"#,
        r#"{"calls":[],"hosted_bindings":[{"tool":"unknown","node_ids":["n"]}]}"#,
        r#"{"calls":[],"hosted_bindings":[],"version":"v1"}"#,
    ] {
        assert!(catalog.decode_plan(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn catalog_rejects_collisions_and_unprojectable_client_specs() {
    let collision = vec![ToolSpec::Function(function("read_map"))];
    assert_eq!(
        TaskSpaceExecCatalog::build(&collision).unwrap_err(),
        TaskSpaceExecCatalogError::MapCapabilityCollision {
            public_name: "read_map".into()
        }
    );

    let duplicate_namespace = vec![
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
    assert_eq!(
        TaskSpaceExecCatalog::build(&duplicate_namespace).unwrap_err(),
        TaskSpaceExecCatalogError::DuplicateCapability {
            tool_name: ToolName::namespaced("same", "lookup"),
        }
    );

    assert_eq!(
        TaskSpaceExecCatalog::build(&[ToolSpec::LocalShell {}]).unwrap_err(),
        TaskSpaceExecCatalogError::UnsupportedToolSpec {
            tool_name: "local_shell".into()
        }
    );
}

#[test]
fn capability_order_is_independent_of_source_order() {
    let mut reversed = specs();
    reversed.reverse();
    let left = TaskSpaceExecCatalog::build(&specs()).unwrap();
    let right = TaskSpaceExecCatalog::build(&reversed).unwrap();
    assert_eq!(
        serde_json::to_vec(left.declaration()).unwrap(),
        serde_json::to_vec(right.declaration()).unwrap()
    );
}

#[test]
fn declaration_without_hosted_tools_remains_valid_and_runtime_rejects_bindings() {
    let catalog =
        TaskSpaceExecCatalog::build(&[ToolSpec::Function(function("read_file"))]).unwrap();
    let declaration = serde_json::to_value(catalog.declaration()).unwrap();
    assert!(!declaration.to_string().contains(r#""enum":[]"#));
    assert!(
        catalog
            .decode_plan(
                r#"{"calls":[],"hosted_bindings":[{"tool":"web_search","node_ids":["n"]}]}"#
            )
            .is_err()
    );
}
