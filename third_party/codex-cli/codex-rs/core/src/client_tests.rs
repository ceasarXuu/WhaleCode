use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::ProviderRequestAttribution;
use super::ProviderRequestBudgetContext;
use super::UnauthorizedRecoveryExecution;
use super::X_CODEX_INSTALLATION_ID_HEADER;
use super::X_CODEX_PARENT_THREAD_ID_HEADER;
use super::X_CODEX_TURN_METADATA_HEADER;
use super::X_CODEX_WINDOW_ID_HEADER;
use super::X_OPENAI_SUBAGENT_HEADER;
use super::provider_payload_digest;
use codex_app_server_protocol::AuthMode;
use codex_model_provider::BearerAuthProvider;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_otel::SessionTelemetry;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
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
fn provider_request_budget_blocks_before_dispatch_when_exhausted() {
    let budget = ProviderRequestBudgetContext::enabled(1, 1);

    let err = budget
        .before_dispatch("responses_http")
        .expect_err("exhausted budget should block before dispatch");

    assert!(
        err.to_string()
            .contains("active provider request budget is exhausted")
    );
    let events = budget.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].request_id,
        "provider-request:scope-unknown:logical-2:attempt-1"
    );
    assert_eq!(
        events[0].logical_request_id,
        "provider-request:scope-unknown:logical-2"
    );
    assert_eq!(events[0].attempt_seq, 1);
    assert_eq!(events[0].transport, "responses_http");
    assert_eq!(events[0].status, "blocked");
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 1);
    assert_eq!(events[0].max_requests, 1);
    assert_eq!(events[0].budget_state_before, "hard_stopped");
    assert_eq!(events[0].budget_state_after, "hard_stopped");
    assert_eq!(
        events[0].budget_transition_reason,
        "provider_request_budget_exhausted"
    );
}

#[test]
fn provider_request_budget_blocks_regular_dispatch_at_compact_checkpoint() {
    let budget = ProviderRequestBudgetContext::enabled(1, 2);

    let err = budget
        .before_dispatch("responses_http")
        .expect_err("compact checkpoint state should block regular dispatch");

    assert!(
        err.to_string()
            .contains("requires a compact checkpoint or final synthesis response")
    );
    let events = budget.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "blocked");
    assert_eq!(events[0].request_count_before, 1);
    assert_eq!(events[0].request_count_after, 1);
    assert_eq!(events[0].max_requests, 2);
    assert_eq!(events[0].budget_state_before, "compact_checkpoint_required");
    assert_eq!(events[0].budget_state_after, "compact_checkpoint_required");
    assert_eq!(
        events[0].budget_transition_reason,
        "provider_request_compact_checkpoint_required"
    );
}

#[test]
fn provider_request_budget_allows_final_synthesis_at_compact_checkpoint() {
    let budget = ProviderRequestBudgetContext::enabled_with_attribution(
        1,
        2,
        ProviderRequestAttribution {
            request_phase: Some("final_synthesis".to_string()),
            ..ProviderRequestAttribution::default()
        },
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
    assert_eq!(events[0].budget_state_after, "hard_stopped");
    assert_eq!(
        events.last().expect("terminal event").status,
        "response_completed"
    );
}

#[test]
fn provider_request_budget_records_started_and_terminal_status() {
    let budget = ProviderRequestBudgetContext::enabled(0, 2);

    let dispatch = budget
        .before_dispatch("responses_websocket")
        .expect("first request should be within budget");
    let payload = provider_payload_digest(&json!({
        "input": "ContextProjectionV1 active replacement:\n- protected"
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
    assert_eq!(events[1].legacy_taskspace_history_present, Some(false));
    assert_eq!(events[1].replacement_confirmed, Some(true));
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
    assert!(terminal_events[0].completed_at_ms.is_some());
    assert!(terminal_events[0].latency_ms.is_some());
}

#[test]
fn provider_payload_scan_rejects_shadow_or_legacy_taskspace_history() {
    let active = provider_payload_digest(&json!({
        "input": "ContextProjectionV1 active replacement:\n- protected"
    }))
    .expect("active payload digest");
    assert!(active.scan.exact_payload_scan_passed);
    assert!(active.scan.replacement_confirmed);

    let legacy = provider_payload_digest(&json!({
        "input": "ContextProjectionV1 active replacement:\n- protected\nContextProjectionV1 shadow (not active replacement):\ntaskspace_control"
    }))
    .expect("legacy payload digest");
    assert!(legacy.scan.active_projection_present);
    assert!(legacy.scan.legacy_taskspace_history_present);
    assert!(!legacy.scan.exact_payload_scan_passed);
    assert!(!legacy.scan.replacement_confirmed);

    let missing_protected = provider_payload_digest(&json!({
        "input": "ContextProjectionV1 active replacement:\n- summary only"
    }))
    .expect("missing protected payload digest");
    assert!(missing_protected.scan.active_projection_present);
    assert!(!missing_protected.scan.protected_items_present);
    assert!(!missing_protected.scan.exact_payload_scan_passed);

    let raw_output = "x".repeat(60 * 1024);
    let large_raw = provider_payload_digest(&json!({
        "input": format!("ContextProjectionV1 active replacement:\n- protected\n{raw_output}")
    }))
    .expect("large raw payload digest");
    assert!(large_raw.scan.large_raw_output_tokens > 0);
    assert!(!large_raw.scan.exact_payload_scan_passed);

    let output_ref = provider_payload_digest(&json!({
        "input": format!("ContextProjectionV1 active replacement:\n- protected\nOutputReferenceV1:\nraw_output_elided: true\n{raw_output}")
    }))
    .expect("output ref payload digest");
    assert_eq!(output_ref.scan.large_raw_output_tokens, 0);
    assert!(output_ref.scan.exact_payload_scan_passed);
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
