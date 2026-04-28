use crate::legacy_core::config::edit::ConfigEdit;
use codex_protocol::config_types::WebFetchProvider;
use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::config_types::WebSearchProvider;
use codex_protocol::config_types::WebSearchStrategy;
use toml_edit::value;

pub(super) fn segments(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(super) fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

pub(super) fn web_search_provider_name(provider: WebSearchProvider) -> &'static str {
    match provider {
        WebSearchProvider::Brave => "brave",
        WebSearchProvider::Jina => "jina",
        WebSearchProvider::Github => "github",
        WebSearchProvider::Exa => "exa",
        WebSearchProvider::Tavily => "tavily",
        WebSearchProvider::StackExchange => "stack_exchange",
    }
}

pub(super) fn web_search_provider_display_name(provider: WebSearchProvider) -> &'static str {
    match provider {
        WebSearchProvider::Brave => "Brave Search",
        WebSearchProvider::Jina => "Jina",
        WebSearchProvider::Github => "GitHub Search",
        WebSearchProvider::Exa => "Exa",
        WebSearchProvider::Tavily => "Tavily",
        WebSearchProvider::StackExchange => "Stack Exchange",
    }
}

pub(super) fn parse_web_search_provider(value: &str) -> Option<WebSearchProvider> {
    match value.to_ascii_lowercase().replace('-', "_").as_str() {
        "brave" | "brave_search" => Some(WebSearchProvider::Brave),
        "jina" => Some(WebSearchProvider::Jina),
        "github" | "github_search" => Some(WebSearchProvider::Github),
        "exa" => Some(WebSearchProvider::Exa),
        "tavily" => Some(WebSearchProvider::Tavily),
        "stack_exchange" | "stackexchange" | "stackoverflow" => {
            Some(WebSearchProvider::StackExchange)
        }
        _ => None,
    }
}

pub(super) fn web_search_provider_secret_env(provider: WebSearchProvider) -> Option<&'static str> {
    match provider {
        WebSearchProvider::Brave => Some("BRAVE_SEARCH_API_KEY"),
        WebSearchProvider::Github => Some("GITHUB_TOKEN"),
        WebSearchProvider::Exa => Some("EXA_API_KEY"),
        WebSearchProvider::Tavily => Some("TAVILY_API_KEY"),
        WebSearchProvider::StackExchange => Some("STACK_EXCHANGE_KEY"),
        WebSearchProvider::Jina => None,
    }
}

pub(super) fn web_config_key_env(
    config: &WebSearchConfig,
    provider: WebSearchProvider,
) -> Option<&str> {
    match provider {
        WebSearchProvider::Brave => Some(config.client.brave_api_key_env.as_str()),
        WebSearchProvider::Github => Some(config.client.github_token_env.as_str()),
        WebSearchProvider::Exa => Some(config.client.exa_api_key_env.as_str()),
        WebSearchProvider::Tavily => Some(config.client.tavily_api_key_env.as_str()),
        WebSearchProvider::StackExchange => Some(config.client.stack_exchange_key_env.as_str()),
        WebSearchProvider::Jina => None,
    }
}

pub(super) fn set_web_config_key_env(
    config: &mut WebSearchConfig,
    provider: WebSearchProvider,
    env_name: &str,
) {
    match provider {
        WebSearchProvider::Brave => config.client.brave_api_key_env = env_name.to_string(),
        WebSearchProvider::Github => config.client.github_token_env = env_name.to_string(),
        WebSearchProvider::Exa => config.client.exa_api_key_env = env_name.to_string(),
        WebSearchProvider::Tavily => config.client.tavily_api_key_env = env_name.to_string(),
        WebSearchProvider::StackExchange => {
            config.client.stack_exchange_key_env = env_name.to_string()
        }
        WebSearchProvider::Jina => {}
    }
}

pub(super) fn provider_key_env_edits(
    provider: WebSearchProvider,
    env_name: &str,
) -> Vec<ConfigEdit> {
    let key = match provider {
        WebSearchProvider::Brave => "brave_api_key_env",
        WebSearchProvider::Github => "github_token_env",
        WebSearchProvider::Exa => "exa_api_key_env",
        WebSearchProvider::Tavily => "tavily_api_key_env",
        WebSearchProvider::StackExchange => "stack_exchange_key_env",
        WebSearchProvider::Jina => return Vec::new(),
    };
    vec![ConfigEdit::SetPath {
        segments: segments(&["tools", "web_search", key]),
        value: value(env_name),
    }]
}

pub(super) fn web_search_strategy_name(strategy: WebSearchStrategy) -> &'static str {
    match strategy {
        WebSearchStrategy::Auto => "auto",
        WebSearchStrategy::Single => "single",
        WebSearchStrategy::Fanout => "fanout",
    }
}

pub(super) fn web_fetch_provider_name(provider: WebFetchProvider) -> &'static str {
    match provider {
        WebFetchProvider::Jina => "jina",
        WebFetchProvider::Direct => "direct",
    }
}

pub(super) fn web_search_mode_name(mode: WebSearchMode) -> &'static str {
    match mode {
        WebSearchMode::Disabled => "disabled",
        WebSearchMode::Cached => "cached",
        WebSearchMode::Live => "live",
    }
}
