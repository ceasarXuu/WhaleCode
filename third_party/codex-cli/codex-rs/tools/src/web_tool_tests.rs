use super::*;
use codex_protocol::config_types::WebSearchProvider;

#[test]
fn provider_tool_names_round_trip() {
    for provider in [
        WebSearchProvider::Brave,
        WebSearchProvider::Exa,
        WebSearchProvider::Tavily,
        WebSearchProvider::Github,
        WebSearchProvider::StackExchange,
        WebSearchProvider::Jina,
    ] {
        let name = web_search_provider_tool_name(provider);
        assert_eq!(web_search_provider_from_tool_name(name), Some(provider));
    }
}

#[test]
fn github_search_tool_does_not_expose_auto_or_provider_policy() {
    let tool = create_web_search_provider_tool(WebSearchProvider::Github);
    let ToolSpec::Function(function) = tool else {
        panic!("expected function tool");
    };

    assert_eq!(function.name, GITHUB_SEARCH_TOOL_NAME);
    let serialized = serde_json::to_string(&function.parameters).expect("serialize schema");
    assert!(serialized.contains("repositories"));
    assert!(serialized.contains("code"));
    assert!(!serialized.contains("provider_policy"));
    assert!(!serialized.contains("preferred_providers"));
    assert!(!serialized.contains("\"auto\""));
}
