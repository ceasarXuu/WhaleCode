use super::GithubSearchType;
use super::SearchFreshness;
use super::SearchProvider;
use super::SearchRequest;
use super::WebSearchOutput;
use super::result_parsers::parse_exa_results;
use super::result_parsers::parse_github_results;
use super::result_parsers::parse_stack_exchange_results;
use super::result_parsers::parse_tavily_results;
use super::support::brave_freshness_param;
use super::support::http_error;
use super::support::parse_brave_results;
use super::support::parse_jina_search_results;
use crate::web_tools::WebToolError;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

pub(super) struct BraveSearchProvider;

#[async_trait]
impl SearchProvider for BraveSearchProvider {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let api_key = request.required_secret("brave", &request.brave_api_key_env)?;
        let mut query_params = vec![
            ("q", request.shaped_query.clone()),
            ("count", request.max_results.to_string()),
        ];
        if let Some(freshness) = request.freshness.and_then(brave_freshness_param) {
            query_params.push(("freshness", freshness.to_string()));
        }

        let body = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .timeout(request.timeout)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", api_key)
            .query(&query_params)
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "brave",
                source,
            })?;
        response_text("brave", body).await.and_then(|body| {
            output(
                "brave",
                request,
                parse_brave_results(&body, request.max_results)?,
            )
        })
    }
}

pub(super) struct JinaSearchProvider;

#[async_trait]
impl SearchProvider for JinaSearchProvider {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let api_key = request.required_secret("jina", &request.jina_api_key_env)?;
        let encoded_query = url::form_urlencoded::byte_serialize(request.shaped_query.as_bytes())
            .collect::<String>();
        let body = client
            .get(format!("https://s.jina.ai/{encoded_query}"))
            .timeout(request.timeout)
            .header("Accept", "text/plain")
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "jina",
                source,
            })?;
        response_text("jina", body).await.and_then(|body| {
            output(
                "jina",
                request,
                parse_jina_search_results(&body, request.max_results)?,
            )
        })
    }
}

pub(super) struct ExaSearchProvider;

#[async_trait]
impl SearchProvider for ExaSearchProvider {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let api_key = request.required_secret("exa", &request.exa_api_key_env)?;
        let body = json!({
            "query": request.shaped_query,
            "numResults": request.max_results,
            "type": "auto",
            "contents": { "highlights": true, "summary": true, "text": false }
        });
        let response = client
            .post("https://api.exa.ai/search")
            .timeout(request.timeout)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .json(&body)
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "exa",
                source,
            })?;
        response_text("exa", response).await.and_then(|body| {
            output(
                "exa",
                request,
                parse_exa_results(&body, request.max_results)?,
            )
        })
    }
}

pub(super) struct TavilySearchProvider;

#[async_trait]
impl SearchProvider for TavilySearchProvider {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let api_key = request.required_secret("tavily", &request.tavily_api_key_env)?;
        let body = json!({
            "query": request.shaped_query,
            "search_depth": "basic",
            "max_results": request.max_results,
            "include_answer": false,
            "include_raw_content": false
        });
        let response = client
            .post("https://api.tavily.com/search")
            .timeout(request.timeout)
            .header("Accept", "application/json")
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "tavily",
                source,
            })?;
        response_text("tavily", response).await.and_then(|body| {
            output(
                "tavily",
                request,
                parse_tavily_results(&body, request.max_results)?,
            )
        })
    }
}

pub(super) struct GithubSearchProvider;

#[async_trait]
impl SearchProvider for GithubSearchProvider {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let search_type = github_search_type(request);
        let token = request.optional_secret(&request.github_token_env)?;
        if matches!(search_type, GithubSearchType::Code) && token.is_none() {
            return Err(WebToolError::MissingApiKey {
                provider: "github",
                env_var: request.github_token_env.clone(),
            });
        }
        let query = github_query(request, search_type);
        let mut builder = client
            .get(format!(
                "https://api.github.com/search/{}",
                github_endpoint(search_type)
            ))
            .timeout(request.timeout)
            .header("Accept", "application/vnd.github+json")
            .query(&[("q", query), ("per_page", request.max_results.to_string())]);
        if let Some(token) = token {
            builder = builder.bearer_auth(token);
        }
        let response = builder
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "github",
                source,
            })?;
        let source_type = match search_type {
            GithubSearchType::Repositories => "github_repo",
            GithubSearchType::Code => "github_code",
            GithubSearchType::Issues => "github_issue",
            GithubSearchType::Commits => "github_commit",
            GithubSearchType::Users => "github_user",
        };
        response_text("github", response).await.and_then(|body| {
            output(
                "github",
                request,
                parse_github_results(&body, request.max_results, source_type)?,
            )
        })
    }
}

pub(super) struct StackExchangeSearchProvider;

#[async_trait]
impl SearchProvider for StackExchangeSearchProvider {
    async fn search(
        &self,
        client: &Client,
        request: &SearchRequest,
    ) -> Result<WebSearchOutput, WebToolError> {
        let key = request.optional_secret(&request.stack_exchange_key_env)?;
        let site = request
            .stack_exchange
            .site
            .as_deref()
            .unwrap_or(&request.stack_exchange_site);
        let mut params = vec![
            ("order".to_string(), "desc".to_string()),
            (
                "sort".to_string(),
                request
                    .stack_exchange
                    .sort
                    .clone()
                    .unwrap_or_else(|| "relevance".to_string()),
            ),
            ("site".to_string(), site.to_string()),
            ("pagesize".to_string(), request.max_results.to_string()),
            ("filter".to_string(), "withbody".to_string()),
            ("q".to_string(), request.query.clone()),
        ];
        if let Some(tags) = &request.stack_exchange.tags
            && !tags.is_empty()
        {
            params.push(("tagged".to_string(), tags.join(";")));
        }
        if let Some(accepted) = request.stack_exchange.accepted {
            params.push(("accepted".to_string(), accepted.to_string()));
        }
        if let Some(key) = key {
            params.push(("key".to_string(), key));
        }

        let response = client
            .get("https://api.stackexchange.com/2.3/search/advanced")
            .timeout(request.timeout)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "stack_exchange",
                source,
            })?;
        response_text("stack_exchange", response)
            .await
            .and_then(|body| {
                output(
                    "stack_exchange",
                    request,
                    parse_stack_exchange_results(&body, request.max_results)?,
                )
            })
    }
}

async fn response_text(
    provider: &'static str,
    response: reqwest::Response,
) -> Result<String, WebToolError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|source| WebToolError::Network { provider, source })?;
    if !status.is_success() {
        return Err(http_error(provider, status, body));
    }
    Ok(body)
}

fn output(
    provider: &str,
    request: &SearchRequest,
    results: Vec<super::WebSearchResult>,
) -> Result<WebSearchOutput, WebToolError> {
    Ok(WebSearchOutput {
        provider: provider.to_string(),
        query: request.query.clone(),
        fallback_used: false,
        fallback_reason: None,
        results,
        latency_ms: 0,
    })
}

fn github_search_type(request: &SearchRequest) -> GithubSearchType {
    if let Some(search_type) = request.github.search_type {
        return search_type;
    }
    let query = request.query.to_ascii_lowercase();
    if query.contains("repo:")
        || query.contains("path:")
        || query.contains("filename:")
        || query.contains(" in:file")
    {
        GithubSearchType::Code
    } else {
        GithubSearchType::Repositories
    }
}

fn github_endpoint(search_type: GithubSearchType) -> &'static str {
    match search_type {
        GithubSearchType::Repositories => "repositories",
        GithubSearchType::Code => "code",
        GithubSearchType::Issues => "issues",
        GithubSearchType::Commits => "commits",
        GithubSearchType::Users => "users",
    }
}

fn github_query(request: &SearchRequest, search_type: GithubSearchType) -> String {
    let mut query = request.query.clone();
    if let Some(repo) = &request.github.repo {
        query.push_str(" repo:");
        query.push_str(repo);
    }
    if let Some(org) = &request.github.org {
        query.push_str(" org:");
        query.push_str(org);
    }
    if let Some(user) = &request.github.user {
        query.push_str(" user:");
        query.push_str(user);
    }
    if let Some(language) = &request.github.language {
        query.push_str(" language:");
        query.push_str(language);
    }
    if let Some(path) = &request.github.path {
        query.push_str(" path:");
        query.push_str(path);
    }
    if let Some(filename) = &request.github.filename {
        query.push_str(" filename:");
        query.push_str(filename);
    }
    if matches!(search_type, GithubSearchType::Code) && !query.contains(" in:file") {
        query.push_str(" in:file");
    }
    query
}

#[allow(dead_code)]
fn _freshness_is_any(freshness: Option<SearchFreshness>) -> bool {
    freshness.is_none() || matches!(freshness, Some(SearchFreshness::Any))
}
