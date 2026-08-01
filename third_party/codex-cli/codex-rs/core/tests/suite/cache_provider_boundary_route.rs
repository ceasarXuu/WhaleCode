use codex_config::build_cli_overrides_layer;
use codex_config::config_toml::ConfigToml;
use codex_model_provider_info::DEEPSEEK_PROVIDER_ID;
use codex_model_provider_info::ModelProviderInfo;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use toml::Value as TomlValue;

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

#[test]
fn provider_boundary_alias_matches_builtin_deepseek_runtime_fields() {
    let route = load_route();
    assert_eq!(route.logical_provider_id, DEEPSEEK_PROVIDER_ID);
    assert_ne!(route.transport_provider_id, DEEPSEEK_PROVIDER_ID);
    assert_eq!(route.intentional_differences, ["provider_id", "base_url"]);

    let overrides = vec![
        (
            "model_provider".to_string(),
            TomlValue::String(route.transport_provider_id.clone()),
        ),
        (
            format!("model_providers.{}.name", route.transport_provider_id),
            TomlValue::String(route.name),
        ),
        (
            format!("model_providers.{}.base_url", route.transport_provider_id),
            TomlValue::String(route.base_url.clone()),
        ),
        (
            format!("model_providers.{}.env_key", route.transport_provider_id),
            TomlValue::String(route.env_key),
        ),
        (
            format!(
                "model_providers.{}.env_key_instructions",
                route.transport_provider_id
            ),
            TomlValue::String(route.env_key_instructions),
        ),
        (
            format!("model_providers.{}.wire_api", route.transport_provider_id),
            TomlValue::String(route.wire_api),
        ),
    ];
    let config: ConfigToml = build_cli_overrides_layer(&overrides)
        .try_into()
        .expect("deserialize provider boundary CLI overrides");
    assert_eq!(
        config.model_provider.as_deref(),
        Some(route.transport_provider_id.as_str())
    );
    let actual = config
        .model_providers
        .get(&route.transport_provider_id)
        .expect("transport provider exists");
    let mut expected = ModelProviderInfo::create_deepseek_provider();
    expected.base_url = Some(route.base_url);
    assert_eq!(actual, &expected);
    assert!(actual.is_deepseek());
}
