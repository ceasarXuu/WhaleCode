use super::AuthRequestTelemetryContext;
use super::ModelClient;
use super::PendingUnauthorizedRetry;
use super::ProviderRequestAttribution;
use super::ProviderRequestBudgetContext;
use super::ProviderRequestBudgetLimits;
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
        "input": "ContextProjectionV1 epoch snapshot:\n- protected"
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
fn provider_payload_scan_validates_canonical_projection_shape() {
    let blank_bootstrap = provider_payload_digest(&json!({
        "input": "user task",
        "tools": [{
            "type": "function",
            "function": { "name": "taskspace_control" }
        }]
    }))
    .expect("blank bootstrap payload digest");
    assert!(!blank_bootstrap.scan.projection_required);
    assert_eq!(blank_bootstrap.scan.active_projection_count, 0);
    assert!(blank_bootstrap.scan.passed);
    assert!(blank_bootstrap.scan.replacement_confirmed);

    let fresh_active_without_projection = provider_payload_digest(&json!({
        "input": "canonical initialize/control history",
        "tools": [
            { "type": "function", "function": { "name": "taskspace_control" } },
            { "type": "function", "function": { "name": "shell_command" } }
        ]
    }))
    .expect("fresh active payload digest");
    assert!(fresh_active_without_projection.scan.projection_required);
    assert_eq!(
        fresh_active_without_projection.scan.active_projection_count,
        0
    );
    assert!(fresh_active_without_projection.scan.passed);
    assert!(fresh_active_without_projection.scan.replacement_confirmed);

    let active_projection = concat!(
        "ContextProjectionV1 epoch snapshot:\n",
        "- task_id: task-1\n",
        "- map_id: map-1\n",
        "- root_source_event_ids:\n",
        "  - task-event-1\n",
        "- sections:\n",
        "  success_criteria:\n",
        "    - criterion\n",
        "  current_node: node-1\n",
        "  active_frontier:\n",
        "    - node-1\n",
        "  map_nodes:\n",
        "    - node-1 kind=inspect_code_context status=running\n",
        "  map_edges:\n",
        "    - none\n",
        "  blockers:\n",
        "    - none\n",
        "  decisions:\n",
        "    - none\n",
        "  facts:\n",
        "    - fact\n",
        "  relevant_results:\n",
        "    - none\n",
        "  verified_input_evidence:\n",
        "    - none\n",
        "  fact_source_coverage:\n",
        "    - none\n",
        "  node_details:\n",
        "    - none\n",
    );
    let active = provider_payload_digest(&json!({
        "input": active_projection
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

    let duplicate_active = provider_payload_digest(&json!({
        "input": format!("{active_projection}\n{active_projection}")
    }))
    .expect("duplicate active payload digest");
    assert_eq!(duplicate_active.scan.active_projection_count, 2);
    assert!(!duplicate_active.scan.passed);
    assert!(!duplicate_active.scan.replacement_confirmed);
    assert!(
        duplicate_active
            .scan
            .failure_reasons
            .contains(&"active_projection_not_unique".to_string())
    );

    let active_with_transition_notice = provider_payload_digest(&json!({
        "input": format!(
            "TaskSpace mode is now active.\n\
             hard_state: current node binding is required.\n\
             execution_contract: runtime executes Agent-declared provider calls in order.\n\
             strategy_owner: Agent.\n\
             {active_projection}"
        )
    }))
    .expect("active payload with transition notice");
    assert!(active_with_transition_notice.scan.passed);

    let bundled_active_with_forbidden_strategy = provider_payload_digest(&json!({
        "input": format!("{active_projection}\nTaskSpaceAgentContextBundleV1\nnext_valid_actions")
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
        "input": "ContextProjectionV1 epoch snapshot:\n- summary only"
    }))
    .expect("missing protected payload digest");
    assert!(missing_protected.scan.active_projection_present);
    assert!(!missing_protected.scan.protected_items_present);
    assert!(!missing_protected.scan.passed);
    assert_eq!(
        missing_protected.scan.failure_reasons,
        vec!["projection_required_sections_missing".to_string()]
    );

    let large_instruction_text = "x".repeat(60 * 1024);
    let large_active_instructions = provider_payload_digest(&json!({
        "input": format!("{active_projection}\n{large_instruction_text}")
    }))
    .expect("large active instruction payload digest");
    assert_eq!(large_active_instructions.scan.large_raw_output_tokens, 0);
    assert!(large_active_instructions.scan.passed);

    let raw_output = "x".repeat(60 * 1024);
    let large_raw = provider_payload_digest(&json!({
        "input": [
            {
                "type": "message",
                "role": "developer",
                "content": active_projection
            },
            {
                "type": "function_call_output",
                "call_id": "call-1",
                "output": raw_output
            }
        ]
    }))
    .expect("large raw payload digest");
    assert!(large_raw.scan.large_raw_output_tokens > 0);
    assert!(!large_raw.scan.passed);

    let output_ref = provider_payload_digest(&json!({
        "input": [
            {
                "type": "message",
                "role": "developer",
                "content": "ContextProjectionV1 epoch snapshot:\n- protected"
            },
            {
                "type": "function_call_output",
                "call_id": "call-1",
                "output": format!("OutputReferenceV1:\nraw_output_elided: true\n{raw_output}")
            }
        ]
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
