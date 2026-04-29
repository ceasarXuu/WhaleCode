use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchProvider;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use std::path::Path;

const SEARCH_PROVIDER_MANIFEST_ORDER: [WebSearchProvider; 6] = [
    WebSearchProvider::Github,
    WebSearchProvider::Exa,
    WebSearchProvider::Tavily,
    WebSearchProvider::Brave,
    WebSearchProvider::StackExchange,
    WebSearchProvider::Jina,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebToolManifestAvailability {
    pub(crate) search_providers: Vec<WebSearchProvider>,
}

pub(crate) fn resolve_web_tool_manifest_availability(
    config: &WebSearchConfig,
    codex_home: &Path,
) -> WebToolManifestAvailability {
    let secrets = SecretsManager::new(codex_home.to_path_buf(), SecretsBackendKind::Local);
    WebToolManifestAvailability {
        search_providers: resolve_search_providers_for_manifest(config, |env_name| {
            has_configured_secret(env_name, &secrets)
        }),
    }
}

fn resolve_search_providers_for_manifest(
    config: &WebSearchConfig,
    mut has_secret: impl FnMut(&str) -> bool,
) -> Vec<WebSearchProvider> {
    if !config.client.enabled {
        return Vec::new();
    }

    SEARCH_PROVIDER_MANIFEST_ORDER
        .into_iter()
        .filter(|provider| {
            let env_name = search_provider_secret_env(config, *provider);
            has_secret(env_name)
        })
        .collect()
}

fn has_configured_secret(env_name: &str, secrets: &SecretsManager) -> bool {
    if std::env::var(env_name)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return true;
    }

    let Ok(name) = SecretName::new(env_name) else {
        return false;
    };
    match secrets.get(&SecretScope::Global, &name) {
        Ok(Some(value)) => !value.trim().is_empty(),
        Ok(None) => false,
        Err(err) => {
            tracing::warn!(
                target: "codex_core::web_tools",
                secret_name = env_name,
                error = %err,
                "failed to read web tool manifest secret"
            );
            false
        }
    }
}

fn search_provider_secret_env(config: &WebSearchConfig, provider: WebSearchProvider) -> &str {
    match provider {
        WebSearchProvider::Brave => config.client.brave_api_key_env.as_str(),
        WebSearchProvider::Github => config.client.github_token_env.as_str(),
        WebSearchProvider::Exa => config.client.exa_api_key_env.as_str(),
        WebSearchProvider::Tavily => config.client.tavily_api_key_env.as_str(),
        WebSearchProvider::StackExchange => config.client.stack_exchange_key_env.as_str(),
        WebSearchProvider::Jina => config.client.jina_api_key_env.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::config_types::WebSearchConfig;

    #[test]
    fn manifest_exposes_only_providers_with_secrets() {
        let mut config = WebSearchConfig::default();
        config.client.brave_api_key_env = "TEST_BRAVE".to_string();
        config.client.exa_api_key_env = "TEST_EXA".to_string();
        config.client.github_token_env = "TEST_GITHUB".to_string();

        let providers = resolve_search_providers_for_manifest(&config, |name| {
            matches!(name, "TEST_EXA" | "TEST_GITHUB")
        });

        assert_eq!(
            providers,
            vec![WebSearchProvider::Github, WebSearchProvider::Exa]
        );
    }

    #[test]
    fn manifest_is_empty_when_client_search_is_disabled() {
        let mut config = WebSearchConfig::default();
        config.client.enabled = false;

        let providers = resolve_search_providers_for_manifest(&config, |_| true);

        assert!(providers.is_empty());
    }
}
