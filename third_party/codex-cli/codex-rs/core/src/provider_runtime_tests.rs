use super::*;
use crate::config::ConfigBuilder;
use codex_login::test_support::auth_manager_from_optional_auth;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn explicit_access_method_is_new_session_route_authority() -> std::io::Result<()> {
    let codex_home = tempfile::tempdir()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        "model_provider = \"openai\"\nmodel_provider_access_method = \"chatgpt\"\n",
    )?;
    let config = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .build()
        .await?;
    let auth_manager = auth_manager_from_optional_auth(None);
    let registry = ProviderRuntimeRegistry::from_config(&config, Arc::clone(&auth_manager));

    assert_eq!(
        registry.initial_route(
            &config.model_provider_id,
            config.model_provider_access_method,
            &config.model_provider,
            &config.model_providers,
            auth_manager.as_ref(),
        ),
        Some(ProviderRoute::new(
            OPENAI_PROVIDER_ID,
            ProviderAccessMethod::Chatgpt,
        ))
    );

    Ok(())
}
