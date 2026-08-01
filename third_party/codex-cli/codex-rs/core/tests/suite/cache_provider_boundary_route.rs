use super::cache_payload_contract::completed_response_stream;
use super::cache_payload_contract::stabilize_fixture_inputs;
use super::cache_payload_contract::submit_turn;
use codex_core::config::ConfigBuilder;
use codex_model_provider_info::DEEPSEEK_PROVIDER_ID;
use codex_model_provider_info::ModelProviderInfo;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use toml::Value as TomlValue;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[derive(Debug, Deserialize)]
struct ContainerContract {
    provider_boundary: ProviderBoundaryRoute,
}

#[derive(Debug, Deserialize)]
struct ProviderBoundaryRoute {
    logical_provider_id: String,
    transport_provider_id: String,
    name: String,
    base_url: String,
    env_key: String,
    env_key_instructions: String,
    wire_api: String,
    intentional_differences: Vec<String>,
}

fn load_route() -> ProviderBoundaryRoute {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../benchmarks/taskspace/container-runtime-contract.json");
    let content = fs::read_to_string(path).expect("read container runtime contract");
    serde_json::from_str::<ContainerContract>(&content)
        .expect("parse container runtime contract")
        .provider_boundary
}

fn alias_overrides(route: &ProviderBoundaryRoute) -> Vec<(String, TomlValue)> {
    let provider_id = &route.transport_provider_id;
    vec![
        (
            "model_provider".to_string(),
            TomlValue::String(provider_id.clone()),
        ),
        (
            format!("model_providers.{provider_id}.name"),
            TomlValue::String(route.name.clone()),
        ),
        (
            format!("model_providers.{provider_id}.base_url"),
            TomlValue::String(route.base_url.clone()),
        ),
        (
            format!("model_providers.{provider_id}.env_key"),
            TomlValue::String(route.env_key.clone()),
        ),
        (
            format!("model_providers.{provider_id}.env_key_instructions"),
            TomlValue::String(route.env_key_instructions.clone()),
        ),
        (
            format!("model_providers.{provider_id}.wire_api"),
            TomlValue::String(route.wire_api.clone()),
        ),
    ]
}

async fn resolved_alias(route: &ProviderBoundaryRoute) -> anyhow::Result<ModelProviderInfo> {
    let home = TempDir::new()?;
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .fallback_cwd(Some(PathBuf::from("/tmp")))
        .cli_overrides(alias_overrides(route))
        .build()
        .await?;
    assert_eq!(config.model_provider_id, route.transport_provider_id);
    Ok(config.model_provider)
}

async fn capture_normal_request(
    provider_id: String,
    mut provider: ModelProviderInfo,
) -> anyhow::Result<serde_json::Value> {
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    completed_response_stream("resp-provider-route"),
                    "text/event-stream",
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    provider.env_key = None;
    provider.env_key_instructions = None;
    provider.experimental_bearer_token = Some("test-deepseek-key".to_string());
    let test = test_codex()
        .with_config(move |config| {
            provider.base_url = config.model_provider.base_url.clone();
            config.model_provider_id = provider_id;
            config.model_provider = provider;
            config.model = Some("deepseek-v4-flash".to_string());
            config.cwd =
                AbsolutePathBuf::try_from(PathBuf::from("/tmp")).expect("fixed provider route cwd");
        })
        .build(&server)
        .await?;
    submit_turn(&test, "provider route contract").await?;
    let requests = server
        .received_requests()
        .await
        .expect("final-wire request");
    assert_eq!(requests.len(), 1);
    let mut body = FinalWireEvidence::from_raw_body(&requests[0].body)?.structured_body;
    let codex_home = test.codex_home_path().to_string_lossy().into_owned();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    stabilize_fixture_inputs(
        &mut body,
        &[
            (&codex_home, "<CODEX_HOME>"),
            (&source_root, "<CODEX_SOURCE_ROOT>"),
        ],
    );
    Ok(body)
}

#[tokio::test]
async fn provider_boundary_alias_matches_builtin_deepseek_runtime_fields() -> anyhow::Result<()> {
    let route = load_route();
    assert_eq!(route.logical_provider_id, DEEPSEEK_PROVIDER_ID);
    assert_ne!(route.transport_provider_id, DEEPSEEK_PROVIDER_ID);
    assert_eq!(route.intentional_differences, ["provider_id", "base_url"]);

    let actual = resolved_alias(&route).await?;
    let mut expected = ModelProviderInfo::create_deepseek_provider();
    expected.base_url = Some(route.base_url);
    assert_eq!(actual, expected);
    assert!(actual.is_deepseek());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provider_boundary_alias_preserves_normal_final_wire() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let route = load_route();
    let alias = resolved_alias(&route).await?;
    let builtin = ModelProviderInfo::create_deepseek_provider();

    let builtin_body = capture_normal_request(DEEPSEEK_PROVIDER_ID.to_string(), builtin).await?;
    let alias_body = capture_normal_request(route.transport_provider_id, alias).await?;
    assert_eq!(alias_body, builtin_body);
    Ok(())
}
