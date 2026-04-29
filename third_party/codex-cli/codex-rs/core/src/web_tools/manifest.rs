use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchProvider;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

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
    if !config.client.enabled {
        return WebToolManifestAvailability {
            search_providers: Vec::new(),
        };
    }

    let cache_key = WebToolManifestCacheKey::new(config, codex_home);
    if let Some(cached) = cached_manifest_availability(&cache_key) {
        return cached;
    }

    let secret_names = configured_global_secret_names(codex_home);
    let availability = WebToolManifestAvailability {
        search_providers: resolve_search_providers_for_manifest(
            config,
            &secret_names,
            env_secret_present,
        ),
    };
    cache_manifest_availability(cache_key, availability.clone());
    availability
}

fn resolve_search_providers_for_manifest(
    config: &WebSearchConfig,
    secret_names: &HashSet<SecretName>,
    mut env_present: impl FnMut(&str) -> bool,
) -> Vec<WebSearchProvider> {
    if !config.client.enabled {
        return Vec::new();
    }

    search_provider_envs(config)
        .into_iter()
        .filter_map(|(provider, env_name)| {
            has_configured_secret(env_name, secret_names, &mut env_present).then_some(provider)
        })
        .collect()
}

fn has_configured_secret(
    env_name: &str,
    secret_names: &HashSet<SecretName>,
    env_present: &mut impl FnMut(&str) -> bool,
) -> bool {
    if env_present(env_name) {
        return true;
    }

    let Ok(name) = SecretName::new(env_name) else {
        return false;
    };
    secret_names.contains(&name)
}

fn env_secret_present(env_name: &str) -> bool {
    std::env::var(env_name)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

fn configured_global_secret_names(codex_home: &Path) -> HashSet<SecretName> {
    let secrets = SecretsManager::new(codex_home.to_path_buf(), SecretsBackendKind::Local);
    match secrets.list(Some(&SecretScope::Global)) {
        Ok(entries) => entries.into_iter().map(|entry| entry.name).collect(),
        Err(err) => {
            tracing::warn!(
                target: "codex_core::web_tools",
                error = %err,
                "failed to list web tool manifest secrets"
            );
            HashSet::new()
        }
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

type ManifestAvailabilityCache =
    Mutex<HashMap<WebToolManifestCacheKey, WebToolManifestAvailability>>;

fn manifest_cache() -> &'static ManifestAvailabilityCache {
    static CACHE: OnceLock<ManifestAvailabilityCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_manifest_availability(
    key: &WebToolManifestCacheKey,
) -> Option<WebToolManifestAvailability> {
    let cache = manifest_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.get(key).cloned()
}

fn cache_manifest_availability(
    key: WebToolManifestCacheKey,
    availability: WebToolManifestAvailability,
) {
    let mut cache = manifest_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.insert(key, availability);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WebToolManifestCacheKey {
    codex_home: PathBuf,
    client_enabled: bool,
    provider_secret_envs: [String; 6],
    provider_env_present: [bool; 6],
    secrets_file: Option<SecretsFileFingerprint>,
}

impl WebToolManifestCacheKey {
    fn new(config: &WebSearchConfig, codex_home: &Path) -> Self {
        let provider_envs = search_provider_envs(config);
        Self {
            codex_home: codex_home.to_path_buf(),
            client_enabled: config.client.enabled,
            provider_secret_envs: provider_envs.map(|(_, env_name)| env_name.to_string()),
            provider_env_present: provider_envs.map(|(_, env_name)| env_secret_present(env_name)),
            secrets_file: secrets_file_fingerprint(codex_home),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SecretsFileFingerprint {
    len: u64,
    modified_ms: Option<u128>,
}

fn secrets_file_fingerprint(codex_home: &Path) -> Option<SecretsFileFingerprint> {
    let metadata = fs::metadata(codex_home.join("secrets").join("local.age")).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis());
    Some(SecretsFileFingerprint {
        len: metadata.len(),
        modified_ms,
    })
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

        let secret_names = secret_names(&["TEST_EXA", "TEST_GITHUB"]);
        let providers = resolve_search_providers_for_manifest(&config, &secret_names, |_| false);

        assert_eq!(
            providers,
            vec![WebSearchProvider::Github, WebSearchProvider::Exa]
        );
    }

    #[test]
    fn manifest_is_empty_when_client_search_is_disabled() {
        let mut config = WebSearchConfig::default();
        config.client.enabled = false;

        let providers = resolve_search_providers_for_manifest(&config, &HashSet::new(), |_| true);

        assert!(providers.is_empty());
    }

    #[test]
    fn manifest_uses_env_presence_without_secret_store_name() {
        let mut config = WebSearchConfig::default();
        config.client.tavily_api_key_env = "TEST_TAVILY".to_string();

        let providers = resolve_search_providers_for_manifest(&config, &HashSet::new(), |name| {
            name == "TEST_TAVILY"
        });

        assert_eq!(providers, vec![WebSearchProvider::Tavily]);
    }

    fn secret_names(names: &[&str]) -> HashSet<SecretName> {
        names
            .iter()
            .map(|name| SecretName::new(name).expect("test secret names are valid"))
            .collect()
    }
}
