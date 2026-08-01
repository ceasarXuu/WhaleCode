#![cfg(not(target_os = "windows"))]

use super::cache_payload_contract::completed_response_stream;
use super::cache_payload_contract::configure_deepseek_responses;
use super::cache_payload_contract::provider_identity;
use super::cache_payload_contract::stabilize_fixture_inputs;
use super::cache_payload_contract::submit_turn;
use super::cache_payload_contract::value_contains_text;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::apps_test_server::AppsTestServer;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::cache_payload::render_cache_snapshot;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[derive(Clone, Copy)]
enum CapabilityScenario {
    Default,
    App,
    Plugin,
}

fn write_plugin_fixture(home: &TempDir) -> anyhow::Result<()> {
    let plugin_root = home.path().join("plugins/cache/test/sample/local");
    std::fs::create_dir_all(plugin_root.join(".codex-plugin"))?;
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"sample","description":"inspect sample data"}"#,
    )?;
    let skill_dir = plugin_root.join("skills/sample-search");
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\ndescription: inspect sample data\n---\n\n# Sample search\n",
    )?;
    std::fs::write(
        home.path().join("config.toml"),
        "[features]\nplugins = true\n\n[plugins.\"sample@test\"]\nenabled = true\n",
    )?;
    Ok(())
}

async fn submit_capability_first_turn(
    test: &core_test_support::test_codex::TestCodex,
    scenario: CapabilityScenario,
) -> anyhow::Result<()> {
    let mut items = vec![UserInput::Text {
        text: "capability contract turn one".to_string(),
        text_elements: Vec::new(),
    }];
    match scenario {
        CapabilityScenario::App => items.push(UserInput::Mention {
            name: "Google Calendar".to_string(),
            path: "app://calendar".to_string(),
        }),
        CapabilityScenario::Plugin => items.push(UserInput::Mention {
            name: "sample".to_string(),
            path: "plugin://sample@test".to_string(),
        }),
        CapabilityScenario::Default => {}
    }
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items,
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}

async fn capture_capability_request_pair(scenario: CapabilityScenario) -> anyhow::Result<Value> {
    let server = start_mock_server().await;
    let app_server = if matches!(scenario, CapabilityScenario::App) {
        Some(AppsTestServer::mount_with_connector_name(&server, "Google Calendar").await?)
    } else {
        None
    };
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    completed_response_stream("resp-capability-contract"),
                    "text/event-stream",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;

    let codex_home = Arc::new(TempDir::new()?);
    if matches!(scenario, CapabilityScenario::Plugin) {
        write_plugin_fixture(codex_home.as_ref())?;
    }
    let mut builder = test_codex().with_home(Arc::clone(&codex_home));
    if matches!(scenario, CapabilityScenario::App) {
        builder = builder.with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    }
    let app_base_url = app_server.map(|server| server.chatgpt_base_url);
    let test = builder
        .with_config(move |config| {
            configure_deepseek_responses(config);
            config.cwd = AbsolutePathBuf::try_from(PathBuf::from("/tmp"))
                .expect("fixed capability contract cwd");
            if let Some(base_url) = app_base_url {
                config
                    .features
                    .enable(Feature::Apps)
                    .expect("Apps feature must be configurable");
                config.chatgpt_base_url = base_url;
            }
        })
        .build(&server)
        .await?;
    submit_capability_first_turn(&test, scenario).await?;
    submit_turn(&test, "capability contract turn two").await?;

    let all_requests = server
        .received_requests()
        .await
        .expect("capability final-wire requests");
    let requests = all_requests
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
    let codex_home_path = test.codex_home_path().to_string_lossy();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    stabilize_fixture_inputs(
        &mut snapshot,
        &[
            (codex_home_path.as_ref(), "<CODEX_HOME>"),
            (&source_root, "<CODEX_SOURCE_ROOT>"),
        ],
    );
    Ok(snapshot)
}

fn context_contains(request: &Value, marker: &str) -> bool {
    value_contains_text(&request["instructions"], marker)
        || value_contains_text(&request["input"], marker)
}

fn tool_names(request: &Value) -> Vec<&str> {
    request["tools"]
        .as_array()
        .expect("request tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apps_and_plugins_have_independent_final_wire_contracts() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let default = capture_capability_request_pair(CapabilityScenario::Default).await?;
    let app = capture_capability_request_pair(CapabilityScenario::App).await?;
    let plugin = capture_capability_request_pair(CapabilityScenario::Plugin).await?;

    for request_key in ["request_1", "request_2"] {
        assert!(!context_contains(
            &default[request_key],
            "<apps_instructions>"
        ));
        assert!(!context_contains(
            &default[request_key],
            "<plugins_instructions>"
        ));
        assert!(context_contains(&app[request_key], "<apps_instructions>"));
        assert!(context_contains(
            &plugin[request_key],
            "<plugins_instructions>"
        ));
        assert!(context_contains(
            &plugin[request_key],
            "sample:sample-search: inspect sample data"
        ));
        assert_eq!(
            tool_names(&plugin[request_key]),
            tool_names(&default[request_key])
        );
    }
    let app_tool_names = tool_names(&app["request_1"]);
    assert!(
        app_tool_names
            .iter()
            .any(|name| name.starts_with("mcp__codex_apps__")),
        "expected explicit App tools, got {app_tool_names:?}"
    );
    assert!(context_contains(
        &plugin["request_1"],
        "Skills from this plugin"
    ));

    let app_rendered = render_cache_snapshot("app_two_request_final_wire", &app)?;
    let plugin_rendered = render_cache_snapshot("plugin_two_request_final_wire", &plugin)?;
    insta::assert_snapshot!("app_two_request_final_wire", app_rendered);
    insta::assert_snapshot!("plugin_two_request_final_wire", plugin_rendered);
    Ok(())
}
