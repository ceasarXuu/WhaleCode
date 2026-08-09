use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::Prompt;
use super::ProviderProjectionIdentityExpectation;
use super::ProviderRequestAttribution;
use super::ProviderRequestBudgetContext;
use super::ProviderRequestBudgetLimits;
use super::ProviderRequestHardLimit;
use super::UnauthorizedRecoveryExecution;
use super::X_CODEX_INSTALLATION_ID_HEADER;
use super::X_CODEX_PARENT_THREAD_ID_HEADER;
use super::X_CODEX_TURN_METADATA_HEADER;
use super::X_CODEX_WINDOW_ID_HEADER;
use super::X_OPENAI_SUBAGENT_HEADER;
use super::apply_api_provider_request_hard_limit_retry_policy;
use super::apply_projection_identity_expectation;
use super::apply_provider_request_hard_limit_retry_policy;
use super::ensure_realtime_session_is_metered;
use super::provider_payload_digest;
use codex_api::RealtimeEventParser;
use codex_api::RealtimeSessionMode;
use codex_api::ToolChoice;
use codex_app_server_protocol::AuthMode;
use codex_model_provider::BearerAuthProvider;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_otel::SessionTelemetry;
use codex_protocol::ThreadId;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::protocol::TokenUsage;
use pretty_assertions::assert_eq;
use serde_json::json;

fn test_model_client(session_source: SessionSource) -> ModelClient {
    let provider = create_oss_provider_with_base_url("https://example.com/v1", WireApi::Responses);
    ModelClient::new(
        /*auth_manager*/ None,
        ThreadId::new(),
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        provider,
        session_source,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
    )
}

#[test]
fn provider_request_hard_limit_rejects_before_the_excess_dispatch() {
    let limit = ProviderRequestHardLimit::with_limit(2);
    assert!(limit.claim("/responses").is_ok());
    assert!(limit.claim("/responses").is_ok());
    let error = limit
        .claim("/responses")
        .expect_err("third dispatch must be rejected");
    assert_eq!(
        error.to_string(),
        "Fatal error: provider request hard limit reached (max 2)"
    );
    assert_eq!(limit.count.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn invalid_provider_request_hard_limit_fails_closed() {
    let limit = ProviderRequestHardLimit {
        limit: None,
        count: std::sync::atomic::AtomicUsize::new(0),
        state_path: None,
        configuration_error: Some(
            "WHALE_PROVIDER_REQUEST_HARD_LIMIT must be a positive integer".to_string(),
        ),
    };
    assert_eq!(
        limit
            .claim("/responses")
            .expect_err("invalid limit must reject")
            .to_string(),
        "Fatal error: WHALE_PROVIDER_REQUEST_HARD_LIMIT must be a positive integer"
    );
}

#[test]
fn model_clients_share_one_process_provider_request_hard_limit() {
    let first = test_model_client(SessionSource::Cli);
    let second = test_model_client(SessionSource::Cli);
    assert!(std::sync::Arc::ptr_eq(
        &first.state.provider_request_hard_limit,
        &second.state.provider_request_hard_limit,
    ));
}

#[test]
fn enabled_provider_request_hard_limit_disables_hidden_transport_retries() {
    let limit = ProviderRequestHardLimit::with_limit(2);
    let mut provider =
        create_oss_provider_with_base_url("https://example.com/v1", WireApi::Responses);
    provider.request_max_retries = Some(9);
    provider.stream_max_retries = Some(7);
    apply_provider_request_hard_limit_retry_policy(&mut provider, &limit);
    assert_eq!(provider.request_max_retries, Some(0));
    assert_eq!(provider.stream_max_retries, Some(0));

    let mut api_provider = provider
        .to_api_provider(/*auth_mode*/ None)
        .expect("create realtime API provider");
    apply_api_provider_request_hard_limit_retry_policy(&mut api_provider, &limit);
    assert_eq!(api_provider.retry.max_attempts, 0);
}

#[test]
fn provider_request_hard_limit_rejects_unmetered_realtime_generation() {
    let enabled = ProviderRequestHardLimit::with_limit(2);
    let disabled = ProviderRequestHardLimit::disabled();
    assert!(
        ensure_realtime_session_is_metered(
            &enabled,
            RealtimeEventParser::RealtimeV2,
            RealtimeSessionMode::Conversational
        )
        .is_err()
    );
    assert!(
        ensure_realtime_session_is_metered(
            &enabled,
            RealtimeEventParser::RealtimeV2,
            RealtimeSessionMode::Transcription
        )
        .is_err()
    );
    assert!(
        ensure_realtime_session_is_metered(
            &disabled,
            RealtimeEventParser::V1,
            RealtimeSessionMode::Conversational
        )
        .is_ok()
    );
}

#[test]
fn provider_request_hard_limit_uses_normalized_realtime_mode() {
    let enabled = ProviderRequestHardLimit::with_limit(2);
    assert!(
        ensure_realtime_session_is_metered(
            &enabled,
            RealtimeEventParser::V1,
            RealtimeSessionMode::Transcription
        )
        .is_err()
    );
}

#[test]
fn invalid_provider_request_hard_limit_rejects_realtime_before_connect() {
    let invalid = ProviderRequestHardLimit {
        limit: None,
        count: std::sync::atomic::AtomicUsize::new(0),
        state_path: None,
        configuration_error: Some("invalid hard limit".to_string()),
    };
    let error = ensure_realtime_session_is_metered(
        &invalid,
        RealtimeEventParser::RealtimeV2,
        RealtimeSessionMode::Transcription,
    )
    .expect_err("invalid hard limit must reject every Realtime session");
    assert_eq!(error.to_string(), "Fatal error: invalid hard limit");
}

#[cfg(unix)]
#[test]
fn provider_request_hard_limit_is_shared_across_process_state_instances() {
    let directory = tempfile::tempdir().expect("temporary hard-limit state");
    let state_path = directory.path().join("request-count");
    let first = ProviderRequestHardLimit {
        limit: Some(2),
        count: std::sync::atomic::AtomicUsize::new(0),
        state_path: Some(state_path.clone()),
        configuration_error: None,
    };
    let second = ProviderRequestHardLimit {
        limit: Some(2),
        count: std::sync::atomic::AtomicUsize::new(0),
        state_path: Some(state_path),
        configuration_error: None,
    };
    assert!(first.claim("/responses").is_ok());
    assert!(second.claim("/responses").is_ok());
    assert!(first.claim("/responses").is_err());
}

fn test_model_info() -> ModelInfo {
    serde_json::from_value(json!({
        "slug": "gpt-test",
        "display_name": "gpt-test",
        "description": "desc",
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            {"effort": "medium", "description": "medium"}
        ],
        "shell_type": "shell_command",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 1,
        "upgrade": null,
        "base_instructions": "base instructions",
        "model_messages": null,
        "supports_reasoning_summaries": false,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": {"mode": "bytes", "limit": 10000},
        "supports_parallel_tool_calls": false,
        "supports_image_detail_original": false,
        "context_window": 272000,
        "auto_compact_token_limit": null,
        "experimental_supported_tools": []
    }))
    .expect("deserialize test model info")
}

fn test_session_telemetry() -> SessionTelemetry {
    SessionTelemetry::new(
        ThreadId::new(),
        "gpt-test",
        "gpt-test",
        /*account_id*/ None,
        /*account_email*/ None,
        /*auth_mode*/ None,
        "test-originator".to_string(),
        /*log_user_prompts*/ false,
        "test-terminal".to_string(),
        SessionSource::Cli,
    )
}

#[test]
fn named_tool_choice_preserves_requested_chat_reasoning() {
    let provider_info =
        create_oss_provider_with_base_url("https://example.com/v1", WireApi::ChatCompletions);
    let api_provider = provider_info
        .to_api_provider(/*auth_mode*/ None)
        .expect("create chat completions provider");
    let client = ModelClient::new(
        /*auth_manager*/ None,
        ThreadId::new(),
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        provider_info,
        SessionSource::Cli,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
    );
    let session = client.new_session();
    let prompt = Prompt {
        tool_choice: ToolChoice::function("exec_command"),
        ..Prompt::default()
    };

    let request = session
        .build_responses_request(
            &api_provider,
            &prompt,
            &test_model_info(),
            Some(ReasoningEffort::Max),
            ReasoningSummary::None,
            /*service_tier*/ None,
        )
        .expect("build named-tool request");
    let mut taskspace_prompt = prompt.clone();
    taskspace_prompt.taskspace_capability_identity = Some(std::sync::Arc::from(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ));
    let taskspace_request = session
        .build_responses_request(
            &api_provider,
            &taskspace_prompt,
            &test_model_info(),
            Some(ReasoningEffort::Max),
            ReasoningSummary::None,
            /*service_tier*/ None,
        )
        .expect("build request with Runtime-only TaskSpace identity");
    assert_eq!(request, taskspace_request);

    assert_eq!(
        request.reasoning.and_then(|reasoning| reasoning.effort),
        Some(ReasoningEffort::Max)
    );
    assert_eq!(request.tool_choice, ToolChoice::function("exec_command"));
}

#[test]
fn provider_request_budget_observes_profile_hint_overrun_before_dispatch() {
    let budget = ProviderRequestBudgetContext::enabled(1, 1);

    let dispatch = budget
        .before_dispatch("responses_http")
        .expect("profile hint overrun should not block dispatch");

    assert_eq!(dispatch.request_count_before, 1);
    assert_eq!(dispatch.request_count_after, 2);
    assert_eq!(dispatch.budget_state_before, "over_profile_hint");
    assert_eq!(dispatch.budget_state_after, "over_profile_hint");

    let events = budget.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "started");
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 2);
    assert_eq!(events[0].max_requests, 1);
    assert_eq!(events[0].budget_state_before, "over_profile_hint");
    assert_eq!(events[0].budget_state_after, "over_profile_hint");
}

#[test]
fn provider_request_budget_compact_checkpoint_remains_advisory() {
    let budget = ProviderRequestBudgetContext::enabled(1, 2);

    let _dispatch = budget
        .before_dispatch("responses_http")
        .expect("compact checkpoint profile hint should not block dispatch");

    let events = budget.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "started");
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 2);
    assert_eq!(events[0].max_requests, 2);
    assert_eq!(events[0].budget_state_before, "compact_checkpoint_required");
    assert_eq!(events[0].budget_state_after, "over_profile_hint");
    assert_eq!(
        events[0].budget_transition_reason,
        "provider_request_profile_hint_exceeded"
    );
    assert_eq!(events[0].request_phase.as_deref(), None);

    budget.record_response_completed(None);
    let _ = budget.drain_events();
    let dispatch = budget
        .before_dispatch("responses_http")
        .expect("profile hint overrun should continue after terminal event");
    assert_eq!(dispatch.request_count_before, 2);
    assert_eq!(dispatch.request_count_after, 3);
}

#[test]
fn provider_request_budget_preserves_final_synthesis_phase_at_profile_hint() {
    let budget = ProviderRequestBudgetContext::enabled_with_attribution(
        ProviderRequestBudgetLimits {
            request_count: 1,
            max_requests: 2,
            node_request_count: 0,
            max_model_requests_per_node: usize::MAX,
            budget_state: "compact_checkpoint_required".to_string(),
        },
        ProviderRequestAttribution {
            request_phase: Some("final_synthesis".to_string()),
            ..ProviderRequestAttribution::default()
        },
        None,
    );

    let _dispatch = budget
        .before_dispatch("responses_http")
        .expect("final synthesis should be allowed at compact checkpoint");

    budget.record_response_completed(None);
    let events = budget.drain_events();
    assert_eq!(events[0].status, "started");
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 2);
    assert_eq!(events[0].budget_state_before, "compact_checkpoint_required");
    assert_eq!(events[0].budget_state_after, "over_profile_hint");
    assert_eq!(events[0].request_phase.as_deref(), Some("final_synthesis"));
    assert_eq!(
        events.last().expect("terminal event").status,
        "response_completed"
    );
    assert_eq!(
        events
            .last()
            .expect("terminal event")
            .request_phase
            .as_deref(),
        Some("final_synthesis")
    );
}

#[test]
fn provider_request_budget_node_profile_hint_does_not_force_recovery_phase() {
    let budget = ProviderRequestBudgetContext::enabled_with_attribution(
        ProviderRequestBudgetLimits {
            request_count: 1,
            max_requests: 10,
            node_request_count: 1,
            max_model_requests_per_node: 1,
            budget_state: "normal".to_string(),
        },
        ProviderRequestAttribution {
            node_id: Some("node-1".to_string()),
            request_phase: Some("model_sampling".to_string()),
            ..ProviderRequestAttribution::default()
        },
        None,
    );

    let _dispatch = budget
        .before_dispatch("responses_http")
        .expect("node profile hint should not force recovery dispatch");

    let events = budget.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "started");
    assert_eq!(events[0].request_phase.as_deref(), Some("model_sampling"));
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 2);
    assert_eq!(events[0].node_id.as_deref(), Some("node-1"));

    budget.record_response_completed(None);
    let _ = budget.drain_events();
    let dispatch = budget
        .before_dispatch("responses_http")
        .expect("node profile hint should stay advisory after repeated use");
    assert_eq!(dispatch.request_phase.as_deref(), Some("model_sampling"));
}

#[test]
fn provider_request_budget_explicit_budget_recovery_phase_remains_advisory() {
    let budget = ProviderRequestBudgetContext::enabled_with_attribution(
        ProviderRequestBudgetLimits {
            request_count: 1,
            max_requests: 10,
            node_request_count: 1,
            max_model_requests_per_node: 1,
            budget_state: "normal".to_string(),
        },
        ProviderRequestAttribution {
            node_id: Some("node-1".to_string()),
            request_phase: Some("budget_recovery".to_string()),
            ..ProviderRequestAttribution::default()
        },
        None,
    );

    let _dispatch = budget
        .before_dispatch("responses_http")
        .expect("budget_recovery phase should remain advisory");

    let events = budget.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "started");
    assert_eq!(events[0].request_phase.as_deref(), Some("budget_recovery"));
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 2);

    budget.record_response_completed(None);
    let _ = budget.drain_events();
    let dispatch = budget
        .before_dispatch("responses_http")
        .expect("budget_recovery phase should not be single-use");
    assert_eq!(dispatch.request_phase.as_deref(), Some("budget_recovery"));
}

#[test]
fn provider_request_budget_allows_rebuilt_context_after_recovery_grace_spent() {
    let budget = ProviderRequestBudgetContext::enabled_with_attribution(
        ProviderRequestBudgetLimits {
            request_count: 2,
            max_requests: 10,
            node_request_count: 2,
            max_model_requests_per_node: 2,
            budget_state: "normal".to_string(),
        },
        ProviderRequestAttribution {
            node_id: Some("node-1".to_string()),
            request_phase: Some("budget_recovery".to_string()),
            ..ProviderRequestAttribution::default()
        },
        None,
    );

    let dispatch = budget
        .before_dispatch("responses_http")
        .expect("rebuilt budget context should not hard-stop after recovery grace");
    assert_eq!(dispatch.request_phase.as_deref(), Some("budget_recovery"));
}

#[test]
fn provider_request_budget_records_started_and_terminal_status() {
    let budget = ProviderRequestBudgetContext::enabled(0, 2);

    let dispatch = budget
        .before_dispatch("responses_websocket")
        .expect("first request should be within budget");
    let payload = provider_payload_digest(&json!({
        "input": "TaskSpaceMapProjectionR8V1:\n- schema_version: taskspace-map-projection-r8-v1\n- projection_kind: bootstrap_required\n- map: none\n- bootstrap_required: true\nTaskSpaceMapProjectionR8V1 end.",
        "tools": [{
            "type": "function",
            "function": { "name": "exec_command" }
        }]
    }))
    .expect("payload digest");
    let payload_sha256 = payload.sha256.clone();
    let payload_bytes = payload.bytes;
    dispatch.record_provider_payload(payload);
    dispatch.record_status("stream_opened");

    let events = budget.drain_events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].status, "started");
    assert_eq!(events[1].status, "payload_captured");
    assert_eq!(events[2].status, "stream_opened");
    assert_eq!(events[0].request_id, events[1].request_id);
    assert_eq!(events[0].request_id, events[2].request_id);
    assert_eq!(events[0].logical_request_id, events[1].logical_request_id);
    assert_eq!(events[0].logical_request_id, events[2].logical_request_id);
    assert_eq!(events[0].attempt_seq, 1);
    assert_eq!(events[0].parent_request_id, None);
    assert!(
        events[0]
            .request_id
            .starts_with("provider-request:scope-unknown:logical-1:attempt-1")
    );
    assert_eq!(events[0].request_count_after, 1);
    assert_eq!(events[2].request_count_after, 1);
    assert_eq!(events[0].budget_state_before, "normal");
    assert_eq!(events[0].budget_state_after, "compact_checkpoint_required");
    assert_eq!(
        events[0].budget_transition_reason,
        "provider_request_compact_checkpoint_required"
    );
    assert_eq!(events[2].budget_state_after, "compact_checkpoint_required");
    assert_eq!(
        events[1].provider_payload_sha256.as_deref(),
        Some(payload_sha256.as_str())
    );
    assert_eq!(events[1].provider_payload_bytes, Some(payload_bytes));
    assert_eq!(events[1].exact_payload_scan_passed, Some(true));
    assert_eq!(events[1].active_projection_present, Some(true));
    assert_eq!(events[1].replacement_confirmed, Some(true));
    let exact_scan = events[1]
        .exact_payload_scan
        .as_ref()
        .expect("payload_captured exact scan");
    assert_eq!(exact_scan.request_id, events[1].request_id);
    assert_eq!(exact_scan.provider_payload_sha256, payload_sha256);
    assert_eq!(
        exact_scan.scan_event_id,
        format!("scan:{}:{}", events[1].request_id, payload_sha256)
    );
    assert!(exact_scan.passed);
    assert_eq!(
        events[2].provider_payload_sha256.as_deref(),
        Some(payload_sha256.as_str())
    );
    assert_eq!(events[2].provider_payload_bytes, Some(payload_bytes));

    budget.record_response_completed(Some(&TokenUsage {
        input_tokens: 100,
        cached_input_tokens: 25,
        output_tokens: 40,
        reasoning_output_tokens: 7,
        total_tokens: 140,
    }));
    let terminal_events = budget.drain_events();
    assert_eq!(terminal_events.len(), 1);
    assert_eq!(terminal_events[0].status, "response_completed");
    assert_eq!(terminal_events[0].request_id, events[0].request_id);
    assert_eq!(terminal_events[0].input_tokens, Some(100));
    assert_eq!(terminal_events[0].cached_input_tokens, Some(25));
    assert_eq!(terminal_events[0].output_tokens, Some(40));
    assert_eq!(terminal_events[0].reasoning_output_tokens, Some(7));
    assert_eq!(terminal_events[0].total_tokens, Some(140));
    assert_eq!(
        terminal_events[0].provider_payload_sha256.as_deref(),
        Some(payload_sha256.as_str())
    );
    assert_eq!(
        terminal_events[0].provider_payload_bytes,
        Some(payload_bytes)
    );
    assert_eq!(terminal_events[0].exact_payload_scan_passed, Some(true));
    assert_eq!(terminal_events[0].replacement_confirmed, Some(true));
    assert_eq!(
        terminal_events[0]
            .exact_payload_scan
            .as_ref()
            .map(|scan| scan.scan_event_id.as_str()),
        Some(exact_scan.scan_event_id.as_str())
    );
    assert!(terminal_events[0].completed_at_ms.is_some());
    assert!(terminal_events[0].latency_ms.is_some());
}

#[test]
fn provider_request_budget_confirms_projection_identity_on_final_payload() {
    let projection = "TaskSpaceMapProjectionR8V1:\n- schema_version: taskspace-map-projection-r8-v1\n- projection_kind: bootstrap_required\n- map: none\n- bootstrap_required: true\nTaskSpaceMapProjectionR8V1 end.";
    let expectation = ProviderProjectionIdentityExpectation::from_projection_context(
        TaskSpaceProjectionPolicy::MapAlways,
        projection,
    )
    .expect("bootstrap projection identity");
    let budget = ProviderRequestBudgetContext::enabled_with_attribution(
        ProviderRequestBudgetLimits {
            request_count: 0,
            max_requests: 2,
            node_request_count: 0,
            max_model_requests_per_node: usize::MAX,
            budget_state: "normal".to_string(),
        },
        ProviderRequestAttribution::default(),
        Some(expectation),
    );
    let dispatch = budget
        .before_dispatch("responses_http")
        .expect("request dispatch");
    let payload = provider_payload_digest(&json!({
        "input": projection,
        "tools": [{
            "type": "function",
            "function": { "name": "exec_command" }
        }]
    }))
    .expect("payload digest");
    dispatch.record_provider_payload(payload);

    let event = budget
        .drain_events()
        .into_iter()
        .find(|event| event.status == "payload_captured")
        .expect("payload captured event");
    let scan = event.exact_payload_scan.expect("exact payload scan");
    assert_eq!(scan.projection_identity_confirmed, Some(true));
    assert_eq!(scan.projection_kind.as_deref(), Some("bootstrap_required"));
    assert_eq!(scan.projection_policy.as_deref(), Some("map-always"));
    assert_eq!(
        scan.projection_sha256.as_deref(),
        scan.expected_projection_sha256.as_deref()
    );
    assert!(scan.passed);
    assert!(scan.replacement_confirmed);
}

#[test]
fn provider_request_budget_accepts_map_request_without_automatic_projection() {
    let payload = provider_payload_digest(&json!({
        "input": "TaskSpaceMapHandleR8V1:\n- taskspace_active: true\nTaskSpaceMapHandleR8V1 end.",
        "tools": [{
            "type": "function",
            "function": { "name": "exec_command" }
        }]
    }))
    .expect("map-request payload digest");
    let mut scan = payload.scan;
    let expectation = ProviderProjectionIdentityExpectation::without_automatic_projection(
        TaskSpaceProjectionPolicy::MapRequest,
    );
    apply_projection_identity_expectation(&mut scan, Some(&expectation));

    assert!(!scan.projection_required);
    assert_eq!(scan.active_projection_count, 0);
    assert_eq!(scan.projection_policy.as_deref(), Some("map-request"));
    assert_eq!(scan.projection_identity_confirmed, Some(true));
    assert!(scan.passed, "{:?}", scan.failure_reasons);
    assert!(scan.replacement_confirmed);
}

#[test]
fn provider_payload_scan_validates_canonical_projection_shape() {
    let standard = provider_payload_digest(&json!({
        "input": "standard request",
        "tools": [{
            "type": "function",
            "function": { "name": "shell_command" }
        }]
    }))
    .expect("standard payload digest");
    assert!(!standard.scan.projection_required);
    assert!(standard.scan.passed);

    let blank_bootstrap = provider_payload_digest(&json!({
        "input": "TaskSpaceMapProjectionR8V1:\n- schema_version: taskspace-map-projection-r8-v1\n- projection_kind: bootstrap_required\n- map: none\n- bootstrap_required: true\nTaskSpaceMapProjectionR8V1 end.",
        "tools": [{
            "type": "function",
            "function": { "name": "exec_command" }
        }]
    }))
    .expect("blank bootstrap payload digest");
    assert!(blank_bootstrap.scan.projection_required);
    assert_eq!(blank_bootstrap.scan.active_projection_count, 1);
    assert!(blank_bootstrap.scan.passed);
    assert!(blank_bootstrap.scan.replacement_confirmed);

    let payload_without_projection = provider_payload_digest(&json!({
        "input": "ordinary history",
        "tools": [
            { "type": "function", "function": { "name": "exec_command" } },
            { "type": "function", "function": { "name": "shell_command" } }
        ]
    }))
    .expect("ordinary payload digest");
    assert!(!payload_without_projection.scan.projection_required);
    assert_eq!(payload_without_projection.scan.active_projection_count, 0);
    assert!(payload_without_projection.scan.passed);
    assert!(payload_without_projection.scan.replacement_confirmed);

    let active_projection = concat!(
        "TaskSpaceMapProjectionR8V1:\n",
        "- schema_version: taskspace-map-projection-r8-v1\n",
        "- projection_kind: current_projection\n",
        "- map_id: map-1\n",
        "- revision: 2\n",
        "- canonical_sha256: canonical-map-2\n",
        "- map:\n",
        "  {\"root_node_id\":\"root\",\"finish_node_id\":\"finish\",\"complete\":false,\"nodes\":[]}\n",
        "TaskSpaceMapProjectionR8V1 end.\n",
    );
    let active_tools = json!([
        { "type": "function", "function": { "name": "exec_command" } },
        { "type": "function", "function": { "name": "shell_command" } }
    ]);
    let active = provider_payload_digest(&json!({
        "input": active_projection,
        "tools": active_tools
    }))
    .expect("active payload digest");
    assert!(
        active.scan.passed,
        "scan failure reasons: {:?}",
        active.scan.failure_reasons
    );
    assert!(active.scan.projection_required);
    assert_eq!(active.scan.active_projection_count, 1);
    assert!(active.scan.replacement_confirmed);

    let matching_expectation = ProviderProjectionIdentityExpectation::from_projection_context(
        TaskSpaceProjectionPolicy::MapAlways,
        active_projection,
    )
    .expect("matching projection identity");
    let mut matching_scan = active.scan.clone();
    apply_projection_identity_expectation(&mut matching_scan, Some(&matching_expectation));
    assert_eq!(matching_scan.projection_identity_confirmed, Some(true));
    assert!(matching_scan.passed);
    assert!(matching_scan.replacement_confirmed);

    let request_snapshot = |revision: u64| {
        active_projection
            .replace(
                "- projection_kind: current_projection",
                "- projection_kind: request_snapshot",
            )
            .replace("- revision: 2", &format!("- revision: {revision}"))
            .replace(
                "- canonical_sha256: canonical-map-2",
                &format!("- canonical_sha256: canonical-map-{revision}"),
            )
    };
    let append_revision_2 = request_snapshot(2);
    let append_revision_3 = request_snapshot(3);
    let append_payload = provider_payload_digest(&json!({
        "input": format!("{append_revision_2}\n{append_revision_3}"),
        "tools": active_tools
    }))
    .expect("append payload digest");
    let append_expectation = ProviderProjectionIdentityExpectation::from_projection_context(
        TaskSpaceProjectionPolicy::MapAppend,
        &append_revision_3,
    )
    .expect("latest append projection identity");
    let mut append_scan = append_payload.scan;
    apply_projection_identity_expectation(&mut append_scan, Some(&append_expectation));
    assert_eq!(append_scan.active_projection_count, 2);
    assert_eq!(
        append_scan.projection_kind.as_deref(),
        Some("request_snapshot")
    );
    assert_eq!(append_scan.projection_revision, Some(3));
    assert_eq!(append_scan.projection_identity_confirmed, Some(true));
    assert!(append_scan.passed, "{:?}", append_scan.failure_reasons);
    assert!(append_scan.replacement_confirmed);

    let duplicate_append = provider_payload_digest(&json!({
        "input": format!("{append_revision_2}\n{append_revision_2}"),
        "tools": active_tools
    }))
    .expect("duplicate append payload digest");
    let mut duplicate_append_scan = duplicate_append.scan;
    apply_projection_identity_expectation(
        &mut duplicate_append_scan,
        Some(
            &ProviderProjectionIdentityExpectation::from_projection_context(
                TaskSpaceProjectionPolicy::MapAppend,
                &append_revision_2,
            )
            .expect("duplicate expectation"),
        ),
    );
    assert!(
        duplicate_append_scan.passed,
        "same revision is valid when the map did not change between requests: {:?}",
        duplicate_append_scan.failure_reasons
    );

    let revision_3_projection = active_projection.replace("- revision: 2", "- revision: 3");
    let revision_3_expectation = ProviderProjectionIdentityExpectation::from_projection_context(
        TaskSpaceProjectionPolicy::MapAlways,
        &revision_3_projection,
    )
    .expect("revision 3 projection identity");
    let mut stale_scan = active.scan;
    apply_projection_identity_expectation(&mut stale_scan, Some(&revision_3_expectation));
    assert_eq!(stale_scan.projection_identity_confirmed, Some(false));
    assert!(!stale_scan.passed);
    assert!(!stale_scan.replacement_confirmed);
    assert!(
        stale_scan
            .failure_reasons
            .contains(&"projection_identity_mismatch".to_string())
    );

    let tool_output_marker = provider_payload_digest(&json!({
        "messages": [
            { "role": "tool", "content": active_projection },
            { "role": "developer", "content": active_projection }
        ],
        "tools": active_tools
    }))
    .expect("tool output marker payload digest");
    assert_eq!(tool_output_marker.scan.active_projection_count, 1);
    assert!(tool_output_marker.scan.passed);

    let projection_not_at_tail = provider_payload_digest(&json!({
        "messages": [
            { "role": "user", "content": append_revision_2 },
            { "role": "assistant", "content": "later history" }
        ],
        "tools": active_tools
    }))
    .expect("non-tail projection payload digest");
    assert!(!projection_not_at_tail.scan.passed);
    assert!(
        projection_not_at_tail
            .scan
            .failure_reasons
            .contains(&"current_projection_not_message_tail".to_string())
    );

    let duplicate_active = provider_payload_digest(&json!({
        "input": format!("{active_projection}\n{active_projection}"),
        "tools": active_tools
    }))
    .expect("duplicate active payload digest");
    assert_eq!(duplicate_active.scan.active_projection_count, 2);
    assert!(!duplicate_active.scan.passed);
    assert!(!duplicate_active.scan.replacement_confirmed);
    assert!(
        duplicate_active
            .scan
            .failure_reasons
            .contains(&"current_projection_not_unique".to_string())
    );

    let active_with_transition_notice = provider_payload_digest(&json!({
        "input": format!(
            "TaskSpace mode is now active.\n\
             hard_state: current node binding is required.\n\
             execution_contract: runtime executes Agent-declared provider calls in order.\n\
             strategy_owner: Agent.\n\
             {active_projection}"
        ),
        "tools": active_tools
    }))
    .expect("active payload with transition notice");
    assert!(active_with_transition_notice.scan.passed);

    let bundled_active_with_forbidden_strategy = provider_payload_digest(&json!({
        "input": format!("{active_projection}\nTaskSpaceAgentContextBundleV1\nnext_valid_actions"),
        "tools": active_tools
    }))
    .expect("bundled active payload digest");
    assert!(!bundled_active_with_forbidden_strategy.scan.passed);
    assert!(
        bundled_active_with_forbidden_strategy
            .scan
            .protected_items_present
    );
    assert_eq!(
        bundled_active_with_forbidden_strategy
            .scan
            .runtime_boundary_forbidden_markers,
        vec![
            "TaskSpaceAgentContextBundleV1".to_string(),
            "next_valid_actions".to_string(),
        ]
    );
    assert!(
        bundled_active_with_forbidden_strategy
            .scan
            .failure_reasons
            .contains(&"runtime_boundary_forbidden_marker_present".to_string())
    );

    let missing_protected = provider_payload_digest(&json!({
        "input": "TaskSpaceMapProjectionR8V1:\n- map_id: map-1\n- summary: incomplete\nTaskSpaceMapProjectionR8V1 end.",
        "tools": active_tools
    }))
    .expect("missing protected payload digest");
    assert!(missing_protected.scan.active_projection_present);
    assert!(!missing_protected.scan.protected_items_present);
    assert!(!missing_protected.scan.passed);
    assert_eq!(
        missing_protected.scan.failure_reasons,
        vec!["current_projection_required_sections_missing".to_string()]
    );

    let large_instruction_text = "x".repeat(60 * 1024);
    let large_active_instructions = provider_payload_digest(&json!({
        "input": format!("{large_instruction_text}\n{active_projection}"),
        "tools": active_tools
    }))
    .expect("large active instruction payload digest");
    assert_eq!(large_active_instructions.scan.large_raw_output_tokens, 0);
    assert!(large_active_instructions.scan.passed);

    let raw_output = "x".repeat(60 * 1024);
    let large_raw = provider_payload_digest(&json!({
        "input": [
            {
                "type": "function_call_output",
                "call_id": "call-1",
                "output": raw_output
            },
            {
                "type": "message",
                "role": "developer",
                "content": active_projection
            }
        ],
        "tools": active_tools
    }))
    .expect("large raw payload digest");
    assert!(large_raw.scan.large_raw_output_tokens > 0);
    assert!(!large_raw.scan.passed);

    let output_ref = provider_payload_digest(&json!({
        "input": [
            {
                "type": "function_call_output",
                "call_id": "call-1",
                "output": format!("OutputReferenceV1:\nraw_output_elided: true\n{raw_output}")
            },
            {
                "type": "message",
                "role": "developer",
                "content": active_projection
            }
        ],
        "tools": active_tools
    }))
    .expect("output ref payload digest");
    assert_eq!(output_ref.scan.large_raw_output_tokens, 0);
    assert!(output_ref.scan.passed);
}

#[test]
fn build_subagent_headers_sets_other_subagent_label() {
    let client = test_model_client(SessionSource::SubAgent(SubAgentSource::Other(
        "memory_consolidation".to_string(),
    )));
    let headers = client.build_subagent_headers();
    let value = headers
        .get(X_OPENAI_SUBAGENT_HEADER)
        .and_then(|value| value.to_str().ok());
    assert_eq!(value, Some("memory_consolidation"));
}

#[test]
fn build_ws_client_metadata_includes_window_lineage_and_turn_metadata() {
    let parent_thread_id = ThreadId::new();
    let client = test_model_client(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth: 2,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
    }));

    client.advance_window_generation();

    let client_metadata = client.build_ws_client_metadata(Some(r#"{"turn_id":"turn-123"}"#));
    let conversation_id = client.state.conversation_id;
    assert_eq!(
        client_metadata,
        std::collections::HashMap::from([
            (
                X_CODEX_INSTALLATION_ID_HEADER.to_string(),
                "11111111-1111-4111-8111-111111111111".to_string(),
            ),
            (
                X_CODEX_WINDOW_ID_HEADER.to_string(),
                format!("{conversation_id}:1"),
            ),
            (
                X_OPENAI_SUBAGENT_HEADER.to_string(),
                "collab_spawn".to_string(),
            ),
            (
                X_CODEX_PARENT_THREAD_ID_HEADER.to_string(),
                parent_thread_id.to_string(),
            ),
            (
                X_CODEX_TURN_METADATA_HEADER.to_string(),
                r#"{"turn_id":"turn-123"}"#.to_string(),
            ),
        ])
    );
}

#[tokio::test]
async fn summarize_memories_returns_empty_for_empty_input() {
    let client = test_model_client(SessionSource::Cli);
    let model_info = test_model_info();
    let session_telemetry = test_session_telemetry();

    let output = client
        .summarize_memories(
            Vec::new(),
            &model_info,
            /*effort*/ None,
            &session_telemetry,
        )
        .await
        .expect("empty summarize request should succeed");
    assert_eq!(output.len(), 0);
}

#[test]
fn auth_request_telemetry_context_tracks_attached_auth_and_retry_phase() {
    let auth_context = AuthRequestTelemetryContext::new(
        Some(AuthMode::Chatgpt),
        &BearerAuthProvider::for_test(Some("access-token"), Some("workspace-123")),
        PendingUnauthorizedRetry::from_recovery(UnauthorizedRecoveryExecution {
            mode: "managed",
            phase: "refresh_token",
        }),
    );

    assert_eq!(auth_context.auth_mode, Some("Chatgpt"));
    assert!(auth_context.auth_header_attached);
    assert_eq!(auth_context.auth_header_name, Some("authorization"));
    assert!(auth_context.retry_after_unauthorized);
    assert_eq!(auth_context.recovery_mode, Some("managed"));
    assert_eq!(auth_context.recovery_phase, Some("refresh_token"));
}
