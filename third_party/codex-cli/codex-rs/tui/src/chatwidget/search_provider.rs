use super::search_provider_health::spawn_search_provider_health_check;
use super::search_provider_support::*;
use super::*;
use crate::legacy_core::config::edit::ConfigEdit;
use crate::legacy_core::config::edit::ConfigEditsBuilder;
use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::config_types::WebSearchProvider;
use codex_protocol::config_types::WebSearchStrategy;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use toml_edit::value;

const SEARCH_PROVIDER_USAGE: &str = "Usage: /search-provider [status|configure|set PROVIDER|fallback PROVIDER|fallback off|key PROVIDER [ENV_VAR]|test|on|off]";
const SEARCH_PROVIDER_SETUP: [WebSearchProvider; 6] = [
    WebSearchProvider::Brave,
    WebSearchProvider::Jina,
    WebSearchProvider::Github,
    WebSearchProvider::Exa,
    WebSearchProvider::Tavily,
    WebSearchProvider::StackExchange,
];

impl ChatWidget {
    pub(crate) fn open_search_provider_popup(&mut self) {
        let items = SEARCH_PROVIDER_SETUP
            .into_iter()
            .map(|provider| {
                let provider_name = web_search_provider_display_name(provider).to_string();
                let env_name = web_search_provider_secret_env(provider)
                    .unwrap_or("no api key")
                    .to_string();
                SelectionItem {
                    name: provider_name,
                    description: Some(format!("Store credential as {env_name}")),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::OpenSearchProviderKeyPrompt { provider });
                    })],
                    dismiss_on_select: true,
                    search_value: Some(format!(
                        "{} {}",
                        web_search_provider_name(provider),
                        web_search_provider_display_name(provider)
                    )),
                    ..Default::default()
                }
            })
            .collect();
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Search provider".to_string()),
            subtitle: Some("Choose a provider, then enter its API key or token.".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            is_searchable: true,
            search_placeholder: Some("Type to search providers".to_string()),
            ..Default::default()
        });
        self.request_redraw();
    }

    pub(crate) fn show_search_provider_key_prompt(&mut self, provider: WebSearchProvider) {
        let Some(env_name) = web_search_provider_secret_env(provider) else {
            self.set_search_provider(provider);
            return;
        };
        let provider_name = web_search_provider_display_name(provider).to_string();
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new_secret(
            format!("{provider_name} API key"),
            "Paste key and press Enter".to_string(),
            Some(format!("Stored locally as {env_name}")),
            Box::new(move |secret: String| {
                tx.send(AppEvent::PersistSearchProviderSecret { provider, secret });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }

    pub(crate) fn persist_search_provider_secret(
        &mut self,
        provider: WebSearchProvider,
        secret: String,
    ) {
        let Some(env_name) = web_search_provider_secret_env(provider) else {
            self.set_search_provider(provider);
            return;
        };
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            self.add_error_message("Search provider key cannot be empty.".to_string());
            return;
        }
        let manager = SecretsManager::new(
            self.config.codex_home.to_path_buf(),
            SecretsBackendKind::Local,
        );
        let name = match SecretName::new(env_name) {
            Ok(name) => name,
            Err(err) => {
                self.add_error_message(format!("Invalid secret name {env_name}: {err}"));
                return;
            }
        };
        if let Err(err) = manager.set(&SecretScope::Global, &name, trimmed) {
            self.add_error_message(format!("Failed to save search provider key: {err}"));
            return;
        }

        let mut web_config = self.web_config_for_update();
        let configured_providers = configured_providers_with(&web_config, provider);
        let mut edits = provider_key_env_edits(provider, env_name);
        edits.extend([
            ConfigEdit::SetPath {
                segments: segments(&["web_search"]),
                value: value("live"),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "enabled"]),
                value: value(true),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "provider"]),
                value: value(web_search_provider_name(provider)),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "strategy"]),
                value: value("auto"),
            },
            configured_provider_edit(&configured_providers),
        ]);
        if !self.apply_search_provider_edits(edits) {
            return;
        }

        web_config.client.enabled = true;
        web_config.client.provider = provider;
        web_config.client.configured_providers = configured_providers;
        web_config.client.strategy = WebSearchStrategy::Auto;
        set_web_config_key_env(&mut web_config, provider, env_name);
        self.config.web_search_config = Some(web_config);
        if let Err(err) = self.config.web_search_mode.set(WebSearchMode::Live) {
            self.add_error_message(err.to_string());
            return;
        }
        let provider_name = web_search_provider_name(provider);
        self.add_info_message(
            format!("search_provider={provider_name} key=saved health_check=running"),
            /*hint*/ None,
        );
        spawn_search_provider_health_check(
            self.app_event_tx.clone(),
            provider,
            trimmed.to_string(),
            self.web_config_for_update().client.stack_exchange_site,
        );
    }

    pub(crate) fn dispatch_search_provider_command(&mut self, trimmed: &str) {
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            [] | ["configure"] => self.open_search_provider_popup(),
            ["status"] => self.add_search_provider_status(),
            ["set", provider] => match parse_web_search_provider(provider) {
                Some(provider) => self.set_search_provider(provider),
                None => self.add_error_message(SEARCH_PROVIDER_USAGE.to_string()),
            },
            ["fallback", "off"] => self.set_search_fallback(None),
            ["fallback", provider] => match parse_web_search_provider(provider) {
                Some(provider) => self.set_search_fallback(Some(provider)),
                None => self.add_error_message(SEARCH_PROVIDER_USAGE.to_string()),
            },
            ["key", provider] => match parse_web_search_provider(provider) {
                Some(provider) => self.add_provider_key_status(provider),
                None => self.add_error_message(SEARCH_PROVIDER_USAGE.to_string()),
            },
            ["key", provider, env_var] => match parse_web_search_provider(provider) {
                Some(provider) => self.set_provider_key_env(provider, env_var),
                None => self.add_error_message(SEARCH_PROVIDER_USAGE.to_string()),
            },
            ["test"] => self.add_search_provider_test_result(),
            ["on"] => self.set_search_enabled(true),
            ["off"] => self.set_search_enabled(false),
            _ => self.add_error_message(SEARCH_PROVIDER_USAGE.to_string()),
        }
    }

    fn add_search_provider_status(&mut self) {
        let web_config = self.config.web_search_config.clone().unwrap_or_default();
        let mode = web_search_mode_name(self.config.web_search_mode.value());
        let provider = web_search_provider_name(web_config.client.provider);
        let fallback = web_config
            .client
            .fallback_provider
            .map(web_search_provider_name)
            .unwrap_or("off");
        let configured = provider_names(&web_config.client.configured_providers);
        let fetch_provider = web_fetch_provider_name(web_config.fetch.provider);
        let key_status = SEARCH_PROVIDER_SETUP
            .into_iter()
            .map(|provider| {
                let status = self.search_provider_key_status(provider);
                format!("{}={status}", web_search_provider_name(provider))
            })
            .collect::<Vec<_>>()
            .join(" ");
        self.add_info_message(
            format!(
                "web_search={} mode={} provider={} strategy={} fallback={} configured=[{}] max_results={} keys=[{}] web_fetch={} fetch_provider={} fetch_max_chars={}",
                on_off(web_config.client.enabled),
                mode,
                provider,
                web_search_strategy_name(web_config.client.strategy),
                fallback,
                configured,
                web_config.client.max_results,
                key_status,
                on_off(web_config.fetch.enabled),
                fetch_provider,
                web_config.fetch.max_chars
            ),
            Some(SEARCH_PROVIDER_USAGE.to_string()),
        );
    }

    fn set_search_provider(&mut self, provider: WebSearchProvider) {
        let provider_name = web_search_provider_name(provider);
        let configured_providers = self
            .web_config_for_update()
            .client
            .configured_providers
            .clone();
        let edits = vec![
            ConfigEdit::SetPath {
                segments: segments(&["web_search"]),
                value: value("live"),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "enabled"]),
                value: value(true),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "provider"]),
                value: value(provider_name),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "strategy"]),
                value: value("auto"),
            },
            configured_provider_edit(&configured_providers),
        ];
        if self.apply_search_provider_edits(edits) {
            let mut web_config = self.web_config_for_update();
            web_config.client.enabled = true;
            web_config.client.provider = provider;
            web_config.client.configured_providers = configured_providers;
            web_config.client.strategy = WebSearchStrategy::Auto;
            self.config.web_search_config = Some(web_config);
            if let Err(err) = self.config.web_search_mode.set(WebSearchMode::Live) {
                self.add_error_message(err.to_string());
                return;
            }
            self.add_info_message(format!("web_search provider={provider_name}"), None);
        }
    }

    fn set_search_fallback(&mut self, fallback: Option<WebSearchProvider>) {
        let configured_providers = self
            .web_config_for_update()
            .client
            .configured_providers
            .clone();
        let mut edits = vec![match fallback {
            Some(provider) => ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "fallback_provider"]),
                value: value(web_search_provider_name(provider)),
            },
            None => ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "fallback_provider"]),
                value: value("off"),
            },
        }];
        edits.push(configured_provider_edit(&configured_providers));
        if self.apply_search_provider_edits(edits) {
            let mut web_config = self.web_config_for_update();
            web_config.client.fallback_provider = fallback;
            web_config.client.configured_providers = configured_providers;
            self.config.web_search_config = Some(web_config);
            let fallback_name = fallback.map(web_search_provider_name).unwrap_or("off");
            self.add_info_message(format!("web_search fallback={fallback_name}"), None);
        }
    }

    fn set_search_enabled(&mut self, enabled: bool) {
        let edits = [
            ConfigEdit::SetPath {
                segments: segments(&["web_search"]),
                value: value(if enabled { "live" } else { "disabled" }),
            },
            ConfigEdit::SetPath {
                segments: segments(&["tools", "web_search", "enabled"]),
                value: value(enabled),
            },
        ];
        if self.apply_search_provider_edits(edits) {
            let mut web_config = self.web_config_for_update();
            web_config.client.enabled = enabled;
            self.config.web_search_config = Some(web_config);
            let mode = if enabled {
                WebSearchMode::Live
            } else {
                WebSearchMode::Disabled
            };
            if let Err(err) = self.config.web_search_mode.set(mode) {
                self.add_error_message(err.to_string());
                return;
            }
            self.add_info_message(format!("web_search={}", on_off(enabled)), None);
        }
    }

    fn add_provider_key_status(&mut self, provider: WebSearchProvider) {
        let web_config = self.config.web_search_config.clone().unwrap_or_default();
        let Some(env_name) = web_config_key_env(&web_config, provider) else {
            self.add_info_message("jina_key=not_required".to_string(), /*hint*/ None);
            return;
        };
        let key_status = self.search_provider_key_status(provider);
        self.add_info_message(
            format!(
                "{}_api_key_env={} key={key_status}",
                web_search_provider_name(provider),
                env_name
            ),
            Some(format!(
                "Use /search-provider key {} ENV_VAR to change the environment variable name.",
                web_search_provider_name(provider)
            )),
        );
    }

    fn set_provider_key_env(&mut self, provider: WebSearchProvider, env_var: &str) {
        if !is_valid_env_var_name(env_var) {
            self.add_error_message("Invalid environment variable name.".to_string());
            return;
        }
        let edits = provider_key_env_edits(provider, env_var);
        if edits.is_empty() {
            self.add_info_message("jina_key=not_required".to_string(), /*hint*/ None);
            return;
        }
        if self.apply_search_provider_edits(edits) {
            let mut web_config = self.web_config_for_update();
            set_web_config_key_env(&mut web_config, provider, env_var);
            self.config.web_search_config = Some(web_config);
            self.add_info_message(
                format!(
                    "{}_api_key_env={env_var}",
                    web_search_provider_name(provider)
                ),
                None,
            );
        }
    }

    fn add_search_provider_test_result(&mut self) {
        let web_config = self.config.web_search_config.clone().unwrap_or_default();
        let provider = web_config.client.provider;
        let provider_name = web_search_provider_name(provider);
        let fallback = web_config
            .client
            .fallback_provider
            .map(web_search_provider_name)
            .unwrap_or("off");
        if !web_config.client.enabled {
            self.add_info_message(
                format!(
                    "search_provider_test=disabled provider={provider_name} fallback={fallback}"
                ),
                /*hint*/ None,
            );
            return;
        }
        let Some(secret) = self.search_provider_secret_value(provider) else {
            self.add_info_message(
                format!(
                    "search_provider_test=needs_key provider={provider_name} fallback={fallback}"
                ),
                /*hint*/ None,
            );
            return;
        };
        self.add_info_message(
            format!("search_provider_test=running provider={provider_name} fallback={fallback}"),
            /*hint*/ None,
        );
        spawn_search_provider_health_check(
            self.app_event_tx.clone(),
            provider,
            secret,
            web_config.client.stack_exchange_site,
        );
    }

    fn search_provider_key_status(&self, provider: WebSearchProvider) -> &'static str {
        let web_config = self.config.web_search_config.clone().unwrap_or_default();
        let Some(env_name) = web_config_key_env(&web_config, provider) else {
            return "not_required";
        };
        if std::env::var(env_name).is_ok_and(|value| !value.trim().is_empty()) {
            return "env";
        }
        if web_config.client.configured_providers.contains(&provider) {
            "configured"
        } else {
            "unset"
        }
    }

    fn search_provider_secret_value(&self, provider: WebSearchProvider) -> Option<String> {
        let web_config = self.config.web_search_config.clone().unwrap_or_default();
        let env_name = web_config_key_env(&web_config, provider)?;
        if let Ok(value) = std::env::var(env_name)
            && !value.trim().is_empty()
        {
            return Some(value.trim().to_string());
        }
        let name = SecretName::new(env_name).ok()?;
        let manager = SecretsManager::new(
            self.config.codex_home.to_path_buf(),
            SecretsBackendKind::Local,
        );
        manager
            .get(&SecretScope::Global, &name)
            .ok()
            .flatten()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string())
    }

    fn web_config_for_update(&self) -> WebSearchConfig {
        self.config.web_search_config.clone().unwrap_or_default()
    }

    fn apply_search_provider_edits<I>(&mut self, edits: I) -> bool
    where
        I: IntoIterator<Item = ConfigEdit>,
    {
        match ConfigEditsBuilder::new(&self.config.codex_home)
            .with_edits(edits)
            .apply_blocking()
        {
            Ok(()) => true,
            Err(err) => {
                self.add_error_message(format!("Failed to update search provider config: {err}"));
                false
            }
        }
    }
}
