#![cfg(not(target_os = "windows"))]

use super::cache_payload_contract::completed_response_stream;
use super::cache_payload_contract::configure_deepseek_responses;
use super::cache_payload_contract::provider_identity;
use super::cache_payload_contract::stabilize_fixture_inputs;
use super::cache_payload_contract::submit_turn;
use codex_config::types::McpServerConfig;
use codex_config::types::McpServerTransportConfig;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::stdio_server_bin;
use core_test_support::test_codex::test_codex;
use serde_json::Value;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

async fn capture_mcp_request_pair(enabled: bool) -> anyhow::Result<Value> {
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    completed_response_stream("resp-mcp-contract"),
                    "text/event-stream",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;

    let rmcp_test_server_bin = enabled.then(stdio_server_bin).transpose()?;
    let test = test_codex()
        .with_config(move |config| {
            configure_deepseek_responses(config);
            config.cwd = AbsolutePathBuf::try_from(PathBuf::from("/tmp"))
                .expect("fixed MCP cache contract cwd");
            if let Some(command) = rmcp_test_server_bin {
                let mut servers = config.mcp_servers.get().clone();
                servers.insert(
                    "rmcp".to_string(),
                    McpServerConfig {
                        transport: McpServerTransportConfig::Stdio {
                            command,
                            args: Vec::new(),
                            env: None,
                            env_vars: Vec::new(),
                            cwd: None,
                        },
                        experimental_environment: None,
                        enabled: true,
                        required: true,
                        supports_parallel_tool_calls: false,
                        disabled_reason: None,
                        startup_timeout_sec: Some(Duration::from_secs(10)),
                        tool_timeout_sec: None,
                        default_tools_approval_mode: None,
                        enabled_tools: Some(vec!["echo".to_string()]),
                        disabled_tools: None,
                        scopes: None,
                        oauth_resource: None,
                        tools: HashMap::new(),
                    },
                );
                config
                    .mcp_servers
                    .set(servers)
                    .expect("test MCP configuration");
            }
        })
        .build(&server)
        .await?;

    submit_turn(&test, "MCP cache contract turn one").await?;
    submit_turn(&test, "MCP cache contract turn two").await?;

    let requests = server
        .received_requests()
        .await
        .expect("MCP final-wire requests")
        .into_iter()
        .filter(|request| request.url.path() == "/v1/responses")
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 2);
    let first = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    let second = FinalWireEvidence::from_raw_body(&requests[1].body)?;
    let mut snapshot = serde_json::json!({
        "provider_identity": provider_identity(&test.config),
        "request_1": first.structured_body,
        "request_2": second.structured_body,
    });
    let codex_home = test.codex_home_path().to_string_lossy();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    stabilize_fixture_inputs(
        &mut snapshot,
        &[
            (codex_home.as_ref(), "<CODEX_HOME>"),
            (&source_root, "<CODEX_SOURCE_ROOT>"),
        ],
    );
    Ok(snapshot)
}

fn remove_rmcp_namespace(request: &mut Value) -> Value {
    let tools = request["tools"].as_array_mut().expect("request tools");
    let index = tools
        .iter()
        .position(|tool| tool["type"] == "namespace" && tool["name"] == "mcp__rmcp__")
        .expect("rmcp namespace");
    tools.remove(index)
}

fn tool_identity(tool: &Value) -> &str {
    tool["name"]
        .as_str()
        .or_else(|| tool["type"].as_str())
        .expect("tool identity")
}

fn first_difference(left: &Value, right: &Value, path: &str) -> Option<String> {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => left
            .keys()
            .chain(right.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .find_map(|key| match (left.get(key), right.get(key)) {
                (Some(left), Some(right)) => {
                    first_difference(left, right, &format!("{path}/{key}"))
                }
                _ => Some(format!("{path}/{key}: field presence differs")),
            }),
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!(
                    "{path}: array lengths differ ({} != {})",
                    left.len(),
                    right.len()
                ));
            }
            left.iter()
                .zip(right)
                .enumerate()
                .find_map(|(index, (left, right))| {
                    first_difference(left, right, &format!("{path}/{index}"))
                })
        }
        _ if left == right => None,
        _ => Some(format!("{path}: {left:?} != {right:?}")),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_tools_have_an_independent_final_wire_contract() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let default = capture_mcp_request_pair(false).await?;
    let mcp = capture_mcp_request_pair(true).await?;

    for request_key in ["request_1", "request_2"] {
        let mut normalized_mcp = mcp[request_key].clone();
        let namespace = remove_rmcp_namespace(&mut normalized_mcp);
        assert_eq!(namespace["name"], "mcp__rmcp__");
        assert_eq!(namespace["type"], "namespace");
        let children = namespace["tools"].as_array().expect("namespace tools");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["type"], "function");
        assert_eq!(children[0]["name"], "echo");

        let default_tools = default[request_key]["tools"]
            .as_array()
            .expect("default tools");
        let mcp_tools = normalized_mcp["tools"].as_array().expect("MCP tools");
        for default_tool in default_tools {
            let name = tool_identity(default_tool);
            let matching_mcp_tool = mcp_tools
                .iter()
                .find(|tool| tool_identity(tool) == name)
                .unwrap_or_else(|| panic!("ordinary tool missing with MCP enabled: {name}"));
            assert_eq!(matching_mcp_tool, default_tool);
        }
        let mut added_tool_names = mcp_tools
            .iter()
            .filter(|tool| {
                !default_tools
                    .iter()
                    .any(|default_tool| tool_identity(default_tool) == tool_identity(tool))
            })
            .map(tool_identity)
            .collect::<Vec<_>>();
        added_tool_names.sort_unstable();
        assert_eq!(
            added_tool_names,
            vec![
                "list_mcp_resource_templates",
                "list_mcp_resources",
                "read_mcp_resource"
            ]
        );

        let mut normalized_default = default[request_key].clone();
        normalized_default["tools"] = Value::Null;
        normalized_mcp["tools"] = Value::Null;
        assert_eq!(
            first_difference(&normalized_mcp, &normalized_default, ""),
            None
        );
    }

    insta::assert_snapshot!(
        "mcp_two_request_final_wire",
        serde_json::to_string_pretty(&mcp)?
    );
    Ok(())
}
