//! Session- and turn-scoped helpers for talking to model provider APIs.
//!
//! `ModelClient` is intended to live for the lifetime of a Codex session and holds the stable
//! configuration and state needed to talk to a provider (auth, provider selection, conversation id,
//! and transport fallback state).
//!
//! Per-turn settings (model selection, reasoning controls, telemetry context, and turn metadata)
//! are passed explicitly to streaming and unary methods so that the turn lifetime is visible at the
//! call site.
//!
//! A [`ModelClientSession`] is created per turn and is used to stream one or more Responses API
//! requests during that turn. It caches a Responses WebSocket connection (opened lazily) and stores
//! per-turn state such as the `x-codex-turn-state` token used for sticky routing.
//!
//! WebSocket prewarm is a v2-only `response.create` with `generate=false`; it waits for completion
//! so the next request can reuse the same connection and `previous_response_id`.
//!
//! Turn execution performs prewarm as a best-effort step before the first stream request so the
//! subsequent request can reuse the same connection.
//!
//! ## Retry-Budget Tradeoff
//!
//! WebSocket prewarm is treated as the first websocket connection attempt for a turn. If it
//! fails, normal stream retry/fallback logic handles recovery on the same turn.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::MutexGuard as StdMutexGuard;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::action_map::ActionMapProviderRequestBudgetSnapshot;
use codex_api::ApiError;
use codex_api::AuthProvider;
use codex_api::CompactClient as ApiCompactClient;
use codex_api::CompactionInput as ApiCompactionInput;
use codex_api::Compression;
use codex_api::MemoriesClient as ApiMemoriesClient;
use codex_api::MemorySummarizeInput as ApiMemorySummarizeInput;
use codex_api::MemorySummarizeOutput as ApiMemorySummarizeOutput;
use codex_api::Provider as ApiProvider;
use codex_api::RawMemory as ApiRawMemory;
use codex_api::RealtimeCallClient as ApiRealtimeCallClient;
use codex_api::RealtimeSessionConfig as ApiRealtimeSessionConfig;
use codex_api::Reasoning;
use codex_api::RequestTelemetry;
use codex_api::ReqwestTransport;
use codex_api::ResponseCreateWsRequest;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesClient as ApiResponsesClient;
use codex_api::ResponsesOptions as ApiResponsesOptions;
use codex_api::ResponsesWebsocketClient as ApiWebSocketResponsesClient;
use codex_api::ResponsesWebsocketConnection as ApiWebSocketConnection;
use codex_api::ResponsesWsRequest;
use codex_api::SharedAuthProvider;
use codex_api::SseTelemetry;
use codex_api::TransportError;
use codex_api::WebsocketTelemetry;
use codex_api::auth_header_telemetry;
use codex_api::build_conversation_headers;
use codex_api::create_text_param_for_request;
use codex_api::response_create_client_metadata;
use codex_app_server_protocol::AuthMode;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::RefreshTokenError;
use codex_login::UnauthorizedRecovery;
use codex_login::default_client::build_reqwest_client;
use codex_otel::SessionTelemetry;
use codex_otel::current_span_w3c_trace_context;

use codex_protocol::ThreadId;
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::config_types::Verbosity as VerbosityConfig;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::W3cTraceContext;
use codex_rollout_trace::CompactionTraceContext;
use codex_rollout_trace::InferenceTraceAttempt;
use codex_rollout_trace::InferenceTraceContext;
use codex_tools::create_tools_json_for_responses_api;
use eventsource_stream::Event;
use eventsource_stream::EventStreamError;
use futures::StreamExt;
use http::HeaderMap as ApiHeaderMap;
use http::HeaderValue;
use http::StatusCode as HttpStatusCode;
use reqwest::StatusCode;
use sha2::Digest;
use sha2::Sha256;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::Message;
use tracing::instrument;
use tracing::trace;
use tracing::warn;

use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::client_common::ResponseStream;
use crate::flags::CODEX_RS_SSE_FIXTURE;
use crate::provider_wire_trace::ProviderWireTrace;
use crate::util::emit_feedback_auth_recovery_tags;
use codex_api::map_api_error;
use codex_feedback::FeedbackRequestTags;
use codex_feedback::emit_feedback_request_tags_with_auth_env;
use codex_login::auth_env_telemetry::AuthEnvTelemetry;
use codex_login::auth_env_telemetry::collect_auth_env_telemetry;
use codex_model_provider::SharedModelProvider;
use codex_model_provider::create_model_provider;
#[cfg(test)]
use codex_model_provider_info::DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result;
use codex_response_debug_context::extract_response_debug_context;
use codex_response_debug_context::extract_response_debug_context_from_api_error;
use codex_response_debug_context::telemetry_api_error_message;
use codex_response_debug_context::telemetry_transport_error_message;

pub const OPENAI_BETA_HEADER: &str = "OpenAI-Beta";
pub const X_CODEX_INSTALLATION_ID_HEADER: &str = "x-codex-installation-id";
pub const X_CODEX_TURN_STATE_HEADER: &str = "x-codex-turn-state";
pub const X_CODEX_TURN_METADATA_HEADER: &str = "x-codex-turn-metadata";
pub const X_CODEX_PARENT_THREAD_ID_HEADER: &str = "x-codex-parent-thread-id";
pub const X_CODEX_WINDOW_ID_HEADER: &str = "x-codex-window-id";
pub const X_OPENAI_MEMGEN_REQUEST_HEADER: &str = "x-openai-memgen-request";
pub const X_OPENAI_SUBAGENT_HEADER: &str = "x-openai-subagent";
pub const X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER: &str =
    "x-responsesapi-include-timing-metrics";
const RESPONSES_WEBSOCKETS_V2_BETA_HEADER_VALUE: &str = "responses_websockets=2026-02-06";
const RESPONSES_ENDPOINT: &str = "/responses";
const RESPONSES_COMPACT_ENDPOINT: &str = "/responses/compact";
const TASKSPACE_PROJECTION_MARKER: &str = "TaskSpaceMapProjectionR7V1:";
const TASKSPACE_PROJECTION_END_MARKER: &str = "TaskSpaceMapProjectionR7V1 end.";
const TASKSPACE_PROJECTION_REQUIRED_SECTIONS: &[&str] = &[
    "schema_version",
    "projection_kind",
    "map_id",
    "revision",
    "canonical_sha256",
    "root_node_id",
    "finish_node_id",
    "complete",
    "current_terminal",
    "terminal_history",
    "root_source_event_ids",
    "active_frontier",
    "map_nodes",
    "map_edges",
    "node_details",
];
// `/responses/compact` is unary, so the timeout covers the full response rather than one idle
// period between stream events.
const COMPACT_REQUEST_TIMEOUT_IDLE_MULTIPLIER: u32 = 4;
const MEMORIES_SUMMARIZE_ENDPOINT: &str = "/memories/trace_summarize";
const PROVIDER_REQUEST_HARD_LIMIT_ENV: &str = "WHALE_PROVIDER_REQUEST_HARD_LIMIT";
#[cfg(test)]
pub(crate) const WEBSOCKET_CONNECT_TIMEOUT: Duration =
    Duration::from_millis(DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ExactPayloadScanEventV1 {
    pub(crate) schema_version: &'static str,
    pub(crate) scan_event_id: String,
    pub(crate) request_id: String,
    pub(crate) provider_payload_sha256: String,
    pub(crate) scanner_version: String,
    pub(crate) matcher_version: String,
    pub(crate) checked_byte_ranges: Vec<(usize, usize)>,
    pub(crate) negative_checks_performed: Vec<String>,
    pub(crate) projection_required: bool,
    pub(crate) active_projection_present: bool,
    pub(crate) active_projection_count: usize,
    pub(crate) projection_is_message_tail: bool,
    pub(crate) large_raw_output_tokens: usize,
    pub(crate) runtime_boundary_forbidden_markers: Vec<String>,
    pub(crate) protected_items_present: bool,
    pub(crate) projection_kind: Option<String>,
    pub(crate) projection_map_id_sha256: Option<String>,
    pub(crate) projection_revision: Option<u64>,
    pub(crate) projection_canonical_sha256: Option<String>,
    pub(crate) projection_sha256: Option<String>,
    pub(crate) projection_policy: Option<String>,
    pub(crate) expected_projection_kind: Option<String>,
    pub(crate) expected_projection_map_id_sha256: Option<String>,
    pub(crate) expected_projection_revision: Option<u64>,
    pub(crate) expected_projection_canonical_sha256: Option<String>,
    pub(crate) expected_projection_sha256: Option<String>,
    pub(crate) projection_identity_confirmed: Option<bool>,
    pub(crate) replacement_confirmed: bool,
    pub(crate) passed: bool,
    pub(crate) failure_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderProjectionIdentityExpectation {
    policy: TaskSpaceProjectionPolicy,
    automatic_projection_required: bool,
    kind: String,
    map_id_sha256: Option<String>,
    revision: Option<u64>,
    canonical_sha256: Option<String>,
    projection_sha256: String,
}

impl ProviderProjectionIdentityExpectation {
    pub(crate) fn from_projection_context(
        policy: TaskSpaceProjectionPolicy,
        context: &str,
    ) -> Option<Self> {
        let projection = projection_blocks_from_text(context).into_iter().next()?;
        let projection_sha256 = sha256_hex(projection.as_bytes());
        let bootstrap = projection.lines().any(|line| line == "- map: none")
            && projection
                .lines()
                .any(|line| line == "- bootstrap_required: true");
        if bootstrap {
            return Some(Self {
                policy,
                automatic_projection_required: true,
                kind: "bootstrap_required".to_string(),
                map_id_sha256: None,
                revision: None,
                canonical_sha256: None,
                projection_sha256,
            });
        }
        let map_id = projection_mechanical_field(projection, "map_id")?;
        let revision = projection_mechanical_field(projection, "revision")
            .and_then(|value| value.parse::<u64>().ok())?;
        let canonical_sha256 = projection_mechanical_field(projection, "canonical_sha256")?;
        let kind = projection_mechanical_field(projection, "projection_kind")?;
        Some(Self {
            policy,
            automatic_projection_required: true,
            kind: kind.to_string(),
            map_id_sha256: Some(sha256_hex(map_id.as_bytes())),
            revision: Some(revision),
            canonical_sha256: Some(canonical_sha256.to_string()),
            projection_sha256,
        })
    }

    pub(crate) fn without_automatic_projection(policy: TaskSpaceProjectionPolicy) -> Self {
        Self {
            policy,
            automatic_projection_required: false,
            kind: "none".to_string(),
            map_id_sha256: None,
            revision: None,
            canonical_sha256: None,
            projection_sha256: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderRequestBudgetEvent {
    pub(crate) request_id: String,
    pub(crate) logical_request_id: String,
    pub(crate) parent_request_id: Option<String>,
    pub(crate) attempt_seq: usize,
    pub(crate) transport: String,
    pub(crate) status: String,
    pub(crate) request_count_before: usize,
    pub(crate) request_count_after: usize,
    pub(crate) max_requests: usize,
    pub(crate) budget_state_before: String,
    pub(crate) budget_state_after: String,
    pub(crate) budget_transition_reason: String,
    pub(crate) started_at_ms: i64,
    pub(crate) completed_at_ms: Option<i64>,
    pub(crate) latency_ms: Option<i64>,
    pub(crate) input_tokens: Option<i64>,
    pub(crate) cached_input_tokens: Option<i64>,
    pub(crate) output_tokens: Option<i64>,
    pub(crate) reasoning_output_tokens: Option<i64>,
    pub(crate) total_tokens: Option<i64>,
    pub(crate) provider_payload_sha256: Option<String>,
    pub(crate) provider_payload_bytes: Option<usize>,
    pub(crate) provider_wire_api: Option<String>,
    pub(crate) tools_count: Option<usize>,
    pub(crate) tools_present: Option<bool>,
    pub(crate) request_shape_classifier: Option<String>,
    pub(crate) messages_hash: Option<String>,
    pub(crate) stable_prefix_hash: Option<String>,
    pub(crate) dynamic_suffix_hash: Option<String>,
    pub(crate) exact_payload_scan_passed: Option<bool>,
    pub(crate) active_projection_present: Option<bool>,
    pub(crate) active_projection_count: Option<usize>,
    pub(crate) large_raw_output_tokens: Option<usize>,
    pub(crate) protected_items_present: Option<bool>,
    pub(crate) replacement_confirmed: Option<bool>,
    pub(crate) exact_payload_scan: Option<ExactPayloadScanEventV1>,
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) request_phase: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderRequestBudgetContext {
    state: Arc<ProviderRequestBudgetState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderRequestBudgetLimits {
    pub(crate) request_count: usize,
    pub(crate) max_requests: usize,
    pub(crate) node_request_count: usize,
    pub(crate) max_model_requests_per_node: usize,
    pub(crate) budget_state: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderRequestAttribution {
    pub(crate) request_scope_id: Option<String>,
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) request_phase: Option<String>,
}

impl ProviderRequestAttribution {
    pub(crate) fn from_snapshot(
        snapshot: &ActionMapProviderRequestBudgetSnapshot,
        request_scope_id: &str,
    ) -> Self {
        Self {
            request_scope_id: Some(request_scope_id.to_string()),
            task_id: snapshot
                .task_id
                .as_ref()
                .map(std::string::ToString::to_string),
            map_id: Some(snapshot.map_id.to_string()),
            node_id: snapshot
                .node_id
                .as_ref()
                .map(std::string::ToString::to_string),
            request_phase: snapshot.request_phase.clone(),
        }
    }
}

#[derive(Debug)]
struct ProviderRequestBudgetState {
    enabled: bool,
    count: AtomicUsize,
    max_requests: usize,
    node_count: AtomicUsize,
    budget_state: String,
    attribution: ProviderRequestAttribution,
    expected_projection_identity: Option<ProviderProjectionIdentityExpectation>,
    events: StdMutex<Vec<ProviderRequestBudgetEvent>>,
    active_request: StdMutex<Option<ProviderRequestBudgetActiveRequest>>,
}

#[derive(Debug, Clone)]
struct ProviderRequestBudgetActiveRequest {
    request_id: String,
    logical_request_id: String,
    parent_request_id: Option<String>,
    attempt_seq: usize,
    transport: String,
    request_count_before: usize,
    request_count_after: usize,
    budget_state_before: String,
    budget_state_after: String,
    budget_transition_reason: String,
    started_at_ms: i64,
    request_phase: Option<String>,
    provider_payload_sha256: Option<String>,
    provider_payload_bytes: Option<usize>,
    provider_wire_api: Option<String>,
    tools_count: Option<usize>,
    tools_present: Option<bool>,
    request_shape_classifier: Option<String>,
    messages_hash: Option<String>,
    stable_prefix_hash: Option<String>,
    dynamic_suffix_hash: Option<String>,
    payload_scan: Option<ExactPayloadScanEventV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderPayloadDigest {
    sha256: String,
    bytes: usize,
    provider_wire_api: String,
    tools_count: usize,
    tools_present: bool,
    request_shape_classifier: String,
    messages_hash: String,
    stable_prefix_hash: String,
    dynamic_suffix_hash: String,
    scan: ExactPayloadScanEventV1,
}

struct ProviderRequestIdentity {
    request_id: String,
    logical_request_id: String,
    attempt_seq: usize,
}

fn provider_request_budget_state_for(count: usize, max_requests: usize) -> &'static str {
    if max_requests == 0 || count >= max_requests {
        return "over_profile_hint";
    }
    let remaining = max_requests.saturating_sub(count);
    if remaining <= 1 {
        return "compact_checkpoint_required";
    }
    if count.saturating_mul(4) >= max_requests.saturating_mul(3) {
        return "thin_downgraded";
    }
    if count.saturating_mul(2) >= max_requests {
        return "warned";
    }
    "normal"
}

fn provider_request_budget_transition_reason(before: &str, after: &str) -> &'static str {
    if before == after {
        return "request_dispatched_without_state_change";
    }
    match after {
        "warned" => "provider_request_warning_threshold_reached",
        "thin_downgraded" => "provider_request_thin_threshold_reached",
        "compact_checkpoint_required" => "provider_request_compact_checkpoint_required",
        "over_profile_hint" => "provider_request_profile_hint_exceeded",
        _ => "provider_request_state_transition",
    }
}

impl ProviderRequestBudgetContext {
    pub(crate) fn disabled() -> Self {
        Self {
            state: Arc::new(ProviderRequestBudgetState {
                enabled: false,
                count: AtomicUsize::new(0),
                max_requests: usize::MAX,
                node_count: AtomicUsize::new(0),
                budget_state: "disabled".to_string(),
                attribution: ProviderRequestAttribution::default(),
                expected_projection_identity: None,
                events: StdMutex::new(Vec::new()),
                active_request: StdMutex::new(None),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn enabled(starting_count: usize, max_requests: usize) -> Self {
        Self::enabled_with_attribution(
            ProviderRequestBudgetLimits {
                request_count: starting_count,
                max_requests,
                node_request_count: 0,
                max_model_requests_per_node: usize::MAX,
                budget_state: provider_request_budget_state_for(starting_count, max_requests)
                    .to_string(),
            },
            ProviderRequestAttribution::default(),
            None,
        )
    }

    pub(crate) fn enabled_with_attribution(
        limits: ProviderRequestBudgetLimits,
        attribution: ProviderRequestAttribution,
        expected_projection_identity: Option<ProviderProjectionIdentityExpectation>,
    ) -> Self {
        Self {
            state: Arc::new(ProviderRequestBudgetState {
                enabled: true,
                count: AtomicUsize::new(limits.request_count),
                max_requests: limits.max_requests,
                node_count: AtomicUsize::new(limits.node_request_count),
                budget_state: limits.budget_state,
                attribution,
                expected_projection_identity,
                events: StdMutex::new(Vec::new()),
                active_request: StdMutex::new(None),
            }),
        }
    }

    fn before_dispatch(&self, transport: &str) -> Result<ProviderRequestBudgetDispatch> {
        if !self.state.enabled {
            return Ok(ProviderRequestBudgetDispatch::disabled());
        }
        let before = self.state.count.load(Ordering::SeqCst);
        let budget_state_before = if self.state.budget_state.trim().is_empty() {
            provider_request_budget_state_for(before, self.state.max_requests).to_string()
        } else {
            self.state.budget_state.clone()
        };
        let after = self.state.count.fetch_add(1, Ordering::SeqCst) + 1;
        let _node_after = self.state.node_count.fetch_add(1, Ordering::SeqCst) + 1;
        let budget_state_after = provider_request_budget_state_for(after, self.state.max_requests);
        let budget_transition_reason =
            provider_request_budget_transition_reason(&budget_state_before, budget_state_after);
        let request_phase = self.state.attribution.request_phase.clone();
        let request_identity = self.build_request_identity(after, 1);
        let request_id = request_identity.request_id.clone();
        let started_at_ms = provider_request_budget_now_ms();
        let active_request = ProviderRequestBudgetActiveRequest {
            request_id: request_id.clone(),
            logical_request_id: request_identity.logical_request_id.clone(),
            parent_request_id: None,
            attempt_seq: request_identity.attempt_seq,
            transport: transport.to_string(),
            request_count_before: before,
            request_count_after: after,
            budget_state_before: budget_state_before.clone(),
            budget_state_after: budget_state_after.to_string(),
            budget_transition_reason: budget_transition_reason.to_string(),
            started_at_ms,
            request_phase: request_phase.clone(),
            provider_payload_sha256: None,
            provider_payload_bytes: None,
            provider_wire_api: None,
            tools_count: None,
            tools_present: None,
            request_shape_classifier: None,
            messages_hash: None,
            stable_prefix_hash: None,
            dynamic_suffix_hash: None,
            payload_scan: None,
        };
        *lock_std_mutex(&self.state.active_request) = Some(active_request);
        self.push_event(ProviderRequestBudgetEvent {
            request_id: request_id.clone(),
            logical_request_id: request_identity.logical_request_id.clone(),
            parent_request_id: None,
            attempt_seq: request_identity.attempt_seq,
            transport: transport.to_string(),
            status: "started".to_string(),
            request_count_before: before,
            request_count_after: after,
            max_requests: self.state.max_requests,
            budget_state_before: budget_state_before.clone(),
            budget_state_after: budget_state_after.to_string(),
            budget_transition_reason: budget_transition_reason.to_string(),
            started_at_ms,
            completed_at_ms: None,
            latency_ms: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_output_tokens: None,
            total_tokens: None,
            provider_payload_sha256: None,
            provider_payload_bytes: None,
            provider_wire_api: None,
            tools_count: None,
            tools_present: None,
            request_shape_classifier: None,
            messages_hash: None,
            stable_prefix_hash: None,
            dynamic_suffix_hash: None,
            exact_payload_scan_passed: None,
            active_projection_present: None,
            active_projection_count: None,
            large_raw_output_tokens: None,
            protected_items_present: None,
            replacement_confirmed: None,
            exact_payload_scan: None,
            task_id: self.state.attribution.task_id.clone(),
            map_id: self.state.attribution.map_id.clone(),
            node_id: self.state.attribution.node_id.clone(),
            request_phase: request_phase.clone(),
        });
        Ok(ProviderRequestBudgetDispatch {
            context: Some(self.clone()),
            request_id,
            logical_request_id: request_identity.logical_request_id,
            parent_request_id: None,
            attempt_seq: request_identity.attempt_seq,
            transport: transport.to_string(),
            request_count_before: before,
            request_count_after: after,
            started_at_ms,
            budget_state_before: budget_state_before.to_string(),
            budget_state_after: budget_state_after.to_string(),
            budget_transition_reason: budget_transition_reason.to_string(),
            request_phase,
        })
    }

    pub(crate) fn record_response_completed(&self, token_usage: Option<&TokenUsage>) {
        self.record_active_terminal_status("response_completed", token_usage);
    }

    pub(crate) fn record_response_failed(&self) {
        self.record_active_terminal_status("response_failed", None);
    }

    pub(crate) fn record_cancelled(&self) {
        self.record_active_terminal_status("cancelled", None);
    }

    fn record_provider_payload(&self, mut payload: ProviderPayloadDigest) {
        if !self.state.enabled {
            return;
        }
        apply_projection_identity_expectation(
            &mut payload.scan,
            self.state.expected_projection_identity.as_ref(),
        );
        let active_request = {
            let mut active_request = lock_std_mutex(&self.state.active_request);
            let Some(active_request) = active_request.as_mut() else {
                return;
            };
            payload.scan.request_id = active_request.request_id.clone();
            payload.scan.provider_payload_sha256 = payload.sha256.clone();
            payload.scan.scan_event_id =
                format!("scan:{}:{}", active_request.request_id, payload.sha256);
            active_request.provider_payload_sha256 = Some(payload.sha256);
            active_request.provider_payload_bytes = Some(payload.bytes);
            active_request.provider_wire_api = Some(payload.provider_wire_api);
            active_request.tools_count = Some(payload.tools_count);
            active_request.tools_present = Some(payload.tools_present);
            active_request.request_shape_classifier = Some(payload.request_shape_classifier);
            active_request.messages_hash = Some(payload.messages_hash);
            active_request.stable_prefix_hash = Some(payload.stable_prefix_hash);
            active_request.dynamic_suffix_hash = Some(payload.dynamic_suffix_hash);
            active_request.payload_scan = Some(payload.scan);
            active_request.clone()
        };
        let scan = active_request.payload_scan.clone();
        self.push_event(ProviderRequestBudgetEvent {
            request_id: active_request.request_id,
            logical_request_id: active_request.logical_request_id,
            parent_request_id: active_request.parent_request_id,
            attempt_seq: active_request.attempt_seq,
            transport: active_request.transport,
            status: "payload_captured".to_string(),
            request_count_before: active_request.request_count_before,
            request_count_after: active_request.request_count_after,
            max_requests: self.state.max_requests,
            budget_state_before: active_request.budget_state_before,
            budget_state_after: active_request.budget_state_after,
            budget_transition_reason: active_request.budget_transition_reason,
            started_at_ms: active_request.started_at_ms,
            completed_at_ms: None,
            latency_ms: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_output_tokens: None,
            total_tokens: None,
            provider_payload_sha256: active_request.provider_payload_sha256,
            provider_payload_bytes: active_request.provider_payload_bytes,
            provider_wire_api: active_request.provider_wire_api,
            tools_count: active_request.tools_count,
            tools_present: active_request.tools_present,
            request_shape_classifier: active_request.request_shape_classifier,
            messages_hash: active_request.messages_hash,
            stable_prefix_hash: active_request.stable_prefix_hash,
            dynamic_suffix_hash: active_request.dynamic_suffix_hash,
            exact_payload_scan_passed: scan.as_ref().map(|scan| scan.passed),
            active_projection_present: scan.as_ref().map(|scan| scan.active_projection_present),
            active_projection_count: scan.as_ref().map(|scan| scan.active_projection_count),
            large_raw_output_tokens: scan.as_ref().map(|scan| scan.large_raw_output_tokens),
            protected_items_present: scan.as_ref().map(|scan| scan.protected_items_present),
            replacement_confirmed: scan.as_ref().map(|scan| scan.replacement_confirmed),
            exact_payload_scan: scan,
            task_id: self.state.attribution.task_id.clone(),
            map_id: self.state.attribution.map_id.clone(),
            node_id: self.state.attribution.node_id.clone(),
            request_phase: active_request.request_phase,
        });
    }

    fn record_active_terminal_status(&self, status: &str, token_usage: Option<&TokenUsage>) {
        if !self.state.enabled {
            return;
        }
        let active_request = lock_std_mutex(&self.state.active_request).take();
        if let Some(active_request) = active_request {
            let completed_at_ms = provider_request_budget_now_ms();
            self.push_event(ProviderRequestBudgetEvent {
                request_id: active_request.request_id,
                logical_request_id: active_request.logical_request_id,
                parent_request_id: active_request.parent_request_id,
                attempt_seq: active_request.attempt_seq,
                transport: active_request.transport,
                status: status.to_string(),
                request_count_before: active_request.request_count_before,
                request_count_after: active_request.request_count_after,
                max_requests: self.state.max_requests,
                budget_state_before: active_request.budget_state_before,
                budget_state_after: active_request.budget_state_after,
                budget_transition_reason: active_request.budget_transition_reason,
                started_at_ms: active_request.started_at_ms,
                completed_at_ms: Some(completed_at_ms),
                latency_ms: Some(completed_at_ms.saturating_sub(active_request.started_at_ms)),
                input_tokens: token_usage.map(|usage| usage.input_tokens),
                cached_input_tokens: token_usage.map(|usage| usage.cached_input_tokens),
                output_tokens: token_usage.map(|usage| usage.output_tokens),
                reasoning_output_tokens: token_usage.map(|usage| usage.reasoning_output_tokens),
                total_tokens: token_usage.map(|usage| usage.total_tokens),
                provider_payload_sha256: active_request.provider_payload_sha256,
                provider_payload_bytes: active_request.provider_payload_bytes,
                provider_wire_api: active_request.provider_wire_api,
                tools_count: active_request.tools_count,
                tools_present: active_request.tools_present,
                request_shape_classifier: active_request.request_shape_classifier,
                messages_hash: active_request.messages_hash,
                stable_prefix_hash: active_request.stable_prefix_hash,
                dynamic_suffix_hash: active_request.dynamic_suffix_hash,
                exact_payload_scan_passed: active_request
                    .payload_scan
                    .as_ref()
                    .map(|scan| scan.passed),
                active_projection_present: active_request
                    .payload_scan
                    .as_ref()
                    .map(|scan| scan.active_projection_present),
                active_projection_count: active_request
                    .payload_scan
                    .as_ref()
                    .map(|scan| scan.active_projection_count),
                large_raw_output_tokens: active_request
                    .payload_scan
                    .as_ref()
                    .map(|scan| scan.large_raw_output_tokens),
                protected_items_present: active_request
                    .payload_scan
                    .as_ref()
                    .map(|scan| scan.protected_items_present),
                replacement_confirmed: active_request
                    .payload_scan
                    .as_ref()
                    .map(|scan| scan.replacement_confirmed),
                exact_payload_scan: active_request.payload_scan,
                task_id: self.state.attribution.task_id.clone(),
                map_id: self.state.attribution.map_id.clone(),
                node_id: self.state.attribution.node_id.clone(),
                request_phase: active_request.request_phase,
            });
        }
    }

    pub(crate) fn drain_events(&self) -> Vec<ProviderRequestBudgetEvent> {
        let mut events = lock_std_mutex(&self.state.events);
        std::mem::take(&mut *events)
    }

    fn push_event(&self, event: ProviderRequestBudgetEvent) {
        lock_std_mutex(&self.state.events).push(event);
    }

    fn build_request_identity(
        &self,
        logical_request_seq: usize,
        attempt_seq: usize,
    ) -> ProviderRequestIdentity {
        let scope = self
            .state
            .attribution
            .request_scope_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("scope-unknown");
        let scope = sanitize_provider_request_id_part(scope);
        let logical_request_id = format!("provider-request:{scope}:logical-{logical_request_seq}");
        let request_id = format!("{logical_request_id}:attempt-{attempt_seq}");
        ProviderRequestIdentity {
            request_id,
            logical_request_id,
            attempt_seq,
        }
    }

    fn active_payload_for_request(
        &self,
        request_id: &str,
    ) -> Option<ProviderRequestBudgetActiveRequest> {
        let active_request = lock_std_mutex(&self.state.active_request);
        let active_request = active_request.as_ref()?;
        if active_request.request_id != request_id {
            return None;
        }
        Some(active_request.clone())
    }
}

fn lock_std_mutex<T>(mutex: &StdMutex<T>) -> StdMutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Debug)]
struct ProviderRequestBudgetDispatch {
    context: Option<ProviderRequestBudgetContext>,
    request_id: String,
    logical_request_id: String,
    parent_request_id: Option<String>,
    attempt_seq: usize,
    transport: String,
    request_count_before: usize,
    request_count_after: usize,
    budget_state_before: String,
    budget_state_after: String,
    budget_transition_reason: String,
    started_at_ms: i64,
    request_phase: Option<String>,
}

impl ProviderRequestBudgetDispatch {
    fn disabled() -> Self {
        Self {
            context: None,
            request_id: String::new(),
            logical_request_id: String::new(),
            parent_request_id: None,
            attempt_seq: 0,
            transport: String::new(),
            request_count_before: 0,
            request_count_after: 0,
            budget_state_before: String::new(),
            budget_state_after: String::new(),
            budget_transition_reason: String::new(),
            started_at_ms: 0,
            request_phase: None,
        }
    }

    fn record_status(&self, status: &str) {
        if let Some(context) = &self.context {
            let active_payload = context.active_payload_for_request(&self.request_id);
            let payload_scan = active_payload
                .as_ref()
                .and_then(|active| active.payload_scan.clone());
            context.push_event(ProviderRequestBudgetEvent {
                request_id: self.request_id.clone(),
                logical_request_id: self.logical_request_id.clone(),
                parent_request_id: self.parent_request_id.clone(),
                attempt_seq: self.attempt_seq,
                transport: self.transport.clone(),
                status: status.to_string(),
                request_count_before: self.request_count_before,
                request_count_after: self.request_count_after,
                max_requests: context.state.max_requests,
                budget_state_before: self.budget_state_before.clone(),
                budget_state_after: self.budget_state_after.clone(),
                budget_transition_reason: self.budget_transition_reason.clone(),
                started_at_ms: self.started_at_ms,
                completed_at_ms: None,
                latency_ms: None,
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                reasoning_output_tokens: None,
                total_tokens: None,
                provider_payload_sha256: active_payload
                    .as_ref()
                    .and_then(|active| active.provider_payload_sha256.clone()),
                provider_payload_bytes: active_payload
                    .as_ref()
                    .and_then(|active| active.provider_payload_bytes),
                provider_wire_api: active_payload
                    .as_ref()
                    .and_then(|active| active.provider_wire_api.clone()),
                tools_count: active_payload
                    .as_ref()
                    .and_then(|active| active.tools_count),
                tools_present: active_payload
                    .as_ref()
                    .and_then(|active| active.tools_present),
                request_shape_classifier: active_payload
                    .as_ref()
                    .and_then(|active| active.request_shape_classifier.clone()),
                messages_hash: active_payload
                    .as_ref()
                    .and_then(|active| active.messages_hash.clone()),
                stable_prefix_hash: active_payload
                    .as_ref()
                    .and_then(|active| active.stable_prefix_hash.clone()),
                dynamic_suffix_hash: active_payload
                    .as_ref()
                    .and_then(|active| active.dynamic_suffix_hash.clone()),
                exact_payload_scan_passed: payload_scan.as_ref().map(|scan| scan.passed),
                active_projection_present: payload_scan
                    .as_ref()
                    .map(|scan| scan.active_projection_present),
                active_projection_count: payload_scan
                    .as_ref()
                    .map(|scan| scan.active_projection_count),
                large_raw_output_tokens: payload_scan
                    .as_ref()
                    .map(|scan| scan.large_raw_output_tokens),
                protected_items_present: payload_scan
                    .as_ref()
                    .map(|scan| scan.protected_items_present),
                replacement_confirmed: payload_scan.as_ref().map(|scan| scan.replacement_confirmed),
                exact_payload_scan: payload_scan,
                task_id: context.state.attribution.task_id.clone(),
                map_id: context.state.attribution.map_id.clone(),
                node_id: context.state.attribution.node_id.clone(),
                request_phase: self.request_phase.clone(),
            });
        }
    }

    fn record_provider_payload(&self, payload: ProviderPayloadDigest) {
        if let Some(context) = &self.context {
            context.record_provider_payload(payload);
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn json_field_hash(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|field_value| serde_json::to_vec(field_value).ok())
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|| sha256_hex(b"null"))
}

#[cfg(test)]
fn provider_payload_digest<T: serde::Serialize>(payload: &T) -> Option<ProviderPayloadDigest> {
    provider_payload_digest_for_wire(payload, codex_api::WireApi::Responses)
}

fn provider_payload_digest_for_wire<T: serde::Serialize>(
    payload: &T,
    provider_wire_api: codex_api::WireApi,
) -> Option<ProviderPayloadDigest> {
    let value = serde_json::to_value(payload).ok()?;
    provider_payload_digest_for_wire_value(&value, provider_wire_api)
}

fn provider_payload_digest_for_wire_value(
    value: &serde_json::Value,
    provider_wire_api: codex_api::WireApi,
) -> Option<ProviderPayloadDigest> {
    let bytes = serde_json::to_vec(value).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let sha256 = sha256_hex(&bytes);
    let tools_count = value
        .get("tools")
        .and_then(|tools| tools.as_array())
        .map(std::vec::Vec::len)
        .unwrap_or(0);
    let tools_present = tools_count > 0;
    let request_shape_classifier = if tools_present {
        "native_tools_schema_hot_path"
    } else {
        "tool_free_action_contract"
    };
    Some(ProviderPayloadDigest {
        sha256: sha256.clone(),
        bytes: bytes.len(),
        provider_wire_api: format!("{provider_wire_api:?}"),
        tools_count,
        tools_present,
        request_shape_classifier: request_shape_classifier.to_string(),
        messages_hash: json_field_hash(value, provider_wire_message_field(provider_wire_api)),
        stable_prefix_hash: provider_wire_stable_prefix_hash(value, provider_wire_api),
        dynamic_suffix_hash: provider_wire_dynamic_suffix_hash(value, provider_wire_api),
        scan: scan_provider_payload_text("request-unbound", &sha256, &text, value),
    })
}

fn provider_wire_message_field(provider_wire_api: codex_api::WireApi) -> &'static str {
    if provider_wire_api == codex_api::WireApi::ChatCompletions {
        "messages"
    } else {
        "input"
    }
}

fn provider_wire_stable_prefix_hash(
    value: &serde_json::Value,
    provider_wire_api: codex_api::WireApi,
) -> String {
    let first_message = value
        .get(provider_wire_message_field(provider_wire_api))
        .and_then(serde_json::Value::as_array)
        .and_then(|messages| messages.first())
        .unwrap_or(&serde_json::Value::Null);
    let stable_prefix = serde_json::json!({
        "first_message": first_message,
        "tools": value.get("tools").unwrap_or(&serde_json::Value::Null),
    });
    serde_json::to_vec(&stable_prefix)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"null"))
}

fn provider_wire_dynamic_suffix_hash(
    value: &serde_json::Value,
    provider_wire_api: codex_api::WireApi,
) -> String {
    value
        .get(provider_wire_message_field(provider_wire_api))
        .and_then(serde_json::Value::as_array)
        .and_then(|messages| messages.last())
        .and_then(|message| serde_json::to_vec(message).ok())
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|| sha256_hex(b"null"))
}

fn scan_provider_payload_text(
    request_id: &str,
    sha256: &str,
    text: &str,
    value: &serde_json::Value,
) -> ExactPayloadScanEventV1 {
    let projection_required = provider_payload_has_tool(value, "taskspace_control");
    let projection_blocks = provider_projection_blocks(value);
    let active_projection_count = projection_blocks.len();
    let active_projection_present = active_projection_count > 0;
    let projection_is_message_tail = provider_projection_is_message_tail(value);
    let protected_items_present = !projection_blocks.is_empty()
        && projection_blocks
            .iter()
            .all(|block| projection_block_is_valid(block));
    let projection_identity = projection_blocks
        .last()
        .map(|projection| projection_identity(projection));
    let revision_sequence_valid = projection_revision_sequence_valid(&projection_blocks);
    let large_raw_output_tokens = estimate_large_raw_output_tokens(value);
    let runtime_boundary_forbidden_markers = [
        "TaskSpaceProviderBudgetHardStopV1",
        "provider_request_hard_limit_exceeded",
        "TaskSpaceAgentContextBundleV1",
        "TaskSpaceToolSemanticSummaryV1",
        "schema_property_rename_hints=",
        "next_valid_actions",
        "validation_needs_test",
        "rejected_by_state_baseline",
    ]
    .into_iter()
    .filter(|marker| text.contains(marker))
    .map(str::to_string)
    .collect::<Vec<_>>();
    let replacement_confirmed = if projection_required {
        active_projection_count == 1
            && protected_items_present
            && large_raw_output_tokens == 0
            && runtime_boundary_forbidden_markers.is_empty()
    } else {
        active_projection_count == 0
            && large_raw_output_tokens == 0
            && runtime_boundary_forbidden_markers.is_empty()
    };
    let mut failure_reasons = Vec::new();
    if projection_required && active_projection_count == 0 {
        failure_reasons.push("current_projection_missing".to_string());
    }
    if projection_required && active_projection_count > 0 && !projection_is_message_tail {
        failure_reasons.push("current_projection_not_message_tail".to_string());
    }
    if projection_required && active_projection_count > 1 {
        failure_reasons.push("current_projection_not_unique".to_string());
    }
    if active_projection_count > 1 && !revision_sequence_valid {
        failure_reasons.push("projection_revision_order_invalid".to_string());
    }
    if !projection_required && active_projection_count != 0 {
        failure_reasons.push("unexpected_current_projection".to_string());
    }
    if large_raw_output_tokens > 0 {
        failure_reasons.push("large_raw_output_present".to_string());
    }
    if !runtime_boundary_forbidden_markers.is_empty() {
        failure_reasons.push("runtime_boundary_forbidden_marker_present".to_string());
    }
    if projection_required && active_projection_count == 1 && !protected_items_present {
        failure_reasons.push("current_projection_required_sections_missing".to_string());
    }
    ExactPayloadScanEventV1 {
        schema_version: "taskspace-exact-payload-scan-event-v1",
        scan_event_id: format!("scan:{request_id}:{sha256}"),
        request_id: request_id.to_string(),
        provider_payload_sha256: sha256.to_string(),
        scanner_version: "r7-exact-scan-1".to_string(),
        matcher_version: "r7-projection-checks-1".to_string(),
        checked_byte_ranges: vec![(0, text.len())],
        negative_checks_performed: vec![
            "current_projection_uniqueness".to_string(),
            "current_projection_message_tail".to_string(),
            "large_raw_output".to_string(),
            "runtime_boundary_forbidden_markers".to_string(),
        ],
        projection_required,
        active_projection_present,
        active_projection_count,
        projection_is_message_tail,
        large_raw_output_tokens,
        runtime_boundary_forbidden_markers,
        protected_items_present,
        projection_kind: projection_identity
            .as_ref()
            .map(|identity| identity.kind.clone()),
        projection_map_id_sha256: projection_identity
            .as_ref()
            .and_then(|identity| identity.map_id_sha256.clone()),
        projection_revision: projection_identity
            .as_ref()
            .and_then(|identity| identity.revision),
        projection_canonical_sha256: projection_identity
            .as_ref()
            .and_then(|identity| identity.canonical_sha256.clone()),
        projection_sha256: projection_identity.map(|identity| identity.projection_sha256),
        projection_policy: None,
        expected_projection_kind: None,
        expected_projection_map_id_sha256: None,
        expected_projection_revision: None,
        expected_projection_canonical_sha256: None,
        expected_projection_sha256: None,
        projection_identity_confirmed: None,
        replacement_confirmed,
        passed: failure_reasons.is_empty(),
        failure_reasons,
    }
}

#[derive(Debug)]
struct ProviderProjectionIdentity {
    kind: String,
    map_id_sha256: Option<String>,
    revision: Option<u64>,
    canonical_sha256: Option<String>,
    projection_sha256: String,
}

fn provider_projection_blocks(value: &serde_json::Value) -> Vec<&str> {
    let Some(messages) = value.get("messages").or_else(|| value.get("input")) else {
        return Vec::new();
    };
    if let Some(text) = messages.as_str() {
        return projection_blocks_from_text(text);
    }
    let Some(messages) = messages.as_array() else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    for message in messages {
        let role = message.get("role").and_then(serde_json::Value::as_str);
        if !matches!(role, Some("developer" | "system" | "user")) {
            continue;
        }
        let Some(content) = message.get("content") else {
            continue;
        };
        let mut strings = Vec::new();
        collect_provider_strings(content, &mut strings);
        for text in strings {
            blocks.extend(projection_blocks_from_text(text));
        }
    }
    blocks
}

fn provider_projection_is_message_tail(value: &serde_json::Value) -> bool {
    let Some(messages) = value.get("messages").or_else(|| value.get("input")) else {
        return false;
    };
    if let Some(text) = messages.as_str() {
        return !projection_blocks_from_text(text).is_empty()
            && text.trim_end().ends_with(TASKSPACE_PROJECTION_END_MARKER);
    }
    let Some(last_message) = messages.as_array().and_then(|messages| messages.last()) else {
        return false;
    };
    let role = last_message.get("role").and_then(serde_json::Value::as_str);
    if !matches!(role, Some("developer" | "system" | "user")) {
        return false;
    }
    let Some(content) = last_message.get("content") else {
        return false;
    };
    let mut strings = Vec::new();
    collect_provider_strings(content, &mut strings);
    strings
        .into_iter()
        .any(|text| !projection_blocks_from_text(text).is_empty())
}

fn projection_blocks_from_text(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut remainder = text;
    while let Some(start) = remainder.find(TASKSPACE_PROJECTION_MARKER) {
        let candidate = &remainder[start..];
        let Some(end) = candidate.find(TASKSPACE_PROJECTION_END_MARKER) else {
            blocks.push(candidate);
            break;
        };
        let end = end + TASKSPACE_PROJECTION_END_MARKER.len();
        blocks.push(&candidate[..end]);
        remainder = &candidate[end..];
    }
    blocks
}

fn collect_provider_strings<'a>(value: &'a serde_json::Value, strings: &mut Vec<&'a str>) {
    match value {
        serde_json::Value::String(text) => strings.push(text),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_provider_strings(value, strings);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                collect_provider_strings(value, strings);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

fn projection_identity(projection: &str) -> ProviderProjectionIdentity {
    let projection_sha256 = sha256_hex(projection.as_bytes());
    let bootstrap = projection.lines().any(|line| line == "- map: none")
        && projection
            .lines()
            .any(|line| line == "- bootstrap_required: true");
    if bootstrap {
        return ProviderProjectionIdentity {
            kind: "bootstrap_required".to_string(),
            map_id_sha256: None,
            revision: None,
            canonical_sha256: None,
            projection_sha256,
        };
    }
    let map_id_sha256 = projection_mechanical_field(projection, "map_id")
        .map(|map_id| sha256_hex(map_id.as_bytes()));
    let revision = projection_mechanical_field(projection, "revision")
        .and_then(|value| value.parse::<u64>().ok());
    let canonical_sha256 =
        projection_mechanical_field(projection, "canonical_sha256").map(str::to_string);
    let kind = projection_mechanical_field(projection, "projection_kind")
        .filter(|_| map_id_sha256.is_some() && revision.is_some() && canonical_sha256.is_some())
        .unwrap_or("unavailable")
        .to_string();
    ProviderProjectionIdentity {
        kind,
        map_id_sha256,
        revision,
        canonical_sha256,
        projection_sha256,
    }
}

fn projection_mechanical_field<'a>(projection: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("- {field}: ");
    projection
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .filter(|value| !value.is_empty())
}

fn apply_projection_identity_expectation(
    scan: &mut ExactPayloadScanEventV1,
    expected: Option<&ProviderProjectionIdentityExpectation>,
) {
    let Some(expected) = expected else {
        return;
    };
    scan.projection_policy = Some(expected.policy.to_string());
    if !expected.automatic_projection_required {
        scan.projection_required = false;
        scan.expected_projection_kind = Some("none".to_string());
        scan.failure_reasons.retain(|reason| {
            !matches!(
                reason.as_str(),
                "current_projection_missing"
                    | "current_projection_not_message_tail"
                    | "current_projection_not_unique"
                    | "current_projection_required_sections_missing"
            )
        });
        let confirmed = scan.active_projection_count == 0;
        scan.projection_identity_confirmed = Some(confirmed);
        scan.protected_items_present = confirmed;
        scan.negative_checks_performed
            .push("automatic_projection_absence".to_string());
        scan.matcher_version = "r7-projection-checks-3".to_string();
        if !confirmed {
            scan.failure_reasons
                .push("unexpected_automatic_projection".to_string());
        }
        scan.passed = scan.failure_reasons.is_empty();
        scan.replacement_confirmed = scan.passed && confirmed;
        return;
    }
    scan.expected_projection_kind = Some(expected.kind.clone());
    scan.expected_projection_map_id_sha256 = expected.map_id_sha256.clone();
    scan.expected_projection_revision = expected.revision;
    scan.expected_projection_canonical_sha256 = expected.canonical_sha256.clone();
    scan.expected_projection_sha256 = Some(expected.projection_sha256.clone());
    if expected.policy == TaskSpaceProjectionPolicy::MapAppend {
        scan.failure_reasons.retain(|reason| {
            !matches!(
                reason.as_str(),
                "current_projection_missing"
                    | "current_projection_not_unique"
                    | "current_projection_required_sections_missing"
            )
        });
        scan.protected_items_present =
            scan.active_projection_count == 0 || scan.protected_items_present;
    }
    let confirmed = scan.projection_kind.as_ref() == Some(&expected.kind)
        && scan.projection_map_id_sha256 == expected.map_id_sha256
        && scan.projection_revision == expected.revision
        && scan.projection_canonical_sha256 == expected.canonical_sha256
        && scan.projection_sha256.as_ref() == Some(&expected.projection_sha256);
    scan.projection_identity_confirmed = Some(confirmed);
    scan.negative_checks_performed
        .push("projection_identity".to_string());
    scan.matcher_version = "r7-projection-checks-2".to_string();
    if !confirmed {
        scan.failure_reasons
            .push("projection_identity_mismatch".to_string());
        scan.passed = false;
        scan.replacement_confirmed = false;
    }
    if expected.policy == TaskSpaceProjectionPolicy::MapAppend {
        scan.passed = scan.failure_reasons.is_empty();
        scan.replacement_confirmed = scan.passed && confirmed;
    }
}

fn projection_revision_sequence_valid(blocks: &[&str]) -> bool {
    if blocks.len() < 2 {
        return true;
    }
    let mut current_map_id = None;
    let mut previous_revision = None;
    let mut closed_map_ids = HashSet::new();
    for block in blocks {
        if projection_is_bootstrap(block) {
            if current_map_id.is_some() || !closed_map_ids.is_empty() {
                return false;
            }
            continue;
        }
        let Some(map_id) = projection_mechanical_field(block, "map_id") else {
            return false;
        };
        let Some(revision) = projection_mechanical_field(block, "revision")
            .and_then(|value| value.parse::<u64>().ok())
        else {
            return false;
        };
        if current_map_id.is_some_and(|current| current != map_id) {
            if let Some(current) = current_map_id {
                closed_map_ids.insert(current);
            }
            if closed_map_ids.contains(map_id) {
                return false;
            }
        } else if previous_revision.is_some_and(|previous| previous > revision) {
            return false;
        }
        current_map_id = Some(map_id);
        previous_revision = Some(revision);
    }
    true
}

fn projection_is_bootstrap(projection: &str) -> bool {
    projection.lines().any(|line| line == "- map: none")
        && projection
            .lines()
            .any(|line| line == "- bootstrap_required: true")
}

fn projection_block_is_valid(block: &str) -> bool {
    let normalized = block.replace("\\r\\n", "\n").replace("\\n", "\n");
    if !normalized
        .lines()
        .any(|line| line.trim() == "- schema_version: taskspace-map-projection-r7-v1")
    {
        return false;
    }
    if normalized.lines().any(|line| {
        line.trim().starts_with("- integrity_status: invalid")
            || line.trim().starts_with("integrity_status: invalid")
    }) {
        return false;
    }
    let blank_bootstrap = normalized
        .lines()
        .any(|line| line.trim().starts_with("- map: none") || line.trim().starts_with("map: none"));
    if blank_bootstrap {
        return normalized.lines().any(|line| {
            line.trim().starts_with("- bootstrap_required: true")
                || line.trim().starts_with("bootstrap_required: true")
        });
    }
    if !projection_block_contains_required_sections(&normalized) {
        return false;
    }
    if projection_mechanical_field(&normalized, "projection_kind") == Some("request_snapshot") {
        return projection_mechanical_field(&normalized, "supersedes_all_prior_projections")
            == Some("true")
            && projection_mechanical_field(&normalized, "current_state_rule")
                == Some("last_projection_only");
    }
    true
}

fn provider_payload_has_tool(value: &serde_json::Value, expected: &str) -> bool {
    value
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| provider_tool_name(tool) == Some(expected))
        })
}

fn provider_tool_name(tool: &serde_json::Value) -> Option<&str> {
    tool.get("name")
        .or_else(|| {
            tool.get("function")
                .and_then(|function| function.get("name"))
        })
        .and_then(serde_json::Value::as_str)
}

fn projection_block_contains_required_sections(block: &str) -> bool {
    let normalized = block.replace("\\r\\n", "\n").replace("\\n", "\n");
    TASKSPACE_PROJECTION_REQUIRED_SECTIONS
        .iter()
        .all(|section| projection_block_contains_section(&normalized, section))
}

fn projection_block_contains_section(block: &str, section: &str) -> bool {
    let section_prefix = format!("{section}:");
    block.lines().any(|line| {
        line.trim_start()
            .strip_prefix("- ")
            .unwrap_or_else(|| line.trim_start())
            .starts_with(&section_prefix)
    })
}

fn estimate_large_raw_output_tokens(value: &serde_json::Value) -> usize {
    const LARGE_RAW_OUTPUT_BYTES: usize = 50 * 1024;
    fn walk(value: &serde_json::Value, inside_tool_output: bool, threshold: usize) -> usize {
        match value {
            serde_json::Value::Object(object) => {
                let item_type = object
                    .get("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let is_tool_output = inside_tool_output
                    || matches!(
                        item_type,
                        "function_call_output"
                            | "custom_tool_call_output"
                            | "tool_search_output"
                            | "mcp_tool_call_output"
                    );
                object
                    .values()
                    .map(|value| walk(value, is_tool_output, threshold))
                    .sum()
            }
            serde_json::Value::Array(items) => items
                .iter()
                .map(|value| walk(value, inside_tool_output, threshold))
                .sum(),
            serde_json::Value::String(text) if inside_tool_output && text.len() > threshold => {
                if text.contains("OutputReferenceV1:") && text.contains("raw_output_elided: true") {
                    0
                } else {
                    text.len() / 4
                }
            }
            _ => 0,
        }
    }
    walk(value, false, LARGE_RAW_OUTPUT_BYTES)
}

fn sanitize_provider_request_id_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "scope-unknown".to_string()
    } else {
        sanitized
    }
}

fn provider_request_budget_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[derive(Debug)]
struct ProviderRequestHardLimit {
    limit: Option<usize>,
    count: AtomicUsize,
    configuration_error: Option<String>,
}

impl ProviderRequestHardLimit {
    fn from_env() -> Self {
        let Some(raw) = std::env::var_os(PROVIDER_REQUEST_HARD_LIMIT_ENV) else {
            return Self::disabled();
        };
        let value = raw.to_string_lossy();
        match value.parse::<usize>() {
            Ok(limit) if limit > 0 => Self::with_limit(limit),
            _ => Self {
                limit: None,
                count: AtomicUsize::new(0),
                configuration_error: Some(format!(
                    "{PROVIDER_REQUEST_HARD_LIMIT_ENV} must be a positive integer"
                )),
            },
        }
    }

    fn disabled() -> Self {
        Self {
            limit: None,
            count: AtomicUsize::new(0),
            configuration_error: None,
        }
    }

    fn with_limit(limit: usize) -> Self {
        Self {
            limit: Some(limit),
            count: AtomicUsize::new(0),
            configuration_error: None,
        }
    }

    fn claim(&self, route: &str) -> Result<()> {
        if let Some(error) = &self.configuration_error {
            warn!(
                route,
                error, "provider request hard limit configuration rejected dispatch"
            );
            return Err(CodexErr::Fatal(error.clone()));
        }
        let Some(limit) = self.limit else {
            return Ok(());
        };
        self.count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                (count < limit).then_some(count + 1)
            })
            .map(|_| ())
            .map_err(|count| {
                warn!(
                    route,
                    limit, count, "provider request hard limit rejected dispatch"
                );
                CodexErr::Fatal(format!("provider request hard limit reached (max {limit})"))
            })
    }
}

/// Session-scoped state shared by all [`ModelClient`] clones.
///
/// This is intentionally kept minimal so `ModelClient` does not need to hold a full `Config`. Most
/// configuration is per turn and is passed explicitly to streaming/unary methods.
#[derive(Debug)]
struct ModelClientState {
    conversation_id: ThreadId,
    window_generation: AtomicU64,
    installation_id: String,
    provider: SharedModelProvider,
    auth_env_telemetry: AuthEnvTelemetry,
    session_source: SessionSource,
    model_verbosity: Option<VerbosityConfig>,
    enable_request_compression: bool,
    include_timing_metrics: bool,
    beta_features_header: Option<String>,
    disable_websockets: AtomicBool,
    cached_websocket_session: StdMutex<WebsocketSession>,
    provider_wire_trace: ProviderWireTrace,
    provider_request_hard_limit: ProviderRequestHardLimit,
}

/// Resolved API client setup for a single request attempt.
///
/// Keeping this as a single bundle ensures prewarm and normal request paths
/// share the same auth/provider setup flow.
struct CurrentClientSetup {
    auth: Option<CodexAuth>,
    api_provider: ApiProvider,
    api_auth: SharedAuthProvider,
}

#[derive(Clone, Copy)]
struct RequestRouteTelemetry {
    endpoint: &'static str,
}

impl RequestRouteTelemetry {
    fn for_endpoint(endpoint: &'static str) -> Self {
        Self { endpoint }
    }
}

/// A session-scoped client for model-provider API calls.
///
/// This holds configuration and state that should be shared across turns within a Codex session
/// (auth, provider selection, conversation id, and transport fallback state).
///
/// WebSocket fallback is session-scoped: once a turn activates the HTTP fallback, subsequent turns
/// will also use HTTP for the remainder of the session.
///
/// Turn-scoped settings (model selection, reasoning controls, telemetry context, and turn
/// metadata) are passed explicitly to the relevant methods to keep turn lifetime visible at the
/// call site.
#[derive(Debug, Clone)]
pub struct ModelClient {
    state: Arc<ModelClientState>,
}

/// A turn-scoped streaming session created from a [`ModelClient`].
///
/// The session establishes a Responses WebSocket connection lazily and reuses it across multiple
/// requests within the turn. It also caches per-turn state:
///
/// - The last full request, so subsequent calls can reuse incremental websocket request payloads
///   only when the current request is an incremental extension of the previous one.
/// - The `x-codex-turn-state` sticky-routing token, which must be replayed for all requests within
///   the same turn.
///
/// Create a fresh `ModelClientSession` for each Codex turn. Reusing it across turns would replay
/// the previous turn's sticky-routing token into the next turn, which violates the client/server
/// contract and can cause routing bugs.
pub struct ModelClientSession {
    client: ModelClient,
    websocket_session: WebsocketSession,
    /// Turn state for sticky routing.
    ///
    /// This is an `OnceLock` that stores the turn state value received from the server
    /// on turn start via the `x-codex-turn-state` response header. Once set, this value
    /// should be sent back to the server in the `x-codex-turn-state` request header for
    /// all subsequent requests within the same turn to maintain sticky routing.
    ///
    /// This is a contract between the client and server: we receive it at turn start,
    /// keep sending it unchanged between turn requests (e.g., for retries, incremental
    /// appends, or continuation requests), and must not send it between different turns.
    turn_state: Arc<OnceLock<String>>,
}

#[derive(Debug, Clone)]
struct LastResponse {
    response_id: String,
    items_added: Vec<ResponseItem>,
}

#[derive(Debug, Default)]
struct WebsocketSession {
    connection: Option<ApiWebSocketConnection>,
    last_request: Option<ResponsesApiRequest>,
    last_response_rx: Option<oneshot::Receiver<LastResponse>>,
    connection_reused: StdMutex<bool>,
}

impl WebsocketSession {
    fn set_connection_reused(&self, connection_reused: bool) {
        *self
            .connection_reused
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = connection_reused;
    }

    fn connection_reused(&self) -> bool {
        *self
            .connection_reused
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

enum WebsocketStreamOutcome {
    Stream(ResponseStream),
    FallbackToHttp,
}

/// Result of opening a WebRTC Realtime call.
///
/// The SDP answer goes back to the client. The call id and auth headers stay on the server so the
/// ordinary Realtime WebSocket machinery can join the same in-progress call as a sideband
/// controller.
pub(crate) struct RealtimeWebrtcCallStart {
    pub(crate) sdp: String,
    pub(crate) call_id: String,
    pub(crate) sideband_headers: ApiHeaderMap,
}

/// Reuses the API-auth material that created the WebRTC call for the sideband WebSocket join.
///
/// API-key sessions send that API bearer. ChatGPT-auth sessions send their bearer plus account id;
/// transceiver is responsible for accepting that same call-create identity on the direct
/// `api.openai.com` sideband path.
fn sideband_websocket_auth_headers(api_auth: &dyn AuthProvider) -> ApiHeaderMap {
    let mut headers = ApiHeaderMap::new();
    api_auth.add_auth_headers(&mut headers);
    headers
}

impl ModelClient {
    #[allow(clippy::too_many_arguments)]
    /// Creates a new session-scoped `ModelClient`.
    ///
    /// All arguments are expected to be stable for the lifetime of a Codex session. Per-turn values
    /// are passed to [`ModelClientSession::stream`] (and other turn-scoped methods) explicitly.
    pub fn new(
        auth_manager: Option<Arc<AuthManager>>,
        conversation_id: ThreadId,
        installation_id: String,
        provider_info: ModelProviderInfo,
        session_source: SessionSource,
        model_verbosity: Option<VerbosityConfig>,
        enable_request_compression: bool,
        include_timing_metrics: bool,
        beta_features_header: Option<String>,
    ) -> Self {
        let model_provider = create_model_provider(provider_info, auth_manager);
        let codex_api_key_env_enabled = model_provider
            .auth_manager()
            .as_ref()
            .is_some_and(|manager| manager.codex_api_key_env_enabled());
        let auth_env_telemetry =
            collect_auth_env_telemetry(model_provider.info(), codex_api_key_env_enabled);
        Self {
            state: Arc::new(ModelClientState {
                conversation_id,
                window_generation: AtomicU64::new(0),
                installation_id,
                provider: model_provider,
                auth_env_telemetry,
                session_source,
                model_verbosity,
                enable_request_compression,
                include_timing_metrics,
                beta_features_header,
                disable_websockets: AtomicBool::new(false),
                cached_websocket_session: StdMutex::new(WebsocketSession::default()),
                provider_wire_trace: ProviderWireTrace::from_env(),
                provider_request_hard_limit: ProviderRequestHardLimit::from_env(),
            }),
        }
    }

    /// Creates a fresh turn-scoped streaming session.
    ///
    /// This constructor does not perform network I/O itself; the session opens a websocket lazily
    /// when the first stream request is issued.
    pub fn new_session(&self) -> ModelClientSession {
        ModelClientSession {
            client: self.clone(),
            websocket_session: self.take_cached_websocket_session(),
            turn_state: Arc::new(OnceLock::new()),
        }
    }

    pub(crate) fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.state.provider.auth_manager()
    }

    pub(crate) fn set_window_generation(&self, window_generation: u64) {
        self.state
            .window_generation
            .store(window_generation, Ordering::Relaxed);
        self.store_cached_websocket_session(WebsocketSession::default());
    }

    pub(crate) fn advance_window_generation(&self) {
        self.state.window_generation.fetch_add(1, Ordering::Relaxed);
        self.store_cached_websocket_session(WebsocketSession::default());
    }

    fn current_window_id(&self) -> String {
        let conversation_id = self.state.conversation_id;
        let window_generation = self.state.window_generation.load(Ordering::Relaxed);
        format!("{conversation_id}:{window_generation}")
    }

    fn take_cached_websocket_session(&self) -> WebsocketSession {
        let mut cached_websocket_session = self
            .state
            .cached_websocket_session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *cached_websocket_session)
    }

    fn store_cached_websocket_session(&self, websocket_session: WebsocketSession) {
        *self
            .state
            .cached_websocket_session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = websocket_session;
    }

    pub(crate) fn force_http_fallback(
        &self,
        session_telemetry: &SessionTelemetry,
        _model_info: &ModelInfo,
    ) -> bool {
        let websocket_enabled = self.responses_websocket_enabled();
        let activated =
            websocket_enabled && !self.state.disable_websockets.swap(true, Ordering::Relaxed);
        if activated {
            warn!("falling back to HTTP");
            session_telemetry.counter(
                "codex.transport.fallback_to_http",
                /*inc*/ 1,
                &[("from_wire_api", "responses_websocket")],
            );
        }

        self.store_cached_websocket_session(WebsocketSession::default());
        activated
    }

    /// Compacts the current conversation history using the Compact endpoint.
    ///
    /// This is a unary call (no streaming) that returns a new list of
    /// `ResponseItem`s representing the compacted transcript.
    ///
    /// The model selection and telemetry context are passed explicitly to keep `ModelClient`
    /// session-scoped.
    pub async fn compact_conversation_history(
        &self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        session_telemetry: &SessionTelemetry,
        compaction_trace: &CompactionTraceContext,
    ) -> Result<Vec<ResponseItem>> {
        if prompt.input.is_empty() {
            return Ok(Vec::new());
        }
        let client_setup = self.current_client_setup().await?;
        let transport = ReqwestTransport::new(build_reqwest_client());
        let compact_request_timeout = client_setup
            .api_provider
            .stream_idle_timeout
            .saturating_mul(COMPACT_REQUEST_TIMEOUT_IDLE_MULTIPLIER);
        let request_telemetry = Self::build_request_telemetry(
            session_telemetry,
            AuthRequestTelemetryContext::new(
                client_setup.auth.as_ref().map(CodexAuth::auth_mode),
                client_setup.api_auth.as_ref(),
                PendingUnauthorizedRetry::default(),
            ),
            RequestRouteTelemetry::for_endpoint(RESPONSES_COMPACT_ENDPOINT),
            self.state.auth_env_telemetry.clone(),
        );
        let client =
            ApiCompactClient::new(transport, client_setup.api_provider, client_setup.api_auth)
                .with_telemetry(Some(request_telemetry));

        let instructions = prompt.base_instructions.text.clone();
        let input = prompt.get_formatted_input();
        let tools = create_tools_json_for_responses_api(&prompt.tools)?;
        let reasoning = Self::build_reasoning(model_info, effort, summary);
        let verbosity = if model_info.support_verbosity {
            self.state.model_verbosity.or(model_info.default_verbosity)
        } else {
            if self.state.model_verbosity.is_some() {
                warn!(
                    "model_verbosity is set but ignored as the model does not support verbosity: {}",
                    model_info.slug
                );
            }
            None
        };
        let text = create_text_param_for_request(
            verbosity,
            &prompt.output_schema,
            prompt.output_schema_strict,
        );
        let payload = ApiCompactionInput {
            model: &model_info.slug,
            input: &input,
            instructions: &instructions,
            tools,
            parallel_tool_calls: prompt.parallel_tool_calls,
            reasoning,
            text,
        };

        let mut extra_headers = ApiHeaderMap::new();
        if let Ok(header_value) = HeaderValue::from_str(&self.state.installation_id) {
            extra_headers.insert(X_CODEX_INSTALLATION_ID_HEADER, header_value);
        }
        extra_headers.extend(self.build_responses_identity_headers());
        extra_headers.extend(build_conversation_headers(Some(
            self.state.conversation_id.to_string(),
        )));
        let trace_attempt = compaction_trace.start_attempt(&payload);
        self.state
            .provider_request_hard_limit
            .claim(RESPONSES_COMPACT_ENDPOINT)?;
        let result = client
            .compact_input(&payload, extra_headers, compact_request_timeout)
            .await
            .map_err(map_api_error);
        trace_attempt.record_result(result.as_deref());
        result
    }

    pub(crate) async fn create_realtime_call_with_headers(
        &self,
        sdp: String,
        session_config: ApiRealtimeSessionConfig,
        extra_headers: ApiHeaderMap,
    ) -> Result<RealtimeWebrtcCallStart> {
        // Create the media call over HTTP first, then retain matching auth so realtime can attach
        // the server-side control WebSocket to the call id from that HTTP response.
        let client_setup = self.current_client_setup().await?;
        let mut sideband_headers = extra_headers.clone();
        sideband_headers.extend(sideband_websocket_auth_headers(
            client_setup.api_auth.as_ref(),
        ));
        let transport = ReqwestTransport::new(build_reqwest_client());
        let response =
            ApiRealtimeCallClient::new(transport, client_setup.api_provider, client_setup.api_auth)
                .create_with_session_and_headers(sdp, session_config, extra_headers)
                .await
                .map_err(map_api_error)?;
        Ok(RealtimeWebrtcCallStart {
            sdp: response.sdp,
            call_id: response.call_id,
            sideband_headers,
        })
    }

    /// Builds memory summaries for each provided normalized raw memory.
    ///
    /// This is a unary call (no streaming) to `/v1/memories/trace_summarize`.
    ///
    /// The model selection, reasoning effort, and telemetry context are passed explicitly to keep
    /// `ModelClient` session-scoped.
    pub async fn summarize_memories(
        &self,
        raw_memories: Vec<ApiRawMemory>,
        model_info: &ModelInfo,
        effort: Option<ReasoningEffortConfig>,
        session_telemetry: &SessionTelemetry,
    ) -> Result<Vec<ApiMemorySummarizeOutput>> {
        if raw_memories.is_empty() {
            return Ok(Vec::new());
        }

        let client_setup = self.current_client_setup().await?;
        let transport = ReqwestTransport::new(build_reqwest_client());
        let request_telemetry = Self::build_request_telemetry(
            session_telemetry,
            AuthRequestTelemetryContext::new(
                client_setup.auth.as_ref().map(CodexAuth::auth_mode),
                client_setup.api_auth.as_ref(),
                PendingUnauthorizedRetry::default(),
            ),
            RequestRouteTelemetry::for_endpoint(MEMORIES_SUMMARIZE_ENDPOINT),
            self.state.auth_env_telemetry.clone(),
        );
        let client =
            ApiMemoriesClient::new(transport, client_setup.api_provider, client_setup.api_auth)
                .with_telemetry(Some(request_telemetry));

        let payload = ApiMemorySummarizeInput {
            model: model_info.slug.clone(),
            raw_memories,
            reasoning: effort.map(|effort| Reasoning {
                effort: Some(effort),
                summary: None,
            }),
        };

        self.state
            .provider_request_hard_limit
            .claim(MEMORIES_SUMMARIZE_ENDPOINT)?;
        client
            .summarize_input(&payload, self.build_subagent_headers())
            .await
            .map_err(map_api_error)
    }

    fn build_subagent_headers(&self) -> ApiHeaderMap {
        let mut extra_headers = ApiHeaderMap::new();
        if let Some(subagent) = subagent_header_value(&self.state.session_source)
            && let Ok(val) = HeaderValue::from_str(&subagent)
        {
            extra_headers.insert(X_OPENAI_SUBAGENT_HEADER, val);
        }
        if matches!(
            self.state.session_source,
            SessionSource::SubAgent(SubAgentSource::MemoryConsolidation)
        ) {
            extra_headers.insert(
                X_OPENAI_MEMGEN_REQUEST_HEADER,
                HeaderValue::from_static("true"),
            );
        }
        extra_headers
    }

    fn build_responses_identity_headers(&self) -> ApiHeaderMap {
        let mut extra_headers = self.build_subagent_headers();
        if let Some(parent_thread_id) = parent_thread_id_header_value(&self.state.session_source)
            && let Ok(val) = HeaderValue::from_str(&parent_thread_id)
        {
            extra_headers.insert(X_CODEX_PARENT_THREAD_ID_HEADER, val);
        }
        if let Ok(val) = HeaderValue::from_str(&self.current_window_id()) {
            extra_headers.insert(X_CODEX_WINDOW_ID_HEADER, val);
        }
        extra_headers
    }

    fn build_ws_client_metadata(
        &self,
        turn_metadata_header: Option<&str>,
    ) -> HashMap<String, String> {
        let mut client_metadata = HashMap::new();
        client_metadata.insert(
            X_CODEX_INSTALLATION_ID_HEADER.to_string(),
            self.state.installation_id.clone(),
        );
        client_metadata.insert(
            X_CODEX_WINDOW_ID_HEADER.to_string(),
            self.current_window_id(),
        );
        if let Some(subagent) = subagent_header_value(&self.state.session_source) {
            client_metadata.insert(X_OPENAI_SUBAGENT_HEADER.to_string(), subagent);
        }
        if let Some(parent_thread_id) = parent_thread_id_header_value(&self.state.session_source) {
            client_metadata.insert(
                X_CODEX_PARENT_THREAD_ID_HEADER.to_string(),
                parent_thread_id,
            );
        }
        if let Some(turn_metadata_header) = parse_turn_metadata_header(turn_metadata_header)
            && let Ok(turn_metadata) = turn_metadata_header.to_str()
        {
            client_metadata.insert(
                X_CODEX_TURN_METADATA_HEADER.to_string(),
                turn_metadata.to_string(),
            );
        }
        client_metadata
    }

    /// Builds request telemetry for unary API calls (e.g., Compact endpoint).
    fn build_request_telemetry(
        session_telemetry: &SessionTelemetry,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
        auth_env_telemetry: AuthEnvTelemetry,
    ) -> Arc<dyn RequestTelemetry> {
        let telemetry = Arc::new(ApiTelemetry::new(
            session_telemetry.clone(),
            auth_context,
            request_route_telemetry,
            auth_env_telemetry,
        ));
        let request_telemetry: Arc<dyn RequestTelemetry> = telemetry;
        request_telemetry
    }

    fn build_reasoning(
        model_info: &ModelInfo,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
    ) -> Option<Reasoning> {
        if model_info.supports_reasoning_summaries {
            Some(Reasoning {
                effort: effort.or(model_info.default_reasoning_level),
                summary: if summary == ReasoningSummaryConfig::None {
                    None
                } else {
                    Some(summary)
                },
            })
        } else {
            None
        }
    }

    /// Returns whether the Responses-over-WebSocket transport is active for this session.
    ///
    /// WebSocket use is controlled by provider capability and session-scoped fallback state.
    pub fn responses_websocket_enabled(&self) -> bool {
        if !self.state.provider.info().supports_websockets
            || self.state.disable_websockets.load(Ordering::Relaxed)
            || (*CODEX_RS_SSE_FIXTURE).is_some()
        {
            return false;
        }

        true
    }

    /// Returns auth + provider configuration resolved from the current session auth state.
    ///
    /// This centralizes setup used by both prewarm and normal request paths so they stay in
    /// lockstep when auth/provider resolution changes.
    async fn current_client_setup(&self) -> Result<CurrentClientSetup> {
        let auth = self.state.provider.auth().await;
        let api_provider = self.state.provider.api_provider().await?;
        let api_auth = self.state.provider.api_auth().await?;
        Ok(CurrentClientSetup {
            auth,
            api_provider,
            api_auth,
        })
    }

    /// Opens a websocket connection using the same header and telemetry wiring as normal turns.
    ///
    /// Both startup prewarm and in-turn `needs_new` reconnects call this path so handshake
    /// behavior remains consistent across both flows.
    #[allow(clippy::too_many_arguments)]
    async fn connect_websocket(
        &self,
        session_telemetry: &SessionTelemetry,
        api_provider: codex_api::Provider,
        api_auth: SharedAuthProvider,
        turn_state: Option<Arc<OnceLock<String>>>,
        turn_metadata_header: Option<&str>,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
    ) -> std::result::Result<ApiWebSocketConnection, ApiError> {
        let headers = self.build_websocket_headers(turn_state.as_ref(), turn_metadata_header);
        let websocket_telemetry = ModelClientSession::build_websocket_telemetry(
            session_telemetry,
            auth_context,
            request_route_telemetry,
            self.state.auth_env_telemetry.clone(),
        );
        let websocket_connect_timeout = self.state.provider.info().websocket_connect_timeout();
        let start = Instant::now();
        let result = match tokio::time::timeout(
            websocket_connect_timeout,
            ApiWebSocketResponsesClient::new(api_provider, api_auth).connect(
                headers,
                codex_login::default_client::default_headers(),
                turn_state,
                Some(websocket_telemetry),
            ),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(ApiError::Transport(TransportError::Timeout)),
        };
        let error_message = result.as_ref().err().map(telemetry_api_error_message);
        let response_debug = result
            .as_ref()
            .err()
            .map(extract_response_debug_context_from_api_error)
            .unwrap_or_default();
        let status = result.as_ref().err().and_then(api_error_http_status);
        session_telemetry.record_websocket_connect(
            start.elapsed(),
            status,
            error_message.as_deref(),
            auth_context.auth_header_attached,
            auth_context.auth_header_name,
            auth_context.retry_after_unauthorized,
            auth_context.recovery_mode,
            auth_context.recovery_phase,
            request_route_telemetry.endpoint,
            /*connection_reused*/ false,
            response_debug.request_id.as_deref(),
            response_debug.cf_ray.as_deref(),
            response_debug.auth_error.as_deref(),
            response_debug.auth_error_code.as_deref(),
        );
        emit_feedback_request_tags_with_auth_env(
            &FeedbackRequestTags {
                endpoint: request_route_telemetry.endpoint,
                auth_header_attached: auth_context.auth_header_attached,
                auth_header_name: auth_context.auth_header_name,
                auth_mode: auth_context.auth_mode,
                auth_retry_after_unauthorized: Some(auth_context.retry_after_unauthorized),
                auth_recovery_mode: auth_context.recovery_mode,
                auth_recovery_phase: auth_context.recovery_phase,
                auth_connection_reused: Some(false),
                auth_request_id: response_debug.request_id.as_deref(),
                auth_cf_ray: response_debug.cf_ray.as_deref(),
                auth_error: response_debug.auth_error.as_deref(),
                auth_error_code: response_debug.auth_error_code.as_deref(),
                auth_recovery_followup_success: auth_context
                    .retry_after_unauthorized
                    .then_some(result.is_ok()),
                auth_recovery_followup_status: auth_context
                    .retry_after_unauthorized
                    .then_some(status)
                    .flatten(),
            },
            &self.state.auth_env_telemetry,
        );
        result
    }

    /// Builds websocket handshake headers for both prewarm and turn-time reconnect.
    ///
    /// Callers should pass the current turn-state lock when available so sticky-routing state is
    /// replayed on reconnect within the same turn.
    fn build_websocket_headers(
        &self,
        turn_state: Option<&Arc<OnceLock<String>>>,
        turn_metadata_header: Option<&str>,
    ) -> ApiHeaderMap {
        let turn_metadata_header = parse_turn_metadata_header(turn_metadata_header);
        let conversation_id = self.state.conversation_id.to_string();
        let mut headers = build_responses_headers(
            self.state.beta_features_header.as_deref(),
            turn_state,
            turn_metadata_header.as_ref(),
        );
        if let Ok(header_value) = HeaderValue::from_str(&conversation_id) {
            headers.insert("x-client-request-id", header_value);
        }
        headers.extend(build_conversation_headers(Some(conversation_id)));
        headers.extend(self.build_responses_identity_headers());
        headers.insert(
            OPENAI_BETA_HEADER,
            HeaderValue::from_static(RESPONSES_WEBSOCKETS_V2_BETA_HEADER_VALUE),
        );
        if self.state.include_timing_metrics {
            headers.insert(
                X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER,
                HeaderValue::from_static("true"),
            );
        }
        headers
    }
}

impl Drop for ModelClientSession {
    fn drop(&mut self) {
        let websocket_session = std::mem::take(&mut self.websocket_session);
        self.client
            .store_cached_websocket_session(websocket_session);
    }
}

impl ModelClientSession {
    pub(crate) fn record_provider_wire_terminal(
        &self,
        status: &str,
        token_usage: Option<&TokenUsage>,
    ) -> Option<crate::provider_wire_trace::ProviderWireRequestIdentity> {
        self.client
            .state
            .provider_wire_trace
            .record_terminal(status, token_usage)
    }

    pub(crate) fn reset_websocket_session(&mut self) {
        self.websocket_session.connection = None;
        self.websocket_session.last_request = None;
        self.websocket_session.last_response_rx = None;
        self.websocket_session
            .set_connection_reused(/*connection_reused*/ false);
    }

    fn build_responses_request(
        &self,
        provider: &codex_api::Provider,
        prompt: &Prompt,
        model_info: &ModelInfo,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<ServiceTier>,
    ) -> Result<ResponsesApiRequest> {
        let instructions = &prompt.base_instructions.text;
        let input = prompt.get_formatted_input();
        let tools = create_tools_json_for_responses_api(&prompt.tools)?;
        let default_reasoning_effort = model_info.default_reasoning_level;
        let reasoning = if model_info.supports_reasoning_summaries {
            Some(Reasoning {
                effort: effort.or(default_reasoning_effort),
                summary: if summary == ReasoningSummaryConfig::None {
                    None
                } else {
                    Some(summary)
                },
            })
        } else if provider.wire_api == codex_api::WireApi::ChatCompletions
            && (effort.is_some() || default_reasoning_effort.is_some())
        {
            Some(Reasoning {
                effort: effort.or(default_reasoning_effort),
                summary: None,
            })
        } else {
            None
        };
        let include = if model_info.supports_reasoning_summaries && reasoning.is_some() {
            vec!["reasoning.encrypted_content".to_string()]
        } else {
            Vec::new()
        };
        let verbosity = if model_info.support_verbosity {
            self.client
                .state
                .model_verbosity
                .or(model_info.default_verbosity)
        } else {
            if self.client.state.model_verbosity.is_some() {
                warn!(
                    "model_verbosity is set but ignored as the model does not support verbosity: {}",
                    model_info.slug
                );
            }
            None
        };
        let text = create_text_param_for_request(
            verbosity,
            &prompt.output_schema,
            prompt.output_schema_strict,
        );
        let prompt_cache_key = Some(self.client.state.conversation_id.to_string());
        let request = ResponsesApiRequest {
            model: model_info.slug.clone(),
            instructions: instructions.clone(),
            input,
            tools,
            tool_choice: prompt.tool_choice.clone(),
            parallel_tool_calls: prompt.parallel_tool_calls,
            reasoning,
            store: provider.is_azure_responses_endpoint(),
            stream: true,
            include,
            service_tier: match service_tier {
                Some(ServiceTier::Fast) => Some("priority".to_string()),
                Some(service_tier) => Some(service_tier.to_string()),
                None => None,
            },
            prompt_cache_key,
            text,
            client_metadata: Some(HashMap::from([(
                X_CODEX_INSTALLATION_ID_HEADER.to_string(),
                self.client.state.installation_id.clone(),
            )])),
        };
        Ok(request)
    }

    #[allow(clippy::too_many_arguments)]
    /// Builds shared Responses API transport options and request-body options.
    ///
    /// Keeping option construction in one place ensures request-scoped headers are consistent
    /// regardless of transport choice.
    fn build_responses_options(
        &self,
        turn_metadata_header: Option<&str>,
        compression: Compression,
    ) -> ApiResponsesOptions {
        let turn_metadata_header = parse_turn_metadata_header(turn_metadata_header);
        let conversation_id = self.client.state.conversation_id.to_string();
        ApiResponsesOptions {
            conversation_id: Some(conversation_id),
            session_source: Some(self.client.state.session_source.clone()),
            extra_headers: {
                let mut headers = build_responses_headers(
                    self.client.state.beta_features_header.as_deref(),
                    Some(&self.turn_state),
                    turn_metadata_header.as_ref(),
                );
                headers.extend(self.client.build_responses_identity_headers());
                headers
            },
            compression,
            turn_state: Some(Arc::clone(&self.turn_state)),
        }
    }

    fn get_incremental_items(
        &self,
        request: &ResponsesApiRequest,
        last_response: Option<&LastResponse>,
        allow_empty_delta: bool,
    ) -> Option<Vec<ResponseItem>> {
        // Checks whether the current request is an incremental extension of the previous request.
        // We only reuse an incremental input delta when non-input request fields are unchanged and
        // `input` is a strict
        // extension of the previous known input. Server-returned output items are treated as part
        // of the baseline so we do not resend them.
        let previous_request = self.websocket_session.last_request.as_ref()?;
        let mut previous_without_input = previous_request.clone();
        previous_without_input.input.clear();
        let mut request_without_input = request.clone();
        request_without_input.input.clear();
        if previous_without_input != request_without_input {
            trace!(
                "incremental request failed, properties didn't match {previous_without_input:?} != {request_without_input:?}"
            );
            return None;
        }

        let mut baseline = previous_request.input.clone();
        if let Some(last_response) = last_response {
            baseline.extend(last_response.items_added.clone());
        }

        let baseline_len = baseline.len();
        if request.input.starts_with(&baseline)
            && (allow_empty_delta || baseline_len < request.input.len())
        {
            Some(request.input[baseline_len..].to_vec())
        } else {
            trace!("incremental request failed, items didn't match");
            None
        }
    }

    fn get_last_response(&mut self) -> Option<LastResponse> {
        self.websocket_session
            .last_response_rx
            .take()
            .and_then(|mut receiver| match receiver.try_recv() {
                Ok(last_response) => Some(last_response),
                Err(TryRecvError::Closed) | Err(TryRecvError::Empty) => None,
            })
    }

    fn prepare_websocket_request(
        &mut self,
        payload: ResponseCreateWsRequest,
        request: &ResponsesApiRequest,
    ) -> ResponsesWsRequest {
        let Some(last_response) = self.get_last_response() else {
            return ResponsesWsRequest::ResponseCreate(payload);
        };
        let Some(incremental_items) = self.get_incremental_items(
            request,
            Some(&last_response),
            /*allow_empty_delta*/ true,
        ) else {
            return ResponsesWsRequest::ResponseCreate(payload);
        };

        if last_response.response_id.is_empty() {
            trace!("incremental request failed, no previous response id");
            return ResponsesWsRequest::ResponseCreate(payload);
        }

        ResponsesWsRequest::ResponseCreate(ResponseCreateWsRequest {
            previous_response_id: Some(last_response.response_id),
            input: incremental_items,
            ..payload
        })
    }

    /// Opportunistically preconnects a websocket for this turn-scoped client session.
    ///
    /// This performs only connection setup; it never sends prompt payloads.
    pub async fn preconnect_websocket(
        &mut self,
        session_telemetry: &SessionTelemetry,
        _model_info: &ModelInfo,
    ) -> std::result::Result<(), ApiError> {
        if !self.client.responses_websocket_enabled() {
            return Ok(());
        }
        if self.websocket_session.connection.is_some() {
            return Ok(());
        }

        let client_setup = self.client.current_client_setup().await.map_err(|err| {
            ApiError::Stream(format!(
                "failed to build websocket prewarm client setup: {err}"
            ))
        })?;
        let auth_context = AuthRequestTelemetryContext::new(
            client_setup.auth.as_ref().map(CodexAuth::auth_mode),
            client_setup.api_auth.as_ref(),
            PendingUnauthorizedRetry::default(),
        );
        let connection = self
            .client
            .connect_websocket(
                session_telemetry,
                client_setup.api_provider,
                client_setup.api_auth,
                Some(Arc::clone(&self.turn_state)),
                /*turn_metadata_header*/ None,
                auth_context,
                RequestRouteTelemetry::for_endpoint(RESPONSES_ENDPOINT),
            )
            .await?;
        self.websocket_session.connection = Some(connection);
        self.websocket_session
            .set_connection_reused(/*connection_reused*/ false);
        Ok(())
    }
    /// Returns a websocket connection for this turn.
    #[instrument(
        name = "model_client.websocket_connection",
        level = "info",
        skip_all,
        fields(
            provider = %self.client.state.provider.info().name,
            wire_api = %self.client.state.provider.info().wire_api,
            transport = "responses_websocket",
            api.path = "responses",
            turn.has_metadata_header = params.turn_metadata_header.is_some()
        )
    )]
    async fn websocket_connection(
        &mut self,
        params: WebsocketConnectParams<'_>,
    ) -> std::result::Result<&ApiWebSocketConnection, ApiError> {
        let WebsocketConnectParams {
            session_telemetry,
            api_provider,
            api_auth,
            turn_metadata_header,
            options,
            auth_context,
            request_route_telemetry,
        } = params;
        let needs_new = match self.websocket_session.connection.as_ref() {
            Some(conn) => conn.is_closed().await,
            None => true,
        };

        if needs_new {
            self.websocket_session.last_request = None;
            self.websocket_session.last_response_rx = None;
            let turn_state = options
                .turn_state
                .clone()
                .unwrap_or_else(|| Arc::clone(&self.turn_state));
            let new_conn = match self
                .client
                .connect_websocket(
                    session_telemetry,
                    api_provider,
                    api_auth,
                    Some(turn_state),
                    turn_metadata_header,
                    auth_context,
                    request_route_telemetry,
                )
                .await
            {
                Ok(new_conn) => new_conn,
                Err(err) => {
                    if matches!(err, ApiError::Transport(TransportError::Timeout)) {
                        self.reset_websocket_session();
                    }
                    return Err(err);
                }
            };
            self.websocket_session.connection = Some(new_conn);
            self.websocket_session
                .set_connection_reused(/*connection_reused*/ false);
        } else {
            self.websocket_session
                .set_connection_reused(/*connection_reused*/ true);
        }

        self.websocket_session
            .connection
            .as_ref()
            .ok_or(ApiError::Stream(
                "websocket connection is unavailable".to_string(),
            ))
    }

    fn responses_request_compression(&self, auth: Option<&CodexAuth>) -> Compression {
        if self.client.state.enable_request_compression
            && auth.is_some_and(CodexAuth::uses_codex_backend)
            && self.client.state.provider.info().is_openai()
        {
            Compression::Zstd
        } else {
            Compression::None
        }
    }

    /// Streams a turn via the OpenAI Responses API.
    ///
    /// Handles SSE fixtures, reasoning summaries, verbosity, and the
    /// `text` controls used for output schemas.
    #[allow(clippy::too_many_arguments)]
    #[instrument(
        name = "model_client.stream_responses_api",
        level = "info",
        skip_all,
        fields(
            model = %model_info.slug,
            wire_api = %self.client.state.provider.info().wire_api,
            transport = "responses_http",
            http.method = "POST",
            api.path = "responses",
            turn.has_metadata_header = turn_metadata_header.is_some()
        )
    )]
    async fn stream_responses_api(
        &self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<ServiceTier>,
        turn_metadata_header: Option<&str>,
        inference_trace: &InferenceTraceContext,
        provider_request_budget: &ProviderRequestBudgetContext,
    ) -> Result<ResponseStream> {
        if let Some(path) = &*CODEX_RS_SSE_FIXTURE {
            warn!(path, "Streaming from fixture");
            let stream = codex_api::stream_from_fixture(
                path,
                self.client.state.provider.info().stream_idle_timeout(),
            )
            .map_err(map_api_error)?;
            let (stream, _last_request_rx) = map_response_stream(
                stream,
                session_telemetry.clone(),
                InferenceTraceAttempt::disabled(),
            );
            return Ok(stream);
        }

        let auth_manager = self.client.state.provider.auth_manager();
        let mut auth_recovery = auth_manager
            .as_ref()
            .map(AuthManager::unauthorized_recovery);
        let mut pending_retry = PendingUnauthorizedRetry::default();
        let wire_epoch_id = self.client.current_window_id();
        let wire_logical_request_id = self
            .client
            .state
            .provider_wire_trace
            .begin_logical_request(wire_epoch_id.as_str());
        let mut wire_attempt_seq = 0usize;
        loop {
            let client_setup = self.client.current_client_setup().await?;
            let transport = ReqwestTransport::new(build_reqwest_client());
            let request_auth_context = AuthRequestTelemetryContext::new(
                client_setup.auth.as_ref().map(CodexAuth::auth_mode),
                client_setup.api_auth.as_ref(),
                pending_retry,
            );
            let (request_telemetry, sse_telemetry) = Self::build_streaming_telemetry(
                session_telemetry,
                request_auth_context,
                RequestRouteTelemetry::for_endpoint(RESPONSES_ENDPOINT),
                self.client.state.auth_env_telemetry.clone(),
            );
            let compression = self.responses_request_compression(client_setup.auth.as_ref());
            let options = self.build_responses_options(turn_metadata_header, compression);

            let request = self.build_responses_request(
                &client_setup.api_provider,
                prompt,
                model_info,
                effort,
                summary,
                service_tier,
            )?;
            let inference_trace_attempt = inference_trace.start_attempt();
            inference_trace_attempt.record_started(&request);
            let provider_wire_api = client_setup.api_provider.wire_api;
            let client = ApiResponsesClient::new(
                transport,
                client_setup.api_provider,
                client_setup.api_auth,
            )
            .with_telemetry(Some(request_telemetry), Some(sse_telemetry));
            self.client
                .state
                .provider_request_hard_limit
                .claim(RESPONSES_ENDPOINT)?;
            let budget_dispatch = provider_request_budget.before_dispatch("responses_http")?;
            wire_attempt_seq += 1;
            let wire_value = self.client.state.provider_wire_trace.record_request(
                wire_epoch_id.as_str(),
                &wire_logical_request_id,
                wire_attempt_seq,
                "responses_http",
                provider_wire_api,
                &request,
                None,
            );
            if let Some(payload) =
                provider_payload_digest_for_wire_value(&wire_value, provider_wire_api)
            {
                budget_dispatch.record_provider_payload(payload);
            }
            let stream_result = client.stream_request(request, options).await;

            match stream_result {
                Ok(stream) => {
                    budget_dispatch.record_status("stream_opened");
                    let (stream, _) = map_response_stream(
                        stream,
                        session_telemetry.clone(),
                        inference_trace_attempt,
                    );
                    return Ok(stream);
                }
                Err(ApiError::Transport(
                    unauthorized_transport @ TransportError::Http { status, .. },
                )) if status == StatusCode::UNAUTHORIZED => {
                    self.record_provider_wire_terminal("retry_unauthorized", None);
                    budget_dispatch.record_status("retry_unauthorized");
                    inference_trace_attempt.record_failed(&unauthorized_transport);
                    pending_retry = PendingUnauthorizedRetry::from_recovery(
                        handle_unauthorized(
                            unauthorized_transport,
                            &mut auth_recovery,
                            session_telemetry,
                        )
                        .await?,
                    );
                    continue;
                }
                Err(err) => {
                    let err = map_api_error(err);
                    self.record_provider_wire_terminal("response_failed", None);
                    budget_dispatch.record_status("failed");
                    inference_trace_attempt.record_failed(&err);
                    return Err(err);
                }
            }
        }
    }

    /// Streams a turn via the Responses API over WebSocket transport.
    #[allow(clippy::too_many_arguments)]
    #[instrument(
        name = "model_client.stream_responses_websocket",
        level = "info",
        skip_all,
        fields(
            model = %model_info.slug,
            wire_api = %self.client.state.provider.info().wire_api,
            transport = "responses_websocket",
            api.path = "responses",
            turn.has_metadata_header = turn_metadata_header.is_some(),
            websocket.warmup = warmup
        )
    )]
    async fn stream_responses_websocket(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<ServiceTier>,
        turn_metadata_header: Option<&str>,
        warmup: bool,
        request_trace: Option<W3cTraceContext>,
        inference_trace: &InferenceTraceContext,
        provider_request_budget: &ProviderRequestBudgetContext,
    ) -> Result<WebsocketStreamOutcome> {
        let auth_manager = self.client.state.provider.auth_manager();

        let mut auth_recovery = auth_manager
            .as_ref()
            .map(AuthManager::unauthorized_recovery);
        let mut pending_retry = PendingUnauthorizedRetry::default();
        let wire_epoch_id = self.client.current_window_id();
        let wire_logical_request_id = self
            .client
            .state
            .provider_wire_trace
            .begin_logical_request(wire_epoch_id.as_str());
        let mut wire_attempt_seq = 0usize;
        loop {
            let client_setup = self.client.current_client_setup().await?;
            let request_auth_context = AuthRequestTelemetryContext::new(
                client_setup.auth.as_ref().map(CodexAuth::auth_mode),
                client_setup.api_auth.as_ref(),
                pending_retry,
            );
            let compression = self.responses_request_compression(client_setup.auth.as_ref());

            let options = self.build_responses_options(turn_metadata_header, compression);
            let request = self.build_responses_request(
                &client_setup.api_provider,
                prompt,
                model_info,
                effort,
                summary,
                service_tier,
            )?;
            let mut ws_payload = ResponseCreateWsRequest {
                client_metadata: response_create_client_metadata(
                    Some(self.client.build_ws_client_metadata(turn_metadata_header)),
                    request_trace.as_ref(),
                ),
                ..ResponseCreateWsRequest::from(&request)
            };
            if warmup {
                ws_payload.generate = Some(false);
            }
            let budget_dispatch = if warmup {
                ProviderRequestBudgetDispatch::disabled()
            } else {
                self.client
                    .state
                    .provider_request_hard_limit
                    .claim(RESPONSES_ENDPOINT)?;
                provider_request_budget.before_dispatch("responses_websocket")?
            };
            let provider_wire_api = client_setup.api_provider.wire_api;

            match self
                .websocket_connection(WebsocketConnectParams {
                    session_telemetry,
                    api_provider: client_setup.api_provider,
                    api_auth: client_setup.api_auth,
                    turn_metadata_header,
                    options: &options,
                    auth_context: request_auth_context,
                    request_route_telemetry: RequestRouteTelemetry::for_endpoint(
                        RESPONSES_ENDPOINT,
                    ),
                })
                .await
            {
                Ok(_) => {}
                Err(ApiError::Transport(TransportError::Http { status, .. }))
                    if status == StatusCode::UPGRADE_REQUIRED =>
                {
                    return Ok(WebsocketStreamOutcome::FallbackToHttp);
                }
                Err(ApiError::Transport(
                    unauthorized_transport @ TransportError::Http { status, .. },
                )) if status == StatusCode::UNAUTHORIZED => {
                    pending_retry = PendingUnauthorizedRetry::from_recovery(
                        handle_unauthorized(
                            unauthorized_transport,
                            &mut auth_recovery,
                            session_telemetry,
                        )
                        .await?,
                    );
                    continue;
                }
                Err(err) => return Err(map_api_error(err)),
            }

            let ws_request = self.prepare_websocket_request(ws_payload, &request);
            self.websocket_session.last_request = Some(request);
            let inference_trace_attempt = if warmup {
                // Prewarm sends `generate=false`; it is connection setup, not a
                // model inference attempt that should appear in rollout traces.
                InferenceTraceAttempt::disabled()
            } else {
                inference_trace.start_attempt()
            };
            inference_trace_attempt.record_started(&ws_request);
            let websocket_connection =
                self.websocket_session.connection.as_ref().ok_or_else(|| {
                    map_api_error(ApiError::Stream(
                        "websocket connection is unavailable".to_string(),
                    ))
                })?;
            if !warmup {
                wire_attempt_seq += 1;
                let wire_value =
                    serde_json::to_value(&ws_request).unwrap_or(serde_json::Value::Null);
                self.client.state.provider_wire_trace.record_request(
                    wire_epoch_id.as_str(),
                    &wire_logical_request_id,
                    wire_attempt_seq,
                    "responses_websocket",
                    provider_wire_api,
                    self.websocket_session
                        .last_request
                        .as_ref()
                        .expect("websocket request source is retained"),
                    Some(wire_value),
                );
            }
            if let Some(payload) = provider_payload_digest_for_wire(&ws_request, provider_wire_api)
            {
                budget_dispatch.record_provider_payload(payload);
            }
            let stream_result = websocket_connection
                .stream_request(ws_request, self.websocket_session.connection_reused())
                .await
                .map_err(|err| {
                    let err = map_api_error(err);
                    if !warmup {
                        self.record_provider_wire_terminal("response_failed", None);
                    }
                    budget_dispatch.record_status("failed");
                    inference_trace_attempt.record_failed(&err);
                    err
                })?;
            budget_dispatch.record_status("stream_opened");
            let (stream, last_request_rx) = map_response_stream(
                stream_result,
                session_telemetry.clone(),
                inference_trace_attempt,
            );
            self.websocket_session.last_response_rx = Some(last_request_rx);
            return Ok(WebsocketStreamOutcome::Stream(stream));
        }
    }

    /// Builds request and SSE telemetry for streaming API calls.
    fn build_streaming_telemetry(
        session_telemetry: &SessionTelemetry,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
        auth_env_telemetry: AuthEnvTelemetry,
    ) -> (Arc<dyn RequestTelemetry>, Arc<dyn SseTelemetry>) {
        let telemetry = Arc::new(ApiTelemetry::new(
            session_telemetry.clone(),
            auth_context,
            request_route_telemetry,
            auth_env_telemetry,
        ));
        let request_telemetry: Arc<dyn RequestTelemetry> = telemetry.clone();
        let sse_telemetry: Arc<dyn SseTelemetry> = telemetry;
        (request_telemetry, sse_telemetry)
    }

    /// Builds telemetry for the Responses API WebSocket transport.
    fn build_websocket_telemetry(
        session_telemetry: &SessionTelemetry,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
        auth_env_telemetry: AuthEnvTelemetry,
    ) -> Arc<dyn WebsocketTelemetry> {
        let telemetry = Arc::new(ApiTelemetry::new(
            session_telemetry.clone(),
            auth_context,
            request_route_telemetry,
            auth_env_telemetry,
        ));
        let websocket_telemetry: Arc<dyn WebsocketTelemetry> = telemetry;
        websocket_telemetry
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn prewarm_websocket(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<ServiceTier>,
        turn_metadata_header: Option<&str>,
    ) -> Result<()> {
        if !self.client.responses_websocket_enabled() {
            return Ok(());
        }
        if self.websocket_session.last_request.is_some() {
            return Ok(());
        }

        let disabled_trace = InferenceTraceContext::disabled();
        let disabled_budget = ProviderRequestBudgetContext::disabled();
        match self
            .stream_responses_websocket(
                prompt,
                model_info,
                session_telemetry,
                effort,
                summary,
                service_tier,
                turn_metadata_header,
                /*warmup*/ true,
                current_span_w3c_trace_context(),
                &disabled_trace,
                &disabled_budget,
            )
            .await
        {
            Ok(WebsocketStreamOutcome::Stream(mut stream)) => {
                // Wait for the v2 warmup request to complete before sending the first turn request.
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(ResponseEvent::Completed { .. }) => break,
                        Err(err) => return Err(err),
                        _ => {}
                    }
                }
                Ok(())
            }
            Ok(WebsocketStreamOutcome::FallbackToHttp) => {
                self.try_switch_fallback_transport(session_telemetry, model_info);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Streams a single model request within the current turn.
    ///
    /// The caller is responsible for passing per-turn settings explicitly (model selection,
    /// reasoning settings, telemetry context, and turn metadata). This method will prefer the
    /// Responses WebSocket transport when the provider supports it and it remains healthy, and will
    /// fall back to the HTTP Responses API transport otherwise. The trace context may be enabled or
    /// disabled, but is always explicit so transport paths do not need separate trace/no-trace
    /// branches.
    pub async fn stream(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<ServiceTier>,
        turn_metadata_header: Option<&str>,
        inference_trace: &InferenceTraceContext,
    ) -> Result<ResponseStream> {
        let provider_request_budget = ProviderRequestBudgetContext::disabled();
        self.stream_with_provider_request_budget(
            prompt,
            model_info,
            session_telemetry,
            effort,
            summary,
            service_tier,
            turn_metadata_header,
            inference_trace,
            &provider_request_budget,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn stream_with_provider_request_budget(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<ServiceTier>,
        turn_metadata_header: Option<&str>,
        inference_trace: &InferenceTraceContext,
        provider_request_budget: &ProviderRequestBudgetContext,
    ) -> Result<ResponseStream> {
        let wire_api = self.client.state.provider.info().wire_api;
        match wire_api {
            WireApi::Responses => {
                if self.client.responses_websocket_enabled() {
                    let request_trace = current_span_w3c_trace_context();
                    match self
                        .stream_responses_websocket(
                            prompt,
                            model_info,
                            session_telemetry,
                            effort,
                            summary,
                            service_tier,
                            turn_metadata_header,
                            /*warmup*/ false,
                            request_trace,
                            inference_trace,
                            provider_request_budget,
                        )
                        .await?
                    {
                        WebsocketStreamOutcome::Stream(stream) => return Ok(stream),
                        WebsocketStreamOutcome::FallbackToHttp => {
                            self.try_switch_fallback_transport(session_telemetry, model_info);
                        }
                    }
                }

                self.stream_responses_api(
                    prompt,
                    model_info,
                    session_telemetry,
                    effort,
                    summary,
                    service_tier,
                    turn_metadata_header,
                    inference_trace,
                    provider_request_budget,
                )
                .await
            }
            WireApi::ChatCompletions => {
                self.stream_responses_api(
                    prompt,
                    model_info,
                    session_telemetry,
                    effort,
                    summary,
                    service_tier,
                    turn_metadata_header,
                    inference_trace,
                    provider_request_budget,
                )
                .await
            }
        }
    }

    /// Permanently disables WebSockets for this Whale session and resets WebSocket state.
    ///
    /// This is used after exhausting the provider retry budget, to force subsequent requests onto
    /// the HTTP transport.
    ///
    /// Returns `true` if this call activated fallback, or `false` if fallback was already active.
    pub(crate) fn try_switch_fallback_transport(
        &mut self,
        session_telemetry: &SessionTelemetry,
        model_info: &ModelInfo,
    ) -> bool {
        let activated = self
            .client
            .force_http_fallback(session_telemetry, model_info);
        self.websocket_session = WebsocketSession::default();
        activated
    }
}

/// Parses per-turn metadata into an HTTP header value.
///
/// Invalid values are treated as absent so callers can compare and propagate
/// metadata with the same sanitization path used when constructing headers.
fn parse_turn_metadata_header(turn_metadata_header: Option<&str>) -> Option<HeaderValue> {
    turn_metadata_header.and_then(|value| HeaderValue::from_str(value).ok())
}

/// Builds the extra headers attached to Responses API requests.
///
/// These headers implement Codex-specific conventions:
///
/// - `x-codex-beta-features`: comma-separated beta feature keys enabled for the session.
/// - `x-codex-turn-state`: sticky routing token captured earlier in the turn.
/// - `x-codex-turn-metadata`: optional per-turn metadata for observability.
fn build_responses_headers(
    beta_features_header: Option<&str>,
    turn_state: Option<&Arc<OnceLock<String>>>,
    turn_metadata_header: Option<&HeaderValue>,
) -> ApiHeaderMap {
    let mut headers = ApiHeaderMap::new();
    if let Some(value) = beta_features_header
        && !value.is_empty()
        && let Ok(header_value) = HeaderValue::from_str(value)
    {
        headers.insert("x-codex-beta-features", header_value);
    }
    if let Some(turn_state) = turn_state
        && let Some(state) = turn_state.get()
        && let Ok(header_value) = HeaderValue::from_str(state)
    {
        headers.insert(X_CODEX_TURN_STATE_HEADER, header_value);
    }
    if let Some(header_value) = turn_metadata_header {
        headers.insert(X_CODEX_TURN_METADATA_HEADER, header_value.clone());
    }
    headers
}

fn subagent_header_value(session_source: &SessionSource) -> Option<String> {
    let SessionSource::SubAgent(subagent_source) = session_source else {
        return None;
    };
    match subagent_source {
        SubAgentSource::Review => Some("review".to_string()),
        SubAgentSource::Compact => Some("compact".to_string()),
        SubAgentSource::MemoryConsolidation => Some("memory_consolidation".to_string()),
        SubAgentSource::ThreadSpawn { .. } => Some("collab_spawn".to_string()),
        SubAgentSource::Other(label) => Some(label.clone()),
    }
}

fn parent_thread_id_header_value(session_source: &SessionSource) -> Option<String> {
    match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) => Some(parent_thread_id.to_string()),
        SessionSource::Cli
        | SessionSource::VSCode
        | SessionSource::Exec
        | SessionSource::Mcp
        | SessionSource::Custom(_)
        | SessionSource::SubAgent(_)
        | SessionSource::Unknown => None,
    }
}

fn map_response_stream<S>(
    api_stream: S,
    session_telemetry: SessionTelemetry,
    inference_trace_attempt: InferenceTraceAttempt,
) -> (ResponseStream, oneshot::Receiver<LastResponse>)
where
    S: futures::Stream<Item = std::result::Result<ResponseEvent, ApiError>>
        + Unpin
        + Send
        + 'static,
{
    let (tx_event, rx_event) = mpsc::channel::<Result<ResponseEvent>>(1600);
    let (tx_last_response, rx_last_response) = oneshot::channel::<LastResponse>();

    tokio::spawn(async move {
        let mut logged_error = false;
        let mut tx_last_response = Some(tx_last_response);
        let mut items_added: Vec<ResponseItem> = Vec::new();
        let mut api_stream = api_stream;
        while let Some(event) = api_stream.next().await {
            match event {
                Ok(ResponseEvent::OutputItemDone(item)) => {
                    items_added.push(item.clone());
                    if tx_event
                        .send(Ok(ResponseEvent::OutputItemDone(item)))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Ok(ResponseEvent::Completed {
                    response_id,
                    token_usage,
                    end_turn,
                }) => {
                    if let Some(usage) = &token_usage {
                        session_telemetry.sse_event_completed(
                            usage.input_tokens,
                            usage.output_tokens,
                            Some(usage.cached_input_tokens),
                            Some(usage.reasoning_output_tokens),
                            usage.total_tokens,
                        );
                    }
                    inference_trace_attempt.record_completed(
                        &response_id,
                        &token_usage,
                        &items_added,
                    );
                    if let Some(sender) = tx_last_response.take() {
                        let _ = sender.send(LastResponse {
                            response_id: response_id.clone(),
                            items_added: std::mem::take(&mut items_added),
                        });
                    }
                    if tx_event
                        .send(Ok(ResponseEvent::Completed {
                            response_id,
                            token_usage,
                            end_turn,
                        }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Ok(event) => {
                    if tx_event.send(Ok(event)).await.is_err() {
                        return;
                    }
                }
                Err(err) => {
                    let mapped = map_api_error(err);
                    inference_trace_attempt.record_failed(&mapped);
                    if !logged_error {
                        session_telemetry.see_event_completed_failed(&mapped);
                        logged_error = true;
                    }
                    if tx_event.send(Err(mapped)).await.is_err() {
                        return;
                    }
                }
            }
        }
    });

    (ResponseStream { rx_event }, rx_last_response)
}

/// Handles a 401 response by optionally refreshing ChatGPT tokens once.
///
/// When refresh succeeds, the caller should retry the API call; otherwise
/// the mapped `CodexErr` is returned to the caller.
#[derive(Clone, Copy, Debug)]
struct UnauthorizedRecoveryExecution {
    mode: &'static str,
    phase: &'static str,
}

#[derive(Clone, Copy, Debug, Default)]
struct PendingUnauthorizedRetry {
    retry_after_unauthorized: bool,
    recovery_mode: Option<&'static str>,
    recovery_phase: Option<&'static str>,
}

impl PendingUnauthorizedRetry {
    fn from_recovery(recovery: UnauthorizedRecoveryExecution) -> Self {
        Self {
            retry_after_unauthorized: true,
            recovery_mode: Some(recovery.mode),
            recovery_phase: Some(recovery.phase),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AuthRequestTelemetryContext {
    auth_mode: Option<&'static str>,
    auth_header_attached: bool,
    auth_header_name: Option<&'static str>,
    retry_after_unauthorized: bool,
    recovery_mode: Option<&'static str>,
    recovery_phase: Option<&'static str>,
}

impl AuthRequestTelemetryContext {
    fn new(
        auth_mode: Option<AuthMode>,
        api_auth: &dyn AuthProvider,
        retry: PendingUnauthorizedRetry,
    ) -> Self {
        let auth_telemetry = auth_header_telemetry(api_auth);
        Self {
            auth_mode: auth_mode.map(|mode| match mode {
                AuthMode::ApiKey => "ApiKey",
                AuthMode::Chatgpt | AuthMode::ChatgptAuthTokens | AuthMode::AgentIdentity => {
                    "Chatgpt"
                }
            }),
            auth_header_attached: auth_telemetry.attached,
            auth_header_name: auth_telemetry.name,
            retry_after_unauthorized: retry.retry_after_unauthorized,
            recovery_mode: retry.recovery_mode,
            recovery_phase: retry.recovery_phase,
        }
    }
}

struct WebsocketConnectParams<'a> {
    session_telemetry: &'a SessionTelemetry,
    api_provider: codex_api::Provider,
    api_auth: SharedAuthProvider,
    turn_metadata_header: Option<&'a str>,
    options: &'a ApiResponsesOptions,
    auth_context: AuthRequestTelemetryContext,
    request_route_telemetry: RequestRouteTelemetry,
}

async fn handle_unauthorized(
    transport: TransportError,
    auth_recovery: &mut Option<UnauthorizedRecovery>,
    session_telemetry: &SessionTelemetry,
) -> Result<UnauthorizedRecoveryExecution> {
    let debug = extract_response_debug_context(&transport);
    if let Some(recovery) = auth_recovery
        && recovery.has_next()
    {
        let mode = recovery.mode_name();
        let phase = recovery.step_name();
        return match recovery.next().await {
            Ok(step_result) => {
                session_telemetry.record_auth_recovery(
                    mode,
                    phase,
                    "recovery_succeeded",
                    debug.request_id.as_deref(),
                    debug.cf_ray.as_deref(),
                    debug.auth_error.as_deref(),
                    debug.auth_error_code.as_deref(),
                    /*recovery_reason*/ None,
                    step_result.auth_state_changed(),
                );
                emit_feedback_auth_recovery_tags(
                    mode,
                    phase,
                    "recovery_succeeded",
                    debug.request_id.as_deref(),
                    debug.cf_ray.as_deref(),
                    debug.auth_error.as_deref(),
                    debug.auth_error_code.as_deref(),
                );
                Ok(UnauthorizedRecoveryExecution { mode, phase })
            }
            Err(RefreshTokenError::Permanent(failed)) => {
                session_telemetry.record_auth_recovery(
                    mode,
                    phase,
                    "recovery_failed_permanent",
                    debug.request_id.as_deref(),
                    debug.cf_ray.as_deref(),
                    debug.auth_error.as_deref(),
                    debug.auth_error_code.as_deref(),
                    /*recovery_reason*/ None,
                    /*auth_state_changed*/ None,
                );
                emit_feedback_auth_recovery_tags(
                    mode,
                    phase,
                    "recovery_failed_permanent",
                    debug.request_id.as_deref(),
                    debug.cf_ray.as_deref(),
                    debug.auth_error.as_deref(),
                    debug.auth_error_code.as_deref(),
                );
                Err(CodexErr::RefreshTokenFailed(failed))
            }
            Err(RefreshTokenError::Transient(other)) => {
                session_telemetry.record_auth_recovery(
                    mode,
                    phase,
                    "recovery_failed_transient",
                    debug.request_id.as_deref(),
                    debug.cf_ray.as_deref(),
                    debug.auth_error.as_deref(),
                    debug.auth_error_code.as_deref(),
                    /*recovery_reason*/ None,
                    /*auth_state_changed*/ None,
                );
                emit_feedback_auth_recovery_tags(
                    mode,
                    phase,
                    "recovery_failed_transient",
                    debug.request_id.as_deref(),
                    debug.cf_ray.as_deref(),
                    debug.auth_error.as_deref(),
                    debug.auth_error_code.as_deref(),
                );
                Err(CodexErr::Io(other))
            }
        };
    }

    let (mode, phase, recovery_reason) = match auth_recovery.as_ref() {
        Some(recovery) => (
            recovery.mode_name(),
            recovery.step_name(),
            Some(recovery.unavailable_reason()),
        ),
        None => ("none", "none", Some("auth_manager_missing")),
    };
    session_telemetry.record_auth_recovery(
        mode,
        phase,
        "recovery_not_run",
        debug.request_id.as_deref(),
        debug.cf_ray.as_deref(),
        debug.auth_error.as_deref(),
        debug.auth_error_code.as_deref(),
        recovery_reason,
        /*auth_state_changed*/ None,
    );
    emit_feedback_auth_recovery_tags(
        mode,
        phase,
        "recovery_not_run",
        debug.request_id.as_deref(),
        debug.cf_ray.as_deref(),
        debug.auth_error.as_deref(),
        debug.auth_error_code.as_deref(),
    );

    Err(map_api_error(ApiError::Transport(transport)))
}

fn api_error_http_status(error: &ApiError) -> Option<u16> {
    match error {
        ApiError::Transport(TransportError::Http { status, .. }) => Some(status.as_u16()),
        _ => None,
    }
}

struct ApiTelemetry {
    session_telemetry: SessionTelemetry,
    auth_context: AuthRequestTelemetryContext,
    request_route_telemetry: RequestRouteTelemetry,
    auth_env_telemetry: AuthEnvTelemetry,
}

impl ApiTelemetry {
    fn new(
        session_telemetry: SessionTelemetry,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
        auth_env_telemetry: AuthEnvTelemetry,
    ) -> Self {
        Self {
            session_telemetry,
            auth_context,
            request_route_telemetry,
            auth_env_telemetry,
        }
    }
}

impl RequestTelemetry for ApiTelemetry {
    fn on_request(
        &self,
        attempt: u64,
        status: Option<HttpStatusCode>,
        error: Option<&TransportError>,
        duration: Duration,
    ) {
        let error_message = error.map(telemetry_transport_error_message);
        let status = status.map(|s| s.as_u16());
        let debug = error
            .map(extract_response_debug_context)
            .unwrap_or_default();
        self.session_telemetry.record_api_request(
            attempt,
            status,
            error_message.as_deref(),
            duration,
            self.auth_context.auth_header_attached,
            self.auth_context.auth_header_name,
            self.auth_context.retry_after_unauthorized,
            self.auth_context.recovery_mode,
            self.auth_context.recovery_phase,
            self.request_route_telemetry.endpoint,
            debug.request_id.as_deref(),
            debug.cf_ray.as_deref(),
            debug.auth_error.as_deref(),
            debug.auth_error_code.as_deref(),
        );
        emit_feedback_request_tags_with_auth_env(
            &FeedbackRequestTags {
                endpoint: self.request_route_telemetry.endpoint,
                auth_header_attached: self.auth_context.auth_header_attached,
                auth_header_name: self.auth_context.auth_header_name,
                auth_mode: self.auth_context.auth_mode,
                auth_retry_after_unauthorized: Some(self.auth_context.retry_after_unauthorized),
                auth_recovery_mode: self.auth_context.recovery_mode,
                auth_recovery_phase: self.auth_context.recovery_phase,
                auth_connection_reused: None,
                auth_request_id: debug.request_id.as_deref(),
                auth_cf_ray: debug.cf_ray.as_deref(),
                auth_error: debug.auth_error.as_deref(),
                auth_error_code: debug.auth_error_code.as_deref(),
                auth_recovery_followup_success: self
                    .auth_context
                    .retry_after_unauthorized
                    .then_some(error.is_none()),
                auth_recovery_followup_status: self
                    .auth_context
                    .retry_after_unauthorized
                    .then_some(status)
                    .flatten(),
            },
            &self.auth_env_telemetry,
        );
    }
}

impl SseTelemetry for ApiTelemetry {
    fn on_sse_poll(
        &self,
        result: &std::result::Result<
            Option<std::result::Result<Event, EventStreamError<TransportError>>>,
            tokio::time::error::Elapsed,
        >,
        duration: Duration,
    ) {
        self.session_telemetry.log_sse_event(result, duration);
    }
}

impl WebsocketTelemetry for ApiTelemetry {
    fn on_ws_request(&self, duration: Duration, error: Option<&ApiError>, connection_reused: bool) {
        let error_message = error.map(telemetry_api_error_message);
        let status = error.and_then(api_error_http_status);
        let debug = error
            .map(extract_response_debug_context_from_api_error)
            .unwrap_or_default();
        self.session_telemetry.record_websocket_request(
            duration,
            error_message.as_deref(),
            connection_reused,
        );
        emit_feedback_request_tags_with_auth_env(
            &FeedbackRequestTags {
                endpoint: self.request_route_telemetry.endpoint,
                auth_header_attached: self.auth_context.auth_header_attached,
                auth_header_name: self.auth_context.auth_header_name,
                auth_mode: self.auth_context.auth_mode,
                auth_retry_after_unauthorized: Some(self.auth_context.retry_after_unauthorized),
                auth_recovery_mode: self.auth_context.recovery_mode,
                auth_recovery_phase: self.auth_context.recovery_phase,
                auth_connection_reused: Some(connection_reused),
                auth_request_id: debug.request_id.as_deref(),
                auth_cf_ray: debug.cf_ray.as_deref(),
                auth_error: debug.auth_error.as_deref(),
                auth_error_code: debug.auth_error_code.as_deref(),
                auth_recovery_followup_success: self
                    .auth_context
                    .retry_after_unauthorized
                    .then_some(error.is_none()),
                auth_recovery_followup_status: self
                    .auth_context
                    .retry_after_unauthorized
                    .then_some(status)
                    .flatten(),
            },
            &self.auth_env_telemetry,
        );
    }

    fn on_ws_event(
        &self,
        result: &std::result::Result<Option<std::result::Result<Message, Error>>, ApiError>,
        duration: Duration,
    ) {
        self.session_telemetry
            .record_websocket_event(result, duration);
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
