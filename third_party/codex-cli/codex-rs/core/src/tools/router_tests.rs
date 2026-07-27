use std::collections::HashSet;
use std::sync::Arc;

use crate::session::tests::make_session_and_context;
use crate::tools::context::ToolPayload;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::ShellCommandToolCallParams;
use codex_tools::ToolName;

use super::ToolCall;
use super::ToolRouter;
use super::ToolRouterParams;

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
                tool_name: ToolName::plain(*name),
                call_id: "call-parallel-tool".to_string(),
                payload: ToolPayload::Function {
                    arguments: "{}".to_string(),
                },
            })
        })
        .expect("test session should expose a parallel shell-like tool");

    assert!(!router.tool_supports_parallel(&ToolCall {
        tool_name: ToolName::namespaced("mcp__server__", parallel_tool_name),
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
        call.tool_name,
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

    assert_eq!(call.tool_name, ToolName::plain("shell_command"));
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

    assert_eq!(call.tool_name, ToolName::plain("shell_command"));
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
        tool_name: ToolName::namespaced("mcp__echo__", "query_with_delay"),
        call_id: "call-deferred".to_string(),
        payload: ToolPayload::Mcp {
            server: "echo".to_string(),
            tool: "query_with_delay".to_string(),
            raw_arguments: "{}".to_string(),
        },
    };
    assert!(router.tool_supports_parallel(&deferred_call));

    let different_server_call = ToolCall {
        tool_name: ToolName::namespaced("mcp__hello_echo__", "query_with_delay"),
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
