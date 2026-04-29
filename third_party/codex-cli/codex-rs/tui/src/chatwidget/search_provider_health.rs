use super::search_provider_support::web_search_provider_name;
use crate::app_event::AppEvent;
use codex_protocol::config_types::WebSearchProvider;

pub(super) fn spawn_search_provider_health_check(
    tx: crate::app_event_sender::AppEventSender,
    provider: WebSearchProvider,
    secret: String,
    stack_exchange_site: String,
) {
    tokio::spawn(async move {
        let provider_name = web_search_provider_name(provider);
        let result = search_provider_health_check(provider, &secret, &stack_exchange_site).await;
        let cell = match result {
            Ok(()) => crate::history_cell::new_info_event(
                format!("search_provider={provider_name} health_check=ok"),
                /*hint*/ None,
            ),
            Err(err) => crate::history_cell::new_error_event(format!(
                "search_provider={provider_name} health_check=failed: {err}"
            )),
        };
        tx.send(AppEvent::InsertHistoryCell(Box::new(cell)));
    });
}

async fn search_provider_health_check(
    provider: WebSearchProvider,
    secret: &str,
    stack_exchange_site: &str,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("whale-code-search-provider-setup/1.0")
        .build()
        .map_err(|err| err.to_string())?;
    let response = match provider {
        WebSearchProvider::Brave => {
            client
                .get("https://api.search.brave.com/res/v1/web/search")
                .header("Accept", "application/json")
                .header("X-Subscription-Token", secret)
                .query(&[("q", "whale code health check"), ("count", "1")])
                .send()
                .await
        }
        WebSearchProvider::Github => {
            client
                .get("https://api.github.com/rate_limit")
                .header("Accept", "application/vnd.github+json")
                .bearer_auth(secret)
                .send()
                .await
        }
        WebSearchProvider::Exa => {
            client
                .post("https://api.exa.ai/search")
                .header("Accept", "application/json")
                .header("x-api-key", secret)
                .json(&serde_json::json!({
                    "query": "whale code health check",
                    "numResults": 1,
                    "contents": { "summary": false, "text": false }
                }))
                .send()
                .await
        }
        WebSearchProvider::Tavily => {
            client
                .post("https://api.tavily.com/search")
                .header("Accept", "application/json")
                .bearer_auth(secret)
                .json(&serde_json::json!({
                    "query": "whale code health check",
                    "max_results": 1,
                    "search_depth": "basic"
                }))
                .send()
                .await
        }
        WebSearchProvider::StackExchange => {
            client
                .get("https://api.stackexchange.com/2.3/info")
                .header("Accept", "application/json")
                .query(&[
                    ("site", stack_exchange_site),
                    ("key", secret),
                    ("filter", "default"),
                ])
                .send()
                .await
        }
        WebSearchProvider::Jina => {
            client
                .get("https://s.jina.ai/whale%20code%20health%20check")
                .header("Accept", "text/plain")
                .bearer_auth(secret)
                .send()
                .await
        }
    }
    .map_err(|err| err.to_string())?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", status.as_u16()))
    }
}
