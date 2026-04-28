use super::WebSearchOutput;
use super::WebSearchResult;
use crate::web_tools::WebToolError;
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub(super) fn parse_exa_results(
    body: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, WebToolError> {
    let value = parse_json_provider("exa", body)?;
    let Some(results) = value.get("results").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(results
        .iter()
        .take(limit)
        .enumerate()
        .filter_map(|(index, item)| {
            let url = item.get("url").and_then(Value::as_str)?;
            let title = string_field(item, &["title"]).unwrap_or("Untitled result");
            let snippet = string_field(item, &["summary", "text"])
                .or_else(|| first_array_string(item, "highlights"))
                .unwrap_or_default();
            let published = string_field(item, &["publishedDate", "published_at"]);
            result_from_url("exa", index + 1, title, url, snippet, published)
        })
        .collect())
}

pub(super) fn parse_tavily_results(
    body: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, WebToolError> {
    let value = parse_json_provider("tavily", body)?;
    let Some(results) = value.get("results").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(results
        .iter()
        .take(limit)
        .enumerate()
        .filter_map(|(index, item)| {
            let url = item.get("url").and_then(Value::as_str)?;
            let title = string_field(item, &["title"]).unwrap_or("Untitled result");
            let snippet = string_field(item, &["content", "raw_content"]).unwrap_or_default();
            let published = string_field(item, &["published_date", "publishedDate"]);
            result_from_url("tavily", index + 1, title, url, snippet, published)
        })
        .collect())
}

pub(super) fn parse_github_results(
    body: &str,
    limit: usize,
    source_type: &str,
) -> Result<Vec<WebSearchResult>, WebToolError> {
    let value = parse_json_provider("github", body)?;
    let Some(items) = value.get("items").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(items
        .iter()
        .take(limit)
        .enumerate()
        .filter_map(|(index, item)| {
            let url = string_field(item, &["html_url", "url"])?;
            let title = github_title(item);
            let snippet = github_snippet(item);
            let published = string_field(item, &["updated_at", "created_at"]);
            let mut result =
                result_from_url("github", index + 1, &title, url, &snippet, published)?;
            result.source_type = source_type.to_string();
            Some(result)
        })
        .collect())
}

pub(super) fn parse_stack_exchange_results(
    body: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, WebToolError> {
    let value = parse_json_provider("stack_exchange", body)?;
    let Some(items) = value.get("items").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(items
        .iter()
        .take(limit)
        .enumerate()
        .filter_map(|(index, item)| {
            let url = item.get("link").and_then(Value::as_str)?;
            let title = string_field(item, &["title"]).unwrap_or("Untitled question");
            let snippet = string_field(item, &["body_markdown", "body"])
                .map(strip_html)
                .unwrap_or_default();
            let mut result =
                result_from_url("stack_exchange", index + 1, title, url, &snippet, None)?;
            result.source_type = "stack_exchange_question".to_string();
            Some(result)
        })
        .collect())
}

pub(super) fn merge_ranked_results(
    outputs: Vec<WebSearchOutput>,
    limit: usize,
) -> Vec<WebSearchResult> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for mut result in outputs.into_iter().flat_map(|output| output.results) {
        let key = canonical_url_key(&result.url);
        if !seen.insert(key) {
            continue;
        }
        result.rank = merged.len() + 1;
        merged.push(result);
        if merged.len() >= limit {
            break;
        }
    }
    merged
}

fn parse_json_provider(provider: &'static str, body: &str) -> Result<Value, WebToolError> {
    serde_json::from_str(body).map_err(|err| WebToolError::Parse {
        provider,
        message: err.to_string(),
    })
}

fn result_from_url(
    provider: &str,
    rank: usize,
    title: &str,
    raw_url: &str,
    snippet: &str,
    published_at: Option<&str>,
) -> Option<WebSearchResult> {
    let url = Url::parse(raw_url).ok()?;
    let domain = url.domain().unwrap_or_default().to_string();
    Some(WebSearchResult {
        rank,
        title: html_unescape(title).chars().take(180).collect(),
        url: raw_url.to_string(),
        domain: domain.clone(),
        snippet: strip_html(snippet).chars().take(420).collect(),
        published_at: published_at.map(str::to_string),
        source_type: source_type_for_domain(&domain),
        provider: provider.to_string(),
    })
}

fn string_field<'a>(item: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| item.get(*key).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
}

fn first_array_string<'a>(item: &'a Value, key: &str) -> Option<&'a str> {
    item.get(key)
        .and_then(Value::as_array)
        .and_then(|values| values.iter().find_map(Value::as_str))
}

fn github_title(item: &Value) -> String {
    if let Some(full_name) = string_field(item, &["full_name"]) {
        return full_name.to_string();
    }
    if let Some(repo) = item.get("repository")
        && let Some(full_name) = string_field(repo, &["full_name"])
    {
        if let Some(path) = string_field(item, &["path", "name"]) {
            return format!("{full_name}/{path}");
        }
        return full_name.to_string();
    }
    string_field(item, &["title", "name", "login"])
        .unwrap_or("Untitled GitHub result")
        .to_string()
}

fn github_snippet(item: &Value) -> String {
    if let Some(description) = string_field(item, &["description", "body"]) {
        return description.to_string();
    }
    if let Some(text_matches) = item.get("text_matches").and_then(Value::as_array) {
        for matched in text_matches {
            if let Some(fragment) = string_field(matched, &["fragment"]) {
                return fragment.to_string();
            }
        }
    }
    String::new()
}

fn canonical_url_key(url: &str) -> String {
    if let Ok(mut parsed) = Url::parse(url) {
        parsed.set_fragment(None);
        parsed.set_query(None);
        parsed.to_string()
    } else {
        url.to_ascii_lowercase()
    }
}

fn strip_html(value: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html_unescape(value).chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn source_type_for_domain(domain: &str) -> String {
    if domain.ends_with("github.com") || domain.ends_with("githubusercontent.com") {
        "github".to_string()
    } else if domain.contains("docs") || domain.ends_with(".dev") || domain.ends_with(".rs") {
        "docs".to_string()
    } else if domain.contains("stackoverflow") || domain.contains("forum") {
        "community".to_string()
    } else {
        "web".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exa_response_keeps_provider_and_summary() {
        let body = r#"{
            "results": [{
                "title": "Search API guide",
                "url": "https://exa.ai/docs/reference/search-api-guide-for-coding-agents",
                "summary": "Search API for coding agents"
            }]
        }"#;

        let results = parse_exa_results(body, 3).expect("parse");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "exa");
        assert_eq!(results[0].source_type, "web");
        assert_eq!(results[0].snippet, "Search API for coding agents");
    }

    #[test]
    fn tavily_response_is_parsed_into_web_results() {
        let body = r#"{
            "results": [{
                "title": "Tavily Search",
                "url": "https://docs.tavily.com/documentation/api-reference/endpoint/search",
                "content": "Real-time search for agents"
            }]
        }"#;

        let results = parse_tavily_results(body, 3).expect("parse");

        assert_eq!(results[0].provider, "tavily");
        assert_eq!(results[0].domain, "docs.tavily.com");
    }

    #[test]
    fn github_response_preserves_repo_path_for_code_results() {
        let body = r#"{
            "items": [{
                "html_url": "https://github.com/openai/codex/blob/main/README.md",
                "path": "README.md",
                "repository": { "full_name": "openai/codex" }
            }]
        }"#;

        let results = parse_github_results(body, 3, "github_code").expect("parse");

        assert_eq!(results[0].provider, "github");
        assert_eq!(results[0].title, "openai/codex/README.md");
        assert_eq!(results[0].source_type, "github_code");
    }

    #[test]
    fn stack_exchange_response_strips_html_body() {
        let body = r#"{
            "items": [{
                "title": "How to test reqwest?",
                "link": "https://stackoverflow.com/questions/1/how-to-test-reqwest",
                "body": "<p>Use a local mock server.</p>"
            }]
        }"#;

        let results = parse_stack_exchange_results(body, 3).expect("parse");

        assert_eq!(results[0].provider, "stack_exchange");
        assert_eq!(results[0].source_type, "stack_exchange_question");
        assert_eq!(results[0].snippet, "Use a local mock server.");
    }

    #[test]
    fn merge_ranked_results_deduplicates_urls() {
        let output = |provider: &str| WebSearchOutput {
            provider: provider.to_string(),
            query: "codex".to_string(),
            fallback_used: false,
            fallback_reason: None,
            results: vec![WebSearchResult {
                rank: 1,
                title: provider.to_string(),
                url: "https://github.com/openai/codex?utm=1".to_string(),
                domain: "github.com".to_string(),
                snippet: String::new(),
                published_at: None,
                source_type: "github".to_string(),
                provider: provider.to_string(),
            }],
            latency_ms: 0,
        };

        let merged = merge_ranked_results(vec![output("github"), output("exa")], 5);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].rank, 1);
        assert_eq!(merged[0].provider, "github");
    }
}
