use crate::web_tools::WebToolError;
use crate::web_tools::safety::normalize_fetch_url;
use async_trait::async_trait;
use codex_protocol::config_types::WebFetchConfig;
use codex_protocol::config_types::WebFetchProvider;
use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchProvider;
use codex_protocol::config_types::WebSearchStrategy;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use futures::future::join_all;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use tracing::info;
use url::Url;

mod fetch_adapters;
mod result_parsers;
#[cfg(test)]
mod router_tests;
mod search_adapters;
mod support;

use fetch_adapters::DirectFetchProvider;
use fetch_adapters::JinaFetchProvider;
use result_parsers::merge_ranked_results;
use search_adapters::BraveSearchProvider;
use search_adapters::ExaSearchProvider;
use search_adapters::GithubSearchProvider;
use search_adapters::JinaSearchProvider;
use search_adapters::StackExchangeSearchProvider;
use search_adapters::TavilySearchProvider;
use support::fallback_reason;
use support::shape_query;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WebSearchArgs {
    pub(crate) query: String,
    pub(crate) max_results: Option<usize>,
    pub(crate) freshness: Option<SearchFreshness>,
    pub(crate) domains: Option<Vec<String>>,
    pub(crate) exclude_domains: Option<Vec<String>>,
    pub(crate) source_hint: Option<SourceHint>,
    pub(crate) provider_policy: Option<ProviderPolicy>,
    pub(crate) preferred_providers: Option<Vec<WebSearchProvider>>,
    pub(crate) github: Option<GithubSearchArgs>,
    pub(crate) stack_exchange: Option<StackExchangeSearchArgs>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderPolicy {
    Auto,
    Single,
    Fanout,
}

impl From<WebSearchStrategy> for ProviderPolicy {
    fn from(value: WebSearchStrategy) -> Self {
        match value {
            WebSearchStrategy::Auto => Self::Auto,
            WebSearchStrategy::Single => Self::Single,
            WebSearchStrategy::Fanout => Self::Fanout,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SearchFreshness {
    Any,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceHint {
    General,
    Technical,
    Github,
    Docs,
    Community,
    Research,
    News,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct GithubSearchArgs {
    pub(crate) search_type: Option<GithubSearchType>,
    pub(crate) repo: Option<String>,
    pub(crate) org: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) filename: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GithubSearchType {
    Repositories,
    Code,
    Issues,
    Commits,
    Users,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct StackExchangeSearchArgs {
    pub(crate) site: Option<String>,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) accepted: Option<bool>,
    pub(crate) sort: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WebFetchArgs {
    pub(crate) url: String,
    pub(crate) format: Option<FetchFormat>,
    pub(crate) max_chars: Option<usize>,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FetchFormat {
    Markdown,
    Text,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct WebSearchResult {
    pub(crate) rank: usize,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) domain: String,
    pub(crate) snippet: String,
    pub(crate) published_at: Option<String>,
    pub(crate) source_type: String,
    pub(crate) provider: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebSearchOutput {
    pub(crate) provider: String,
    pub(crate) query: String,
    pub(crate) fallback_used: bool,
    pub(crate) fallback_reason: Option<String>,
    pub(crate) results: Vec<WebSearchResult>,
    pub(crate) latency_ms: u128,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebFetchOutput {
    pub(crate) provider: String,
    pub(crate) url: String,
    pub(crate) final_url: String,
    pub(crate) title: Option<String>,
    pub(crate) content: String,
    pub(crate) content_chars: usize,
    pub(crate) truncated: bool,
    pub(crate) latency_ms: u128,
}

#[derive(Clone)]
pub(crate) struct WebProviderRegistry {
    client: Client,
    search_config: WebSearchConfig,
    codex_home: PathBuf,
}

impl WebProviderRegistry {
    pub(crate) fn new(
        search_config: WebSearchConfig,
        codex_home: PathBuf,
    ) -> Result<Self, WebToolError> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("whale-code-web-tools/1.0")
            .build()
            .map_err(|err| WebToolError::Parse {
                provider: "web",
                message: err.to_string(),
            })?;

        Ok(Self {
            client,
            search_config,
            codex_home,
        })
    }

    pub(crate) async fn search(
        &self,
        args: WebSearchArgs,
    ) -> Result<WebSearchOutput, WebToolError> {
        let request = SearchRequest::from_args(args, &self.search_config, self.codex_home.clone())?;
        let started = Instant::now();
        let providers = self.route_providers(&request);
        let (mut providers, skipped) = self.available_search_providers(providers, &request);
        let fanout = matches!(request.provider_policy, ProviderPolicy::Fanout);
        if fanout {
            providers.truncate(self.search_config.client.max_providers_per_query.max(1));
        }
        self.log_search_route(&request, &providers, &skipped);
        if providers.is_empty() {
            return Err(no_available_provider_error(&skipped));
        }
        if fanout {
            return self.search_fanout(providers, &request, started).await;
        }
        let mut fallback_reason_text = None;
        let mut first_error = None;

        for (index, provider) in providers.into_iter().enumerate() {
            match self.search_with_logged_provider(provider, &request).await {
                Ok(mut output) => {
                    output.latency_ms = started.elapsed().as_millis();
                    output.fallback_used = index > 0;
                    output.fallback_reason = fallback_reason_text;
                    return Ok(output);
                }
                Err(err) if err.is_fallback_candidate() => {
                    fallback_reason_text.get_or_insert_with(|| fallback_reason(&err));
                    first_error.get_or_insert(err);
                }
                Err(err) => return Err(err),
            }
        }

        Err(first_error.unwrap_or_else(|| {
            WebToolError::InvalidArguments("no search provider is configured".to_string())
        }))
    }

    async fn search_fanout(
        &self,
        providers: Vec<WebSearchProvider>,
        request: &SearchRequest,
        started: Instant,
    ) -> Result<WebSearchOutput, WebToolError> {
        let results = join_all(providers.into_iter().map(|provider| async move {
            self.search_with_logged_provider(provider, request).await
        }))
        .await;
        let mut successes = Vec::new();
        let mut first_error = None;
        for result in results {
            match result {
                Ok(output) => successes.push(output),
                Err(err) => {
                    first_error.get_or_insert(err);
                }
            }
        }

        if successes.is_empty() {
            return Err(first_error.unwrap_or_else(|| {
                WebToolError::InvalidArguments("no search provider is configured".to_string())
            }));
        }

        let providers = successes
            .iter()
            .map(|output| output.provider.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Ok(WebSearchOutput {
            provider: providers,
            query: request.query.clone(),
            fallback_used: false,
            fallback_reason: None,
            results: merge_ranked_results(successes, request.max_results),
            latency_ms: started.elapsed().as_millis(),
        })
    }

    pub(crate) async fn fetch(&self, args: WebFetchArgs) -> Result<WebFetchOutput, WebToolError> {
        let request = FetchRequest::from_args(args, &self.search_config.fetch)?;
        let started = Instant::now();
        let provider = self.search_config.fetch.provider;
        info!(
            target: "codex_core::web_tools",
            tool = "web_fetch",
            provider = web_fetch_provider_name(provider),
            "web fetch provider started"
        );
        let result = self.fetch_with_provider(provider, &request).await;
        let latency_ms = started.elapsed().as_millis();
        match result {
            Ok(mut output) => {
                output.latency_ms = latency_ms;
                info!(
                    target: "codex_core::web_tools",
                    tool = "web_fetch",
                    provider = web_fetch_provider_name(provider),
                    content_chars = output.content_chars,
                    truncated = output.truncated,
                    latency_ms,
                    "web fetch provider succeeded"
                );
                Ok(output)
            }
            Err(err) => {
                info!(
                    target: "codex_core::web_tools",
                    tool = "web_fetch",
                    provider = web_fetch_provider_name(provider),
                    latency_ms,
                    error = %safe_error_summary(&err),
                    "web fetch provider failed"
                );
                Err(err)
            }
        }
    }

    fn route_providers(&self, request: &SearchRequest) -> Vec<WebSearchProvider> {
        let mut candidates = Vec::new();
        if let Some(providers) = request.preferred_providers.as_ref() {
            candidates.extend(providers.iter().copied());
        } else {
            let routed = routed_candidates(request);
            if self.search_config.client.provider == WebSearchProvider::default() {
                candidates.extend(routed);
                candidates.push(self.search_config.client.provider);
            } else {
                candidates.push(self.search_config.client.provider);
                candidates.extend(routed);
            }
        }

        if let Some(provider) = self.search_config.client.fallback_provider {
            candidates.push(provider);
        }
        candidates.push(WebSearchProvider::Jina);

        let mut candidates = dedupe_providers(candidates);
        if matches!(request.provider_policy, ProviderPolicy::Single) {
            candidates.truncate(1);
        }
        candidates
    }

    fn available_search_providers(
        &self,
        providers: Vec<WebSearchProvider>,
        request: &SearchRequest,
    ) -> (Vec<WebSearchProvider>, Vec<SkippedProvider>) {
        let mut available = Vec::new();
        let mut skipped = Vec::new();
        for provider in providers {
            match missing_search_secret(provider, request) {
                Ok(None) => available.push(provider),
                Ok(Some(reason)) => skipped.push(SkippedProvider { provider, reason }),
                Err(err) => skipped.push(SkippedProvider {
                    provider,
                    reason: format!("secret lookup failed: {}", safe_error_summary(&err)),
                }),
            }
        }
        (available, skipped)
    }

    fn log_search_route(
        &self,
        request: &SearchRequest,
        providers: &[WebSearchProvider],
        skipped: &[SkippedProvider],
    ) {
        info!(
            target: "codex_core::web_tools",
            tool = "web_search",
            policy = provider_policy_name(request.provider_policy),
            source_hint = request.source_hint.map(source_hint_name).unwrap_or("none"),
            configured_provider = web_search_provider_name(self.search_config.client.provider),
            providers = %format_provider_list(providers),
            skipped = %format_skipped_providers(skipped),
            "web search routed"
        );
    }

    async fn search_with_logged_provider(
        &self,
        provider: WebSearchProvider,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let started = Instant::now();
        info!(
            target: "codex_core::web_tools",
            tool = "web_search",
            provider = web_search_provider_name(provider),
            "web search provider started"
        );
        let result = self.search_with_provider(provider, request).await;
        let latency_ms = started.elapsed().as_millis();
        match &result {
            Ok(output) => {
                info!(
                    target: "codex_core::web_tools",
                    tool = "web_search",
                    provider = web_search_provider_name(provider),
                    results_count = output.results.len(),
                    latency_ms,
                    "web search provider succeeded"
                );
            }
            Err(err) => {
                info!(
                    target: "codex_core::web_tools",
                    tool = "web_search",
                    provider = web_search_provider_name(provider),
                    fallback_candidate = err.is_fallback_candidate(),
                    latency_ms,
                    error = %safe_error_summary(err),
                    "web search provider failed"
                );
            }
        }
        result
    }

    async fn search_with_provider(
        &self,
        provider: WebSearchProvider,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        match provider {
            WebSearchProvider::Brave => BraveSearchProvider.search(&self.client, request).await,
            WebSearchProvider::Jina => JinaSearchProvider.search(&self.client, request).await,
            WebSearchProvider::Github => GithubSearchProvider.search(&self.client, request).await,
            WebSearchProvider::Exa => ExaSearchProvider.search(&self.client, request).await,
            WebSearchProvider::Tavily => TavilySearchProvider.search(&self.client, request).await,
            WebSearchProvider::StackExchange => {
                StackExchangeSearchProvider
                    .search(&self.client, request)
                    .await
            }
        }
    }

    async fn fetch_with_provider(
        &self,
        provider: WebFetchProvider,
        request: &FetchRequest,
    ) -> Result<WebFetchOutput, WebToolError> {
        match provider {
            WebFetchProvider::Jina => JinaFetchProvider.fetch(&self.client, request).await,
            WebFetchProvider::Direct => DirectFetchProvider.fetch(&self.client, request).await,
        }
    }
}

pub(super) struct SearchRequest {
    pub(super) query: String,
    pub(super) shaped_query: String,
    pub(super) max_results: usize,
    pub(super) freshness: Option<SearchFreshness>,
    pub(super) source_hint: Option<SourceHint>,
    pub(super) github: GithubSearchArgs,
    pub(super) stack_exchange: StackExchangeSearchArgs,
    pub(super) provider_policy: ProviderPolicy,
    pub(super) preferred_providers: Option<Vec<WebSearchProvider>>,
    pub(super) brave_api_key_env: String,
    pub(super) exa_api_key_env: String,
    pub(super) tavily_api_key_env: String,
    pub(super) jina_api_key_env: String,
    pub(super) github_token_env: String,
    pub(super) stack_exchange_key_env: String,
    pub(super) stack_exchange_site: String,
    pub(super) timeout: Duration,
    codex_home: PathBuf,
}

impl SearchRequest {
    fn from_args(
        args: WebSearchArgs,
        config: &WebSearchConfig,
        codex_home: PathBuf,
    ) -> Result<Self, WebToolError> {
        let query = args.query.trim();
        if query.is_empty() {
            return Err(WebToolError::InvalidArguments("query is empty".to_string()));
        }
        let max_results = args
            .max_results
            .unwrap_or(config.client.max_results)
            .clamp(1, config.client.max_results.max(1));
        let provider_policy = args
            .provider_policy
            .unwrap_or_else(|| config.client.strategy.into());

        Ok(Self {
            query: query.to_string(),
            shaped_query: shape_query(query, &args),
            max_results,
            freshness: args.freshness,
            source_hint: args.source_hint,
            github: args.github.unwrap_or_default(),
            stack_exchange: args.stack_exchange.unwrap_or_default(),
            provider_policy,
            preferred_providers: args.preferred_providers,
            brave_api_key_env: config.client.brave_api_key_env.clone(),
            exa_api_key_env: config.client.exa_api_key_env.clone(),
            tavily_api_key_env: config.client.tavily_api_key_env.clone(),
            jina_api_key_env: config.client.jina_api_key_env.clone(),
            github_token_env: config.client.github_token_env.clone(),
            stack_exchange_key_env: config.client.stack_exchange_key_env.clone(),
            stack_exchange_site: config.client.stack_exchange_site.clone(),
            timeout: Duration::from_millis(config.client.timeout_ms),
            codex_home,
        })
    }

    pub(super) fn required_secret(
        &self,
        provider: &'static str,
        env_var: &str,
    ) -> Result<String, WebToolError> {
        self.optional_secret(env_var)?
            .ok_or_else(|| WebToolError::MissingApiKey {
                provider,
                env_var: env_var.to_string(),
            })
    }

    pub(super) fn optional_secret(&self, env_var: &str) -> Result<Option<String>, WebToolError> {
        if let Ok(value) = std::env::var(env_var)
            && !value.trim().is_empty()
        {
            return Ok(Some(value));
        }
        let name = SecretName::new(env_var).map_err(|err| WebToolError::SecretStore {
            message: err.to_string(),
        })?;
        let manager = SecretsManager::new(self.codex_home.clone(), SecretsBackendKind::Local);
        manager
            .get(&SecretScope::Global, &name)
            .map_err(|err| WebToolError::SecretStore {
                message: err.to_string(),
            })
            .map(|value| value.filter(|secret| !secret.trim().is_empty()))
    }
}

pub(super) struct FetchRequest {
    pub(super) url: Url,
    pub(super) format: FetchFormat,
    pub(super) max_chars: usize,
    pub(super) timeout: Duration,
}

impl FetchRequest {
    fn from_args(args: WebFetchArgs, config: &WebFetchConfig) -> Result<Self, WebToolError> {
        if args.reason.trim().is_empty() {
            return Err(WebToolError::InvalidArguments(
                "reason is empty".to_string(),
            ));
        }
        let max_chars = args
            .max_chars
            .unwrap_or(config.max_chars)
            .clamp(1_000, config.max_chars.max(1_000));
        Ok(Self {
            url: normalize_fetch_url(&args.url)?,
            format: args.format.unwrap_or(FetchFormat::Markdown),
            max_chars,
            timeout: Duration::from_millis(config.timeout_ms),
        })
    }
}

/// Performs a candidate-source search using one configured upstream provider.
#[async_trait]
pub(super) trait SearchProvider: Send + Sync {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError>;
}

/// Reads one already selected URL and returns model-consumable document text.
#[async_trait]
pub(super) trait FetchProvider: Send + Sync {
    async fn fetch(
        &self,
        client: &Client,
        request: &FetchRequest,
    ) -> Result<WebFetchOutput, WebToolError>;
}

fn routed_candidates(request: &SearchRequest) -> Vec<WebSearchProvider> {
    if let Some(SourceHint::Github) = request.source_hint {
        return vec![
            WebSearchProvider::Github,
            WebSearchProvider::Exa,
            WebSearchProvider::Brave,
        ];
    }
    if let Some(SourceHint::Community) = request.source_hint {
        return vec![
            WebSearchProvider::StackExchange,
            WebSearchProvider::Exa,
            WebSearchProvider::Brave,
        ];
    }
    if let Some(SourceHint::Docs | SourceHint::Technical) = request.source_hint {
        return vec![
            WebSearchProvider::Exa,
            WebSearchProvider::Brave,
            WebSearchProvider::Github,
        ];
    }
    if let Some(SourceHint::Research) = request.source_hint {
        return vec![
            WebSearchProvider::Tavily,
            WebSearchProvider::Exa,
            WebSearchProvider::Brave,
        ];
    }
    if let Some(SourceHint::News) = request.source_hint {
        return vec![
            WebSearchProvider::Brave,
            WebSearchProvider::Tavily,
            WebSearchProvider::Exa,
        ];
    }
    vec![WebSearchProvider::Brave, WebSearchProvider::Exa]
}

fn dedupe_providers(providers: Vec<WebSearchProvider>) -> Vec<WebSearchProvider> {
    let mut seen = HashSet::new();
    providers
        .into_iter()
        .filter(|provider| seen.insert(*provider))
        .collect()
}

struct SkippedProvider {
    provider: WebSearchProvider,
    reason: String,
}

fn missing_search_secret(
    provider: WebSearchProvider,
    request: &SearchRequest,
) -> Result<Option<String>, WebToolError> {
    let Some((provider_name, env_var)) = required_search_secret(provider, request) else {
        return Ok(None);
    };
    if request.optional_secret(env_var)?.is_some() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "missing {provider_name} API key environment variable {env_var}"
        )))
    }
}

fn required_search_secret<'a>(
    provider: WebSearchProvider,
    request: &'a SearchRequest,
) -> Option<(&'static str, &'a str)> {
    match provider {
        WebSearchProvider::Brave => Some(("brave", request.brave_api_key_env.as_str())),
        WebSearchProvider::Exa => Some(("exa", request.exa_api_key_env.as_str())),
        WebSearchProvider::Tavily => Some(("tavily", request.tavily_api_key_env.as_str())),
        WebSearchProvider::Jina => Some(("jina", request.jina_api_key_env.as_str())),
        WebSearchProvider::Github if github_search_requires_token(request) => {
            Some(("github", request.github_token_env.as_str()))
        }
        WebSearchProvider::Github | WebSearchProvider::StackExchange => None,
    }
}

fn github_search_requires_token(request: &SearchRequest) -> bool {
    if matches!(request.github.search_type, Some(GithubSearchType::Code)) {
        return true;
    }
    let query = request.query.to_ascii_lowercase();
    query.contains("repo:")
        || query.contains("path:")
        || query.contains("filename:")
        || query.contains(" in:file")
}

fn no_available_provider_error(skipped: &[SkippedProvider]) -> WebToolError {
    if skipped.is_empty() {
        return WebToolError::InvalidArguments("no search provider is configured".to_string());
    }
    WebToolError::InvalidArguments(format!(
        "no available search provider; skipped {}",
        format_skipped_providers(skipped)
    ))
}

fn format_provider_list(providers: &[WebSearchProvider]) -> String {
    if providers.is_empty() {
        return "none".to_string();
    }
    providers
        .iter()
        .map(|provider| web_search_provider_name(*provider))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_skipped_providers(skipped: &[SkippedProvider]) -> String {
    if skipped.is_empty() {
        return "none".to_string();
    }
    skipped
        .iter()
        .map(|skip| {
            format!(
                "{}({})",
                web_search_provider_name(skip.provider),
                skip.reason
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn provider_policy_name(policy: ProviderPolicy) -> &'static str {
    match policy {
        ProviderPolicy::Auto => "auto",
        ProviderPolicy::Single => "single",
        ProviderPolicy::Fanout => "fanout",
    }
}

fn source_hint_name(source_hint: SourceHint) -> &'static str {
    match source_hint {
        SourceHint::General => "general",
        SourceHint::Technical => "technical",
        SourceHint::Github => "github",
        SourceHint::Docs => "docs",
        SourceHint::Community => "community",
        SourceHint::Research => "research",
        SourceHint::News => "news",
    }
}

fn web_search_provider_name(provider: WebSearchProvider) -> &'static str {
    match provider {
        WebSearchProvider::Brave => "brave",
        WebSearchProvider::Jina => "jina",
        WebSearchProvider::Github => "github",
        WebSearchProvider::Exa => "exa",
        WebSearchProvider::Tavily => "tavily",
        WebSearchProvider::StackExchange => "stack_exchange",
    }
}

fn web_fetch_provider_name(provider: WebFetchProvider) -> &'static str {
    match provider {
        WebFetchProvider::Jina => "jina",
        WebFetchProvider::Direct => "direct",
    }
}

fn safe_error_summary(err: &WebToolError) -> String {
    match err {
        WebToolError::MissingApiKey { provider, env_var } => {
            format!("{provider} missing API key environment variable {env_var}")
        }
        WebToolError::Http {
            provider, status, ..
        } => format!("{provider} returned HTTP {status}"),
        WebToolError::Network { provider, .. } => format!("{provider} request failed"),
        WebToolError::Parse { provider, .. } => format!("{provider} response parse failed"),
        WebToolError::Disabled { tool } => format!("{tool} is disabled"),
        WebToolError::InvalidArguments(message) => format!("invalid arguments: {message}"),
        WebToolError::UnsafeUrl(message) => format!("unsafe URL rejected: {message}"),
        WebToolError::SecretStore { .. } => "secret store error".to_string(),
    }
}
