use std::collections::HashMap;
use std::sync::Arc;

use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider::SharedModelProvider;
use codex_model_provider::create_route_model_provider;
use codex_model_provider::create_route_models_manager;
use codex_models_manager::manager::ProviderModelsCatalogEntry;
use codex_models_manager::manager::SharedModelsManager;
use codex_protocol::ProviderAccessMethod;
use codex_protocol::ProviderRoute;

use crate::config::Config;

const OPENAI_PROVIDER_ID: &str = "openai";
const DEEPSEEK_PROVIDER_ID: &str = "deepseek";

#[derive(Clone)]
pub(crate) struct ProviderRuntime {
    pub(crate) provider: SharedModelProvider,
    pub(crate) models_manager: SharedModelsManager,
}

/// Session-safe lookup for the three provider routes supported by v0.0.6.
#[derive(Clone, Default)]
pub(crate) struct ProviderRuntimeRegistry {
    entries: HashMap<ProviderRoute, ProviderRuntime>,
}

impl ProviderRuntimeRegistry {
    pub(crate) fn from_config(config: &Config, auth_manager: Arc<AuthManager>) -> Self {
        let routes = [
            ProviderRoute::new(OPENAI_PROVIDER_ID, ProviderAccessMethod::Chatgpt),
            ProviderRoute::new(OPENAI_PROVIDER_ID, ProviderAccessMethod::ApiKey),
            ProviderRoute::new(DEEPSEEK_PROVIDER_ID, ProviderAccessMethod::ApiKey),
        ];
        let entries = routes
            .into_iter()
            .filter_map(|route| {
                let provider_info = config
                    .model_providers
                    .get(&route.model_provider_id)?
                    .clone();
                let provider = create_route_model_provider(
                    provider_info.clone(),
                    Arc::clone(&auth_manager),
                    route.clone(),
                );
                let models_manager = create_route_models_manager(
                    provider_info,
                    Arc::clone(&auth_manager),
                    config.codex_home.to_path_buf(),
                    config.model_catalog.clone(),
                    route.clone(),
                );
                Some((
                    route,
                    ProviderRuntime {
                        provider,
                        models_manager,
                    },
                ))
            })
            .collect();
        Self { entries }
    }

    pub(crate) fn get(&self, route: &ProviderRoute) -> Option<&ProviderRuntime> {
        self.entries.get(route)
    }

    pub(crate) fn initial_route(
        &self,
        model_provider_id: &str,
        configured_access_method: Option<ProviderAccessMethod>,
        configured_provider: &codex_model_provider_info::ModelProviderInfo,
        model_providers: &HashMap<String, codex_model_provider_info::ModelProviderInfo>,
        auth_manager: &AuthManager,
    ) -> Option<ProviderRoute> {
        // Legacy/custom callers can replace the effective provider without also updating the
        // provider ID. Do not reinterpret such a provider as a built-in runtime route.
        if model_providers.get(model_provider_id) != Some(configured_provider) {
            return None;
        }
        let access_method = match configured_access_method {
            Some(access_method) => access_method,
            None => match model_provider_id {
                OPENAI_PROVIDER_ID
                    if matches!(
                        auth_manager.auth_cached(),
                        Some(CodexAuth::Chatgpt(_) | CodexAuth::ChatgptAuthTokens(_))
                    ) =>
                {
                    ProviderAccessMethod::Chatgpt
                }
                OPENAI_PROVIDER_ID | DEEPSEEK_PROVIDER_ID => ProviderAccessMethod::ApiKey,
                _ => return None,
            },
        };
        let route = ProviderRoute::new(model_provider_id, access_method);
        self.entries.contains_key(&route).then_some(route)
    }

    pub(crate) fn catalog_entries(&self) -> Vec<ProviderModelsCatalogEntry> {
        [
            ProviderRoute::new(OPENAI_PROVIDER_ID, ProviderAccessMethod::Chatgpt),
            ProviderRoute::new(OPENAI_PROVIDER_ID, ProviderAccessMethod::ApiKey),
            ProviderRoute::new(DEEPSEEK_PROVIDER_ID, ProviderAccessMethod::ApiKey),
        ]
        .into_iter()
        .filter_map(|route| {
            let runtime = self.entries.get(&route)?;
            Some(ProviderModelsCatalogEntry {
                route: route.clone(),
                display_name: match (route.model_provider_id.as_str(), route.access_method) {
                    (OPENAI_PROVIDER_ID, ProviderAccessMethod::Chatgpt) => "OpenAI Subscription",
                    (OPENAI_PROVIDER_ID, ProviderAccessMethod::ApiKey) => "OpenAI API",
                    (DEEPSEEK_PROVIDER_ID, ProviderAccessMethod::ApiKey) => "DeepSeek API",
                    _ => "Provider",
                }
                .to_string(),
                manager: Arc::clone(&runtime.models_manager),
            })
        })
        .collect()
    }
}

#[cfg(test)]
#[path = "provider_runtime_tests.rs"]
mod tests;
