use super::FetchFormat;
use super::SearchFreshness;
use super::SourceHint;
use super::WebFetchOutput;
use super::WebSearchResult;
use crate::web_tools::WebToolError;
use crate::web_tools::safety::sanitized_url_for_event;
use regex_lite::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashSet;
use url::Url;

pub(super) fn parse_brave_results(
    body: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, WebToolError> {
    let response: BraveResponse =
        serde_json::from_str(body).map_err(|err| WebToolError::Parse {
            provider: "brave",
            message: err.to_string(),
        })?;
    let Some(web) = response.web else {
        return Ok(Vec::new());
    };

    Ok(web
        .results
        .into_iter()
        .take(limit)
        .enumerate()
        .filter_map(|(index, item)| {
            let url = Url::parse(&item.url).ok()?;
            let domain = url.domain().unwrap_or_default().to_string();
            Some(WebSearchResult {
                rank: index + 1,
                title: item.title,
                url: item.url,
                domain: domain.clone(),
                snippet: item.description.unwrap_or_default(),
                published_at: item.page_age.or(item.age),
                source_type: source_type_for_domain(&domain),
                provider: "brave".to_string(),
            })
        })
        .collect())
}

pub(super) fn parse_jina_search_results(
    body: &str,
    limit: usize,
) -> Result<Vec<WebSearchResult>, WebToolError> {
    let url_re = Regex::new(r#"https?://[^\s<>)\]\"']+"#).map_err(|err| WebToolError::Parse {
        provider: "jina",
        message: err.to_string(),
    })?;
    let lines = body.lines().collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        for mat in url_re.find_iter(line) {
            let url = mat.as_str().trim_end_matches(['.', ',', ';', ':']);
            if !seen.insert(url.to_string()) {
                continue;
            }
            let Ok(parsed) = Url::parse(url) else {
                continue;
            };
            let domain = parsed.domain().unwrap_or_default().to_string();
            results.push(WebSearchResult {
                rank: results.len() + 1,
                title: infer_title(line, lines.get(line_index.saturating_sub(1)).copied()),
                url: url.to_string(),
                domain: domain.clone(),
                snippet: infer_snippet(&lines, line_index),
                published_at: None,
                source_type: source_type_for_domain(&domain),
                provider: "jina".to_string(),
            });
            if results.len() >= limit {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

pub(super) fn build_fetch_output(
    provider: &str,
    url: &Url,
    final_url: &Url,
    body: String,
    max_chars: usize,
    format: FetchFormat,
    body_truncated: bool,
) -> WebFetchOutput {
    let title = infer_document_title(&body);
    let mut content = match format {
        FetchFormat::Markdown => body,
        FetchFormat::Text => markdown_to_text(&body),
    };
    let original_chars = content.chars().count();
    let truncated = body_truncated || original_chars > max_chars;
    if truncated {
        content = content.chars().take(max_chars).collect();
    }
    let content_chars = content.chars().count();

    WebFetchOutput {
        provider: provider.to_string(),
        url: sanitized_url_for_event(url),
        final_url: sanitized_url_for_event(final_url),
        title,
        content,
        content_chars,
        truncated,
        latency_ms: 0,
    }
}

pub(super) async fn read_limited_body(
    mut response: reqwest::Response,
    provider: &'static str,
    max_chars: usize,
) -> Result<(String, bool), WebToolError> {
    let max_bytes = max_chars
        .saturating_mul(4)
        .saturating_add(8 * 1024)
        .clamp(16 * 1024, 1024 * 1024);
    let mut bytes = Vec::new();
    let mut truncated = false;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|source| WebToolError::Network { provider, source })?
    {
        let remaining = max_bytes.saturating_sub(bytes.len());
        if chunk.len() > remaining {
            bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        bytes.extend_from_slice(&chunk);
        if bytes.len() >= max_bytes {
            truncated = true;
            break;
        }
    }

    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

pub(super) fn shape_query(query: &str, args: &super::WebSearchArgs) -> String {
    let mut shaped = query.to_string();
    if let Some(source_hint) = args.source_hint {
        match source_hint {
            SourceHint::General => {}
            SourceHint::Technical => shaped.push_str(" documentation OR issue OR example"),
            SourceHint::Github => shaped.push_str(" site:github.com"),
            SourceHint::Docs => shaped.push_str(" documentation docs"),
            SourceHint::Community => shaped.push_str(" forum OR stackoverflow OR discussion"),
            SourceHint::Research => shaped.push_str(" paper benchmark analysis"),
            SourceHint::News => shaped.push_str(" latest news"),
        }
    }
    for domain in args.domains.clone().unwrap_or_default().into_iter().take(8) {
        if let Some(domain) = sanitize_domain(&domain) {
            shaped.push_str(" site:");
            shaped.push_str(&domain);
        }
    }
    for domain in args
        .exclude_domains
        .clone()
        .unwrap_or_default()
        .into_iter()
        .take(8)
    {
        if let Some(domain) = sanitize_domain(&domain) {
            shaped.push_str(" -site:");
            shaped.push_str(&domain);
        }
    }
    shaped
}

pub(super) fn brave_freshness_param(freshness: SearchFreshness) -> Option<&'static str> {
    match freshness {
        SearchFreshness::Any => None,
        SearchFreshness::Day => Some("pd"),
        SearchFreshness::Week => Some("pw"),
        SearchFreshness::Month => Some("pm"),
        SearchFreshness::Year => Some("py"),
    }
}

pub(super) fn http_error(provider: &'static str, status: StatusCode, body: String) -> WebToolError {
    WebToolError::Http {
        provider,
        status: status.as_u16(),
        message: body.chars().take(240).collect(),
    }
}

pub(super) fn fallback_reason(err: &WebToolError) -> String {
    match err {
        WebToolError::MissingApiKey { env_var, .. } => {
            format!("missing environment variable {env_var}")
        }
        WebToolError::Http {
            provider, status, ..
        } => format!("{provider} returned HTTP {status}"),
        WebToolError::Network { provider, .. } => format!("{provider} request failed"),
        _ => err.to_string(),
    }
}

fn sanitize_domain(domain: &str) -> Option<String> {
    let domain = domain
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_matches('/')
        .to_ascii_lowercase();
    if domain.is_empty()
        || domain.contains('/')
        || domain.contains(' ')
        || domain.contains('@')
        || domain == "localhost"
    {
        None
    } else {
        Some(domain)
    }
}

fn infer_title(line: &str, previous_line: Option<&str>) -> String {
    if let Some((prefix, _)) = line.split_once("](")
        && let Some(start) = prefix.rfind('[')
    {
        let title = prefix[start + 1..].trim();
        if !title.is_empty() {
            return title.to_string();
        }
    }
    for candidate in [previous_line.unwrap_or_default(), line] {
        let title = candidate
            .trim()
            .trim_start_matches('#')
            .trim_start_matches('-')
            .trim();
        if !title.is_empty() && !title.starts_with("http") {
            return title.to_string();
        }
    }
    "Untitled result".to_string()
}

fn infer_snippet(lines: &[&str], line_index: usize) -> String {
    lines
        .get(line_index + 1)
        .copied()
        .unwrap_or_default()
        .trim()
        .trim_start_matches("Markdown Content:")
        .trim()
        .chars()
        .take(280)
        .collect()
}

fn infer_document_title(body: &str) -> Option<String> {
    for line in body.lines().take(30) {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("Title:") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
        if let Some(title) = trimmed.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

fn markdown_to_text(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            line.trim_start_matches('#')
                .trim_start_matches('>')
                .trim_start_matches('-')
                .trim()
        })
        .collect::<Vec<_>>()
        .join("\n")
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

#[derive(Deserialize)]
struct BraveResponse {
    web: Option<BraveWeb>,
}

#[derive(Deserialize)]
struct BraveWeb {
    results: Vec<BraveResult>,
}

#[derive(Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    description: Option<String>,
    age: Option<String>,
    page_age: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brave_response_is_parsed_into_ranked_results() {
        let body = r#"{
            "web": { "results": [
                { "title": "Codex", "url": "https://github.com/openai/codex", "description": "Agent", "page_age": "2026-04-01" }
            ] }
        }"#;

        let results = parse_brave_results(body, 5).expect("parse");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rank, 1);
        assert_eq!(results[0].domain, "github.com");
        assert_eq!(results[0].source_type, "github");
    }

    #[test]
    fn jina_response_extracts_urls_and_titles() {
        let body = r#"
## Search Results
[Codex README](https://github.com/openai/codex)
Markdown Content: A coding agent.
URL Source: https://docs.rs/reqwest/latest/reqwest/
HTTP client docs.
"#;

        let results = parse_jina_search_results(body, 5).expect("parse");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Codex README");
        assert_eq!(results[1].source_type, "docs");
    }

    #[test]
    fn fetch_output_truncates_on_char_boundary() {
        let url = Url::parse("https://example.com/page").expect("url");
        let output = build_fetch_output(
            "jina",
            &url,
            &url,
            "# Title\nabcdef😀ghijk".to_string(),
            10,
            FetchFormat::Markdown,
            false,
        );

        assert!(output.truncated);
        assert_eq!(output.content_chars, 10);
        assert!(output.content.is_char_boundary(output.content.len()));
    }

    #[test]
    fn fetch_output_marks_body_limit_truncation() {
        let url = Url::parse("https://example.com/page").expect("url");
        let output = build_fetch_output(
            "jina",
            &url,
            &url,
            "short".to_string(),
            50,
            FetchFormat::Markdown,
            true,
        );

        assert!(output.truncated);
        assert_eq!(output.content, "short");
    }
}
