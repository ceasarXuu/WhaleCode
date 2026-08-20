use std::collections::HashSet;
use std::sync::Arc;

use codex_tools::FreeformTool;
use codex_tools::FreeformToolFormat;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::function_tool::FunctionCallError;
use crate::session::tests::make_session_and_context;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolCallSource;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::nested_call::build_native_nested_tool_call;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::registry::ToolRegistryBuilder;
use crate::turn_diff_tracker::TurnDiffTracker;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::ShellCommandToolCallParams;
use codex_tools::ToolName;

use super::ToolCall;
use super::ToolRouter;
use super::ToolRouterParams;

#[derive(Default)]
struct RecordingHandler {
    calls: Mutex<Vec<(String, String)>>,
}

impl ToolHandler for RecordingHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(
            payload,
            ToolPayload::Function { .. } | ToolPayload::Custom { .. }
        )
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        self.calls.lock().await.push((
            invocation.tool_name.display(),
            invocation.payload.log_payload().into_owned(),
        ));
        Ok(FunctionToolOutput::from_text(
            "recorded".to_string(),
            Some(true),
        ))
    }
}

#[tokio::test]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "test builds a router from session-owned MCP manager state"
)]
async fn parallel_support_does_not_match_namespaced_local_tool_names() -> anyhow::Result<()> {
    let (session, turn) = make_session_and_context().await;
    let mcp_tools = session
        .services
        .mcp_connection_manager
        .read()
        .await
        .list_all_tools()
        .await;
    let router = ToolRouter::from_config(
        &turn.tools_config,
        ToolRouterParams {
            deferred_mcp_tools: None,
            mcp_tools: Some(mcp_tools),
            unavailable_called_tools: Vec::new(),
            parallel_mcp_server_names: HashSet::new(),
            discoverable_tools: None,
            dynamic_tools: turn.dynamic_tools.as_slice(),
        },
    );

    let parallel_tool_name = ["shell", "local_shell", "exec_command", "shell_command"]
        .into_iter()
        .find(|name| {
            router.tool_supports_parallel(&ToolCall {
                provider_tool_name: ToolName::plain(*name),
                dispatch_tool_name: ToolName::plain(*name),
                call_id: "call-parallel-tool".to_string(),
                payload: ToolPayload::Function {
                    arguments: "{}".to_string(),
                },
            })
        })
        .expect("test session should expose a parallel shell-like tool");

    assert!(!router.tool_supports_parallel(&ToolCall {
        provider_tool_name: ToolName::namespaced("mcp__server__", parallel_tool_name),
        dispatch_tool_name: ToolName::namespaced("mcp__server__", parallel_tool_name),
        call_id: "call-namespaced-tool".to_string(),
        payload: ToolPayload::Function {
            arguments: "{}".to_string(),
        },
    }));

    Ok(())
}

#[tokio::test]
async fn build_tool_call_uses_namespace_for_registry_name() -> anyhow::Result<()> {
    let (session, _) = make_session_and_context().await;
    let session = Arc::new(session);
    let tool_name = "create_event".to_string();

    let call = ToolRouter::build_tool_call(
        &session,
        ResponseItem::FunctionCall {
            id: None,
            name: tool_name.clone(),
            namespace: Some("mcp__codex_apps__calendar".to_string()),
            arguments: "{}".to_string(),
            call_id: "call-namespace".to_string(),
        },
    )
    .await?
    .expect("function_call should produce a tool call");

    assert_eq!(
        call.dispatch_tool_name,
        ToolName::namespaced("mcp__codex_apps__calendar", tool_name)
    );
    assert_eq!(call.call_id, "call-namespace");
    match call.payload {
        ToolPayload::Function { arguments } => {
            assert_eq!(arguments, "{}");
        }
        other => panic!("expected function payload, got {other:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn build_tool_call_normalizes_exec_command_alias_to_shell_command() -> anyhow::Result<()> {
    let (session, _) = make_session_and_context().await;
    let session = Arc::new(session);

    let call = ToolRouter::build_tool_call(
        &session,
        ResponseItem::FunctionCall {
            id: None,
            name: "exec_command".to_string(),
            namespace: None,
            arguments: serde_json::json!({
                "cmd": "cat README.md",
                "workdir": "."
            })
            .to_string(),
            call_id: "call-exec-command".to_string(),
        },
    )
    .await?
    .expect("function_call should produce a tool call");

    assert_eq!(
        call.provider_tool_name,
        ToolName::plain("exec_command"),
        "provider-visible identity must survive internal alias normalization"
    );
    assert_eq!(call.dispatch_tool_name, ToolName::plain("shell_command"));
    match call.payload {
        ToolPayload::Function { arguments } => {
            let params: ShellCommandToolCallParams = serde_json::from_str(&arguments)?;
            assert_eq!(params.command, "cat README.md");
            assert_eq!(params.workdir.as_deref(), Some("."));
        }
        other => panic!("expected function payload, got {other:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn build_tool_call_normalizes_read_file_alias_to_shell_command() -> anyhow::Result<()> {
    let (session, _) = make_session_and_context().await;
    let session = Arc::new(session);

    let call = ToolRouter::build_tool_call(
        &session,
        ResponseItem::FunctionCall {
            id: None,
            name: "read_file".to_string(),
            namespace: None,
            arguments: serde_json::json!({
                "file_path": "README.md",
                "workdir": ".",
                "timeout_ms": 30000
            })
            .to_string(),
            call_id: "call-read-file".to_string(),
        },
    )
    .await?
    .expect("function_call should produce a tool call");

    assert_eq!(
        call.provider_tool_name,
        ToolName::plain("read_file"),
        "provider-visible identity must survive internal alias normalization"
    );
    assert_eq!(call.dispatch_tool_name, ToolName::plain("shell_command"));
    match call.payload {
        ToolPayload::Function { arguments } => {
            let params: ShellCommandToolCallParams = serde_json::from_str(&arguments)?;
            assert!(params.command.contains("README.md"));
            assert!(params.command.contains("ReadFileSummaryV1"));
            assert_eq!(params.workdir.as_deref(), Some("."));
            assert_eq!(params.timeout_ms, Some(30000));
        }
        other => panic!("expected function payload, got {other:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn mcp_parallel_support_uses_exact_payload_server() -> anyhow::Result<()> {
    let (_, turn) = make_session_and_context().await;
    let router = ToolRouter::from_config(
        &turn.tools_config,
        ToolRouterParams {
            deferred_mcp_tools: None,
            mcp_tools: None,
            unavailable_called_tools: Vec::new(),
            parallel_mcp_server_names: HashSet::from(["echo".to_string()]),
            discoverable_tools: None,
            dynamic_tools: turn.dynamic_tools.as_slice(),
        },
    );

    let deferred_call = ToolCall {
        provider_tool_name: ToolName::namespaced("mcp__echo__", "query_with_delay"),
        dispatch_tool_name: ToolName::namespaced("mcp__echo__", "query_with_delay"),
        call_id: "call-deferred".to_string(),
        payload: ToolPayload::Mcp {
            server: "echo".to_string(),
            tool: "query_with_delay".to_string(),
            raw_arguments: "{}".to_string(),
        },
    };
    assert!(router.tool_supports_parallel(&deferred_call));

    let different_server_call = ToolCall {
        provider_tool_name: ToolName::namespaced("mcp__hello_echo__", "query_with_delay"),
        dispatch_tool_name: ToolName::namespaced("mcp__hello_echo__", "query_with_delay"),
        call_id: "call-other-server".to_string(),
        payload: ToolPayload::Mcp {
            server: "hello_echo".to_string(),
            tool: "query_with_delay".to_string(),
            raw_arguments: "{}".to_string(),
        },
    };
    assert!(!router.tool_supports_parallel(&different_server_call));

    Ok(())
}

#[tokio::test]
async fn ordinary_tool_payload_is_forwarded_without_taskspace_parsing() -> anyhow::Result<()> {
    let (session, _) = make_session_and_context().await;
    let session = Arc::new(session);
    let arguments = serde_json::json!({
        "account": "example",
        "taskspace_binding": "business-owned-value"
    })
    .to_string();

    let call = ToolRouter::build_tool_call(
        &session,
        ResponseItem::FunctionCall {
            id: None,
            name: "business_tool".into(),
            namespace: None,
            arguments: arguments.clone(),
            call_id: "business-call".into(),
        },
    )
    .await?
    .expect("function call");

    let ToolPayload::Function {
        arguments: forwarded,
    } = call.payload
    else {
        panic!("expected function payload");
    };
    assert_eq!(forwarded, arguments);
    Ok(())
}

#[tokio::test]
async fn current_session_config_builds_a_single_taskspace_entrypoint() -> anyhow::Result<()> {
    let (_, turn) = make_session_and_context().await;
    let standard = ToolRouter::from_config(
        &turn.tools_config,
        ToolRouterParams {
            deferred_mcp_tools: None,
            mcp_tools: None,
            unavailable_called_tools: Vec::new(),
            parallel_mcp_server_names: HashSet::new(),
            discoverable_tools: None,
            dynamic_tools: turn.dynamic_tools.as_slice(),
        },
    );
    assert!(
        standard
            .model_visible_specs()
            .iter()
            .any(|spec| !matches!(spec.name(), "web_search" | "image_generation"))
    );

    let taskspace = standard
        .into_taskspace(&[])
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let visible = taskspace
        .model_visible_specs()
        .into_iter()
        .map(|spec| spec.name().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        visible
            .iter()
            .filter(|name| name.as_str() == "taskspace_exec")
            .count(),
        1
    );
    assert!(visible.iter().all(|name| matches!(
        name.as_str(),
        "taskspace_exec" | "web_search" | "image_generation"
    )));
    let identity = taskspace
        .taskspace_capability_identity()
        .expect("TaskSpace capability identity");
    assert_eq!(identity.len(), 64);
    Ok(())
}

#[test]
fn taskspace_loaded_deferred_specs_require_a_current_handler() {
    fn router(handler: Arc<RecordingHandler>) -> ToolRouter {
        let deferred = ToolSpec::Function(ResponsesApiTool {
            name: "deferred_current".into(),
            description: "Current deferred Tool.".into(),
            strict: false,
            defer_loading: Some(true),
            parameters: JsonSchema::default(),
            output_schema: None,
        });
        let mut builder = ToolRegistryBuilder::new();
        builder.push_spec(deferred);
        builder.register_handler("deferred_current", handler);
        ToolRouter::from_builder_for_test(builder)
    }

    let loaded = vec![
        ToolSpec::Function(ResponsesApiTool {
            name: "deferred_current".into(),
            description: "Loaded current Tool.".into(),
            strict: false,
            defer_loading: Some(true),
            parameters: JsonSchema::default(),
            output_schema: None,
        }),
        ToolSpec::Function(ResponsesApiTool {
            name: "stale_without_handler".into(),
            description: "Stale Tool.".into(),
            strict: false,
            defer_loading: Some(true),
            parameters: JsonSchema::default(),
            output_schema: None,
        }),
    ];

    let initial = router(Arc::new(RecordingHandler::default()))
        .into_taskspace(&[])
        .unwrap();
    let initial_schema = serde_json::to_string(&initial.specs()).unwrap();
    assert!(!initial_schema.contains("deferred_current"));

    let loaded = router(Arc::new(RecordingHandler::default()))
        .into_taskspace(&loaded)
        .unwrap();
    let loaded_schema = serde_json::to_string(&loaded.specs()).unwrap();
    assert!(loaded_schema.contains("deferred_current"));
    assert!(!loaded_schema.contains("stale_without_handler"));
}

#[tokio::test]
async fn nested_native_calls_reuse_original_router_in_declared_order() -> anyhow::Result<()> {
    let function_name = ToolName::plain("inspect");
    let freeform_name = ToolName::plain("patch");
    let function_spec = ToolSpec::Function(ResponsesApiTool {
        name: function_name.name.clone(),
        description: String::new(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::default(),
        output_schema: None,
    });
    let freeform_spec = ToolSpec::Freeform(FreeformTool {
        name: freeform_name.name.clone(),
        description: String::new(),
        format: FreeformToolFormat {
            r#type: "grammar".into(),
            syntax: "lark".into(),
            definition: String::new(),
        },
    });
    let handler = Arc::new(RecordingHandler::default());
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec(function_spec.clone());
    builder.push_spec(freeform_spec.clone());
    builder.register_handler(function_name.clone(), Arc::clone(&handler));
    builder.register_handler(freeform_name.clone(), Arc::clone(&handler));
    let (specs, registry) = builder.build();
    let router = ToolRouter {
        registry,
        model_visible_specs: specs.iter().map(|item| item.spec.clone()).collect(),
        specs,
        parallel_mcp_server_names: HashSet::new(),
        taskspace_response_scope: None,
        taskspace_catalog: None,
    };
    let (session, turn) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    let tracker = Arc::new(Mutex::new(TurnDiffTracker::new()));

    let calls = [
        build_native_nested_tool_call(
            &function_spec,
            function_name,
            "nested-function".into(),
            Some(serde_json::json!({"path": "README.md"})),
        )
        .map_err(anyhow::Error::msg)?,
        build_native_nested_tool_call(
            &freeform_spec,
            freeform_name,
            "nested-freeform".into(),
            Some(serde_json::Value::String("patch body".into())),
        )
        .map_err(anyhow::Error::msg)?,
    ];

    for call in calls {
        router
            .dispatch_tool_call_with_code_mode_result(
                Arc::clone(&session),
                Arc::clone(&turn),
                CancellationToken::new(),
                Arc::clone(&tracker),
                call,
                ToolCallSource::Direct,
            )
            .await?;
    }

    assert_eq!(
        *handler.calls.lock().await,
        vec![
            ("inspect".to_string(), r#"{"path":"README.md"}"#.to_string()),
            ("patch".to_string(), "patch body".to_string()),
        ]
    );
    Ok(())
}
