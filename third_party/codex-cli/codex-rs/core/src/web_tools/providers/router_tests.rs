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
fn fanout_route_keeps_candidates_until_availability_filtering() {
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
        vec![
            WebSearchProvider::Brave,
            WebSearchProvider::Exa,
            WebSearchProvider::Jina
        ]
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

#[test]
fn availability_filter_skips_paid_providers_without_keys() {
    let mut config = WebSearchConfig::default();
    config.client.brave_api_key_env = "WHALE_TEST_MISSING_BRAVE_SEARCH_API_KEY".to_string();
    config.client.exa_api_key_env = "WHALE_TEST_MISSING_EXA_API_KEY".to_string();
    config.client.tavily_api_key_env = "WHALE_TEST_MISSING_TAVILY_API_KEY".to_string();
    config.client.jina_api_key_env = "WHALE_TEST_MISSING_JINA_API_KEY".to_string();
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let mut args = search_args("rust async stack overflow", None);
    args.provider_policy = Some(ProviderPolicy::Fanout);
    args.preferred_providers = Some(vec![
        WebSearchProvider::Tavily,
        WebSearchProvider::Brave,
        WebSearchProvider::StackExchange,
        WebSearchProvider::Github,
    ]);
    let request = SearchRequest::from_args(args, &config, PathBuf::from(".")).expect("request");

    let providers = registry.route_providers(&request);
    let (mut available, skipped) = registry.available_search_providers(providers, &request);
    available.truncate(config.client.max_providers_per_query);

    assert_eq!(
        available,
        vec![WebSearchProvider::StackExchange, WebSearchProvider::Github]
    );
    assert_eq!(skipped.len(), 3);
    assert!(
        skipped
            .iter()
            .any(|skip| skip.provider == WebSearchProvider::Tavily)
    );
    assert!(
        skipped
            .iter()
            .any(|skip| skip.provider == WebSearchProvider::Brave)
    );
    assert!(
        skipped
            .iter()
            .any(|skip| skip.provider == WebSearchProvider::Jina)
    );
}

#[test]
fn github_code_search_is_skipped_without_token() {
    let mut config = WebSearchConfig::default();
    config.client.github_token_env = "WHALE_TEST_MISSING_GITHUB_TOKEN".to_string();
    let registry = WebProviderRegistry::new(config.clone(), PathBuf::from(".")).expect("registry");
    let mut args = search_args("repo:openai/codex tool registry", Some(SourceHint::Github));
    args.github = Some(GithubSearchArgs {
        search_type: Some(GithubSearchType::Code),
        ..Default::default()
    });
    args.preferred_providers = Some(vec![WebSearchProvider::Github]);
    args.provider_policy = Some(ProviderPolicy::Single);
    let request = SearchRequest::from_args(args, &config, PathBuf::from(".")).expect("request");

    let providers = registry.route_providers(&request);
    let (available, skipped) = registry.available_search_providers(providers, &request);

    assert!(available.is_empty());
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].provider, WebSearchProvider::Github);
}
