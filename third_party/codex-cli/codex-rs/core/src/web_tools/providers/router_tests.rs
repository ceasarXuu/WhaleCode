use super::*;

fn search_args(query: &str, source_hint: Option<SourceHint>) -> WebSearchArgs {
    WebSearchArgs {
        query: query.to_string(),
        max_results: None,
        freshness: None,
        domains: None,
        exclude_domains: None,
        source_hint,
        provider_policy: None,
        preferred_providers: None,
        github: None,
        stack_exchange: None,
    }
}

#[test]
fn github_hint_routes_to_github_before_general_search() {
    let config = WebSearchConfig::default();
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let request = SearchRequest::from_args(
        search_args("repo:openai/codex tool registry", Some(SourceHint::Github)),
        &config,
        PathBuf::from("."),
    )
    .expect("request");

    let providers = registry.route_providers(&request);

    assert_eq!(providers[0], WebSearchProvider::Github);
    assert!(providers.contains(&WebSearchProvider::Exa));
}

#[test]
fn configured_provider_leads_auto_route() {
    let mut config = WebSearchConfig::default();
    config.client.provider = WebSearchProvider::Tavily;
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let request = SearchRequest::from_args(
        search_args("repo:openai/codex tool registry", Some(SourceHint::Github)),
        &config,
        PathBuf::from("."),
    )
    .expect("request");

    let providers = registry.route_providers(&request);

    assert_eq!(providers[0], WebSearchProvider::Tavily);
    assert!(providers.contains(&WebSearchProvider::Github));
}

#[test]
fn single_policy_uses_configured_provider() {
    let mut config = WebSearchConfig::default();
    config.client.provider = WebSearchProvider::Exa;
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let mut args = search_args("rust async stack overflow", Some(SourceHint::Community));
    args.provider_policy = Some(ProviderPolicy::Single);
    let request = SearchRequest::from_args(args, &config, PathBuf::from(".")).expect("request");

    let providers = registry.route_providers(&request);

    assert_eq!(providers, vec![WebSearchProvider::Exa]);
}

#[test]
fn fanout_dedupes_before_limiting_provider_count() {
    let mut config = WebSearchConfig::default();
    config.client.provider = WebSearchProvider::Brave;
    config.client.max_providers_per_query = 2;
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let mut args = search_args("rust async stack overflow", None);
    args.provider_policy = Some(ProviderPolicy::Fanout);
    let request = SearchRequest::from_args(args, &config, PathBuf::from(".")).expect("request");

    let providers = registry.route_providers(&request);

    assert_eq!(
        providers,
        vec![WebSearchProvider::Brave, WebSearchProvider::Exa]
    );
}

#[test]
fn preferred_single_provider_bypasses_auto_route() {
    let config = WebSearchConfig::default();
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let mut args = search_args("rust async stack overflow", Some(SourceHint::Community));
    args.provider_policy = Some(ProviderPolicy::Single);
    args.preferred_providers = Some(vec![WebSearchProvider::StackExchange]);
    let request = SearchRequest::from_args(args, &config, PathBuf::from(".")).expect("request");

    let providers = registry.route_providers(&request);

    assert_eq!(providers, vec![WebSearchProvider::StackExchange]);
}
