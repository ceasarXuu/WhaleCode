use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchProvider;
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
    _codex_home: &Path,
) -> WebToolManifestAvailability {
    let availability = WebToolManifestAvailability {
        search_providers: resolve_search_providers_for_manifest(config, env_secret_present),
    };
    availability
}

fn resolve_search_providers_for_manifest(
    config: &WebSearchConfig,
    mut env_present: impl FnMut(&str) -> bool,
) -> Vec<WebSearchProvider> {
    if !config.client.enabled {
        return Vec::new();
    }

    let mut providers = Vec::new();
    for provider in &config.client.configured_providers {
        push_unique_provider(&mut providers, *provider);
    }

    for (provider, env_name) in search_provider_envs(config) {
        if env_present(env_name) {
            push_unique_provider(&mut providers, provider);
        }
    }

    providers
}

fn env_secret_present(env_name: &str) -> bool {
    std::env::var(env_name)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

fn push_unique_provider(providers: &mut Vec<WebSearchProvider>, provider: WebSearchProvider) {
    if !providers.contains(&provider) {
        providers.push(provider);
    }
}

fn search_provider_envs(config: &WebSearchConfig) -> [(WebSearchProvider, &str); 6] {
    SEARCH_PROVIDER_MANIFEST_ORDER
        .map(|provider| (provider, search_provider_secret_env(config, provider)))
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
    fn manifest_exposes_configured_provider_markers_and_env_hints() {
        let mut config = WebSearchConfig::default();
        config.client.provider = WebSearchProvider::Tavily;
        config.client.fallback_provider = None;
        config.client.configured_providers = vec![WebSearchProvider::Tavily];
        config.client.exa_api_key_env = "TEST_EXA".to_string();
        config.client.github_token_env = "TEST_GITHUB".to_string();

        let providers = resolve_search_providers_for_manifest(&config, |name| {
            matches!(name, "TEST_EXA" | "TEST_GITHUB")
        });

        assert_eq!(
            providers,
            vec![
                WebSearchProvider::Tavily,
                WebSearchProvider::Github,
                WebSearchProvider::Exa
            ]
        );
    }

    #[test]
    fn manifest_is_empty_when_client_search_is_disabled() {
        let mut config = WebSearchConfig::default();
        config.client.enabled = false;

        let providers = resolve_search_providers_for_manifest(&config, |_| true);

        assert!(providers.is_empty());
    }

    #[test]
    fn manifest_ignores_selected_provider_without_marker_or_env() {
        let mut config = WebSearchConfig::default();
        config.client.provider = WebSearchProvider::Tavily;
        config.client.fallback_provider = Some(WebSearchProvider::Jina);
        config.client.configured_providers = Vec::new();

        let providers = resolve_search_providers_for_manifest(&config, |_| false);

        assert!(providers.is_empty());
    }
}
