use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use codex_protocol::config_types::WebSearchProvider;
use serde_json::json;
use std::collections::BTreeMap;

pub const WEB_FETCH_TOOL_NAME: &str = "web_fetch";
pub const BRAVE_SEARCH_TOOL_NAME: &str = "brave_search";
pub const EXA_SEARCH_TOOL_NAME: &str = "exa_search";
pub const TAVILY_SEARCH_TOOL_NAME: &str = "tavily_search";
pub const GITHUB_SEARCH_TOOL_NAME: &str = "github_search";
pub const STACK_EXCHANGE_SEARCH_TOOL_NAME: &str = "stack_exchange_search";
pub const JINA_SEARCH_TOOL_NAME: &str = "jina_search";

pub fn web_search_provider_tool_name(provider: WebSearchProvider) -> &'static str {
    match provider {
        WebSearchProvider::Brave => BRAVE_SEARCH_TOOL_NAME,
        WebSearchProvider::Exa => EXA_SEARCH_TOOL_NAME,
        WebSearchProvider::Tavily => TAVILY_SEARCH_TOOL_NAME,
        WebSearchProvider::Github => GITHUB_SEARCH_TOOL_NAME,
        WebSearchProvider::StackExchange => STACK_EXCHANGE_SEARCH_TOOL_NAME,
        WebSearchProvider::Jina => JINA_SEARCH_TOOL_NAME,
    }
}

pub fn web_search_provider_from_tool_name(name: &str) -> Option<WebSearchProvider> {
    match name {
        BRAVE_SEARCH_TOOL_NAME => Some(WebSearchProvider::Brave),
        EXA_SEARCH_TOOL_NAME => Some(WebSearchProvider::Exa),
        TAVILY_SEARCH_TOOL_NAME => Some(WebSearchProvider::Tavily),
        GITHUB_SEARCH_TOOL_NAME => Some(WebSearchProvider::Github),
        STACK_EXCHANGE_SEARCH_TOOL_NAME => Some(WebSearchProvider::StackExchange),
        JINA_SEARCH_TOOL_NAME => Some(WebSearchProvider::Jina),
        _ => None,
    }
}

pub fn create_web_search_provider_tools(providers: &[WebSearchProvider]) -> Vec<ToolSpec> {
    providers
        .iter()
        .copied()
        .map(create_web_search_provider_tool)
        .collect()
}

pub fn create_web_search_provider_tool(provider: WebSearchProvider) -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: web_search_provider_tool_name(provider).to_string(),
        description: web_search_provider_description(provider).to_string(),
        strict: false,
        defer_loading: None,
        parameters: web_search_provider_parameters(provider),
        output_schema: None,
    })
}

pub fn create_web_fetch_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "url".to_string(),
            JsonSchema::string(Some("HTTP or HTTPS URL to read.".to_string())),
        ),
        (
            "format".to_string(),
            JsonSchema::string_enum(
                vec![json!("markdown"), json!("text")],
                Some("Requested output format. Defaults to markdown.".to_string()),
            ),
        ),
        (
            "max_chars".to_string(),
            JsonSchema::integer(Some(
                "Maximum number of characters to return. Defaults to the configured limit."
                    .to_string(),
            )),
        ),
        (
            "reason".to_string(),
            JsonSchema::string(Some(
                "Brief reason this URL is needed for the current task.".to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: WEB_FETCH_TOOL_NAME.to_string(),
        description: "Reads the content of a previously discovered or user-provided HTTP(S) URL and returns markdown or text.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["url".to_string(), "reason".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

fn web_search_provider_description(provider: WebSearchProvider) -> &'static str {
    match provider {
        WebSearchProvider::Brave => {
            "Search Brave's broad web and news index for candidate sources. Use for general web, recent public information, official pages, and non-code-specific discovery."
        }
        WebSearchProvider::Exa => {
            "Search Exa for technical sources such as documentation, repositories, changelogs, Stack Overflow pages, engineering blogs, and semantic coding references."
        }
        WebSearchProvider::Tavily => {
            "Search Tavily for agent-oriented web research and pages likely to contain useful summaries or source context across multiple sites."
        }
        WebSearchProvider::Github => {
            "Search GitHub repositories, code, issues, commits, or users with structured GitHub filters such as repo, org, user, language, path, or filename."
        }
        WebSearchProvider::StackExchange => {
            "Search Stack Overflow and other Stack Exchange sites for technical questions and answers using tags, accepted-answer preference, and sorting hints."
        }
        WebSearchProvider::Jina => {
            "Search Jina Search for web pages with reader-friendly snippets. Use when Jina search is configured and its normalized web discovery is appropriate."
        }
    }
}

fn web_search_provider_parameters(provider: WebSearchProvider) -> JsonSchema {
    let mut properties = common_search_properties();
    match provider {
        WebSearchProvider::Github => {
            properties.insert("github".to_string(), github_search_options_schema());
        }
        WebSearchProvider::StackExchange => {
            properties.insert(
                "stack_exchange".to_string(),
                stack_exchange_options_schema(),
            );
        }
        WebSearchProvider::Brave
        | WebSearchProvider::Exa
        | WebSearchProvider::Tavily
        | WebSearchProvider::Jina => {}
    }

    JsonSchema::object(
        properties,
        Some(vec!["query".to_string()]),
        Some(false.into()),
    )
}

fn common_search_properties() -> BTreeMap<String, JsonSchema> {
    BTreeMap::from([
        (
            "query".to_string(),
            JsonSchema::string(Some("Search query.".to_string())),
        ),
        (
            "max_results".to_string(),
            JsonSchema::integer(Some(
                "Maximum number of ranked results to return.".to_string(),
            )),
        ),
        (
            "freshness".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("any"),
                    json!("day"),
                    json!("week"),
                    json!("month"),
                    json!("year"),
                ],
                Some("Optional recency preference.".to_string()),
            ),
        ),
        (
            "domains".to_string(),
            JsonSchema::array(
                JsonSchema::string(None),
                Some("Optional domains to prefer, such as github.com or docs.rs.".to_string()),
            ),
        ),
        (
            "exclude_domains".to_string(),
            JsonSchema::array(
                JsonSchema::string(None),
                Some("Optional domains to exclude.".to_string()),
            ),
        ),
    ])
}

fn github_search_options_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "search_type".to_string(),
                JsonSchema::string_enum(
                    vec![
                        json!("repositories"),
                        json!("code"),
                        json!("issues"),
                        json!("commits"),
                        json!("users"),
                    ],
                    Some("GitHub search category.".to_string()),
                ),
            ),
            (
                "repo".to_string(),
                JsonSchema::string(Some(
                    "Optional owner/name repository qualifier.".to_string(),
                )),
            ),
            (
                "org".to_string(),
                JsonSchema::string(Some("Optional GitHub organization qualifier.".to_string())),
            ),
            (
                "user".to_string(),
                JsonSchema::string(Some("Optional GitHub user qualifier.".to_string())),
            ),
            (
                "language".to_string(),
                JsonSchema::string(Some("Optional programming language qualifier.".to_string())),
            ),
            (
                "path".to_string(),
                JsonSchema::string(Some("Optional repository path qualifier.".to_string())),
            ),
            (
                "filename".to_string(),
                JsonSchema::string(Some("Optional filename qualifier.".to_string())),
            ),
        ]),
        None,
        Some(false.into()),
    )
}

fn stack_exchange_options_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "site".to_string(),
                JsonSchema::string(Some(
                    "Stack Exchange site, such as stackoverflow.".to_string(),
                )),
            ),
            (
                "tags".to_string(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Optional Stack Exchange tags.".to_string()),
                ),
            ),
            (
                "accepted".to_string(),
                JsonSchema::boolean(Some(
                    "Prefer questions with accepted answers when supported.".to_string(),
                )),
            ),
            (
                "sort".to_string(),
                JsonSchema::string_enum(
                    vec![
                        json!("activity"),
                        json!("votes"),
                        json!("creation"),
                        json!("relevance"),
                    ],
                    Some("Optional Stack Exchange sort order.".to_string()),
                ),
            ),
        ]),
        None,
        Some(false.into()),
    )
}

#[cfg(test)]
#[path = "web_tool_tests.rs"]
mod tests;
