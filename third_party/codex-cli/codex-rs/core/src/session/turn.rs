use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::SkillInjections;
use crate::SkillLoadOutcome;
use crate::action_map::ActionClass;
use crate::action_map::ActionMapProviderResponseActionabilityInput;
use crate::action_map::compile_taskspace_agent_context_text;
use crate::build_skill_injections;
use crate::client::ModelClientSession;
use crate::client::ProviderRequestAttribution;
use crate::client::ProviderRequestBudgetContext;
use crate::client::ProviderRequestBudgetLimits;
use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::collect_env_var_dependencies;
use crate::collect_explicit_skill_mentions;
use crate::compact::CompactStrategy;
use crate::compact::InitialContextInjection;
use crate::compact::collect_user_messages;
use crate::compact::compact_strategy;
use crate::compact::run_inline_auto_compact_task;
use crate::compact_remote::run_inline_remote_auto_compact_task;
use crate::connectors;
use crate::context::ContextualUserFragment;
use crate::feedback_tags;
use crate::hook_runtime::PendingInputHookDisposition;
use crate::hook_runtime::emit_hook_completed_events;
use crate::hook_runtime::inspect_pending_input;
use crate::hook_runtime::record_additional_contexts;
use crate::hook_runtime::record_pending_input;
use crate::hook_runtime::run_pending_session_start_hooks;
use crate::hook_runtime::run_user_prompt_submit_hooks;
use crate::injection::ToolMentionKind;
use crate::injection::app_id_from_path;
use crate::injection::tool_kind_for_path;
use crate::mcp_skill_dependencies::maybe_prompt_and_install_mcp_dependencies;
use crate::mcp_tool_exposure::build_mcp_tool_exposure;
use crate::mentions::build_connector_slug_counts;
use crate::mentions::build_skill_name_counts;
use crate::mentions::collect_explicit_app_ids;
use crate::mentions::collect_explicit_plugin_mentions;
use crate::mentions::collect_tool_mentions_from_messages;
use crate::parse_turn_item;
use crate::plugins::build_plugin_injections;
use crate::resolve_skill_dependencies_for_turn;
use crate::session::PreviousTurnSettings;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::stream_events_utils::HandleOutputCtx;
use crate::stream_events_utils::handle_non_tool_response_item;
use crate::stream_events_utils::handle_output_item_done;
use crate::stream_events_utils::last_assistant_message_from_item;
use crate::stream_events_utils::mark_thread_memory_mode_polluted_if_external_context;
use crate::stream_events_utils::raw_assistant_output_text_from_item;
use crate::stream_events_utils::record_completed_response_item;
use crate::tools::ToolRouter;
use crate::tools::append_taskspace_tool_tail_sentinels;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::registry::ToolArgumentDiffConsumer;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouterParams;
use crate::turn_diff_tracker::TurnDiffTracker;
use crate::turn_timing::record_turn_ttft_metric;
use crate::unavailable_tool::collect_unavailable_called_tools;
use crate::util::backoff;
use crate::util::error_or_panic;
use codex_analytics::AppInvocation;
use codex_analytics::CompactionPhase;
use codex_analytics::CompactionReason;
use codex_analytics::InvocationType;
use codex_analytics::TurnResolvedConfigFact;
use codex_analytics::build_track_events_context;
use codex_async_utils::OrCancelExt;
use codex_features::Feature;
use codex_hooks::HookEvent;
use codex_hooks::HookEventAfterAgent;
use codex_hooks::HookPayload;
use codex_hooks::HookResult;
use codex_protocol::config_types::ModeKind;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::items::PlanItem;
use codex_protocol::items::TurnItem;
use codex_protocol::items::UserMessageItem;
use codex_protocol::items::build_hook_prompt_message;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::AgentMessageContentDeltaEvent;
use codex_protocol::protocol::AgentReasoningSectionBreakEvent;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::CodexErrorInfo;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::PlanDeltaEvent;
use codex_protocol::protocol::ReasoningContentDeltaEvent;
use codex_protocol::protocol::ReasoningRawContentDeltaEvent;
use codex_protocol::protocol::TurnDiffEvent;
use codex_protocol::protocol::WarningEvent;
use codex_protocol::user_input::UserInput;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_tools::filter_tool_suggest_discoverable_tools_for_client;
use codex_utils_stream_parser::AssistantTextChunk;
use codex_utils_stream_parser::AssistantTextStreamParser;
use codex_utils_stream_parser::ProposedPlanSegment;
use codex_utils_stream_parser::extract_proposed_plan_text;
use codex_utils_stream_parser::strip_citations;
use futures::future::BoxFuture;
use futures::prelude::*;
use futures::stream::FuturesOrdered;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use tracing::error;
use tracing::field;
use tracing::info;
use tracing::instrument;
use tracing::trace;
use tracing::trace_span;
use tracing::warn;

const TASKSPACE_ACTIVE_PROFILE_MARKER: &str = "TaskSpace v0.0.5 active compact profile is enabled.";
const TASKSPACE_ACTIVE_PROJECTION_MARKER: &str = "ContextProjectionV1 active replacement:";
const TASKSPACE_SHADOW_PROJECTION_MARKER: &str =
    "ContextProjectionV1 shadow (not active replacement):";
const TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_ITEMS: usize = 3;
const TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_CHARS: usize = 2400;
const TASKSPACE_DEEPSEEK_CACHE_ANCHOR_LINES: usize = 4200;
const TASKSPACE_IMPLEMENT_PROGRESS_BEFORE_EDIT_HINT: usize = 10;
const TASKSPACE_NO_ACTION_RECOVERY_MARKER: &str = "TaskSpaceNoActionRecoveryV1:";
const TASKSPACE_NO_ACTION_RECOVERY_HARD_STOP_MARKER: &str = "TaskSpaceNoActionRecoveryHardStopV1:";
const TASKSPACE_GATE_RECOVERY_MARKER: &str = "TaskSpaceGateRecoveryV1:";
const TASKSPACE_FORCED_INSPECT_TRANSITION_MARKER: &str =
    "TaskSpaceForcedInspectTransitionRecoveryV1:";
const TASKSPACE_FORCED_IMPLEMENT_TRANSITION_MARKER: &str =
    "TaskSpaceForcedImplementTransitionRecoveryV1:";
const TASKSPACE_FORCED_VALIDATION_CLOSEOUT_MARKER: &str =
    "TaskSpaceForcedValidationCloseoutRecoveryV1:";
const TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER: &str = "TaskSpaceImplementNeedsEditRecoveryV1:";
const TASKSPACE_IMPLEMENT_NEEDS_EDIT_HARD_STOP_MARKER: &str =
    "TaskSpaceImplementationNeedsEditHardStopV1:";
const TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER: &str =
    "TaskSpaceValidationReworkDuplicateReadRecoveryV1:";
const TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_HARD_STOP_MARKER: &str =
    "TaskSpaceValidationReworkDuplicateReadHardStopV1:";
const TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER: &str =
    "TaskSpaceValidationReworkPatchOnlyRecoveryV1:";
const TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_HARD_STOP_MARKER: &str =
    "TaskSpaceValidationReworkPatchOnlyHardStopV1:";
const TASKSPACE_EDIT_FAILURE_MARKER: &str = "TaskSpaceEditFailureRecoveryV1:";
const TASKSPACE_APPLY_PATCH_FORMAT_MARKER: &str = "TaskSpaceApplyPatchFormatRecoveryV1:";
const TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER: &str =
    "TaskSpaceApplyPatchMissingTargetRecoveryV1:";
const TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER: &str =
    "TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1:";
const TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER: &str = "TaskSpaceApplyPatchNativeHunkRecoveryV1:";
const TASKSPACE_APPLY_PATCH_RECOVERY_HARD_STOP_MARKER: &str =
    "TaskSpaceApplyPatchRecoveryHardStopV1:";
const TASKSPACE_PATCH_INTENT_FORMAT_MARKER: &str = "TaskSpacePatchIntentFormatRecoveryV1:";
const TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER: &str = "TaskSpaceValidationInfraRecoveryV1:";
const TASKSPACE_VALIDATION_NEEDS_TEST_MARKER: &str = "TaskSpaceValidationNeedsTestRecoveryV1:";
const TASKSPACE_PROVIDER_BUDGET_HARD_STOP_MARKER: &str = "TaskSpaceProviderBudgetHardStopV1:";
const TASKSPACE_TOOL_FEEDBACK_MARKER: &str = "TaskSpaceToolFeedbackV1:";
const TASKSPACE_INSPECT_BOOTSTRAP_CALL_ID: &str = "taskspace-inspect-bootstrap-rg-files";
const TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_CALL_ID: &str =
    "taskspace-inspect-bootstrap-repeated-blocked-read";
const TASKSPACE_MISSING_FACT_SOURCE_BOOTSTRAP_CALL_ID: &str =
    "taskspace-inspect-bootstrap-missing-fact-source";
const TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_COMMAND_WINDOWS: &str = "rg --files -g '*.py' -g '*.md' -g '*.txt' -g '*.json' -g '*.csv' -g '*.yaml' -g '*.yml' | Select-Object -First 12 | ForEach-Object { Write-Output ('===== ' + $_); Get-Content -LiteralPath $_ -TotalCount 120 }";
const TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_COMMAND_UNIX: &str = "rg --files -g '*.py' -g '*.md' -g '*.txt' -g '*.json' -g '*.csv' -g '*.yaml' -g '*.yml' | head -n 12 | while IFS= read -r path; do printf '===== %s\\n' \"$path\"; sed -n '1,120p' -- \"$path\"; done";
const TASKSPACE_INSPECT_TEST_BOOTSTRAP_CALL_ID: &str = "taskspace-inspect-bootstrap-pytest";
const TASKSPACE_VALIDATION_REQUIRED_COMMAND_BOOTSTRAP_CALL_ID: &str =
    "taskspace-validation-required-command-bootstrap";
const TASKSPACE_ACTIVE_MAX_RAW_TOOL_OUTPUT_CHARS: usize = 12_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskspaceProviderResponseActionability {
    Actionable,
    NoActionFollowUp,
    EmptyFollowUp,
    FinalCandidate,
    FinalRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskspaceProviderToolVisibility {
    All,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskspaceProviderTransportMode {
    NativeTools,
    CacheOptimizedActionContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskspaceProviderTransportBudgetLimits {
    max_requests: usize,
    max_model_requests_per_node: usize,
    post_budget_grace_requests: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct TaskSpaceActionV1 {
    schema_version: String,
    action: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    args: serde_json::Value,
    #[serde(default)]
    rationale: Option<String>,
}

impl TaskspaceProviderResponseActionability {
    fn as_str(self) -> &'static str {
        match self {
            Self::Actionable => "actionable",
            Self::NoActionFollowUp => "no_action_follow_up",
            Self::EmptyFollowUp => "empty_follow_up",
            Self::FinalCandidate => "final_candidate",
            Self::FinalRejected => "final_rejected",
        }
    }

    fn needs_recovery(self) -> bool {
        matches!(
            self,
            Self::NoActionFollowUp | Self::EmptyFollowUp | Self::FinalRejected
        )
    }
}

/// Takes a user message as input and runs a loop where, at each sampling request, the model
/// replies with either:
///
/// - requested function calls
/// - an assistant message
///
/// While it is possible for the model to return multiple of these items in a
/// single sampling request, in practice, we generally one item per sampling request:
///
/// - If the model requests a function call, we execute it and send the output
///   back to the model in the next sampling request.
/// - If the model sends only an assistant message, we record it in the
///   conversation history and consider the turn complete.
///
#[expect(
    clippy::await_holding_invalid_type,
    reason = "turn execution must keep active-turn state transitions atomic"
)]
pub(crate) async fn run_turn(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    input: Vec<UserInput>,
    prewarmed_client_session: Option<ModelClientSession>,
    cancellation_token: CancellationToken,
) -> Option<String> {
    if input.is_empty() && !sess.has_pending_input().await {
        return None;
    }

    let model_info = turn_context.model_info.clone();
    let auto_compact_limit = model_info.auto_compact_token_limit().unwrap_or(i64::MAX);
    let mut prewarmed_client_session = prewarmed_client_session;
    // TODO(ccunningham): Pre-turn compaction runs before context updates and the
    // new user message are recorded. Estimate pending incoming items (context
    // diffs/full reinjection + user input) and trigger compaction preemptively
    // when they would push the thread over the compaction threshold.
    let pre_sampling_compacted = match run_pre_sampling_compact(&sess, &turn_context).await {
        Ok(pre_sampling_compacted) => pre_sampling_compacted,
        Err(_) => {
            error!("Failed to run pre-sampling compact");
            return None;
        }
    };
    if pre_sampling_compacted && let Some(mut client_session) = prewarmed_client_session.take() {
        client_session.reset_websocket_session();
    }

    let skills_outcome = Some(turn_context.turn_skills.outcome.as_ref());

    sess.record_context_updates_and_set_reference_context_item(turn_context.as_ref())
        .await;

    let loaded_plugins = sess
        .services
        .plugins_manager
        .plugins_for_config(&turn_context.config)
        .await;
    // Structured plugin:// mentions are resolved from the current session's
    // enabled plugins, then converted into turn-scoped guidance below.
    let mentioned_plugins =
        collect_explicit_plugin_mentions(&input, loaded_plugins.capability_summaries());
    let mcp_tools = if turn_context.apps_enabled() || !mentioned_plugins.is_empty() {
        // Plugin mentions need raw MCP/app inventory even when app tools
        // are normally hidden so we can describe the plugin's currently
        // usable capabilities for this turn.
        match sess
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools_non_blocking()
            .or_cancel(&cancellation_token)
            .await
        {
            Ok(mcp_tools) => mcp_tools,
            Err(_) if turn_context.apps_enabled() => return None,
            Err(_) => HashMap::new(),
        }
    } else {
        HashMap::new()
    };
    let available_connectors = if turn_context.apps_enabled() {
        let connectors = codex_connectors::merge::merge_plugin_connectors_with_accessible(
            loaded_plugins
                .effective_apps()
                .into_iter()
                .map(|connector_id| connector_id.0),
            connectors::accessible_connectors_from_mcp_tools(&mcp_tools),
        );
        connectors::with_app_enabled_state(connectors, &turn_context.config)
    } else {
        Vec::new()
    };
    let connector_slug_counts = build_connector_slug_counts(&available_connectors);
    let skill_name_counts_lower = skills_outcome
        .as_ref()
        .map_or_else(HashMap::new, |outcome| {
            build_skill_name_counts(&outcome.skills, &outcome.disabled_paths).1
        });
    let mentioned_skills = skills_outcome.as_ref().map_or_else(Vec::new, |outcome| {
        collect_explicit_skill_mentions(
            &input,
            &outcome.skills,
            &outcome.disabled_paths,
            &connector_slug_counts,
        )
    });
    let config = turn_context.config.clone();
    if config
        .features
        .enabled(Feature::SkillEnvVarDependencyPrompt)
    {
        let env_var_dependencies = collect_env_var_dependencies(&mentioned_skills);
        resolve_skill_dependencies_for_turn(&sess, &turn_context, &env_var_dependencies).await;
    }

    maybe_prompt_and_install_mcp_dependencies(
        sess.as_ref(),
        turn_context.as_ref(),
        &cancellation_token,
        &mentioned_skills,
    )
    .await;

    let session_telemetry = turn_context.session_telemetry.clone();
    let thread_id = sess.conversation_id.to_string();
    let tracking = build_track_events_context(
        turn_context.model_info.slug.clone(),
        thread_id,
        turn_context.sub_id.clone(),
    );
    let SkillInjections {
        items: skill_injections,
        warnings: skill_warnings,
    } = build_skill_injections(
        &mentioned_skills,
        skills_outcome,
        Some(&session_telemetry),
        &sess.services.analytics_events_client,
        tracking.clone(),
    )
    .await;

    for message in skill_warnings {
        sess.send_event(&turn_context, EventMsg::Warning(WarningEvent { message }))
            .await;
    }

    let skill_items: Vec<ResponseItem> = skill_injections
        .iter()
        .map(|skill| ContextualUserFragment::into(crate::context::SkillInstructions::from(skill)))
        .collect();

    let plugin_items =
        build_plugin_injections(&mentioned_plugins, &mcp_tools, &available_connectors);
    let mentioned_plugin_metadata = mentioned_plugins
        .iter()
        .filter_map(crate::plugins::PluginCapabilitySummary::telemetry_metadata)
        .collect::<Vec<_>>();

    let mut explicitly_enabled_connectors = collect_explicit_app_ids(&input);
    explicitly_enabled_connectors.extend(collect_explicit_app_ids_from_skill_items(
        &skill_items,
        &available_connectors,
        &skill_name_counts_lower,
    ));
    let connector_names_by_id = available_connectors
        .iter()
        .map(|connector| (connector.id.as_str(), connector.name.as_str()))
        .collect::<HashMap<&str, &str>>();
    let mentioned_app_invocations = explicitly_enabled_connectors
        .iter()
        .map(|connector_id| AppInvocation {
            connector_id: Some(connector_id.clone()),
            app_name: connector_names_by_id
                .get(connector_id.as_str())
                .map(|name| (*name).to_string()),
            invocation_type: Some(InvocationType::Explicit),
        })
        .collect::<Vec<_>>();

    if run_pending_session_start_hooks(&sess, &turn_context).await {
        return None;
    }
    let additional_contexts = if input.is_empty() {
        Vec::new()
    } else {
        let initial_input_for_turn: ResponseInputItem = ResponseInputItem::from(input.clone());
        let response_item: ResponseItem = initial_input_for_turn.clone().into();
        let user_prompt_submit_outcome = run_user_prompt_submit_hooks(
            &sess,
            &turn_context,
            UserMessageItem::new(&input).message(),
        )
        .await;
        if user_prompt_submit_outcome.should_stop {
            record_additional_contexts(
                &sess,
                &turn_context,
                user_prompt_submit_outcome.additional_contexts,
            )
            .await;
            return None;
        }
        sess.record_user_prompt_and_emit_turn_item(turn_context.as_ref(), &input, response_item)
            .await;
        user_prompt_submit_outcome.additional_contexts
    };
    sess.services
        .analytics_events_client
        .track_app_mentioned(tracking.clone(), mentioned_app_invocations);
    for plugin in mentioned_plugin_metadata {
        sess.services
            .analytics_events_client
            .track_plugin_used(tracking.clone(), plugin);
    }
    sess.merge_connector_selection(explicitly_enabled_connectors.clone())
        .await;
    record_additional_contexts(&sess, &turn_context, additional_contexts).await;
    if !input.is_empty() {
        // Track the previous-turn baseline from the regular user-turn path only so
        // standalone tasks (compact/shell/review/undo) cannot suppress future
        // model/realtime injections.
        sess.set_previous_turn_settings(Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        }))
        .await;
    }
    if !skill_items.is_empty() {
        sess.record_conversation_items(&turn_context, &skill_items)
            .await;
    }
    if !plugin_items.is_empty() {
        sess.record_conversation_items(&turn_context, &plugin_items)
            .await;
    }

    track_turn_resolved_config_analytics(&sess, &turn_context, &input).await;

    let skills_outcome = Some(turn_context.turn_skills.outcome.as_ref());
    sess.maybe_start_ghost_snapshot(Arc::clone(&turn_context), cancellation_token.child_token())
        .await;
    let mut last_agent_message: Option<String> = None;
    let mut stop_hook_active = false;
    let mut taskspace_no_action_recovery_count = 0usize;
    let mut taskspace_no_action_recovery_key: Option<String> = None;
    let mut taskspace_implement_needs_edit_recovery_count = 0usize;
    let mut taskspace_implement_needs_edit_recovery_key: Option<String> = None;
    let mut taskspace_apply_patch_recovery_count = 0usize;
    let mut taskspace_apply_patch_recovery_key: Option<String> = None;
    let mut taskspace_validation_rework_duplicate_read_recovery_count = 0usize;
    let mut taskspace_validation_rework_duplicate_read_recovery_key: Option<String> = None;
    let mut taskspace_validation_rework_patch_only_recovery_count = 0usize;
    let mut taskspace_validation_rework_patch_only_recovery_key: Option<String> = None;
    // Although from the perspective of codex.rs, TurnDiffTracker has the lifecycle of a Task which contains
    // many turns, from the perspective of the user, it is a single turn.
    let turn_diff_tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));

    // `ModelClientSession` is turn-scoped and caches WebSocket + sticky routing state, so we reuse
    // one instance across retries within this turn.
    let mut client_session =
        prewarmed_client_session.unwrap_or_else(|| sess.services.model_client.new_session());
    // Pending input is drained into history before building the next model request.
    // However, we defer that drain until after sampling in two cases:
    // 1. At the start of a turn, so the fresh user prompt in `input` gets sampled first.
    // 2. After auto-compact, when model/tool continuation needs to resume before any steer.
    let mut can_drain_pending_input = input.is_empty();

    loop {
        if run_pending_session_start_hooks(&sess, &turn_context).await {
            break;
        }

        // Note that pending_input would be something like a message the user
        // submitted through the UI while the model was running. Though the UI
        // may support this, the model might not.
        let pending_input = if can_drain_pending_input {
            sess.get_pending_input().await
        } else {
            Vec::new()
        };

        let mut blocked_pending_input = false;
        let mut blocked_pending_input_contexts = Vec::new();
        let mut requeued_pending_input = false;
        let mut accepted_pending_input = Vec::new();
        if !pending_input.is_empty() {
            let mut pending_input_iter = pending_input.into_iter();
            while let Some(pending_input_item) = pending_input_iter.next() {
                match inspect_pending_input(&sess, &turn_context, pending_input_item).await {
                    PendingInputHookDisposition::Accepted(pending_input) => {
                        accepted_pending_input.push(*pending_input);
                    }
                    PendingInputHookDisposition::Blocked {
                        additional_contexts,
                    } => {
                        let remaining_pending_input = pending_input_iter.collect::<Vec<_>>();
                        if !remaining_pending_input.is_empty() {
                            let _ = sess.prepend_pending_input(remaining_pending_input).await;
                            requeued_pending_input = true;
                        }
                        blocked_pending_input_contexts = additional_contexts;
                        blocked_pending_input = true;
                        break;
                    }
                }
            }
        }

        let has_accepted_pending_input = !accepted_pending_input.is_empty();
        for pending_input in accepted_pending_input {
            record_pending_input(&sess, &turn_context, pending_input).await;
        }
        record_additional_contexts(&sess, &turn_context, blocked_pending_input_contexts).await;

        if blocked_pending_input && !has_accepted_pending_input {
            if requeued_pending_input {
                continue;
            }
            break;
        }

        if let Some(action_map_projection) = {
            let mut state = sess.state.lock().await;
            state.action_map_runtime.build_developer_context()
        } && let Some(item) = crate::context_manager::updates::build_developer_update_item(vec![
            action_map_projection,
        ]) {
            sess.remove_action_map_projection_history_items().await;
            sess.record_conversation_items(&turn_context, &[item]).await;
        }

        // Construct the input that we will send to the model.
        let sampling_request_input: Vec<ResponseItem> = prepare_provider_visible_prompt_items(
            sess.clone_history()
                .await
                .for_prompt(&turn_context.model_info.input_modalities),
        );

        let sampling_request_input_messages = sampling_request_input
            .iter()
            .filter_map(|item| match parse_turn_item(item) {
                Some(TurnItem::UserMessage(user_message)) => Some(user_message),
                _ => None,
            })
            .map(|user_message| user_message.message())
            .collect::<Vec<String>>();
        let turn_metadata_header = turn_context.turn_metadata_state.current_header_value();
        match run_sampling_request(
            Arc::clone(&sess),
            Arc::clone(&turn_context),
            Arc::clone(&turn_diff_tracker),
            &mut client_session,
            turn_metadata_header.as_deref(),
            sampling_request_input,
            &explicitly_enabled_connectors,
            skills_outcome,
            cancellation_token.child_token(),
        )
        .await
        {
            Ok(sampling_request_output) => {
                let SamplingRequestResult {
                    needs_follow_up: model_needs_follow_up,
                    last_agent_message: sampling_request_last_agent_message,
                    taskspace_no_action_recovery_item,
                } = sampling_request_output;
                can_drain_pending_input = true;
                if let Some(recovery_item) = taskspace_no_action_recovery_item {
                    let current_recovery_snapshot =
                        sess.action_map_provider_request_budget_snapshot().await;
                    let is_provider_budget_hard_stop =
                        is_taskspace_provider_budget_hard_stop_item(&recovery_item);
                    let counts_against_no_action_cap =
                        is_taskspace_no_action_recovery_item(&recovery_item);
                    let counts_against_implement_needs_edit_cap =
                        is_taskspace_implement_needs_edit_recovery_item(&recovery_item)
                            || is_taskspace_validation_rework_patch_only_recovery_item(
                                &recovery_item,
                            );
                    let counts_against_plain_implement_needs_edit_cap =
                        is_taskspace_plain_implement_needs_edit_recovery_item(&recovery_item);
                    let is_validation_rework_duplicate_read_recovery =
                        is_taskspace_validation_rework_duplicate_read_recovery_item(&recovery_item);
                    let is_validation_rework_patch_only_recovery =
                        is_taskspace_validation_rework_patch_only_recovery_item(&recovery_item);
                    let is_apply_patch_recovery =
                        is_taskspace_apply_patch_recovery_item(&recovery_item);
                    if is_apply_patch_recovery {
                        taskspace_reset_recovery_count_for_snapshot_node(
                            &mut taskspace_apply_patch_recovery_key,
                            &mut taskspace_apply_patch_recovery_count,
                            current_recovery_snapshot.as_ref(),
                        );
                    }
                    if is_validation_rework_duplicate_read_recovery {
                        taskspace_reset_recovery_count_for_snapshot_node(
                            &mut taskspace_validation_rework_duplicate_read_recovery_key,
                            &mut taskspace_validation_rework_duplicate_read_recovery_count,
                            current_recovery_snapshot.as_ref(),
                        );
                    }
                    if is_validation_rework_patch_only_recovery {
                        taskspace_reset_recovery_count_for_snapshot_node(
                            &mut taskspace_validation_rework_patch_only_recovery_key,
                            &mut taskspace_validation_rework_patch_only_recovery_count,
                            current_recovery_snapshot.as_ref(),
                        );
                    }
                    let validation_rework_duplicate_read_hard_stop =
                        taskspace_validation_rework_duplicate_read_should_hard_stop(
                            &recovery_item,
                            taskspace_validation_rework_duplicate_read_recovery_count,
                        );
                    let validation_rework_patch_only_hard_stop =
                        taskspace_validation_rework_patch_only_should_hard_stop(
                            &recovery_item,
                            taskspace_validation_rework_patch_only_recovery_count,
                        );
                    let apply_patch_recovery_hard_stop =
                        taskspace_apply_patch_recovery_should_hard_stop(
                            &recovery_item,
                            taskspace_apply_patch_recovery_count,
                        );
                    let no_action_recovery_cap = current_recovery_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.node_kind.clone())
                        .as_deref()
                        .map(taskspace_no_action_recovery_cap_for_node_kind)
                        .unwrap_or(1usize);
                    if counts_against_no_action_cap {
                        taskspace_reset_recovery_count_for_snapshot_node(
                            &mut taskspace_no_action_recovery_key,
                            &mut taskspace_no_action_recovery_count,
                            current_recovery_snapshot.as_ref(),
                        );
                        taskspace_no_action_recovery_count += 1;
                    }
                    let no_action_recovery_over_advisory = counts_against_no_action_cap
                        && taskspace_no_action_recovery_count > no_action_recovery_cap;
                    let no_action_recovery_hard_stop =
                        taskspace_no_action_recovery_should_hard_stop(
                            &recovery_item,
                            taskspace_no_action_recovery_count,
                            no_action_recovery_cap,
                        );
                    if counts_against_implement_needs_edit_cap {
                        let recovery_key = current_recovery_snapshot
                            .as_ref()
                            .and_then(|snapshot| snapshot.node_id.as_deref())
                            .map(str::to_string)
                            .unwrap_or_else(|| "unknown-node".to_string());
                        if taskspace_implement_needs_edit_recovery_key.as_deref()
                            != Some(recovery_key.as_str())
                        {
                            taskspace_implement_needs_edit_recovery_key = Some(recovery_key);
                            taskspace_implement_needs_edit_recovery_count = 0;
                        }
                        taskspace_implement_needs_edit_recovery_count += 1;
                    }
                    if is_apply_patch_recovery {
                        taskspace_apply_patch_recovery_count += 1;
                    }
                    if is_validation_rework_duplicate_read_recovery {
                        taskspace_validation_rework_duplicate_read_recovery_count += 1;
                    }
                    if is_validation_rework_patch_only_recovery {
                        taskspace_validation_rework_patch_only_recovery_count += 1;
                    }
                    if taskspace_implementation_needs_edit_should_hard_stop(
                        &recovery_item,
                        taskspace_implement_needs_edit_recovery_count,
                    ) && counts_against_plain_implement_needs_edit_cap
                    {
                        let hard_stop_item =
                            build_taskspace_implementation_needs_edit_hard_stop_item(
                                &recovery_item,
                                taskspace_implement_needs_edit_recovery_count,
                            );
                        sess.send_event(
                            &turn_context,
                            EventMsg::Warning(WarningEvent {
                                message: taskspace_special_recovery_warning_message(
                                    &hard_stop_item,
                                ),
                            }),
                        )
                        .await;
                        sess.record_conversation_items(&turn_context, &[hard_stop_item])
                            .await;
                        last_agent_message = Some(
                            "TaskSpace implementation needs-edit hard stop: repeated_finish_without_successful_edit".to_string(),
                        );
                        break;
                    }
                    if apply_patch_recovery_hard_stop {
                        let hard_stop_item = build_taskspace_apply_patch_recovery_hard_stop_item(
                            &recovery_item,
                            taskspace_apply_patch_recovery_count,
                        );
                        sess.send_event(
                            &turn_context,
                            EventMsg::Warning(WarningEvent {
                                message: taskspace_special_recovery_warning_message(
                                    &hard_stop_item,
                                ),
                            }),
                        )
                        .await;
                        sess.record_conversation_items(&turn_context, &[hard_stop_item])
                            .await;
                        last_agent_message = Some(
                            "TaskSpace apply_patch recovery hard stop: repeated_failed_or_malformed_patch".to_string(),
                        );
                        break;
                    }
                    if validation_rework_patch_only_hard_stop {
                        let hard_stop_item =
                            build_taskspace_validation_rework_patch_only_hard_stop_item(
                                &recovery_item,
                                taskspace_validation_rework_patch_only_recovery_count,
                            );
                        sess.send_event(
                            &turn_context,
                            EventMsg::Warning(WarningEvent {
                                message: taskspace_special_recovery_warning_message(
                                    &hard_stop_item,
                                ),
                            }),
                        )
                        .await;
                        sess.record_conversation_items(&turn_context, &[hard_stop_item])
                            .await;
                        last_agent_message = Some(
                            "TaskSpace validation rework patch-only hard stop: repeated_non_edit_after_target_read".to_string(),
                        );
                        break;
                    }
                    if validation_rework_duplicate_read_hard_stop {
                        let hard_stop_item =
                            build_taskspace_validation_rework_duplicate_read_hard_stop_item(
                                &recovery_item,
                                taskspace_validation_rework_duplicate_read_recovery_count,
                            );
                        sess.send_event(
                            &turn_context,
                            EventMsg::Warning(WarningEvent {
                                message: taskspace_special_recovery_warning_message(
                                    &hard_stop_item,
                                ),
                            }),
                        )
                        .await;
                        sess.record_conversation_items(&turn_context, &[hard_stop_item])
                            .await;
                        last_agent_message = Some(
                            "TaskSpace validation rework duplicate-read hard stop: repeated_validation_rework_duplicate_artifact_read".to_string(),
                        );
                        break;
                    }
                    if no_action_recovery_hard_stop {
                        let hard_stop_item = build_taskspace_no_action_recovery_hard_stop_item(
                            &recovery_item,
                            taskspace_no_action_recovery_count,
                            no_action_recovery_cap,
                        );
                        sess.send_event(
                            &turn_context,
                            EventMsg::Warning(WarningEvent {
                                message: taskspace_special_recovery_warning_message(
                                    &hard_stop_item,
                                ),
                            }),
                        )
                        .await;
                        sess.record_conversation_items(&turn_context, &[hard_stop_item])
                            .await;
                        last_agent_message = Some(
                            "TaskSpace no-action recovery hard stop: repeated_no_action_after_recovery_threshold".to_string(),
                        );
                        break;
                    }
                    sess.send_event(
                        &turn_context,
                        EventMsg::Warning(WarningEvent {
                            message: if counts_against_no_action_cap {
                                if no_action_recovery_over_advisory {
                                    format!("TaskSpace inserted TaskSpaceNoActionRecoveryV1 beyond the advisory recovery threshold because the provider still requested follow-up or hit a recoverable action-contract rejection. Recovery attempt {} is being used after advisory threshold {}. The turn will continue; the model must emit a tool call, taskspace_control transition, or blocked-with-evidence result.", taskspace_no_action_recovery_count, no_action_recovery_cap)
                                } else {
                                    format!("TaskSpace inserted TaskSpaceNoActionRecoveryV1 because the provider response requested follow-up or was rejected by the final-response gate without an actionable tool/control/final result. Recovery attempt {}/{} is being used.", taskspace_no_action_recovery_count, no_action_recovery_cap)
                                }
                            } else if counts_against_implement_needs_edit_cap {
                                taskspace_implement_recovery_advisory_warning_message(
                                    &recovery_item,
                                    taskspace_implement_needs_edit_recovery_count,
                                )
                            } else {
                                taskspace_special_recovery_warning_message(&recovery_item)
                            },
                        }),
                    )
                    .await;
                    sess.record_conversation_items(&turn_context, &[recovery_item])
                        .await;
                    if is_provider_budget_hard_stop {
                        last_agent_message = sampling_request_last_agent_message;
                        break;
                    }
                    if let Some(snapshot) = current_recovery_snapshot.as_ref()
                        && taskspace_provider_budget_limit_reached(snapshot)
                    {
                        sess.action_map_mark_next_provider_request_budget_recovery(
                            &turn_context,
                            "taskspace_recovery_item_after_provider_budget_limit",
                        )
                        .await;
                    }
                    continue;
                }
                let has_pending_input = sess.has_pending_input().await;
                let needs_follow_up = model_needs_follow_up || has_pending_input;
                let total_usage_tokens = sess.get_total_token_usage().await;
                let token_limit_reached = total_usage_tokens >= auto_compact_limit;

                let estimated_token_count =
                    sess.get_estimated_token_count(turn_context.as_ref()).await;

                trace!(
                    turn_id = %turn_context.sub_id,
                    total_usage_tokens,
                    estimated_token_count = ?estimated_token_count,
                    auto_compact_limit,
                    token_limit_reached,
                    model_needs_follow_up,
                    has_pending_input,
                    needs_follow_up,
                    "post sampling token usage"
                );

                // as long as compaction works well in getting us way below the token limit, we shouldn't worry about being in an infinite loop.
                if token_limit_reached && needs_follow_up {
                    if run_auto_compact(
                        &sess,
                        &turn_context,
                        InitialContextInjection::BeforeLastUserMessage,
                        CompactionReason::ContextLimit,
                        CompactionPhase::MidTurn,
                    )
                    .await
                    .is_err()
                    {
                        return None;
                    }
                    client_session.reset_websocket_session();
                    can_drain_pending_input = !model_needs_follow_up;
                    continue;
                }

                if !needs_follow_up {
                    last_agent_message = sampling_request_last_agent_message;
                    let stop_hook_permission_mode = match turn_context.approval_policy.value() {
                        AskForApproval::Never => "bypassPermissions",
                        AskForApproval::UnlessTrusted
                        | AskForApproval::OnFailure
                        | AskForApproval::OnRequest
                        | AskForApproval::Granular(_) => "default",
                    }
                    .to_string();
                    let stop_request = codex_hooks::StopRequest {
                        session_id: sess.conversation_id,
                        turn_id: turn_context.sub_id.clone(),
                        cwd: turn_context.cwd.clone(),
                        transcript_path: sess.hook_transcript_path().await,
                        model: turn_context.model_info.slug.clone(),
                        permission_mode: stop_hook_permission_mode,
                        stop_hook_active,
                        last_assistant_message: last_agent_message.clone(),
                    };
                    for run in sess.hooks().preview_stop(&stop_request) {
                        sess.send_event(
                            &turn_context,
                            EventMsg::HookStarted(codex_protocol::protocol::HookStartedEvent {
                                turn_id: Some(turn_context.sub_id.clone()),
                                run,
                            }),
                        )
                        .await;
                    }
                    let stop_outcome = sess.hooks().run_stop(stop_request).await;
                    emit_hook_completed_events(&sess, &turn_context, stop_outcome.hook_events)
                        .await;
                    if stop_outcome.should_block {
                        if let Some(hook_prompt_message) =
                            build_hook_prompt_message(&stop_outcome.continuation_fragments)
                        {
                            sess.record_conversation_items(
                                &turn_context,
                                std::slice::from_ref(&hook_prompt_message),
                            )
                            .await;
                            stop_hook_active = true;
                            continue;
                        } else {
                            sess.send_event(
                                &turn_context,
                                EventMsg::Warning(WarningEvent {
                                    message: "Stop hook requested continuation without a prompt; ignoring the block.".to_string(),
                                }),
                            )
                            .await;
                        }
                    }
                    if stop_outcome.should_stop {
                        break;
                    }
                    let hook_outcomes = sess
                        .hooks()
                        .dispatch(HookPayload {
                            session_id: sess.conversation_id,
                            cwd: turn_context.cwd.clone(),
                            client: turn_context.app_server_client_name.clone(),
                            triggered_at: chrono::Utc::now(),
                            hook_event: HookEvent::AfterAgent {
                                event: HookEventAfterAgent {
                                    thread_id: sess.conversation_id,
                                    turn_id: turn_context.sub_id.clone(),
                                    input_messages: sampling_request_input_messages,
                                    last_assistant_message: last_agent_message.clone(),
                                },
                            },
                        })
                        .await;

                    let mut abort_message = None;
                    for hook_outcome in hook_outcomes {
                        let hook_name = hook_outcome.hook_name;
                        match hook_outcome.result {
                            HookResult::Success => {}
                            HookResult::FailedContinue(error) => {
                                warn!(
                                    turn_id = %turn_context.sub_id,
                                    hook_name = %hook_name,
                                    error = %error,
                                    "after_agent hook failed; continuing"
                                );
                            }
                            HookResult::FailedAbort(error) => {
                                let message = format!(
                                    "after_agent hook '{hook_name}' failed and aborted turn completion: {error}"
                                );
                                warn!(
                                    turn_id = %turn_context.sub_id,
                                    hook_name = %hook_name,
                                    error = %error,
                                    "after_agent hook failed; aborting operation"
                                );
                                if abort_message.is_none() {
                                    abort_message = Some(message);
                                }
                            }
                        }
                    }
                    if let Some(message) = abort_message {
                        sess.send_event(
                            &turn_context,
                            EventMsg::Error(ErrorEvent {
                                message,
                                codex_error_info: None,
                            }),
                        )
                        .await;
                        return None;
                    }
                    break;
                }
                continue;
            }
            Err(CodexErr::TurnAborted) => {
                // Aborted turn is reported via a different event.
                break;
            }
            Err(CodexErr::InvalidImageRequest()) => {
                {
                    let mut state = sess.state.lock().await;
                    error_or_panic(
                        "Invalid image detected; sanitizing tool output to prevent poisoning",
                    );
                    if state.history.replace_last_turn_images("Invalid image") {
                        continue;
                    }
                }

                let event = EventMsg::Error(ErrorEvent {
                    message: "Invalid image in your last message. Please remove it and try again."
                        .to_string(),
                    codex_error_info: Some(CodexErrorInfo::BadRequest),
                });
                sess.send_event(&turn_context, event).await;
                break;
            }
            Err(e) => {
                info!("Turn error: {e:#}");
                let event = EventMsg::Error(e.to_error_event(/*message_prefix*/ None));
                sess.send_event(&turn_context, event).await;
                // let the user continue the conversation
                break;
            }
        }
    }

    last_agent_message
}

async fn track_turn_resolved_config_analytics(
    sess: &Session,
    turn_context: &TurnContext,
    input: &[UserInput],
) {
    if !sess.enabled(Feature::GeneralAnalytics) {
        return;
    }

    let thread_config = {
        let state = sess.state.lock().await;
        state.session_configuration.thread_config_snapshot()
    };
    let is_first_turn = {
        let mut state = sess.state.lock().await;
        state.take_next_turn_is_first()
    };
    sess.services
        .analytics_events_client
        .track_turn_resolved_config(TurnResolvedConfigFact {
            turn_id: turn_context.sub_id.clone(),
            thread_id: sess.conversation_id.to_string(),
            num_input_images: input
                .iter()
                .filter(|item| {
                    matches!(item, UserInput::Image { .. } | UserInput::LocalImage { .. })
                })
                .count(),
            submission_type: None,
            ephemeral: thread_config.ephemeral,
            session_source: thread_config.session_source,
            model: turn_context.model_info.slug.clone(),
            model_provider: turn_context.config.model_provider_id.clone(),
            sandbox_policy: turn_context.sandbox_policy.get().clone(),
            reasoning_effort: turn_context.reasoning_effort,
            reasoning_summary: Some(turn_context.reasoning_summary),
            service_tier: turn_context.config.service_tier,
            approval_policy: turn_context.approval_policy.value(),
            approvals_reviewer: turn_context.config.approvals_reviewer,
            sandbox_network_access: turn_context.network_sandbox_policy.is_enabled(),
            collaboration_mode: turn_context.collaboration_mode.mode,
            personality: turn_context.personality,
            is_first_turn,
        });
}

async fn run_pre_sampling_compact(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
) -> CodexResult<bool> {
    let total_usage_tokens_before_compaction = sess.get_total_token_usage().await;
    let mut pre_sampling_compacted = maybe_run_previous_model_inline_compact(
        sess,
        turn_context,
        total_usage_tokens_before_compaction,
    )
    .await?;
    let total_usage_tokens = sess.get_total_token_usage().await;
    let auto_compact_limit = turn_context
        .model_info
        .auto_compact_token_limit()
        .unwrap_or(i64::MAX);
    // Compact if the total usage tokens are greater than the auto compact limit
    if total_usage_tokens >= auto_compact_limit {
        run_auto_compact(
            sess,
            turn_context,
            InitialContextInjection::DoNotInject,
            CompactionReason::ContextLimit,
            CompactionPhase::PreTurn,
        )
        .await?;
        pre_sampling_compacted = true;
    }
    Ok(pre_sampling_compacted)
}

/// Runs pre-sampling compaction against the previous model when switching to a smaller
/// context-window model.
///
/// Returns `Ok(true)` when compaction ran successfully, `Ok(false)` when compaction was skipped
/// because the model/context-window preconditions were not met, and `Err(_)` only when compaction
/// was attempted and failed.
async fn maybe_run_previous_model_inline_compact(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    total_usage_tokens: i64,
) -> CodexResult<bool> {
    let Some(previous_turn_settings) = sess.previous_turn_settings().await else {
        return Ok(false);
    };
    let previous_model_turn_context = Arc::new(
        turn_context
            .with_model(previous_turn_settings.model, &sess.services.models_manager)
            .await,
    );

    let Some(old_context_window) = previous_model_turn_context.model_context_window() else {
        return Ok(false);
    };
    let Some(new_context_window) = turn_context.model_context_window() else {
        return Ok(false);
    };
    let new_auto_compact_limit = turn_context
        .model_info
        .auto_compact_token_limit()
        .unwrap_or(i64::MAX);
    let should_run = total_usage_tokens > new_auto_compact_limit
        && previous_model_turn_context.model_info.slug != turn_context.model_info.slug
        && old_context_window > new_context_window;
    if should_run {
        run_auto_compact(
            sess,
            &previous_model_turn_context,
            InitialContextInjection::DoNotInject,
            CompactionReason::ModelDownshift,
            CompactionPhase::PreTurn,
        )
        .await?;
        return Ok(true);
    }
    Ok(false)
}

async fn run_auto_compact(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    initial_context_injection: InitialContextInjection,
    reason: CompactionReason,
    phase: CompactionPhase,
) -> CodexResult<()> {
    match compact_strategy(turn_context.provider.info()) {
        CompactStrategy::OpenAiRemote => {
            run_inline_remote_auto_compact_task(
                Arc::clone(sess),
                Arc::clone(turn_context),
                initial_context_injection,
                reason,
                phase,
            )
            .await?;
        }
        CompactStrategy::DeepSeekPro | CompactStrategy::LocalFallback => {
            run_inline_auto_compact_task(
                Arc::clone(sess),
                Arc::clone(turn_context),
                initial_context_injection,
                reason,
                phase,
            )
            .await?;
        }
    }
    Ok(())
}

pub(super) fn collect_explicit_app_ids_from_skill_items(
    skill_items: &[ResponseItem],
    connectors: &[connectors::AppInfo],
    skill_name_counts_lower: &HashMap<String, usize>,
) -> HashSet<String> {
    if skill_items.is_empty() || connectors.is_empty() {
        return HashSet::new();
    }

    let skill_messages = skill_items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { content, .. } => {
                content.iter().find_map(|content_item| match content_item {
                    ContentItem::InputText { text } => Some(text.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect::<Vec<String>>();
    if skill_messages.is_empty() {
        return HashSet::new();
    }

    let mentions = collect_tool_mentions_from_messages(&skill_messages);
    let mention_names_lower = mentions
        .plain_names
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<HashSet<String>>();
    let mut connector_ids = mentions
        .paths
        .iter()
        .filter(|path| tool_kind_for_path(path) == ToolMentionKind::App)
        .filter_map(|path| app_id_from_path(path).map(str::to_string))
        .collect::<HashSet<String>>();

    let connector_slug_counts = build_connector_slug_counts(connectors);
    for connector in connectors {
        let slug = codex_connectors::metadata::connector_mention_slug(connector);
        let connector_count = connector_slug_counts.get(&slug).copied().unwrap_or(0);
        let skill_count = skill_name_counts_lower.get(&slug).copied().unwrap_or(0);
        if connector_count == 1 && skill_count == 0 && mention_names_lower.contains(&slug) {
            connector_ids.insert(connector.id.clone());
        }
    }

    connector_ids
}

pub(super) fn filter_connectors_for_input(
    connectors: &[connectors::AppInfo],
    input: &[ResponseItem],
    explicitly_enabled_connectors: &HashSet<String>,
    skill_name_counts_lower: &HashMap<String, usize>,
) -> Vec<connectors::AppInfo> {
    let connectors: Vec<connectors::AppInfo> = connectors
        .iter()
        .filter(|connector| connector.is_enabled)
        .cloned()
        .collect::<Vec<_>>();
    if connectors.is_empty() {
        return Vec::new();
    }

    let user_messages = collect_user_messages(input);
    if user_messages.is_empty() && explicitly_enabled_connectors.is_empty() {
        return Vec::new();
    }

    let mentions = collect_tool_mentions_from_messages(&user_messages);
    let mention_names_lower = mentions
        .plain_names
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<HashSet<String>>();

    let connector_slug_counts = build_connector_slug_counts(&connectors);
    let mut allowed_connector_ids = explicitly_enabled_connectors.clone();
    for path in mentions
        .paths
        .iter()
        .filter(|path| tool_kind_for_path(path) == ToolMentionKind::App)
    {
        if let Some(connector_id) = app_id_from_path(path) {
            allowed_connector_ids.insert(connector_id.to_string());
        }
    }

    connectors
        .into_iter()
        .filter(|connector| {
            connector_inserted_in_messages(
                connector,
                &mention_names_lower,
                &allowed_connector_ids,
                &connector_slug_counts,
                skill_name_counts_lower,
            )
        })
        .collect()
}

fn connector_inserted_in_messages(
    connector: &connectors::AppInfo,
    mention_names_lower: &HashSet<String>,
    allowed_connector_ids: &HashSet<String>,
    connector_slug_counts: &HashMap<String, usize>,
    skill_name_counts_lower: &HashMap<String, usize>,
) -> bool {
    if allowed_connector_ids.contains(&connector.id) {
        return true;
    }

    let mention_slug = codex_connectors::metadata::connector_mention_slug(connector);
    let connector_count = connector_slug_counts
        .get(&mention_slug)
        .copied()
        .unwrap_or(0);
    let skill_count = skill_name_counts_lower
        .get(&mention_slug)
        .copied()
        .unwrap_or(0);
    connector_count == 1 && skill_count == 0 && mention_names_lower.contains(&mention_slug)
}

pub(crate) fn build_prompt(
    input: Vec<ResponseItem>,
    router: &ToolRouter,
    turn_context: &TurnContext,
    base_instructions: BaseInstructions,
) -> Prompt {
    build_prompt_with_tool_visibility(
        input,
        router,
        turn_context,
        base_instructions,
        TaskspaceProviderToolVisibility::All,
    )
}

fn build_prompt_with_tool_visibility(
    input: Vec<ResponseItem>,
    router: &ToolRouter,
    turn_context: &TurnContext,
    base_instructions: BaseInstructions,
    tool_visibility: TaskspaceProviderToolVisibility,
) -> Prompt {
    let deferred_dynamic_tools = turn_context
        .dynamic_tools
        .iter()
        .filter(|tool| tool.defer_loading)
        .map(|tool| ToolName::new(tool.namespace.clone(), tool.name.clone()))
        .collect::<HashSet<_>>();
    let tools = if deferred_dynamic_tools.is_empty() {
        router.model_visible_specs()
    } else {
        router
            .model_visible_specs()
            .into_iter()
            .filter_map(|spec| filter_deferred_dynamic_tool_spec(spec, &deferred_dynamic_tools))
            .collect()
    };
    let tools = match tool_visibility {
        TaskspaceProviderToolVisibility::All => tools,
        TaskspaceProviderToolVisibility::None => Vec::new(),
    };

    Prompt {
        input,
        tools,
        parallel_tool_calls: matches!(tool_visibility, TaskspaceProviderToolVisibility::All)
            && turn_context.model_info.supports_parallel_tool_calls,
        tool_choice: "auto".to_string(),
        base_instructions,
        personality: turn_context.personality,
        output_schema: turn_context.final_output_json_schema.clone(),
        output_schema_strict: !crate::guardian::is_guardian_reviewer_source(
            &turn_context.session_source,
        ),
    }
}

fn taskspace_provider_tool_visibility_for_budget(
    _request_count: usize,
    _max_requests: usize,
    _node_request_count: usize,
    _max_model_requests_per_node: usize,
    _request_phase: Option<&str>,
    _node_kind: Option<&str>,
) -> TaskspaceProviderToolVisibility {
    TaskspaceProviderToolVisibility::All
}

fn taskspace_provider_transport_mode(
    turn_context: &TurnContext,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
    taskspace_context_visible: bool,
) -> TaskspaceProviderTransportMode {
    if snapshot.is_none() && !taskspace_context_visible {
        return TaskspaceProviderTransportMode::NativeTools;
    }
    let configured = std::env::var("WHALE_TASKSPACE_PROVIDER_TRANSPORT")
        .ok()
        .or_else(|| std::env::var("TASKSPACE_PROVIDER_TRANSPORT").ok())
        .unwrap_or_default();
    let provider_info = turn_context.provider.info();
    let provider_name = provider_info.name.to_ascii_lowercase();
    let model = turn_context.model_info.slug.to_ascii_lowercase();
    let deepseek_chat = provider_info.wire_api
        == codex_model_provider_info::WireApi::ChatCompletions
        && (provider_name.contains("deepseek") || model.contains("deepseek"));
    taskspace_provider_transport_mode_for_request(deepseek_chat, configured.as_str())
}

fn taskspace_provider_transport_mode_for_request(
    deepseek_chat: bool,
    configured: &str,
) -> TaskspaceProviderTransportMode {
    if !deepseek_chat || configured == "native_tools" {
        TaskspaceProviderTransportMode::NativeTools
    } else {
        TaskspaceProviderTransportMode::CacheOptimizedActionContract
    }
}

fn taskspace_transport_budget_limits(
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> TaskspaceProviderTransportBudgetLimits {
    TaskspaceProviderTransportBudgetLimits {
        max_requests: snapshot.max_requests,
        max_model_requests_per_node: snapshot.max_model_requests_per_node,
        post_budget_grace_requests: snapshot.post_budget_grace_requests,
    }
}

fn taskspace_static_action_contract_instructions() -> &'static str {
    "TaskSpaceActionContractV1:
You are running in TaskSpace cache-optimized action-contract transport.
Provider-native tools are intentionally disabled for this request.
Return one taskspace-action-v1 JSON object as the assistant message body.
Do not emit markdown fences, DSML tool calls, XML tags, prose before JSON, or prose after JSON.
Required JSON shape:
{\"schema_version\":\"taskspace-action-v1\",\"action\":\"<action>\",\"node_id\":\"<active node id or null>\",\"args\":{},\"rationale\":\"short reason\"}
Allowed actions by active node kind:
- bootstrap/no active task: taskspace_control, blocked
- existing task with no active node: final_answer, taskspace_control, blocked
- inspect_code_context: list_files, search, read_file, run_test, taskspace_control, blocked; use run_test only for pre-edit diagnostic or baseline evidence, not final validation closeout
- implement_solution: list_files, search, read_file, apply_patch, taskspace_control, blocked before implementation_needs_edit; once TaskSpaceActionContractStateV1 says implementation_needs_edit, only apply_patch, taskspace_control, blocked, or a read_file explicitly targeting a validation rework artifact named in TaskSpaceActionContractStateV1/projection/recent feedback are valid while that rework target has not yet been read completely
- validation rework override: if TaskSpaceActionContractStateV1, projection, or recent feedback says validation_rework_patch_only_after_target_read, complete_read/eof_reached=true, or validation_rework_closed_action_space_read_disallowed, read_file/list_files/search/schema inspection are not valid; emit apply_patch for the named target artifact or taskspace_control block_node only
- smoke_test/regression_test: run_test, taskspace_control, blocked
- final_synthesis: final_answer, taskspace_control, blocked
Action argument rules:
- list_files args: {\"path\":\".\"}
- search args: {\"pattern\":\"literal or regex\",\"path\":\".\"}
- read_file args: {\"path\":\"relative/path\"}
- apply_patch args: {\"patch\":\"*** Begin Patch\\n...\\n*** End Patch\\n\"}; create new files with native `*** Add File: <path>` plus `+` content lines, and update existing files with `*** Update File: <path>` hunks.
- run_test args: {\"command\":\"test command\",\"timeout_ms\":120000}
- taskspace_control args: {\"action\":\"start_task|finish_node|create_node|bind_node|block_node|record_fact|record_fact_source|record_output_contract|record_success_criteria|state_commit\",...}; use canonical key \"action\", not \"action_name\" or \"command\", for lifecycle commands.
- final_answer args: {\"message\":\"user-facing final answer\"}
- blocked args: {\"reason\":\"exact missing evidence or blocker\"}
Validation invariants:
- Unknown actions or actions disallowed for the active node will be rejected and no tool will execute.
- If provider-native tool-call markup appears after the JSON object, TaskSpace ignores that markup and executes only the JSON action."
}

fn taskspace_deepseek_cache_anchor() -> String {
    let mut text = String::from(
        "TaskSpaceDeepSeekCacheAnchorV1:\n\
This block is a provider-cache stability anchor for DeepSeek ChatCompletions.\n\
It has no task semantics. Ignore every CACHE_ANCHOR_LINE when deciding actions.\n\
The line content is intentionally byte-stable across TaskSpace requests.\n",
    );
    for index in 1..=TASKSPACE_DEEPSEEK_CACHE_ANCHOR_LINES {
        text.push_str(&format!(
            "CACHE_ANCHOR_LINE_{index:04}: stable TaskSpace DeepSeek prefix anchor; ignore for task reasoning and actions.\n"
        ));
    }
    text.push_str("TaskSpaceDeepSeekCacheAnchorV1 end.\n");
    text
}

fn taskspace_action_contract_state_item(
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> ResponseItem {
    let node_id = snapshot.node_id.as_deref().unwrap_or("none");
    let node_kind = snapshot.node_kind.as_deref().unwrap_or("unknown");
    let mut text = format!(
        "TaskSpaceActionContractStateV1:\n\
Active node id: {node_id}\n\
Active node kind: {node_kind}"
    );
    if snapshot.task_id.is_some() && snapshot.node_id.is_none() {
        text.push_str(
            "\nExisting TaskSpace task has no active bound node. If the requested work is complete, return final_answer. Do not create or bind another node unless unresolved work remains.",
        );
    }
    if taskspace_snapshot_requires_implementation_edit(snapshot) {
        text.push_str(
            "\nImplementation convergence state: implementation_needs_edit. Current request allowed actions are narrowed to apply_patch, taskspace_control, blocked, or read_file only for a validation rework target artifact explicitly named below/projection/recent feedback. Do not call list_files, search, broad read_file, shell discovery, or validation before a successful edit is recorded.",
        );
        if !snapshot.current_node_validation_rework_artifacts.is_empty() {
            text.push_str("\nValidation rework target artifacts that may be read once if current contents are not visible:");
            for artifact in &snapshot.current_node_validation_rework_artifacts {
                text.push_str("\n- ");
                text.push_str(artifact);
            }
        }
        if !snapshot
            .current_node_uncovered_mandatory_evidence
            .is_empty()
        {
            text.push_str("\nRequired edit targets from uncovered mandatory evidence:");
            for item in &snapshot.current_node_uncovered_mandatory_evidence {
                text.push_str("\n- ");
                text.push_str(item);
            }
            text.push_str(
                "\nThe next apply_patch must update the exact artifact path(s) named above.",
            );
        }
    } else if matches!(
        snapshot.node_kind.as_deref(),
        Some("smoke_test" | "regression_test")
    ) {
        text.push_str(
            "\nValidation convergence state: validation_needs_test. Current request allowed actions are narrowed to run_test, taskspace_control, or blocked only. Do not call list_files, search, read_file, shell discovery, apply_patch, or spawn_agent from a validation node. If a discovered local validator is named in the projection or recent tool feedback, run that exact validator command before any generic pytest/cargo/npm test command.",
        );
    }
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_action_contract_bootstrap_state_item() -> ResponseItem {
    let text = "TaskSpaceActionContractStateV1:\n\
Active node id: none\n\
Active node kind: bootstrap\n\
No active TaskSpace task exists yet. The next action must be taskspace_control with action=start_task or blocked."
        .to_string();
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_action_contract_closed_validation_item() -> ResponseItem {
    let text = "TaskSpaceActionContractClosedValidationV1:\n\
Existing TaskSpace task has no active bound node because validation is closed as blocked by local infrastructure evidence.\n\
Current request allowed actions are narrowed to final_answer or blocked only.\n\
Do not call start_task, create_node, bind_node, list_files, read_file, search, apply_patch, run_test, or spawn_agent.\n\
The next response must summarize the exact validator infrastructure blocker and the implementation evidence already recorded."
        .to_string();
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_action_contract_tool_runtime_bootstrap_failure_item() -> ResponseItem {
    let text = "TaskSpaceActionContractToolRuntimeBootstrapFailureV1:\n\
Existing TaskSpace task has no active bound node because ordinary tools are blocked by sandbox/tool runtime bootstrap failure evidence.\n\
Current request allowed actions are narrowed to final_answer or blocked only.\n\
Do not call start_task, create_node, bind_node, list_files, read_file, search, apply_patch, run_test, or spawn_agent.\n\
The next response must summarize the exact sandbox/tool runtime blocker and the tool failure evidence already recorded."
        .to_string();
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_action_contract_inspect_unread_scripts_item(scripts: &[String]) -> ResponseItem {
    let mut text = String::from(
        "TaskSpaceActionContractInspectMissingScriptsV1:\n\
Inspect convergence is blocked because already-read script evidence references script(s) that have not been read yet.\n\
Current request allowed actions are narrowed to read_file or blocked only. Do not call list_files, search, apply_patch, run_test, taskspace_control finish_node, or re-read already inspected files before these missing script(s) are read.\n\
Required read_file targets:",
    );
    for script in scripts {
        text.push_str("\n- ");
        text.push_str(script);
    }
    if let Some(first) = scripts.first() {
        text.push_str("\nThe next action must be read_file for `");
        text.push_str(first);
        text.push_str("` unless an external blocker makes that impossible.");
    }
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_provider_budget_guidance_item(
    _request_count: usize,
    _max_requests: usize,
    _node_request_count: usize,
    _max_model_requests_per_node: usize,
    _request_phase: Option<&str>,
    _node_kind: Option<&str>,
    _node_id: Option<&str>,
    _tool_visibility: TaskspaceProviderToolVisibility,
) -> Option<ResponseItem> {
    return None;
}

fn build_taskspace_provider_budget_hard_stop_item(
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    decision: &crate::action_map::TaskSpaceBudgetGateDecision,
) -> ResponseItem {
    let blocking_items = if decision.blocking_items.is_empty() {
        "- (none)".to_string()
    } else {
        decision
            .blocking_items
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let next_valid_actions = if decision.next_valid_actions.is_empty() {
        "- stop provider sampling for this turn".to_string()
    } else {
        decision
            .next_valid_actions
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let node_id = snapshot
        .node_id
        .as_deref()
        .unwrap_or("provider-context-missing");
    let node_kind = snapshot.node_kind.as_deref().unwrap_or("unknown");
    let request_phase = snapshot.request_phase.as_deref().unwrap_or("unknown");
    let recovery_phase = decision
        .recovery_request_phase
        .as_deref()
        .unwrap_or("budget_recovery");
    let text = format!(
        "{TASKSPACE_PROVIDER_BUDGET_HARD_STOP_MARKER}\n\
reason: {reason}\n\
budget_state: {budget_state}\n\
node_id: {node_id}\n\
node_kind: {node_kind}\n\
request_phase: {request_phase}\n\
recovery_request_phase: {recovery_phase}\n\
request_count: {request_count}/{max_requests}\n\
node_request_count: {node_request_count}/{max_model_requests_per_node}\n\
post_budget_grace: {post_budget_grace_request_count}/{post_budget_grace_requests}\n\
quality_impact_required: {quality_impact_required}\n\
blocking_items:\n{blocking_items}\n\
next_valid_actions:\n{next_valid_actions}\n\
required_behavior:\n\
- Do not send another provider request for this turn.\n\
- Preserve bounded evidence and stop; a later turn may block the node or continue only after TaskSpace state changes.",
        reason = decision.reason,
        budget_state = decision.budget_state.as_str(),
        request_count = snapshot.request_count,
        max_requests = snapshot.max_requests,
        node_request_count = snapshot.node_request_count,
        max_model_requests_per_node = snapshot.max_model_requests_per_node,
        post_budget_grace_request_count = snapshot.post_budget_grace_request_count,
        post_budget_grace_requests = snapshot.post_budget_grace_requests,
        quality_impact_required = decision.quality_impact_required,
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_no_action_recovery_item(last_message: Option<&str>) -> ResponseItem {
    let previous = last_message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("(no assistant text was captured)");
    let gate_recovery = taskspace_gate_recovery_context(previous);
    let text = format!(
        "{TASKSPACE_NO_ACTION_RECOVERY_MARKER}\n\
The previous assistant message requested follow-up but did not produce effective TaskSpace progress: no successful tool result, taskspace_control transition, or final response accepted by TaskSpace was recorded.\n\
Previous assistant message: {previous}\n\
{gate_recovery}\
Required behavior for the next response:\n\
- Do not send commentary-only text such as \"let me check\".\n\
- Emit exactly one actionable operation now: a tool call, a taskspace_control finish/transition/state_commit, or a final blocked-with-evidence answer with the exact missing evidence.\n\
- If the current node is inspect_code_context and no source/test evidence has been read yet, call shell_command with `rg --files` now.\n\
- If inspect evidence is sufficient, finish the inspect node into implement_solution before more environment probing.\n\
- If a tool was blocked, follow the most recent TaskSpaceGateRecoveryV1 recovery instructions instead of repeating the same blocked action.\n\
- If a concrete file edit is identified, call apply_patch now before any further validation."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_no_action_recovery_hard_stop_item(
    recovery_item: &ResponseItem,
    attempt: usize,
    advisory_cap: usize,
) -> ResponseItem {
    let recovery_text = response_item_text(recovery_item).unwrap_or_default();
    let recovery_excerpt = recovery_text.chars().take(1800).collect::<String>();
    let text = format!(
        "{TASKSPACE_NO_ACTION_RECOVERY_HARD_STOP_MARKER}\n\
reason: repeated_no_action_after_recovery_threshold\n\
attempt_count: {attempt}\n\
advisory_threshold: {advisory_cap}\n\
The provider repeatedly returned follow-up-only text or recoverable action-contract output without producing effective TaskSpace progress.\n\
Runtime decision:\n\
- Stop provider sampling for this turn instead of spending the remaining provider budget on another generic no-action recovery request.\n\
- Preserve the last no-action recovery contract for audit.\n\
- A later turn may continue only after TaskSpace state changes or the provider emits a tool call, taskspace_control transition, or blocked-with-evidence result.\n\
Last recovery contract excerpt:\n{recovery_excerpt}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_gate_recovery_context(message: &str) -> String {
    let Some(marker_start) = message.find(TASKSPACE_GATE_RECOVERY_MARKER) else {
        return String::new();
    };
    let gate = message[marker_start..]
        .chars()
        .take(1800)
        .collect::<String>();
    format!(
        "Most recent blocked tool recovery context:\n{gate}\n\
Current recovery priority: obey the `next_valid_actions` in this TaskSpaceGateRecoveryV1 payload before generic no-action guidance.\n"
    )
}

fn taskspace_message_has_gate_recovery(message: Option<&str>) -> bool {
    message.is_some_and(|message| message.contains(TASKSPACE_GATE_RECOVERY_MARKER))
}

fn taskspace_message_has_gate_recovery_reason(message: Option<&str>, reason: &str) -> bool {
    message.is_some_and(|message| {
        message.contains(TASKSPACE_GATE_RECOVERY_MARKER) && message.contains(reason)
    })
}

fn taskspace_message_has_inspect_duplicate_successful_evidence(message: Option<&str>) -> bool {
    taskspace_message_has_gate_recovery_reason(
        message,
        "inspect_duplicate_successful_diagnostic_test",
    ) || taskspace_message_has_gate_recovery_reason(
        message,
        "inspect_duplicate_successful_read_or_search",
    )
}

fn taskspace_inspect_duplicate_successful_evidence_trigger(message: Option<&str>) -> &'static str {
    if taskspace_message_has_gate_recovery_reason(
        message,
        "inspect_duplicate_successful_read_or_search",
    ) {
        "inspect_duplicate_read_search_gate_recovery"
    } else {
        "inspect_duplicate_diagnostic_gate_recovery"
    }
}

fn taskspace_message_has_repeated_blocked_action(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        message.contains("\"repeated_blocked_action\"")
            || message.contains("repeated_blocked_action:")
    })
}

fn taskspace_repeated_duplicate_read_search_should_bootstrap(message: Option<&str>) -> bool {
    taskspace_message_has_repeated_blocked_action(message)
        && taskspace_message_has_gate_recovery_reason(
            message,
            "inspect_duplicate_successful_read_or_search",
        )
}

fn build_taskspace_duplicate_diagnostic_inspect_recovery_item(
    last_message: Option<&str>,
) -> ResponseItem {
    let previous = last_message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("(no assistant text was captured)");
    let gate_recovery = taskspace_gate_recovery_context(previous);
    let text = format!(
        "TaskSpaceDuplicateDiagnosticInspectRecoveryV1:\n\
The required diagnostic command has already completed successfully on this inspect_code_context node. The last blocked tool feedback only means the same diagnostic must not be rerun.\n\
Previous assistant/tool feedback: {previous}\n\
{gate_recovery}\
Current required behavior:\n\
- Do not rerun the blocked diagnostic command.\n\
- Treat the user's first-command diagnostic requirement as already satisfied by the recorded diagnostic result named in the blocked feedback.\n\
- The next action must not be run_test. Emit a read_file or search action for source/test evidence.\n\
- Do not finish this inspect node into implement_solution until source or test file contents have been read.\n\
- Emit exactly one inspect action now: read_file or search for the concrete source/test artifact named by recent file-list or diagnostic evidence.\n\
- If file-list evidence names both a source file and a test file, prefer reading the test file first, then the source file on the following turn.\n\
- Do not return blocked merely because the diagnostic cannot be rerun.\n\
Example action intent: action=read_file, node_id=<active inspect node id>, args.path=tests/test_example.py."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_duplicate_read_search_inspect_recovery_item(
    last_message: Option<&str>,
) -> ResponseItem {
    let previous = last_message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("(no assistant text was captured)");
    let gate_recovery = taskspace_gate_recovery_context(previous);
    let text = format!(
        "TaskSpaceDuplicateReadSearchInspectRecoveryV1:\n\
The requested read/search command has already completed successfully on this inspect_code_context node. Re-reading the same artifact is not new evidence.\n\
Previous assistant/tool feedback: {previous}\n\
{gate_recovery}\
Current required behavior:\n\
- Do not rerun the blocked read/search command.\n\
- Use the recorded result named in the blocked feedback as current evidence.\n\
- If enough source/test evidence has been read, emit taskspace_control with action=finish_node and next_node_kind=implement_solution.\n\
- Only emit another read_file or search if it targets a different concrete artifact needed for implementation.\n\
- Do not return blocked merely because the duplicate read/search was blocked."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_duplicate_inspect_successful_evidence_recovery_item(
    last_message: Option<&str>,
) -> ResponseItem {
    if taskspace_message_has_gate_recovery_reason(
        last_message,
        "inspect_duplicate_successful_read_or_search",
    ) {
        build_taskspace_duplicate_read_search_inspect_recovery_item(last_message)
    } else {
        build_taskspace_duplicate_diagnostic_inspect_recovery_item(last_message)
    }
}

fn build_taskspace_inspect_bootstrap_evidence_item(
    command: &str,
    response_item: &ResponseItem,
) -> ResponseItem {
    let output = match response_item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            function_call_output_body_text(&output.body)
        }
        _ => String::new(),
    };
    let output_preview = output.chars().take(6000).collect::<String>();
    let truncated_note = if output.chars().count() > 6000 {
        "\n[preview_truncated=true]"
    } else {
        ""
    };
    let text = format!(
        "TaskSpaceInspectBootstrapEvidenceV1:\n\
command: {command}\n\
The following bounded read/search evidence was collected automatically after the same diagnostic action was blocked repeatedly. Treat it as current inspect evidence; do not rerun the blocked diagnostic command. Use this evidence to choose the next legal action: read/search another concrete file, state_commit accepted findings, or finish the inspect node into implement_solution.\n\
result_preview:\n{output_preview}{truncated_note}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_apply_patch_format_recovery_item(targets: &str) -> ResponseItem {
    let targets = targets.trim();
    let targets = if targets.is_empty() {
        "(unknown existing file)"
    } else {
        targets
    };
    let text = format!(
        "{TASKSPACE_APPLY_PATCH_FORMAT_MARKER}\n\
The previous apply_patch attempted to add file(s) that already exist: {targets}\n\
Current required behavior:\n\
- Do not use /dev/null, Add File, or new file mode for those path(s).\n\
- Emit exactly one apply_patch now that updates the existing file(s).\n\
- For apply_patch grammar, use `*** Update File: <path>` hunks for existing files.\n\
- For unified diff input, use `--- a/<path>` and `+++ b/<path>` for existing files, never `--- /dev/null`.\n\
- If the inspected evidence named an invalid shebang, patch the first line of that existing file.\n\
- Do not call finish_node or validation until the update patch succeeds."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_apply_patch_missing_target_recovery_item(targets: &str) -> ResponseItem {
    let targets = targets.trim();
    let targets = if targets.is_empty() {
        "(unknown missing file)"
    } else {
        targets
    };
    let text = format!(
        "{TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER}\n\
The previous apply_patch tried to update missing file(s): {targets}\n\
This usually means the patch used unified-diff new-file syntax such as `--- /dev/null` / `+++ b/<path>`, which TaskSpace's native apply_patch grammar treats as an update.\n\
Current required behavior:\n\
- Do not return blocked merely because the file does not exist; missing files are created with native Add File syntax.\n\
- If the intended change is to create the file, emit exactly one apply_patch now using `*** Add File: <relative/path>` and prefix every content line with `+`.\n\
- If the intended change is to modify an already inspected existing artifact instead, emit exactly one apply_patch using `*** Update File: <relative/path>` for that existing artifact.\n\
- Do not use `--- /dev/null`, `+++ b/<path>`, or `@@ -0,0 +...` unified-diff add-file headers in native apply_patch.\n\
- Do not call read_file, list_files, search, finish_node, or validation until this edit succeeds."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_apply_patch_unanchored_update_recovery_item(targets: &str) -> ResponseItem {
    let targets = targets.trim();
    let targets = if targets.is_empty() {
        "(unknown updated file)"
    } else {
        targets
    };
    let text = format!(
        "{TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER}\n\
The previous apply_patch used `*** Update File` without a valid native update hunk for: {targets}\n\
That patch shape is ambiguous for an existing file: it can insert new text without replacing the broken code that validation reported, or it can send non-diff command text to the patch tool.\n\
Current required behavior:\n\
- Emit exactly one corrected apply_patch now.\n\
- For an in-place fix, include existing context lines and the exact `-old` / `+new` replacement lines.\n\
- If the file is small or generated and the full intended contents are known, use `*** Delete File: <path>` followed by `*** Add File: <path>` with the complete corrected file.\n\
- Do not put shell, Python, or JSON transformation commands inside the patch payload; apply_patch only accepts native diff content.\n\
- Do not call finish_node, create_node, read_file, search, or validation until this corrected edit succeeds."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_apply_patch_native_hunk_recovery_item(
    targets: &str,
    force_complete_replacement: bool,
) -> ResponseItem {
    let targets = targets.trim();
    let targets = if targets.is_empty() {
        "(unknown updated file)"
    } else {
        targets
    };
    let recovery_mode = if force_complete_replacement {
        "\
Current required behavior:\n\
- Emit exactly one corrected apply_patch now: a whole-file native replacement for the target above.\n\
- Use `*** Delete File: <relative/path>` followed by `*** Add File: <relative/path>` with the complete corrected file contents.\n\
- Prefix every added replacement line with `+`.\n\
- Do not emit `*** Update File` for this recovery; the previous attempts already repeated invalid unified/range hunks inside native update sections.\n\
- Do not put `--- a/...`, `+++ b/...`, or `@@ -old,+new @@` anywhere in the patch payload.\n\
- Do not call read_file, list_files, search, finish_node, or validation until this corrected edit succeeds."
    } else {
        "\
\tCurrent required behavior:\n\
\t- Emit exactly one corrected apply_patch now.\n\
\t- Use native `*** Update File: <relative/path>` with `@@` plus exact existing context and exact `-old` / `+new` lines.\n\
\t- Do not put `--- a/...`, `+++ b/...`, or `@@ -old,+new @@` anywhere after `*** Update File`; those are unified-diff markers, not native apply_patch hunks.\n\
\t- If you are unsure whether exact context still matches, prefer complete replacement with `*** Delete File: <path>` followed by `*** Add File: <path>` for small/generated files.\n\
\t- If the file is small or generated and the full intended contents are known, use `*** Delete File: <path>` followed by `*** Add File: <path>` with the complete corrected file.\n\
\t- Do not call read_file, list_files, search, finish_node, or validation until this corrected edit succeeds."
    };
    let text = format!(
        "{TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER}\n\
The previous apply_patch mixed native apply_patch grammar with unified-diff/range hunk syntax for: {targets}\n\
Native apply_patch does not use `--- Update File:`, `--- a/...`, `+++ b/...`, or `@@ -old,+new @@` range headers inside `*** Update File` sections.\n\
{recovery_mode}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_apply_patch_replacement_required_recovery_item(targets: &str) -> ResponseItem {
    build_taskspace_apply_patch_native_hunk_recovery_item(targets, true)
}

fn build_taskspace_patch_intent_format_recovery_item(
    evidence_summary: Option<&str>,
    raw_preview: Option<&str>,
) -> ResponseItem {
    let evidence = evidence_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let bullets = value
                .split(" | ")
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\nAlready inspected evidence available to use now:\n{bullets}\n")
        })
        .unwrap_or_default();
    let raw_preview = raw_preview
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\nRejected assistant output preview: {value}\n"))
        .unwrap_or_default();
    let text = format!(
        "{TASKSPACE_PATCH_INTENT_FORMAT_MARKER}\n\
The previous assistant response appeared to contain an apply_patch action, but TaskSpace rejected it because the response was not exactly one taskspace-action-v1 JSON object.\n\
{raw_preview}\
{evidence}\
Current required behavior:\n\
- Do not call read_file, list_files, search, broad shell discovery, or validation tests from this implementation node.\n\
- Emit exactly one valid taskspace-action-v1 JSON object now.\n\
- The JSON action must be apply_patch with the patch payload, or blocked with the exact reason the patch cannot be safely emitted.\n\
- Do not include markdown fences, prose before or after the JSON object, or a second action.\n\
- Use the file contents and failure clues already present in inspected evidence; do not rediscover them."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_forced_inspect_transition_recovery_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_FORCED_INSPECT_TRANSITION_MARKER}\n\
TaskSpace already transitioned the previous inspect_code_context node into implement_solution because inspected evidence was sufficient for a concrete implementation step.\n\
Current required behavior:\n\
- Do not run more shell/environment checks, test discovery, Docker inspection, or broad reads.\n\
- Do not emit commentary-only text such as \"let me check\".\n\
- Emit exactly one implementation action now: call apply_patch with the smallest concrete fix identified from inspect evidence.\n\
- If no safe edit can be made from the inspected evidence, answer blocked-with-evidence with the exact missing evidence."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_forced_implement_transition_recovery_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_FORCED_IMPLEMENT_TRANSITION_MARKER}\n\
TaskSpace already transitioned the previous implement_solution node into smoke_test because a successful edit was recorded and validation is now the next coherent step.\n\
Current required behavior:\n\
- Do not run more broad reads, environment discovery, or speculative edits.\n\
- Do not emit commentary-only text such as \"let me run tests\".\n\
- Emit exactly one focused validation tool call now.\n\
- If validation fails, report the validator evidence and continue from that concrete failure."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_forced_validation_closeout_recovery_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_FORCED_VALIDATION_CLOSEOUT_MARKER}\n\
TaskSpace already finished the current smoke_test/regression_test node because a successful validation tool result was recorded.\n\
Current required behavior:\n\
- Do not run more file discovery, reads, searches, or validation commands.\n\
- Do not create more TaskSpace nodes.\n\
- Emit exactly one final_answer now, summarizing that validation passed and the task result is ready."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_validation_infra_recovery_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER}\n\
The latest validation command failed because local validator infrastructure or the host shell failed, not because new code evidence was found.\n\
Current required behavior:\n\
- Do not run more bash, PowerShell, Docker, or shell-discovery commands for the same local validator failure.\n\
- Emit exactly one blocked taskspace-action-v1 JSON object for the current validation node.\n\
- The blocked reason must include the exact local infrastructure evidence, such as Bash/Service/CreateInstance/E_ACCESSDENIED.\n\
- Do not create another inspect node, re-read scripts, or retry validation before this validation node is closed."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_validation_needs_test_recovery_item(last_message: Option<&str>) -> ResponseItem {
    let gate_context = last_message
        .filter(|value| value.contains(TASKSPACE_GATE_RECOVERY_MARKER))
        .map(taskspace_gate_recovery_context)
        .unwrap_or_default();
    let last = last_message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            format!(
                "\nPrevious rejected validation action:\n- {}\n",
                taskspace_last_message_preview(Some(value)).unwrap_or_else(|| value.to_string())
            )
        })
        .unwrap_or_default();
    let text = format!(
        "{TASKSPACE_VALIDATION_NEEDS_TEST_MARKER}\n\
The current node is smoke_test/regression_test. Validation nodes must execute validation, not rediscover files.\n\
{gate_context}\
{last}\
Current required behavior:\n\
- Emit exactly one run_test action now, or blocked with the exact reason validation cannot run.\n\
- If the recovery context contains next_valid_actions, use the named command exactly before trying generic validators or alternate filenames.\n\
- Do not call list_files, read_file, search, broad shell discovery, apply_patch, or create another inspect node.\n\
- If a local validator such as scripts/validate.py was already discovered, run it directly, for example: {{\"schema_version\":\"taskspace-action-v1\",\"action\":\"run_test\",\"node_id\":\"<active node id>\",\"args\":{{\"command\":\"python scripts/validate.py\",\"timeout_ms\":120000}},\"rationale\":\"run discovered local validator\"}}.\n\
- After a successful run_test, finish validation or answer final according to the next TaskSpace guidance."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_implement_needs_edit_recovery_item(
    evidence_summary: Option<&str>,
) -> ResponseItem {
    let evidence = evidence_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let bullets = value
                .split(" | ")
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "\nAlready inspected evidence available to use now:\n{bullets}\n\
Coverage rule: if inspected evidence names a concrete artifact with a high-signal defect marker such as invalid shebang, traceback, syntax error, or command not found, the implementation must patch that artifact or return blocked with the exact reason it cannot be changed.\n"
            )
        })
        .unwrap_or_default();
    let text = format!(
        "{TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER}\n\
TaskSpace implement_solution has enough read/search evidence on the current node and no successful edit has been recorded.\n\
{evidence}\
Current required behavior:\n\
- Do not call read_file, list_files, search, broad shell discovery, or validation tests from this implementation node.\n\
- Emit exactly one implementation action now: call apply_patch with the smallest concrete fix supported by the inspected evidence.\n\
- If the evidence contains a validation failure, treat that failure as the primary target and patch the artifact named by that failure before making generic improvements.\n\
- If the failure is a top-level Python `IndentationError` in a generated file, fix the whole affected file or block indentation in one patch rather than patching a single import or line at a time.\n\
- If the failure is a `KeyError` or missing field, use only field names observed in the inspected schema/CSV/JSON evidence; do not invent unobserved columns.\n\
- Use the file contents and failure clues already present in inspected evidence; do not rediscover them.\n\
- If no safe edit can be made, return blocked with the exact missing evidence instead of reading the same files again."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_validation_rework_duplicate_read_recovery_item(
    last_message: Option<&str>,
    evidence_summary: Option<&str>,
    failed_edit_summary: Option<&str>,
) -> ResponseItem {
    let previous = last_message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("(no blocked read feedback was captured)");
    let previous_excerpt = previous.chars().take(2200).collect::<String>();
    let artifact = taskspace_validation_rework_duplicate_artifact(previous)
        .unwrap_or_else(|| "already-read validation rework artifact".to_string());
    let previous_result = taskspace_validation_rework_duplicate_previous_result(previous)
        .unwrap_or_else(|| "previous read result".to_string());
    let repair_contract = taskspace_validation_rework_repair_contract(previous)
        .map(|contract| format!("\nrepair_contract: {contract}\n"))
        .unwrap_or_default();
    let gate_recovery = taskspace_gate_recovery_context(previous);
    let evidence = evidence_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let bullets = value
                .split(" | ")
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\nAlready inspected evidence available to use now:\n{bullets}\n")
        })
        .unwrap_or_default();
    let failed_edit = failed_edit_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\nMost recent failed edit feedback to preserve:\n- {value}\n"))
        .unwrap_or_default();
    let text = format!(
        "{TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER}\n\
failure_kind: validation_rework_duplicate_artifact_read\n\
target_artifact: {artifact}\n\
previous_read_result: {previous_result}\n\
{repair_contract}\
{failed_edit}\
The previous action was blocked because this validation rework node already read the failure artifact and no successful edit has been recorded after that read.\n\
Current required behavior:\n\
- Emit exactly one taskspace-action-v1 apply_patch action targeting `{artifact}` now, using the current contents already visible in `{previous_result}` and the failed validation evidence.\n\
- Use native apply_patch grammar only: `*** Update File: <path>` with `@@` plus exact context and exact `-old` / `+new` lines, or `*** Delete File` followed by `*** Add File` for a complete small/generated rewrite. Do not include `--- Update File:`, `--- a/...`, `+++ b/...`, or `@@ -old,+new @@` range headers.\n\
- If the most recent failed edit feedback mentions `apply_patch_mixed_native_unified`, `apply_patch_native_hunk_header`, or `apply_patch_unanchored_update`, correct that patch grammar now; read_file/context refresh is not a valid recovery for that failure.\n\
- If repair_contract is present, satisfy it exactly before rerunning validation.\n\
- Do not call read_file, list_files, search, broad shell discovery, schema inspection, or validation from this implementation node before a successful edit is recorded.\n\
- If no safe edit can be made from the already visible evidence, emit exactly one taskspace_control block_node with the exact missing evidence or unsafe-edit reason.\n\
- Do not repeat the blocked read under a different rationale.\n\
- Use the evidence below only to construct the patch; do not treat it as permission to rediscover the same files.\n\
Previous blocked feedback:\n{previous_excerpt}\n\
{gate_recovery}\
{evidence}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_validation_rework_duplicate_read_hard_stop_item(
    recovery_item: &ResponseItem,
    attempt: usize,
) -> ResponseItem {
    let recovery_text = response_item_text(recovery_item).unwrap_or_default();
    let artifact = taskspace_validation_rework_duplicate_artifact(&recovery_text)
        .unwrap_or_else(|| "already-read validation rework artifact".to_string());
    let previous_result = taskspace_validation_rework_duplicate_previous_result(&recovery_text)
        .unwrap_or_else(|| "previous read result".to_string());
    let read_context = if recovery_text.contains("complete read_file context")
        || recovery_text.contains("read_context: complete_read")
        || recovery_text.contains("eof_reached=true")
    {
        "read_context: complete_read; complete read_file context already visible; no additional file lines are hidden\n"
    } else {
        ""
    };
    let recovery_excerpt = recovery_text.chars().take(1800).collect::<String>();
    let text = format!(
        "{TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_HARD_STOP_MARKER}\n\
reason: repeated_validation_rework_duplicate_artifact_read\n\
attempt_count: {attempt}\n\
target_artifact: {artifact}\n\
previous_read_result: {previous_result}\n\
{read_context}\
The current validation rework node repeatedly requested the same already-visible failure artifact after TaskSpace provided an apply_patch-or-block recovery contract.\n\
Runtime decision:\n\
- Stop provider sampling for this turn instead of issuing another advisory recovery request.\n\
- Preserve the bounded evidence and the last recovery contract for audit.\n\
- A later turn may continue only after TaskSpace state changes or the provider emits the required apply_patch/block_node action.\n\
Last recovery contract excerpt:\n{recovery_excerpt}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_validation_rework_patch_only_recovery_item(
    last_message: Option<&str>,
    evidence_summary: Option<&str>,
    failed_edit_summary: Option<&str>,
) -> ResponseItem {
    let previous = last_message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("(no blocked action feedback was captured)");
    let previous_excerpt = previous.chars().take(1600).collect::<String>();
    let target_artifacts =
        taskspace_validation_rework_patch_only_artifacts(evidence_summary.unwrap_or(""));
    let target_artifact_label = if target_artifacts.is_empty() {
        "validation rework target artifact".to_string()
    } else {
        target_artifacts.join(", ")
    };
    let evidence = evidence_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let bullets = value
                .split(" | ")
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\nAlready inspected evidence available to use now:\n{bullets}\n")
        })
        .unwrap_or_default();
    let failed_edit = failed_edit_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\nMost recent failed edit feedback to preserve:\n- {value}\n"))
        .unwrap_or_default();
    let complete_target_replacement =
        if taskspace_evidence_has_full_visible_validation_rework_target_read(evidence_summary) {
            format!(
                "\nComplete target-read direct replacement scaffold:\n\
- The target file is already fully visible (content_visibility=full_content_visible), so a full replacement patch is safe when a narrow hunk would be fragile.\n\
- For `{target_artifact_label}`, prefer one native apply_patch with `*** Delete File: {target_artifact_label}` followed by `*** Add File: {target_artifact_label}` when the repair changes multiple output construction fields.\n\
- Every added replacement line must be prefixed with `+`; do not wrap the patch in markdown or a shell command.\n"
            )
        } else {
            String::new()
        };
    let schema_repair_synthesis = taskspace_validation_rework_schema_repair_synthesis(
        evidence_summary,
        &target_artifact_label,
    );
    let text = format!(
        "{TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER}\n\
failure_kind: validation_rework_patch_only_after_target_read\n\
target_artifacts: {target_artifact_label}\n\
The previous action was blocked because this validation rework node already has the target file contents and validation repair contract needed for an edit.\n\
Current required behavior:\n\
- Emit exactly one taskspace-action-v1 apply_patch action targeting `{target_artifact_label}` now, or one taskspace_control block_node with the exact unsafe-edit reason.\n\
- Use the visible validation failure, schema repair contract, and validation_rework_target_read evidence already shown in context.\n\
- If the validation_rework_target_read evidence says content_visibility=full_content_visible, no additional file lines are hidden in the current projection; do not treat the displayed target-read excerpt as partial evidence.\n\
- Do not call read_file, list_files, search, broad shell discovery, schema inspection, or validation before a successful edit is recorded.\n\
- Do not move from the named target artifact to `schema.json` or another fact source; those facts are already present in evidence.\n\
- If no safe edit can be made from the already visible evidence, block explicitly instead of requesting more reads.\n\
- Use the evidence below only to construct the patch; do not treat it as permission to rediscover the same files.\n\
{schema_repair_synthesis}\
Patch construction scaffold:\n\
- Patch only `{target_artifact_label}` using the complete target read already in evidence.\n\
- For schema validation failures, convert `schema_property_rename_hints` into output key renames and convert each `missing_required_properties` entry into generated output fields derived from already-read fact sources.\n\
- For traceback/test failures, patch the named failing symbol, file, or output construction path shown in the validation failure.\n\
- Use native apply_patch grammar only. For a narrow edit use `*** Begin Patch`, `*** Update File: <target>`, context lines with `+`/`-` edits, and `*** End Patch`. For a complete replacement use `*** Delete File: <target>` followed by `*** Add File: <target>`.\n\
- Do not put markdown fences, shell commands, JSON generation scripts, or prose inside the patch payload.\n\
{complete_target_replacement}\
Previous blocked feedback:\n{previous_excerpt}\n\
{failed_edit}\
{evidence}\
Final action lock:\n\
- The target read above is complete when it says complete_read or eof_reached=true; projection truncation is not a valid reason to read `{target_artifact_label}` again.\n\
- If the most recent apply_patch failed with expected-lines, context-mismatch, or mixed unified/native hunk feedback and content_visibility=full_content_visible, use a whole-file native replacement for `{target_artifact_label}` (`*** Delete File` then `*** Add File`) from the visible target read instead of another fragile ranged hunk.\n\
- Emit apply_patch for `{target_artifact_label}` now, or block_node with the exact unsafe-edit reason. Do not emit read_file/list_files/search/schema inspection."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_validation_rework_schema_repair_synthesis(
    evidence_summary: Option<&str>,
    target_artifact_label: &str,
) -> String {
    let Some(evidence_summary) = evidence_summary else {
        return String::new();
    };
    let missing = taskspace_schema_repair_values(evidence_summary, "missing_required_properties=");
    let missing = if missing.is_empty() {
        taskspace_schema_repair_values(evidence_summary, "missing_required_properties:")
    } else {
        missing
    };
    let rename_hints =
        taskspace_schema_repair_values(evidence_summary, "schema_property_rename_hints=");
    if missing.is_empty() && rename_hints.is_empty() {
        return String::new();
    }

    let missing_label = if missing.is_empty() {
        "(none captured)".to_string()
    } else {
        missing
            .iter()
            .map(|property| format!("`{property}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let rename_label = if rename_hints.is_empty() {
        "no explicit rename hints were captured; infer the generated output location from the validation failure and complete target read".to_string()
    } else {
        rename_hints
            .iter()
            .map(|hint| format!("`{hint}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "Schema repair synthesis from current validation failure:\n\
- Missing required output properties to implement in `{target_artifact_label}` now: {missing_label}.\n\
- Apply captured output-key rename hints exactly when present: {rename_label}.\n\
- For every missing property without a rename hint, add a generated output field with the exact schema spelling and derive its value from already-read CSV/schema evidence.\n\
- This is a patch-construction requirement, not a reason to read schema/data/target files again.\n"
    )
}

fn taskspace_schema_repair_values(text: &str, marker: &str) -> Vec<String> {
    let mut values = Vec::new();
    for segment in text.split(['\n', '|']) {
        let Some((_, rest)) = segment.split_once(marker) else {
            continue;
        };
        let rest = rest
            .split(" schema_")
            .next()
            .unwrap_or(rest)
            .split(" target_artifacts")
            .next()
            .unwrap_or(rest)
            .split(" patch_requirement")
            .next()
            .unwrap_or(rest);
        for value in rest.split(',') {
            let value = value
                .trim()
                .trim_matches(|ch| matches!(ch, '`' | '"' | '\'' | '[' | ']' | '.' | ';'));
            if value.is_empty() || value.len() > 96 {
                continue;
            }
            if !values.iter().any(|existing| existing == value) {
                values.push(value.to_string());
            }
        }
    }
    values
}

fn build_taskspace_validation_rework_patch_only_hard_stop_item(
    recovery_item: &ResponseItem,
    attempt: usize,
) -> ResponseItem {
    let recovery_text = response_item_text(recovery_item).unwrap_or_default();
    let artifacts = taskspace_validation_rework_patch_only_artifacts(&recovery_text);
    let artifact_label = if artifacts.is_empty() {
        "validation rework target artifact".to_string()
    } else {
        artifacts.join(", ")
    };
    let recovery_excerpt = recovery_text.chars().take(1800).collect::<String>();
    let text = format!(
        "{TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_HARD_STOP_MARKER}\n\
reason: repeated_non_edit_after_validation_rework_target_read\n\
attempt_count: {attempt}\n\
target_artifacts: {artifact_label}\n\
The current validation rework node repeatedly requested read/search/discovery after TaskSpace had already shown the target file contents and patch-only repair contract.\n\
Runtime decision:\n\
- Stop provider sampling for this turn instead of issuing another advisory recovery request.\n\
- Preserve the bounded evidence and the last patch-only recovery contract for audit.\n\
- A later turn may continue only after TaskSpace state changes or the provider emits the required apply_patch/block_node action.\n\
Last recovery contract excerpt:\n{recovery_excerpt}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_implementation_needs_edit_hard_stop_item(
    recovery_item: &ResponseItem,
    attempt: usize,
) -> ResponseItem {
    let recovery_text = response_item_text(recovery_item).unwrap_or_default();
    let recovery_excerpt = recovery_text.chars().take(1800).collect::<String>();
    let text = format!(
        "{TASKSPACE_IMPLEMENT_NEEDS_EDIT_HARD_STOP_MARKER}\n\
reason: repeated_finish_without_successful_edit\n\
attempt_count: {attempt}\n\
The current implement_solution node repeatedly tried to finish or otherwise continue after TaskSpace said a successful edit result is required.\n\
Runtime decision:\n\
- Stop provider sampling for this turn instead of issuing another advisory recovery request.\n\
- Preserve the bounded evidence and the last apply_patch-or-block recovery contract for audit.\n\
- A later turn may continue only after TaskSpace state changes or the provider emits the required apply_patch/block_node action.\n\
Last recovery contract excerpt:\n{recovery_excerpt}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_apply_patch_recovery_hard_stop_item(
    recovery_item: &ResponseItem,
    attempt: usize,
) -> ResponseItem {
    let recovery_text = response_item_text(recovery_item).unwrap_or_default();
    let recovery_excerpt = recovery_text.chars().take(1800).collect::<String>();
    let text = format!(
        "{TASKSPACE_APPLY_PATCH_RECOVERY_HARD_STOP_MARKER}\n\
reason: repeated_failed_or_malformed_patch\n\
attempt_count: {attempt}\n\
The current implementation node repeatedly failed to recover from apply_patch grammar/context/tool feedback before recording a successful edit.\n\
Runtime decision:\n\
- Stop provider sampling for this turn instead of spending the remaining node budget on more malformed or stale-context patches.\n\
- Preserve the most recent apply_patch/edit-failure recovery contract for audit.\n\
- A later turn may continue only after TaskSpace state changes or the provider emits a valid native apply_patch/block_node action.\n\
Last recovery contract excerpt:\n\
{recovery_excerpt}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn build_taskspace_implementation_recovery_item(
    last_agent_message: Option<&str>,
    evidence_summary: Option<&str>,
    failed_edit_summary: Option<&str>,
) -> ResponseItem {
    if last_agent_message
        .is_some_and(taskspace_text_mentions_validation_rework_duplicate_artifact_read)
    {
        build_taskspace_validation_rework_duplicate_read_recovery_item(
            last_agent_message,
            evidence_summary,
            failed_edit_summary,
        )
    } else if failed_edit_summary.is_some() {
        build_taskspace_edit_failure_recovery_item(failed_edit_summary, evidence_summary)
    } else if taskspace_evidence_has_validation_rework_target_read(evidence_summary) {
        build_taskspace_validation_rework_patch_only_recovery_item(
            last_agent_message,
            evidence_summary,
            failed_edit_summary,
        )
    } else {
        build_taskspace_implement_needs_edit_recovery_item(evidence_summary)
    }
}

fn build_taskspace_edit_failure_recovery_item(
    failure_summary: Option<&str>,
    evidence_summary: Option<&str>,
) -> ResponseItem {
    let should_force_complete_rewrite = taskspace_failure_expected_lines_mismatch(failure_summary)
        && taskspace_evidence_has_full_visible_validation_rework_target_read(evidence_summary);
    let complete_rewrite = if should_force_complete_rewrite {
        "\nComplete target-read recovery override:\n- The validation rework target already has full visible target content (content_visibility=full_content_visible), so do not refresh read for context.\n- Because the previous apply_patch failed to find expected lines, stop using fragile Update File range/context hunks for this generated/small repair target.\n- Emit exactly one native apply_patch that replaces the target file with complete corrected contents: `*** Delete File: <path>` followed by `*** Add File: <path>` and every new file line prefixed with `+`.\n- Do not emit `*** Update File` for this recovery. Do not emit read_file/list_files/search/validation.\n"
    } else {
        ""
    };
    let recovery_action = if should_force_complete_rewrite {
        "- Emit exactly one recovery action now: a native apply_patch whole-file replacement for the failed target (`*** Delete File` followed by `*** Add File`). Do not call read_file; the full target content is already visible in evidence.\n\
- do not repeat the same hunk. The previous context/range hunk already failed against the real file.\n\
- If you cannot safely construct the complete replacement from the visible evidence, emit one taskspace_control block_node with the specific non-source-visibility blocker.\n\
- Do not use `*** Update File`, unified/range hunk headers (`@@ -...`), placeholder hunk headers, markdown fences, shell commands, or prose inside the patch payload.\n"
    } else {
        "- Emit exactly one recovery action now: a corrected apply_patch using the inspected existing artifact path and native apply_patch grammar, or one narrow read_file of the same failed target artifact only when the failed edit needs refreshed existing context.\n\
- If the failure says `Failed to find expected lines`, do not repeat the same hunk. Use exact existing context if known; for a small/generated file whose full intended contents are known, replace it with `*** Delete File: <path>` followed by `*** Add File: <path>` and the complete corrected contents. If the available source excerpt is truncated or stale after the failed edit, read the same target artifact once to refresh context, then patch.\n"
    };
    let failure = failure_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\nMost recent failed edit feedback:\n- {value}\n"))
        .unwrap_or_default();
    let structured_failure = taskspace_edit_failure_recovery_contract(failure_summary);
    let evidence = evidence_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let bullets = value
                .split(" | ")
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\nAlready inspected evidence available to use now:\n{bullets}\n")
        })
        .unwrap_or_default();
    let text = format!(
        "{TASKSPACE_EDIT_FAILURE_MARKER}\n\
	The previous edit tool call failed. Treat the tool result exactly like standard mode feedback: inspect the failure text, correct the patch target/grammar/context, and retry the edit if the intended change is still valid.\n\
	{failure}\
	{structured_failure}\
	{evidence}\
	{complete_rewrite}\
	Current required behavior:\n\
- Do not ignore the failed edit result.\n\
- Do not call list_files, search, broad shell discovery, unrelated read_file, or validation before resolving the failed edit.\n\
{recovery_action}\
	- If the failure says the target file is missing, use the already listed/read existing path when available."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_edit_failure_recovery_contract(failure_summary: Option<&str>) -> String {
    let Some(text) = failure_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return String::new();
    };
    let mut lines = Vec::new();
    if let Some(kind) = taskspace_tool_feedback_field(text, "failure_kind") {
        lines.push(format!("failure_kind: {kind}"));
    }
    if let Some(target) = taskspace_tool_feedback_field(text, "target") {
        lines.push(format!(
            "failed_target: {}",
            taskspace_normalize_apply_patch_target(&target)
        ));
    } else if let Some(target) = taskspace_expected_lines_target_from_apply_patch_text(text) {
        lines.push("failure_kind: apply_patch_expected_lines_mismatch".to_string());
        lines.push(format!("failed_target: {target}"));
    } else if let Some(target) = taskspace_context_mismatch_target_from_apply_patch_text(text) {
        lines.push("failure_kind: apply_patch_context_mismatch".to_string());
        lines.push(format!("failed_target: {target}"));
    } else if let Some(target) = taskspace_missing_update_targets_from_apply_patch_text(text) {
        lines.push("failure_kind: apply_patch_missing_update_target".to_string());
        lines.push(format!("failed_target: {target}"));
        if target.starts_with("app/") {
            let corrected = target.trim_start_matches("app/").to_string();
            if !corrected.is_empty() {
                lines.push(format!(
                    "path_correction: use `{corrected}`, not `{target}`"
                ));
            }
        }
    } else if let Some(targets) = taskspace_native_hunk_targets_from_rejection(Some(text)) {
        lines.push("failure_kind: apply_patch_native_hunk_header".to_string());
        lines.push(format!("failed_target: {targets}"));
    } else if taskspace_apply_patch_invalid_hunk_looks_unified(text) {
        lines.push("failure_kind: apply_patch_unified_hunk_header_in_native_patch".to_string());
    }
    if lines.is_empty() {
        return String::new();
    }
    if lines.iter().any(|line| {
        line.contains("apply_patch_expected_lines_mismatch")
            || line.contains("apply_patch_context_mismatch")
    }) {
        lines.push("mandatory_next_action: do not repeat the failed hunk; use exact current context or a complete native Delete File/Add File replacement for a small/generated target".to_string());
    }
    if lines.iter().any(|line| {
        line.contains("apply_patch_native_hunk_header")
            || line.contains("apply_patch_unified_hunk_header_in_native_patch")
    }) {
        lines.push("mandatory_next_action: remove all unified-diff markers (`--- a/...`, `+++ b/...`, `@@ -old,+new @@`) from native apply_patch".to_string());
    }
    format!("\nStructured failed-edit contract:\n{}\n", lines.join("\n"))
}

fn taskspace_tool_feedback_field(text: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix(&prefix)?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn taskspace_failure_expected_lines_mismatch(failure_summary: Option<&str>) -> bool {
    failure_summary
        .map(|value| value.to_ascii_lowercase())
        .is_some_and(|value| {
            value.contains("failed to find expected lines")
                || value.contains("apply_patch_expected_lines_mismatch")
                || value.contains("apply_patch_context_mismatch")
        })
}

fn taskspace_evidence_has_full_visible_validation_rework_target_read(
    evidence_summary: Option<&str>,
) -> bool {
    evidence_summary
        .map(|value| value.to_ascii_lowercase())
        .is_some_and(|value| {
            value.contains("validation_rework_target_read")
                && value.contains("content_visibility")
                && value.contains("full_content_visible")
        })
}

fn is_taskspace_no_action_recovery_item(item: &ResponseItem) -> bool {
    if is_taskspace_no_action_recovery_hard_stop_item(item) {
        return false;
    }
    response_item_text_contains(item, TASKSPACE_NO_ACTION_RECOVERY_MARKER)
}

fn is_taskspace_no_action_recovery_hard_stop_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_NO_ACTION_RECOVERY_HARD_STOP_MARKER)
}

fn is_taskspace_provider_budget_hard_stop_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_PROVIDER_BUDGET_HARD_STOP_MARKER)
}

fn is_taskspace_validation_rework_duplicate_read_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER)
}

fn is_taskspace_validation_rework_duplicate_read_hard_stop_item(item: &ResponseItem) -> bool {
    response_item_text_contains(
        item,
        TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_HARD_STOP_MARKER,
    )
}

fn is_taskspace_validation_rework_patch_only_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER)
}

fn is_taskspace_validation_rework_patch_only_hard_stop_item(item: &ResponseItem) -> bool {
    response_item_text_contains(
        item,
        TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_HARD_STOP_MARKER,
    )
}

fn is_taskspace_implementation_needs_edit_hard_stop_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_HARD_STOP_MARKER)
}

fn is_taskspace_apply_patch_recovery_hard_stop_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_APPLY_PATCH_RECOVERY_HARD_STOP_MARKER)
}

fn is_taskspace_apply_patch_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_FORMAT_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER)
        || response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER)
}

fn taskspace_validation_rework_duplicate_read_should_hard_stop(
    item: &ResponseItem,
    previous_recovery_count: usize,
) -> bool {
    is_taskspace_validation_rework_duplicate_read_recovery_item(item)
        && (previous_recovery_count > 0
            || response_item_text_contains(item, "\"repeated_blocked_action\"")
            || response_item_text_contains(item, "repeated_blocked_action:"))
}

fn taskspace_validation_rework_patch_only_should_hard_stop(
    item: &ResponseItem,
    previous_recovery_count: usize,
) -> bool {
    if !is_taskspace_validation_rework_patch_only_recovery_item(item) {
        return false;
    }
    let closed_action_rejection = response_item_text_contains(
        item,
        "validation_rework_closed_action_space_read_disallowed",
    );
    if closed_action_rejection {
        previous_recovery_count > 1
    } else {
        previous_recovery_count > 0
    }
}

fn taskspace_apply_patch_recovery_should_hard_stop(
    item: &ResponseItem,
    previous_recovery_count: usize,
) -> bool {
    is_taskspace_apply_patch_recovery_item(item) && previous_recovery_count >= 3
}

fn taskspace_no_action_recovery_should_hard_stop(
    item: &ResponseItem,
    current_recovery_count: usize,
    advisory_cap: usize,
) -> bool {
    is_taskspace_no_action_recovery_item(item) && current_recovery_count > advisory_cap
}

fn taskspace_recovery_snapshot_node_key(
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) -> String {
    snapshot
        .and_then(|snapshot| snapshot.node_id.as_deref())
        .unwrap_or("unknown-node")
        .to_string()
}

fn taskspace_reset_recovery_count_for_snapshot_node(
    current_key: &mut Option<String>,
    recovery_count: &mut usize,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) {
    let recovery_key = taskspace_recovery_snapshot_node_key(snapshot);
    if current_key.as_deref() != Some(recovery_key.as_str()) {
        *current_key = Some(recovery_key);
        *recovery_count = 0;
    }
}

fn taskspace_implementation_needs_edit_should_hard_stop(
    item: &ResponseItem,
    current_node_recovery_count: usize,
) -> bool {
    is_taskspace_plain_implement_needs_edit_recovery_item(item) && current_node_recovery_count >= 3
}

fn taskspace_provider_budget_limit_reached(
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> bool {
    (snapshot.max_requests > 0 && snapshot.request_count >= snapshot.max_requests)
        || (snapshot.max_model_requests_per_node > 0
            && snapshot.node_request_count >= snapshot.max_model_requests_per_node)
}

fn is_taskspace_plain_implement_needs_edit_recovery_item(item: &ResponseItem) -> bool {
    if is_taskspace_implementation_needs_edit_hard_stop_item(item) {
        return false;
    }
    response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER)
}

fn is_taskspace_implement_needs_edit_recovery_item(item: &ResponseItem) -> bool {
    if is_taskspace_apply_patch_recovery_hard_stop_item(item) {
        return false;
    }
    if is_taskspace_validation_rework_duplicate_read_hard_stop_item(item) {
        return false;
    }
    if is_taskspace_implementation_needs_edit_hard_stop_item(item) {
        return false;
    }
    response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER)
        || response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER)
        || response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER)
        || response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER)
}

fn taskspace_implement_recovery_advisory_warning_message(
    item: &ResponseItem,
    attempt: usize,
) -> String {
    if response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER) {
        format!(
            "TaskSpace inserted TaskSpacePatchIntentFormatRecoveryV1 because an apply_patch intent was rejected for non-strict JSON and must be re-emitted as one strict action. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_FORMAT_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchFormatRecoveryV1 because apply_patch tried to add an existing file and must be re-emitted as an update. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchMissingTargetRecoveryV1 because apply_patch tried to update a missing file and must be re-emitted with Add File or the correct existing target. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1 because apply_patch used an unanchored Update File patch and must be re-emitted with exact context. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchNativeHunkRecoveryV1 because apply_patch mixed native grammar with unified/range hunk syntax and must be re-emitted in native apply_patch grammar. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceEditFailureRecoveryV1 because the previous edit tool call failed and the model must use that tool feedback to retry or block. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER)
        && taskspace_duplicate_read_recovery_preserves_patch_grammar_failure(item)
    {
        format!(
            "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1 because validation rework repeated an already-read artifact after a patch grammar failure, and the model must preserve the failed edit feedback while re-emitting native apply_patch grammar. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadRecoveryV1 because validation rework already has the target file contents and the model must patch or block instead of reading again. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceValidationReworkPatchOnlyRecoveryV1 because validation rework already has target file contents plus a repair contract, so read/search/schema inspection is no longer valid before apply_patch or block. Advisory recovery attempt {attempt} is being used."
        )
    } else {
        format!(
            "TaskSpace inserted TaskSpaceImplementNeedsEditRecoveryV1 because implementation has enough read/search evidence and must edit or block. Advisory recovery attempt {attempt} is being used."
        )
    }
}

fn taskspace_duplicate_read_recovery_preserves_patch_grammar_failure(item: &ResponseItem) -> bool {
    response_item_text_contains(item, "Most recent failed edit feedback to preserve")
        && response_item_texts_contain(item, &|text| {
            text.contains("apply_patch_mixed_native_unified")
                || text.contains("apply_patch_native_hunk_header")
                || text.contains("apply_patch_unanchored_update")
        })
}

fn taskspace_special_recovery_warning_message(item: &ResponseItem) -> String {
    if response_item_text_contains(item, TASKSPACE_PROVIDER_BUDGET_HARD_STOP_MARKER) {
        "TaskSpace inserted TaskSpaceProviderBudgetHardStopV1 because provider request budget was exhausted before dispatch. The current turn will stop without another model request.".to_string()
    } else if is_taskspace_no_action_recovery_hard_stop_item(item) {
        "TaskSpace inserted TaskSpaceNoActionRecoveryHardStopV1 because no-action recovery exceeded its advisory threshold without effective TaskSpace progress. The current turn will stop without another model request.".to_string()
    } else if is_taskspace_validation_rework_duplicate_read_hard_stop_item(item) {
        "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadHardStopV1 because validation rework repeated an already-blocked artifact read after patch-only recovery. The current turn will stop without another model request.".to_string()
    } else if is_taskspace_validation_rework_patch_only_hard_stop_item(item) {
        "TaskSpace inserted TaskSpaceValidationReworkPatchOnlyHardStopV1 because validation rework repeated read/search/discovery after target contents were already visible and only apply_patch/block was valid. The current turn will stop without another model request.".to_string()
    } else if is_taskspace_implementation_needs_edit_hard_stop_item(item) {
        "TaskSpace inserted TaskSpaceImplementationNeedsEditHardStopV1 because implementation repeatedly tried to finish without a successful edit after apply_patch-or-block recovery. The current turn will stop without another model request.".to_string()
    } else if is_taskspace_apply_patch_recovery_hard_stop_item(item) {
        "TaskSpace inserted TaskSpaceApplyPatchRecoveryHardStopV1 because apply_patch/edit-failure recovery repeated without a successful edit. The current turn will stop without another model request.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_FORCED_IMPLEMENT_TRANSITION_MARKER) {
        "TaskSpace inserted TaskSpaceForcedImplementTransitionRecoveryV1 after a provider-budget forced implement transition. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_FORMAT_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchFormatRecoveryV1 after apply_patch tried to add an existing file. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchMissingTargetRecoveryV1 after apply_patch tried to update a missing file. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1 after apply_patch used an unanchored Update File patch. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchNativeHunkRecoveryV1 after apply_patch mixed native grammar with unified/range hunk syntax. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER) {
        "TaskSpace inserted TaskSpaceEditFailureRecoveryV1 after an edit tool call failed. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER) {
        "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadRecoveryV1 after a validation rework node repeated an already-read failure artifact. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER) {
        "TaskSpace inserted TaskSpaceValidationReworkPatchOnlyRecoveryV1 after a validation rework node requested read/search/discovery even though target contents were already visible. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER) {
        "TaskSpace inserted TaskSpaceImplementNeedsEditRecoveryV1 because implementation has enough read/search evidence and must edit or block. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER) {
        "TaskSpace inserted TaskSpacePatchIntentFormatRecoveryV1 after an apply_patch intent was rejected for non-strict JSON. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER) {
        "TaskSpace inserted TaskSpaceValidationInfraRecoveryV1 after local validator infrastructure failed. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_NEEDS_TEST_MARKER) {
        "TaskSpace inserted TaskSpaceValidationNeedsTestRecoveryV1 after a validation node needed a concrete coverage-correct run_test. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_FORCED_INSPECT_TRANSITION_MARKER) {
        "TaskSpace inserted TaskSpaceForcedInspectTransitionRecoveryV1 after a provider-budget forced inspect transition. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_FORCED_VALIDATION_CLOSEOUT_MARKER) {
        "TaskSpace inserted TaskSpaceForcedValidationCloseoutRecoveryV1 after successful validation was finished automatically. This guidance does not consume the no-action recovery allowance.".to_string()
    } else {
        "TaskSpace inserted non-cap TaskSpace recovery guidance. This guidance does not consume the no-action recovery allowance.".to_string()
    }
}

fn taskspace_message_hit_implementation_needs_edit(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        message.contains("implementation_needs_edit")
            || message.contains("validation_rework_patch_only_after_target_read")
            || message.contains("validation_rework_closed_action_space_read_disallowed")
            || taskspace_text_mentions_validation_rework_duplicate_artifact_read(message)
            || message.contains("has enough read/search evidence and no successful edit")
    })
}

fn taskspace_evidence_has_validation_rework_target_read(evidence_summary: Option<&str>) -> bool {
    evidence_summary.is_some_and(|text| text.contains("validation_rework_target_read"))
}

fn taskspace_validation_rework_patch_only_artifacts(text: &str) -> Vec<String> {
    let explicit_targets = taskspace_validation_rework_explicit_target_artifacts(text);
    if !explicit_targets.is_empty() {
        return explicit_targets;
    }

    let mut artifacts = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        if !line.contains("validation_rework_target_read") {
            continue;
        }
        for part in line.split_whitespace() {
            let value = part
                .strip_prefix("artifact=")
                .or_else(|| part.strip_prefix("artifacts="));
            if let Some(value) = value {
                for artifact in value.split(',') {
                    let artifact = taskspace_clean_artifact_token(artifact);
                    if !artifact.is_empty() && seen.insert(artifact.clone()) {
                        artifacts.push(artifact);
                    }
                }
            }
        }
    }
    artifacts
}

fn taskspace_validation_rework_explicit_target_artifacts(text: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    let mut seen = HashSet::new();
    for marker in ["target_artifacts=", "target_artifacts:"] {
        for segment in text.split('|') {
            let Some((_, rest)) = segment.split_once(marker) else {
                continue;
            };
            let rest = rest
                .split(" patch_requirement")
                .next()
                .unwrap_or(rest)
                .split(" schema_")
                .next()
                .unwrap_or(rest);
            for artifact in rest.split(',') {
                let artifact = taskspace_clean_artifact_token(artifact);
                if !artifact.is_empty() && seen.insert(artifact.clone()) {
                    artifacts.push(artifact);
                }
            }
        }
    }
    artifacts
}

fn taskspace_clean_artifact_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch| matches!(ch, ',' | ';' | ':' | '`' | '\'' | '"' | '[' | ']'))
        .trim()
        .to_string()
}

fn taskspace_text_mentions_validation_rework_duplicate_artifact_read(text: &str) -> bool {
    text.contains("validation_rework_duplicate_artifact_read")
        || (text.contains("validation rework node")
            && text.contains("already read failure artifact")
            && text.contains("no successful edit"))
}

fn taskspace_validation_rework_duplicate_artifact(text: &str) -> Option<String> {
    taskspace_backtick_value_after(text, "failure artifact `")
}

fn taskspace_validation_rework_duplicate_previous_result(text: &str) -> Option<String> {
    taskspace_backtick_value_after(text, "in result `")
}

fn taskspace_validation_rework_repair_contract(text: &str) -> Option<String> {
    for line in text.lines() {
        for marker in [
            "Validation repair contract:",
            "validation_schema_repair_contract:",
            "validation_rework_contract:",
        ] {
            let Some((_, rest)) = line.split_once(marker) else {
                continue;
            };
            let contract = rest
                .trim()
                .trim_matches(',')
                .trim_matches('"')
                .trim_matches('`')
                .trim();
            if !contract.is_empty() {
                return Some(contract.to_string());
            }
        }
    }
    None
}

fn taskspace_backtick_value_after(text: &str, marker: &str) -> Option<String> {
    let (_, rest) = text.split_once(marker)?;
    let (value, _) = rest.split_once('`')?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn taskspace_message_hit_validation_needs_test(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        let validation_coverage_gate = message.contains(TASKSPACE_GATE_RECOVERY_MARKER)
            && (message.contains("validation_test_missing_changed_artifact_coverage")
                || message.contains("validation_test_missing_local_validator_coverage")
                || message.contains("validation_test_missing_output_contract_coverage"));
        (message.contains("node_policy_violation:smoke_test:")
            || message.contains("node_policy_violation:regression_test:"))
            && (message.contains(":list_files")
                || message.contains(":read_file")
                || message.contains(":search"))
            || validation_coverage_gate
            || taskspace_text_mentions_current_validation_test_required(message)
    })
}

fn taskspace_text_mentions_current_validation_test_required(text: &str) -> bool {
    taskspace_text_mentions_stale_validation_failure_without_current_test(text)
        || taskspace_text_mentions_validation_finish_missing_current_test(text)
        || text.contains("validation_stale_failure_without_current_test")
        || text.contains("validation_finish_missing_current_test_result")
}

fn taskspace_text_mentions_stale_validation_failure_without_current_test(text: &str) -> bool {
    (text.contains("cannot be blocked as failed validation")
        || text.contains("cannot be finished as failed validation"))
        && text.contains("before this node records a test/build result")
}

fn taskspace_text_mentions_validation_finish_missing_current_test(text: &str) -> bool {
    text.contains("cannot be completed without a recorded successful test or build action")
}

fn taskspace_message_hit_apply_patch_intent_format_rejection(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        message.contains("action_contract_output_not_strict_json:apply_patch_intent")
    })
}

fn taskspace_rejected_apply_patch_intent_preview(message: Option<&str>) -> Option<&str> {
    let message = message?;
    let (_, preview) = message.split_once("Rejected assistant output preview:")?;
    let preview = preview
        .split_once(". Return exactly")
        .map(|(preview, _)| preview)
        .unwrap_or(preview)
        .trim();
    (!preview.is_empty()).then_some(preview)
}

fn taskspace_raw_text_mentions_apply_patch_intent(raw_text: &str) -> bool {
    let raw_text = raw_text.to_ascii_lowercase();
    raw_text.contains("\"action\"")
        && raw_text.contains("apply_patch")
        && raw_text.contains("taskspace-action-v1")
}

fn taskspace_existing_file_add_targets_from_rejection(message: Option<&str>) -> Option<String> {
    let message = message?;
    let (_, rest) = message.split_once("apply_patch_existing_file_as_add:")?;
    let targets = rest
        .split_once(". Return exactly")
        .map(|(targets, _)| targets)
        .unwrap_or(rest)
        .trim();
    (!targets.is_empty()).then(|| targets.to_string())
}

fn taskspace_unanchored_update_targets_from_rejection(message: Option<&str>) -> Option<String> {
    let message = message?;
    let (_, rest) = message.split_once("apply_patch_unanchored_update:")?;
    let targets = rest
        .split_once(". Return exactly")
        .map(|(targets, _)| targets)
        .unwrap_or(rest)
        .trim();
    (!targets.is_empty()).then(|| targets.to_string())
}

fn taskspace_native_hunk_targets_from_rejection(message: Option<&str>) -> Option<String> {
    let message = message?;
    for marker in [
        "apply_patch_mixed_native_unified:",
        "apply_patch_native_hunk_header:",
    ] {
        let Some((_, rest)) = message.split_once(marker) else {
            continue;
        };
        let targets = rest
            .split_once(". Return exactly")
            .map(|(targets, _)| targets)
            .unwrap_or(rest)
            .trim();
        if !targets.is_empty() {
            return Some(targets.to_string());
        }
    }
    None
}

fn taskspace_replacement_required_targets_from_rejection(message: Option<&str>) -> Option<String> {
    let message = message?;
    let (_, rest) = message.split_once("apply_patch_replacement_required:")?;
    let targets = rest
        .split_once(". Return exactly")
        .map(|(targets, _)| targets)
        .unwrap_or(rest)
        .trim();
    (!targets.is_empty()).then(|| targets.to_string())
}

fn taskspace_missing_update_targets_from_apply_patch_error(
    message: Option<&str>,
) -> Option<String> {
    let message = message?;
    if !message.contains("TaskSpace tool call failed: apply_patch verification failed")
        || !message.contains("Failed to read file to update ")
    {
        return None;
    }
    taskspace_missing_update_targets_from_apply_patch_text(message)
}

fn taskspace_missing_update_targets_from_apply_patch_text(message: &str) -> Option<String> {
    if !message.contains("apply_patch verification failed")
        || !message.contains("Failed to read file to update ")
    {
        return None;
    }
    let (_, rest) = message.split_once("Failed to read file to update ")?;
    let target = rest
        .split_once(" (os error")
        .map(|(target, _)| target)
        .unwrap_or(rest)
        .rsplit_once(':')
        .map(|(target, _)| target)
        .unwrap_or(rest)
        .trim();
    let target = taskspace_trim_apply_patch_error_target(target);
    let target = taskspace_normalize_apply_patch_target(target);
    (!target.is_empty()).then_some(target)
}

fn taskspace_expected_lines_target_from_apply_patch_text(message: &str) -> Option<String> {
    if !message.contains("apply_patch verification failed")
        || !message.contains("Failed to find expected lines in ")
    {
        return None;
    }
    let (_, rest) = message.split_once("Failed to find expected lines in ")?;
    let target = rest
        .lines()
        .next()
        .unwrap_or(rest)
        .trim()
        .trim_end_matches(':');
    let target = taskspace_trim_apply_patch_error_target(target);
    let target = taskspace_normalize_apply_patch_target(target);
    (!target.is_empty()).then_some(target)
}

fn taskspace_context_mismatch_target_from_apply_patch_text(message: &str) -> Option<String> {
    if !message.contains("apply_patch verification failed")
        || !message.contains("Failed to find context ")
        || !message.contains(" in ")
    {
        return None;
    }
    let (_, rest) = message.rsplit_once(" in ")?;
    let target = rest
        .lines()
        .next()
        .unwrap_or(rest)
        .trim()
        .trim_end_matches(':');
    let target = taskspace_trim_apply_patch_error_target(target);
    let target = taskspace_normalize_apply_patch_target(target);
    (!target.is_empty()).then_some(target)
}

fn taskspace_apply_patch_invalid_hunk_looks_unified(message: &str) -> bool {
    message.contains("apply_patch verification failed")
        && message.contains("invalid hunk")
        && (message.contains("@@ -") || message.contains("@@ +"))
}

fn taskspace_normalize_apply_patch_target(target: &str) -> String {
    let normalized = target.trim().trim_matches('"').replace('\\', "/");
    if let Some((_, relative)) = normalized.split_once("/app/src/") {
        return relative.trim_matches('/').to_string();
    }
    if let Some((_, relative)) = normalized.split_once("/app/") {
        return taskspace_strip_common_workspace_patch_prefix(relative.trim_matches('/'))
            .to_string();
    }
    if let Some((_, relative)) = normalized.split_once("/workspace/") {
        return taskspace_strip_common_workspace_patch_prefix(relative.trim_matches('/'))
            .to_string();
    }
    taskspace_strip_common_workspace_patch_prefix(normalized.trim_matches('/')).to_string()
}

fn taskspace_trim_apply_patch_error_target(target: &str) -> &str {
    let trimmed = target.trim();
    for ext in [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".json", ".toml", ".yaml", ".yml", ".sh",
    ] {
        let marker = format!("{ext}:");
        if let Some(index) = trimmed.find(&marker) {
            return &trimmed[..index + ext.len()];
        }
    }
    trimmed
}

fn taskspace_strip_common_workspace_patch_prefix(path: &str) -> &str {
    path.strip_prefix("app/")
        .or_else(|| path.strip_prefix("./app/"))
        .unwrap_or(path)
}

fn taskspace_snapshot_requires_implementation_edit(
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> bool {
    snapshot.node_kind.as_deref() == Some("implement_solution")
        && (snapshot.current_node_has_uncovered_mandatory_evidence
            || (!snapshot.current_node_has_successful_edit
                && (snapshot.current_node_has_dependency_working_evidence
                    || snapshot
                        .current_node_progress_signature
                        .is_some_and(|progress| {
                            progress >= TASKSPACE_IMPLEMENT_PROGRESS_BEFORE_EDIT_HINT
                        }))))
}

fn taskspace_no_action_recovery_cap_for_node_kind(node_kind: &str) -> usize {
    match node_kind {
        "inspect_code_context" => 3,
        "implement_solution" => 3,
        _ => 1,
    }
}

fn taskspace_budget_pressure_follow_up_intent(
    request_count: usize,
    max_requests: usize,
    node_kind: Option<&str>,
    last_message: Option<&str>,
) -> bool {
    if !taskspace_budget_pressure_active_for_node_kind(request_count, max_requests, node_kind) {
        return false;
    }
    let Some(message) = last_message else {
        return false;
    };
    taskspace_budget_pressure_message_has_follow_up_intent(message)
}

fn taskspace_budget_pressure_silent_action_requires_transition(
    request_count: usize,
    max_requests: usize,
    node_kind: Option<&str>,
    assistant_message_present: bool,
    saw_actionable_output: bool,
) -> bool {
    if !matches!(
        node_kind,
        Some("inspect_code_context" | "implement_solution")
    ) || assistant_message_present
        || !saw_actionable_output
    {
        return false;
    }
    taskspace_budget_pressure_active_for_node_kind(request_count, max_requests, node_kind)
}

fn taskspace_budget_pressure_active_for_node_kind(
    _request_count: usize,
    _max_requests: usize,
    _node_kind: Option<&str>,
) -> bool {
    false
}

fn taskspace_budget_pressure_message_has_follow_up_intent(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    const FOLLOW_UP_MARKERS: &[&str] = &[
        "let me check",
        "let me verify",
        "let me inspect",
        "let me run",
        "let me try",
        "i need to check",
        "i should check",
        "try running",
        "\u{5148}\u{8dd1}",
        "\u{8dd1}\u{6d4b}\u{8bd5}",
        "\u{8fd0}\u{884c}\u{6d4b}\u{8bd5}",
        "\u{6267}\u{884c}\u{6d4b}\u{8bd5}",
        "\u{786e}\u{8ba4}\u{5f53}\u{524d}\u{5931}\u{8d25}",
    ];
    FOLLOW_UP_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn classify_taskspace_provider_response_actionability(
    needs_follow_up: bool,
    saw_actionable_output: bool,
    assistant_message_present: bool,
    gate_recovery_message_present: bool,
    final_response_rejected: bool,
    _provider_budget_exhausted_followup: bool,
) -> TaskspaceProviderResponseActionability {
    if final_response_rejected {
        TaskspaceProviderResponseActionability::FinalRejected
    } else if gate_recovery_message_present {
        TaskspaceProviderResponseActionability::NoActionFollowUp
    } else if saw_actionable_output {
        TaskspaceProviderResponseActionability::Actionable
    } else if needs_follow_up && assistant_message_present {
        TaskspaceProviderResponseActionability::NoActionFollowUp
    } else if needs_follow_up {
        TaskspaceProviderResponseActionability::EmptyFollowUp
    } else {
        TaskspaceProviderResponseActionability::FinalCandidate
    }
}

fn taskspace_active_node_empty_response_requires_follow_up(
    node_kind: Option<&str>,
    saw_actionable_output: bool,
    assistant_message_present: bool,
    taskspace_terminal_action_observed_in_request: bool,
) -> bool {
    node_kind.is_some()
        && !saw_actionable_output
        && !assistant_message_present
        && !taskspace_terminal_action_observed_in_request
}

fn taskspace_last_message_preview(message: Option<&str>) -> Option<String> {
    let message = message?.trim();
    if message.is_empty() {
        return None;
    }
    let preview = message
        .chars()
        .take(160)
        .collect::<String>()
        .replace(['\r', '\n', '\t'], " ");
    Some(preview)
}

fn taskspace_message_requests_validation(message: Option<&str>) -> bool {
    let Some(message) = message else {
        return false;
    };
    let message = message.to_ascii_lowercase();
    (message.contains("run") || message.contains("execute"))
        && (message.contains("test") || message.contains("pytest") || message.contains("confirm"))
}

fn filter_deferred_dynamic_tool_spec(
    spec: ToolSpec,
    deferred_dynamic_tools: &HashSet<ToolName>,
) -> Option<ToolSpec> {
    match spec {
        ToolSpec::Function(tool) => {
            if deferred_dynamic_tools.contains(&ToolName::plain(tool.name.as_str())) {
                None
            } else {
                Some(ToolSpec::Function(tool))
            }
        }
        ToolSpec::Namespace(mut namespace) => {
            let namespace_name = namespace.name.clone();
            namespace.tools.retain(|tool| match tool {
                ResponsesApiNamespaceTool::Function(tool) => !deferred_dynamic_tools.contains(
                    &ToolName::namespaced(namespace_name.as_str(), tool.name.as_str()),
                ),
            });
            if namespace.tools.is_empty() {
                None
            } else {
                Some(ToolSpec::Namespace(namespace))
            }
        }
        spec => Some(spec),
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(level = "trace",
    skip_all,
    fields(
        turn_id = %turn_context.sub_id,
        model = %turn_context.model_info.slug,
        cwd = %turn_context.cwd.display()
    )
)]
async fn run_sampling_request(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    turn_diff_tracker: SharedTurnDiffTracker,
    client_session: &mut ModelClientSession,
    turn_metadata_header: Option<&str>,
    input: Vec<ResponseItem>,
    explicitly_enabled_connectors: &HashSet<String>,
    skills_outcome: Option<&SkillLoadOutcome>,
    cancellation_token: CancellationToken,
) -> CodexResult<SamplingRequestResult> {
    let router = built_tools(
        sess.as_ref(),
        turn_context.as_ref(),
        &input,
        explicitly_enabled_connectors,
        skills_outcome,
        &cancellation_token,
    )
    .await?;

    let base_instructions = sess.get_base_instructions().await;

    let tool_runtime = ToolCallRuntime::new(
        Arc::clone(&router),
        Arc::clone(&sess),
        Arc::clone(&turn_context),
        Arc::clone(&turn_diff_tracker),
    );
    let _code_mode_worker = sess
        .services
        .code_mode_service
        .start_turn_worker(
            &sess,
            &turn_context,
            Arc::clone(&router),
            Arc::clone(&turn_diff_tracker),
        )
        .await;
    let mut retries = 0;
    let mut initial_input = Some(input);
    loop {
        let provider_budget_snapshot = sess.action_map_provider_request_budget_snapshot().await;
        let prompt_source = if let Some(input) = initial_input.take() {
            input
        } else {
            sess.clone_history()
                .await
                .for_prompt(&turn_context.model_info.input_modalities)
        };
        let taskspace_context_visible = prompt_source.iter().any(is_taskspace_active_context_item);
        let transport_mode = taskspace_provider_transport_mode(
            turn_context.as_ref(),
            provider_budget_snapshot.as_ref(),
            taskspace_context_visible,
        );
        let mut prompt_input = match transport_mode {
            TaskspaceProviderTransportMode::NativeTools => {
                prepare_provider_visible_prompt_items(prompt_source)
            }
            TaskspaceProviderTransportMode::CacheOptimizedActionContract => {
                prepare_taskspace_action_contract_prompt_items_for_node(
                    prompt_source,
                    provider_budget_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.node_kind.as_deref()),
                )
            }
        };
        let budget_tool_visibility = provider_budget_snapshot
            .as_ref()
            .map(|snapshot| {
                let limits = taskspace_transport_budget_limits(snapshot);
                taskspace_provider_tool_visibility_for_budget(
                    snapshot.request_count,
                    limits.max_requests,
                    snapshot.node_request_count,
                    limits.max_model_requests_per_node,
                    snapshot.request_phase.as_deref(),
                    snapshot.node_kind.as_deref(),
                )
            })
            .unwrap_or(TaskspaceProviderToolVisibility::All);
        let tool_visibility = match transport_mode {
            TaskspaceProviderTransportMode::NativeTools => budget_tool_visibility,
            TaskspaceProviderTransportMode::CacheOptimizedActionContract => {
                TaskspaceProviderToolVisibility::None
            }
        };
        if let Some(snapshot) = provider_budget_snapshot.as_ref()
            && let Some(item) = build_taskspace_provider_budget_guidance_item(
                snapshot.request_count,
                taskspace_transport_budget_limits(snapshot).max_requests,
                snapshot.node_request_count,
                taskspace_transport_budget_limits(snapshot).max_model_requests_per_node,
                snapshot.request_phase.as_deref(),
                snapshot.node_kind.as_deref(),
                snapshot.node_id.as_deref(),
                budget_tool_visibility,
            )
        {
            prompt_input.push(item);
        }
        if let Some(snapshot) = provider_budget_snapshot.as_ref()
            && transport_mode == TaskspaceProviderTransportMode::CacheOptimizedActionContract
        {
            prompt_input.push(taskspace_action_contract_state_item(snapshot));
            if snapshot.node_id.is_none()
                && sess.action_map_has_tool_runtime_bootstrap_failure().await
            {
                prompt_input.push(taskspace_action_contract_tool_runtime_bootstrap_failure_item());
            } else if snapshot.node_id.is_none()
                && sess.action_map_has_blocked_validation_result().await
                && !sess.action_map_has_ready_recovery_node().await
            {
                prompt_input.push(taskspace_action_contract_closed_validation_item());
            }
            if snapshot.node_kind.as_deref() == Some("inspect_code_context") {
                let unread_scripts = sess
                    .action_map_current_inspect_unread_referenced_scripts()
                    .await;
                if !unread_scripts.is_empty() {
                    prompt_input.push(taskspace_action_contract_inspect_unread_scripts_item(
                        &unread_scripts,
                    ));
                }
            }
        } else if provider_budget_snapshot.is_none()
            && transport_mode == TaskspaceProviderTransportMode::CacheOptimizedActionContract
        {
            prompt_input.push(taskspace_action_contract_bootstrap_state_item());
        }
        let mut prompt_base_instructions = base_instructions.clone();
        if transport_mode == TaskspaceProviderTransportMode::CacheOptimizedActionContract {
            prompt_base_instructions.text.push_str("\n\n");
            prompt_base_instructions
                .text
                .push_str(&taskspace_deepseek_cache_anchor());
            prompt_base_instructions.text.push_str("\n\n");
            prompt_base_instructions
                .text
                .push_str(taskspace_static_action_contract_instructions());
        }
        let prompt = build_prompt_with_tool_visibility(
            prompt_input,
            router.as_ref(),
            turn_context.as_ref(),
            prompt_base_instructions,
            tool_visibility,
        );
        let err = match try_run_sampling_request(
            tool_runtime.clone(),
            Arc::clone(&sess),
            Arc::clone(&turn_context),
            client_session,
            turn_metadata_header,
            Arc::clone(&turn_diff_tracker),
            &prompt,
            cancellation_token.child_token(),
        )
        .await
        {
            Ok(output) => {
                return Ok(output);
            }
            Err(CodexErr::ContextWindowExceeded) => {
                sess.set_total_tokens_full(&turn_context).await;
                return Err(CodexErr::ContextWindowExceeded);
            }
            Err(CodexErr::UsageLimitReached(e)) => {
                let rate_limits = e.rate_limits.clone();
                if let Some(rate_limits) = rate_limits {
                    sess.update_rate_limits(&turn_context, *rate_limits).await;
                }
                return Err(CodexErr::UsageLimitReached(e));
            }
            Err(err) => err,
        };

        if !err.is_retryable() {
            return Err(err);
        }

        // Use the configured provider-specific stream retry budget.
        let max_retries = turn_context.provider.info().stream_max_retries();
        if retries >= max_retries
            && client_session.try_switch_fallback_transport(
                &turn_context.session_telemetry,
                &turn_context.model_info,
            )
        {
            sess.send_event(
                &turn_context,
                EventMsg::Warning(WarningEvent {
                    message: format!("Falling back from WebSockets to HTTPS transport. {err:#}"),
                }),
            )
            .await;
            retries = 0;
            continue;
        }
        if retries < max_retries {
            retries += 1;
            let delay = match &err {
                CodexErr::Stream(_, requested_delay) => {
                    requested_delay.unwrap_or_else(|| backoff(retries))
                }
                _ => backoff(retries),
            };
            warn!(
                "stream disconnected - retrying sampling request ({retries}/{max_retries} in {delay:?})...",
            );

            // In release builds, hide the first websocket retry notification to reduce noisy
            // transient reconnect messages. In debug builds, keep full visibility for diagnosis.
            let report_error = retries > 1
                || cfg!(debug_assertions)
                || !sess.services.model_client.responses_websocket_enabled();
            if report_error {
                // Surface retry information to any UI/front‑end so the
                // user understands what is happening instead of staring
                // at a seemingly frozen screen.
                sess.notify_stream_error(
                    &turn_context,
                    format!("Reconnecting... {retries}/{max_retries}"),
                    err,
                )
                .await;
            }
            tokio::time::sleep(delay).await;
        } else {
            return Err(err);
        }
    }
}

#[expect(
    clippy::await_holding_invalid_type,
    reason = "tool router construction reads through the session-owned manager guard"
)]
pub(crate) async fn built_tools(
    sess: &Session,
    turn_context: &TurnContext,
    input: &[ResponseItem],
    explicitly_enabled_connectors: &HashSet<String>,
    skills_outcome: Option<&SkillLoadOutcome>,
    cancellation_token: &CancellationToken,
) -> CodexResult<Arc<ToolRouter>> {
    let mcp_connection_manager = sess.services.mcp_connection_manager.read().await;
    let has_mcp_servers = mcp_connection_manager.has_servers();
    let all_mcp_tools = mcp_connection_manager
        .list_all_tools_non_blocking()
        .or_cancel(cancellation_token)
        .await?;
    drop(mcp_connection_manager);
    let loaded_plugins = sess
        .services
        .plugins_manager
        .plugins_for_config(&turn_context.config)
        .await;

    let mut effective_explicitly_enabled_connectors = explicitly_enabled_connectors.clone();
    effective_explicitly_enabled_connectors.extend(sess.get_connector_selection().await);

    let apps_enabled = turn_context.apps_enabled();
    let accessible_connectors =
        apps_enabled.then(|| connectors::accessible_connectors_from_mcp_tools(&all_mcp_tools));
    let accessible_connectors_with_enabled_state =
        accessible_connectors.as_ref().map(|connectors| {
            connectors::with_app_enabled_state(connectors.clone(), &turn_context.config)
        });
    let connectors = if apps_enabled {
        let connectors = codex_connectors::merge::merge_plugin_connectors_with_accessible(
            loaded_plugins
                .effective_apps()
                .into_iter()
                .map(|connector_id| connector_id.0),
            accessible_connectors.clone().unwrap_or_default(),
        );
        Some(connectors::with_app_enabled_state(
            connectors,
            &turn_context.config,
        ))
    } else {
        None
    };
    let auth = sess.services.auth_manager.auth().await;
    let discoverable_tools = if apps_enabled && turn_context.tools_config.tool_suggest {
        if let Some(accessible_connectors) = accessible_connectors_with_enabled_state.as_ref() {
            match connectors::list_tool_suggest_discoverable_tools_with_auth(
                &turn_context.config,
                auth.as_ref(),
                accessible_connectors.as_slice(),
            )
            .await
            .map(|discoverable_tools| {
                filter_tool_suggest_discoverable_tools_for_client(
                    discoverable_tools,
                    turn_context.app_server_client_name.as_deref(),
                )
            }) {
                Ok(discoverable_tools) if discoverable_tools.is_empty() => None,
                Ok(discoverable_tools) => Some(discoverable_tools),
                Err(err) => {
                    warn!("failed to load discoverable tool suggestions: {err:#}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let explicitly_enabled = if let Some(connectors) = connectors.as_ref() {
        let skill_name_counts_lower = skills_outcome.map_or_else(HashMap::new, |outcome| {
            build_skill_name_counts(&outcome.skills, &outcome.disabled_paths).1
        });

        filter_connectors_for_input(
            connectors,
            input,
            &effective_explicitly_enabled_connectors,
            &skill_name_counts_lower,
        )
    } else {
        Vec::new()
    };
    let mcp_tool_exposure = build_mcp_tool_exposure(
        &all_mcp_tools,
        connectors.as_deref(),
        explicitly_enabled.as_slice(),
        &turn_context.config,
        &turn_context.tools_config,
    );
    let mcp_tools = has_mcp_servers.then_some(mcp_tool_exposure.direct_tools);
    let deferred_mcp_tools = mcp_tool_exposure.deferred_tools;
    let unavailable_called_tools = if turn_context
        .config
        .features
        .enabled(Feature::UnavailableDummyTools)
    {
        let exposed_tool_names = mcp_tools
            .iter()
            .chain(deferred_mcp_tools.iter())
            .flat_map(|tools| tools.keys().map(String::as_str))
            .collect::<HashSet<_>>();
        collect_unavailable_called_tools(input, &exposed_tool_names)
    } else {
        Vec::new()
    };

    let parallel_mcp_server_names = turn_context
        .config
        .mcp_servers
        .get()
        .iter()
        .filter_map(|(server_name, server_config)| {
            server_config
                .supports_parallel_tool_calls
                .then_some(server_name.clone())
        })
        .collect::<HashSet<_>>();

    Ok(Arc::new(ToolRouter::from_config(
        &turn_context.tools_config,
        ToolRouterParams {
            mcp_tools,
            deferred_mcp_tools,
            unavailable_called_tools,
            parallel_mcp_server_names,
            discoverable_tools,
            dynamic_tools: turn_context.dynamic_tools.as_slice(),
        },
    )))
}

fn prepare_provider_visible_prompt_items(items: Vec<ResponseItem>) -> Vec<ResponseItem> {
    let composition = compose_provider_visible_history(items);
    let omitted_count = composition
        .decisions
        .iter()
        .filter(|decision| matches!(decision.action, ProviderVisibleHistoryAction::Omit(_)))
        .count();
    if omitted_count > 0 {
        trace!(
            target = "codex_core::taskspace",
            omitted_count,
            included_count = composition.items.len(),
            "taskspace_active_provider_visible_history_composed"
        );
    }
    composition.items
}

#[cfg(test)]
fn prepare_taskspace_action_contract_prompt_items(items: Vec<ResponseItem>) -> Vec<ResponseItem> {
    prepare_taskspace_action_contract_prompt_items_for_node(items, None)
}

fn prepare_taskspace_action_contract_prompt_items_for_node(
    items: Vec<ResponseItem>,
    current_node_kind: Option<&str>,
) -> Vec<ResponseItem> {
    let mut latest_user_input: Option<(usize, ResponseItem)> = None;
    let mut latest_taskspace_context: Option<(usize, ResponseItem)> = None;
    let mut tool_outputs: Vec<(usize, ResponseItem)> = Vec::new();

    for (index, item) in items.into_iter().enumerate() {
        if is_taskspace_active_context_item(&item) {
            latest_taskspace_context = Some((index, compile_taskspace_context_item(item)));
        } else if is_protected_user_input(&item) {
            latest_user_input = Some((index, item));
        } else if is_tool_output_item(&item) {
            tool_outputs.push((index, item));
        }
    }

    let latest_user_index = latest_user_input
        .as_ref()
        .map(|(index, _)| *index)
        .unwrap_or(0);
    let mut prepared = Vec::with_capacity(3);
    if let Some((_, item)) = latest_user_input {
        prepared.push(item);
    }
    let latest_context_index = latest_taskspace_context
        .as_ref()
        .map(|(index, _)| *index)
        .unwrap_or(latest_user_index);
    let validation_rework_target_reads = latest_taskspace_context
        .as_ref()
        .map(|(_, item)| taskspace_validation_rework_target_read_artifacts_from_item(item))
        .unwrap_or_default();
    if let Some((_, item)) = latest_taskspace_context {
        prepared.push(item);
    }
    let recent_tool_output_floor = latest_user_index.max(latest_context_index);
    let recent_tool_outputs = tool_outputs
        .into_iter()
        .filter_map(|(index, item)| {
            let is_recent_candidate =
                is_taskspace_action_contract_latest_tool_output_candidate(&item);
            let is_current_rework_target_read = index > latest_user_index
                && !validation_rework_target_reads.is_empty()
                && is_taskspace_validation_rework_target_read_output(
                    &item,
                    &validation_rework_target_reads,
                );
            if (index > recent_tool_output_floor && is_recent_candidate)
                || is_current_rework_target_read
            {
                Some(item)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if let Some(item) =
        taskspace_action_contract_recent_tool_outputs_item(&recent_tool_outputs, current_node_kind)
    {
        prepared.push(item);
    }
    prepared
}

fn taskspace_validation_rework_target_read_artifacts_from_item(
    item: &ResponseItem,
) -> HashSet<String> {
    response_item_text(item)
        .map(|text| taskspace_validation_rework_target_read_artifacts_from_text(&text))
        .unwrap_or_default()
}

fn taskspace_validation_rework_target_read_artifacts_from_text(text: &str) -> HashSet<String> {
    text.lines()
        .filter(|line| line.contains("validation_rework_target_read"))
        .flat_map(|line| line.split_whitespace())
        .filter_map(|part| part.strip_prefix("artifact="))
        .map(|artifact| {
            artifact
                .trim_matches(|ch| ch == ',' || ch == ';')
                .to_string()
        })
        .filter(|artifact| !artifact.is_empty())
        .collect()
}

fn is_taskspace_validation_rework_target_read_output(
    item: &ResponseItem,
    artifacts: &HashSet<String>,
) -> bool {
    response_item_texts_contain(item, &|text| {
        let normalized_text = text.replace('\\', "/");
        artifacts.iter().any(|artifact| {
            let normalized_artifact = artifact.replace('\\', "/");
            normalized_text.contains(&format!(
                "TaskSpaceReadFileSummaryV1: path={normalized_artifact}"
            ))
        })
    })
}

fn is_taskspace_active_context_item(item: &ResponseItem) -> bool {
    is_active_context_projection_item(item)
        || response_item_text_contains(item, TASKSPACE_ACTIVE_PROFILE_MARKER)
        || response_item_text_contains(item, "TaskSpace mode is now active.")
}

fn prompt_contains_taskspace_active_context(prompt: &Prompt) -> bool {
    prompt.input.iter().any(is_taskspace_active_context_item)
}

fn is_tool_output_item(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. }
    )
}

fn is_taskspace_action_contract_latest_tool_output_candidate(item: &ResponseItem) -> bool {
    is_tool_output_item(item)
        && (!is_legacy_taskspace_tool_output(item)
            || is_actionable_taskspace_gate_feedback_output(item))
}

fn taskspace_text_mentions_missing_source_visibility_blocker_rejection(text: &str) -> bool {
    text.contains("cannot be blocked for missing source visibility")
        && (text.contains("already recorded implementation source evidence")
            || text.contains(
                "dependency evidence already identifies the implementation artifact or validation rework target",
            ))
}

fn is_actionable_taskspace_gate_feedback_output(item: &ResponseItem) -> bool {
    (response_item_text_contains(item, "high-signal inspected evidence")
        && response_item_text_contains(item, "uncovered"))
        || response_item_text_contains(item, "validation_test_missing_local_validator_coverage")
        || response_item_text_contains(item, "validation_test_missing_changed_artifact_coverage")
        || response_item_text_contains(item, "validation_test_missing_output_contract_coverage")
        || response_item_texts_contain(item, &|text| {
            taskspace_text_mentions_current_validation_test_required(text)
        })
        || (response_item_texts_contain(item, &|text| {
            taskspace_missing_command_script_from_text(text).is_some()
        }))
        || response_item_text_contains(item, "required_validator:python scripts/validate.py")
        || (response_item_text_contains(item, "still unreviewed")
            && response_item_text_contains(item, "result_validities"))
        || (response_item_text_contains(item, "missing diagnostic prerequisite")
            && response_item_text_contains(item, "already recorded successful diagnostic evidence"))
        || (response_item_text_contains(
            item,
            "cannot be completed without a recorded successful edit action",
        ) && response_item_text_contains(item, "Execute the edit in this node"))
        || (response_item_text_contains(item, "cannot be blocked for an internal node-policy")
            && response_item_text_contains(
                item,
                "inspected implementation evidence is already available",
            ))
        || response_item_texts_contain(item, &|text| {
            taskspace_text_mentions_missing_source_visibility_blocker_rejection(text)
        })
        || (response_item_text_contains(item, "cannot be blocked for validator procedure")
            && response_item_text_contains(item, "implementation failure"))
        || (response_item_text_contains(item, "cannot be blocked for editable validation failure")
            && response_item_text_contains(item, "failed validation evidence"))
        || response_item_texts_contain(item, &|text| {
            taskspace_output_mentions_local_validator_infra_state_commit(text)
        })
}

fn taskspace_action_contract_recent_tool_outputs_item(
    items: &[ResponseItem],
    current_node_kind: Option<&str>,
) -> Option<ResponseItem> {
    let summaries = items
        .iter()
        .rev()
        .filter_map(taskspace_action_contract_tool_output_summary)
        .take(TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_ITEMS)
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        return None;
    }
    let edit_success_seen = summaries
        .iter()
        .any(|(_, text)| text.contains("Success. Updated the following files"));
    let uncovered_high_signal_seen = summaries.iter().any(|(_, text)| {
        text.contains("high-signal inspected evidence") && text.contains("uncovered")
    });
    let local_validator_infra_failure_seen = summaries
        .iter()
        .any(|(_, text)| taskspace_output_mentions_local_validator_infra_failure(text));
    let recoverable_local_validator_command_failure_seen = summaries.iter().any(|(_, text)| {
        taskspace_output_mentions_recoverable_local_validator_command_failure(text)
    });
    let unrecoverable_local_validator_infra_failure_seen = summaries.iter().any(|(_, text)| {
        taskspace_output_mentions_unrecoverable_local_validator_infra_failure(text)
    });
    let local_validator_infra_state_commit_seen = summaries
        .iter()
        .any(|(_, text)| taskspace_output_mentions_local_validator_infra_state_commit(text));
    let local_validator_coverage_failure_seen = summaries.iter().any(|(_, text)| {
        text.contains("validation_test_missing_local_validator_coverage")
            || text.contains("required_validator:python scripts/validate.py")
    });
    let changed_artifact_coverage_failure_seen = summaries
        .iter()
        .any(|(_, text)| text.contains("validation_test_missing_changed_artifact_coverage"));
    let output_contract_coverage_failure_seen = summaries
        .iter()
        .any(|(_, text)| text.contains("validation_test_missing_output_contract_coverage"));
    let validation_command_missing_script_seen = summaries
        .iter()
        .any(|(_, text)| text.contains("validation_command_missing_script"));
    let validation_current_test_required_seen = summaries.iter().any(|(_, text)| {
        text.contains("validation_stale_failure_without_current_test")
            || text.contains("validation_finish_missing_current_test_result")
            || taskspace_text_mentions_current_validation_test_required(text)
    });
    let unreviewed_result_blocker_seen = summaries.iter().any(|(_, text)| {
        text.contains("taskspace_unreviewed_result_blocker")
            || (text.contains("still unreviewed") && text.contains("result_validities"))
    });
    let diagnostic_prerequisite_already_satisfied_seen = summaries.iter().any(|(_, text)| {
        text.contains("diagnostic_prerequisite_already_satisfied")
            || (text.contains("missing diagnostic prerequisite")
                && text.contains("already recorded successful diagnostic evidence"))
    });
    let implement_missing_edit_before_finish_seen = summaries.iter().any(|(_, text)| {
        text.contains("implement_missing_edit_before_finish")
            || (text.contains("cannot be completed without a recorded successful edit action")
                && text.contains("Execute the edit in this node"))
    });
    let validation_rework_duplicate_read_seen = summaries
        .iter()
        .any(|(_, text)| taskspace_text_mentions_validation_rework_duplicate_artifact_read(text));
    let validation_rework_closed_action_space_read_seen = summaries
        .iter()
        .any(|(_, text)| text.contains("validation_rework_closed_action_space_read_disallowed"));
    let implementation_needs_edit_seen = summaries.iter().any(|(_, text)| {
        text.contains("failure_kind: implementation_needs_edit")
            || text.contains("implementation_needs_edit")
            || (text.contains("has enough read/search evidence")
                && text.contains("no successful edit"))
    });
    let internal_policy_blocker_rejected_seen = summaries.iter().any(|(_, text)| {
        text.contains("internal_policy_blocker_rejected")
            || (text.contains("cannot be blocked for an internal node-policy")
                && text.contains("inspected implementation evidence is already available"))
    });
    let missing_source_blocker_rejected_seen = summaries.iter().any(|(_, text)| {
        text.contains("missing_source_visibility_blocker_rejected")
            || taskspace_text_mentions_missing_source_visibility_blocker_rejection(text)
    });
    let validator_procedure_blocker_rejected_seen = summaries.iter().any(|(_, text)| {
        text.contains("validator_procedure_blocker_rejected")
            || (text.contains("cannot be blocked for validator procedure")
                && text.contains("implementation failure"))
    });
    let editable_validation_failure_blocker_rejected_seen = summaries.iter().any(|(_, text)| {
        text.contains("editable_validation_failure_blocker_rejected")
            || (text.contains("cannot be blocked for editable validation failure")
                && text.contains("failed validation evidence"))
    });

    let mut remaining_chars = TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_CHARS;
    let mut sections = Vec::new();
    for (call_id, text) in summaries.into_iter().rev() {
        if remaining_chars == 0 {
            break;
        }
        let char_count = text.chars().count();
        let mut output = text.chars().take(remaining_chars).collect::<String>();
        if char_count > remaining_chars {
            output.push_str("\n[truncated]");
            output = append_taskspace_tool_tail_sentinels(output, &text);
            remaining_chars = 0;
        } else {
            remaining_chars = remaining_chars.saturating_sub(char_count);
        }
        sections.push(format!("call_id: {call_id}\noutput:\n{output}"));
    }
    if sections.is_empty() {
        return None;
    }

    let in_implement_rework = current_node_kind == Some("implement_solution");
    let progress_hint = if editable_validation_failure_blocker_rejected_seen {
        "progress_hint: A previous block_node action was rejected because validation evidence identifies an editable implementation failure such as IndentationError, SyntaxError, or KeyError. Do not close validation as infrastructure-blocked and do not block for needing more inspection. Next action must be apply_patch for the implementation artifact named by the failed validation evidence; for top-level Python indentation or syntax failures, patch the whole affected file or block in one edit.\n"
    } else if validator_procedure_blocker_rejected_seen {
        "progress_hint: A previous block_node action was rejected because it blamed validator procedure or test-command setup while dependency validation evidence already identifies an implementation failure. Do not create tests, adjust validator commands, or block for pytest/cache procedure concerns. Next action must be apply_patch for the implementation artifact named by the failed validation evidence.\n"
    } else if missing_source_blocker_rejected_seen {
        "progress_hint: A previous block_node action was rejected because implementation source evidence is already available. Do not create another inspect node and do not rerun diagnostics. Next action must be apply_patch; if a failed edit made the target context stale or truncated, one read_file of the same validation rework target is allowed to refresh context before patching.\n"
    } else if internal_policy_blocker_rejected_seen {
        "progress_hint: A previous block_node action was rejected because it described TaskSpace internal policy or a repeated diagnostic, not an external blocker. Do not create another inspect node and do not rerun the diagnostic. Next action must be apply_patch with the smallest concrete implementation fix from dependency evidence.\n"
    } else if implement_missing_edit_before_finish_seen {
        "progress_hint: A previous finish_node action was rejected because the implement_solution node has no successful edit. Do not finish, block, create another inspect node, or rerun diagnostics. Next action must be apply_patch with the smallest concrete implementation fix from dependency evidence.\n"
    } else if validation_rework_duplicate_read_seen
        || validation_rework_closed_action_space_read_seen
    {
        "progress_hint: A previous read/search was blocked because this validation rework node already has the target artifact contents visible and no successful edit has happened yet. Do not read_file, list_files, search, inspect schema again, or run validation from this implementation node. Next action must be apply_patch for the already-read validation rework artifact, or blocked only with a concrete external reason editing is impossible.\n"
    } else if implementation_needs_edit_seen {
        "progress_hint: A previous read/search/control action was blocked because this implement_solution node already has enough evidence and no successful edit has happened yet. Do not read_file, list_files, search, broad shell discovery, or validation from this implementation node. Next action must be apply_patch with the smallest concrete fix from the existing evidence, or blocked only with a concrete external reason editing is impossible.\n"
    } else if diagnostic_prerequisite_already_satisfied_seen {
        "progress_hint: A previous block_node action was rejected because the dependency inspect node already recorded successful diagnostic evidence. Do not rerun diagnostic commands and do not block for the same prerequisite. Next action must be apply_patch with the smallest concrete implementation fix from inspected evidence.\n"
    } else if unreviewed_result_blocker_seen {
        "progress_hint: A previous ordinary tool was blocked because a TaskSpace node result is still unreviewed. Do not repeat read/list/search. Next action must be taskspace_control with action=state_commit and result_validities for the named result.\n"
    } else if validation_current_test_required_seen {
        "progress_hint: A previous block_node or finish_node action tried to close the current validation node using failure text before this node had its own test/build result. Stay on the validation node. Next action must be run_test with the required validation command from the current TaskSpace projection; do not finish_node, block_node, create implementation rework, read_file, list_files, or search before the current validation result is recorded.\n"
    } else if validation_command_missing_script_seen {
        "progress_hint: The previous run_test did not start the validator because the command referenced a missing script. Stay on the validation node. Next action must be run_test with an existing changed script from the current TaskSpace projection, then validate the expected output artifact.\n"
    } else if changed_artifact_coverage_failure_seen {
        "progress_hint: A previous run_test was rejected because it did not exercise the changed artifact required for validation. Do not substitute generic validators or alternate filenames. Next action must be the concrete run_test command named in the validation_test_missing_changed_artifact_coverage feedback, or blocked with the exact reason that command cannot run.\n"
    } else if output_contract_coverage_failure_seen {
        "progress_hint: A previous run_test was rejected because it executed code but did not validate declared output contract artifacts such as expected files, formats, schemas, or validators. Next action must be the concrete combined run_test command named in the validation_test_missing_output_contract_coverage feedback, or a real project/official validator that checks those output contracts.\n"
    } else if local_validator_coverage_failure_seen {
        "progress_hint: A previous run_test was rejected because it skipped a discovered local validator. Do not repeat pytest or discovery. Next action must be run_test with command `python scripts/validate.py`, or blocked with the exact reason that command cannot run.\n"
    } else if uncovered_high_signal_seen {
        "progress_hint: A previous finish_node was rejected because high-signal inspected evidence is still uncovered. Do not repeat finish_node. Next action must be apply_patch for the named uncovered artifact, or blocked with the exact reason it cannot be changed.\n"
    } else if in_implement_rework
        && unrecoverable_local_validator_infra_failure_seen
        && !recoverable_local_validator_command_failure_seen
    {
        "progress_hint: Local validation failed because the host validator service or shell executor was unavailable, not because implementation code produced a test failure. Do not patch code or run more shell-discovery commands for the same E_ACCESSDENIED-style infrastructure failure. Next action must be blocked with the exact local validator infrastructure evidence.\n"
    } else if in_implement_rework
        && local_validator_infra_failure_seen
        && local_validator_infra_state_commit_seen
    {
        "progress_hint: Local validation infrastructure failed earlier and that failure is already recorded. The current node is implementation rework, not the closed validation node. Do not repeat state_commit or block for the same local validator infrastructure result. Next action must either patch the implementation or run the changed artifact with platform-compatible syntax, for example use PowerShell `;` or separate commands instead of bash `&&`.\n"
    } else if in_implement_rework && local_validator_infra_failure_seen {
        "progress_hint: The previous validation command hit local validator infrastructure or host-shell syntax, but the current node is implementation rework. Do not close this node as infrastructure-blocked. Next action must either patch the implementation or run the changed artifact with platform-compatible syntax, for example use PowerShell `;` or separate commands instead of bash `&&`.\n"
    } else if local_validator_infra_failure_seen && local_validator_infra_state_commit_seen {
        "progress_hint: Local validation already failed because the host validator infrastructure or shell is unavailable, and that failure has been recorded. Do not run more bash/PowerShell diagnostic commands for the same local validator failure. Next action must be blocked with the exact local validator infrastructure evidence, or taskspace_control with action=finish_node only if the node is explicitly being closed as infrastructure-blocked.\n"
    } else if local_validator_infra_failure_seen {
        "progress_hint: The last run_test failed because local validator infrastructure or the host shell failed, not because new code evidence was found. Do not run more bash/PowerShell diagnostic commands for the same failure. Next action must be taskspace_control with action=state_commit marking the failed run_test result invalid because local validator infrastructure failed, or blocked with the exact infrastructure error.\n"
    } else if edit_success_seen {
        "progress_hint: A file edit already succeeded. Do not repeat apply_patch, read_file, or search. Next action must be taskspace_control with action=finish_node for the current implementation node, then run validation in the next node.\n"
    } else {
        ""
    };
    let text = format!(
        "TaskSpaceActionContractRecentToolOutputsV1:\n{progress_hint}{}",
        sections.join("\n---\n")
    );
    Some(ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    })
}

fn taskspace_action_contract_tool_output_summary(item: &ResponseItem) -> Option<(String, String)> {
    let (call_id, text, success) = match item {
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        }
        | ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => (
            call_id.as_str(),
            function_call_output_body_text(&output.body),
            output.success,
        ),
        _ => return None,
    };
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    Some((
        call_id.to_string(),
        taskspace_action_contract_tool_feedback_summary(call_id, text, success),
    ))
}

fn taskspace_action_contract_tool_feedback_summary(
    call_id: &str,
    text: &str,
    success: Option<bool>,
) -> String {
    if !call_id.starts_with("taskspace-action-contract-") || success != Some(false) {
        return text.to_string();
    }
    let action = taskspace_action_contract_action_name_from_call_id(call_id).unwrap_or("unknown");
    if taskspace_text_mentions_stale_validation_failure_without_current_test(text) {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: validation_stale_failure_without_current_test\n\
next_valid_action: emit exactly one run_test action on the current validation node using the required validation command from the current TaskSpace projection. Do not finish_node, block_node, create implementation rework, read_file, list_files, or search until this validation node records its own test/build result.\n\
raw_output:\n{text}"
        );
    }
    if taskspace_text_mentions_validation_finish_missing_current_test(text) {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: validation_finish_missing_current_test_result\n\
next_valid_action: emit exactly one run_test action on the current validation node using the required validation command from the current TaskSpace projection. Do not finish_node, block_node, create implementation rework, read_file, list_files, or search before a current test/build result exists.\n\
raw_output:\n{text}"
        );
    }
    if taskspace_text_mentions_validation_rework_duplicate_artifact_read(text) {
        let artifact = taskspace_validation_rework_duplicate_artifact(text)
            .unwrap_or_else(|| "already-read validation rework artifact".to_string());
        let previous_result = taskspace_validation_rework_duplicate_previous_result(text)
            .unwrap_or_else(|| "previous read result".to_string());
        let repair_contract = taskspace_validation_rework_repair_contract(text)
            .map(|contract| format!("repair_contract: {contract}\n"))
            .unwrap_or_default();
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: validation_rework_duplicate_artifact_read\n\
target_artifact: {artifact}\n\
previous_read_result: {previous_result}\n\
{repair_contract}\
next_valid_action: emit exactly one apply_patch action targeting `{artifact}` using the current contents already visible in `{previous_result}` and the failed validation evidence. If repair_contract is present, satisfy it exactly. Do not read_file, list_files, search, inspect schema again, or run validation from this implementation node before a successful edit is recorded.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("implementation_needs_edit")
        || (text.contains("has enough read/search evidence") && text.contains("no successful edit"))
    {
        let repair_contract = taskspace_validation_rework_repair_contract(text)
            .map(|contract| format!("repair_contract: {contract}\n"))
            .unwrap_or_default();
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: implementation_needs_edit\n\
{repair_contract}\
next_valid_action: emit exactly one apply_patch action with the smallest concrete implementation fix from already inspected evidence or failed validation output. If repair_contract is present, satisfy it exactly. Do not read_file, list_files, search, broad shell discovery, or validation from this implementation node before a successful edit is recorded.\n\
raw_output:\n{text}"
        );
    }
    if let Some(targets) = taskspace_native_hunk_targets_from_rejection(Some(text)) {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: apply_patch_native_hunk_header\n\
target: {targets}\n\
next_valid_action: emit exactly one corrected apply_patch using native apply_patch grammar. Do not mix `*** Update File` with `--- a/...` / `+++ b/...`, and do not use `@@ -old,+new @@` range hunks. Use exact context with `@@`, or replace the small/generated file with `*** Delete File: <path>` followed by `*** Add File: <path>` and complete contents.\n\
raw_output:\n{text}"
        );
    }
    if action == "apply_patch"
        && let Some(target) = taskspace_missing_update_targets_from_apply_patch_text(text)
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: apply_patch\n\
tool_result: failed\n\
failure_kind: apply_patch_missing_update_target\n\
target: {target}\n\
next_valid_action: emit exactly one apply_patch. Use `*** Add File: <relative/path>` if the file should be created, or correct the path and use `*** Update File: <relative/path>` if the file already exists.\n\
raw_output:\n{text}"
        );
    }
    if action == "apply_patch"
        && let Some(target) = taskspace_expected_lines_target_from_apply_patch_text(text)
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: apply_patch\n\
tool_result: failed\n\
failure_kind: apply_patch_expected_lines_mismatch\n\
target: {target}\n\
next_valid_action: emit exactly one corrected apply_patch, or one read_file of `{target}` only if the current target context is truncated/stale after this failed edit. Do not repeat the same context hunk. If the intended full contents are known or the file is small/generated, use `*** Delete File: {target}` followed by `*** Add File: {target}` with the complete corrected file contents; otherwise use an `*** Update File: {target}` hunk with exact existing context.\n\
raw_output:\n{text}"
        );
    }
    if action == "apply_patch"
        && let Some(target) = taskspace_context_mismatch_target_from_apply_patch_text(text)
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: apply_patch\n\
tool_result: failed\n\
failure_kind: apply_patch_context_mismatch\n\
target: {target}\n\
next_valid_action: emit exactly one corrected apply_patch, or one read_file of `{target}` only if the current target context is truncated/stale after this failed edit. Do not repeat the same context hunk. If the failed hunk used a unified-diff header such as `@@ -1,1 +1,1 @@`, remove the range header and use native apply_patch `@@` grammar, or replace the small/generated file with `*** Delete File: {target}` followed by `*** Add File: {target}` and complete corrected contents.\n\
raw_output:\n{text}"
        );
    }
    if action == "apply_patch" && taskspace_apply_patch_invalid_hunk_looks_unified(text) {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: apply_patch\n\
tool_result: failed\n\
failure_kind: apply_patch_unified_hunk_header_in_native_patch\n\
next_valid_action: emit exactly one corrected apply_patch using native apply_patch grammar. Do not include unified-diff range headers like `@@ -0,0 +1,44 @@`; use `@@` for native hunks, `*** Add File: <path>` for new files, and prefix every added file content line with `+`.\n\
raw_output:\n{text}"
        );
    }
    if action == "run_test" && text.contains("validation_test_missing_local_validator_coverage") {
        let required_validator = if text.contains("python scripts/validate.py") {
            "python scripts/validate.py"
        } else {
            "the discovered local validator"
        };
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: run_test\n\
tool_result: blocked\n\
failure_kind: validation_test_missing_local_validator_coverage\n\
required_validator: {required_validator}\n\
next_valid_action: emit exactly one run_test action with command `{required_validator}`. Do not repeat pytest, read_file, list_files, search, or finish validation before this validator result is recorded.\n\
raw_output:\n{text}"
        );
    }
    if action == "run_test" && text.contains("validation_test_missing_changed_artifact_coverage") {
        let required_command = taskspace_validation_changed_artifact_required_command(text)
            .unwrap_or_else(|| {
                "the changed artifact command named in TaskSpaceGateRecoveryV1".to_string()
            });
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: run_test\n\
tool_result: blocked\n\
failure_kind: validation_test_missing_changed_artifact_coverage\n\
required_command: {required_command}\n\
next_valid_action: emit exactly one run_test action with command `{required_command}`. Do not repeat generic validators, read_file, list_files, search, or alternate filenames before this changed artifact is executed.\n\
raw_output:\n{text}"
        );
    }
    if action == "run_test" && text.contains("validation_test_missing_output_contract_coverage") {
        let required_command = taskspace_validation_changed_artifact_required_command(text)
            .unwrap_or_else(|| {
                "the output-contract validation command named in TaskSpaceGateRecoveryV1"
                    .to_string()
            });
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: run_test\n\
tool_result: blocked\n\
failure_kind: validation_test_missing_output_contract_coverage\n\
required_command: {required_command}\n\
next_valid_action: emit exactly one run_test action with command `{required_command}`. It must execute the changed artifact when needed and validate the declared output contract artifacts. Do not finish validation after a generator-only success.\n\
raw_output:\n{text}"
        );
    }
    if action == "run_test"
        && let Some(missing_script) = taskspace_missing_command_script_from_text(text)
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: run_test\n\
tool_result: failed\n\
failure_kind: validation_command_missing_script\n\
missing_script: {missing_script}\n\
next_valid_action: emit exactly one run_test action that executes an existing changed script named in the current TaskSpace projection, then validates the expected output artifact. Do not switch to implement_solution or finish the validation node for this command-name error.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("still unreviewed") && text.contains("result_validities") {
        let (result_id, node_id) = taskspace_unreviewed_result_refs(text);
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: taskspace_unreviewed_result_blocker\n\
blocked_result: {result_id}\n\
blocked_node: {node_id}\n\
next_valid_action: emit exactly one taskspace_control action with args.action `state_commit` and result_validities for `{result_id}` before any read_file, list_files, search, run_test, or apply_patch.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("missing diagnostic prerequisite")
        && text.contains("already recorded successful diagnostic evidence")
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: diagnostic_prerequisite_already_satisfied\n\
next_valid_action: emit exactly one apply_patch action with the smallest concrete implementation fix from inspected evidence. Do not rerun the diagnostic and do not block for the same prerequisite.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("cannot be completed without a recorded successful edit action")
        && text.contains("Execute the edit in this node")
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: implement_missing_edit_before_finish\n\
next_valid_action: emit exactly one apply_patch action with the smallest concrete implementation fix from dependency evidence. Do not finish_node, block_node, create another inspect node, or rerun diagnostics before a successful edit is recorded.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("cannot be blocked for an internal node-policy")
        && text.contains("inspected implementation evidence is already available")
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: internal_policy_blocker_rejected\n\
next_valid_action: emit exactly one apply_patch action with the smallest concrete implementation fix from dependency evidence. Do not create another inspect node, rerun diagnostics, or block for TaskSpace internal policy.\n\
raw_output:\n{text}"
        );
    }
    if taskspace_text_mentions_missing_source_visibility_blocker_rejection(text) {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: missing_source_visibility_blocker_rejected\n\
next_valid_action: emit exactly one apply_patch action. Use the failed patch feedback plus inspected source evidence to correct the target, function signature, or context lines. Do not create another inspect node, rerun diagnostics, or block for missing source visibility.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("cannot be blocked for validator procedure")
        && text.contains("implementation failure")
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: validator_procedure_blocker_rejected\n\
next_valid_action: emit exactly one apply_patch action for the implementation artifact named by the failed validation evidence. Do not create tests, adjust validator commands, or block for pytest/cache procedure concerns.\n\
raw_output:\n{text}"
        );
    }
    if text.contains("cannot be blocked for editable validation failure")
        && text.contains("failed validation evidence")
    {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: editable_validation_failure_blocker_rejected\n\
next_valid_action: emit exactly one apply_patch action for the implementation artifact named by the failed validation evidence. For top-level Python IndentationError or SyntaxError, patch the whole affected file or block in one edit. Do not close validation as infrastructure-blocked and do not block for needing more inspection.\n\
raw_output:\n{text}"
        );
    }
    format!(
        "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: failed\n\
failure_kind: tool_execution_failed\n\
next_valid_action: inspect this tool result and emit one corrected taskspace-action-v1 action. Do not ignore this failed tool result or finish the node until the failure is resolved or explicitly blocked with evidence.\n\
raw_output:\n{text}"
    )
}

fn taskspace_validation_changed_artifact_required_command(text: &str) -> Option<String> {
    let (_, rest) = text.split_once("run_test with command `")?;
    let (command, _) = rest.split_once('`')?;
    let command = command.trim();
    (!command.is_empty()).then(|| command.to_string())
}

fn taskspace_validation_required_command_from_gate_recovery(text: Option<&str>) -> Option<String> {
    let text = text?;
    if !text.contains(TASKSPACE_GATE_RECOVERY_MARKER) {
        return None;
    }
    if !text.contains("validation_test_missing_changed_artifact_coverage")
        && !text.contains("validation_test_missing_output_contract_coverage")
    {
        return None;
    }
    taskspace_validation_changed_artifact_required_command(text)
}

fn taskspace_validation_chained_required_command(
    previous_command: &str,
    output: &str,
) -> Option<String> {
    let next_command = taskspace_validation_required_command_from_gate_recovery(Some(output))?;
    (next_command != previous_command.trim()).then_some(next_command)
}

fn taskspace_missing_command_script_from_text(text: &str) -> Option<String> {
    let lower = text.replace('\\', "/").to_ascii_lowercase();
    if !lower.contains("can't open file") && !lower.contains("cannot open file") {
        return None;
    }
    let marker = if lower.contains("can't open file") {
        "can't open file"
    } else {
        "cannot open file"
    };
    let (_, rest) = lower.split_once(marker)?;
    let rest = rest.trim_start_matches(|ch: char| ch.is_whitespace() || ch == ':');
    let value = if let Some(rest) = rest.strip_prefix('\'') {
        rest.split_once('\'').map(|(path, _)| path.to_string())
    } else if let Some(rest) = rest.strip_prefix('"') {
        rest.split_once('"').map(|(path, _)| path.to_string())
    } else {
        rest.split(|ch: char| ch.is_whitespace() || ch == ':')
            .next()
            .map(str::to_string)
    }?;
    let file_name = value.rsplit('/').next().unwrap_or(value.as_str());
    matches!(
        file_name.rsplit_once('.').map(|(_, ext)| ext),
        Some("py" | "js" | "mjs" | "sh")
    )
    .then(|| file_name.to_string())
}

fn taskspace_action_contract_action_name_from_call_id(call_id: &str) -> Option<&str> {
    let suffix = call_id.strip_prefix("taskspace-action-contract-")?;
    let (_, action) = suffix.rsplit_once('-')?;
    Some(action)
}

fn taskspace_unreviewed_result_refs(text: &str) -> (String, String) {
    let result_id = text
        .split("TaskSpace result `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-result")
        .to_string();
    let node_id = text
        .split(" on node `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-node")
        .to_string();
    (result_id, node_id)
}

fn taskspace_output_mentions_local_validator_infra_failure(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let signal = taskspace_compact_ascii_signal(&text);
    text.contains("bash/service/createinstance/e_accessdenied")
        || text.contains("e_accessdenied")
        || text.contains("fullyqualifiederrorid : invalidendofline")
        || text.contains("fullyqualifiederrorid: invalidendofline")
        || text.contains("invalidendofline")
        || (text.contains("not a valid statement separator")
            && (text.contains("parsererror") || text.contains("invalidendofline")))
        || signal.contains("bashservicecreateinstanceeaccessdenied")
        || signal.contains("bashservicecreateinstancee_accessdenied")
        || signal.contains("eaccessdenied")
        || signal.contains("e_accessdenied")
        || signal.contains("invalidendofline")
}

fn taskspace_output_mentions_recoverable_local_validator_command_failure(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let signal = taskspace_compact_ascii_signal(&text);
    text.contains("invalidendofline")
        || text.contains("not a valid statement separator")
        || signal.contains("invalidendofline")
}

fn taskspace_output_mentions_unrecoverable_local_validator_infra_failure(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let signal = taskspace_compact_ascii_signal(&text);
    text.contains("bash/service/createinstance/e_accessdenied")
        || text.contains("wsl/service/e_accessdenied")
        || text.contains("e_accessdenied")
        || signal.contains("bashservicecreateinstanceeaccessdenied")
        || signal.contains("bashservicecreateinstancee_accessdenied")
        || signal.contains("wslserviceeaccessdenied")
        || signal.contains("wslservicee_accessdenied")
        || signal.contains("eaccessdenied")
        || signal.contains("e_accessdenied")
}

fn taskspace_output_mentions_local_validator_infra_state_commit(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("state_commit")
        && text.contains("accepted")
        && (text.contains("local validator")
            || text.contains("validator infrastructure")
            || text.contains("result_validities")
            || text.contains("blockers"))
}

fn taskspace_compact_ascii_signal(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Debug)]
struct ProviderVisibleHistoryComposition {
    items: Vec<ResponseItem>,
    decisions: Vec<ProviderVisibleHistoryDecision>,
}

#[derive(Debug, PartialEq, Eq)]
struct ProviderVisibleHistoryDecision {
    index: usize,
    category: ProviderVisibleItemCategory,
    action: ProviderVisibleHistoryAction,
}

#[derive(Debug, PartialEq, Eq)]
enum ProviderVisibleHistoryAction {
    Include,
    Omit(&'static str),
}

#[derive(Debug, PartialEq, Eq)]
enum ProviderVisibleItemCategory {
    ActiveProjection,
    ProtectedUserInput,
    ProtectedDeveloperOrSystemInput,
    ShadowProjection,
    LegacyTaskspaceInstruction,
    TaskspaceControlCall,
    LegacyTaskspaceToolOutput,
    LargeRawToolOutput,
    Other,
}

fn compose_provider_visible_history(items: Vec<ResponseItem>) -> ProviderVisibleHistoryComposition {
    if !items.iter().any(is_active_context_projection_item) {
        let decisions = items
            .iter()
            .enumerate()
            .map(|(index, item)| ProviderVisibleHistoryDecision {
                index,
                category: classify_provider_visible_item(item),
                action: ProviderVisibleHistoryAction::Include,
            })
            .collect();
        return ProviderVisibleHistoryComposition { items, decisions };
    }

    let classified_items = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let category = classify_provider_visible_item(&item);
            let action = provider_visible_history_action(&category);
            (index, item, category, action)
        })
        .collect::<Vec<_>>();
    let paired_omitted_tool_call_ids =
        omitted_provider_visible_tool_call_ids(classified_items.as_slice());

    let mut prepared = Vec::with_capacity(classified_items.len());
    let mut decisions = Vec::with_capacity(classified_items.len());
    for (index, item, category, base_action) in classified_items {
        let action =
            provider_visible_history_pair_action(&item, base_action, &paired_omitted_tool_call_ids);
        if matches!(action, ProviderVisibleHistoryAction::Include) {
            prepared.push(match category {
                ProviderVisibleItemCategory::ActiveProjection => {
                    compile_taskspace_context_item(item)
                }
                _ => item,
            });
        }
        decisions.push(ProviderVisibleHistoryDecision {
            index,
            category,
            action,
        });
    }

    ProviderVisibleHistoryComposition {
        items: prepared,
        decisions,
    }
}

fn provider_visible_history_action(
    category: &ProviderVisibleItemCategory,
) -> ProviderVisibleHistoryAction {
    match category {
        ProviderVisibleItemCategory::ShadowProjection => {
            ProviderVisibleHistoryAction::Omit("shadow_projection_replaced_by_active_projection")
        }
        ProviderVisibleItemCategory::LegacyTaskspaceInstruction => {
            ProviderVisibleHistoryAction::Omit("legacy_taskspace_instruction_replaced")
        }
        ProviderVisibleItemCategory::TaskspaceControlCall => {
            ProviderVisibleHistoryAction::Omit("taskspace_control_call_not_provider_surface")
        }
        ProviderVisibleItemCategory::LegacyTaskspaceToolOutput => {
            ProviderVisibleHistoryAction::Omit("legacy_taskspace_tool_output_replaced")
        }
        ProviderVisibleItemCategory::LargeRawToolOutput => {
            ProviderVisibleHistoryAction::Omit("large_raw_tool_output_requires_output_reference")
        }
        ProviderVisibleItemCategory::ActiveProjection
        | ProviderVisibleItemCategory::ProtectedUserInput
        | ProviderVisibleItemCategory::ProtectedDeveloperOrSystemInput
        | ProviderVisibleItemCategory::Other => ProviderVisibleHistoryAction::Include,
    }
}

fn omitted_provider_visible_tool_call_ids(
    classified_items: &[(
        usize,
        ResponseItem,
        ProviderVisibleItemCategory,
        ProviderVisibleHistoryAction,
    )],
) -> HashSet<String> {
    classified_items
        .iter()
        .filter_map(|(_, item, _, action)| {
            matches!(action, ProviderVisibleHistoryAction::Omit(_))
                .then(|| response_item_tool_call_id(item))
                .flatten()
                .map(str::to_string)
        })
        .collect()
}

fn provider_visible_history_pair_action(
    item: &ResponseItem,
    base_action: ProviderVisibleHistoryAction,
    paired_omitted_tool_call_ids: &HashSet<String>,
) -> ProviderVisibleHistoryAction {
    if matches!(base_action, ProviderVisibleHistoryAction::Include)
        && response_item_tool_call_id(item)
            .is_some_and(|call_id| paired_omitted_tool_call_ids.contains(call_id))
    {
        return ProviderVisibleHistoryAction::Omit(
            "paired_tool_call_or_output_replaced_by_active_projection",
        );
    }
    base_action
}

fn classify_provider_visible_item(item: &ResponseItem) -> ProviderVisibleItemCategory {
    if is_active_context_projection_item(item) {
        return ProviderVisibleItemCategory::ActiveProjection;
    }
    if response_item_text_contains(item, TASKSPACE_SHADOW_PROJECTION_MARKER) {
        return ProviderVisibleItemCategory::ShadowProjection;
    }
    if is_taskspace_control_call(item) {
        return ProviderVisibleItemCategory::TaskspaceControlCall;
    }
    if is_protected_user_input(item) {
        return ProviderVisibleItemCategory::ProtectedUserInput;
    }
    if is_legacy_taskspace_instruction(item) {
        return ProviderVisibleItemCategory::LegacyTaskspaceInstruction;
    }
    if is_protected_developer_or_system_input(item) {
        return ProviderVisibleItemCategory::ProtectedDeveloperOrSystemInput;
    }
    if is_legacy_taskspace_tool_output(item) {
        return ProviderVisibleItemCategory::LegacyTaskspaceToolOutput;
    }
    if is_large_raw_tool_output(item) {
        return ProviderVisibleItemCategory::LargeRawToolOutput;
    }
    ProviderVisibleItemCategory::Other
}

fn is_active_context_projection_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_ACTIVE_PROFILE_MARKER)
        && response_item_text_contains(item, TASKSPACE_ACTIVE_PROJECTION_MARKER)
}

fn compile_taskspace_context_item(item: ResponseItem) -> ResponseItem {
    match item {
        ResponseItem::Message {
            id,
            role,
            content,
            end_turn,
            phase,
        } => {
            let content = content
                .into_iter()
                .map(|content_item| match content_item {
                    ContentItem::InputText { text } => {
                        if let Some(compiled) = compile_taskspace_agent_context_text(&text) {
                            ContentItem::InputText { text: compiled }
                        } else {
                            ContentItem::InputText { text }
                        }
                    }
                    ContentItem::OutputText { text } => {
                        if let Some(compiled) = compile_taskspace_agent_context_text(&text) {
                            ContentItem::OutputText { text: compiled }
                        } else {
                            ContentItem::OutputText { text }
                        }
                    }
                    other => other,
                })
                .collect();
            ResponseItem::Message {
                id,
                role,
                content,
                end_turn,
                phase,
            }
        }
        other => other,
    }
}

fn is_protected_user_input(item: &ResponseItem) -> bool {
    matches!(item, ResponseItem::Message { role, .. } if role == "user")
}

fn is_protected_developer_or_system_input(item: &ResponseItem) -> bool {
    matches!(item, ResponseItem::Message { role, .. } if role == "developer" || role == "system")
}

fn is_legacy_taskspace_instruction(item: &ResponseItem) -> bool {
    response_item_text_contains(item, "TaskSpace mode is now active")
        || response_item_text_contains(item, "TaskSpace final answer gate rejected")
        || response_item_text_contains(item, "taskspace_control(")
}

fn is_taskspace_control_call(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::FunctionCall { name, .. } | ResponseItem::CustomToolCall { name, .. }
            if name == "taskspace_control"
    )
}

fn response_item_tool_call_id(item: &ResponseItem) -> Option<&str> {
    match item {
        ResponseItem::FunctionCall { call_id, .. }
        | ResponseItem::CustomToolCall { call_id, .. }
        | ResponseItem::FunctionCallOutput { call_id, .. }
        | ResponseItem::CustomToolCallOutput { call_id, .. } => Some(call_id.as_str()),
        _ => None,
    }
}

fn is_legacy_taskspace_tool_output(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. }
    ) && (response_item_text_contains(item, "TaskSpace")
        || response_item_text_contains(item, "ActionMap")
        || response_item_text_contains(item, "taskspace_control"))
}

fn is_large_raw_tool_output(item: &ResponseItem) -> bool {
    if !matches!(
        item,
        ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. }
    ) || response_item_text_contains(item, "output-ref://")
        || response_item_text_contains(item, "OutputReferenceV1")
    {
        return false;
    }

    response_item_text_len(item) > TASKSPACE_ACTIVE_MAX_RAW_TOOL_OUTPUT_CHARS
}

fn response_item_text_len(item: &ResponseItem) -> usize {
    match item {
        ResponseItem::Message { role, content, .. } => {
            role.len()
                + content
                    .iter()
                    .map(|content_item| match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                            text.len()
                        }
                        ContentItem::InputImage { image_url, .. } => image_url.len(),
                    })
                    .sum::<usize>()
        }
        ResponseItem::FunctionCall {
            name,
            namespace,
            arguments,
            ..
        } => name.len() + namespace.as_ref().map_or(0, |value| value.len()) + arguments.len(),
        ResponseItem::CustomToolCall { name, input, .. } => name.len() + input.len(),
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            function_call_output_body_text_len(&output.body)
        }
        ResponseItem::ToolSearchCall {
            execution,
            arguments,
            ..
        } => execution.len() + arguments.to_string().len(),
        ResponseItem::ToolSearchOutput {
            execution, tools, ..
        } => {
            execution.len()
                + tools
                    .iter()
                    .map(|tool| tool.to_string().len())
                    .sum::<usize>()
        }
        ResponseItem::LocalShellCall { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::GhostSnapshot { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::Other => 0,
    }
}

fn function_call_output_body_text_len(body: &FunctionCallOutputBody) -> usize {
    match body {
        FunctionCallOutputBody::Text(text) => text.len(),
        FunctionCallOutputBody::ContentItems(items) => items
            .iter()
            .map(|item| match item {
                codex_protocol::models::FunctionCallOutputContentItem::InputText { text } => {
                    text.len()
                }
                codex_protocol::models::FunctionCallOutputContentItem::InputImage {
                    image_url,
                    ..
                } => image_url.len(),
            })
            .sum(),
    }
}

fn function_call_output_body_text(body: &FunctionCallOutputBody) -> String {
    match body {
        FunctionCallOutputBody::Text(text) => text.clone(),
        FunctionCallOutputBody::ContentItems(items) => items
            .iter()
            .filter_map(|item| match item {
                codex_protocol::models::FunctionCallOutputContentItem::InputText { text } => {
                    Some(text.as_str())
                }
                codex_protocol::models::FunctionCallOutputContentItem::InputImage { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn response_item_text(item: &ResponseItem) -> Option<String> {
    let parts = match item {
        ResponseItem::Message { role, content, .. } => {
            let mut parts = vec![role.clone()];
            parts.extend(
                content
                    .iter()
                    .filter_map(|content_item| match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                            Some(text.clone())
                        }
                        ContentItem::InputImage { image_url, .. } => Some(image_url.clone()),
                    }),
            );
            parts
        }
        ResponseItem::FunctionCall {
            name,
            namespace,
            arguments,
            ..
        } => {
            let mut parts = vec![name.clone(), arguments.clone()];
            if let Some(namespace) = namespace {
                parts.push(namespace.clone());
            }
            parts
        }
        ResponseItem::CustomToolCall { name, input, .. } => vec![name.clone(), input.clone()],
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            vec![function_call_output_body_text(&output.body)]
        }
        ResponseItem::ToolSearchCall {
            execution,
            arguments,
            ..
        } => vec![execution.clone(), arguments.to_string()],
        ResponseItem::ToolSearchOutput {
            execution, tools, ..
        } => {
            let mut parts = vec![execution.clone()];
            parts.extend(tools.iter().map(ToString::to_string));
            parts
        }
        ResponseItem::LocalShellCall { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::GhostSnapshot { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::Other => Vec::new(),
    };
    let parts = parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn response_item_text_contains(item: &ResponseItem, needle: &str) -> bool {
    response_item_texts_contain(item, &|text| text.contains(needle))
}

fn taskspace_gate_recovery_from_response_item(item: &ResponseItem) -> Option<String> {
    let output = match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            function_call_output_body_text(&output.body)
        }
        _ => return None,
    };
    output
        .contains(TASKSPACE_GATE_RECOVERY_MARKER)
        .then(|| output.chars().take(2200).collect::<String>())
}

fn response_item_texts_contain(item: &ResponseItem, predicate: &dyn Fn(&str) -> bool) -> bool {
    match item {
        ResponseItem::Message { role, content, .. } => {
            predicate(role)
                || content.iter().any(|content_item| match content_item {
                    ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                        predicate(text)
                    }
                    ContentItem::InputImage { image_url, .. } => predicate(image_url),
                })
        }
        ResponseItem::FunctionCall {
            name,
            namespace,
            arguments,
            ..
        } => predicate(name) || namespace.as_deref().is_some_and(predicate) || predicate(arguments),
        ResponseItem::CustomToolCall { name, input, .. } => predicate(name) || predicate(input),
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            function_call_output_body_texts_contain(&output.body, predicate)
        }
        ResponseItem::ToolSearchCall {
            execution,
            arguments,
            ..
        } => predicate(execution) || predicate(&arguments.to_string()),
        ResponseItem::ToolSearchOutput {
            execution, tools, ..
        } => predicate(execution) || tools.iter().any(|tool| predicate(&tool.to_string())),
        ResponseItem::LocalShellCall { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::GhostSnapshot { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::Other => false,
    }
}

fn function_call_output_body_texts_contain(
    body: &FunctionCallOutputBody,
    predicate: &dyn Fn(&str) -> bool,
) -> bool {
    match body {
        FunctionCallOutputBody::Text(text) => predicate(text),
        FunctionCallOutputBody::ContentItems(items) => items.iter().any(|item| match item {
            codex_protocol::models::FunctionCallOutputContentItem::InputText { text } => {
                predicate(text)
            }
            codex_protocol::models::FunctionCallOutputContentItem::InputImage {
                image_url, ..
            } => predicate(image_url),
        }),
    }
}

#[cfg(test)]
mod active_context_replacement_tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputPayload;

    fn message(role: &str, text: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: role.to_string(),
            content: vec![ContentItem::InputText {
                text: text.to_string(),
            }],
            end_turn: None,
            phase: None,
        }
    }

    fn tool_call(name: &str, call_id: &str) -> ResponseItem {
        ResponseItem::FunctionCall {
            id: None,
            name: name.to_string(),
            namespace: None,
            arguments: "{}".to_string(),
            call_id: call_id.to_string(),
        }
    }

    fn tool_output_with_call_id(call_id: &str, text: &str) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output: FunctionCallOutputPayload::from_text(text.to_string()),
        }
    }

    fn tool_output(text: &str) -> ResponseItem {
        tool_output_with_call_id("call-1", text)
    }

    fn item_texts(items: &[ResponseItem]) -> Vec<String> {
        items
            .iter()
            .filter_map(|item| match item {
                ResponseItem::Message { content, .. } => content.iter().find_map(|content| {
                    if let ContentItem::InputText { text } = content {
                        Some(text.clone())
                    } else {
                        None
                    }
                }),
                ResponseItem::FunctionCallOutput { output, .. } => output.body.to_text(),
                _ => None,
            })
            .collect()
    }

    fn item_text(item: ResponseItem) -> String {
        let ResponseItem::Message { content, .. } = item else {
            panic!("expected message item");
        };
        content
            .into_iter()
            .find_map(|content| match content {
                ContentItem::InputText { text } => Some(text),
                _ => None,
            })
            .expect("expected input text")
    }

    fn provider_snapshot(
        node_kind: &str,
    ) -> crate::action_map::ActionMapProviderRequestBudgetSnapshot {
        crate::action_map::ActionMapProviderRequestBudgetSnapshot {
            task_id: Some("task-1".to_string()),
            map_id: "map-1".to_string(),
            node_id: Some("node-1".to_string()),
            node_kind: Some(node_kind.to_string()),
            current_node_progress_signature: None,
            current_node_has_successful_edit: false,
            current_node_has_dependency_working_evidence: false,
            current_node_has_uncovered_mandatory_evidence: false,
            current_node_uncovered_mandatory_evidence: Vec::new(),
            current_node_validation_rework_artifacts: Vec::new(),
            route_mode: Some("thin".to_string()),
            profile_name: Some("taskspace-v005-active".to_string()),
            request_phase: Some("model_sampling".to_string()),
            provider_request_context_missing_reason: None,
            request_count: 1,
            max_requests: 8,
            node_request_count: 0,
            max_model_requests_per_node: 3,
            post_budget_grace_requests: 1,
            post_budget_grace_request_count: 0,
            budget_state: "normal".to_string(),
        }
    }

    #[test]
    fn taskspace_action_contract_parser_accepts_strict_json() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/lib.rs"},"rationale":"inspect"}"#,
        )
        .expect("valid action");

        assert_eq!(action.action, "read_file");
        assert_eq!(action.node_id.as_deref(), Some("node-1"));
        assert_eq!(
            taskspace_action_arg_string(&action.args, "path").as_deref(),
            Some("src/lib.rs")
        );
    }

    #[test]
    fn taskspace_action_contract_bootstrap_state_requires_start_task() {
        let text = item_text(taskspace_action_contract_bootstrap_state_item());

        assert!(text.contains("Active node kind: bootstrap"));
        assert!(text.contains("taskspace_control"));
        assert!(text.contains("action=start_task"));
    }

    #[test]
    fn taskspace_action_contract_state_guides_final_answer_without_active_node() {
        let mut snapshot = provider_snapshot("smoke_test");
        snapshot.node_id = None;
        snapshot.node_kind = None;

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Active node id: none"));
        assert!(text.contains("Existing TaskSpace task has no active bound node"));
        assert!(text.contains("return final_answer"));
        assert!(!text.contains("action=start_task"));
    }

    #[test]
    fn taskspace_action_contract_closed_validation_forbids_new_nodes() {
        let text = item_text(taskspace_action_contract_closed_validation_item());

        assert!(text.contains("TaskSpaceActionContractClosedValidationV1"));
        assert!(text.contains("final_answer or blocked only"));
        assert!(text.contains("Do not call start_task"));
        assert!(text.contains("create_node"));
        assert!(text.contains("validator infrastructure blocker"));
    }

    #[test]
    fn taskspace_action_contract_tool_runtime_bootstrap_failure_forbids_new_nodes() {
        let text = item_text(taskspace_action_contract_tool_runtime_bootstrap_failure_item());

        assert!(text.contains("TaskSpaceActionContractToolRuntimeBootstrapFailureV1"));
        assert!(text.contains("final_answer or blocked only"));
        assert!(text.contains("Do not call start_task"));
        assert!(text.contains("create_node"));
        assert!(text.contains("sandbox/tool runtime blocker"));
    }

    #[test]
    fn taskspace_action_contract_inspect_missing_scripts_narrows_to_read_file() {
        let scripts = vec!["generate_report.sh".to_string()];
        let text = item_text(taskspace_action_contract_inspect_unread_scripts_item(
            &scripts,
        ));

        assert!(text.contains("TaskSpaceActionContractInspectMissingScriptsV1"));
        assert!(text.contains("read_file or blocked only"));
        assert!(text.contains("generate_report.sh"));
        assert!(text.contains("The next action must be read_file"));
        assert!(text.contains("Do not call list_files"));
    }

    #[test]
    fn taskspace_action_contract_state_narrows_implementation_after_progress() {
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_progress_signature =
            Some(TASKSPACE_IMPLEMENT_PROGRESS_BEFORE_EDIT_HINT);

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Implementation convergence state: implementation_needs_edit"));
        assert!(text.contains("apply_patch, taskspace_control, or blocked only"));
        assert!(text.contains("Do not call list_files, search, read_file"));

        snapshot.current_node_has_successful_edit = true;
        let text_after_edit = item_text(taskspace_action_contract_state_item(&snapshot));
        assert!(!text_after_edit.contains("implementation_needs_edit"));
    }

    #[test]
    fn taskspace_action_contract_state_narrows_implementation_from_dependency_evidence() {
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_has_dependency_working_evidence = true;

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Implementation convergence state: implementation_needs_edit"));
        assert!(text.contains("Do not call list_files, search, read_file"));

        let read = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_report.sh"},"rationale":"read more"}"#,
        )
        .expect("valid read action");
        let err = taskspace_action_to_tool_call(&read, &snapshot)
            .expect_err("dependency evidence should narrow implementation to edit/block");
        assert_eq!(
            err,
            "node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        );
    }

    #[test]
    fn taskspace_action_contract_state_keeps_uncovered_mandatory_evidence_edit_pressure() {
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_has_successful_edit = true;
        snapshot.current_node_has_uncovered_mandatory_evidence = true;

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Implementation convergence state: implementation_needs_edit"));
        assert!(text.contains("Do not call list_files, search, read_file"));

        let read = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-2","args":{"path":"collect_data.sh"},"rationale":"read more"}"#,
        )
        .expect("valid read action");
        let err = taskspace_action_to_tool_call(&read, &snapshot)
            .expect_err("uncovered mandatory evidence should narrow implementation to edit/block");
        assert_eq!(
            err,
            "node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        );
    }

    #[test]
    fn taskspace_apply_patch_must_cover_uncovered_mandatory_evidence_target() {
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_has_successful_edit = true;
        snapshot.current_node_has_uncovered_mandatory_evidence = true;
        snapshot.current_node_uncovered_mandatory_evidence =
            vec!["generate_report.sh (invalid_shebang, result-13)".to_string()];

        let text = item_text(taskspace_action_contract_state_item(&snapshot));
        assert!(text.contains("Required edit targets from uncovered mandatory evidence"));
        assert!(text.contains("generate_report.sh (invalid_shebang, result-13)"));

        let wrong = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** Update File: report_generation.sh\n@@\n-#!/bin/nonexistent\n+#!/bin/bash\n*** End Patch\n"},"rationale":"fix shebang"}"#,
        )
        .expect("valid wrong patch action");
        let err = taskspace_action_to_tool_call(&wrong, &snapshot)
            .expect_err("wrong artifact must not satisfy mandatory evidence");
        assert_eq!(
            err,
            "apply_patch_missing_mandatory_evidence_targets:generate_report.sh"
        );

        let right = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** Update File: generate_report.sh\n@@\n-#!/bin/nonexistent\n+#!/bin/bash\n*** End Patch\n"},"rationale":"fix shebang"}"#,
        )
        .expect("valid right patch action");
        taskspace_action_to_tool_call(&right, &snapshot)
            .expect("right artifact should satisfy mandatory evidence")
            .expect("tool call");
    }

    #[test]
    fn taskspace_action_contract_bootstrap_allows_only_taskspace_control() {
        let start_task = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":null,"args":{"action":"start_task","title":"Fix test","objective":"Fix the failing test"},"rationale":"bootstrap"}"#,
        )
        .expect("valid bootstrap action");
        let call = taskspace_bootstrap_action_to_tool_call(&start_task)
            .expect("taskspace control should be allowed")
            .expect("taskspace control should execute");

        assert_eq!(call.tool_name.name, "taskspace_control");

        let read_file = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":null,"args":{"path":"README.md"}}"#,
        )
        .expect("valid action shape");
        let err = taskspace_bootstrap_action_to_tool_call(&read_file)
            .expect_err("bootstrap must not allow ordinary tools");
        assert!(err.contains("bootstrap_policy_violation"));
    }

    #[test]
    fn taskspace_action_contract_bootstrap_canonicalizes_action_name_alias() {
        let start_task = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":null,"args":{"action_name":"start_task","first_node_id":"inspect_context","first_node_kind":"inspect_code_context","initial_success_criteria":"Tax calculation tests pass","initial_fact_sources":["README","test files","source files"]},"rationale":"Initialize task"}"#,
        )
        .expect("valid bootstrap action");
        let call = taskspace_bootstrap_action_to_tool_call(&start_task)
            .expect("taskspace control should be allowed")
            .expect("taskspace control should execute");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "start_task");
                assert!(value.get("action_name").is_none());
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_preserves_start_task_rationale_as_objective() {
        let start_task = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":null,"args":{"action":"start_task","initial_node_kind":"inspect_code_context","initial_success_criteria":["found schema.json"],"initial_output_contracts":["schema structure summary"],"initial_fact_sources":["schema.json at root"]},"rationale":"Build a CSV-to-JSON processor that produces organization.json following schema.json."}"#,
        )
        .expect("valid start task action");
        let snapshot = provider_snapshot("inspect_code_context");
        let call = taskspace_action_to_tool_call(&start_task, &snapshot)
            .expect("start task normalizes")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "start_task");
                let objective = value["objective"].as_str().unwrap_or_default();
                assert!(objective.contains("organization.json"), "{objective}");
                assert!(objective.contains("schema.json"), "{objective}");
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_transport_preserves_existing_budget_limits() {
        let snapshot = provider_snapshot("inspect_code_context");
        let limits = taskspace_transport_budget_limits(&snapshot);

        assert_eq!(limits.max_requests, snapshot.max_requests);
        assert_eq!(
            limits.max_model_requests_per_node,
            snapshot.max_model_requests_per_node
        );
        assert_eq!(
            limits.post_budget_grace_requests,
            snapshot.post_budget_grace_requests
        );
    }

    #[test]
    fn taskspace_action_contract_parser_accepts_single_fenced_json() {
        let action = parse_taskspace_action_v1(
            "```json\n{\"schema_version\":\"taskspace-action-v1\",\"action\":\"read_file\",\"node_id\":\"node-1\",\"args\":{\"path\":\"README.md\"}}\n```",
        )
        .expect("single fenced json action should be recoverable");
        assert_eq!(action.action, "read_file");
        assert_eq!(
            taskspace_action_arg_string(&action.args, "path").as_deref(),
            Some("README.md")
        );
    }

    #[test]
    fn taskspace_action_contract_parser_rejects_non_json_fence() {
        let err =
            parse_taskspace_action_v1("```text\n{}\n```").expect_err("must reject non-json fence");
        assert_eq!(err, "action_contract_output_not_strict_json");
    }

    #[test]
    fn taskspace_action_contract_parser_accepts_deepseek_dsml_suffix() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"README.md"}}

<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("leading action json should be recoverable");

        assert_eq!(action.action, "read_file");
        assert_eq!(
            taskspace_action_arg_string(&action.args, "path").as_deref(),
            Some("README.md")
        );
    }

    #[test]
    fn taskspace_action_contract_parser_rejects_prose_suffix() {
        let err = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"README.md"}}
Then I will inspect the file."#,
        )
        .expect_err("non-DSML trailing prose remains invalid");
        assert_eq!(err, "action_contract_output_not_strict_json");
    }

    #[test]
    fn taskspace_action_contract_parser_recovers_json_after_prose_prefix() {
        let action = parse_taskspace_action_v1(
            r#"Let me read the key files.

{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"README.md"}}

<｜｜DSML｜｜tool_calls></｜｜DSML｜｜tool_calls>"#,
        )
        .expect("json after prose prefix should be recoverable");

        assert_eq!(action.action, "read_file");
        assert_eq!(
            taskspace_action_arg_string(&action.args, "path").as_deref(),
            Some("README.md")
        );
    }

    #[test]
    fn taskspace_action_contract_parser_recovers_pure_deepseek_dsml_read_file() {
        let action = parse_taskspace_action_v1(
            r#"<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
<｜｜DSML｜｜parameter name="command" string="true">Get-Content -LiteralPath "tests/test_tax_calc.py" -TotalCount 100</｜｜DSML｜｜parameter>
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("known DSML read command should map to read_file");

        assert_eq!(action.action, "read_file");
        assert_eq!(
            taskspace_action_arg_string(&action.args, "path").as_deref(),
            Some("tests/test_tax_calc.py")
        );
    }

    #[test]
    fn taskspace_action_contract_parser_recovers_dsml_path_and_type_reads() {
        let path_action = parse_taskspace_action_v1(
            r#"<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
<｜｜DSML｜｜parameter name="command" string="true">Get-Content -Path "README.md" -Raw</｜｜DSML｜｜parameter>
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("Get-Content -Path should map to read_file");
        assert_eq!(
            taskspace_action_arg_string(&path_action.args, "path").as_deref(),
            Some("README.md")
        );

        let type_action = parse_taskspace_action_v1(
            r#"<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
<｜｜DSML｜｜parameter name="command" string="true">type src\tax_calc.py</｜｜DSML｜｜parameter>
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("type should map to read_file");
        assert_eq!(
            taskspace_action_arg_string(&type_action.args, "path").as_deref(),
            Some("src\\tax_calc.py")
        );
    }

    #[test]
    fn taskspace_action_contract_parser_recovers_dsml_python_open_read() {
        let action = parse_taskspace_action_v1(
            r#"<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
<｜｜DSML｜｜parameter name="command" string="true">python -c "with open('src/tax_calc.py', 'r') as f: print(f.read())"</｜｜DSML｜｜parameter>
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("python open read should map to read_file");

        assert_eq!(
            taskspace_action_arg_string(&action.args, "path").as_deref(),
            Some("src/tax_calc.py")
        );
    }

    #[test]
    fn taskspace_action_contract_policy_allows_dsml_diagnostic_test_in_inspect_node() {
        let action = parse_taskspace_action_v1(
            r#"<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
<｜｜DSML｜｜parameter name="command" string="true">python -m pytest tests/test_tax_calc.py -v</｜｜DSML｜｜parameter>
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("known DSML test command should map to run_test");
        let call =
            taskspace_action_to_tool_call(&action, &provider_snapshot("inspect_code_context"))
                .expect("inspect nodes allow diagnostic tests")
                .expect("diagnostic test maps to shell command");

        assert_eq!(call.tool_name.name, "shell_command");
        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(
                    value["command"],
                    "python -m pytest tests/test_tax_calc.py -v"
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_policy_rejects_tests_in_implementation_node() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{"command":"cargo test"}}"#,
        )
        .expect("valid json");
        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("implement nodes cannot run tests");

        assert!(err.contains("node_policy_violation:implement_solution:run_test"));
    }

    #[test]
    fn taskspace_action_contract_blocks_late_implementation_reads_until_edit() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_report.sh"}}"#,
        )
        .expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_progress_signature =
            Some(TASKSPACE_IMPLEMENT_PROGRESS_BEFORE_EDIT_HINT);
        snapshot.current_node_has_successful_edit = false;

        let err = taskspace_action_to_tool_call(&action, &snapshot)
            .expect_err("implementation reads should stop after enough evidence");

        assert!(err.contains(
            "node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        ));

        snapshot.current_node_has_successful_edit = true;
        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("successful edit clears late read block")
            .expect("read_file still maps to a shell command after edit");
        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn taskspace_action_contract_allows_named_validation_rework_artifact_read() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_org.py"}}"#,
        )
        .expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_progress_signature =
            Some(TASKSPACE_IMPLEMENT_PROGRESS_BEFORE_EDIT_HINT);
        snapshot.current_node_validation_rework_artifacts = vec!["generate_org.py".to_string()];

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("named validation rework target read should be allowed")
            .expect("read_file maps to a shell command");
        assert_eq!(call.tool_name.name, "shell_command");

        let broad_read = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"schema.json"}}"#,
        )
        .expect("valid json");
        let err = taskspace_action_to_tool_call(&broad_read, &snapshot)
            .expect_err("non-target read remains blocked");
        assert!(err.contains(
            "node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        ));

        let state_text = item_text(taskspace_action_contract_state_item(&snapshot));
        assert!(state_text.contains("generate_org.py"));
        assert!(state_text.contains("may be read once"));
    }

    #[test]
    fn taskspace_implementation_needs_edit_rejection_uses_specific_recovery() {
        assert!(taskspace_message_hit_implementation_needs_edit(Some(
            "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        )));
        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "result-7: generate_report.sh preview #!/bin/nonexistent",
        ));
        assert!(response_item_text_contains(
            &item,
            TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER
        ));
        assert!(response_item_text_contains(&item, "call apply_patch"));
        assert!(response_item_text_contains(&item, "do not rediscover"));
        assert!(response_item_text_contains(&item, "#!/bin/nonexistent"));
    }

    #[test]
    fn implementation_needs_edit_hard_stop_triggers_on_third_plain_recovery() {
        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "validation_schema_repair_contract: target_artifacts=generate_organization.py",
        ));

        assert!(is_taskspace_plain_implement_needs_edit_recovery_item(&item));
        assert!(!taskspace_implementation_needs_edit_should_hard_stop(
            &item, 2
        ));
        assert!(taskspace_implementation_needs_edit_should_hard_stop(
            &item, 3
        ));

        let hard_stop = build_taskspace_implementation_needs_edit_hard_stop_item(&item, 3);
        let text = item_text(hard_stop.clone());

        assert!(text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_HARD_STOP_MARKER));
        assert!(text.contains("repeated_finish_without_successful_edit"));
        assert!(text.contains("attempt_count: 3"));
        assert!(text.contains("apply_patch-or-block recovery contract"));
        assert!(!is_taskspace_plain_implement_needs_edit_recovery_item(
            &hard_stop
        ));
        assert!(!is_taskspace_implement_needs_edit_recovery_item(&hard_stop));
        assert!(
            taskspace_special_recovery_warning_message(&hard_stop)
                .contains("TaskSpaceImplementationNeedsEditHardStopV1")
        );
    }

    #[test]
    fn validation_rework_duplicate_read_rejection_uses_edit_recovery() {
        assert!(taskspace_message_hit_implementation_needs_edit(Some(
            "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `recover.py` in result `result-12` and no successful edit has been recorded after that read. Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made."
        )));

        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "result-12: recover.py current binary scan recovered 2 rows",
        ));
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(text.contains("recover.py"));
        assert!(text.contains("apply_patch"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn validation_rework_duplicate_read_recovery_preserves_patch_only_contract() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-10` and no successful edit has been recorded after that read. Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | schema_required_groups=schema.json:properties.statistics requires averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService | target_artifacts=process.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `process.py`\"]}";
        let item = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some(
                "validation_rework: SmokeTest `node-3` failed schema validation | result-10 artifacts=process.py",
            ),
            None,
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(text.contains("failure_kind: validation_rework_duplicate_artifact_read"));
        assert!(text.contains("target_artifact: process.py"));
        assert!(text.contains("previous_read_result: result-10"));
        assert!(text.contains("repair_contract: missing_required_properties=members"));
        assert!(text.contains("projectStatusDistribution"));
        assert!(text.contains("TaskSpaceGateRecoveryV1"));
        assert!(text.contains("Emit exactly one taskspace-action-v1 apply_patch"));
        assert!(text.contains("Use native apply_patch grammar only"));
        assert!(text.contains("Do not include `--- Update File:`"));
        assert!(text.contains("`--- a/...`"));
        assert!(text.contains("Do not call read_file"));
        assert!(text.contains("Do not repeat the blocked read"));
        let required_behavior_pos = text
            .find("Current required behavior:")
            .expect("required behavior heading");
        let evidence_pos = text
            .find("Already inspected evidence available to use now:")
            .expect("evidence heading");
        assert!(
            required_behavior_pos < evidence_pos,
            "patch directive must precede long evidence block:\n{text}"
        );
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn validation_rework_duplicate_read_recovery_preserves_failed_patch_grammar() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `generate_org.py`\"]}";
        let failed_edit = "TaskSpaceActionV1 rejected: apply_patch_mixed_native_unified:generate_org.py. Return exactly one valid taskspace-action-v1 JSON object.";
        let item = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some("validation_rework result-10 artifacts=generate_org.py"),
            Some(failed_edit),
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(text.contains("Most recent failed edit feedback to preserve"));
        assert!(text.contains("apply_patch_mixed_native_unified:generate_org.py"));
        assert!(text.contains("correct that patch grammar now"));
        assert!(text.contains("read_file/context refresh is not a valid recovery"));
        assert!(text.contains("`--- a/...`"));
        assert!(!text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(
            taskspace_implement_recovery_advisory_warning_message(&item, 7)
                .contains("TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1")
        );
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn validation_rework_duplicate_read_recovery_preserves_unanchored_patch_feedback() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `generate.py`\"]}";
        let failed_edit = "TaskSpaceActionV1 rejected: apply_patch_unanchored_update:generate.py. Return exactly one valid taskspace-action-v1 JSON object.";
        let item = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some("validation_rework result-10 artifacts=generate.py"),
            Some(failed_edit),
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(text.contains("Most recent failed edit feedback to preserve"));
        assert!(text.contains("apply_patch_unanchored_update:generate.py"));
        assert!(text.contains("correct that patch grammar now"));
        assert!(text.contains("read_file/context refresh is not a valid recovery"));
        assert!(
            text.find("Most recent failed edit feedback to preserve")
                < text.find("Previous blocked feedback")
        );
        assert!(
            taskspace_implement_recovery_advisory_warning_message(&item, 6)
                .contains("TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1")
        );
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn implementation_recovery_prioritizes_duplicate_rework_read_feedback() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `process.py`\"]}";
        let item = build_taskspace_implementation_recovery_item(
            Some(last_message),
            Some("result-10 artifacts=process.py"),
            None,
        );
        let text = item_text(item);

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(!text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(text.contains("previous_read_result: result-10"));
    }

    #[test]
    fn implementation_recovery_selects_patch_only_after_target_read_evidence() {
        let last_message = "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit. Return exactly one valid taskspace-action-v1 JSON object.";
        let evidence = "validation_rework: smoke_test `node-3` failed result `result-10`: missing_required_properties: members, averageDepartmentBudget \
| validation_rework_target_read result=result-12 artifact=generate_org.py excerpt: member_ids -> members \
| result-2 artifacts=schema.json: required members and averageDepartmentBudget";
        let item =
            build_taskspace_implementation_recovery_item(Some(last_message), Some(evidence), None);
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(text.contains("failure_kind: validation_rework_patch_only_after_target_read"));
        assert!(text.contains("target_artifacts: generate_org.py"));
        assert!(text.contains("Do not move from the named target artifact to `schema.json`"));
        assert!(text.contains("no additional file lines are hidden"));
        assert!(text.contains("Emit exactly one taskspace-action-v1 apply_patch"));
        assert!(text.contains("Patch construction scaffold:"));
        assert!(text.contains("convert `schema_property_rename_hints` into output key renames"));
        assert!(text.contains("Schema repair synthesis from current validation failure:"));
        assert!(text.contains("Missing required output properties"));
        assert!(text.contains("`members`"));
        assert!(text.contains("`averageDepartmentBudget`"));
        assert!(text.contains("Use native apply_patch grammar only"));
        assert!(text.contains("Final action lock:"));
        assert!(text.contains("projection truncation is not a valid reason to read"));
        assert!(text.contains("whole-file native replacement"));
        assert!(!text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
        assert!(!is_taskspace_plain_implement_needs_edit_recovery_item(
            &item
        ));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(
            taskspace_implement_recovery_advisory_warning_message(&item, 4)
                .contains("TaskSpaceValidationReworkPatchOnlyRecoveryV1")
        );
        assert!(!taskspace_validation_rework_patch_only_should_hard_stop(
            &item, 0
        ));
        assert!(taskspace_validation_rework_patch_only_should_hard_stop(
            &item, 1
        ));
    }

    #[test]
    fn validation_rework_patch_only_prefers_explicit_target_artifacts() {
        let evidence = "validation_rework_target_read result=result-12 artifact=generate.py \
| validation_schema_repair_contract: missing_required_properties=members | target_artifacts=generate.py | patch_requirement=update generated output \
| result-2 artifacts=schema.json: required members | result-3 artifacts=departments.csv:";

        let artifacts = taskspace_validation_rework_patch_only_artifacts(evidence);

        assert_eq!(artifacts, vec!["generate.py".to_string()]);
    }

    #[test]
    fn implementation_recovery_selects_patch_only_after_closed_action_space_read_reject() {
        let last_message = "TaskSpaceActionV1 rejected: validation_rework_closed_action_space_read_disallowed:read_file. Return exactly one valid taskspace-action-v1 JSON object.";
        assert!(taskspace_message_hit_implementation_needs_edit(Some(
            last_message
        )));
        let evidence = "validation_rework: smoke_test `node-3` failed result `result-10`: missing_required_properties: members, averageDepartmentBudget \
| validation_rework_target_read result=result-12 artifact=generate_organization.py read_context: complete_read eof_reached=true content_visibility: full_content_visible excerpt: member_ids -> members \
| validation_schema_repair_contract: missing_required_properties=members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService | schema_property_rename_hints=member_ids->members";
        let item =
            build_taskspace_implementation_recovery_item(Some(last_message), Some(evidence), None);
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(text.contains("target_artifacts: generate_organization.py"));
        assert!(text.contains("Emit exactly one taskspace-action-v1 apply_patch"));
        assert!(text.contains("no additional file lines are hidden"));
        assert!(text.contains("Schema repair synthesis from current validation failure:"));
        assert!(text.contains("`members`"));
        assert!(text.contains("`averageYearsOfService`"));
        assert!(text.contains("`member_ids->members`"));
        assert!(text.contains("This is a patch-construction requirement"));
        assert!(text.contains("Patch construction scaffold:"));
        assert!(text.contains("Complete target-read direct replacement scaffold"));
        assert!(text.contains("*** Delete File: generate_organization.py"));
        assert!(text.contains("*** Add File: generate_organization.py"));
        assert!(
            text.find("Complete target-read direct replacement scaffold")
                < text.find("Previous blocked feedback")
        );
        assert!(text.contains("Do not put markdown fences"));
        assert!(!text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(
            taskspace_implement_recovery_advisory_warning_message(&item, 4)
                .contains("TaskSpaceValidationReworkPatchOnlyRecoveryV1")
        );
    }

    #[test]
    fn implementation_recovery_prioritizes_failed_edit_over_patch_only_after_target_read() {
        let last_message =
            "TaskSpace inserted TaskSpaceImplementNeedsEditRecoveryV1 after failed edit.";
        let evidence = "validation_rework_target_read result=result-12 artifact=process.py read_context: complete_read eof_reached=true content_visibility: full_content_visible excerpt: \
| validation_schema_repair_contract: missing_required_properties=members,totalEmployees";
        let failed_edit = "result-13: apply_patch verification failed: Failed to find expected lines in process.py:\nimport csv";

        let item = build_taskspace_implementation_recovery_item(
            Some(last_message),
            Some(evidence),
            Some(failed_edit),
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(text.contains("Failed to find expected lines"));
        assert!(text.contains("Structured failed-edit contract"));
        assert!(text.contains("failure_kind: apply_patch_expected_lines_mismatch"));
        assert!(text.contains("failed_target: process.py"));
        assert!(text.contains("mandatory_next_action: do not repeat the failed hunk"));
        assert!(text.contains("do not repeat the same hunk"));
        assert!(text.contains("Complete target-read recovery override"));
        assert!(text.contains("*** Delete File"));
        assert!(text.contains("*** Add File"));
        assert!(text.contains("Do not emit `*** Update File`"));
        assert!(text.contains("Do not call read_file"));
        assert!(!text.contains("one narrow read_file of the same failed target artifact"));
        assert!(!text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(!is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
    }

    #[test]
    fn edit_failure_recovery_normalizes_double_app_missing_target() {
        let failed_edit = "result-15: apply_patch verification failed: Failed to read file to update /tmp/run/right/app/app/process.py: No such file or directory (os error 2)";
        let item = build_taskspace_edit_failure_recovery_item(Some(failed_edit), None);
        let text = item_text(item);

        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(text.contains("failure_kind: apply_patch_missing_update_target"));
        assert!(text.contains("failed_target: process.py"));
        assert!(!text.contains("failed_target: app/process.py"));
    }

    #[test]
    fn validation_rework_patch_only_hard_stops_after_one_recovery() {
        let item = build_taskspace_validation_rework_patch_only_recovery_item(
            Some(
                "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit",
            ),
            Some(
                "validation_rework_target_read result=result-12 artifact=generate_org.py | validation_schema_repair_contract: missing_required_properties=members",
            ),
            None,
        );
        let recovery_text = item_text(item.clone());
        let required_behavior_pos = recovery_text
            .find("Current required behavior:")
            .expect("required behavior heading");
        let evidence_pos = recovery_text
            .find("Already inspected evidence available to use now:")
            .expect("evidence heading");
        assert!(
            required_behavior_pos < evidence_pos,
            "patch directive must precede long evidence block:\n{recovery_text}"
        );

        assert!(!taskspace_validation_rework_patch_only_should_hard_stop(
            &item, 0
        ));
        assert!(taskspace_validation_rework_patch_only_should_hard_stop(
            &item, 1
        ));

        let hard_stop = build_taskspace_validation_rework_patch_only_hard_stop_item(&item, 2);
        let text = item_text(hard_stop.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_HARD_STOP_MARKER));
        assert!(text.contains("reason: repeated_non_edit_after_validation_rework_target_read"));
        assert!(text.contains("target_artifacts: generate_org.py"));
        assert!(is_taskspace_validation_rework_patch_only_hard_stop_item(
            &hard_stop
        ));
        assert!(!is_taskspace_no_action_recovery_item(&hard_stop));
        assert!(!is_taskspace_implement_needs_edit_recovery_item(&hard_stop));
        assert!(
            taskspace_special_recovery_warning_message(&hard_stop)
                .contains("TaskSpaceValidationReworkPatchOnlyHardStopV1")
        );
    }

    #[test]
    fn validation_rework_recovery_count_resets_when_rework_node_changes() {
        let mut key = None;
        let mut count = 2usize;
        let mut node_4 = provider_snapshot("implement_solution");
        node_4.node_id = Some("node-4".to_string());
        let mut node_6 = provider_snapshot("implement_solution");
        node_6.node_id = Some("node-6".to_string());

        taskspace_reset_recovery_count_for_snapshot_node(&mut key, &mut count, Some(&node_4));
        assert_eq!(key.as_deref(), Some("node-4"));
        assert_eq!(count, 0);

        count = 1;
        taskspace_reset_recovery_count_for_snapshot_node(&mut key, &mut count, Some(&node_4));
        assert_eq!(key.as_deref(), Some("node-4"));
        assert_eq!(count, 1);

        taskspace_reset_recovery_count_for_snapshot_node(&mut key, &mut count, Some(&node_6));
        assert_eq!(key.as_deref(), Some("node-6"));
        assert_eq!(count, 0);

        count = 3;
        taskspace_reset_recovery_count_for_snapshot_node(&mut key, &mut count, None);
        assert_eq!(key.as_deref(), Some("unknown-node"));
        assert_eq!(count, 0);
    }

    #[test]
    fn implementation_recovery_does_not_enter_patch_only_before_target_read() {
        let item = build_taskspace_implementation_recovery_item(
            Some(
                "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit",
            ),
            Some(
                "validation_schema_repair_contract: missing_required_properties=members | target_artifacts=generate_org.py",
            ),
            None,
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(!text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(is_taskspace_plain_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn implementation_recovery_preserves_failed_patch_grammar_on_duplicate_read() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `generate_org.py`\"]}";
        let item = build_taskspace_implementation_recovery_item(
            Some(last_message),
            Some("result-10 artifacts=generate_org.py"),
            Some("TaskSpaceActionV1 rejected: apply_patch_unanchored_update:generate_org.py"),
        );
        let text = item_text(item);

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(text.contains("apply_patch_unanchored_update:generate_org.py"));
        assert!(text.contains("correct that patch grammar now"));
        assert!(!text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
    }

    #[test]
    fn implementation_recovery_selects_duplicate_rework_from_gate_text_without_reason() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate.py` in result `result-11` and no successful edit has been recorded after that read. Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | target_artifacts=generate.py";
        let item = build_taskspace_implementation_recovery_item(
            Some(last_message),
            Some("validation_rework result-11 artifacts=generate.py"),
            None,
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(text.contains("failure_kind: validation_rework_duplicate_artifact_read"));
        assert!(text.contains("target_artifact: generate.py"));
        assert!(text.contains("previous_read_result: result-11"));
        assert!(text.contains("repair_contract: missing_required_properties=members"));
        assert!(!text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(
            taskspace_special_recovery_warning_message(&item)
                .contains("TaskSpaceValidationReworkDuplicateReadRecoveryV1")
        );
    }

    #[test]
    fn implement_recovery_prioritizes_validation_failure_and_inspected_fields() {
        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "validation_rework: SmokeTest `node-7` has blocked validation evidence `result-9`: KeyError: 'salary' in generate_organization.py | result-3 artifacts=data/employees.csv: employee_id,name,department_id,title",
        ));
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(text.contains("validation failure"));
        assert!(text.contains("primary target"));
        assert!(text.contains("IndentationError"));
        assert!(text.contains("whole affected file"));
        assert!(text.contains("KeyError"));
        assert!(text.contains("do not invent unobserved columns"));
        assert!(text.contains("employee_id,name,department_id,title"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn taskspace_strict_json_apply_patch_intent_rejection_is_detected() {
        let raw = r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** Update File: app.py\n*** End Patch\n"}} extra"#;
        assert!(taskspace_raw_text_mentions_apply_patch_intent(raw));
        let message = "TaskSpaceActionV1 rejected: action_contract_output_not_strict_json:apply_patch_intent. Rejected assistant output preview: {\"schema_version\":\"taskspace-action-v1\",\"action\":\"apply_patch\"} extra. Return exactly one valid taskspace-action-v1 JSON object.";

        assert!(taskspace_message_hit_apply_patch_intent_format_rejection(
            Some(message)
        ));
        assert_eq!(
            taskspace_rejected_apply_patch_intent_preview(Some(message)),
            Some("{\"schema_version\":\"taskspace-action-v1\",\"action\":\"apply_patch\"} extra")
        );
    }

    #[test]
    fn taskspace_patch_intent_format_recovery_forces_single_patch_action() {
        let item = build_taskspace_patch_intent_format_recovery_item(
            Some("result-9: task_deps/generator.log traceback"),
            Some("{\"schema_version\":\"taskspace-action-v1\",\"action\":\"apply_patch\"} extra"),
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_PATCH_INTENT_FORMAT_MARKER));
        assert!(text.contains("not exactly one taskspace-action-v1 JSON object"));
        assert!(text.contains("Emit exactly one valid taskspace-action-v1 JSON object now"));
        assert!(text.contains("apply_patch with the patch payload"));
        assert!(text.contains("Do not call read_file, list_files, search"));
        assert!(text.contains("task_deps/generator.log"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn taskspace_apply_patch_missing_target_error_is_detected() {
        let windows_message = "TaskSpace tool call failed: apply_patch verification failed: Failed to read file to update W:\\app\\src\\recover_accuracy.py: 系统找不到指定的路径。 (os error 3)";
        let unix_message = "TaskSpace tool call failed: apply_patch verification failed: Failed to read file to update /app/src/recover_accuracy.py: No such file or directory (os error 2)";

        assert_eq!(
            taskspace_missing_update_targets_from_apply_patch_error(Some(windows_message)),
            Some("recover_accuracy.py".to_string())
        );
        assert_eq!(
            taskspace_missing_update_targets_from_apply_patch_error(Some(unix_message)),
            Some("recover_accuracy.py".to_string())
        );
        assert_eq!(
            taskspace_missing_update_targets_from_apply_patch_error(Some(
                "TaskSpace tool call failed: failed to parse function arguments"
            )),
            None
        );
    }

    #[test]
    fn taskspace_apply_patch_expected_lines_target_is_detected() {
        let windows_message = "apply_patch verification failed: Failed to find expected lines in V:\\app\\convert.py:\n    import pandas as pd";
        let unix_message = "apply_patch verification failed: Failed to find expected lines in /app/src/recover_accuracy.py:\n    def recover";
        let flattened_message = "result-16: apply_patch verification failed: Failed to find expected lines in /tmp/run/right/app/generate.py: total_projects = len(projects) budgets = [int(d['budget']) for d in departments]";

        assert_eq!(
            taskspace_expected_lines_target_from_apply_patch_text(windows_message),
            Some("convert.py".to_string())
        );
        assert_eq!(
            taskspace_expected_lines_target_from_apply_patch_text(unix_message),
            Some("recover_accuracy.py".to_string())
        );
        assert_eq!(
            taskspace_expected_lines_target_from_apply_patch_text(flattened_message),
            Some("generate.py".to_string())
        );
        assert_eq!(
            taskspace_expected_lines_target_from_apply_patch_text(
                "apply_patch verification failed: invalid hunk"
            ),
            None
        );
    }

    #[test]
    fn taskspace_apply_patch_context_mismatch_target_is_detected() {
        let windows_message = "apply_patch verification failed: Failed to find context '-1,1 +1,1 @@' in S:\\app\\recover.py";
        let unix_message = "apply_patch verification failed: Failed to find context '-4,3 +4,5 @@' in /app/src/recover_accuracy.py";

        assert_eq!(
            taskspace_context_mismatch_target_from_apply_patch_text(windows_message),
            Some("recover.py".to_string())
        );
        assert_eq!(
            taskspace_context_mismatch_target_from_apply_patch_text(unix_message),
            Some("recover_accuracy.py".to_string())
        );
        assert_eq!(
            taskspace_context_mismatch_target_from_apply_patch_text(
                "apply_patch verification failed: invalid hunk"
            ),
            None
        );
    }

    #[test]
    fn apply_patch_expected_lines_feedback_allows_target_context_refresh() {
        let summary = taskspace_action_contract_tool_feedback_summary(
            "taskspace-action-contract-12-apply_patch",
            "apply_patch verification failed: Failed to find expected lines in /app/generate_org.py:\n import csv",
            Some(false),
        );

        assert!(summary.contains("failure_kind: apply_patch_expected_lines_mismatch"));
        assert!(summary.contains("target: generate_org.py"));
        assert!(summary.contains("one read_file of `generate_org.py`"));
        assert!(summary.contains("current target context is truncated/stale"));

        let recovery = build_taskspace_edit_failure_recovery_item(
            Some(
                "result-11: apply_patch verification failed: Failed to find expected lines in generate_org.py",
            ),
            Some("result-10 artifacts=generate_org.py excerpt truncated"),
        );
        let text = item_text(recovery);
        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(text.contains("one narrow read_file of the same failed target artifact"));
        assert!(text.contains("read the same target artifact once to refresh context"));
    }

    #[test]
    fn complete_validation_rework_expected_lines_failure_forces_full_rewrite() {
        let recovery = build_taskspace_edit_failure_recovery_item(
            Some(
                "result-14: apply_patch verification failed: Failed to find expected lines in process.py",
            ),
            Some(
                "validation_rework_target_read result-12 artifacts=process.py read_context: complete_read eof_reached=true content_visibility: full_content_visible | missing_required_properties=id, members",
            ),
        );
        let text = item_text(recovery);

        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(text.contains("Complete target-read recovery override"));
        assert!(text.contains("content_visibility=full_content_visible"));
        assert!(text.contains("*** Delete File: <path>"));
        assert!(text.contains("*** Add File: <path>"));
        assert!(text.contains("do not refresh read"));
        assert!(text.contains("Do not emit `*** Update File`"));
        assert!(!text.contains("one narrow read_file of the same failed target artifact"));
    }

    #[test]
    fn complete_validation_rework_summary_only_does_not_force_full_rewrite() {
        let recovery = build_taskspace_edit_failure_recovery_item(
            Some(
                "result-14: apply_patch verification failed: Failed to find expected lines in process.py",
            ),
            Some(
                "validation_rework_target_read result-12 artifacts=process.py read_context: complete_read eof_reached=true content_visibility: summary_excerpt_only | missing_required_properties=id, members",
            ),
        );
        let text = item_text(recovery);

        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(!text.contains("Complete target-read recovery override"));
        assert!(text.contains("one narrow read_file of the same failed target artifact"));
    }

    #[test]
    fn taskspace_apply_patch_missing_target_recovery_forces_add_file_grammar() {
        let targets = taskspace_missing_update_targets_from_apply_patch_error(Some(
            "TaskSpace tool call failed: apply_patch verification failed: Failed to read file to update W:\\app\\src\\recover_accuracy.py: 系统找不到指定的路径。 (os error 3)",
        ))
        .expect("missing update target parsed");
        let item = build_taskspace_apply_patch_missing_target_recovery_item(&targets);
        let text = item_text(item.clone());

        assert_eq!(targets, "recover_accuracy.py");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER));
        assert!(text.contains("recover_accuracy.py"));
        assert!(text.contains("*** Add File: <relative/path>"));
        assert!(text.contains("Do not return blocked merely because the file does not exist"));
        assert!(text.contains("Do not use `--- /dev/null`"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn taskspace_action_contract_final_answer_allowed_without_active_node() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"final_answer","node_id":null,"args":{"message":"All tests pass."}}"#,
        )
        .expect("valid json");
        let mut snapshot = provider_snapshot("unknown");
        snapshot.node_id = None;
        snapshot.node_kind = None;

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("final answer should bypass node tool policy");

        assert!(call.is_none());
    }

    #[test]
    fn taskspace_action_contract_final_answer_visible_text_uses_message() {
        let text = taskspace_action_contract_visible_text(
            r#"{"schema_version":"taskspace-action-v1","action":"final_answer","node_id":null,"args":{"message":"All tests pass."},"rationale":"done"}"#,
        )
        .expect("final answer message");

        assert_eq!(text, "All tests pass.");
    }

    #[test]
    fn taskspace_terminal_actions_are_detected() {
        let final_answer = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"final_answer","node_id":"node-3","args":{"message":"All tests pass."}}"#,
        )
        .expect("valid final answer");
        let blocked = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"blocked","node_id":"node-3","args":{"reason":"budget exhausted"}}"#,
        )
        .expect("valid blocked action");
        let read_file = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-3","args":{"path":"README.md"}}"#,
        )
        .expect("valid read action");

        assert!(taskspace_action_is_terminal(&final_answer));
        assert!(taskspace_action_is_terminal(&blocked));
        assert!(!taskspace_action_is_terminal(&read_file));
    }

    #[test]
    fn taskspace_final_answer_does_not_block_successful_required_action_auto_finish() {
        let final_answer = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"final_answer","node_id":"node-3","args":{"message":"All tests pass."}}"#,
        )
        .expect("valid final answer");
        let finish_node = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-3","args":{"action":"finish_node","outcome":"success","summary":"tests pass"}}"#,
        )
        .expect("valid finish node");

        assert!(!taskspace_action_blocks_successful_required_action_auto_finish(&final_answer));
        assert!(taskspace_action_blocks_successful_required_action_auto_finish(&finish_node));
    }

    #[test]
    fn taskspace_terminal_action_clears_follow_up_state() {
        let mut needs_follow_up = true;
        let mut saw_actionable_output = true;
        let mut last_agent_message =
            Some("A successful implementation edit is already recorded.".to_string());

        apply_taskspace_terminal_action_message(
            &mut needs_follow_up,
            &mut saw_actionable_output,
            &mut last_agent_message,
            "All tests pass.".to_string(),
        );

        assert!(!needs_follow_up);
        assert!(!saw_actionable_output);
        assert_eq!(last_agent_message.as_deref(), Some("All tests pass."));
        assert_eq!(
            classify_taskspace_provider_response_actionability(
                needs_follow_up,
                saw_actionable_output,
                true,
                false,
                false,
                false,
            ),
            TaskspaceProviderResponseActionability::FinalCandidate
        );
    }

    #[test]
    fn taskspace_blocked_action_blocks_active_node() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"blocked","node_id":"node-1","args":{"reason":"local validator failed with E_ACCESSDENIED"}}"#,
        )
        .expect("valid blocked action");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("tool call");

        assert_eq!(call.tool_name.name.as_str(), "taskspace_control");
        assert!(call.call_id.ends_with("-blocked"));
        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(
                    value.get("action").and_then(serde_json::Value::as_str),
                    Some("block_node")
                );
                assert_eq!(
                    value.get("node_id").and_then(serde_json::Value::as_str),
                    Some("node-1")
                );
                assert_eq!(
                    value
                        .get("blocker_summary")
                        .and_then(serde_json::Value::as_str),
                    Some("local validator failed with E_ACCESSDENIED")
                );
            }
            _ => panic!("expected function payload"),
        }
    }

    #[test]
    fn action_contract_failed_validation_finish_normalizes_to_block_node() {
        let snapshot = provider_snapshot("smoke_test");
        let args = serde_json::json!({
            "action": "finish_node",
            "status": "failed",
            "reason": "Test failed with IndentationError on process.py line 1."
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("failed validation finish should normalize");

        assert_eq!(normalized["action"], "block_node");
        assert_eq!(normalized["node_id"], "node-1");
        assert_eq!(
            normalized["blocker_summary"],
            "Test failed with IndentationError on process.py line 1."
        );
        assert!(normalized.get("reason").is_none());
    }

    #[test]
    fn action_contract_feedback_requires_current_test_after_stale_validation_block() {
        let tool_call = ToolCall {
            tool_name: ToolName::plain("taskspace_control"),
            call_id: "taskspace-action-contract-17-blocked".to_string(),
            payload: ToolPayload::Function {
                arguments: "{\"action\":\"block_node\"}".to_string(),
            },
        };
        let err = CodexErr::Fatal(
            "TaskSpace smoke_test node `node-5` cannot be blocked as failed validation before this node records a test/build result. Run the required validation command on `node-5` first, or block only with a specific external blocker that prevents validation from running.".to_string(),
        );
        let response_input = response_input_for_taskspace_action_tool_error(&tool_call, &err);
        let response_item: ResponseItem = response_input.into();
        let (_, summary) = taskspace_action_contract_tool_output_summary(&response_item)
            .expect("failed action-contract output summarizes");
        let recent = taskspace_action_contract_recent_tool_outputs_item(
            &[response_item],
            Some("smoke_test"),
        )
        .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("failure_kind: validation_stale_failure_without_current_test"));
        assert!(summary.contains("next_valid_action: emit exactly one run_test action"));
        assert!(text.contains("Next action must be run_test"));
        assert!(text.contains("do not finish_node"));
        assert!(taskspace_message_hit_validation_needs_test(Some(&summary)));
    }

    #[test]
    fn action_contract_feedback_requires_current_test_after_validation_finish_without_result() {
        let tool_call = ToolCall {
            tool_name: ToolName::plain("taskspace_control"),
            call_id: "taskspace-action-contract-18-taskspace_control".to_string(),
            payload: ToolPayload::Function {
                arguments: "{\"action\":\"finish_node\"}".to_string(),
            },
        };
        let err = CodexErr::Fatal(
            "TaskSpace smoke_test node `node-5` cannot be completed without a recorded successful test or build action. Run validation in this node, or block it if validation fails and create a follow-up implementation node.".to_string(),
        );
        let response_input = response_input_for_taskspace_action_tool_error(&tool_call, &err);
        let response_item: ResponseItem = response_input.into();
        let (_, summary) = taskspace_action_contract_tool_output_summary(&response_item)
            .expect("failed action-contract output summarizes");
        let recent = taskspace_action_contract_recent_tool_outputs_item(
            &[response_item],
            Some("smoke_test"),
        )
        .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("failure_kind: validation_finish_missing_current_test_result"));
        assert!(summary.contains("next_valid_action: emit exactly one run_test action"));
        assert!(text.contains("Next action must be run_test"));
        assert!(text.contains("before this node had its own test/build result"));
        assert!(taskspace_message_hit_validation_needs_test(Some(&summary)));
    }

    #[test]
    fn action_contract_feedback_requires_patch_after_rework_duplicate_read() {
        let tool_call = ToolCall {
            tool_name: ToolName::plain("shell_command"),
            call_id: "taskspace-action-contract-19-read_file".to_string(),
            payload: ToolPayload::Function {
                arguments: "{\"command\":\"sed -n '1,240p' -- process.py\"}".to_string(),
            },
        };
        let err = CodexErr::Fatal(
            "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-10` and no successful edit has been recorded after that read. Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | schema_required_groups=schema.json:properties.statistics requires averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService | target_artifacts=process.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `process.py`\"]}".to_string(),
        );
        let response_input = response_input_for_taskspace_action_tool_error(&tool_call, &err);
        let response_item: ResponseItem = response_input.into();
        let (_, summary) = taskspace_action_contract_tool_output_summary(&response_item)
            .expect("failed action-contract output summarizes");
        let recent = taskspace_action_contract_recent_tool_outputs_item(
            &[response_item],
            Some("implement_solution"),
        )
        .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("failure_kind: validation_rework_duplicate_artifact_read"));
        assert!(summary.contains("target_artifact: process.py"));
        assert!(summary.contains("previous_read_result: result-10"));
        assert!(summary.contains("repair_contract: missing_required_properties=members"));
        assert!(summary.contains("projectStatusDistribution"));
        assert!(summary.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(summary.contains("satisfy it exactly"));
        assert!(text.contains("Next action must be apply_patch"));
        assert!(text.contains("Do not read_file"));
        assert!(taskspace_message_hit_implementation_needs_edit(Some(
            &summary
        )));
    }

    #[test]
    fn action_contract_feedback_requires_patch_after_implementation_needs_edit() {
        let tool_call = ToolCall {
            tool_name: ToolName::plain("shell_command"),
            call_id: "taskspace-action-contract-20-read_file".to_string(),
            payload: ToolPayload::Function {
                arguments: "{\"command\":\"sed -n '1,240p' -- schema.json\"}".to_string(),
            },
        };
        let err = CodexErr::Fatal(
            "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | schema_required_groups=schema.json:properties.statistics requires averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService | target_artifacts=process.py. Return exactly one valid taskspace-action-v1 JSON object.".to_string(),
        );
        let response_input = response_input_for_taskspace_action_tool_error(&tool_call, &err);
        let response_item: ResponseItem = response_input.into();
        let (_, summary) = taskspace_action_contract_tool_output_summary(&response_item)
            .expect("failed action-contract output summarizes");
        let recent = taskspace_action_contract_recent_tool_outputs_item(
            &[response_item],
            Some("implement_solution"),
        )
        .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("failure_kind: implementation_needs_edit"));
        assert!(summary.contains("repair_contract: missing_required_properties=members"));
        assert!(summary.contains("projectStatusDistribution"));
        assert!(summary.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(summary.contains("satisfy it exactly"));
        assert!(text.contains("Next action must be apply_patch"));
        assert!(text.contains("no successful edit"));
        assert!(taskspace_message_hit_implementation_needs_edit(Some(
            &summary
        )));
    }

    #[test]
    fn taskspace_action_contract_run_test_prefixes_bare_pytest_file() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{"command":"pytest test_tax_calc.py -v"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["command"], "pytest tests/test_tax_calc.py -v");
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_run_test_preserves_python_m_pytest_prefix() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{"command":"python -m pytest test_tax_calc.py -v"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(
                    value["command"],
                    "python -m pytest tests/test_tax_calc.py -v"
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_run_test_prefixes_bare_shell_script() {
        let cases = [
            ("./run_pipeline.sh", "bash run_pipeline.sh"),
            (
                "cd /app && ./run_pipeline.sh",
                if cfg!(windows) {
                    "cd /app; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; bash run_pipeline.sh"
                } else {
                    "cd /app && bash run_pipeline.sh"
                },
            ),
        ];
        for (input, expected) in cases {
            let action = parse_taskspace_action_v1(&format!(
                r#"{{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{{"command":{}}}}}"#,
                serde_json::to_string(input).expect("quoted command")
            ))
            .expect("valid json");
            let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
                .expect("policy ok")
                .expect("tool call");

            match call.payload {
                ToolPayload::Function { arguments } => {
                    let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                    assert_eq!(value["command"], expected);
                }
                other => panic!("expected function payload, got {other:?}"),
            }
        }
    }

    #[test]
    fn taskspace_action_contract_run_test_normalizes_powershell_and_chain() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{"command":"python merge_users.py && python -c 'print(\"ok\")'"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                if cfg!(windows) {
                    assert_eq!(
                        value["command"],
                        "python merge_users.py; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; python -c 'print(\"ok\")'"
                    );
                } else {
                    assert_eq!(
                        value["command"],
                        "python merge_users.py && python -c 'print(\"ok\")'"
                    );
                }
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_run_test_normalizes_powershell_or_chain() {
        let command = r#"sqlite3 trunc.db ".tables" 2>&1 || echo 'sqlite3 not available, trying python'; python -c "print('fallback')""#;
        let action = parse_taskspace_action_v1(&format!(
            r#"{{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{{"command":{}}}}}"#,
            serde_json::to_string(command).expect("quoted command")
        ))
        .expect("valid json");
        let call =
            taskspace_action_to_tool_call(&action, &provider_snapshot("inspect_code_context"))
                .expect("policy ok")
                .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                if cfg!(windows) {
                    assert_eq!(
                        value["command"],
                        "sqlite3 trunc.db \".tables\" 2>&1; if ($LASTEXITCODE -ne 0) { echo 'sqlite3 not available, trying python' }; python -c \"print('fallback')\""
                    );
                } else {
                    assert_eq!(value["command"], command);
                }
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_powershell_chain_split_ignores_quoted_ampersands() {
        assert_eq!(
            split_top_level_double_ampersand("python -c 'print(\"a && b\")' && pytest -q"),
            Some(vec![
                "python -c 'print(\"a && b\")'".to_string(),
                "pytest -q".to_string()
            ])
        );
        assert_eq!(split_top_level_double_ampersand("python -c 'a && b'"), None);
    }

    #[test]
    fn taskspace_powershell_or_split_ignores_quoted_pipes() {
        assert_eq!(
            split_top_level_double_pipe_once("python -c 'print(\"a || b\")' || pytest -q"),
            Some((
                "python -c 'print(\"a || b\")'".to_string(),
                "pytest -q".to_string()
            ))
        );
        assert_eq!(split_top_level_double_pipe_once("python -c 'a || b'"), None);
        assert_eq!(
            split_top_level_semicolon_once("echo 'a; b'; python recover.py"),
            Some(("echo 'a; b'".to_string(), "python recover.py".to_string()))
        );
    }

    #[test]
    fn taskspace_action_contract_finish_node_defaults_implementation_to_smoke_test() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"action":"finish_node","outcome":"success"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["node_id"], "node-1");
                assert_eq!(value["next_node_kind"], "smoke_test");
                assert_eq!(value["next_dependency_node_ids"][0], "node-1");
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_finish_node_canonicalizes_command_alias() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"command":"finish_node","node_id":"node-1","result_summary":"Inspected project structure and tests.","next_node_kind":"implement_solution","next_node_title":"Apply fix","next_node_context_summary":"Fix src/tax_calc.py based on tests."},"rationale":"Completed initial inspection"}"#,
        )
        .expect("valid control action");

        assert!(taskspace_action_is_finish_node_control(&action));

        let call =
            taskspace_action_to_tool_call(&action, &provider_snapshot("inspect_code_context"))
                .expect("policy ok")
                .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "finish_node");
                assert_eq!(value["node_id"], "node-1");
                assert_eq!(value["next_node_kind"], "implement_solution");
                assert!(value.get("command").is_none());
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_missing_control_action_is_rejected_before_handler() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"result_summary":"No discriminator."}}"#,
        )
        .expect("valid action shape");
        let err =
            taskspace_action_to_tool_call(&action, &provider_snapshot("inspect_code_context"))
                .expect_err("control actions need a stable discriminator");

        assert_eq!(err, TASKSPACE_CONTROL_ACTION_MISSING_ERROR);
    }

    #[test]
    fn taskspace_action_contract_conflicting_control_action_aliases_are_rejected() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"action":"finish_node","command":"start_task"}}"#,
        )
        .expect("valid action shape");
        let err =
            taskspace_action_to_tool_call(&action, &provider_snapshot("inspect_code_context"))
                .expect_err("conflicting control action aliases must not be guessed");

        assert!(err.starts_with(TASKSPACE_CONTROL_ACTION_CONFLICT_ERROR));
    }

    #[test]
    fn taskspace_action_contract_state_commit_compact_validation_blocker_normalizes() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"action":"state_commit","result_validities":{"result-13":"failed"},"success_criteria":{"criterion-1":"failed"},"blockers":["Local bash failed with E_ACCESSDENIED."],"decisions":["Treat local validation as infrastructure-blocked."]},"rationale":"record local validation result"}"#,
        )
        .expect("valid action shape");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "state_commit");
                assert_eq!(value["schema_version"], "taskspace-state-commit-v1");
                assert_eq!(value["active_node_id"], "node-1");
                assert_eq!(value["blockers"][0]["node_id"], "node-1");
                assert_eq!(
                    value["blockers"][0]["blocker_summary"],
                    "Local bash failed with E_ACCESSDENIED."
                );
                assert_eq!(value["decisions"][0]["decision_kind"], "validation");
                assert_eq!(
                    value["decisions"][0]["decision"],
                    "Treat local validation as infrastructure-blocked."
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_top_level_state_commit_normalizes_to_control_tool() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"state_commit","node_id":"node-1","args":{"result_validities":[{"result_id":"result-13","validity":"invalid","validity_reason":"local validator infrastructure failed"}],"blockers":["PowerShell rejected bash syntax with InvalidEndOfLine."]},"rationale":"record local validation result"}"#,
        )
        .expect("valid action shape");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "taskspace_control");
        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "state_commit");
                assert_eq!(value["schema_version"], "taskspace-state-commit-v1");
                assert_eq!(value["active_node_id"], "node-1");
                assert_eq!(value["blockers"][0]["node_id"], "node-1");
                assert_eq!(
                    value["blockers"][0]["blocker_summary"],
                    "PowerShell rejected bash syntax with InvalidEndOfLine."
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_finish_node_completes_smoke_test_draft_metadata() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"action":"finish_node","next_node_kind":"smoke_test"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["node_id"], "node-1");
                assert_eq!(value["next_node_kind"], "smoke_test");
                assert_eq!(value["next_node_title"], "Run focused validation");
                assert_eq!(value["next_dependency_node_ids"][0], "node-1");
                assert!(
                    value["next_node_context_summary"]
                        .as_str()
                        .expect("context")
                        .contains("focused test command")
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_finish_node_on_validation_node_remains_lifecycle_tool() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-1","args":{"action":"finish_node","result_validities":[{"id":"criterion-1","status":"closed"}],"result":"All tests pass."},"rationale":"close validation node"}"#,
        )
        .expect("valid json");

        assert!(taskspace_action_is_finish_node_control(&action));
        assert!(taskspace_action_final_message(&action).is_none());

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("smoke_test"))
            .expect("policy ok")
            .expect("finish_node must execute taskspace_control");

        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "finish_node");
                assert_eq!(value["node_id"], "node-1");
                assert_eq!(value["result_summary"], "All tests pass.");
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_apply_patch_uses_custom_payload() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** End Patch\n"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        assert!(matches!(call.payload, ToolPayload::Custom { .. }));
        let response_item = response_item_for_taskspace_action_tool_call(&call);
        assert!(matches!(response_item, ResponseItem::CustomToolCall { .. }));
    }

    #[test]
    fn taskspace_action_contract_apply_patch_normalizes_unified_diff() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n--- a/src/tax_calc.py\n+++ b/src/tax_calc.py\n@@ -1 +1 @@\n-old\n+new\n*** End Patch\n"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/tax_calc.py"));
                assert!(!input.contains("--- a/src/tax_calc.py"));
                assert!(!input.contains("+++ b/src/tax_calc.py"));
                assert!(input.contains("@@"));
                assert!(!input.contains("-1 +1"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_native_unified_update_hunk_headers() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** Update File: core/src/session/turn.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n*** End Patch\n"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("range hunk can be normalized")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: core/src/session/turn.rs"));
                assert!(input.contains("@@\n-old\n+new"));
                assert!(!input.contains("@@ -1,1 +1,1 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_unified_hunk_header_from_add_file() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** Add File: recover.py\n@@ -0,0 +1,2 @@\n+line 1\n+line 2\n*** End Patch\n"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("add-file range hunk can be removed")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Add File: recover.py"));
                assert!(input.contains("+line 1\n+line 2"));
                assert!(!input.contains("@@ -0,0 +1,2 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_apply_patch_normalizes_plain_unified_diff() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n--- tax_calc.py\n+++ tax_calc.py\n@@ -1 +1 @@\n-old\n+new\n*** End Patch\n"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/tax_calc.py"));
                assert!(!input.contains("--- tax_calc.py"));
                assert!(!input.contains("+++ tax_calc.py"));
                assert!(!input.contains("-1 +1"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_apply_patch_normalizes_bare_multi_file_unified_diff() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "--- a/collect_data.sh\n+++ b/collect_data.sh\n@@ -1,1 +1,1 @@\n-#!/bin/nonexistent\n+#!/bin/bash\n--- a/generate_report.sh\n+++ b/generate_report.sh\n@@ -1,1 +1,1 @@\n-#!/bin/nonexistent\n+#!/bin/bash\n--- a/run_pipeline.sh\n+++ b/run_pipeline.sh\n@@ -4,3 +4,5 @@\n echo \"Starting the data pipeline...\"\n-\n+# Ensure data output directory exists\n+mkdir -p /data/output\n+\n # Step 1: Collect data"
            },
            "rationale": "fix scripts"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert_eq!(input.matches("*** Update File: ").count(), 3);
                assert!(input.contains("collect_data.sh"));
                assert!(input.contains("generate_report.sh"));
                assert!(input.contains("run_pipeline.sh"));
                assert!(!input.contains("--- a/collect_data.sh"));
                assert!(!input.contains("+++ b/collect_data.sh"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_rejects_existing_file_as_add_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n--- /dev/null\n+++ b/Cargo.toml\n@@ -0,0 +1 @@\n+[workspace]\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("existing files must be patched as updates");
        assert_eq!(err, "apply_patch_existing_file_as_add:Cargo.toml");
    }

    #[test]
    fn taskspace_action_contract_rejects_unanchored_update_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: recover.py\n+import sqlite3\n+print('fixed')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("unanchored updates can insert without replacing broken code");

        assert_eq!(err, "apply_patch_unanchored_update:src/recover.py");
    }

    #[test]
    fn taskspace_action_contract_rejects_non_diff_update_payload() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: organization.json\npython3 -c \"\nimport json\nwith open('organization.json') as f:\n    data = json.load(f)\nwith open('organization.json', 'w') as f:\n    json.dump(data, f, indent=2)\n\"\n*** End Patch"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("command payload is not native apply_patch diff content");

        assert!(err.starts_with("apply_patch_unanchored_update:"));
        assert!(err.ends_with("organization.json"));
    }

    #[test]
    fn taskspace_action_contract_normalizes_whole_python_update_replacement() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: generate_org_json.py\n#!/usr/bin/env python3\nimport csv\nimport json\n\ndef main():\n    print('fixed')\n\nif __name__ == '__main__':\n    main()\n*** End Patch"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("whole-file python replacement can be normalized")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Delete File: src/generate_org_json.py"));
                assert!(input.contains("*** Add File: src/generate_org_json.py"));
                assert!(input.contains("+#!/usr/bin/env python3\n+import csv\n+import json"));
                assert!(!input.contains("*** Update File: src/generate_org_json.py"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_allows_delete_only_update_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: recover.py\n@@\n-print('remove me')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("delete-only patch is anchored by a deleted line")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
    }

    #[test]
    fn taskspace_action_contract_rejects_mixed_native_unified_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: recover.py\n--- a/recover.py\n+++ b/recover.py\n@@ -1,1 +1,1 @@\n-print('bad')\n+print('fixed')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("mixed native/unified patch must be rejected before tool execution");

        assert_eq!(err, "apply_patch_mixed_native_unified:recover.py");
    }

    #[test]
    fn taskspace_action_contract_requires_replacement_for_rework_target_mixed_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: process_csv.py\n--- a/process_csv.py\n+++ b/process_csv.py\n@@ -1,1 +1,1 @@\n-print('bad')\n+print('fixed')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_validation_rework_artifacts = vec!["process_csv.py".to_string()];

        let err = taskspace_action_to_tool_call(&action, &snapshot).expect_err(
            "rework target mixed patch must be converted to replacement-required feedback",
        );

        assert_eq!(err, "apply_patch_replacement_required:process_csv.py");
    }

    #[test]
    fn taskspace_action_contract_rejects_live_wrapped_mixed_native_unified_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: csv2json.py\n--- a/csv2json.py\n+++ b/csv2json.py\n@@ -1,2 +1,2 @@\n #!/usr/bin/env python3\n- \"\"\"CSV to JSON processor - reads CSV files and produces organization.json following schema.json.\"\"\"\n+\"\"\"CSV to JSON processor - reads CSV files and produces organization.json following schema.json.\"\"\"\n*** End Patch"
            },
            "rationale": "fix indentation"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("live mixed native/unified patch must be rejected before tool execution");

        assert_eq!(err, "apply_patch_mixed_native_unified:csv2json.py");
    }

    #[test]
    fn taskspace_action_contract_rejects_live_unwrapped_mixed_native_unified_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: csv2json.py\n--- a/csv2json.py\n+++ b/csv2json.py\n@@ -1,2 +1,2 @@\n- #!/usr/bin/env python3\n+#!/usr/bin/env python3\n \"\"\"CSV to JSON processor - reads CSV files and produces organization.json following schema.json.\"\"\""
            },
            "rationale": "fix indentation"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("live unwrapped mixed patch must be rejected before tool execution");

        assert_eq!(err, "apply_patch_mixed_native_unified:csv2json.py");
    }

    #[test]
    fn taskspace_action_contract_rejects_targetless_unified_headers_without_fake_src_target() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n--- \n+++ \n@@ -1,2 +1,3 @@\n existing_line\n+new_line\n*** End Patch\n"
            },
            "rationale": "patch a file"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("targetless unified headers must be rejected before tool dispatch");

        assert_eq!(
            err,
            "apply_patch_mixed_native_unified:(missing patch target)"
        );
        assert!(!err.contains("src/---"));
    }

    #[test]
    fn bare_file_patch_normalizer_does_not_treat_unified_separator_as_path() {
        let patch = "*** Begin Patch\n--- \n+++ \n@@ -1,2 +1,3 @@\n existing_line\n+new_line\n*** End Patch\n";

        assert!(normalize_taskspace_bare_file_patch(patch).is_none());
    }

    #[test]
    fn taskspace_action_contract_accepts_apply_patch_json_with_single_trailing_quote() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: process.py\nold\n---\nnew\n"
            },
            "rationale": "patch a file"
        })
        .to_string()
            + "\"";

        let action = parse_taskspace_action_v1(&raw).expect("trailing quote tolerated");

        assert_eq!(action.action, "apply_patch");
    }

    #[test]
    fn taskspace_action_contract_normalizes_separator_update_sections() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: process.py\n    'member_ids': [m.strip() for m in p['member_ids'].split(';')],\n---\n    'members': [m.strip() for m in p['member_ids'].split(';')],\n*** Update File: process.py\n    'total_employees': total_employees,\n---\n    'totalEmployees': total_employees,\n"
            },
            "rationale": "fix schema keys"
        })
        .to_string()
            + "\"";
        let action = parse_taskspace_action_v1(&raw).expect("valid json with trailing quote");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("separator update sections normalize")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert_eq!(input.matches("*** Update File: ").count(), 2);
                assert!(input.contains("process.py"));
                assert!(input.contains("@@\n-    'member_ids':"));
                assert!(input.contains("+    'members':"));
                assert!(input.contains("@@\n-    'total_employees':"));
                assert!(input.contains("+    'totalEmployees':"));
                assert!(!input.contains("\n---\n"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_duplicate_unwrapped_update_wrapper() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: process_csv_to_json.py\n\n*** Update File: process_csv_to_json.py\n--- \n+++ \n@@ -38,7 +38,7 @@\n def build_organization():\n     for p in projs:\n-        'member_ids': [m.strip() for m in p['member_ids'].split(';') if m],\n+        'members': [m.strip() for m in p['member_ids'].split(';') if m],\n"
            },
            "rationale": "fix schema members"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("duplicate empty Update File wrapper can be normalized")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert_eq!(input.matches("*** Update File: ").count(), 1);
                assert!(input.contains("process_csv_to_json.py"));
                assert!(input.contains("@@\n def build_organization():"));
                assert!(input.contains("-        'member_ids':"));
                assert!(input.contains("+        'members':"));
                assert!(!input.contains("--- \n+++ "));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_misordered_begin_update_mixed_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: csv2json.py\n*** Begin Patch\n--- a/csv2json.py\n+++ b/csv2json.py\n@@ -1,2 +1,2 @@\n- #!/usr/bin/env python3\n+#!/usr/bin/env python3\n \"\"\"CSV to JSON processor.\"\"\"\n*** End Patch"
            },
            "rationale": "fix indentation"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("misordered wrapper can be normalized")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert_eq!(input.matches("*** Begin Patch").count(), 1);
                assert_eq!(input.matches("*** Update File: ").count(), 1);
                assert!(input.contains("csv2json.py"));
                assert!(input.contains("@@\n- #!/usr/bin/env python3\n+#!/usr/bin/env python3"));
                assert!(!input.contains("--- a/csv2json.py"));
                assert!(!input.contains("+++ b/csv2json.py"));
                assert!(!input.contains("@@ -1,2 +1,2 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_native_placeholder_hunk_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: recover.py\n@@ ... @@\n print('bad')\n+print('fixed')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("anchored placeholder hunk can be normalized")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/recover.py"));
                assert!(input.contains("@@\n print('bad')\n+print('fixed')"));
                assert!(!input.contains("@@ ... @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_live_mixed_placeholder_hunk_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: generate_json.py\n--- a/generate_json.py\n+++ b/generate_json.py\n@@ ... @@\n def build_organization(departments, employees, projects):\n+    for emp in employees:\n+        emp['skills'] = emp['skills'].split(';')\n     # index employees and projects by department_id\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("live mixed placeholder hunk can be normalized")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert!(input.contains("generate_json.py"));
                assert!(input.contains("@@\n def build_organization"));
                assert!(!input.contains("--- a/generate_json.py"));
                assert!(!input.contains("+++ b/generate_json.py"));
                assert!(!input.contains("@@ ... @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_rejects_dash_native_update_header_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n--- Update File: generate_organization.py\n@@ -... +@@ ... @@\n+import json\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("dash native update header must be corrected before tool execution");

        assert_eq!(
            err,
            "apply_patch_native_hunk_header:generate_organization.py"
        );
    }

    #[test]
    fn taskspace_action_contract_allows_anchored_update_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: recover.py\n@@\n- import sqlite3\n+import sqlite3\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("anchored replacement is valid")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
    }

    #[test]
    fn apply_patch_unanchored_update_recovery_does_not_count_as_no_action_retry() {
        let targets = taskspace_unanchored_update_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_unanchored_update:recover.py. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("target parsed");
        let item = build_taskspace_apply_patch_unanchored_update_recovery_item(&targets);
        let text = item_text(item.clone());

        assert_eq!(targets, "recover.py");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER));
        assert!(text.contains("recover.py"));
        assert!(text.contains("`-old` / `+new`"));
        assert!(text.contains("*** Delete File: <path>"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn apply_patch_native_hunk_recovery_does_not_count_as_no_action_retry() {
        let targets = taskspace_native_hunk_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_native_hunk_header:recover.py. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("target parsed");
        let item = build_taskspace_apply_patch_native_hunk_recovery_item(&targets, false);
        let text = item_text(item.clone());

        assert_eq!(targets, "recover.py");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER));
        assert!(text.contains("recover.py"));
        assert!(text.contains("Do not call read_file"));
        assert!(text.contains("`--- Update File:`"));
        assert!(text.contains("*** Delete File: <path>"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn apply_patch_mixed_native_unified_recovery_uses_native_hunk_warning() {
        let targets = taskspace_native_hunk_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_mixed_native_unified:generate_org.py. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("target parsed");
        let item = build_taskspace_apply_patch_native_hunk_recovery_item(&targets, false);
        let text = item_text(item.clone());
        let warning = taskspace_implement_recovery_advisory_warning_message(&item, 7);

        assert_eq!(targets, "generate_org.py");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER));
        assert!(text.contains("generate_org.py"));
        assert!(text.contains("Do not call read_file"));
        assert!(text.contains("`--- Update File:`"));
        assert!(warning.contains("TaskSpaceApplyPatchNativeHunkRecoveryV1"));
        assert!(!warning.contains("TaskSpaceImplementNeedsEditRecoveryV1"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn apply_patch_native_hunk_recovery_forces_complete_replacement_when_target_full_visible() {
        let targets = taskspace_native_hunk_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_mixed_native_unified:process.py. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("target parsed");
        let item = build_taskspace_apply_patch_native_hunk_recovery_item(&targets, true);
        let text = item_text(item.clone());

        assert_eq!(targets, "process.py");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER));
        assert!(text.contains("whole-file native replacement"));
        assert!(text.contains("*** Delete File: <relative/path>"));
        assert!(text.contains("*** Add File: <relative/path>"));
        assert!(text.contains("Do not emit `*** Update File`"));
        assert!(!text.contains("Use native `*** Update File"));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn apply_patch_recovery_hard_stops_after_repeated_same_node_failures() {
        let item = build_taskspace_apply_patch_unanchored_update_recovery_item("recover.py");

        assert!(is_taskspace_apply_patch_recovery_item(&item));
        assert!(!taskspace_apply_patch_recovery_should_hard_stop(&item, 2));
        assert!(taskspace_apply_patch_recovery_should_hard_stop(&item, 3));

        let hard_stop = build_taskspace_apply_patch_recovery_hard_stop_item(&item, 4);
        let text = item_text(hard_stop.clone());

        assert!(text.contains(TASKSPACE_APPLY_PATCH_RECOVERY_HARD_STOP_MARKER));
        assert!(text.contains("reason: repeated_failed_or_malformed_patch"));
        assert!(text.contains("attempt_count: 4"));
        assert!(text.contains("TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1"));
        assert!(is_taskspace_apply_patch_recovery_hard_stop_item(&hard_stop));
        assert!(!is_taskspace_implement_needs_edit_recovery_item(&hard_stop));
        assert!(
            taskspace_special_recovery_warning_message(&hard_stop)
                .contains("TaskSpaceApplyPatchRecoveryHardStopV1")
        );
    }

    #[test]
    fn taskspace_action_contract_record_fact_compacts_to_state_commit() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "taskspace_control",
            "node_id": "node-1",
            "args": {
                "action": "record_fact",
                "fact": "generate_report.sh has an invalid shebang",
            },
            "rationale": "record implementation evidence"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("record_fact compacted")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "taskspace_control");
        let ToolPayload::Function { arguments } = call.payload else {
            panic!("expected function payload");
        };
        let arguments: serde_json::Value =
            serde_json::from_str(&arguments).expect("arguments parse");
        assert_eq!(arguments["action"], "state_commit");
        assert_eq!(arguments["schema_version"], "taskspace-state-commit-v1");
        assert_eq!(arguments["active_node_id"], "node-1");
        assert_eq!(
            arguments["facts"][0]["statement"],
            "generate_report.sh has an invalid shebang"
        );
        assert_eq!(
            arguments["facts"][0]["evidence_refs"][0]["fact_source_id"],
            "fact-source-action-contract-1"
        );
        assert_eq!(
            arguments["fact_sources"][0]["id"],
            "fact-source-action-contract-1"
        );
    }

    #[test]
    fn taskspace_apply_patch_normalizes_bare_file_hunk() {
        let patch = normalize_taskspace_apply_patch(
            "*** Begin Patch\n\
tax_calc.py\n\
\n\
 def calculate_tax(subtotal, region):\n\
-    return round(subtotal * RATES[region], 1)\n\
+    return round(subtotal * RATES[region], 2)\n\
*** End Patch\n",
        );

        assert_eq!(
            patch,
            "*** Begin Patch\n*** Update File: src/tax_calc.py\n@@\ndef calculate_tax(subtotal, region):\n-    return round(subtotal * RATES[region], 1)\n+    return round(subtotal * RATES[region], 2)\n*** End Patch\n"
        );
    }

    #[test]
    fn taskspace_apply_patch_keeps_existing_hunk_header_for_bare_file_patch() {
        let patch = normalize_taskspace_apply_patch(
            "*** Begin Patch\n\
tax_calc.py\n\
@@ -5,7 +5,7 @@ def calculate_tax(subtotal, region):\n\
     if region not in RATES:\n\
         raise ValueError(f\"unsupported region: {region}\")\n\
-    return round(subtotal * RATES[region], 1)\n\
+    return round(subtotal * RATES[region], 2)\n\
\n\
\n\
 def calculate_total(subtotal, region):\n\
*** End Patch\n",
        );

        assert!(patch.starts_with("*** Begin Patch\n*** Update File: src/tax_calc.py\n"));
        assert!(patch.contains("@@ def calculate_tax(subtotal, region):"));
        assert!(!patch.contains("@@ -5,7 +5,7 @@"));
        assert!(patch.ends_with("*** End Patch\n"));
    }

    #[test]
    fn taskspace_apply_patch_strips_common_python_add_file_indent() {
        let patch = normalize_taskspace_apply_patch(
            "*** Begin Patch\n\
*** Add File: generate_org_json.py\n\
+ import csv\n\
+ import json\n\
+ \n\
+ def main():\n\
+     print('ok')\n\
+ \n\
+ main()\n\
*** End Patch\n",
        );

        assert!(patch.contains("*** Add File: generate_org_json.py"));
        assert!(patch.contains("+import csv\n+import json\n+\n+def main():\n+    print('ok')"));
        assert!(!patch.contains("+ import csv"));
        assert!(!patch.contains("+ def main():"));
        assert!(patch.ends_with("*** End Patch\n"));
    }

    #[test]
    fn taskspace_apply_patch_preserves_non_python_add_file_indent() {
        let patch = normalize_taskspace_apply_patch(
            "*** Begin Patch\n\
*** Add File: notes.txt\n\
+ indented text\n\
+   nested text\n\
*** End Patch\n",
        );

        assert!(patch.contains("+ indented text\n+   nested text"));
    }

    #[test]
    fn taskspace_apply_patch_wraps_unwrapped_update_file_patch() {
        let patch = normalize_taskspace_apply_patch(
            "*** Update File: tax_calc.py\n\
@@ -5,7 +5,7 @@ def calculate_tax(subtotal, region):\n\
     if region not in RATES:\n\
         raise ValueError(f\"unsupported region: {region}\")\n\
-    return round(subtotal * RATES[region], 1)\n\
+    return round(subtotal * RATES[region], 2)\n\
\n\
\n\
 def calculate_total(subtotal, region):",
        );

        assert!(patch.starts_with("*** Begin Patch\n*** Update File: src/tax_calc.py\n"));
        assert!(patch.contains("@@ def calculate_tax(subtotal, region):"));
        assert!(!patch.contains("@@ -5,7 +5,7 @@"));
        assert!(patch.ends_with("*** End Patch\n"));
    }

    #[test]
    fn taskspace_apply_patch_strips_unified_file_headers_inside_native_update() {
        let patch = normalize_taskspace_apply_patch(
            "*** Update File: generate.py\n\
--- \n\
+++ \n\
@@ -1,10 +1,35 @@\n\
-import csv\n\
+import csv\n\
+import json\n",
        );

        assert!(patch.starts_with("*** Begin Patch\n*** Update File: "));
        assert!(patch.contains("generate.py\n"));
        assert!(patch.contains("@@\n-import csv\n+import csv\n+import json\n"));
        assert!(!patch.contains("\n--- \n"));
        assert!(!patch.contains("\n+++ \n"));
        assert!(patch.ends_with("*** End Patch\n"));
    }

    #[test]
    fn taskspace_apply_patch_keeps_unified_like_content_after_native_hunk_starts() {
        let patch = normalize_taskspace_apply_patch(
            "*** Update File: generate.py\n\
@@\n\
-old\n\
+--- not a file header\n\
+++ not a file header\n\
+new\n",
        );

        assert!(patch.contains("+--- not a file header\n"));
        assert!(patch.contains("+++ not a file header\n"));
    }

    #[test]
    fn taskspace_finish_current_node_action_builds_control_finish() {
        let action =
            taskspace_finish_current_node_action(Some("node-2"), "Implementation edit succeeded.");

        assert_eq!(action.action, "taskspace_control");
        assert_eq!(action.node_id.as_deref(), Some("node-2"));
        assert_eq!(action.args["action"], "finish_node");
        assert_eq!(action.args["node_id"], "node-2");
        assert!(taskspace_action_is_finish_node_control(&action));
    }

    #[test]
    fn taskspace_finish_inspect_to_implementation_action_builds_next_node() {
        let action = taskspace_finish_inspect_to_implementation_action(Some("node-1"));

        assert_eq!(action.action, "taskspace_control");
        assert_eq!(action.node_id.as_deref(), Some("node-1"));
        assert_eq!(action.args["action"], "finish_node");
        assert_eq!(action.args["node_id"], "node-1");
        assert_eq!(action.args["next_node_kind"], "implement_solution");
        assert_eq!(action.args["next_node_title"], "Apply inspected fix");
        assert_eq!(action.args["next_dependency_node_ids"][0], "node-1");
        assert!(taskspace_action_is_finish_node_control(&action));
    }

    #[test]
    fn taskspace_provider_transport_defaults_deepseek_to_action_contract() {
        assert_eq!(
            taskspace_provider_transport_mode_for_request(true, ""),
            TaskspaceProviderTransportMode::CacheOptimizedActionContract
        );
        assert_eq!(
            taskspace_provider_transport_mode_for_request(true, "native_tools"),
            TaskspaceProviderTransportMode::NativeTools
        );
        assert_eq!(
            taskspace_provider_transport_mode_for_request(false, ""),
            TaskspaceProviderTransportMode::NativeTools
        );
    }

    #[test]
    fn taskspace_static_contract_closes_complete_validation_rework_reads() {
        let instructions = taskspace_static_action_contract_instructions();

        assert!(
            instructions
                .contains("valid while that rework target has not yet been read completely")
        );
        assert!(instructions.contains("validation rework override"));
        assert!(instructions.contains("validation_rework_patch_only_after_target_read"));
        assert!(instructions.contains("validation_rework_closed_action_space_read_disallowed"));
        assert!(
            instructions.contains("read_file/list_files/search/schema inspection are not valid")
        );
        assert!(instructions.contains(
            "emit apply_patch for the named target artifact or taskspace_control block_node only"
        ));
    }

    #[test]
    fn taskspace_action_contract_node_policy_matrix_blocks_cross_node_actions() {
        assert!(taskspace_action_allowed_for_node(
            "read_file",
            Some("inspect_code_context")
        ));
        assert!(taskspace_action_allowed_for_node(
            "search",
            Some("inspect_code_context")
        ));
        assert!(!taskspace_action_allowed_for_node(
            "apply_patch",
            Some("inspect_code_context")
        ));
        assert!(taskspace_action_allowed_for_node(
            "run_test",
            Some("inspect_code_context")
        ));

        assert!(taskspace_action_allowed_for_node(
            "list_files",
            Some("implement_solution")
        ));
        assert!(taskspace_action_allowed_for_node(
            "read_file",
            Some("implement_solution")
        ));
        assert!(taskspace_action_allowed_for_node(
            "search",
            Some("implement_solution")
        ));
        assert!(taskspace_action_allowed_for_node(
            "apply_patch",
            Some("implement_solution")
        ));
        assert!(!taskspace_action_allowed_for_node(
            "run_test",
            Some("implement_solution")
        ));

        for validation_kind in ["smoke_test", "regression_test"] {
            assert!(taskspace_action_allowed_for_node(
                "run_test",
                Some(validation_kind)
            ));
            assert!(!taskspace_action_allowed_for_node(
                "read_file",
                Some(validation_kind)
            ));
            assert!(!taskspace_action_allowed_for_node(
                "search",
                Some(validation_kind)
            ));
            assert!(!taskspace_action_allowed_for_node(
                "apply_patch",
                Some(validation_kind)
            ));
        }

        assert!(taskspace_action_allowed_for_node(
            "final_answer",
            Some("final_synthesis")
        ));
        assert!(taskspace_action_allowed_for_node(
            "taskspace_control",
            Some("final_synthesis")
        ));
        assert!(!taskspace_action_allowed_for_node(
            "read_file",
            Some("final_synthesis")
        ));
        assert!(!taskspace_action_allowed_for_node(
            "run_test",
            Some("final_synthesis")
        ));
        assert!(!taskspace_action_allowed_for_node(
            "apply_patch",
            Some("final_synthesis")
        ));

        assert!(taskspace_action_allowed_for_node("blocked", None));
        assert!(!taskspace_action_allowed_for_node("read_file", None));
    }

    #[test]
    fn taskspace_action_contract_allows_first_validation_rework_target_read() {
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_validation_rework_artifacts =
            vec!["generate_organization.py".to_string()];
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_organization.py"}}"#,
        )
        .expect("valid action");

        assert!(
            taskspace_closed_validation_rework_read_reject_reason(&action, &snapshot, false)
                .is_none()
        );
        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("first target read should be allowed")
            .expect("read_file maps to shell command");

        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn taskspace_action_contract_rejects_closed_validation_rework_target_read() {
        let mut snapshot = provider_snapshot("implement_solution");
        snapshot.current_node_validation_rework_artifacts =
            vec!["generate_organization.py".to_string()];
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_organization.py"}}"#,
        )
        .expect("valid action");

        let reason =
            taskspace_closed_validation_rework_read_reject_reason(&action, &snapshot, true)
                .expect("closed rework read should be rejected");

        assert_eq!(
            reason,
            "validation_rework_closed_action_space_read_disallowed:read_file"
        );
    }

    #[test]
    fn taskspace_finish_node_detects_control_type_alias() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":"node-3","args":{"control_type":"finish_node","result_summary":"Tests passed."}}"#,
        )
        .expect("valid action");

        assert!(taskspace_action_is_finish_node_control(&action));
    }

    #[test]
    fn taskspace_control_create_final_synthesis_is_detected_for_direct_final() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","args":{"action":"create_node","node_kind":"final_synthesis","label":"Final summary"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_action_control_creates_final_synthesis(&action));
        let final_action = taskspace_final_answer_action("done");
        assert_eq!(final_action.action, "final_answer");
        assert_eq!(
            taskspace_action_final_message(&final_action).as_deref(),
            Some("done")
        );
    }

    #[test]
    fn taskspace_control_create_final_synthesis_detects_control_action_alias() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","args":{"control_action":"create_node","node_kind":"final_synthesis","node_label":"Final summary"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_action_control_creates_final_synthesis(&action));
    }

    #[test]
    fn taskspace_control_create_final_synthesis_detects_control_type_alias() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","args":{"control_type":"create_node","node_kind":"final_synthesis","node_label":"Final summary"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_action_control_creates_final_synthesis(&action));
    }

    #[test]
    fn taskspace_control_create_validation_node_detects_slash_kind() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","args":{"action":"create_node","kind":"smoke_test/regression_test","label":"run tests"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_action_control_creates_validation_node(&action));
    }

    #[test]
    fn taskspace_control_create_validation_node_detects_control_action_alias() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","args":{"control_action":"create_node","node_kind":"smoke_test","label":"run tests"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_action_control_creates_validation_node(&action));
    }

    #[test]
    fn taskspace_apply_patch_resolves_unique_short_update_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir");
        fs::write(temp.path().join("src").join("tax_calc.py"), "old").expect("write");

        let resolved = resolve_unique_existing_relative_path_from(temp.path(), "tax_calc.py");

        assert_eq!(resolved.as_deref(), Some("src/tax_calc.py"));
    }

    #[test]
    fn taskspace_apply_patch_strips_b_app_header_for_app_cwd() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("process.py"), "old").expect("write");

        let resolved =
            normalize_taskspace_relative_patch_path_from(temp.path(), "b/app/process.py");

        assert_eq!(resolved, "process.py");
    }

    #[test]
    fn taskspace_apply_patch_keeps_ambiguous_short_update_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("mkdir");
        fs::create_dir_all(temp.path().join("tests")).expect("mkdir");
        fs::write(temp.path().join("src").join("tax_calc.py"), "old").expect("write");
        fs::write(temp.path().join("tests").join("tax_calc.py"), "old").expect("write");

        let resolved = resolve_unique_existing_relative_path_from(temp.path(), "tax_calc.py");

        assert_eq!(resolved, None);
    }

    #[test]
    fn taskspace_apply_patch_falls_back_to_src_for_unresolved_short_path() {
        let temp = tempfile::tempdir().expect("tempdir");

        let resolved = resolve_unique_existing_relative_path_from(temp.path(), "tax_calc.py");

        assert_eq!(resolved.as_deref(), Some("src/tax_calc.py"));
    }

    #[test]
    fn taskspace_apply_patch_resolves_unique_directory_suffix_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src").join("order_pipeline")).expect("mkdir");
        fs::write(
            temp.path()
                .join("src")
                .join("order_pipeline")
                .join("pricing.py"),
            "old",
        )
        .expect("write");

        let resolved =
            resolve_unique_existing_relative_path_from(temp.path(), "order_pipeline/pricing.py");

        assert_eq!(resolved.as_deref(), Some("src/order_pipeline/pricing.py"));
    }

    #[test]
    fn taskspace_apply_patch_keeps_ambiguous_directory_suffix_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("pkg_a").join("order_pipeline")).expect("mkdir");
        fs::create_dir_all(temp.path().join("pkg_b").join("order_pipeline")).expect("mkdir");
        fs::write(
            temp.path()
                .join("pkg_a")
                .join("order_pipeline")
                .join("pricing.py"),
            "old",
        )
        .expect("write");
        fs::write(
            temp.path()
                .join("pkg_b")
                .join("order_pipeline")
                .join("pricing.py"),
            "old",
        )
        .expect("write");

        let resolved =
            resolve_unique_existing_relative_path_from(temp.path(), "order_pipeline/pricing.py");

        assert_eq!(resolved, None);
    }

    #[test]
    fn provider_budget_guidance_is_absent_before_warning_threshold() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                9,
                20,
                0,
                3,
                Some("model_sampling"),
                None,
                None,
                TaskspaceProviderToolVisibility::All,
            )
            .is_none()
        );
    }

    #[test]
    fn provider_budget_guidance_is_advisory_only_at_half_profile_hint() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                10,
                20,
                0,
                3,
                Some("model_sampling"),
                None,
                None,
                TaskspaceProviderToolVisibility::All,
            )
            .is_none()
        );
    }

    #[test]
    fn provider_budget_guidance_does_not_force_thin_implementation_priority() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                15,
                20,
                0,
                3,
                Some("model_sampling"),
                None,
                None,
                TaskspaceProviderToolVisibility::All,
            )
            .is_none()
        );
    }

    #[test]
    fn provider_budget_guidance_does_not_force_implementation_node_patch_order() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                4,
                20,
                0,
                3,
                Some("model_sampling"),
                Some("implement_solution"),
                Some("node-2"),
                TaskspaceProviderToolVisibility::All,
            )
            .is_none()
        );
    }

    #[test]
    fn provider_budget_guidance_does_not_make_inspect_control_only() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                3,
                20,
                3,
                3,
                Some("budget_recovery"),
                Some("inspect_code_context"),
                Some("inspect_code_context"),
                TaskspaceProviderToolVisibility::All,
            )
            .is_none()
        );
    }

    #[test]
    fn provider_budget_guidance_does_not_mark_last_dispatch_as_budget_recovery() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                19,
                20,
                0,
                3,
                Some("model_sampling"),
                None,
                None,
                TaskspaceProviderToolVisibility::None,
            )
            .is_none()
        );
    }

    #[test]
    fn provider_budget_does_not_hide_tools_on_final_non_synthesis_request() {
        assert_eq!(
            taskspace_provider_tool_visibility_for_budget(
                19,
                20,
                0,
                3,
                Some("model_sampling"),
                Some("inspect_code_context")
            ),
            TaskspaceProviderToolVisibility::All
        );
    }

    #[test]
    fn provider_budget_does_not_force_taskspace_control_for_late_inspect_request() {
        assert_eq!(
            taskspace_provider_tool_visibility_for_budget(
                3,
                20,
                3,
                3,
                Some("budget_recovery"),
                Some("inspect_code_context")
            ),
            TaskspaceProviderToolVisibility::All
        );
    }

    #[test]
    fn action_contract_late_inspect_allows_file_reads_past_profile_hint() {
        let mut snapshot = provider_snapshot("inspect_code_context");
        snapshot.request_count = 5;
        snapshot.node_request_count = 5;
        snapshot.max_requests = 14;
        snapshot.max_model_requests_per_node = 4;
        let read_file = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"README.md"}}"#,
        )
        .expect("valid action shape");

        let call = taskspace_action_to_tool_call(&read_file, &snapshot)
            .expect("late inspect should remain advisory-only")
            .expect("read_file should execute shell_command");
        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn action_contract_read_file_uses_host_platform_command() {
        let snapshot = provider_snapshot("inspect_code_context");
        let read_file = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"dir/schema file.json"}}"#,
        )
        .expect("valid action shape");

        let call = taskspace_action_to_tool_call(&read_file, &snapshot)
            .expect("read_file should be valid")
            .expect("read_file should execute shell_command");
        let ToolPayload::Function { arguments } = call.payload else {
            panic!("expected function payload");
        };
        let value: serde_json::Value = serde_json::from_str(&arguments).expect("json payload");
        let command = value["command"].as_str().expect("command");
        if cfg!(windows) {
            assert!(command.starts_with("Get-Content -LiteralPath "));
            assert!(command.contains("-TotalCount 240"));
            assert!(command.contains("TaskSpaceReadFileSummaryV1"));
            assert!(command.contains("eof_reached=$TaskSpaceReadEof"));
        } else {
            assert!(command.starts_with("sed -n "));
            assert!(command.contains("1,240p"));
            assert!(command.contains(" -- "));
            assert!(command.contains("'dir/schema file.json'"));
            assert!(command.contains("&& awk "));
            assert!(command.contains("TaskSpaceReadFileSummaryV1"));
            assert!(command.contains("eof_reached=%s"));
            assert!(!command.contains("Get-Content"));
        }
    }

    #[test]
    fn repeated_blocked_inspect_bootstrap_uses_host_platform_command() {
        let command = taskspace_repeated_blocked_inspect_bootstrap_command();
        assert!(command.contains("*.json"));
        assert!(command.contains("*.csv"));
        assert!(command.contains("*.yaml"));
        assert!(command.contains("*.yml"));
        if cfg!(windows) {
            assert!(command.contains("Get-Content -LiteralPath"));
            assert!(command.contains("Select-Object -First 12"));
        } else {
            assert!(command.contains("head -n 12"));
            assert!(command.contains("sed -n '1,120p' --"));
            assert!(!command.contains("Get-Content"));
            assert!(!command.contains("Select-Object"));
        }
    }

    #[test]
    fn repeated_duplicate_read_search_triggers_inspect_bootstrap() {
        let duplicate_read_search = "TaskSpace blocked this evidence command because inspect node `node-1` already recorded successful read/search evidence `result-1` for the same command `rg --files .`.\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"inspect_duplicate_successful_read_or_search\",\"repeated_blocked_action\":{\"fingerprint\":\"inspect_duplicate_successful_read_or_search:node-1:read:rg --files .\",\"repeat_count\":2,\"same_action_allowed\":false}}";
        assert!(taskspace_repeated_duplicate_read_search_should_bootstrap(
            Some(duplicate_read_search)
        ));

        let duplicate_diagnostic = "TaskSpaceGateRecoveryV1: {\"reason\":\"inspect_duplicate_successful_diagnostic_test\",\"repeated_blocked_action\":{\"repeat_count\":2}}";
        assert!(!taskspace_repeated_duplicate_read_search_should_bootstrap(
            Some(duplicate_diagnostic)
        ));
        assert!(!taskspace_repeated_duplicate_read_search_should_bootstrap(
            None
        ));
    }

    #[test]
    fn missing_fact_source_bootstrap_command_reads_bounded_declared_artifacts() {
        let command = taskspace_missing_fact_source_bootstrap_command(&[
            "employees.csv".to_string(),
            "data/projects.csv".to_string(),
        ]);
        assert!(command.contains("employees.csv"));
        assert!(command.contains("data/projects.csv"));
        assert!(command.contains("TaskSpaceReadFileSummaryV1"));
        if cfg!(windows) {
            assert!(command.contains("Write-Output \"===== employees.csv\""));
            assert!(command.contains("Get-Content -LiteralPath"));
        } else {
            assert!(command.contains("printf"));
            assert!(command.contains("====="));
            assert!(command.contains("%s"));
            assert!(command.contains("sed -n"));
            assert!(
                !command.contains('>'),
                "read-only bootstrap command must not trip shell redirection edit classification: {command}"
            );
        }
    }

    #[test]
    fn provider_budget_does_not_reduce_tools_for_implementation_request() {
        assert_eq!(
            taskspace_provider_tool_visibility_for_budget(
                15,
                20,
                0,
                3,
                Some("model_sampling"),
                Some("implement_solution")
            ),
            TaskspaceProviderToolVisibility::All
        );
    }

    #[test]
    fn provider_budget_does_not_reduce_tools_for_normal_model_sampling() {
        assert_eq!(
            taskspace_provider_tool_visibility_for_budget(
                1,
                20,
                0,
                3,
                Some("model_sampling"),
                Some("inspect_code_context")
            ),
            TaskspaceProviderToolVisibility::All
        );
    }

    #[test]
    fn provider_budget_keeps_tools_for_final_synthesis() {
        assert_eq!(
            taskspace_provider_tool_visibility_for_budget(
                19,
                20,
                0,
                3,
                Some("final_synthesis"),
                Some("final_synthesis")
            ),
            TaskspaceProviderToolVisibility::All
        );
    }

    #[test]
    fn provider_budget_guidance_does_not_override_final_synthesis() {
        assert!(
            build_taskspace_provider_budget_guidance_item(
                19,
                20,
                0,
                3,
                Some("final_synthesis"),
                Some("final_synthesis"),
                Some("final_synthesis"),
                TaskspaceProviderToolVisibility::All,
            )
            .is_none()
        );
    }

    #[test]
    fn active_context_replacement_does_not_inject_provider_budget_guidance() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message("developer", &active_projection),
            message("user", "Keep the direct user requirement."),
        ];

        let prepared = prepare_provider_visible_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(!joined.contains("TaskSpaceProviderBudgetGuidanceV1"));
        assert!(joined.contains("Keep the direct user requirement."));
    }

    #[test]
    fn action_contract_prompt_uses_bounded_user_and_active_projection_only() {
        let old_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: old"
        );
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Old user turn"),
            message("assistant", "Old assistant commentary that must not replay"),
            tool_call("shell_command", "call-1"),
            tool_output_with_call_id("call-1", "Large raw output that must not replay"),
            message("developer", &old_active_projection),
            message("user", "Current user turn"),
            message("developer", &latest_active_projection),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 2);
        assert!(joined.contains("Current user turn"));
        assert!(joined.contains("active_objective: latest"));
        assert!(!joined.contains("Old user turn"));
        assert!(!joined.contains("Old assistant commentary"));
        assert!(!joined.contains("Large raw output"));
        assert!(!joined.contains("active_objective: old"));
    }

    #[test]
    fn action_contract_prompt_includes_recent_post_user_tool_output_summaries() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failing test"),
            message("developer", &latest_active_projection),
            tool_call("shell_command", "call-1"),
            tool_output_with_call_id("call-1", ".\\README.md\n.\\tests\\test_tax_calc.py"),
            tool_call("shell_command", "call-2"),
            tool_output_with_call_id("call-2", "# Tax Calc\nProduct rules: CA tax rate is 7.25%"),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 3);
        assert!(joined.contains("TaskSpaceActionContractRecentToolOutputsV1"));
        assert!(joined.contains(".\\tests\\test_tax_calc.py"));
        assert!(joined.contains("call_id: call-1"));
        assert!(joined.contains("CA tax rate is 7.25%"));
        assert!(joined.contains("call_id: call-2"));
    }

    #[test]
    fn action_contract_recent_outputs_are_scoped_after_latest_active_context() {
        let old_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\ncurrent_node: node-2 kind=implement_solution"
        );
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\ncurrent_node: node-4 kind=implement_solution"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &old_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-9-apply_patch",
                "Success. Updated the following files:\nA generate_org_json.py",
            ),
            message("developer", &latest_active_projection),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 2);
        assert!(joined.contains("current_node: node-4"));
        assert!(!joined.contains("current_node: node-2"));
        assert!(!joined.contains("TaskSpaceActionContractRecentToolOutputsV1"));
        assert!(!joined.contains("Success. Updated the following files"));
        assert!(!joined.contains("A file edit already succeeded"));
    }

    #[test]
    fn action_contract_keeps_current_rework_target_read_across_latest_context() {
        let old_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\ncurrent_node: node-2 kind=implement_solution"
        );
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\n\
current_node: node-4 kind=implement_solution\n\
critical_artifact_evidence:\n\
- validation_rework_target_read result=result-11 artifact=generate_org.py\n\
next_valid_actions:\n\
- apply_patch for generate_org.py"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &old_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-9-apply_patch",
                "Success. Updated the following files:\nA generate_org.py",
            ),
            tool_output_with_call_id(
                "taskspace-action-contract-13-read_file",
                "def build_organization():\n    proj_by_dept.setdefault(p[\"department_id\"], []).append({\"member_ids\": []})\nTaskSpaceReadFileSummaryV1: path=generate_org.py lines_read=84 eof_reached=true max_lines=240",
            ),
            message("developer", &latest_active_projection),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 3);
        assert!(joined.contains("current_node: node-4"));
        assert!(joined.contains("TaskSpaceActionContractRecentToolOutputsV1"));
        assert!(joined.contains("call_id: taskspace-action-contract-13-read_file"));
        assert!(joined.contains("TaskSpaceReadFileSummaryV1: path=generate_org.py"));
        assert!(joined.contains("member_ids"));
        assert!(!joined.contains("call_id: taskspace-action-contract-9-apply_patch"));
        assert!(!joined.contains("Success. Updated the following files"));
        assert!(!joined.contains("A file edit already succeeded"));
    }

    #[test]
    fn action_contract_recent_output_preserves_truncated_read_summary() {
        let summary = "TaskSpaceReadFileSummaryV1: path=process_csv.py lines_read=92 eof_reached=true max_lines=240";
        let response_item = tool_output_with_call_id(
            "taskspace-action-contract-13-read_file",
            &format!(
                "def build_organization():\n{}\n{summary}",
                "x".repeat(TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_CHARS + 256)
            ),
        );

        let recent = taskspace_action_contract_recent_tool_outputs_item(
            &[response_item],
            Some("implement_solution"),
        )
        .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(text.contains("[truncated]"));
        assert!(text.contains("TaskSpaceToolTailSentinelV1"));
        assert!(text.contains(summary));
    }

    #[test]
    fn action_contract_prompt_structures_internal_apply_patch_missing_target_feedback() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failing implementation"),
            message("developer", &latest_active_projection),
            ResponseItem::CustomToolCallOutput {
                call_id: "taskspace-action-contract-7-apply_patch".to_string(),
                name: Some("apply_patch".to_string()),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "apply_patch verification failed: Failed to read file to update W:\\app\\src\\call_stack_counter\\__main__.py: 系统找不到指定的路径。 (os error 3)"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("tool_source: action_contract_internal"));
        assert!(joined.contains("tool_action: apply_patch"));
        assert!(joined.contains("failure_kind: apply_patch_missing_update_target"));
        assert!(joined.contains("target: call_stack_counter/__main__.py"));
        assert!(joined.contains("next_valid_action: emit exactly one apply_patch"));
        assert!(joined.contains("raw_output:"));
        assert!(joined.contains("Failed to read file to update"));
    }

    #[test]
    fn action_contract_prompt_structures_apply_patch_expected_lines_feedback() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failed validation"),
            message("developer", &latest_active_projection),
            ResponseItem::CustomToolCallOutput {
                call_id: "taskspace-action-contract-10-apply_patch".to_string(),
                name: Some("apply_patch".to_string()),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "apply_patch verification failed: Failed to find expected lines in V:\\app\\convert.py:\n    import pandas as pd\nimport pyarrow as pa"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: apply_patch_expected_lines_mismatch"));
        assert!(joined.contains("target: convert.py"));
        assert!(joined.contains("Do not repeat the same context hunk"));
        assert!(joined.contains("*** Delete File: convert.py"));
        assert!(joined.contains("*** Add File: convert.py"));
    }

    #[test]
    fn action_contract_prompt_structures_apply_patch_context_mismatch_feedback() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failed implementation"),
            message("developer", &latest_active_projection),
            ResponseItem::CustomToolCallOutput {
                call_id: "taskspace-action-contract-11-apply_patch".to_string(),
                name: Some("apply_patch".to_string()),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "apply_patch verification failed: Failed to find context '-1,1 +1,1 @@' in S:\\app\\recover.py"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: apply_patch_context_mismatch"));
        assert!(joined.contains("target: recover.py"));
        assert!(joined.contains("native apply_patch `@@` grammar"));
        assert!(joined.contains("Do not repeat the same context hunk"));
    }

    #[test]
    fn action_contract_prompt_structures_apply_patch_unified_hunk_header_feedback() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failed implementation"),
            message("developer", &latest_active_projection),
            ResponseItem::CustomToolCallOutput {
                call_id: "taskspace-action-contract-12-apply_patch".to_string(),
                name: Some("apply_patch".to_string()),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "apply_patch verification failed: invalid hunk at line 3, '@@ -0,0 +1,44 @@' is not a valid hunk header. Valid hunk headers: '*** Add File: {path}', '*** Delete File: {path}', '*** Update File: {path}'"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: apply_patch_unified_hunk_header_in_native_patch"));
        assert!(joined.contains("Do not include unified-diff range headers"));
        assert!(joined.contains("prefix every added file content line with `+`"));
    }

    #[test]
    fn action_contract_prompt_structures_generic_internal_tool_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Run the validation"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-9-run_test".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "pytest failed: 2 failed, 1 passed".to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("tool_action: run_test"));
        assert!(joined.contains("failure_kind: tool_execution_failed"));
        assert!(joined.contains("pytest failed: 2 failed, 1 passed"));
    }

    #[test]
    fn action_contract_prompt_structures_local_validator_coverage_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Run validation"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-9-run_test".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace blocked this validation command because current node `node-3` kind: smoke_test has already discovered local validator `python scripts/validate.py` but requested command `pytest` does not run it.\nTaskSpaceGateRecoveryV1: {\"reason\":\"validation_test_missing_local_validator_coverage\",\"blocking_items\":[\"required_validator:python scripts/validate.py\"]}"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared =
            prepare_taskspace_action_contract_prompt_items_for_node(items, Some("smoke_test"));
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: validation_test_missing_local_validator_coverage"));
        assert!(joined.contains("progress_hint: A previous run_test was rejected"));
        assert!(joined.contains("required_validator: python scripts/validate.py"));
        assert!(joined.contains(
            "next_valid_action: emit exactly one run_test action with command `python scripts/validate.py`"
        ));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_changed_artifact_coverage_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Run validation"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-9-run_test".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace blocked this validation command because current node `node-3` kind: smoke_test must validate the implementation edit, but requested command `python -m jsonschema -i organization.json schema.json` does not execute or reference changed artifact(s): generate_organization.py. Declared output contract targets: organization.json.\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_changed_artifact_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_organization.py` to execute changed artifact `generate_organization.py`\"]}"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared =
            prepare_taskspace_action_contract_prompt_items_for_node(items, Some("smoke_test"));
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: validation_test_missing_changed_artifact_coverage"));
        assert!(joined.contains("progress_hint: A previous run_test was rejected because it did not exercise the changed artifact"));
        assert!(joined.contains("required_command: python generate_organization.py"));
        assert!(joined.contains(
            "next_valid_action: emit exactly one run_test action with command `python generate_organization.py`"
        ));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_output_contract_coverage_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Run validation"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-9-run_test".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace blocked this validation command because current node `node-3` kind: smoke_test has declared output contract artifact(s): organization.json, schema.json, but requested command `python generate_json.py` does not validate those output contract(s).\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_output_contract_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_json.py && python -m jsonschema -i organization.json schema.json` to execute the changed artifact and validate declared output contract(s)\"]}"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared =
            prepare_taskspace_action_contract_prompt_items_for_node(items, Some("smoke_test"));
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: validation_test_missing_output_contract_coverage"));
        assert!(joined.contains("progress_hint: A previous run_test was rejected because it executed code but did not validate declared output contract"));
        assert!(joined.contains(
            "required_command: python generate_json.py && python -m jsonschema -i organization.json schema.json"
        ));
        assert!(joined.contains("Do not finish validation after a generator-only success"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_missing_validation_script_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Run validation"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-9-run_test".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpaceToolInvocationV1:\n\
tool: shell_command\n\
command: python process.py && python -c 'import json; json.load(open(\"organization.json\"))'\n\
raw_output:\n\
python: can't open file '/workspace/process.py': [Errno 2] No such file or directory"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared =
            prepare_taskspace_action_contract_prompt_items_for_node(items, Some("smoke_test"));
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: validation_command_missing_script"));
        assert!(joined.contains("missing_script: process.py"));
        assert!(
            joined.contains("progress_hint: The previous run_test did not start the validator")
        );
        assert!(joined.contains("Stay on the validation node"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_unreviewed_result_blocker() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-17-list_files".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace result `result-8` on node `node-2` is still unreviewed. Before ordinary work or subagent spawn, call taskspace_control(action=state_commit) with result_validities including claims, evidence_refs, changed_artifacts, validator_refs, and remaining_uncertainty so the main agent explicitly accepts, questions, or rejects the node result before relying on it.\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"taskspace_gate_blocked\"}"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("inspect_code_context"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: taskspace_unreviewed_result_blocker"));
        assert!(joined.contains("blocked_result: result-8"));
        assert!(joined.contains("blocked_node: node-2"));
        assert!(joined.contains("progress_hint: A previous ordinary tool was blocked"));
        assert!(joined.contains("taskspace_control action with args.action `state_commit`"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_completed_diagnostic_blocker() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-5-blocked".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-2` cannot be blocked for a missing diagnostic prerequisite because a dependency inspect node already recorded successful diagnostic evidence. Next valid action: apply_patch with the smallest concrete implementation fix from inspected evidence.\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"taskspace_gate_blocked\"}"
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: diagnostic_prerequisite_already_satisfied"));
        assert!(joined.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_implement_finish_missing_edit() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-5-taskspace_control".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-2` cannot be completed without a recorded successful edit action. Execute the edit in this node, or block the node if the edit cannot be made."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: implement_missing_edit_before_finish"));
        assert!(joined.contains("progress_hint: A previous finish_node action was rejected"));
        assert!(joined.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_internal_policy_blocker_rejection() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-7-blocked".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-2` cannot be blocked for an internal node-policy or diagnostic-repeat concern because inspected implementation evidence is already available and no edit has been attempted. Next valid action: apply_patch with the smallest concrete implementation fix from dependency evidence, or block only with a specific external blocker that makes editing impossible."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: internal_policy_blocker_rejected"));
        assert!(joined.contains("progress_hint: A previous block_node action was rejected"));
        assert!(joined.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_missing_source_blocker_rejection() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-6-blocked".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-2` cannot be blocked for missing source visibility because a dependency inspect node already recorded implementation source evidence. Next valid action: retry apply_patch using the inspected source evidence and the failed patch feedback, or block only with a specific external blocker that makes editing impossible."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: missing_source_visibility_blocker_rejected"));
        assert!(joined.contains("progress_hint: A previous block_node action was rejected"));
        assert!(joined.contains("Next action must be apply_patch"));
        assert!(joined.contains("failed patch feedback"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_validation_rework_missing_source_blocker_rejection() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-14-blocked".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-5` cannot be blocked for missing source visibility because dependency evidence already identifies the implementation artifact or validation rework target. Next valid action: retry apply_patch using the inspected source evidence and failed validation feedback; if a failed edit made the visible target context stale or truncated, read_file the same validation rework target once to refresh context, then patch. Block only with a specific external blocker that makes editing impossible."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: missing_source_visibility_blocker_rejected"));
        assert!(joined.contains("progress_hint: A previous block_node action was rejected"));
        assert!(joined.contains("Next action must be apply_patch"));
        assert!(joined.contains("failed validation feedback"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_validator_procedure_blocker_rejection() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-8-blocked".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-6` cannot be blocked for validator procedure or test-command concerns because dependency validation evidence already identifies an implementation failure and this rework node has no successful edit. Next valid action: apply_patch the implementation artifact named by the failed validation evidence, or block only with a specific external blocker that makes editing impossible."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: validator_procedure_blocker_rejected"));
        assert!(joined.contains("progress_hint: A previous block_node action was rejected"));
        assert!(joined.contains("validator procedure or test-command setup"));
        assert!(joined.contains("Next action must be apply_patch"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_structures_editable_validation_failure_blocker_rejection() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Continue the task"),
            message("developer", &latest_active_projection),
            ResponseItem::FunctionCallOutput {
                call_id: "taskspace-action-contract-9-blocked".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "TaskSpace implement_solution node `node-6` cannot be blocked for editable validation failure because dependency validation evidence already identifies a repairable implementation failure and this rework node has no successful edit. Next valid action: apply_patch the implementation artifact named by the failed validation evidence; for top-level Python IndentationError or SyntaxError, patch the whole affected file or block rather than blocking for inspection. Block only with a specific external blocker that makes editing impossible."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(joined.contains("failure_kind: editable_validation_failure_blocker_rejected"));
        assert!(joined.contains("progress_hint: A previous block_node action was rejected"));
        assert!(joined.contains("IndentationError"));
        assert!(joined.contains("patch the whole affected file"));
        assert!(joined.contains("Next action must be apply_patch"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_pre_user_tool_output() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            tool_output_with_call_id("call-1", ".\\old-output.txt"),
            message("user", "Current user turn"),
            message("developer", &latest_active_projection),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 2);
        assert!(!joined.contains(".\\old-output.txt"));
    }

    #[test]
    fn action_contract_prompt_limits_recent_tool_outputs() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Current user turn"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id("call-1", "oldest output"),
            tool_output_with_call_id("call-2", "recent output 2"),
            tool_output_with_call_id("call-3", "recent output 3"),
            tool_output_with_call_id("call-4", "recent output 4"),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 3);
        assert!(!joined.contains("oldest output"));
        assert!(joined.contains("recent output 2"));
        assert!(joined.contains("recent output 3"));
        assert!(joined.contains("recent output 4"));
    }

    #[test]
    fn action_contract_prompt_guides_after_successful_edit_output() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failing test"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "call-1",
                "{\"output\":\"Success. Updated the following files:\\nM src/tax_calc.py\\n\"}",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("A file edit already succeeded"));
        assert!(joined.contains("Next action must be taskspace_control with action=finish_node"));
        assert!(joined.contains("Do not repeat apply_patch"));
    }

    #[test]
    fn action_contract_prompt_guides_patch_after_uncovered_high_signal_finish_rejection() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the failing test"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "call-1",
                "{\"output\":\"Success. Updated the following files:\\nM collect_data.sh\\n\"}",
            ),
            tool_output_with_call_id(
                "call-2",
                "TaskSpace implement_solution node `node-2` cannot be completed while high-signal inspected evidence remains uncovered by successful edits: generate_report.sh (invalid_shebang, result-5). Apply another patch covering those artifact(s), or block the node with the exact reason coverage is impossible.",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("high-signal inspected evidence is still uncovered"));
        assert!(joined.contains("Next action must be apply_patch"));
        assert!(!joined.contains("Next action must be taskspace_control with action=finish_node"));
    }

    #[test]
    fn action_contract_tool_error_is_recordable_recent_output_feedback() {
        let tool_call = ToolCall {
            tool_name: ToolName::plain("taskspace_control"),
            call_id: "taskspace-action-contract-12-taskspace_control".to_string(),
            payload: ToolPayload::Function {
                arguments: "{\"action\":\"finish_node\"}".to_string(),
            },
        };
        let err = CodexErr::Fatal(
            "TaskSpace implement_solution node `node-2` cannot be completed while high-signal inspected evidence remains uncovered by successful edits: generate_report.sh (invalid_shebang, result-5). Apply another patch covering those artifact(s), or block the node with the exact reason coverage is impossible.".to_string(),
        );
        let response_input = response_input_for_taskspace_action_tool_error(&tool_call, &err);
        let response_item: ResponseItem = response_input.into();
        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item], None)
            .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(text.contains("high-signal inspected evidence is still uncovered"));
        assert!(text.contains("generate_report.sh"));
        assert!(text.contains("Next action must be apply_patch"));
    }

    #[test]
    fn action_contract_control_normalizes_block_node_snapshot_fields() {
        let snapshot = provider_snapshot("smoke_test");
        let args = serde_json::json!({
            "action": "block_node",
            "reason": "validation failed with IndentationError"
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("block node args should normalize");

        assert_eq!(normalized["action"], "block_node");
        assert_eq!(normalized["node_id"], "node-1");
        assert_eq!(
            normalized["blocker_summary"],
            "validation failed with IndentationError"
        );
        assert!(normalized.get("reason").is_none());
    }

    #[test]
    fn action_contract_control_rewrites_failed_validation_finish_to_block() {
        let snapshot = provider_snapshot("smoke_test");
        let args = serde_json::json!({
            "action": "finish_node",
            "result_validities": {
                "result-6": "invalid"
            },
            "decisions": [
                "Test failed because merge_users.py used the wrong data path."
            ]
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("failed validation finish should normalize");

        assert_eq!(normalized["action"], "block_node");
        assert_eq!(normalized["node_id"], "node-1");
        assert_eq!(
            normalized["blocker_summary"],
            "Test failed because merge_users.py used the wrong data path."
        );
    }

    #[test]
    fn action_contract_control_normalizes_create_node_aliases() {
        let args = serde_json::json!({
            "action": "create_node",
            "node_kind": "implement_solution",
            "node_title": "Fix failed validation",
            "description": "Patch merge_users.py after smoke_test failure",
            "bind_current": true
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, None)
            .expect("create node args should normalize");

        assert_eq!(normalized["action"], "create_node");
        assert_eq!(normalized["kind"], "implement_solution");
        assert_eq!(normalized["title"], "Fix failed validation");
        assert_eq!(
            normalized["context_summary"],
            "Patch merge_users.py after smoke_test failure"
        );
        assert!(normalized.get("node_kind").is_none());
        assert!(normalized.get("description").is_none());
    }

    #[test]
    fn action_contract_control_defaults_create_node_title_and_bind_when_no_active_node() {
        let mut snapshot = provider_snapshot("unknown");
        snapshot.node_id = None;
        snapshot.node_kind = None;
        let args = serde_json::json!({
            "action": "create_node",
            "kind": "inspect_code_context",
            "label": "Inspect source files"
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("create node args should normalize");

        assert_eq!(normalized["action"], "create_node");
        assert_eq!(normalized["kind"], "inspect_code_context");
        assert_eq!(normalized["title"], "Inspect source files");
        assert!(
            normalized["context_summary"]
                .as_str()
                .expect("context summary")
                .contains("Inspect source files")
        );
        assert_eq!(normalized["bind_current"], true);
    }

    #[test]
    fn action_contract_control_rewrites_bind_node_without_id_to_create_node() {
        let mut snapshot = provider_snapshot("unknown");
        snapshot.node_id = None;
        snapshot.node_kind = None;
        let args = serde_json::json!({
            "action": "bind_node",
            "child_kind": "inspect_code_context",
            "child_name": "inspect_sources"
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("bind node args should normalize to create node");

        assert_eq!(normalized["action"], "create_node");
        assert_eq!(normalized["kind"], "inspect_code_context");
        assert_eq!(normalized["title"], "inspect_sources");
        assert_eq!(normalized["bind_current"], true);
    }

    #[test]
    fn action_contract_prompt_guides_state_commit_after_local_validator_infra_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the pipeline"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-13-run_test",
                "Bash/Service/CreateInstance/E_ACCESSDENIED",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("local validator infrastructure or the host shell failed"));
        assert!(joined.contains("Do not run more bash/PowerShell diagnostic commands"));
        assert!(joined.contains("action=state_commit"));
        assert!(joined.contains("local validator infrastructure failed"));
    }

    #[test]
    fn action_contract_prompt_detects_utf16_garbled_local_validator_infra_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the pipeline"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-13-run_test",
                "B\0a\0s\0h\0/\0S\0e\0r\0v\0i\0c\0e\0/\0C\0r\0e\0a\0t\0e\0I\0n\0s\0t\0a\0n\0c\0e\0/\0E\0_\0A\0C\0C\0E\0S\0S\0D\0E\0N\0I\0E\0D\0",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("local validator infrastructure or the host shell failed"));
        assert!(joined.contains("Do not run more bash/PowerShell diagnostic commands"));
        assert!(joined.contains("action=state_commit"));
    }

    #[test]
    fn action_contract_prompt_guides_block_after_recorded_local_validator_infra_failure() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the pipeline"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-14-run_test",
                "At line:2 char:30\n+ bash -x run_pipeline.sh 2>&1 || echo EXIT_CODE=$?\n+                              ~~\nThe token '||' is not a valid statement separator in this version.\nFullyQualifiedErrorId : InvalidEndOfLine",
            ),
            tool_output_with_call_id(
                "taskspace-action-contract-15-taskspace_control",
                "TaskSpace state_commit auto-123: status=accepted dry_run=false replayed=false accepted_sections=[result_validities,blockers]",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("Local validation already failed"));
        assert!(joined.contains("that failure has been recorded"));
        assert!(joined.contains("Do not run more bash/PowerShell diagnostic commands"));
        assert!(joined.contains("blocked with the exact local validator infrastructure evidence"));
    }

    #[test]
    fn action_contract_prompt_guides_platform_compatible_rework_after_recorded_local_infra() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Merge the data"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-14-run_test",
                "At line:2 char:23\n+ python merge_users.py && python -c \"...\"\n+                       ~~\nThe token '&&' is not a valid statement separator in this version.\nFullyQualifiedErrorId : InvalidEndOfLine",
            ),
            tool_output_with_call_id(
                "taskspace-action-contract-15-taskspace_control",
                "TaskSpace state_commit auto-123: status=accepted dry_run=false replayed=false accepted_sections=[result_validities,local_infra_blocker]",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("current node is implementation rework"));
        assert!(joined.contains("Do not repeat state_commit or block"));
        assert!(joined.contains("platform-compatible syntax"));
        assert!(joined.contains("PowerShell `;`"));
        assert!(!joined.contains("blocked with the exact local validator infrastructure evidence"));
    }

    #[test]
    fn action_contract_prompt_blocks_unrecoverable_local_infra_in_rework_context() {
        let latest_active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: latest"
        );
        let items = vec![
            message("user", "Fix the pipeline"),
            message("developer", &latest_active_projection),
            tool_output_with_call_id(
                "taskspace-action-contract-13-run_test",
                "Tool call failed before producing a result. local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED",
            ),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items_for_node(
            items,
            Some("implement_solution"),
        );
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("host validator service or shell executor was unavailable"));
        assert!(joined.contains("Do not patch code"));
        assert!(joined.contains("blocked with the exact local validator infrastructure evidence"));
        assert!(!joined.contains("platform-compatible syntax"));
    }

    #[test]
    fn action_contract_prompt_keeps_bootstrap_taskspace_profile() {
        let bootstrap_profile = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\nNo TaskSpace task exists yet. Before ordinary tools, call taskspace_control(action=start_task)."
        );
        let items = vec![
            message("developer", "unrelated developer text"),
            message("user", "Fix the failing test"),
            message("developer", &bootstrap_profile),
        ];

        let prepared = prepare_taskspace_action_contract_prompt_items(items);
        let joined = item_texts(&prepared).join("\n");

        assert_eq!(prepared.len(), 2);
        assert!(joined.contains("Fix the failing test"));
        assert!(joined.contains(TASKSPACE_ACTIVE_PROFILE_MARKER));
        assert!(joined.contains("action=start_task"));
        assert!(!joined.contains("unrelated developer text"));
    }

    #[test]
    fn no_action_recovery_item_requires_actionable_taskspace_output() {
        let text = item_text(build_taskspace_no_action_recovery_item(Some(
            "Let me check the environment.",
        )));

        assert!(text.contains(TASKSPACE_NO_ACTION_RECOVERY_MARKER));
        assert!(text.contains("did not produce effective TaskSpace progress"));
        assert!(text.contains("no successful tool result"));
        assert!(text.contains("Do not send commentary-only text"));
        assert!(text.contains("call shell_command with `rg --files` now"));
        assert!(text.contains("finish the inspect node into implement_solution"));
    }

    #[test]
    fn no_action_recovery_preserves_recent_gate_recovery_context() {
        let blocked_output = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_changed_artifact_coverage\",\"next_valid_actions\":[\"run_test with command `python recover_logs.py` to execute changed artifact `recover_logs.py`\"]}";
        let text = item_text(build_taskspace_no_action_recovery_item(Some(
            blocked_output,
        )));

        assert!(text.contains(TASKSPACE_NO_ACTION_RECOVERY_MARKER));
        assert!(text.contains(TASKSPACE_GATE_RECOVERY_MARKER));
        assert!(text.contains("run_test with command `python recover_logs.py`"));
        assert!(text.contains("obey the `next_valid_actions`"));
    }

    #[test]
    fn no_action_recovery_hard_stops_after_advisory_threshold() {
        let item = build_taskspace_no_action_recovery_item(Some("Let me inspect that next."));

        assert!(is_taskspace_no_action_recovery_item(&item));
        assert!(!taskspace_no_action_recovery_should_hard_stop(&item, 3, 3));
        assert!(taskspace_no_action_recovery_should_hard_stop(&item, 4, 3));

        let hard_stop = build_taskspace_no_action_recovery_hard_stop_item(&item, 4, 3);
        let text = item_text(hard_stop.clone());

        assert!(text.contains(TASKSPACE_NO_ACTION_RECOVERY_HARD_STOP_MARKER));
        assert!(text.contains("reason: repeated_no_action_after_recovery_threshold"));
        assert!(text.contains("attempt_count: 4"));
        assert!(text.contains("advisory_threshold: 3"));
        assert!(text.contains("Stop provider sampling for this turn"));
        assert!(is_taskspace_no_action_recovery_hard_stop_item(&hard_stop));
        assert!(!is_taskspace_no_action_recovery_item(&hard_stop));
        assert!(
            taskspace_special_recovery_warning_message(&hard_stop)
                .contains("TaskSpaceNoActionRecoveryHardStopV1")
        );
    }

    #[test]
    fn provider_budget_hard_stop_item_is_terminal_recovery_guidance() {
        let mut snapshot = provider_snapshot("inspect_code_context");
        snapshot.request_count = 8;
        snapshot.max_requests = 8;
        snapshot.node_request_count = 3;
        snapshot.max_model_requests_per_node = 3;
        snapshot.budget_state = "over_profile_hint".to_string();
        let mut state = crate::action_map::ActionMapRuntimeState::default();
        let mut decision =
            state.gate_create_node_budget("map-1", crate::action_map::NodeKind::InspectCodeContext);
        decision.allowed = false;
        decision.reason = "provider_node_request_hard_limit_exceeded".to_string();
        decision.blocking_items = vec![
            "current_node:node-1".to_string(),
            "request_count:8/8".to_string(),
            "node_request_count:3/3".to_string(),
        ];
        decision.next_valid_actions = vec!["stop provider sampling for this turn".to_string()];
        decision.recovery_request_phase = Some("budget_recovery".to_string());
        decision.quality_impact_required = true;

        let item = build_taskspace_provider_budget_hard_stop_item(&snapshot, &decision);
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_PROVIDER_BUDGET_HARD_STOP_MARKER));
        assert!(text.contains("reason: provider_node_request_hard_limit_exceeded"));
        assert!(text.contains("request_count: 8/8"));
        assert!(text.contains("node_request_count: 3/3"));
        assert!(text.contains("Do not send another provider request for this turn"));
        assert!(is_taskspace_provider_budget_hard_stop_item(&item));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(!is_taskspace_implement_needs_edit_recovery_item(&item));
        assert!(
            taskspace_special_recovery_warning_message(&item)
                .contains("current turn will stop without another model request")
        );
    }

    #[test]
    fn provider_budget_limit_reached_detects_rollout_or_node_limit() {
        let mut snapshot = provider_snapshot("inspect_code_context");

        assert!(!taskspace_provider_budget_limit_reached(&snapshot));

        snapshot.request_count = snapshot.max_requests;
        assert!(taskspace_provider_budget_limit_reached(&snapshot));

        snapshot.request_count = 1;
        snapshot.node_request_count = snapshot.max_model_requests_per_node;
        assert!(taskspace_provider_budget_limit_reached(&snapshot));

        snapshot.max_requests = 0;
        snapshot.max_model_requests_per_node = 0;
        assert!(!taskspace_provider_budget_limit_reached(&snapshot));
    }

    #[test]
    fn validation_rework_duplicate_read_hard_stops_after_one_recovery() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-11` and no successful edit has been recorded after that read. Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | target_artifacts=generate_org.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `generate_org.py`\"]}";
        let recovery = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some("validation_rework result-11 artifacts=generate_org.py"),
            None,
        );

        assert!(!taskspace_validation_rework_duplicate_read_should_hard_stop(&recovery, 0));
        assert!(taskspace_validation_rework_duplicate_read_should_hard_stop(
            &recovery, 1
        ));

        let hard_stop =
            build_taskspace_validation_rework_duplicate_read_hard_stop_item(&recovery, 2);
        let text = item_text(hard_stop.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_HARD_STOP_MARKER));
        assert!(text.contains("reason: repeated_validation_rework_duplicate_artifact_read"));
        assert!(text.contains("target_artifact: generate_org.py"));
        assert!(text.contains("previous_read_result: result-11"));
        assert!(text.contains("Stop provider sampling for this turn"));
        assert!(is_taskspace_validation_rework_duplicate_read_hard_stop_item(&hard_stop));
        assert!(!is_taskspace_no_action_recovery_item(&hard_stop));
        assert!(!is_taskspace_implement_needs_edit_recovery_item(&hard_stop));
        assert!(
            taskspace_special_recovery_warning_message(&hard_stop)
                .contains("current turn will stop without another model request")
        );
    }

    #[test]
    fn validation_rework_duplicate_read_complete_context_gets_one_recovery_before_hard_stop() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-11` and no successful edit has been recorded after that read. Result `result-11` is a complete read_file context (TaskSpaceReadFileSummaryV1 eof_reached=true; no additional file lines are hidden). Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made. Validation repair contract: missing_required_properties=id, members, averageDepartmentBudget | target_artifacts=process.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"call apply_patch for `process.py`\",\"use complete read_file result result-11; do not request the full file again\"]}";
        let recovery = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some(
                "validation_rework_target_read result=result-11 artifact=process.py | read_context: complete_read; TaskSpaceReadFileSummaryV1: path=process.py lines_read=97 eof_reached=true max_lines=240",
            ),
            None,
        );

        assert!(!taskspace_validation_rework_duplicate_read_should_hard_stop(&recovery, 0));
        assert!(taskspace_validation_rework_duplicate_read_should_hard_stop(
            &recovery, 1
        ));

        let hard_stop =
            build_taskspace_validation_rework_duplicate_read_hard_stop_item(&recovery, 2);
        let text = item_text(hard_stop);

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_HARD_STOP_MARKER));
        assert!(text.contains("reason: repeated_validation_rework_duplicate_artifact_read"));
        assert!(text.contains("target_artifact: process.py"));
        assert!(text.contains("previous_read_result: result-11"));
        assert!(text.contains("complete read_file context"));
    }

    #[test]
    fn validation_rework_duplicate_read_repeated_gate_hard_stops_immediately() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-11` and no successful edit has been recorded after that read. Use the existing file contents from that result and apply the smallest fix with apply_patch, or return blocked with the exact reason no safe edit can be made.\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"blocking_items\":[\"current_node:node-4:implement_solution\",\"repeated_blocked_action:validation_rework_duplicate_artifact_read|shell_command|read|sed -n 1,240p -- generate_org.py\"],\"next_valid_actions\":[\"call apply_patch for `generate_org.py`\"],\"repeated_blocked_action\":{\"fingerprint\":\"validation_rework_duplicate_artifact_read|shell_command|read|sed -n 1,240p -- generate_org.py\",\"repeat_count\":2,\"same_action_allowed\":false}}";
        let recovery = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some("validation_rework result-11 artifacts=generate_org.py"),
            None,
        );

        assert!(taskspace_validation_rework_duplicate_read_should_hard_stop(
            &recovery, 0
        ));
    }

    #[test]
    fn gate_recovery_message_is_not_treated_as_inspect_bootstrap_gap() {
        let blocked_output = "TaskSpace blocked this diagnostic command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"inspect_duplicate_successful_diagnostic_test\",\"next_valid_actions\":[\"read_file or search for implementation/test evidence\",\"taskspace_control(action=finish_node, node_id=\\\"node-1\\\", next_node_kind=\\\"implement_solution\\\")\"]}";

        assert!(taskspace_message_has_gate_recovery(Some(blocked_output)));
        assert!(taskspace_message_has_gate_recovery_reason(
            Some(blocked_output),
            "inspect_duplicate_successful_diagnostic_test"
        ));
        assert!(!taskspace_message_has_gate_recovery_reason(
            Some("inspect_duplicate_successful_diagnostic_test"),
            "inspect_duplicate_successful_diagnostic_test"
        ));
        assert!(!taskspace_message_has_gate_recovery(Some(
            "Run python -m pytest -q now."
        )));
        let recovery = item_text(build_taskspace_no_action_recovery_item(Some(
            blocked_output,
        )));
        assert!(recovery.contains("inspect_duplicate_successful_diagnostic_test"));
        assert!(recovery.contains("read_file or search"));
        assert!(recovery.contains("obey the `next_valid_actions`"));
    }

    #[test]
    fn duplicate_diagnostic_recovery_keeps_inspect_on_source_evidence() {
        let blocked_output = "TaskSpace blocked this diagnostic command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"inspect_duplicate_successful_diagnostic_test\",\"next_valid_actions\":[\"read_file or search for implementation/test evidence\"]}";
        let item = build_taskspace_duplicate_diagnostic_inspect_recovery_item(Some(blocked_output));
        let text = item_text(item.clone());

        assert!(text.contains("TaskSpaceDuplicateDiagnosticInspectRecoveryV1"));
        assert!(text.contains("diagnostic command has already completed successfully"));
        assert!(text.contains("Do not rerun the blocked diagnostic command"));
        assert!(text.contains("read_file or search"));
        assert!(text.contains("Do not return blocked"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn duplicate_read_search_recovery_pushes_inspect_transition() {
        let blocked_output = "TaskSpace blocked this evidence command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"inspect_duplicate_successful_read_or_search\",\"next_valid_actions\":[\"taskspace_control(action=finish_node, node_id=\\\"node-1\\\", next_node_kind=\\\"implement_solution\\\")\"]}";

        assert!(taskspace_message_has_inspect_duplicate_successful_evidence(
            Some(blocked_output)
        ));
        assert_eq!(
            taskspace_inspect_duplicate_successful_evidence_trigger(Some(blocked_output)),
            "inspect_duplicate_read_search_gate_recovery"
        );
        let item = build_taskspace_duplicate_inspect_successful_evidence_recovery_item(Some(
            blocked_output,
        ));
        let text = item_text(item.clone());

        assert!(text.contains("TaskSpaceDuplicateReadSearchInspectRecoveryV1"));
        assert!(text.contains("Re-reading the same artifact is not new evidence"));
        assert!(text.contains("finish_node"));
        assert!(text.contains("implement_solution"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn extracts_gate_recovery_from_blocked_tool_output() {
        let item = tool_output(
            "TaskSpace blocked this validation command.\nTaskSpaceGateRecoveryV1: {\"next_valid_actions\":[\"run python recover_logs.py\"]}",
        );
        let extracted = taskspace_gate_recovery_from_response_item(&item)
            .expect("gate recovery should be extracted from function output");

        assert!(extracted.contains(TASKSPACE_GATE_RECOVERY_MARKER));
        assert!(extracted.contains("run python recover_logs.py"));
    }

    #[test]
    fn forced_transition_recovery_item_does_not_count_as_no_action_retry() {
        let item = build_taskspace_forced_inspect_transition_recovery_item();
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_FORCED_INSPECT_TRANSITION_MARKER));
        assert!(text.contains("Current required behavior"));
        assert!(!is_taskspace_no_action_recovery_item(&item));

        let item = build_taskspace_forced_validation_closeout_recovery_item();
        let text = item_text(item.clone());
        assert!(text.contains(TASKSPACE_FORCED_VALIDATION_CLOSEOUT_MARKER));
        assert!(text.contains("final_answer"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn implement_needs_edit_recovery_has_own_advisory_marker() {
        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "result-5: #!/bin/nonexistent",
        ));
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(text.contains("#!/bin/nonexistent"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn edit_failure_recovery_preserves_failed_tool_feedback() {
        let item = build_taskspace_edit_failure_recovery_item(
            Some("result-10: apply_patch verification failed: invalid hunk"),
            Some("result-3 artifacts=src/call_stack_counter.py: format_depth returns depth"),
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(text.contains("apply_patch verification failed"));
        assert!(text.contains("src/call_stack_counter.py"));
        assert!(text.contains("Treat the tool result exactly like standard mode feedback"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn patch_intent_format_recovery_has_own_advisory_marker() {
        let item = build_taskspace_patch_intent_format_recovery_item(None, None);
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_PATCH_INTENT_FORMAT_MARKER));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn validation_infra_recovery_does_not_count_as_no_action_retry() {
        let item = build_taskspace_validation_infra_recovery_item();
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER));
        assert!(text.contains("local validator infrastructure"));
        assert!(text.contains("Emit exactly one blocked"));
        assert!(text.contains("Bash/Service/CreateInstance/E_ACCESSDENIED"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn validation_needs_test_recovery_blocks_discovery_loop() {
        let last = "TaskSpaceActionV1 rejected: node_policy_violation:smoke_test:list_files. Return exactly one valid taskspace-action-v1 JSON object.";
        let item = build_taskspace_validation_needs_test_recovery_item(Some(last));
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_NEEDS_TEST_MARKER));
        assert!(text.contains("Validation nodes must execute validation"));
        assert!(text.contains("Emit exactly one run_test action now"));
        assert!(text.contains("python scripts/validate.py"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(taskspace_message_hit_validation_needs_test(Some(last)));
    }

    #[test]
    fn validation_changed_artifact_coverage_recovery_preserves_next_action() {
        let last = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_changed_artifact_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_organization.py` to execute changed artifact `generate_organization.py`\"]}";
        let item = build_taskspace_validation_needs_test_recovery_item(Some(last));
        let text = item_text(item.clone());

        assert!(taskspace_message_hit_validation_needs_test(Some(last)));
        assert!(text.contains(TASKSPACE_VALIDATION_NEEDS_TEST_MARKER));
        assert!(text.contains(TASKSPACE_GATE_RECOVERY_MARKER));
        assert!(text.contains("validation_test_missing_changed_artifact_coverage"));
        assert!(text.contains("python generate_organization.py"));
        assert!(text.contains("obey the `next_valid_actions`"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn validation_output_contract_coverage_recovery_preserves_next_action() {
        let last = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_output_contract_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_json.py && python -m jsonschema -i organization.json schema.json` to execute the changed artifact and validate declared output contract(s)\"]}";
        let item = build_taskspace_validation_needs_test_recovery_item(Some(last));
        let text = item_text(item.clone());

        assert!(taskspace_message_hit_validation_needs_test(Some(last)));
        assert!(text.contains(TASKSPACE_VALIDATION_NEEDS_TEST_MARKER));
        assert!(text.contains(TASKSPACE_GATE_RECOVERY_MARKER));
        assert!(text.contains("validation_test_missing_output_contract_coverage"));
        assert!(text.contains("python generate_json.py && python -m jsonschema"));
        assert!(text.contains("obey the `next_valid_actions`"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn validation_required_command_bridge_extracts_gate_next_action() {
        let changed = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_changed_artifact_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_organization.py` to execute changed artifact `generate_organization.py`\"]}";
        let output_contract = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_output_contract_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_organization.py && python -m jsonschema -i organization.json schema.json` to execute the changed artifact and validate declared output contract(s)\"]}";

        assert_eq!(
            taskspace_validation_required_command_from_gate_recovery(Some(changed)).as_deref(),
            Some("python generate_organization.py")
        );
        assert_eq!(
            taskspace_validation_required_command_from_gate_recovery(Some(output_contract))
                .as_deref(),
            Some(
                "python generate_organization.py && python -m jsonschema -i organization.json schema.json"
            )
        );
    }

    #[test]
    fn validation_required_command_bridge_rejects_non_gate_failures() {
        let generic_failure = "TaskSpaceToolFeedbackV1:\n\
tool_action: run_test\n\
tool_result: failed\n\
raw_output:\npytest: command not found";
        let unrelated_gate = "TaskSpaceGateRecoveryV1: {\"reason\":\"validation_test_missing_local_validator_coverage\",\"next_valid_actions\":[\"run_test with command `python scripts/validate.py`\"]}";

        assert_eq!(
            taskspace_validation_required_command_from_gate_recovery(Some(generic_failure)),
            None
        );
        assert_eq!(
            taskspace_validation_required_command_from_gate_recovery(Some(unrelated_gate)),
            None
        );
    }

    #[test]
    fn validation_required_command_bridge_chains_output_contract_gate() {
        let output = "TaskSpace blocked this validation command because current node `node-3` kind: smoke_test has declared output contract artifact(s): organization.json, schema.json, but requested command `python transform.py` does not validate those output contract(s).\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_output_contract_coverage\",\"next_valid_actions\":[\"run_test with command `python transform.py && python -m jsonschema -i organization.json schema.json` to execute the changed artifact and validate declared output contract(s)\"]}";

        assert_eq!(
            taskspace_validation_chained_required_command("python transform.py", output).as_deref(),
            Some("python transform.py && python -m jsonschema -i organization.json schema.json")
        );
        assert_eq!(
            taskspace_validation_chained_required_command(
                "python transform.py && python -m jsonschema -i organization.json schema.json",
                output,
            ),
            None
        );
    }

    #[test]
    fn apply_patch_format_recovery_does_not_count_as_no_action_retry() {
        let targets = taskspace_existing_file_add_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_existing_file_as_add:generate_report.sh. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("targets parsed");
        let item = build_taskspace_apply_patch_format_recovery_item(&targets);
        let text = item_text(item.clone());

        assert_eq!(targets, "generate_report.sh");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_FORMAT_MARKER));
        assert!(text.contains("generate_report.sh"));
        assert!(text.contains("*** Update File: <path>"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(!is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn no_action_recovery_cap_allows_inspect_and_implement_retries() {
        assert_eq!(
            taskspace_no_action_recovery_cap_for_node_kind("inspect_code_context"),
            3
        );
        assert_eq!(
            taskspace_no_action_recovery_cap_for_node_kind("implement_solution"),
            3
        );
        assert_eq!(
            taskspace_no_action_recovery_cap_for_node_kind("smoke_test"),
            1
        );
    }

    #[test]
    fn budget_pressure_follow_up_intent_is_disabled_for_implementation_node() {
        assert!(!taskspace_budget_pressure_follow_up_intent(
            32,
            40,
            Some("implement_solution"),
            Some("Let me check the environment and verify the exact problems."),
        ));
        assert!(!taskspace_budget_pressure_follow_up_intent(
            19,
            40,
            Some("implement_solution"),
            Some("Let me check the environment."),
        ));
    }

    #[test]
    fn provider_response_actionability_classifies_final_gate_rejection_as_recovery() {
        let classification = classify_taskspace_provider_response_actionability(
            true, false, true, false, true, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::FinalRejected
        );
        assert!(classification.needs_recovery());
        assert_eq!(classification.as_str(), "final_rejected");
    }

    #[test]
    fn provider_response_actionability_classifies_no_action_follow_up() {
        let classification = classify_taskspace_provider_response_actionability(
            true, false, true, false, false, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::NoActionFollowUp
        );
        assert!(classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_treats_empty_active_node_response_as_recovery() {
        let needs_follow_up = taskspace_active_node_empty_response_requires_follow_up(
            Some("inspect_code_context"),
            false,
            false,
            false,
        );
        let classification = classify_taskspace_provider_response_actionability(
            needs_follow_up,
            false,
            false,
            false,
            false,
            false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::EmptyFollowUp
        );
        assert!(classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_allows_empty_response_without_active_node_final_candidate() {
        let needs_follow_up =
            taskspace_active_node_empty_response_requires_follow_up(None, false, false, false);
        let classification = classify_taskspace_provider_response_actionability(
            needs_follow_up,
            false,
            false,
            false,
            false,
            false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::FinalCandidate
        );
        assert!(!classification.needs_recovery());
    }

    #[test]
    fn budget_pressure_follow_up_intent_does_not_require_recovery() {
        assert!(!taskspace_budget_pressure_follow_up_intent(
            30,
            40,
            Some("inspect_code_context"),
            Some("Now I have enough context. Let me run the pipeline.")
        ));
        assert!(!taskspace_budget_pressure_follow_up_intent(
            19,
            40,
            Some("inspect_code_context"),
            Some("Let me run the pipeline.")
        ));
        assert!(!taskspace_budget_pressure_follow_up_intent(
            19,
            40,
            Some("implement_solution"),
            Some("Let me run the pipeline.")
        ));
    }

    #[test]
    fn budget_pressure_follow_up_intent_ignores_chinese_test_intent() {
        assert!(!taskspace_budget_pressure_follow_up_intent(
            3,
            6,
            Some("inspect_code_context"),
            Some(
                "\u{6211}\u{5df2}\u{7ecf}\u{770b}\u{5230}\u{4e86}\u{95ee}\u{9898}\u{6240}\u{5728}\u{3002}\u{5148}\u{8dd1}\u{6d4b}\u{8bd5}\u{786e}\u{8ba4}\u{5f53}\u{524d}\u{5931}\u{8d25}\u{3002}",
            )
        ));
    }

    #[test]
    fn budget_pressure_silent_action_does_not_force_transition() {
        assert!(
            !taskspace_budget_pressure_silent_action_requires_transition(
                4,
                8,
                Some("inspect_code_context"),
                false,
                true
            )
        );
        assert!(
            !taskspace_budget_pressure_silent_action_requires_transition(
                3,
                8,
                Some("inspect_code_context"),
                false,
                true
            )
        );
        assert!(
            !taskspace_budget_pressure_silent_action_requires_transition(
                4,
                8,
                Some("implement_solution"),
                false,
                true
            )
        );
        assert!(
            !taskspace_budget_pressure_silent_action_requires_transition(
                6,
                8,
                Some("implement_solution"),
                false,
                true
            )
        );
    }

    #[test]
    fn provider_response_actionability_keeps_actionable_response_out_of_recovery() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, false, false, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::Actionable
        );
        assert!(!classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_treats_gate_recovery_as_recovery() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, true, false, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::NoActionFollowUp
        );
        assert!(classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_treats_profile_hint_overrun_as_actionable() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, false, false, true,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::Actionable
        );
        assert!(!classification.needs_recovery());
        assert_eq!(classification.as_str(), "actionable");
    }

    #[test]
    fn active_context_replacement_is_noop_without_active_projection() {
        let items = vec![
            message(
                "developer",
                "<skills_instructions>stable skills surface</skills_instructions>",
            ),
            message(
                "developer",
                "TaskSpace mode is now active; call taskspace_control(...)",
            ),
            message("user", "Preserve this user requirement."),
        ];

        let prepared = prepare_provider_visible_prompt_items(items.clone());

        assert_eq!(prepared, items);
    }

    #[test]
    fn active_context_replacement_removes_legacy_taskspace_history() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message(
                "developer",
                "<skills_instructions>stable skills surface</skills_instructions>",
            ),
            message(
                "developer",
                "TaskSpace mode is now active; call taskspace_control(...)",
            ),
            message("developer", &active_projection),
            message(
                "developer",
                "TaskSpace ContextProjectionV1 shadow update.\nContextProjectionV1 shadow (not active replacement):",
            ),
            tool_output("ActionMap node state from taskspace_control(action=create_node)"),
            message("user", "Preserve this user requirement."),
        ];

        let prepared = prepare_provider_visible_prompt_items(items);
        let texts = item_texts(&prepared);
        let joined = texts.join("\n");

        assert_eq!(prepared.len(), 3);
        assert!(
            joined.contains("<skills_instructions>stable skills surface</skills_instructions>")
        );
        assert!(joined.contains(TASKSPACE_ACTIVE_PROFILE_MARKER));
        assert!(joined.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER));
        assert!(joined.contains("Preserve this user requirement."));
        assert!(!joined.contains("TaskSpace mode is now active"));
        assert!(!joined.contains(TASKSPACE_SHADOW_PROJECTION_MARKER));
        assert!(!joined.contains("ActionMap node state"));
    }

    #[test]
    fn active_context_replacement_removes_taskspace_control_calls() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message("developer", &active_projection),
            tool_call("taskspace_control", "call-1"),
            message("user", "Keep the current task constraints."),
        ];

        let prepared = prepare_provider_visible_prompt_items(items);

        assert_eq!(prepared.len(), 2);
        assert!(
            prepared
                .iter()
                .all(|item| !matches!(item, ResponseItem::FunctionCall { .. }))
        );
    }

    #[test]
    fn active_context_replacement_omits_paired_call_when_tool_output_is_replaced() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message("developer", &active_projection),
            tool_call("shell_command", "blocked-call"),
            tool_output_with_call_id(
                "blocked-call",
                "TaskSpace blocked this tool call.\nTaskSpaceGateRecoveryV1: retry with inspect_code_context",
            ),
            message("user", "Keep the direct user requirement."),
        ];

        let composition = compose_provider_visible_history(items);
        let texts = item_texts(&composition.items);
        let joined = texts.join("\n");

        assert!(!composition.items.iter().any(|item| matches!(
            item,
            ResponseItem::FunctionCall { call_id, .. }
                if call_id == "blocked-call"
        )));
        assert!(!joined.contains("TaskSpaceGateRecoveryV1"));
        assert!(joined.contains("Keep the direct user requirement."));
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Omit(
                "paired_tool_call_or_output_replaced_by_active_projection"
            )
        );
        assert_eq!(
            composition.decisions[2].action,
            ProviderVisibleHistoryAction::Omit("legacy_taskspace_tool_output_replaced")
        );
    }

    #[test]
    fn active_context_replacement_omits_paired_output_when_tool_call_is_replaced() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message("developer", &active_projection),
            tool_call("taskspace_control", "control-call"),
            tool_output_with_call_id("control-call", r#"{"status":"ok"}"#),
            message("user", "Keep the direct user requirement."),
        ];

        let composition = compose_provider_visible_history(items);
        let texts = item_texts(&composition.items);
        let joined = texts.join("\n");

        assert!(!composition.items.iter().any(|item| {
            response_item_tool_call_id(item).is_some_and(|call_id| call_id == "control-call")
        }));
        assert!(!joined.contains(r#"{"status":"ok"}"#));
        assert!(joined.contains("Keep the direct user requirement."));
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Omit("taskspace_control_call_not_provider_surface")
        );
        assert_eq!(
            composition.decisions[2].action,
            ProviderVisibleHistoryAction::Omit(
                "paired_tool_call_or_output_replaced_by_active_projection"
            )
        );
    }

    #[test]
    fn active_context_replacement_preserves_user_text_that_mentions_taskspace() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message("developer", &active_projection),
            message(
                "user",
                "The bug report mentions TaskSpace, but this is user evidence.",
            ),
            tool_output("ActionMap node state from taskspace_control(action=create_node)"),
        ];

        let composition = compose_provider_visible_history(items);
        let texts = item_texts(&composition.items);
        let joined = texts.join("\n");

        assert!(joined.contains("this is user evidence"));
        assert!(!joined.contains("ActionMap node state"));
        assert_eq!(
            composition.decisions[1].category,
            ProviderVisibleItemCategory::ProtectedUserInput
        );
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Include
        );
    }

    #[test]
    fn active_context_replacement_omits_large_raw_tool_output() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let large_raw_output = "x".repeat(TASKSPACE_ACTIVE_MAX_RAW_TOOL_OUTPUT_CHARS + 1);
        let items = vec![
            message("developer", &active_projection),
            tool_output(&large_raw_output),
            message("user", "Keep the direct user requirement."),
        ];

        let composition = compose_provider_visible_history(items);
        let texts = item_texts(&composition.items);
        let joined = texts.join("\n");

        assert!(!joined.contains(&large_raw_output));
        assert!(joined.contains("Keep the direct user requirement."));
        assert_eq!(
            composition.decisions[1].category,
            ProviderVisibleItemCategory::LargeRawToolOutput
        );
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Omit("large_raw_tool_output_requires_output_reference")
        );
    }

    #[test]
    fn active_context_replacement_keeps_output_reference_payloads() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let referenced_output = format!("OutputReferenceV1 output-ref://sha256/{}", "a".repeat(64));
        let items = vec![
            message("developer", &active_projection),
            tool_output(&referenced_output),
        ];

        let prepared = prepare_provider_visible_prompt_items(items);
        let texts = item_texts(&prepared);

        assert!(texts.join("\n").contains("OutputReferenceV1"));
    }
}

#[derive(Debug)]
struct SamplingRequestResult {
    needs_follow_up: bool,
    last_agent_message: Option<String>,
    taskspace_no_action_recovery_item: Option<ResponseItem>,
}

/// Ephemeral per-response state for streaming a single proposed plan.
/// This is intentionally not persisted or stored in session/state since it
/// only exists while a response is actively streaming. The final plan text
/// is extracted from the completed assistant message.
/// Tracks a single proposed plan item across a streaming response.
struct ProposedPlanItemState {
    item_id: String,
    started: bool,
    completed: bool,
}

/// Aggregated state used only while streaming a plan-mode response.
/// Includes per-item parsers, deferred agent message bookkeeping, and the plan item lifecycle.
struct PlanModeStreamState {
    /// Agent message items started by the model but deferred until we see non-plan text.
    pending_agent_message_items: HashMap<String, TurnItem>,
    /// Agent message items whose start notification has been emitted.
    started_agent_message_items: HashSet<String>,
    /// Leading whitespace buffered until we see non-whitespace text for an item.
    leading_whitespace_by_item: HashMap<String, String>,
    /// Tracks plan item lifecycle while streaming plan output.
    plan_item_state: ProposedPlanItemState,
}

impl PlanModeStreamState {
    fn new(turn_id: &str) -> Self {
        Self {
            pending_agent_message_items: HashMap::new(),
            started_agent_message_items: HashSet::new(),
            leading_whitespace_by_item: HashMap::new(),
            plan_item_state: ProposedPlanItemState::new(turn_id),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct AssistantMessageStreamParsers {
    plan_mode: bool,
    parsers_by_item: HashMap<String, AssistantTextStreamParser>,
}

type ParsedAssistantTextDelta = AssistantTextChunk;

impl AssistantMessageStreamParsers {
    pub(super) fn new(plan_mode: bool) -> Self {
        Self {
            plan_mode,
            parsers_by_item: HashMap::new(),
        }
    }

    fn parser_mut(&mut self, item_id: &str) -> &mut AssistantTextStreamParser {
        let plan_mode = self.plan_mode;
        self.parsers_by_item
            .entry(item_id.to_string())
            .or_insert_with(|| AssistantTextStreamParser::new(plan_mode))
    }

    pub(super) fn seed_item_text(&mut self, item_id: &str, text: &str) -> ParsedAssistantTextDelta {
        if text.is_empty() {
            return ParsedAssistantTextDelta::default();
        }
        self.parser_mut(item_id).push_str(text)
    }

    pub(super) fn parse_delta(&mut self, item_id: &str, delta: &str) -> ParsedAssistantTextDelta {
        self.parser_mut(item_id).push_str(delta)
    }

    pub(super) fn finish_item(&mut self, item_id: &str) -> ParsedAssistantTextDelta {
        let Some(mut parser) = self.parsers_by_item.remove(item_id) else {
            return ParsedAssistantTextDelta::default();
        };
        parser.finish()
    }

    fn drain_finished(&mut self) -> Vec<(String, ParsedAssistantTextDelta)> {
        let parsers_by_item = std::mem::take(&mut self.parsers_by_item);
        parsers_by_item
            .into_iter()
            .map(|(item_id, mut parser)| (item_id, parser.finish()))
            .collect()
    }
}

impl ProposedPlanItemState {
    fn new(turn_id: &str) -> Self {
        Self {
            item_id: format!("{turn_id}-plan"),
            started: false,
            completed: false,
        }
    }

    async fn start(&mut self, sess: &Session, turn_context: &TurnContext) {
        if self.started || self.completed {
            return;
        }
        self.started = true;
        let item = TurnItem::Plan(PlanItem {
            id: self.item_id.clone(),
            text: String::new(),
        });
        sess.emit_turn_item_started(turn_context, &item).await;
    }

    async fn push_delta(&mut self, sess: &Session, turn_context: &TurnContext, delta: &str) {
        if self.completed {
            return;
        }
        if delta.is_empty() {
            return;
        }
        let event = PlanDeltaEvent {
            thread_id: sess.conversation_id.to_string(),
            turn_id: turn_context.sub_id.clone(),
            item_id: self.item_id.clone(),
            delta: delta.to_string(),
        };
        sess.send_event(turn_context, EventMsg::PlanDelta(event))
            .await;
    }

    async fn complete_with_text(
        &mut self,
        sess: &Session,
        turn_context: &TurnContext,
        text: String,
    ) {
        if self.completed || !self.started {
            return;
        }
        self.completed = true;
        let item = TurnItem::Plan(PlanItem {
            id: self.item_id.clone(),
            text,
        });
        sess.emit_turn_item_completed(turn_context, item).await;
    }
}

/// In plan mode we defer agent message starts until the parser emits non-plan
/// text. The parser buffers each line until it can rule out a tag prefix, so
/// plan-only outputs never show up as empty assistant messages.
async fn maybe_emit_pending_agent_message_start(
    sess: &Session,
    turn_context: &TurnContext,
    state: &mut PlanModeStreamState,
    item_id: &str,
) {
    if state.started_agent_message_items.contains(item_id) {
        return;
    }
    if let Some(item) = state.pending_agent_message_items.remove(item_id) {
        sess.emit_turn_item_started(turn_context, &item).await;
        state
            .started_agent_message_items
            .insert(item_id.to_string());
    }
}

/// Agent messages are text-only today; concatenate all text entries.
fn agent_message_text(item: &codex_protocol::items::AgentMessageItem) -> String {
    item.content
        .iter()
        .map(|entry| match entry {
            codex_protocol::items::AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect()
}

pub(super) fn realtime_text_for_event(msg: &EventMsg) -> Option<String> {
    match msg {
        EventMsg::AgentMessage(event) => Some(event.message.clone()),
        EventMsg::ItemCompleted(event) => match &event.item {
            TurnItem::AgentMessage(item) => Some(agent_message_text(item)),
            _ => None,
        },
        EventMsg::Error(_)
        | EventMsg::Warning(_)
        | EventMsg::GuardianWarning(_)
        | EventMsg::RealtimeConversationStarted(_)
        | EventMsg::RealtimeConversationSdp(_)
        | EventMsg::RealtimeConversationRealtime(_)
        | EventMsg::RealtimeConversationClosed(_)
        | EventMsg::ModelReroute(_)
        | EventMsg::ModelVerification(_)
        | EventMsg::ContextCompacted(_)
        | EventMsg::ThreadRolledBack(_)
        | EventMsg::TurnStarted(_)
        | EventMsg::TurnComplete(_)
        | EventMsg::TokenCount(_)
        | EventMsg::UserMessage(_)
        | EventMsg::AgentMessageDelta(_)
        | EventMsg::AgentReasoning(_)
        | EventMsg::AgentReasoningDelta(_)
        | EventMsg::AgentReasoningRawContent(_)
        | EventMsg::AgentReasoningRawContentDelta(_)
        | EventMsg::AgentReasoningSectionBreak(_)
        | EventMsg::SessionConfigured(_)
        | EventMsg::ThreadNameUpdated(_)
        | EventMsg::ThreadGoalUpdated(_)
        | EventMsg::MapRuntime(_)
        | EventMsg::McpStartupUpdate(_)
        | EventMsg::McpStartupComplete(_)
        | EventMsg::McpToolCallBegin(_)
        | EventMsg::McpToolCallEnd(_)
        | EventMsg::WebSearchBegin(_)
        | EventMsg::WebSearchEnd(_)
        | EventMsg::WebFetchBegin(_)
        | EventMsg::WebFetchEnd(_)
        | EventMsg::ExecCommandBegin(_)
        | EventMsg::ExecCommandOutputDelta(_)
        | EventMsg::TerminalInteraction(_)
        | EventMsg::ExecCommandEnd(_)
        | EventMsg::PatchApplyBegin(_)
        | EventMsg::PatchApplyUpdated(_)
        | EventMsg::PatchApplyEnd(_)
        | EventMsg::ViewImageToolCall(_)
        | EventMsg::ImageGenerationBegin(_)
        | EventMsg::ImageGenerationEnd(_)
        | EventMsg::ExecApprovalRequest(_)
        | EventMsg::RequestPermissions(_)
        | EventMsg::RequestUserInput(_)
        | EventMsg::DynamicToolCallRequest(_)
        | EventMsg::DynamicToolCallResponse(_)
        | EventMsg::GuardianAssessment(_)
        | EventMsg::ElicitationRequest(_)
        | EventMsg::ApplyPatchApprovalRequest(_)
        | EventMsg::DeprecationNotice(_)
        | EventMsg::BackgroundEvent(_)
        | EventMsg::UndoStarted(_)
        | EventMsg::UndoCompleted(_)
        | EventMsg::StreamError(_)
        | EventMsg::TurnDiff(_)
        | EventMsg::GetHistoryEntryResponse(_)
        | EventMsg::McpListToolsResponse(_)
        | EventMsg::ListSkillsResponse(_)
        | EventMsg::RealtimeConversationListVoicesResponse(_)
        | EventMsg::SkillsUpdateAvailable
        | EventMsg::PlanUpdate(_)
        | EventMsg::TurnAborted(_)
        | EventMsg::ShutdownComplete
        | EventMsg::EnteredReviewMode(_)
        | EventMsg::ExitedReviewMode(_)
        | EventMsg::RawResponseItem(_)
        | EventMsg::ItemStarted(_)
        | EventMsg::HookStarted(_)
        | EventMsg::HookCompleted(_)
        | EventMsg::AgentMessageContentDelta(_)
        | EventMsg::PlanDelta(_)
        | EventMsg::ReasoningContentDelta(_)
        | EventMsg::ReasoningRawContentDelta(_)
        | EventMsg::CollabAgentSpawnBegin(_)
        | EventMsg::CollabAgentSpawnEnd(_)
        | EventMsg::CollabAgentInteractionBegin(_)
        | EventMsg::CollabAgentInteractionEnd(_)
        | EventMsg::CollabWaitingBegin(_)
        | EventMsg::CollabWaitingEnd(_)
        | EventMsg::CollabCloseBegin(_)
        | EventMsg::CollabCloseEnd(_)
        | EventMsg::CollabResumeBegin(_)
        | EventMsg::CollabResumeEnd(_) => None,
    }
}

/// Split the stream into normal assistant text vs. proposed plan content.
/// Normal text becomes AgentMessage deltas; plan content becomes PlanDelta +
/// TurnItem::Plan.
async fn handle_plan_segments(
    sess: &Session,
    turn_context: &TurnContext,
    state: &mut PlanModeStreamState,
    item_id: &str,
    segments: Vec<ProposedPlanSegment>,
) {
    for segment in segments {
        match segment {
            ProposedPlanSegment::Normal(delta) => {
                if delta.is_empty() {
                    continue;
                }
                let has_non_whitespace = delta.chars().any(|ch| !ch.is_whitespace());
                if !has_non_whitespace && !state.started_agent_message_items.contains(item_id) {
                    let entry = state
                        .leading_whitespace_by_item
                        .entry(item_id.to_string())
                        .or_default();
                    entry.push_str(&delta);
                    continue;
                }
                let delta = if !state.started_agent_message_items.contains(item_id) {
                    if let Some(prefix) = state.leading_whitespace_by_item.remove(item_id) {
                        format!("{prefix}{delta}")
                    } else {
                        delta
                    }
                } else {
                    delta
                };
                maybe_emit_pending_agent_message_start(sess, turn_context, state, item_id).await;

                let event = AgentMessageContentDeltaEvent {
                    thread_id: sess.conversation_id.to_string(),
                    turn_id: turn_context.sub_id.clone(),
                    item_id: item_id.to_string(),
                    delta,
                };
                sess.send_event(turn_context, EventMsg::AgentMessageContentDelta(event))
                    .await;
            }
            ProposedPlanSegment::ProposedPlanStart => {
                if !state.plan_item_state.completed {
                    state.plan_item_state.start(sess, turn_context).await;
                }
            }
            ProposedPlanSegment::ProposedPlanDelta(delta) => {
                if !state.plan_item_state.completed {
                    if !state.plan_item_state.started {
                        state.plan_item_state.start(sess, turn_context).await;
                    }
                    state
                        .plan_item_state
                        .push_delta(sess, turn_context, &delta)
                        .await;
                }
            }
            ProposedPlanSegment::ProposedPlanEnd => {}
        }
    }
}

async fn emit_streamed_assistant_text_delta(
    sess: &Session,
    turn_context: &TurnContext,
    plan_mode_state: Option<&mut PlanModeStreamState>,
    item_id: &str,
    parsed: ParsedAssistantTextDelta,
) {
    if parsed.is_empty() {
        return;
    }
    if !parsed.citations.is_empty() {
        // Citation extraction is intentionally local for now; we strip citations from display text
        // but do not yet surface them in protocol events.
        let _citations = parsed.citations;
    }
    if let Some(state) = plan_mode_state {
        if !parsed.plan_segments.is_empty() {
            handle_plan_segments(sess, turn_context, state, item_id, parsed.plan_segments).await;
        }
        return;
    }
    if parsed.visible_text.is_empty() {
        return;
    }
    let event = AgentMessageContentDeltaEvent {
        thread_id: sess.conversation_id.to_string(),
        turn_id: turn_context.sub_id.clone(),
        item_id: item_id.to_string(),
        delta: parsed.visible_text,
    };
    sess.send_event(turn_context, EventMsg::AgentMessageContentDelta(event))
        .await;
}

/// Flush buffered assistant text parser state when an assistant message item ends.
async fn flush_assistant_text_segments_for_item(
    sess: &Session,
    turn_context: &TurnContext,
    plan_mode_state: Option<&mut PlanModeStreamState>,
    parsers: &mut AssistantMessageStreamParsers,
    item_id: &str,
) {
    let parsed = parsers.finish_item(item_id);
    emit_streamed_assistant_text_delta(sess, turn_context, plan_mode_state, item_id, parsed).await;
}

/// Flush any remaining buffered assistant text parser state at response completion.
async fn flush_assistant_text_segments_all(
    sess: &Session,
    turn_context: &TurnContext,
    mut plan_mode_state: Option<&mut PlanModeStreamState>,
    parsers: &mut AssistantMessageStreamParsers,
) {
    for (item_id, parsed) in parsers.drain_finished() {
        emit_streamed_assistant_text_delta(
            sess,
            turn_context,
            plan_mode_state.as_deref_mut(),
            &item_id,
            parsed,
        )
        .await;
    }
}

/// Emit completion for plan items by parsing the finalized assistant message.
async fn maybe_complete_plan_item_from_message(
    sess: &Session,
    turn_context: &TurnContext,
    state: &mut PlanModeStreamState,
    item: &ResponseItem,
) {
    if let ResponseItem::Message { role, content, .. } = item
        && role == "assistant"
    {
        let mut text = String::new();
        for entry in content {
            if let ContentItem::OutputText { text: chunk } = entry {
                text.push_str(chunk);
            }
        }
        if let Some(plan_text) = extract_proposed_plan_text(&text) {
            let (plan_text, _citations) = strip_citations(&plan_text);
            if !state.plan_item_state.started {
                state.plan_item_state.start(sess, turn_context).await;
            }
            state
                .plan_item_state
                .complete_with_text(sess, turn_context, plan_text)
                .await;
        }
    }
}

/// Emit a completed agent message in plan mode, respecting deferred starts.
async fn emit_agent_message_in_plan_mode(
    sess: &Session,
    turn_context: &TurnContext,
    agent_message: codex_protocol::items::AgentMessageItem,
    state: &mut PlanModeStreamState,
) {
    let agent_message_id = agent_message.id.clone();
    let text = agent_message_text(&agent_message);
    if text.trim().is_empty() {
        state.pending_agent_message_items.remove(&agent_message_id);
        state.started_agent_message_items.remove(&agent_message_id);
        return;
    }

    maybe_emit_pending_agent_message_start(sess, turn_context, state, &agent_message_id).await;

    if !state
        .started_agent_message_items
        .contains(&agent_message_id)
    {
        let start_item = state
            .pending_agent_message_items
            .remove(&agent_message_id)
            .unwrap_or_else(|| {
                TurnItem::AgentMessage(codex_protocol::items::AgentMessageItem {
                    id: agent_message_id.clone(),
                    content: Vec::new(),
                    phase: None,
                    memory_citation: None,
                })
            });
        sess.emit_turn_item_started(turn_context, &start_item).await;
        state
            .started_agent_message_items
            .insert(agent_message_id.clone());
    }

    sess.emit_turn_item_completed(turn_context, TurnItem::AgentMessage(agent_message))
        .await;
    state.started_agent_message_items.remove(&agent_message_id);
}

/// Emit completion for a plan-mode turn item, handling agent messages specially.
async fn emit_turn_item_in_plan_mode(
    sess: &Session,
    turn_context: &TurnContext,
    turn_item: TurnItem,
    previously_active_item: Option<&TurnItem>,
    state: &mut PlanModeStreamState,
) {
    match turn_item {
        TurnItem::AgentMessage(agent_message) => {
            emit_agent_message_in_plan_mode(sess, turn_context, agent_message, state).await;
        }
        _ => {
            if previously_active_item.is_none() {
                sess.emit_turn_item_started(turn_context, &turn_item).await;
            }
            sess.emit_turn_item_completed(turn_context, turn_item).await;
        }
    }
}

/// Handle a completed assistant response item in plan mode, returning true if handled.
async fn handle_assistant_item_done_in_plan_mode(
    sess: &Session,
    turn_context: &TurnContext,
    item: &ResponseItem,
    state: &mut PlanModeStreamState,
    previously_active_item: Option<&TurnItem>,
    last_agent_message: &mut Option<String>,
) -> bool {
    if let ResponseItem::Message { role, .. } = item
        && role == "assistant"
    {
        maybe_complete_plan_item_from_message(sess, turn_context, state, item).await;

        if let Some(turn_item) =
            handle_non_tool_response_item(sess, turn_context, item, /*plan_mode*/ true).await
        {
            emit_turn_item_in_plan_mode(
                sess,
                turn_context,
                turn_item,
                previously_active_item,
                state,
            )
            .await;
        }

        record_completed_response_item(sess, turn_context, item).await;
        if let Some(agent_message) = last_assistant_message_from_item(item, /*plan_mode*/ true) {
            *last_agent_message = Some(agent_message);
        }
        return true;
    }
    false
}

async fn drain_in_flight(
    in_flight: &mut FuturesOrdered<BoxFuture<'static, CodexResult<ResponseInputItem>>>,
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
) -> CodexResult<()> {
    while let Some(res) = in_flight.next().await {
        match res {
            Ok(response_input) => {
                record_response_input_item(sess.as_ref(), turn_context.as_ref(), response_input)
                    .await;
            }
            Err(err) => {
                error_or_panic(format!("in-flight tool future failed during drain: {err}"));
            }
        }
    }
    Ok(())
}

async fn record_response_input_item(
    sess: &Session,
    turn_context: &TurnContext,
    response_input: ResponseInputItem,
) -> ResponseItem {
    let response_item = response_input.into();
    sess.record_conversation_items(turn_context, std::slice::from_ref(&response_item))
        .await;
    mark_thread_memory_mode_polluted_if_external_context(sess, turn_context, &response_item).await;
    response_item
}

fn response_input_for_taskspace_action_tool_error(
    tool_call: &ToolCall,
    err: &CodexErr,
) -> ResponseInputItem {
    let output = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(err.to_string()),
        success: Some(false),
    };
    match &tool_call.payload {
        ToolPayload::Custom { .. } => ResponseInputItem::CustomToolCallOutput {
            call_id: tool_call.call_id.clone(),
            name: Some(tool_call.tool_name.name.clone()),
            output,
        },
        _ => ResponseInputItem::FunctionCallOutput {
            call_id: tool_call.call_id.clone(),
            output,
        },
    }
}

async fn run_taskspace_inspect_bootstrap(
    tool_runtime: ToolCallRuntime,
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    request_count: usize,
    call_id_prefix: &str,
    command: &str,
    event_message: &str,
    cancellation_token: CancellationToken,
) -> CodexResult<()> {
    let call_id = format!("{call_id_prefix}-{request_count}");
    let arguments = serde_json::json!({
        "command": command,
        "timeout_ms": 10000,
    })
    .to_string();
    let call_item = ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: arguments.clone(),
        call_id: call_id.clone(),
    };
    record_completed_response_item(sess.as_ref(), turn_context.as_ref(), &call_item).await;
    sess.send_event(
        &turn_context,
        EventMsg::Warning(WarningEvent {
            message: event_message.to_string(),
        }),
    )
    .await;

    let response_input = tool_runtime
        .handle_tool_call(
            ToolCall {
                tool_name: ToolName::plain("shell_command"),
                call_id: call_id.clone(),
                payload: ToolPayload::Function { arguments },
            },
            cancellation_token,
        )
        .await?;
    let response_item =
        record_response_input_item(sess.as_ref(), turn_context.as_ref(), response_input).await;
    let (success, output) =
        taskspace_tool_output_success_and_text(&response_item).unwrap_or((false, String::new()));
    let preview = format!(
        "TaskSpaceToolInvocationV1:\n\
tool: shell_command\n\
command: {command}\n\
raw_output:\n\
{output}"
    );
    sess.record_action_map_main_tool_result(
        &turn_context,
        &call_id,
        "shell_command",
        Some(ActionClass::Read),
        success,
        preview,
    )
    .await;
    let evidence_item = build_taskspace_inspect_bootstrap_evidence_item(command, &response_item);
    record_completed_response_item(sess.as_ref(), turn_context.as_ref(), &evidence_item).await;
    Ok(())
}

async fn run_taskspace_duplicate_read_search_bootstrap_then_force(
    tool_runtime: ToolCallRuntime,
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    snapshot: crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    cancellation_token: CancellationToken,
) -> CodexResult<Option<ResponseItem>> {
    run_taskspace_inspect_bootstrap(
        tool_runtime,
        sess.clone(),
        turn_context.clone(),
        snapshot.request_count,
        TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_CALL_ID,
        taskspace_repeated_blocked_inspect_bootstrap_command(),
        "TaskSpaceRepeatedBlockedInspectBootstrapV1 executed bounded source/test/data artifact reads after repeated duplicate inspect read/search.",
        cancellation_token,
    )
    .await?;

    match sess
        .force_finish_action_map_inspect_for_provider_budget(
            &turn_context,
            snapshot,
            "inspect_duplicate_read_search_bootstrap_complete",
        )
        .await
    {
        Ok(true) => Ok(Some(
            build_taskspace_forced_inspect_transition_recovery_item(),
        )),
        Ok(false) => Ok(None),
        Err(error) => {
            sess.send_event(
                &turn_context,
                EventMsg::Warning(WarningEvent {
                    message: format!(
                        "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_duplicate_read_search_bootstrap_complete error={error}"
                    ),
                }),
            )
            .await;
            Ok(None)
        }
    }
}

async fn run_taskspace_missing_fact_source_bootstrap(
    tool_runtime: ToolCallRuntime,
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    request_count: usize,
    artifacts: &[String],
    cancellation_token: CancellationToken,
) -> CodexResult<()> {
    let artifacts = artifacts.iter().take(4).cloned().collect::<Vec<_>>();
    if artifacts.is_empty() {
        return Ok(());
    }
    let command = taskspace_missing_fact_source_bootstrap_command(&artifacts);
    let call_id = format!("{TASKSPACE_MISSING_FACT_SOURCE_BOOTSTRAP_CALL_ID}-{request_count}");
    let arguments = serde_json::json!({
        "command": command,
        "timeout_ms": 10000,
    })
    .to_string();
    let call_item = ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: arguments.clone(),
        call_id: call_id.clone(),
    };
    record_completed_response_item(sess.as_ref(), turn_context.as_ref(), &call_item).await;
    sess.send_event(
        &turn_context,
        EventMsg::Warning(WarningEvent {
            message: format!(
                "TaskSpaceMissingFactSourceBootstrapV1 read bounded declared fact-source artifact(s) after repeated duplicate inspect read: {}",
                artifacts.join(", ")
            ),
        }),
    )
    .await;

    let response_input = tool_runtime
        .handle_tool_call(
            ToolCall {
                tool_name: ToolName::plain("shell_command"),
                call_id: call_id.clone(),
                payload: ToolPayload::Function { arguments },
            },
            cancellation_token,
        )
        .await?;
    let response_item =
        record_response_input_item(sess.as_ref(), turn_context.as_ref(), response_input).await;
    let (success, output) =
        taskspace_tool_output_success_and_text(&response_item).unwrap_or((false, String::new()));
    let preview = format!(
        "TaskSpaceToolInvocationV1:\n\
tool: shell_command\n\
command: {command}\n\
raw_output:\n\
{output}"
    );
    sess.record_action_map_main_tool_result(
        &turn_context,
        &call_id,
        "shell_command",
        Some(ActionClass::Read),
        success,
        preview,
    )
    .await;
    let evidence_item = build_taskspace_inspect_bootstrap_evidence_item(&command, &response_item);
    record_completed_response_item(sess.as_ref(), turn_context.as_ref(), &evidence_item).await;
    Ok(())
}

async fn run_taskspace_validation_required_command_bootstrap(
    tool_runtime: ToolCallRuntime,
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    request_count: usize,
    command: &str,
    cancellation_token: CancellationToken,
) -> CodexResult<()> {
    let mut current_command = command.trim().to_string();
    for attempt in 1..=3 {
        let call_id = format!(
            "{TASKSPACE_VALIDATION_REQUIRED_COMMAND_BOOTSTRAP_CALL_ID}-{request_count}-{attempt}"
        );
        let arguments = serde_json::json!({
            "command": current_command,
            "timeout_ms": 120000,
        })
        .to_string();
        let call_item = ResponseItem::FunctionCall {
            id: None,
            name: "shell_command".to_string(),
            namespace: None,
            arguments: arguments.clone(),
            call_id: call_id.clone(),
        };
        record_completed_response_item(sess.as_ref(), turn_context.as_ref(), &call_item).await;
        sess.send_event(
            &turn_context,
            EventMsg::Warning(WarningEvent {
                message: format!(
                    "TaskSpaceValidationRequiredCommandBootstrapV1 executed coverage-correct validation command after a rejected validation run_test: {current_command}"
                ),
            }),
        )
        .await;

        let response_input = tool_runtime
            .clone()
            .handle_tool_call(
                ToolCall {
                    tool_name: ToolName::plain("shell_command"),
                    call_id: call_id.clone(),
                    payload: ToolPayload::Function { arguments },
                },
                cancellation_token.child_token(),
            )
            .await?;
        let response_item =
            record_response_input_item(sess.as_ref(), turn_context.as_ref(), response_input).await;
        let (success, output) = taskspace_tool_output_success_and_text(&response_item)
            .unwrap_or((false, String::new()));
        if !success
            && let Some(next_command) =
                taskspace_validation_chained_required_command(&current_command, &output)
        {
            sess.send_event(
                &turn_context,
                EventMsg::Warning(WarningEvent {
                    message: format!(
                        "TaskSpaceValidationRequiredCommandBootstrapChainedV1 followed nested validation gate command after `{current_command}`: {next_command}"
                    ),
                }),
            )
            .await;
            current_command = next_command;
            continue;
        }
        let preview = format!(
            "TaskSpaceToolInvocationV1:\n\
tool: shell_command\n\
command: {current_command}\n\
raw_output:\n\
{output}"
        );
        sess.record_action_map_main_tool_result(
            &turn_context,
            &call_id,
            "shell_command",
            Some(ActionClass::Test),
            success,
            preview,
        )
        .await;
        break;
    }
    Ok(())
}

fn taskspace_tool_output_success_and_text(item: &ResponseItem) -> Option<(bool, String)> {
    match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => Some((
            output.success.unwrap_or(true),
            function_call_output_body_text(&output.body),
        )),
        _ => None,
    }
}

fn parse_taskspace_action_v1(text: &str) -> Result<TaskSpaceActionV1, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty_action_contract_output".to_string());
    }
    if trimmed.starts_with("```") {
        if let Some(fenced_json) = taskspace_single_fenced_json_body(trimmed) {
            return parse_taskspace_action_v1(fenced_json);
        }
        return Err("action_contract_output_not_strict_json".to_string());
    }
    if !trimmed.starts_with('{') {
        if let Some(json_start) = trimmed.find('{') {
            return parse_taskspace_action_v1(&trimmed[json_start..]);
        }
        return taskspace_action_from_deepseek_dsml(trimmed)
            .ok_or_else(|| "action_contract_output_not_strict_json".to_string());
    }
    let json_end = taskspace_leading_json_object_end(trimmed)
        .ok_or_else(|| "malformed_action_json:unterminated_object".to_string())?;
    let action = serde_json::from_str::<TaskSpaceActionV1>(&trimmed[..json_end])
        .map_err(|err| format!("malformed_action_json:{err}"))?;
    let suffix = trimmed[json_end..].trim();
    if !suffix.is_empty()
        && !suffix.contains("DSML")
        && !(suffix == "\"" && action.action == "apply_patch")
    {
        return Err("action_contract_output_not_strict_json".to_string());
    }
    if action.schema_version != "taskspace-action-v1" {
        return Err("unsupported_action_schema_version".to_string());
    }
    if action.action.trim().is_empty() {
        return Err("missing_action".to_string());
    }
    Ok(action)
}

fn taskspace_single_fenced_json_body(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("```")?;
    let header_end = rest.find('\n')?;
    let header = rest[..header_end].trim();
    if !header.is_empty() && !header.eq_ignore_ascii_case("json") {
        return None;
    }
    let body_with_closing = rest[header_end + 1..].trim_end();
    let body = body_with_closing.strip_suffix("```")?.trim();
    if body.starts_with('{') {
        Some(body)
    } else {
        None
    }
}

fn taskspace_action_from_deepseek_dsml(text: &str) -> Option<TaskSpaceActionV1> {
    if !text.contains("DSML") || !text.contains("invoke name=\"shell_command\"") {
        return None;
    }
    let command = taskspace_dsml_parameter(text, "command")?;
    let command = command.trim();
    let args = if command.starts_with("rg --files") {
        let path = command
            .strip_prefix("rg --files")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(".");
        ("list_files", serde_json::json!({ "path": path }))
    } else if command.starts_with("Get-Content") {
        let path = taskspace_powershell_path_arg(command)?;
        ("read_file", serde_json::json!({ "path": path }))
    } else if let Some(path) = command.strip_prefix("cat ").map(str::trim) {
        (
            "read_file",
            serde_json::json!({ "path": path.trim_matches('"') }),
        )
    } else if let Some(path) = command.strip_prefix("type ").map(str::trim) {
        (
            "read_file",
            serde_json::json!({ "path": path.trim_matches('"') }),
        )
    } else if let Some(path) = taskspace_python_open_path(command) {
        ("read_file", serde_json::json!({ "path": path }))
    } else if command.starts_with("rg ") {
        ("search", serde_json::json!({ "pattern": "", "path": "." }))
    } else if command.contains("pytest") || command.contains("cargo test") {
        (
            "run_test",
            serde_json::json!({ "command": command, "timeout_ms": 120000 }),
        )
    } else {
        return None;
    };
    Some(TaskSpaceActionV1 {
        schema_version: "taskspace-action-v1".to_string(),
        action: args.0.to_string(),
        node_id: None,
        args: args.1,
        rationale: Some("Recovered provider DSML shell command as TaskSpaceActionV1".to_string()),
    })
}

fn taskspace_dsml_parameter(text: &str, name: &str) -> Option<String> {
    let marker = format!("parameter name=\"{name}\"");
    let start = text.find(&marker)?;
    let after_marker = &text[start..];
    let value_start = after_marker.find('>')? + 1;
    let after_start = &after_marker[value_start..];
    let value_end = after_start.find("</")?;
    Some(after_start[..value_end].to_string())
}

fn taskspace_powershell_path_arg(command: &str) -> Option<String> {
    let marker = if command.contains("-LiteralPath") {
        "-LiteralPath"
    } else {
        "-Path"
    };
    let start = command.find(marker)? + marker.len();
    let rest = command[start..].trim_start();
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn taskspace_python_open_path(command: &str) -> Option<String> {
    if !command.starts_with("python -c") || !command.contains("open(") {
        return None;
    }
    let open_start = command.find("open(")? + "open(".len();
    let rest = command[open_start..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let after_quote = &rest[quote.len_utf8()..];
    let end = after_quote.find(quote)?;
    Some(after_quote[..end].to_string())
}

fn taskspace_leading_json_object_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(idx + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn taskspace_action_allowed_for_node(action: &str, node_kind: Option<&str>) -> bool {
    match node_kind {
        Some("inspect_code_context") => matches!(
            action,
            "list_files"
                | "search"
                | "read_file"
                | "run_test"
                | "taskspace_control"
                | "state_commit"
                | "blocked"
        ),
        Some("implement_solution") => matches!(
            action,
            "list_files"
                | "read_file"
                | "search"
                | "apply_patch"
                | "taskspace_control"
                | "state_commit"
                | "blocked"
        ),
        Some("smoke_test" | "regression_test") => {
            matches!(
                action,
                "run_test" | "taskspace_control" | "state_commit" | "blocked"
            )
        }
        Some("final_synthesis") => {
            matches!(
                action,
                "final_answer" | "taskspace_control" | "state_commit" | "blocked"
            )
        }
        _ => matches!(action, "taskspace_control" | "state_commit" | "blocked"),
    }
}

fn taskspace_action_is_read_or_search(action: &str) -> bool {
    matches!(action, "list_files" | "read_file" | "search")
}

async fn should_finish_node_after_successful_required_action(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if taskspace_action_blocks_successful_required_action_auto_finish(action) {
        return false;
    }
    match snapshot.node_kind.as_deref() {
        Some("implement_solution") => {
            sess.action_map_current_main_node_has_successful_action(ActionClass::Edit)
                .await
        }
        Some("smoke_test" | "regression_test") => {
            sess.action_map_current_main_node_has_successful_action(ActionClass::Test)
                .await
                || sess
                    .action_map_current_main_node_has_successful_action(ActionClass::Build)
                    .await
        }
        Some("inspect_code_context") => {
            taskspace_action_is_read_or_search(&action.action)
                && sess
                    .action_map_current_main_inspect_has_successful_diagnostic_and_working_evidence(
                    )
                    .await
        }
        _ => false,
    }
}

async fn should_answer_after_successful_validation_redundant_node(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if snapshot.node_id.is_some() {
        return false;
    }
    if !taskspace_action_control_creates_validation_node(action) {
        return false;
    }
    sess.action_map_has_accepted_successful_validation_result()
        .await
}

async fn should_answer_after_completed_task_without_active_node(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if snapshot.node_id.is_some() {
        return false;
    }
    if taskspace_action_final_message(action).is_some() {
        return false;
    }
    sess.action_map_has_accepted_successful_validation_result()
        .await
}

async fn should_block_after_closed_validation_without_active_node(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if snapshot.node_id.is_some() {
        return false;
    }
    if taskspace_action_final_message(action).is_some() {
        return false;
    }
    sess.action_map_has_blocked_validation_result().await
        && !sess.action_map_has_ready_recovery_node().await
}

async fn should_block_after_tool_runtime_bootstrap_failure_without_active_node(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if snapshot.node_id.is_some() {
        return false;
    }
    if taskspace_action_final_message(action).is_some() {
        return false;
    }
    sess.action_map_has_tool_runtime_bootstrap_failure().await
}

fn taskspace_action_is_finish_node_control(action: &TaskSpaceActionV1) -> bool {
    action.action == "taskspace_control"
        && taskspace_action_control_action(action) == Some("finish_node")
}

fn taskspace_action_blocks_successful_required_action_auto_finish(
    action: &TaskSpaceActionV1,
) -> bool {
    taskspace_action_is_finish_node_control(action)
}

const TASKSPACE_CONTROL_ACTION_MISSING_ERROR: &str = "E_TASKSPACE_CONTROL_ACTION_MISSING";
const TASKSPACE_CONTROL_ARGS_NOT_OBJECT_ERROR: &str = "E_TASKSPACE_CONTROL_ARGS_NOT_OBJECT";
const TASKSPACE_CONTROL_ACTION_CONFLICT_ERROR: &str = "E_TASKSPACE_CONTROL_ACTION_CONFLICT";
const TASKSPACE_CONTROL_ACTION_ALIASES: [&str; 4] =
    ["control_action", "control_type", "action_name", "command"];
const TASKSPACE_CONTROL_ACTION_KEYS: [&str; 5] = [
    "action",
    "control_action",
    "control_type",
    "action_name",
    "command",
];

fn taskspace_action_control_action(action: &TaskSpaceActionV1) -> Option<&str> {
    let root = action.args.as_object()?;
    taskspace_control_action_from_root(root)
}

fn taskspace_control_action_from_root<'a>(
    root: &'a serde_json::Map<String, serde_json::Value>,
) -> Option<&'a str> {
    TASKSPACE_CONTROL_ACTION_KEYS.iter().find_map(|key| {
        root.get(*key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn canonicalize_taskspace_control_action_arg(
    root: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let mut selected = root
        .get("action")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    for alias in TASKSPACE_CONTROL_ACTION_ALIASES {
        let Some(alias_value) = root
            .get(alias)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if let Some(current) = selected.as_deref() {
            if current != alias_value {
                return Err(format!(
                    "{TASKSPACE_CONTROL_ACTION_CONFLICT_ERROR}:action={current}:{alias}={alias_value}"
                ));
            }
        } else {
            selected = Some(alias_value.to_string());
        }
    }

    let Some(action) = selected else {
        return Err(TASKSPACE_CONTROL_ACTION_MISSING_ERROR.to_string());
    };

    root.insert(
        "action".to_string(),
        serde_json::Value::String(action.clone()),
    );
    for alias in TASKSPACE_CONTROL_ACTION_ALIASES {
        root.remove(alias);
    }
    Ok(action)
}

fn taskspace_action_control_creates_validation_node(action: &TaskSpaceActionV1) -> bool {
    if action.action != "taskspace_control"
        || taskspace_action_control_action(action) != Some("create_node")
    {
        return false;
    }
    let Some(kind) = action
        .args
        .get("node_kind")
        .or_else(|| action.args.get("kind"))
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    kind.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .any(|part| matches!(part, "smoke_test" | "regression_test"))
}

fn taskspace_action_control_creates_final_synthesis(action: &TaskSpaceActionV1) -> bool {
    action.action == "taskspace_control"
        && taskspace_action_control_action(action) == Some("create_node")
        && action
            .args
            .get("node_kind")
            .or_else(|| action.args.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("final_synthesis")
}

fn taskspace_final_answer_action(message: &str) -> TaskSpaceActionV1 {
    TaskSpaceActionV1 {
        schema_version: "taskspace-action-v1".to_string(),
        action: "final_answer".to_string(),
        node_id: None,
        args: serde_json::json!({ "message": message }),
        rationale: Some("Thin TaskSpace path is complete after validation.".to_string()),
    }
}

fn taskspace_blocked_final_action(reason: &str) -> TaskSpaceActionV1 {
    TaskSpaceActionV1 {
        schema_version: "taskspace-action-v1".to_string(),
        action: "blocked".to_string(),
        node_id: None,
        args: serde_json::json!({ "reason": reason }),
        rationale: Some("TaskSpace path is closed by a blocked validation node.".to_string()),
    }
}

#[cfg(test)]
#[test]
fn final_answer_gate_rejection_followup_preserves_specific_reason() {
    let message = taskspace_final_answer_gate_rejection_followup(
        "TaskSpace final answer gate rejected hidden orchestration term `taskspace`.",
    );

    assert!(message.contains("Rejection reason:"));
    assert!(message.contains("hidden orchestration term `taskspace`"));
    assert!(message.contains("Correct the specific rejection reason before final_answer"));
}

fn taskspace_finish_current_node_action(
    node_id: Option<&str>,
    result_summary: &str,
) -> TaskSpaceActionV1 {
    let mut args = serde_json::Map::new();
    args.insert(
        "action".to_string(),
        serde_json::Value::String("finish_node".to_string()),
    );
    if let Some(node_id) = node_id.filter(|value| !value.trim().is_empty()) {
        args.insert(
            "node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );
    }
    args.insert(
        "result_summary".to_string(),
        serde_json::Value::String(result_summary.to_string()),
    );
    TaskSpaceActionV1 {
        schema_version: "taskspace-action-v1".to_string(),
        action: "taskspace_control".to_string(),
        node_id: node_id.map(str::to_string),
        args: serde_json::Value::Object(args),
        rationale: Some("A successful implementation edit is already recorded.".to_string()),
    }
}

fn taskspace_finish_inspect_to_implementation_action(node_id: Option<&str>) -> TaskSpaceActionV1 {
    let mut action = taskspace_finish_current_node_action(
        node_id,
        "Inspect evidence already includes successful diagnostic output and working evidence; proceed to implementation.",
    );
    if let serde_json::Value::Object(root) = &mut action.args {
        root.insert(
            "next_node_kind".to_string(),
            serde_json::Value::String("implement_solution".to_string()),
        );
        root.insert(
            "next_node_title".to_string(),
            serde_json::Value::String("Apply inspected fix".to_string()),
        );
        root.insert(
            "next_node_context_summary".to_string(),
            serde_json::Value::String(
                "Apply the narrow implementation change indicated by the successful inspect diagnostic and working evidence.".to_string(),
            ),
        );
        if let Some(node_id) = node_id.filter(|value| !value.trim().is_empty()) {
            root.insert(
                "next_dependency_node_ids".to_string(),
                serde_json::Value::Array(vec![serde_json::Value::String(node_id.to_string())]),
            );
        }
    }
    action.rationale = Some(
        "Inspect already has successful diagnostic and working evidence; avoid low-value discovery."
            .to_string(),
    );
    action
}

fn taskspace_action_arg_string(args: &serde_json::Value, name: &str) -> Option<String> {
    args.get(name)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn taskspace_action_arg_u64(args: &serde_json::Value, name: &str, default: u64) -> u64 {
    args.get(name)
        .and_then(|value| value.as_u64())
        .unwrap_or(default)
}

fn normalize_taskspace_apply_patch(patch: &str) -> String {
    let normalized = patch.replace("\r\n", "\n");
    if let Some(rewritten) = normalize_taskspace_unwrapped_apply_patch(&normalized) {
        return normalize_taskspace_native_patch_payloads(&rewritten);
    }
    if let Some(rewritten) = normalize_taskspace_bare_file_patch(&normalized) {
        return normalize_taskspace_native_patch_payloads(&rewritten);
    }
    let lines = normalized.lines().collect::<Vec<_>>();
    let has_native_file_operation = lines
        .iter()
        .any(|line| line.starts_with("*** Update File: ") || line.starts_with("*** Add File: "));
    if lines.len() < 5
        || lines.first() != Some(&"*** Begin Patch")
        || lines.last() != Some(&"*** End Patch")
        || !has_native_file_operation
    {
        return normalized;
    }
    let rewritten = rewrite_taskspace_apply_patch_unique_update_paths(&normalized);
    if let Some(rewritten) = normalize_taskspace_update_file_whole_replacement(&rewritten) {
        return normalize_taskspace_native_patch_payloads(&rewritten);
    }
    normalize_taskspace_native_patch_payloads(&rewritten)
}

fn normalize_taskspace_native_patch_payloads(patch: &str) -> String {
    let patch = normalize_taskspace_duplicate_empty_update_sections(patch);
    let patch = normalize_taskspace_separator_update_sections(&patch);
    normalize_taskspace_python_add_file_common_indent(&normalize_taskspace_native_hunk_headers(
        &patch,
    ))
}

fn normalize_taskspace_duplicate_empty_update_sections(patch: &str) -> String {
    let source_lines = patch.lines().collect::<Vec<_>>();
    let mut changed = false;
    let mut lines = Vec::with_capacity(source_lines.len());
    let mut index = 0usize;
    while index < source_lines.len() {
        let line = source_lines[index];
        let Some(target) = line.strip_prefix("*** Update File: ").map(str::trim) else {
            lines.push(line.to_string());
            index += 1;
            continue;
        };

        let mut next = index + 1;
        while source_lines
            .get(next)
            .is_some_and(|candidate| candidate.trim().is_empty())
        {
            next += 1;
        }
        let duplicate_target = source_lines
            .get(next)
            .and_then(|candidate| candidate.strip_prefix("*** Update File: "))
            .map(str::trim);
        if duplicate_target == Some(target) {
            changed = true;
            index = next;
            continue;
        }

        lines.push(line.to_string());
        index += 1;
    }

    if changed {
        lines.join("\n") + "\n"
    } else {
        patch.to_string()
    }
}

fn normalize_taskspace_separator_update_sections(patch: &str) -> String {
    let source_lines = patch.lines().collect::<Vec<_>>();
    let mut changed = false;
    let mut lines = Vec::with_capacity(source_lines.len());
    let mut index = 0usize;
    while index < source_lines.len() {
        let line = source_lines[index];
        let Some(_target) = line.strip_prefix("*** Update File: ").map(str::trim) else {
            lines.push(line.to_string());
            index += 1;
            continue;
        };

        lines.push(line.to_string());
        index += 1;

        let mut section = Vec::new();
        while index < source_lines.len() {
            let next = source_lines[index];
            if next == "*** End Patch"
                || next.starts_with("*** Add File: ")
                || next.starts_with("*** Update File: ")
                || next.starts_with("*** Delete File: ")
            {
                break;
            }
            section.push(next);
            index += 1;
        }

        if let Some(rewritten) = normalize_taskspace_separator_update_section(&section) {
            changed = true;
            lines.extend(rewritten);
        } else {
            lines.extend(section.into_iter().map(str::to_string));
        }
    }

    if changed {
        lines.join("\n") + "\n"
    } else {
        patch.to_string()
    }
}

fn normalize_taskspace_separator_update_section(section: &[&str]) -> Option<Vec<String>> {
    if section.is_empty()
        || section.iter().any(|line| line.starts_with("@@"))
        || section
            .iter()
            .any(|line| line.starts_with('+') && !line.starts_with("+++"))
        || section
            .iter()
            .any(|line| line.starts_with('-') && !line.starts_with("---"))
    {
        return None;
    }
    let separators = section
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line.trim() == "---").then_some(index))
        .collect::<Vec<_>>();
    let separator = match separators.as_slice() {
        [separator] => *separator,
        _ => return None,
    };
    let old_lines = section[..separator]
        .iter()
        .copied()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let new_lines = section[separator + 1..]
        .iter()
        .copied()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if old_lines.is_empty() || new_lines.is_empty() {
        return None;
    }

    let mut rewritten = Vec::with_capacity(old_lines.len() + new_lines.len() + 1);
    rewritten.push("@@".to_string());
    rewritten.extend(old_lines.into_iter().map(|line| format!("-{line}")));
    rewritten.extend(new_lines.into_iter().map(|line| format!("+{line}")));
    Some(rewritten)
}

fn normalize_taskspace_native_hunk_headers(patch: &str) -> String {
    let mut changed = false;
    let mut section_kind: Option<&str> = None;
    let mut update_hunk_started = false;
    let mut lines = Vec::new();
    let source_lines = patch.lines().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < source_lines.len() {
        let line = source_lines[index];
        if line.starts_with("*** Add File: ") {
            section_kind = Some("add");
            lines.push(line.to_string());
            index += 1;
            continue;
        }
        if line.starts_with("*** Update File: ") {
            section_kind = Some("update");
            update_hunk_started = false;
            lines.push(line.to_string());
            index += 1;
            continue;
        }
        if line.starts_with("*** Delete File: ") {
            section_kind = Some("delete");
            update_hunk_started = false;
            lines.push(line.to_string());
            index += 1;
            continue;
        }
        if line == "*** End Patch" {
            section_kind = None;
            update_hunk_started = false;
            lines.push(line.to_string());
            index += 1;
            continue;
        }
        if section_kind == Some("update")
            && !update_hunk_started
            && taskspace_line_looks_unified_old_file_header(line)
            && source_lines
                .get(index + 1)
                .is_some_and(|next| taskspace_line_looks_unified_new_file_header(next))
        {
            changed = true;
            index += 2;
            continue;
        }
        if line.starts_with("@@") {
            match section_kind {
                Some("add") => {
                    changed = true;
                    index += 1;
                    continue;
                }
                Some("update") => {
                    update_hunk_started = true;
                    let normalized = normalize_taskspace_unified_hunk_line(line);
                    if normalized != line {
                        changed = true;
                    }
                    lines.push(normalized);
                    index += 1;
                    continue;
                }
                _ => {}
            }
        }
        lines.push(line.to_string());
        index += 1;
    }
    if changed {
        lines.join("\n") + "\n"
    } else {
        patch.to_string()
    }
}

fn normalize_taskspace_python_add_file_common_indent(patch: &str) -> String {
    let source_lines = patch.lines().collect::<Vec<_>>();
    let mut changed = false;
    let mut lines = Vec::new();
    let mut index = 0usize;
    while index < source_lines.len() {
        let line = source_lines[index];
        let Some(target) = line.strip_prefix("*** Add File: ").map(str::trim) else {
            lines.push(line.to_string());
            index += 1;
            continue;
        };

        lines.push(line.to_string());
        index += 1;

        let mut content = Vec::new();
        while index < source_lines.len() {
            let next = source_lines[index];
            if next == "*** End Patch"
                || next.starts_with("*** Add File: ")
                || next.starts_with("*** Update File: ")
                || next.starts_with("*** Delete File: ")
            {
                break;
            }
            content.push(next);
            index += 1;
        }

        if taskspace_patch_target_is_python(target)
            && taskspace_added_file_content_has_common_single_indent(&content)
        {
            changed = true;
            lines.extend(content.into_iter().map(|content_line| {
                content_line
                    .strip_prefix("+ ")
                    .map(|rest| format!("+{rest}"))
                    .unwrap_or_else(|| content_line.to_string())
            }));
        } else {
            lines.extend(content.into_iter().map(str::to_string));
        }
    }

    if changed {
        lines.join("\n") + "\n"
    } else {
        patch.to_string()
    }
}

fn taskspace_patch_target_is_python(target: &str) -> bool {
    let normalized = target.trim().to_ascii_lowercase();
    normalized.ends_with(".py") || normalized.ends_with(".pyw")
}

fn normalize_taskspace_update_file_whole_replacement(patch: &str) -> Option<String> {
    let source_lines = patch.lines().collect::<Vec<_>>();
    if source_lines.len() < 7
        || source_lines.first() != Some(&"*** Begin Patch")
        || source_lines.last() != Some(&"*** End Patch")
    {
        return None;
    }

    let update_indices = source_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with("*** Update File: ").then_some(index))
        .collect::<Vec<_>>();
    if update_indices.len() != 1 {
        return None;
    }
    if source_lines
        .iter()
        .any(|line| line.starts_with("*** Add File: ") || line.starts_with("*** Delete File: "))
    {
        return None;
    }

    let update_index = update_indices[0];
    let target = source_lines[update_index]
        .strip_prefix("*** Update File: ")?
        .trim();
    if !taskspace_patch_target_is_python(target) {
        return None;
    }

    let content = &source_lines[update_index + 1..source_lines.len().saturating_sub(1)];
    if content.len() < 5
        || content.iter().any(|line| {
            line.starts_with("@@")
                || line.starts_with('+')
                || line.starts_with('-')
                || line.starts_with("*** ")
        })
        || !taskspace_update_file_replacement_looks_like_python_source(content)
    {
        return None;
    }

    let mut rewritten = Vec::with_capacity(content.len() + 4);
    rewritten.push("*** Begin Patch".to_string());
    rewritten.push(format!("*** Delete File: {target}"));
    rewritten.push(format!("*** Add File: {target}"));
    for line in content {
        rewritten.push(format!("+{line}"));
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
}

fn taskspace_update_file_replacement_looks_like_python_source(content: &[&str]) -> bool {
    let Some(first) = content
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
    else {
        return false;
    };
    first.starts_with("#!")
        || first.starts_with("import ")
        || first.starts_with("from ")
        || first.starts_with("def ")
        || first.starts_with("class ")
}

fn taskspace_added_file_content_has_common_single_indent(content: &[&str]) -> bool {
    let mut saw_non_empty_added_content = false;
    for line in content {
        let Some(rest) = line.strip_prefix('+') else {
            if line.trim().is_empty() {
                continue;
            }
            return false;
        };
        if rest.is_empty() || rest.trim().is_empty() {
            continue;
        }
        saw_non_empty_added_content = true;
        if !rest.starts_with(' ') {
            return false;
        }
    }
    saw_non_empty_added_content
}

fn taskspace_line_looks_unified_old_file_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "---" || trimmed.starts_with("--- ") || trimmed.starts_with("---\t")
}

fn taskspace_line_looks_unified_new_file_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "+++" || trimmed.starts_with("+++ ") || trimmed.starts_with("+++\t")
}

fn normalize_taskspace_bare_file_patch(patch: &str) -> Option<String> {
    let lines = patch.lines().collect::<Vec<_>>();
    if lines.len() < 5 || lines.first() != Some(&"*** Begin Patch") {
        return None;
    }
    if lines.last() != Some(&"*** End Patch") {
        return None;
    }
    let raw_path = lines.get(1)?.trim();
    if raw_path.is_empty()
        || raw_path == "---"
        || raw_path == "+++"
        || raw_path.starts_with("*** ")
        || raw_path.starts_with("--- ")
        || raw_path.starts_with("+++ ")
        || raw_path.contains(char::is_whitespace)
    {
        return None;
    }
    if !lines
        .iter()
        .skip(2)
        .any(|line| line.starts_with('-') || line.starts_with('+'))
    {
        return None;
    }
    let path = normalize_taskspace_relative_patch_path(raw_path);
    let mut rewritten = Vec::with_capacity(lines.len() + 1);
    rewritten.push("*** Begin Patch".to_string());
    rewritten.push(format!("*** Update File: {path}"));
    let has_explicit_hunk = lines
        .iter()
        .skip(2)
        .take(lines.len().saturating_sub(3))
        .any(|line| line.starts_with("@@"));
    if !has_explicit_hunk {
        rewritten.push("@@".to_string());
    }
    for line in lines.iter().skip(2).take(lines.len().saturating_sub(3)) {
        if line.trim().is_empty() && rewritten.last().is_some_and(|last| last == "@@") {
            continue;
        }
        rewritten.push((*line).to_string());
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
}

fn normalize_taskspace_unwrapped_apply_patch(patch: &str) -> Option<String> {
    let lines = patch.lines().collect::<Vec<_>>();
    if lines.is_empty()
        || lines.first() == Some(&"*** Begin Patch")
        || !lines
            .iter()
            .any(|line| line.starts_with("*** Update File: ") || line.starts_with("*** Add File: "))
        || !lines
            .iter()
            .any(|line| line.starts_with('-') || line.starts_with('+'))
    {
        return None;
    }

    let mut rewritten = Vec::with_capacity(lines.len() + 2);
    rewritten.push("*** Begin Patch".to_string());
    for line in lines {
        if line == "*** Begin Patch" || line == "*** End Patch" {
            continue;
        }
        if let Some(path) = line.strip_prefix("*** Update File: ") {
            let path = path.trim();
            let path = normalize_taskspace_relative_patch_path(path);
            rewritten.push(format!("*** Update File: {path}"));
        } else {
            rewritten.push(line.to_string());
        }
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
}

fn normalize_taskspace_unified_diff_patch(patch: &str) -> Option<String> {
    let normalized = patch.replace("\r\n", "\n");
    let mut lines = normalized.lines().collect::<Vec<_>>();
    if lines.first() == Some(&"*** Begin Patch") && lines.last() == Some(&"*** End Patch") {
        lines = lines[1..lines.len().saturating_sub(1)].to_vec();
    }
    if lines.is_empty() || !lines.iter().any(|line| line.starts_with("--- ")) {
        return None;
    }

    let mut rewritten = vec!["*** Begin Patch".to_string()];
    let mut index = 0usize;
    let mut converted_files = 0usize;
    while index < lines.len() {
        if !lines[index].starts_with("--- ") {
            index += 1;
            continue;
        }
        let Some(new_line) = lines.get(index + 1) else {
            return None;
        };
        if !new_line.starts_with("+++ ") {
            return None;
        }
        let path = taskspace_unified_diff_new_path(new_line)?;
        let path = normalize_taskspace_relative_patch_path(path);
        rewritten.push(format!("*** Update File: {path}"));
        index += 2;

        let mut saw_hunk = false;
        while index < lines.len() {
            if lines[index].starts_with("--- ")
                && lines
                    .get(index + 1)
                    .is_some_and(|candidate| candidate.starts_with("+++ "))
            {
                break;
            }
            if lines[index].starts_with("@@") {
                saw_hunk = true;
            }
            rewritten.push(normalize_taskspace_unified_hunk_line(lines[index]));
            index += 1;
        }
        if !saw_hunk {
            return None;
        }
        converted_files += 1;
    }
    if converted_files == 0 {
        return None;
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
}

fn taskspace_unified_diff_new_path(line: &str) -> Option<&str> {
    let path = line.strip_prefix("+++ ")?.trim();
    let path = path.strip_prefix("b/").unwrap_or(path);
    if path.is_empty() || path == "/dev/null" {
        None
    } else {
        Some(path)
    }
}

fn normalize_taskspace_relative_patch_path(path: &str) -> String {
    normalize_taskspace_relative_patch_path_from(Path::new("."), path)
}

fn normalize_taskspace_relative_patch_path_from(root: &Path, path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    let normalized = normalized
        .strip_prefix("b/")
        .or_else(|| normalized.strip_prefix("a/"))
        .unwrap_or(&normalized)
        .trim_matches('/');
    if normalized.is_empty() {
        return normalized.to_string();
    }
    if root.join(normalized).exists() {
        return normalized.to_string();
    }
    let workspace_stripped = taskspace_strip_common_workspace_patch_prefix(normalized);
    if workspace_stripped != normalized && root.join(workspace_stripped).exists() {
        return workspace_stripped.to_string();
    }
    resolve_unique_existing_relative_path_from(root, normalized)
        .or_else(|| {
            (workspace_stripped != normalized)
                .then(|| resolve_unique_existing_relative_path_from(root, workspace_stripped))
                .flatten()
        })
        .unwrap_or_else(|| workspace_stripped.to_string())
}

fn normalize_taskspace_unified_hunk_line(line: &str) -> String {
    let Some(rest) = line.strip_prefix("@@") else {
        return line.to_string();
    };
    let Some((_, trailing)) = rest.split_once("@@") else {
        return "@@".to_string();
    };
    let trailing = trailing.trim();
    if trailing.is_empty() {
        "@@".to_string()
    } else {
        format!("@@ {trailing}")
    }
}

fn taskspace_unified_diff_add_targets_existing_files(patch: &str) -> Vec<String> {
    let normalized = patch.replace("\r\n", "\n");
    let mut lines = normalized.lines().collect::<Vec<_>>();
    if lines.first() == Some(&"*** Begin Patch") && lines.last() == Some(&"*** End Patch") {
        lines = lines[1..lines.len().saturating_sub(1)].to_vec();
    }

    let mut targets = Vec::new();
    let mut index = 0usize;
    while index + 1 < lines.len() {
        let old_line = lines[index];
        let new_line = lines[index + 1];
        if !old_line.starts_with("--- ") || !new_line.starts_with("+++ ") {
            index += 1;
            continue;
        }
        if taskspace_unified_diff_old_path_is_dev_null(old_line)
            && let Some(path) = taskspace_unified_diff_new_path(new_line)
        {
            let normalized_path = path.replace('\\', "/");
            if Path::new(&normalized_path).exists() {
                targets.push(normalized_path);
            }
        }
        index += 2;
    }
    targets.sort();
    targets.dedup();
    targets
}

fn taskspace_unified_diff_old_path_is_dev_null(line: &str) -> bool {
    line.strip_prefix("--- ")
        .map(str::trim)
        .is_some_and(|path| path == "/dev/null")
}

fn taskspace_apply_patch_missing_unified_header_target(patch: &str) -> bool {
    let normalized = patch.replace("\r\n", "\n");
    let mut lines = normalized.lines().collect::<Vec<_>>();
    if lines.first() == Some(&"*** Begin Patch") && lines.last() == Some(&"*** End Patch") {
        lines = lines[1..lines.len().saturating_sub(1)].to_vec();
    }

    let mut index = 0usize;
    while index + 1 < lines.len() {
        let old_line = lines[index];
        let new_line = lines[index + 1];
        if taskspace_line_looks_unified_old_file_header(old_line)
            && taskspace_line_looks_unified_new_file_header(new_line)
        {
            let old_target = taskspace_unified_header_target(old_line, "---");
            let new_target = taskspace_unified_header_target(new_line, "+++");
            if old_target.is_some_and(str::is_empty) || new_target.is_some_and(str::is_empty) {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn taskspace_unified_header_target<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.trim().strip_prefix(marker).map(str::trim)
}

fn rewrite_taskspace_apply_patch_unique_update_paths(patch: &str) -> String {
    let lines = patch.lines().collect::<Vec<_>>();
    let mut rewritten = Vec::with_capacity(lines.len());
    let mut changed = false;
    for line in lines {
        let Some(path) = line.strip_prefix("*** Update File: ") else {
            rewritten.push(line.to_string());
            continue;
        };
        let candidate = normalize_taskspace_relative_patch_path(path.trim());
        if candidate != path.trim() {
            rewritten.push(format!("*** Update File: {candidate}"));
            changed = true;
        } else {
            rewritten.push(line.to_string());
        }
    }
    if changed {
        rewritten.join("\n") + "\n"
    } else {
        patch.to_string()
    }
}

fn taskspace_apply_patch_missing_mandatory_targets(
    patch: &str,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> Option<Vec<String>> {
    let required = taskspace_uncovered_mandatory_artifacts(snapshot);
    if required.is_empty() {
        return None;
    }
    let patch_targets = taskspace_apply_patch_declared_targets(patch)
        .into_iter()
        .map(|target| normalize_taskspace_patch_target_key(&target))
        .collect::<HashSet<_>>();
    let missing = required
        .into_iter()
        .filter(|target| {
            let required_key = normalize_taskspace_patch_target_key(target);
            !patch_targets.iter().any(|patch_target| {
                taskspace_patch_target_covers_required(patch_target, &required_key)
            })
        })
        .collect::<Vec<_>>();
    (!missing.is_empty()).then_some(missing)
}

fn taskspace_uncovered_mandatory_artifacts(
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> Vec<String> {
    snapshot
        .current_node_uncovered_mandatory_evidence
        .iter()
        .filter_map(|item| {
            item.split_once(" (")
                .map(|(artifact, _)| artifact)
                .or(Some(item))
        })
        .map(str::trim)
        .filter(|artifact| !artifact.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>()
}

fn taskspace_apply_patch_declared_targets(patch: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for line in patch.lines() {
        let target = line
            .strip_prefix("*** Update File: ")
            .or_else(|| line.strip_prefix("*** Add File: "))
            .map(str::trim);
        if let Some(target) = target
            && !target.is_empty()
            && !targets.iter().any(|existing| existing == target)
        {
            targets.push(target.to_string());
        }
    }
    targets
}

fn taskspace_apply_patch_mixed_native_unified_targets(patch: &str) -> Vec<String> {
    let has_native_target = patch
        .lines()
        .any(|line| line.starts_with("*** Update File: ") || line.starts_with("*** Add File: "));
    if !has_native_target {
        return Vec::new();
    }
    let has_unified_marker = patch.lines().any(|line| {
        line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@ -")
            || line.starts_with("@@ +")
    });
    if !has_unified_marker {
        return Vec::new();
    }
    taskspace_apply_patch_declared_targets(patch)
}

fn taskspace_apply_patch_native_update_with_unified_file_header_targets(
    patch: &str,
) -> Vec<String> {
    let mut targets = Vec::new();
    let mut current_target: Option<String> = None;
    let mut current_has_unified_file_header = false;

    let finish_section = |targets: &mut Vec<String>,
                          current_target: &mut Option<String>,
                          current_has_unified_file_header: &mut bool| {
        if *current_has_unified_file_header
            && let Some(target) = current_target.take()
            && !targets.iter().any(|existing| existing == &target)
        {
            targets.push(target);
        } else {
            current_target.take();
        }
        *current_has_unified_file_header = false;
    };

    for line in patch.lines() {
        if line == "*** End Patch"
            || line.starts_with("*** Add File: ")
            || line.starts_with("*** Delete File: ")
        {
            finish_section(
                &mut targets,
                &mut current_target,
                &mut current_has_unified_file_header,
            );
            continue;
        }
        if let Some(target) = line.strip_prefix("*** Update File: ") {
            finish_section(
                &mut targets,
                &mut current_target,
                &mut current_has_unified_file_header,
            );
            current_target = Some(target.trim().to_string());
            continue;
        }
        if current_target.is_some()
            && (taskspace_line_looks_unified_old_file_header(line)
                || taskspace_line_looks_unified_new_file_header(line))
        {
            current_has_unified_file_header = true;
        }
    }
    finish_section(
        &mut targets,
        &mut current_target,
        &mut current_has_unified_file_header,
    );
    targets
}

fn taskspace_validation_rework_replacement_required_targets(
    targets: &[String],
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> Vec<String> {
    if snapshot.node_kind.as_deref() != Some("implement_solution")
        || snapshot.current_node_validation_rework_artifacts.is_empty()
    {
        return Vec::new();
    }
    targets
        .iter()
        .filter(|target| {
            snapshot
                .current_node_validation_rework_artifacts
                .iter()
                .any(|artifact| artifact == *target)
        })
        .cloned()
        .collect()
}

fn taskspace_apply_patch_malformed_native_operation_targets(patch: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for line in patch.lines() {
        if let Some(target) = taskspace_malformed_native_patch_operation_target(line)
            && !targets.iter().any(|existing| existing == &target)
        {
            targets.push(target);
        }
    }
    targets
}

fn taskspace_apply_patch_native_hunk_header_targets(patch: &str) -> Vec<String> {
    taskspace_apply_patch_malformed_native_operation_targets(patch)
}

fn taskspace_malformed_native_patch_operation_target(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let target = trimmed
        .strip_prefix("--- Update File: ")
        .or_else(|| trimmed.strip_prefix("--- Add File: "))
        .or_else(|| trimmed.strip_prefix("--- Delete File: "))?
        .trim();
    (!target.is_empty()).then(|| target.to_string())
}

fn taskspace_apply_patch_unanchored_update_targets(patch: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut current_target: Option<String> = None;
    let mut saw_added_line = false;
    let mut saw_deleted_line = false;
    let mut saw_anchor_line = false;
    let mut saw_section_content = false;

    let finish_section = |targets: &mut Vec<String>,
                          current_target: &mut Option<String>,
                          saw_added_line: &mut bool,
                          saw_deleted_line: &mut bool,
                          saw_anchor_line: &mut bool,
                          saw_section_content: &mut bool| {
        if let Some(target) = current_target.take()
            && ((*saw_added_line && !*saw_anchor_line)
                || (*saw_section_content && !*saw_added_line && !*saw_deleted_line))
            && !targets.iter().any(|existing| existing == &target)
        {
            targets.push(target);
        }
        *saw_added_line = false;
        *saw_deleted_line = false;
        *saw_anchor_line = false;
        *saw_section_content = false;
    };

    for line in patch.lines() {
        if let Some(target) = line.strip_prefix("*** Update File: ").map(str::trim) {
            finish_section(
                &mut targets,
                &mut current_target,
                &mut saw_added_line,
                &mut saw_deleted_line,
                &mut saw_anchor_line,
                &mut saw_section_content,
            );
            current_target = (!target.is_empty()).then(|| target.to_string());
            continue;
        }

        if line.starts_with("*** Add File: ")
            || line.starts_with("*** Delete File: ")
            || line.starts_with("*** End Patch")
        {
            finish_section(
                &mut targets,
                &mut current_target,
                &mut saw_added_line,
                &mut saw_deleted_line,
                &mut saw_anchor_line,
                &mut saw_section_content,
            );
            continue;
        }

        if current_target.is_some() {
            if !line.trim().is_empty() {
                saw_section_content = true;
            }
            if line.starts_with('+') && !line.starts_with("+++") {
                saw_added_line = true;
            } else if line.starts_with('-') && !line.starts_with("---") {
                saw_deleted_line = true;
                saw_anchor_line = true;
            } else if line.starts_with(' ') {
                saw_anchor_line = true;
            }
        }
    }

    finish_section(
        &mut targets,
        &mut current_target,
        &mut saw_added_line,
        &mut saw_deleted_line,
        &mut saw_anchor_line,
        &mut saw_section_content,
    );
    targets
}

fn normalize_taskspace_patch_target_key(target: &str) -> String {
    target
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn taskspace_patch_target_covers_required(patch_target: &str, required: &str) -> bool {
    patch_target == required || patch_target.ends_with(&format!("/{required}"))
}

fn resolve_unique_existing_relative_path_from(root: &Path, path: &str) -> Option<String> {
    if path.is_empty() || root.join(path).exists() {
        return None;
    }
    if path.contains('/') || path.contains('\\') {
        let normalized = path.trim().replace('\\', "/");
        let src_prefixed = format!("src/{normalized}");
        if root.join(&src_prefixed).exists() {
            return Some(src_prefixed);
        }
        let mut matches = Vec::new();
        collect_unique_suffix_matches(root, root, &normalized, &mut matches, 2_000);
        if matches.len() != 1 {
            return None;
        }
        let relative = matches.pop()?;
        return relative.to_str().map(|value| value.replace('\\', "/"));
    }
    let mut matches = Vec::new();
    collect_unique_basename_matches(root, root, path, &mut matches, 2_000);
    if matches.len() != 1 {
        if matches.is_empty() {
            return Some(format!("src/{path}"));
        }
        return None;
    }
    let relative = matches.pop()?;
    relative.to_str().map(|value| value.replace('\\', "/"))
}

fn collect_unique_basename_matches(
    root: &Path,
    dir: &Path,
    basename: &str,
    matches: &mut Vec<PathBuf>,
    remaining: usize,
) -> usize {
    if remaining == 0 || matches.len() > 1 {
        return remaining;
    }
    let mut remaining = remaining;
    let Ok(entries) = fs::read_dir(dir) else {
        return remaining;
    };
    for entry in entries.flatten() {
        if remaining == 0 || matches.len() > 1 {
            break;
        }
        remaining = remaining.saturating_sub(1);
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git" || file_name == "target" || file_name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            remaining = collect_unique_basename_matches(root, &path, basename, matches, remaining);
        } else if file_name == basename
            && let Ok(relative) = path.strip_prefix(root)
        {
            matches.push(relative.to_path_buf());
        }
    }
    remaining
}

fn collect_unique_suffix_matches(
    root: &Path,
    dir: &Path,
    suffix: &str,
    matches: &mut Vec<PathBuf>,
    remaining: usize,
) -> usize {
    if remaining == 0 || matches.len() > 1 {
        return remaining;
    }
    let mut remaining = remaining;
    let Ok(entries) = fs::read_dir(dir) else {
        return remaining;
    };
    for entry in entries.flatten() {
        if remaining == 0 || matches.len() > 1 {
            break;
        }
        remaining = remaining.saturating_sub(1);
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git" || file_name == "target" || file_name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            remaining = collect_unique_suffix_matches(root, &path, suffix, matches, remaining);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let relative_text = relative.to_string_lossy().replace('\\', "/");
            if relative_text.ends_with(&format!("/{suffix}")) {
                matches.push(relative.to_path_buf());
            }
        }
    }
    remaining
}

fn taskspace_action_to_tool_call(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> Result<Option<ToolCall>, String> {
    let action_name = action.action.as_str();
    if action_name == "final_answer" {
        return Ok(None);
    }
    if !taskspace_action_allowed_for_node(action_name, snapshot.node_kind.as_deref()) {
        return Err(format!(
            "node_policy_violation:{}:{}",
            snapshot.node_kind.as_deref().unwrap_or("unknown"),
            action_name
        ));
    }
    if taskspace_snapshot_requires_implementation_edit(snapshot)
        && taskspace_action_is_read_or_search(action_name)
        && !taskspace_action_reads_validation_rework_artifact(action, snapshot)
    {
        return Err(format!(
            "node_policy_violation:implement_solution:{action_name}:implementation_needs_edit"
        ));
    }
    if let (Some(expected), Some(actual)) = (snapshot.node_id.as_deref(), action.node_id.as_deref())
        && expected != actual
    {
        return Err("node_id_mismatch".to_string());
    }
    let args = &action.args;
    let call_id = format!(
        "taskspace-action-contract-{}-{}",
        snapshot.request_count.saturating_add(1),
        action_name
    );
    match action_name {
        "list_files" => {
            let path = taskspace_action_arg_string(args, "path").unwrap_or_else(|| ".".to_string());
            let arguments = serde_json::json!({
                "command": format!("rg --files {}", path),
                "timeout_ms": 10000,
            })
            .to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("shell_command"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        "search" => {
            let pattern = taskspace_action_arg_string(args, "pattern")
                .ok_or_else(|| "missing_search_pattern".to_string())?;
            let path = taskspace_action_arg_string(args, "path").unwrap_or_else(|| ".".to_string());
            let arguments = serde_json::json!({
                "command": format!("rg --line-number --no-heading -- {:?} {}", pattern, path),
                "timeout_ms": 10000,
            })
            .to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("shell_command"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        "read_file" => {
            let path = taskspace_action_arg_string(args, "path")
                .ok_or_else(|| "missing_read_file_path".to_string())?;
            let arguments = serde_json::json!({
                "command": taskspace_read_file_command(&path),
                "timeout_ms": 10000,
            })
            .to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("shell_command"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        "apply_patch" => {
            let patch = taskspace_action_arg_string(args, "patch")
                .ok_or_else(|| "missing_apply_patch_patch".to_string())?;
            if taskspace_apply_patch_missing_unified_header_target(&patch) {
                return Err("apply_patch_mixed_native_unified:(missing patch target)".to_string());
            }
            let existing_add_targets = taskspace_unified_diff_add_targets_existing_files(&patch);
            if !existing_add_targets.is_empty() {
                return Err(format!(
                    "apply_patch_existing_file_as_add:{}",
                    existing_add_targets.join(",")
                ));
            }
            let malformed_native_operation_targets =
                taskspace_apply_patch_malformed_native_operation_targets(&patch);
            if !malformed_native_operation_targets.is_empty() {
                return Err(format!(
                    "apply_patch_native_hunk_header:{}",
                    malformed_native_operation_targets.join(",")
                ));
            }
            let native_hunk_header_targets =
                taskspace_apply_patch_native_hunk_header_targets(&patch);
            if !native_hunk_header_targets.is_empty() {
                return Err(format!(
                    "apply_patch_native_hunk_header:{}",
                    native_hunk_header_targets.join(",")
                ));
            }
            let native_update_unified_header_targets =
                taskspace_apply_patch_native_update_with_unified_file_header_targets(&patch);
            if !native_update_unified_header_targets.is_empty() {
                let replacement_required_targets =
                    taskspace_validation_rework_replacement_required_targets(
                        &native_update_unified_header_targets,
                        snapshot,
                    );
                if !replacement_required_targets.is_empty() {
                    return Err(format!(
                        "apply_patch_replacement_required:{}",
                        replacement_required_targets.join(",")
                    ));
                }
                return Err(format!(
                    "apply_patch_mixed_native_unified:{}",
                    native_update_unified_header_targets.join(",")
                ));
            }
            let patch = normalize_taskspace_unified_diff_patch(&patch)
                .unwrap_or_else(|| normalize_taskspace_apply_patch(&patch));
            let native_hunk_header_targets =
                taskspace_apply_patch_native_hunk_header_targets(&patch);
            if !native_hunk_header_targets.is_empty() {
                return Err(format!(
                    "apply_patch_native_hunk_header:{}",
                    native_hunk_header_targets.join(",")
                ));
            }
            let mixed_native_unified_targets =
                taskspace_apply_patch_mixed_native_unified_targets(&patch);
            if !mixed_native_unified_targets.is_empty() {
                return Err(format!(
                    "apply_patch_mixed_native_unified:{}",
                    mixed_native_unified_targets.join(",")
                ));
            }
            let unanchored_update_targets = taskspace_apply_patch_unanchored_update_targets(&patch);
            if !unanchored_update_targets.is_empty() {
                return Err(format!(
                    "apply_patch_unanchored_update:{}",
                    unanchored_update_targets.join(",")
                ));
            }
            if let Some(missing_targets) =
                taskspace_apply_patch_missing_mandatory_targets(&patch, snapshot)
            {
                return Err(format!(
                    "apply_patch_missing_mandatory_evidence_targets:{}",
                    missing_targets.join(",")
                ));
            }
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("apply_patch"),
                call_id,
                payload: ToolPayload::Custom { input: patch },
            }))
        }
        "run_test" => {
            let command = taskspace_action_arg_string(args, "command")
                .ok_or_else(|| "missing_run_test_command".to_string())?;
            let command = normalize_taskspace_action_contract_test_command(&command);
            let arguments = serde_json::json!({
                "command": command,
                "timeout_ms": taskspace_action_arg_u64(args, "timeout_ms", 120000),
            })
            .to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("shell_command"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        "taskspace_control" | "state_commit" => {
            let args = if action_name == "state_commit" {
                let mut normalized = args.clone();
                if let Some(root) = normalized.as_object_mut() {
                    root.entry("action".to_string())
                        .or_insert_with(|| serde_json::Value::String("state_commit".to_string()));
                }
                normalized
            } else {
                let mut normalized = args.clone();
                normalize_taskspace_start_task_rationale_into_objective(
                    &mut normalized,
                    action.rationale.as_deref(),
                );
                normalized
            };
            let arguments =
                normalize_taskspace_action_contract_control_args(&args, Some(snapshot))?
                    .to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("taskspace_control"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        "blocked" => {
            let Some(node_id) = snapshot.node_id.as_deref() else {
                return Ok(None);
            };
            let blocker_summary = taskspace_action_arg_string(args, "reason")
                .ok_or_else(|| "missing_blocked_reason".to_string())?;
            let arguments = serde_json::json!({
                "action": "block_node",
                "node_id": node_id,
                "blocker_summary": blocker_summary,
            })
            .to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("taskspace_control"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        _ => Err(format!("unsupported_action:{action_name}")),
    }
}

fn taskspace_action_reads_validation_rework_artifact(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> bool {
    if action.action != "read_file" || snapshot.current_node_validation_rework_artifacts.is_empty()
    {
        return false;
    }
    let Some(path) = taskspace_action_arg_string(&action.args, "path") else {
        return false;
    };
    let requested = taskspace_normalize_apply_patch_target(&path).to_ascii_lowercase();
    snapshot
        .current_node_validation_rework_artifacts
        .iter()
        .any(|artifact| {
            taskspace_normalize_apply_patch_target(artifact).to_ascii_lowercase() == requested
        })
}

fn taskspace_closed_validation_rework_read_reject_reason(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    visible_validation_rework_target_read: bool,
) -> Option<String> {
    if !visible_validation_rework_target_read {
        return None;
    }
    if snapshot.node_kind.as_deref() != Some("implement_solution")
        || snapshot.current_node_has_successful_edit
        || !taskspace_action_reads_validation_rework_artifact(action, snapshot)
    {
        return None;
    }
    Some(format!(
        "validation_rework_closed_action_space_read_disallowed:{}",
        action.action
    ))
}

fn taskspace_read_file_command(path: &str) -> String {
    const MAX_LINES: usize = 240;
    if cfg!(windows) {
        let summary_path = path.replace('"', "`\"");
        format!(
            "Get-Content -LiteralPath {path:?} -TotalCount {MAX_LINES}; \
$TaskSpaceReadCount = @(Get-Content -LiteralPath {path:?} -TotalCount {}).Count; \
$TaskSpaceReadLines = [Math]::Min($TaskSpaceReadCount, {MAX_LINES}); \
$TaskSpaceReadEof = if ($TaskSpaceReadCount -le {MAX_LINES}) {{ 'true' }} else {{ 'false' }}; \
Write-Output \"TaskSpaceReadFileSummaryV1: path={summary_path} lines_read=$TaskSpaceReadLines eof_reached=$TaskSpaceReadEof max_lines={MAX_LINES}\"",
            MAX_LINES + 1,
        )
    } else {
        let sed_args = vec![
            "sed".to_string(),
            "-n".to_string(),
            format!("1,{MAX_LINES}p"),
            "--".to_string(),
            path.to_string(),
        ];
        let summary_script = format!(
            "NR == {} {{ truncated = 1; exit }} {{ lines = NR }} END {{ eof = truncated ? \"false\" : \"true\"; if ({MAX_LINES} < lines) lines = {MAX_LINES}; printf \"\\nTaskSpaceReadFileSummaryV1: path=%s lines_read=%d eof_reached=%s max_lines={MAX_LINES}\\n\", FILENAME, lines + 0, eof }}",
            MAX_LINES + 1,
        );
        let awk_args = vec!["awk".to_string(), summary_script, path.to_string()];
        format!(
            "{} && {}",
            codex_shell_command::parse_command::shlex_join(&sed_args),
            codex_shell_command::parse_command::shlex_join(&awk_args)
        )
    }
}

fn taskspace_missing_fact_source_bootstrap_command(artifacts: &[String]) -> String {
    let commands = artifacts
        .iter()
        .take(4)
        .map(|artifact| {
            if cfg!(windows) {
                format!(
                    "Write-Output {:?}; {}",
                    format!("===== {artifact}"),
                    taskspace_read_file_command(artifact)
                )
            } else {
                let header_command = codex_shell_command::parse_command::shlex_join(&[
                    "printf".to_string(),
                    "===== %s\\n".to_string(),
                    artifact.to_string(),
                ]);
                format!(
                    "{}; {}",
                    header_command,
                    taskspace_read_file_command(artifact)
                )
            }
        })
        .collect::<Vec<_>>();
    if cfg!(windows) {
        commands.join("; ")
    } else {
        commands.join("\n")
    }
}

fn taskspace_repeated_blocked_inspect_bootstrap_command() -> &'static str {
    if cfg!(windows) {
        TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_COMMAND_WINDOWS
    } else {
        TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_COMMAND_UNIX
    }
}

fn taskspace_action_contract_visible_text(raw_text: &str) -> Option<String> {
    let action = parse_taskspace_action_v1(raw_text).ok()?;
    if action.action == "final_answer" {
        return taskspace_action_final_message(&action);
    }
    None
}

fn normalize_taskspace_action_contract_control_args(
    args: &serde_json::Value,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) -> Result<serde_json::Value, String> {
    let mut normalized = args.clone();
    let Some(root) = normalized.as_object_mut() else {
        return Err(TASKSPACE_CONTROL_ARGS_NOT_OBJECT_ERROR.to_string());
    };
    let inner_action = canonicalize_taskspace_control_action_arg(root)?;
    if inner_action == "state_commit" {
        normalize_taskspace_action_contract_state_commit_args(root, snapshot);
        return Ok(normalized);
    }
    if inner_action == "record_fact" {
        normalize_taskspace_action_contract_record_fact_as_state_commit(root, snapshot);
        return Ok(normalized);
    }
    normalize_taskspace_action_contract_lifecycle_args(root, &inner_action, snapshot);
    Ok(normalized)
}

fn normalize_taskspace_start_task_rationale_into_objective(
    args: &mut serde_json::Value,
    rationale: Option<&str>,
) {
    let Some(rationale) = rationale.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let Some(root) = args.as_object_mut() else {
        return;
    };
    if taskspace_control_action_from_root(root) != Some("start_task") {
        return;
    }
    let objective_key = if root.contains_key("objective") {
        "objective"
    } else if root.contains_key("task_objective") {
        "task_objective"
    } else {
        "objective"
    };
    match root.get_mut(objective_key) {
        Some(serde_json::Value::String(existing)) => {
            if !existing.contains(rationale) {
                let merged = if existing.trim().is_empty() {
                    rationale.to_string()
                } else {
                    format!("{} Rationale: {}", existing.trim(), rationale)
                };
                *existing = merged;
            }
        }
        Some(_) => {}
        None => {
            root.insert(
                objective_key.to_string(),
                serde_json::Value::String(rationale.to_string()),
            );
        }
    }
}

fn normalize_taskspace_action_contract_lifecycle_args(
    root: &mut serde_json::Map<String, serde_json::Value>,
    inner_action: &str,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) {
    let current_node_id = snapshot.and_then(|snapshot| snapshot.node_id.as_deref());
    match inner_action {
        "create_node" => {
            move_taskspace_json_alias(root, "child_kind", "kind");
            move_taskspace_json_alias(root, "node_kind", "kind");
            move_taskspace_json_alias(root, "child_name", "title");
            move_taskspace_json_alias(root, "label", "title");
            move_taskspace_json_alias(root, "name", "title");
            move_taskspace_json_alias(root, "node_title", "title");
            move_taskspace_json_alias(root, "objective", "context_summary");
            move_taskspace_json_alias(root, "node_context_summary", "context_summary");
            move_taskspace_json_alias(root, "summary", "context_summary");
            move_taskspace_json_alias(root, "description", "context_summary");
            root.entry("kind".to_string())
                .or_insert_with(|| serde_json::Value::String("inspect_code_context".to_string()));
            let kind = root
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("inspect_code_context")
                .to_string();
            root.entry("title".to_string()).or_insert_with(|| {
                serde_json::Value::String(default_taskspace_action_node_title(&kind).to_string())
            });
            let title = root
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("TaskSpace node")
                .to_string();
            root.entry("context_summary".to_string())
                .or_insert_with(|| {
                    serde_json::Value::String(format!(
                        "{title}. Continue the current TaskSpace task."
                    ))
                });
            if snapshot.is_some_and(|snapshot| snapshot.node_id.is_none()) {
                root.entry("bind_current".to_string())
                    .or_insert_with(|| serde_json::Value::Bool(true));
            }
        }
        "bind_node" if !root.contains_key("node_id") => {
            if root.contains_key("node_kind")
                || root.contains_key("kind")
                || root.contains_key("child_kind")
                || root.contains_key("objective")
                || root.contains_key("label")
                || root.contains_key("description")
            {
                root.insert(
                    "action".to_string(),
                    serde_json::Value::String("create_node".to_string()),
                );
                normalize_taskspace_action_contract_lifecycle_args(root, "create_node", snapshot);
            }
        }
        "bind_node" | "block_node" | "finish_node" => {
            if !root.contains_key("node_id")
                && let Some(node_id) = current_node_id
            {
                root.insert(
                    "node_id".to_string(),
                    serde_json::Value::String(node_id.to_string()),
                );
            }
            let effective_action = if inner_action == "finish_node"
                && taskspace_finish_node_should_block_failed_validation(root, snapshot)
            {
                root.insert(
                    "action".to_string(),
                    serde_json::Value::String("block_node".to_string()),
                );
                "block_node"
            } else {
                inner_action
            };
            if effective_action == "block_node" {
                move_taskspace_json_alias(root, "reason", "blocker_summary");
                move_taskspace_json_alias(root, "summary", "blocker_summary");
                move_taskspace_json_alias(root, "result", "blocker_summary");
                if !root.contains_key("blocker_summary") {
                    root.insert(
                        "blocker_summary".to_string(),
                        serde_json::Value::String(taskspace_failed_validation_blocker_summary(
                            root,
                        )),
                    );
                }
            }
            if effective_action == "finish_node" {
                move_taskspace_json_alias(root, "result", "result_summary");
                move_taskspace_json_alias(root, "summary", "result_summary");
                move_taskspace_json_alias(root, "reason", "result_summary");
                root.entry("result_summary".to_string()).or_insert_with(|| {
                    serde_json::Value::String(
                        "TaskSpace node completed with the inspected evidence.".to_string(),
                    )
                });
                if snapshot.is_some_and(|snapshot| {
                    snapshot.node_kind.as_deref() == Some("implement_solution")
                }) {
                    root.entry("next_node_kind".to_string())
                        .or_insert_with(|| serde_json::Value::String("smoke_test".to_string()));
                    if root
                        .get("next_node_kind")
                        .and_then(serde_json::Value::as_str)
                        == Some("smoke_test")
                    {
                        root.entry("next_node_title".to_string())
                            .or_insert_with(|| {
                                serde_json::Value::String("Run focused validation".to_string())
                            });
                        root.entry("next_node_context_summary".to_string())
                            .or_insert_with(|| {
                                serde_json::Value::String(
                                    "Run the focused test command after the implementation edit."
                                        .to_string(),
                                )
                            });
                        if !root.contains_key("next_dependency_node_ids")
                            && let Some(node_id) = current_node_id
                        {
                            root.insert(
                                "next_dependency_node_ids".to_string(),
                                serde_json::json!([node_id]),
                            );
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn taskspace_finish_node_should_block_failed_validation(
    root: &serde_json::Map<String, serde_json::Value>,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) -> bool {
    if !snapshot.is_some_and(|snapshot| {
        matches!(
            snapshot.node_kind.as_deref(),
            Some("smoke_test" | "regression_test")
        )
    }) {
        return false;
    }
    taskspace_json_contains_failed_validity(root.get("result_validities"))
        || taskspace_json_contains_failed_validity(root.get("validities"))
        || taskspace_json_contains_failed_validity(root.get("results"))
        || taskspace_json_contains_failed_validity(root.get("result_validity"))
        || taskspace_json_contains_failed_validity(root.get("validity"))
        || taskspace_json_contains_failed_validity(root.get("status"))
        || taskspace_json_contains_failed_validity(root.get("outcome"))
        || taskspace_json_claims_failed_validation(root.get("blocker_summary"))
        || taskspace_json_claims_failed_validation(root.get("result_summary"))
        || taskspace_json_claims_failed_validation(root.get("summary"))
        || taskspace_json_claims_failed_validation(root.get("result"))
        || taskspace_json_claims_failed_validation(root.get("reason"))
}

fn taskspace_json_contains_failed_validity(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else {
        return false;
    };
    match value {
        serde_json::Value::String(value) => taskspace_validity_is_failed(value),
        serde_json::Value::Array(items) => items
            .iter()
            .any(|item| taskspace_json_contains_failed_validity(Some(item))),
        serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
            taskspace_validity_is_failed(key)
                || taskspace_json_contains_failed_validity(Some(value))
        }),
        _ => false,
    }
}

fn taskspace_validity_is_failed(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "fail" | "failed" | "failure" | "invalid" | "questioned" | "blocked"
    )
}

fn taskspace_json_claims_failed_validation(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else {
        return false;
    };
    match value {
        serde_json::Value::String(value) => taskspace_text_claims_failed_validation(value),
        serde_json::Value::Array(items) => items
            .iter()
            .any(|item| taskspace_json_claims_failed_validation(Some(item))),
        serde_json::Value::Object(map) => map
            .values()
            .any(|value| taskspace_json_claims_failed_validation(Some(value))),
        _ => false,
    }
}

fn taskspace_text_claims_failed_validation(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    taskspace_validity_is_failed(&lower)
        || lower.contains("test failed")
        || lower.contains("tests failed")
        || lower.contains("validation failed")
        || lower.contains("validator failed")
        || lower.contains("schema validation failed")
        || lower.contains("exit code 1")
        || lower.contains("indentationerror")
        || lower.contains("syntaxerror")
        || lower.contains("traceback")
}

fn taskspace_failed_validation_blocker_summary(
    root: &serde_json::Map<String, serde_json::Value>,
) -> String {
    for key in [
        "blocker_summary",
        "result_summary",
        "summary",
        "result",
        "reason",
    ] {
        if let Some(value) = root.get(key).and_then(serde_json::Value::as_str) {
            let value = value.trim();
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }
    if let Some(decisions) = root.get("decisions").and_then(serde_json::Value::as_array) {
        let summary = decisions
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !summary.is_empty() {
            return summary;
        }
    }
    "Validation failed; route to implementation rework.".to_string()
}

fn default_taskspace_action_node_title(kind: &str) -> &'static str {
    match kind {
        "inspect_code_context" => "Inspect code context",
        "implement_solution" => "Apply implementation",
        "smoke_test" => "Run focused validation",
        "regression_test" => "Run regression validation",
        "final_synthesis" => "Summarize final outcome",
        _ => "TaskSpace node",
    }
}

fn move_taskspace_json_alias(
    root: &mut serde_json::Map<String, serde_json::Value>,
    alias: &str,
    canonical: &str,
) {
    if root.contains_key(canonical) {
        root.remove(alias);
        return;
    }
    if let Some(value) = root.remove(alias) {
        root.insert(canonical.to_string(), value);
    }
}

fn normalize_taskspace_action_contract_record_fact_as_state_commit(
    root: &mut serde_json::Map<String, serde_json::Value>,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) {
    let statement = root
        .remove("statement")
        .or_else(|| root.remove("fact"))
        .or_else(|| root.remove("summary"))
        .and_then(|value| value.as_str().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "TaskSpace compact fact recorded by action contract.".to_string());
    let claim_id = root
        .remove("claim_id")
        .or_else(|| root.remove("id"))
        .and_then(|value| value.as_str().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "claim-action-contract-1".to_string());
    let fact_source_id = root
        .remove("fact_source_id")
        .and_then(|value| value.as_str().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "fact-source-action-contract-1".to_string());
    let active_node_id = snapshot
        .and_then(|snapshot| snapshot.node_id.as_deref())
        .map(str::to_string);

    root.clear();
    root.insert(
        "action".to_string(),
        serde_json::Value::String("state_commit".to_string()),
    );
    root.insert(
        "schema_version".to_string(),
        serde_json::Value::String("taskspace-state-commit-v1".to_string()),
    );
    if let Some(active_node_id) = active_node_id {
        root.insert(
            "active_node_id".to_string(),
            serde_json::Value::String(active_node_id),
        );
    }
    root.insert(
        "fact_sources".to_string(),
        serde_json::json!([{
            "id": fact_source_id.clone(),
            "provenance": "observed_from_environment",
            "description": "Compact action-contract observation from the current TaskSpace context.",
            "evidence_refs": [{ "artifact_ref": "user-request" }]
        }]),
    );
    root.insert(
        "facts".to_string(),
        serde_json::json!([{
            "claim_id": claim_id,
            "statement": statement,
            "evidence_refs": [{ "fact_source_id": fact_source_id }]
        }]),
    );
}

fn normalize_taskspace_action_contract_state_commit_args(
    root: &mut serde_json::Map<String, serde_json::Value>,
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) {
    root.entry("schema_version".to_string())
        .or_insert_with(|| serde_json::Value::String("taskspace-state-commit-v1".to_string()));
    let current_node_id = snapshot.and_then(|snapshot| snapshot.node_id.as_deref());
    if !root.contains_key("active_node_id")
        && let Some(node_id) = current_node_id
    {
        root.insert(
            "active_node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );
    }
    normalize_taskspace_action_contract_state_commit_blockers(root, current_node_id);
    normalize_taskspace_action_contract_state_commit_decisions(root);
}

fn normalize_taskspace_action_contract_state_commit_blockers(
    root: &mut serde_json::Map<String, serde_json::Value>,
    current_node_id: Option<&str>,
) {
    let Some(serde_json::Value::Array(items)) = root.get_mut("blockers") else {
        return;
    };
    let Some(current_node_id) = current_node_id else {
        return;
    };
    for item in items {
        match item {
            serde_json::Value::String(text) => {
                let blocker_summary = text.trim().to_string();
                if !blocker_summary.is_empty() {
                    *item = serde_json::json!({
                        "node_id": current_node_id,
                        "blocker_summary": blocker_summary,
                    });
                }
            }
            serde_json::Value::Object(object) => {
                if !object.contains_key("blocker_summary")
                    && let Some(reason) = object.remove("reason")
                {
                    object.insert("blocker_summary".to_string(), reason);
                }
                object
                    .entry("node_id".to_string())
                    .or_insert_with(|| serde_json::Value::String(current_node_id.to_string()));
            }
            _ => {}
        }
    }
}

fn normalize_taskspace_action_contract_state_commit_decisions(
    root: &mut serde_json::Map<String, serde_json::Value>,
) {
    let Some(serde_json::Value::Array(items)) = root.get_mut("decisions") else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        match item {
            serde_json::Value::String(text) => {
                let decision = text.trim().to_string();
                if !decision.is_empty() {
                    *item = serde_json::json!({
                        "id": format!("decision-{}", index + 1),
                        "decision_kind": "validation",
                        "decision": decision,
                        "rationale": "recorded by compact action-contract state_commit normalization",
                    });
                }
            }
            serde_json::Value::Object(object) => {
                object.entry("id".to_string()).or_insert_with(|| {
                    serde_json::Value::String(format!("decision-{}", index + 1))
                });
                object
                    .entry("decision_kind".to_string())
                    .or_insert_with(|| serde_json::Value::String("validation".to_string()));
                if !object.contains_key("decision")
                    && let Some(summary) = object.remove("summary")
                {
                    object.insert("decision".to_string(), summary);
                }
                object.entry("decision".to_string()).or_insert_with(|| {
                    serde_json::Value::String("TaskSpace state_commit decision".to_string())
                });
                object.entry("rationale".to_string()).or_insert_with(|| {
                    serde_json::Value::String(
                        "recorded by compact action-contract state_commit normalization"
                            .to_string(),
                    )
                });
            }
            _ => {}
        }
    }
}

fn normalize_taskspace_action_contract_test_command(command: &str) -> String {
    let trimmed = command.trim();
    if let Some(normalized) = normalize_taskspace_shell_script_test_command(trimmed) {
        return normalize_taskspace_host_shell_test_command(&normalized);
    }
    for prefix in [
        "pytest ",
        "python -m pytest ",
        "cd src && python -m pytest ",
    ] {
        let Some(rest) = trimmed.strip_prefix(prefix) else {
            continue;
        };
        let rest = rest.trim();
        let Some((file, suffix)) = split_first_shell_word(rest) else {
            continue;
        };
        if file.ends_with(".py") && !file.contains('/') && !file.contains('\\') {
            let suffix = suffix.trim();
            let normalized = if suffix.is_empty() {
                format!("{} tests/{file}", prefix.trim())
            } else {
                format!("{} tests/{file} {suffix}", prefix.trim())
            };
            return normalize_taskspace_host_shell_test_command(&normalized);
        }
        if file.ends_with(".py")
            && !Path::new(file).exists()
            && let Some(resolved) = resolve_unique_test_file_for_missing_pytest_path(file)
        {
            let suffix = suffix.trim();
            let normalized = if suffix.is_empty() {
                format!("{} {resolved}", prefix.trim())
            } else {
                format!("{} {resolved} {suffix}", prefix.trim())
            };
            return normalize_taskspace_host_shell_test_command(&normalized);
        }
    }
    normalize_taskspace_host_shell_test_command(trimmed)
}

fn normalize_taskspace_host_shell_test_command(command: &str) -> String {
    if cfg!(windows) {
        normalize_taskspace_powershell_and_chain(command).unwrap_or_else(|| command.to_string())
    } else {
        command.to_string()
    }
}

fn normalize_taskspace_powershell_and_chain(command: &str) -> Option<String> {
    if let Some(normalized) = normalize_taskspace_powershell_or_chain(command) {
        return Some(normalized);
    }
    let segments = split_top_level_double_ampersand(command)?;
    if segments.len() < 2 {
        return None;
    }
    let mut normalized = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            normalized.push_str("; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; ");
        }
        normalized.push_str(segment.trim());
    }
    Some(normalized)
}

fn normalize_taskspace_powershell_or_chain(command: &str) -> Option<String> {
    let (left, right) = split_top_level_double_pipe_once(command)?;
    let (fallback, tail) = split_top_level_semicolon_once(&right)
        .map(|(fallback, tail)| (fallback, Some(tail)))
        .unwrap_or((right, None));
    if left.trim().is_empty() || fallback.trim().is_empty() {
        return None;
    }
    let mut normalized = format!(
        "{}; if ($LASTEXITCODE -ne 0) {{ {} }}",
        left.trim(),
        fallback.trim()
    );
    if let Some(tail) = tail
        && !tail.trim().is_empty()
    {
        normalized.push_str("; ");
        normalized.push_str(tail.trim());
    }
    Some(normalized)
}

fn split_top_level_double_ampersand(command: &str) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars = command.char_indices().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let (byte_index, ch) = chars[index];
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '&' if !in_single_quote && !in_double_quote => {
                if chars.get(index + 1).is_some_and(|(_, next)| *next == '&') {
                    let left = command[start..byte_index].trim();
                    if left.is_empty() {
                        return None;
                    }
                    segments.push(left.to_string());
                    start = byte_index + 2;
                    index += 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    let tail = command[start..].trim();
    if tail.is_empty() || segments.is_empty() {
        return None;
    }
    segments.push(tail.to_string());
    Some(segments)
}

fn split_top_level_double_pipe_once(command: &str) -> Option<(String, String)> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars = command.char_indices().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let (byte_index, ch) = chars[index];
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '|' if !in_single_quote && !in_double_quote => {
                if chars.get(index + 1).is_some_and(|(_, next)| *next == '|') {
                    let left = command[..byte_index].trim();
                    let right = command[byte_index + 2..].trim();
                    if left.is_empty() || right.is_empty() {
                        return None;
                    }
                    return Some((left.to_string(), right.to_string()));
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn split_top_level_semicolon_once(command: &str) -> Option<(String, String)> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    for (byte_index, ch) in command.char_indices() {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            ';' if !in_single_quote && !in_double_quote => {
                let left = command[..byte_index].trim();
                let right = command[byte_index + 1..].trim();
                if left.is_empty() || right.is_empty() {
                    return None;
                }
                return Some((left.to_string(), right.to_string()));
            }
            _ => {}
        }
    }
    None
}

fn normalize_taskspace_shell_script_test_command(command: &str) -> Option<String> {
    let lower = command.to_ascii_lowercase();
    if lower.starts_with("bash ") || lower.starts_with("sh ") {
        return None;
    }
    if let Some((left, right)) = command.split_once("&&") {
        let right = right.trim();
        if let Some(normalized_right) = normalize_taskspace_shell_script_test_command(right) {
            return Some(format!("{} && {}", left.trim(), normalized_right));
        }
        return None;
    }
    let (file, suffix) = split_first_shell_word(command)?;
    let normalized_file = file.trim_matches('"').trim_matches('\'');
    if !normalized_file.ends_with(".sh") {
        return None;
    }
    if let Some(script) = normalized_file.strip_prefix("./") {
        let suffix = suffix.trim();
        return Some(if suffix.is_empty() {
            format!("bash {script}")
        } else {
            format!("bash {script} {suffix}")
        });
    }
    if normalized_file.starts_with("/app/") {
        let suffix = suffix.trim();
        return Some(if suffix.is_empty() {
            format!("bash {normalized_file}")
        } else {
            format!("bash {normalized_file} {suffix}")
        });
    }
    None
}

fn resolve_unique_test_file_for_missing_pytest_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if !normalized.starts_with("tests/") {
        return None;
    }
    let entries = fs::read_dir("tests").ok()?;
    let matches = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            if path.is_file() && file_name.starts_with("test_") && file_name.ends_with(".py") {
                Some(format!("tests/{file_name}"))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn split_first_shell_word(value: &str) -> Option<(&str, &str)> {
    let value = value.trim_start();
    if value.is_empty() {
        return None;
    }
    if let Some(index) = value.find(char::is_whitespace) {
        Some((&value[..index], &value[index..]))
    } else {
        Some((value, ""))
    }
}

fn taskspace_bootstrap_action_to_tool_call(
    action: &TaskSpaceActionV1,
) -> Result<Option<ToolCall>, String> {
    let action_name = action.action.as_str();
    if !taskspace_action_allowed_for_node(action_name, None) {
        return Err(format!("bootstrap_policy_violation:{action_name}"));
    }
    match action_name {
        "taskspace_control" => Ok(Some(ToolCall {
            tool_name: ToolName::plain("taskspace_control"),
            call_id: "taskspace-action-contract-bootstrap-taskspace_control".to_string(),
            payload: ToolPayload::Function {
                arguments: normalize_taskspace_action_contract_control_args(&action.args, None)?
                    .to_string(),
            },
        })),
        "blocked" => Ok(None),
        _ => Err(format!("unsupported_bootstrap_action:{action_name}")),
    }
}

fn response_item_for_taskspace_action_tool_call(call: &ToolCall) -> ResponseItem {
    match &call.payload {
        ToolPayload::Custom { input } => ResponseItem::CustomToolCall {
            id: None,
            status: None,
            call_id: call.call_id.clone(),
            name: call.tool_name.name.clone(),
            input: input.clone(),
        },
        ToolPayload::Function { arguments } => ResponseItem::FunctionCall {
            id: None,
            name: call.tool_name.name.clone(),
            namespace: call.tool_name.namespace.clone(),
            arguments: arguments.clone(),
            call_id: call.call_id.clone(),
        },
        _ => ResponseItem::FunctionCall {
            id: None,
            name: call.tool_name.name.clone(),
            namespace: call.tool_name.namespace.clone(),
            arguments: call.payload.log_payload().into_owned(),
            call_id: call.call_id.clone(),
        },
    }
}

fn taskspace_action_final_message(action: &TaskSpaceActionV1) -> Option<String> {
    match action.action.as_str() {
        "final_answer" => taskspace_action_arg_string(&action.args, "message"),
        "blocked" => taskspace_action_arg_string(&action.args, "reason")
            .map(|reason| format!("blocked_by_taskspace_action_contract: {reason}")),
        _ => None,
    }
}

#[cfg(test)]
fn taskspace_action_is_terminal(action: &TaskSpaceActionV1) -> bool {
    taskspace_action_final_message(action).is_some()
}

fn apply_taskspace_terminal_action_message(
    needs_follow_up: &mut bool,
    saw_actionable_output: &mut bool,
    last_agent_message: &mut Option<String>,
    final_message: String,
) {
    *needs_follow_up = false;
    *saw_actionable_output = false;
    *last_agent_message = Some(final_message);
}

fn taskspace_final_answer_gate_rejection_followup(error: &str) -> String {
    format!(
        "TaskSpace final_answer rejected by final readiness gate. Rejection reason: {error}\n\
Continue the same task; do not treat the previous response as final. Correct the specific rejection reason before final_answer."
    )
}

fn taskspace_blocked_gate_rejection_followup(error: &str) -> String {
    format!(
        "TaskSpace blocked response rejected by terminal blocker gate. Rejection reason: {error}\n\
Continue the same task; do not treat the previous response as final blocked. Correct the specific rejection reason before emitting blocked again."
    )
}

async fn record_taskspace_observed_implement_edit(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    turn_diff_tracker: &SharedTurnDiffTracker,
    request_count: usize,
) -> bool {
    if sess
        .action_map_active_map_has_successful_edit_artifacts()
        .await
    {
        return false;
    }
    let unified_diff = {
        let mut tracker = turn_diff_tracker.lock().await;
        tracker.get_unified_diff().ok().flatten()
    };
    let Some(unified_diff) = unified_diff else {
        return false;
    };
    if unified_diff.trim().is_empty() {
        return false;
    }
    let preview = format!(
        "TaskSpace observed implementation edit in turn diff: {}",
        unified_diff
            .lines()
            .take(12)
            .collect::<Vec<_>>()
            .join("\\n")
    );
    if sess
        .backfill_action_map_successful_implementation_edit_artifacts(
            turn_context,
            &format!("taskspace-observed-implement-edit-backfill-{request_count}"),
            preview.clone(),
        )
        .await
    {
        return true;
    }
    sess.record_action_map_main_tool_result(
        turn_context,
        &format!("taskspace-observed-implement-edit-{request_count}"),
        "apply_patch",
        Some(ActionClass::Edit),
        true,
        preview,
    )
    .await;
    true
}

#[allow(clippy::too_many_arguments)]
#[instrument(level = "trace",
    skip_all,
    fields(
        turn_id = %turn_context.sub_id,
        model = %turn_context.model_info.slug
    )
)]
async fn try_run_sampling_request(
    tool_runtime: ToolCallRuntime,
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    client_session: &mut ModelClientSession,
    turn_metadata_header: Option<&str>,
    turn_diff_tracker: SharedTurnDiffTracker,
    prompt: &Prompt,
    cancellation_token: CancellationToken,
) -> CodexResult<SamplingRequestResult> {
    feedback_tags!(
        model = turn_context.model_info.slug.clone(),
        approval_policy = turn_context.approval_policy.value(),
        sandbox_policy = turn_context.sandbox_policy.get(),
        effort = turn_context.reasoning_effort,
        auth_mode = sess.services.auth_manager.auth_mode(),
        features = sess.features.enabled_features(),
    );
    let inference_trace = sess.services.rollout_thread_trace.inference_trace_context(
        turn_context.sub_id.as_str(),
        turn_context.model_info.slug.as_str(),
        turn_context.provider.info().name.as_str(),
    );
    let provider_budget_snapshot = sess.action_map_provider_request_budget_snapshot().await;
    if let Some(snapshot) = provider_budget_snapshot.as_ref() {
        let gate = sess
            .action_map_gate_provider_request_pre_dispatch(snapshot)
            .await;
        if !gate.allowed {
            if snapshot.node_kind.as_deref() == Some("inspect_code_context")
                && sess
                    .action_map_current_inspect_progress_ready_for_transition()
                    .await
            {
                match sess
                    .force_finish_action_map_inspect_for_provider_budget(
                        &turn_context,
                        snapshot.clone(),
                        "inspect_hard_stop_progress_convergence",
                    )
                    .await
                {
                    Ok(true) => {
                        return Ok(SamplingRequestResult {
                            needs_follow_up: true,
                            last_agent_message: Some(
                                "TaskSpace forced inspect transition before provider budget hard stop."
                                    .to_string(),
                            ),
                            taskspace_no_action_recovery_item: Some(
                                build_taskspace_forced_inspect_transition_recovery_item(),
                            ),
                        });
                    }
                    Ok(false) => {}
                    Err(error) => {
                        sess.send_event(
                            &turn_context,
                            EventMsg::Warning(WarningEvent {
                                message: format!(
                                    "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_hard_stop_progress_convergence error={error}"
                                ),
                            }),
                        )
                        .await;
                    }
                }
            }
            sess.send_event(
                &turn_context,
                EventMsg::Warning(WarningEvent {
                    message: format!(
                        "TaskSpaceProviderBudgetHardStopV1 reason={} request_count={}/{} node_request_count={}/{} state={} node_kind={} phase={}",
                        gate.reason,
                        snapshot.request_count,
                        snapshot.max_requests,
                        snapshot.node_request_count,
                        snapshot.max_model_requests_per_node,
                        snapshot.budget_state,
                        snapshot.node_kind.as_deref().unwrap_or("unknown"),
                        snapshot.request_phase.as_deref().unwrap_or("unknown"),
                    ),
                }),
            )
            .await;
            return Ok(SamplingRequestResult {
                needs_follow_up: false,
                last_agent_message: Some(format!(
                    "TaskSpace provider budget hard stop: {}",
                    gate.reason
                )),
                taskspace_no_action_recovery_item: Some(
                    build_taskspace_provider_budget_hard_stop_item(snapshot, &gate),
                ),
            });
        }
    }
    let provider_request_budget = provider_budget_snapshot
        .as_ref()
        .map(|snapshot| {
            let transport_mode =
                taskspace_provider_transport_mode(&turn_context, Some(snapshot), true);
            let limits = taskspace_transport_budget_limits(snapshot);
            let budget_state = match transport_mode {
                TaskspaceProviderTransportMode::NativeTools => snapshot.budget_state.clone(),
                TaskspaceProviderTransportMode::CacheOptimizedActionContract => String::new(),
            };
            ProviderRequestBudgetContext::enabled_with_attribution(
                ProviderRequestBudgetLimits {
                    request_count: snapshot.request_count,
                    max_requests: limits.max_requests,
                    node_request_count: snapshot.node_request_count,
                    max_model_requests_per_node: limits.max_model_requests_per_node,
                    post_budget_grace_requests: limits.post_budget_grace_requests,
                    post_budget_grace_request_count: snapshot.post_budget_grace_request_count,
                    budget_state,
                },
                ProviderRequestAttribution::from_snapshot(snapshot, turn_context.sub_id.as_str()),
            )
        })
        .unwrap_or_else(ProviderRequestBudgetContext::disabled);
    let action_contract_mode = taskspace_provider_transport_mode(
        &turn_context,
        provider_budget_snapshot.as_ref(),
        prompt_contains_taskspace_active_context(prompt),
    ) == TaskspaceProviderTransportMode::CacheOptimizedActionContract;
    let stream_result = client_session
        .stream_with_provider_request_budget(
            prompt,
            &turn_context.model_info,
            &turn_context.session_telemetry,
            turn_context.reasoning_effort,
            turn_context.reasoning_summary,
            turn_context.config.service_tier,
            turn_metadata_header,
            &inference_trace,
            &provider_request_budget,
        )
        .instrument(trace_span!("stream_request"))
        .or_cancel(&cancellation_token)
        .await;
    if let Some(snapshot) = provider_budget_snapshot.as_ref() {
        let events = provider_request_budget.drain_events();
        sess.record_action_map_provider_request_budget_events(
            &turn_context,
            snapshot.clone(),
            events,
        )
        .await;
    }
    let mut stream = stream_result??;
    let taskspace_progress_before_request =
        sess.action_map_current_main_node_progress_signature().await;
    let mut in_flight: FuturesOrdered<BoxFuture<'static, CodexResult<ResponseInputItem>>> =
        FuturesOrdered::new();
    let mut needs_follow_up = false;
    let mut last_agent_message: Option<String> = None;
    let mut saw_actionable_output = false;
    let mut active_item: Option<TurnItem> = None;
    let mut active_tool_argument_diff_consumer: Option<(
        String,
        Box<dyn ToolArgumentDiffConsumer>,
    )> = None;
    let mut should_emit_turn_diff = false;
    let mut taskspace_terminal_action_observed_in_request = false;
    let plan_mode = turn_context.collaboration_mode.mode == ModeKind::Plan;
    let mut assistant_message_stream_parsers = AssistantMessageStreamParsers::new(plan_mode);
    let mut plan_mode_state = plan_mode.then(|| PlanModeStreamState::new(&turn_context.sub_id));
    let receiving_span = trace_span!("receiving_stream");
    let mut outcome: CodexResult<SamplingRequestResult> = loop {
        let handle_responses = trace_span!(
            parent: &receiving_span,
            "handle_responses",
            otel.name = field::Empty,
            tool_name = field::Empty,
            from = field::Empty,
        );

        let event = match stream
            .next()
            .instrument(trace_span!(parent: &handle_responses, "receiving"))
            .or_cancel(&cancellation_token)
            .await
        {
            Ok(event) => event,
            Err(codex_async_utils::CancelErr::Cancelled) => {
                provider_request_budget.record_cancelled();
                if let Some(snapshot) = provider_budget_snapshot.as_ref() {
                    let events = provider_request_budget.drain_events();
                    sess.record_action_map_provider_request_budget_events(
                        &turn_context,
                        snapshot.clone(),
                        events,
                    )
                    .await;
                }
                break Err(CodexErr::TurnAborted);
            }
        };

        let event = match event {
            Some(Ok(event)) => event,
            Some(Err(err)) => {
                provider_request_budget.record_response_failed();
                if let Some(snapshot) = provider_budget_snapshot.as_ref() {
                    let events = provider_request_budget.drain_events();
                    sess.record_action_map_provider_request_budget_events(
                        &turn_context,
                        snapshot.clone(),
                        events,
                    )
                    .await;
                }
                break Err(err);
            }
            None => {
                provider_request_budget.record_response_failed();
                if let Some(snapshot) = provider_budget_snapshot.as_ref() {
                    let events = provider_request_budget.drain_events();
                    sess.record_action_map_provider_request_budget_events(
                        &turn_context,
                        snapshot.clone(),
                        events,
                    )
                    .await;
                }
                break Err(CodexErr::Stream(
                    "stream closed before response.completed".into(),
                    None,
                ));
            }
        };

        sess.services
            .session_telemetry
            .record_responses(&handle_responses, &event);
        record_turn_ttft_metric(&turn_context, &event).await;

        match event {
            ResponseEvent::Created => {}
            ResponseEvent::OutputItemDone(item) => {
                if let Some((_, mut consumer)) = active_tool_argument_diff_consumer.take()
                    && let Ok(Some(event)) = consumer.finish()
                {
                    sess.send_event(&turn_context, event).await;
                }
                let previously_active_item = active_item.take();
                if let Some(previous) = previously_active_item.as_ref()
                    && matches!(previous, TurnItem::AgentMessage(_))
                {
                    let item_id = previous.id();
                    flush_assistant_text_segments_for_item(
                        &sess,
                        &turn_context,
                        plan_mode_state.as_mut(),
                        &mut assistant_message_stream_parsers,
                        &item_id,
                    )
                    .await;
                }
                if let Some(state) = plan_mode_state.as_mut()
                    && handle_assistant_item_done_in_plan_mode(
                        &sess,
                        &turn_context,
                        &item,
                        state,
                        previously_active_item.as_ref(),
                        &mut last_agent_message,
                    )
                    .await
                {
                    continue;
                }

                let mut ctx = HandleOutputCtx {
                    sess: sess.clone(),
                    turn_context: turn_context.clone(),
                    tool_runtime: tool_runtime.clone(),
                    cancellation_token: cancellation_token.child_token(),
                };

                let preempt_for_mailbox_mail = match &item {
                    ResponseItem::Message { role, phase, .. } => {
                        role == "assistant" && matches!(phase, Some(MessagePhase::Commentary))
                    }
                    ResponseItem::Reasoning { .. } => true,
                    ResponseItem::LocalShellCall { .. }
                    | ResponseItem::FunctionCall { .. }
                    | ResponseItem::ToolSearchCall { .. }
                    | ResponseItem::FunctionCallOutput { .. }
                    | ResponseItem::CustomToolCall { .. }
                    | ResponseItem::CustomToolCallOutput { .. }
                    | ResponseItem::ToolSearchOutput { .. }
                    | ResponseItem::WebSearchCall { .. }
                    | ResponseItem::ImageGenerationCall { .. }
                    | ResponseItem::GhostSnapshot { .. }
                    | ResponseItem::Compaction { .. }
                    | ResponseItem::Other => false,
                };

                let raw_assistant_text = raw_assistant_output_text_from_item(&item);
                let output_result =
                    match handle_output_item_done(&mut ctx, item, previously_active_item)
                        .instrument(handle_responses)
                        .await
                    {
                        Ok(output_result) => output_result,
                        Err(err) => break Err(err),
                    };
                if let Some(tool_future) = output_result.tool_future {
                    saw_actionable_output = true;
                    in_flight.push_back(tool_future);
                }
                if let Some(agent_message) = output_result.last_agent_message {
                    last_agent_message = Some(agent_message);
                } else if let Some(raw_text) = raw_assistant_text.as_deref()
                    && !raw_text.trim().is_empty()
                {
                    last_agent_message = Some(raw_text.to_string());
                }
                let mut taskspace_terminal_action_observed = false;
                if action_contract_mode
                    && let Some(raw_text) = raw_assistant_text.as_deref()
                    && !raw_text.trim().is_empty()
                {
                    let parsed_action = match parse_taskspace_action_v1(raw_text) {
                        Ok(action) => Ok(action),
                        Err(reason) => Err(reason),
                    };
                    let action_result = match parsed_action {
                        Ok(action) => {
                            let closed_rework_read_reject = if let Some(snapshot) =
                                provider_budget_snapshot.as_ref()
                            {
                                taskspace_closed_validation_rework_read_reject_reason(
                                        &action,
                                        snapshot,
                                        sess.action_map_current_main_node_has_visible_validation_rework_target_read()
                                            .await,
                                    )
                            } else {
                                None
                            };
                            if let Some(reason) = closed_rework_read_reject {
                                Err(reason)
                            } else {
                                let action = if let Some(snapshot) =
                                    provider_budget_snapshot.as_ref()
                                    && should_finish_node_after_successful_required_action(
                                        &action,
                                        snapshot,
                                        sess.as_ref(),
                                    )
                                    .await
                                {
                                    if snapshot.node_kind.as_deref() == Some("inspect_code_context")
                                    {
                                        taskspace_finish_inspect_to_implementation_action(
                                            snapshot.node_id.as_deref(),
                                        )
                                    } else {
                                        taskspace_finish_current_node_action(
                                            snapshot.node_id.as_deref(),
                                            "Required node work already succeeded; finishing node.",
                                        )
                                    }
                                } else if let Some(snapshot) = provider_budget_snapshot.as_ref()
                                    && should_answer_after_successful_validation_redundant_node(
                                        &action,
                                        snapshot,
                                        sess.as_ref(),
                                    )
                                    .await
                                {
                                    taskspace_final_answer_action(
                                        "Validation passed; final result is ready.",
                                    )
                                } else if let Some(snapshot) = provider_budget_snapshot.as_ref()
                                    && should_block_after_tool_runtime_bootstrap_failure_without_active_node(
                                        &action,
                                        snapshot,
                                        sess.as_ref(),
                                    )
                                    .await
                                {
                                    taskspace_blocked_final_action(
                                        "TaskSpace ordinary tools are blocked by sandbox/tool runtime bootstrap failure evidence already recorded on the closed task path.",
                                    )
                                } else if let Some(snapshot) = provider_budget_snapshot.as_ref()
                                    && should_block_after_closed_validation_without_active_node(
                                        &action,
                                        snapshot,
                                        sess.as_ref(),
                                    )
                                    .await
                                {
                                    taskspace_blocked_final_action(
                                        "TaskSpace validation is blocked by local validator infrastructure evidence already recorded on the closed validation node.",
                                    )
                                } else if let Some(snapshot) = provider_budget_snapshot.as_ref()
                                    && should_answer_after_completed_task_without_active_node(
                                        &action,
                                        snapshot,
                                        sess.as_ref(),
                                    )
                                    .await
                                {
                                    taskspace_final_answer_action(
                                        "Validation passed; final result is ready.",
                                    )
                                } else if taskspace_action_control_creates_final_synthesis(&action) {
                                    taskspace_final_answer_action(
                                        "Validation passed; final result is ready.",
                                    )
                                } else {
                                    action
                                };
                                let final_message = taskspace_action_final_message(&action);
                                let tool_call =
                                    if let Some(snapshot) = provider_budget_snapshot.as_ref() {
                                        taskspace_action_to_tool_call(&action, snapshot)
                                    } else {
                                        taskspace_bootstrap_action_to_tool_call(&action)
                                    };
                                tool_call.map(|tool_call| (action, final_message, tool_call))
                            }
                        }
                        Err(reason) => Err(reason),
                    };
                    match action_result {
                        Ok((action, _final_message, Some(tool_call))) => {
                            let call_item =
                                response_item_for_taskspace_action_tool_call(&tool_call);
                            record_completed_response_item(
                                sess.as_ref(),
                                turn_context.as_ref(),
                                &call_item,
                            )
                            .await;
                            saw_actionable_output = true;
                            needs_follow_up = true;
                            let mut tool_error_message: Option<String> = None;
                            let mut tool_gate_recovery_message: Option<String> = None;
                            match tool_runtime
                                .clone()
                                .handle_tool_call(
                                    tool_call.clone(),
                                    cancellation_token.child_token(),
                                )
                                .await
                            {
                                Ok(response_input) => {
                                    let response_item = record_response_input_item(
                                        sess.as_ref(),
                                        turn_context.as_ref(),
                                        response_input,
                                    )
                                    .await;
                                    tool_gate_recovery_message =
                                        taskspace_gate_recovery_from_response_item(&response_item);
                                }
                                Err(err) => {
                                    let response_input =
                                        response_input_for_taskspace_action_tool_error(
                                            &tool_call, &err,
                                        );
                                    record_response_input_item(
                                        sess.as_ref(),
                                        turn_context.as_ref(),
                                        response_input,
                                    )
                                    .await;
                                    tool_error_message =
                                        Some(format!("TaskSpace tool call failed: {err}"));
                                }
                            }
                            if let Some(message) = tool_gate_recovery_message {
                                last_agent_message = Some(message);
                            } else if let Some(message) = tool_error_message {
                                last_agent_message = Some(message);
                            } else if let Some(rationale) = action.rationale.as_deref()
                                && !rationale.trim().is_empty()
                            {
                                last_agent_message = Some(rationale.to_string());
                            }
                        }
                        Ok((action, Some(final_message), None)) => {
                            let terminal_gate_error = if action.action == "final_answer" {
                                match sess
                                    .record_action_map_main_final_response(
                                        &turn_context,
                                        &final_message,
                                    )
                                    .await
                                {
                                    Ok(_) => None,
                                    Err(error) => Some(error),
                                }
                            } else if action.action == "blocked" {
                                match sess
                                    .validate_action_map_terminal_blocker(&final_message)
                                    .await
                                {
                                    Ok(_) => None,
                                    Err(error) => Some(error),
                                }
                            } else {
                                None
                            };
                            if let Some(error) = terminal_gate_error {
                                needs_follow_up = true;
                                saw_actionable_output = true;
                                last_agent_message = Some(if action.action == "blocked" {
                                    taskspace_blocked_gate_rejection_followup(&error)
                                } else {
                                    taskspace_final_answer_gate_rejection_followup(&error)
                                });
                            } else {
                                apply_taskspace_terminal_action_message(
                                    &mut needs_follow_up,
                                    &mut saw_actionable_output,
                                    &mut last_agent_message,
                                    final_message,
                                );
                                taskspace_terminal_action_observed = true;
                                taskspace_terminal_action_observed_in_request = true;
                            }
                        }
                        Ok((_action, None, None)) => {}
                        Err(reason) => {
                            needs_follow_up = true;
                            let patch_intent_suffix = if reason
                                == "action_contract_output_not_strict_json"
                                && taskspace_raw_text_mentions_apply_patch_intent(raw_text)
                            {
                                taskspace_last_message_preview(Some(raw_text))
                                    .map(|preview| {
                                        format!(
                                            ":apply_patch_intent. Rejected assistant output preview: {preview}"
                                        )
                                    })
                                    .unwrap_or_else(|| ":apply_patch_intent".to_string())
                            } else {
                                String::new()
                            };
                            last_agent_message = Some(format!(
                                "TaskSpaceActionV1 rejected: {reason}{patch_intent_suffix}. Return exactly one valid taskspace-action-v1 JSON object."
                            ));
                        }
                    }
                }
                if !taskspace_terminal_action_observed {
                    needs_follow_up |= output_result.needs_follow_up;
                }
                // todo: remove before stabilizing multi-agent v2
                if preempt_for_mailbox_mail && sess.mailbox_rx.lock().await.has_pending() {
                    break Ok(SamplingRequestResult {
                        needs_follow_up: true,
                        last_agent_message,
                        taskspace_no_action_recovery_item: None,
                    });
                }
            }
            ResponseEvent::OutputItemAdded(item) => {
                if let ResponseItem::CustomToolCall { call_id, name, .. } = &item {
                    let tool_name = ToolName::plain(name.as_str());
                    active_tool_argument_diff_consumer = tool_runtime
                        .create_diff_consumer(&tool_name)
                        .map(|consumer| (call_id.clone(), consumer));
                } else if matches!(&item, ResponseItem::FunctionCall { .. }) {
                    active_tool_argument_diff_consumer = None;
                }
                if let Some(turn_item) = handle_non_tool_response_item(
                    sess.as_ref(),
                    turn_context.as_ref(),
                    &item,
                    plan_mode,
                )
                .await
                {
                    let mut turn_item = turn_item;
                    let mut seeded_parsed: Option<ParsedAssistantTextDelta> = None;
                    let mut seeded_item_id: Option<String> = None;
                    if matches!(turn_item, TurnItem::AgentMessage(_))
                        && let Some(raw_text) = raw_assistant_output_text_from_item(&item)
                    {
                        let item_id = turn_item.id();
                        let visible_source_text = if action_contract_mode {
                            taskspace_action_contract_visible_text(&raw_text)
                                .unwrap_or_else(|| raw_text.clone())
                        } else {
                            raw_text.clone()
                        };
                        let mut seeded = assistant_message_stream_parsers
                            .seed_item_text(&item_id, &visible_source_text);
                        if let TurnItem::AgentMessage(agent_message) = &mut turn_item {
                            agent_message.content =
                                vec![codex_protocol::items::AgentMessageContent::Text {
                                    text: if plan_mode {
                                        String::new()
                                    } else {
                                        std::mem::take(&mut seeded.visible_text)
                                    },
                                }];
                        }
                        seeded_parsed = plan_mode.then_some(seeded);
                        seeded_item_id = Some(item_id);
                    }
                    if let Some(state) = plan_mode_state.as_mut()
                        && matches!(turn_item, TurnItem::AgentMessage(_))
                    {
                        let item_id = turn_item.id();
                        state
                            .pending_agent_message_items
                            .insert(item_id, turn_item.clone());
                    } else {
                        sess.emit_turn_item_started(&turn_context, &turn_item).await;
                    }
                    if let (Some(state), Some(item_id), Some(parsed)) = (
                        plan_mode_state.as_mut(),
                        seeded_item_id.as_deref(),
                        seeded_parsed,
                    ) {
                        emit_streamed_assistant_text_delta(
                            &sess,
                            &turn_context,
                            Some(state),
                            item_id,
                            parsed,
                        )
                        .await;
                    }
                    active_item = Some(turn_item);
                }
            }
            ResponseEvent::ServerModel(server_model) => {
                if !turn_context
                    .server_model_warning_emitted
                    .load(Ordering::Relaxed)
                    && sess
                        .maybe_warn_on_server_model_mismatch(&turn_context, server_model)
                        .await
                {
                    turn_context
                        .server_model_warning_emitted
                        .store(true, Ordering::Relaxed);
                }
            }
            ResponseEvent::ModelVerifications(verifications) => {
                if !turn_context
                    .model_verification_emitted
                    .swap(true, Ordering::Relaxed)
                {
                    sess.emit_model_verification(&turn_context, verifications)
                        .await;
                }
            }
            ResponseEvent::ServerReasoningIncluded(included) => {
                sess.set_server_reasoning_included(included).await;
            }
            ResponseEvent::RateLimits(snapshot) => {
                // Update internal state with latest rate limits, but defer sending until
                // token usage is available to avoid duplicate TokenCount events.
                sess.update_rate_limits(&turn_context, snapshot).await;
            }
            ResponseEvent::ModelsEtag(etag) => {
                // Update internal state with latest models etag
                sess.services.models_manager.refresh_if_new_etag(etag).await;
            }
            ResponseEvent::Completed {
                response_id: _,
                token_usage,
                end_turn,
            } => {
                flush_assistant_text_segments_all(
                    &sess,
                    &turn_context,
                    plan_mode_state.as_mut(),
                    &mut assistant_message_stream_parsers,
                )
                .await;
                sess.update_token_usage_info(&turn_context, token_usage.as_ref())
                    .await;
                provider_request_budget.record_response_completed(token_usage.as_ref());
                if let Some(snapshot) = provider_budget_snapshot.as_ref() {
                    let events = provider_request_budget.drain_events();
                    sess.record_action_map_provider_request_budget_events(
                        &turn_context,
                        snapshot.clone(),
                        events,
                    )
                    .await;
                }
                should_emit_turn_diff = true;
                if let Some(false) = end_turn {
                    needs_follow_up = true;
                }
                let assistant_message_present = last_agent_message
                    .as_deref()
                    .map(|message| !message.trim().is_empty())
                    .unwrap_or(false);
                let mut final_response_rejected = false;
                if !taskspace_terminal_action_observed_in_request
                    && !needs_follow_up
                    && let Some(message) = last_agent_message.as_deref()
                {
                    match sess
                        .record_action_map_main_final_response(&turn_context, message)
                        .await
                    {
                        Ok(_) => {}
                        Err(error) => {
                            needs_follow_up = true;
                            final_response_rejected = true;
                            last_agent_message =
                                Some(taskspace_final_answer_gate_rejection_followup(&error));
                        }
                    }
                }
                let current_budget_snapshot =
                    sess.action_map_provider_request_budget_snapshot().await;
                if taskspace_active_node_empty_response_requires_follow_up(
                    current_budget_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.node_kind.as_deref()),
                    saw_actionable_output,
                    assistant_message_present,
                    taskspace_terminal_action_observed_in_request,
                ) {
                    needs_follow_up = true;
                }
                let provider_budget_exhausted_followup = false;
                let budget_pressure_follow_up_intent = current_budget_snapshot
                    .as_ref()
                    .map(|snapshot| {
                        taskspace_budget_pressure_follow_up_intent(
                            snapshot.request_count,
                            snapshot.max_requests,
                            snapshot.node_kind.as_deref(),
                            last_agent_message.as_deref(),
                        )
                    })
                    .unwrap_or(false);
                let budget_pressure_silent_action_transition = current_budget_snapshot
                    .as_ref()
                    .map(|snapshot| {
                        taskspace_budget_pressure_silent_action_requires_transition(
                            snapshot.request_count,
                            snapshot.max_requests,
                            snapshot.node_kind.as_deref(),
                            assistant_message_present,
                            saw_actionable_output,
                        )
                    })
                    .unwrap_or(false);
                let response_actionability = classify_taskspace_provider_response_actionability(
                    needs_follow_up,
                    saw_actionable_output,
                    assistant_message_present,
                    taskspace_message_has_gate_recovery(last_agent_message.as_deref()),
                    final_response_rejected
                        || budget_pressure_follow_up_intent
                        || budget_pressure_silent_action_transition,
                    provider_budget_exhausted_followup,
                );
                let mut taskspace_no_action_recovery_item = if response_actionability
                    .needs_recovery()
                    && current_budget_snapshot.is_some()
                {
                    if let Some(targets) = taskspace_existing_file_add_targets_from_rejection(
                        last_agent_message.as_deref(),
                    ) {
                        Some(build_taskspace_apply_patch_format_recovery_item(&targets))
                    } else if let Some(targets) =
                        taskspace_replacement_required_targets_from_rejection(
                            last_agent_message.as_deref(),
                        )
                    {
                        Some(
                            build_taskspace_apply_patch_replacement_required_recovery_item(
                                &targets,
                            ),
                        )
                    } else if let Some(targets) =
                        taskspace_native_hunk_targets_from_rejection(last_agent_message.as_deref())
                    {
                        let evidence_summary =
                            sess.action_map_current_working_evidence_summary().await;
                        let force_complete_replacement =
                            taskspace_evidence_has_full_visible_validation_rework_target_read(
                                evidence_summary.as_deref(),
                            );
                        Some(build_taskspace_apply_patch_native_hunk_recovery_item(
                            &targets,
                            force_complete_replacement,
                        ))
                    } else if let Some(targets) = taskspace_unanchored_update_targets_from_rejection(
                        last_agent_message.as_deref(),
                    ) {
                        Some(build_taskspace_apply_patch_unanchored_update_recovery_item(
                            &targets,
                        ))
                    } else if current_budget_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.node_kind.as_deref() == Some("implement_solution")
                    }) && let Some(targets) =
                        taskspace_missing_update_targets_from_apply_patch_error(
                            last_agent_message.as_deref(),
                        )
                    {
                        Some(build_taskspace_apply_patch_missing_target_recovery_item(
                            &targets,
                        ))
                    } else if current_budget_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.node_kind.as_deref() == Some("implement_solution")
                            && taskspace_message_hit_apply_patch_intent_format_rejection(
                                last_agent_message.as_deref(),
                            )
                    }) {
                        let evidence_summary =
                            sess.action_map_current_working_evidence_summary().await;
                        Some(build_taskspace_patch_intent_format_recovery_item(
                            evidence_summary.as_deref(),
                            taskspace_rejected_apply_patch_intent_preview(
                                last_agent_message.as_deref(),
                            ),
                        ))
                    } else if current_budget_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.node_kind.as_deref() == Some("implement_solution")
                            && taskspace_message_hit_implementation_needs_edit(
                                last_agent_message.as_deref(),
                            )
                    }) {
                        let evidence_summary =
                            sess.action_map_current_working_evidence_summary().await;
                        let failed_edit_summary =
                            sess.action_map_current_recent_failed_edit_summary().await;
                        Some(build_taskspace_implementation_recovery_item(
                            last_agent_message.as_deref(),
                            evidence_summary.as_deref(),
                            failed_edit_summary.as_deref(),
                        ))
                    } else if current_budget_snapshot.as_ref().is_some_and(|snapshot| {
                        matches!(
                            snapshot.node_kind.as_deref(),
                            Some("smoke_test" | "regression_test")
                        ) && taskspace_message_hit_validation_needs_test(
                            last_agent_message.as_deref(),
                        )
                    }) {
                        Some(build_taskspace_validation_needs_test_recovery_item(
                            last_agent_message.as_deref(),
                        ))
                    } else if current_budget_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.node_kind.as_deref() == Some("inspect_code_context")
                            && taskspace_message_has_inspect_duplicate_successful_evidence(
                                last_agent_message.as_deref(),
                            )
                    }) {
                        let snapshot = current_budget_snapshot
                            .as_ref()
                            .expect("snapshot checked above")
                            .clone();
                        let duplicate_read_search_gate = taskspace_message_has_gate_recovery_reason(
                            last_agent_message.as_deref(),
                            "inspect_duplicate_successful_read_or_search",
                        );
                        if taskspace_message_has_repeated_blocked_action(
                            last_agent_message.as_deref(),
                        ) && duplicate_read_search_gate
                        {
                            let missing_fact_sources = sess
                                .action_map_current_inspect_missing_required_fact_source_artifacts()
                                .await;
                            if missing_fact_sources.is_empty() {
                                let trigger =
                                    taskspace_inspect_duplicate_successful_evidence_trigger(
                                        last_agent_message.as_deref(),
                                    );
                                match sess
                                    .force_finish_action_map_inspect_for_provider_budget(
                                        &turn_context,
                                        snapshot.clone(),
                                        trigger,
                                    )
                                    .await
                                {
                                    Ok(true) => Some(
                                        build_taskspace_forced_inspect_transition_recovery_item(),
                                    ),
                                    Ok(false) => {
                                        if let Some(forced_item) =
                                            run_taskspace_duplicate_read_search_bootstrap_then_force(
                                                tool_runtime.clone(),
                                                sess.clone(),
                                                turn_context.clone(),
                                                snapshot,
                                                cancellation_token.child_token(),
                                            )
                                            .await?
                                        {
                                            Some(forced_item)
                                        } else {
                                            Some(
                                                build_taskspace_duplicate_inspect_successful_evidence_recovery_item(
                                                    last_agent_message.as_deref(),
                                                ),
                                            )
                                        }
                                    }
                                    Err(error) => {
                                        sess.send_event(
                                            &turn_context,
                                            EventMsg::Warning(WarningEvent {
                                                message: format!(
                                                    "TaskSpaceForcedInspectTransitionFailedV1 trigger={trigger} error={error}"
                                                ),
                                            }),
                                        )
                                        .await;
                                        Some(
                                            build_taskspace_duplicate_inspect_successful_evidence_recovery_item(
                                                last_agent_message.as_deref(),
                                            ),
                                        )
                                    }
                                }
                            } else {
                                run_taskspace_missing_fact_source_bootstrap(
                                    tool_runtime.clone(),
                                    sess.clone(),
                                    turn_context.clone(),
                                    snapshot.request_count,
                                    &missing_fact_sources,
                                    cancellation_token.child_token(),
                                )
                                .await?;
                                let remaining_fact_sources = sess
                                    .action_map_current_inspect_missing_required_fact_source_artifacts()
                                    .await;
                                if remaining_fact_sources.is_empty() {
                                    match sess
                                        .force_finish_action_map_inspect_for_provider_budget(
                                            &turn_context,
                                            snapshot,
                                            "inspect_missing_fact_source_bootstrap_complete",
                                        )
                                        .await
                                    {
                                        Ok(true) => Some(
                                            build_taskspace_forced_inspect_transition_recovery_item(
                                            ),
                                        ),
                                        Ok(false) => None,
                                        Err(error) => {
                                            sess.send_event(
                                                &turn_context,
                                                EventMsg::Warning(WarningEvent {
                                                    message: format!(
                                                        "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_missing_fact_source_bootstrap_complete error={error}"
                                                    ),
                                                }),
                                            )
                                            .await;
                                            None
                                        }
                                    }
                                } else {
                                    None
                                }
                            }
                        } else if taskspace_message_has_repeated_blocked_action(
                            last_agent_message.as_deref(),
                        ) && !duplicate_read_search_gate
                        {
                            match sess
                                .force_finish_action_map_inspect_for_provider_budget(
                                    &turn_context,
                                    snapshot.clone(),
                                    "inspect_repeated_blocked_action_with_evidence",
                                )
                                .await
                            {
                                Ok(true) => {
                                    Some(build_taskspace_forced_inspect_transition_recovery_item())
                                }
                                Ok(false) => {
                                    run_taskspace_inspect_bootstrap(
                                        tool_runtime.clone(),
                                        sess.clone(),
                                        turn_context.clone(),
                                    snapshot.request_count,
                                    TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_CALL_ID,
                                    taskspace_repeated_blocked_inspect_bootstrap_command(),
                                    "TaskSpaceRepeatedBlockedInspectBootstrapV1 executed bounded source/test reads after an inspect_code_context action repeated an already-blocked diagnostic command.",
                                    cancellation_token.child_token(),
                                )
                                    .await?;
                                    None
                                }
                                Err(error) => {
                                    sess.send_event(
                                        &turn_context,
                                        EventMsg::Warning(WarningEvent {
                                            message: format!(
                                                "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_repeated_blocked_action_with_evidence error={error}"
                                            ),
                                        }),
                                    )
                                    .await;
                                    Some(
                                        build_taskspace_duplicate_diagnostic_inspect_recovery_item(
                                            last_agent_message.as_deref(),
                                        ),
                                    )
                                }
                            }
                        } else {
                            let trigger = taskspace_inspect_duplicate_successful_evidence_trigger(
                                last_agent_message.as_deref(),
                            );
                            match sess
                                .force_finish_action_map_inspect_for_provider_budget(
                                    &turn_context,
                                    snapshot,
                                    trigger,
                                )
                                .await
                            {
                                Ok(true) => {
                                    Some(build_taskspace_forced_inspect_transition_recovery_item())
                                }
                                Ok(false) => Some(
                                    build_taskspace_duplicate_inspect_successful_evidence_recovery_item(
                                        last_agent_message.as_deref(),
                                    ),
                                ),
                                Err(error) => {
                                    sess.send_event(
                                        &turn_context,
                                        EventMsg::Warning(WarningEvent {
                                            message: format!(
                                                "TaskSpaceForcedInspectTransitionFailedV1 trigger={trigger} error={error}"
                                            ),
                                        }),
                                    )
                                    .await;
                                    Some(
                                        build_taskspace_duplicate_inspect_successful_evidence_recovery_item(
                                            last_agent_message.as_deref(),
                                        ),
                                    )
                                }
                            }
                        }
                    } else {
                        Some(build_taskspace_no_action_recovery_item(
                            last_agent_message.as_deref(),
                        ))
                    }
                } else {
                    None
                };
                if let Some(snapshot) = current_budget_snapshot {
                    let recovery_action = if taskspace_no_action_recovery_item.is_some() {
                        "developer_recovery"
                    } else {
                        "none"
                    };
                    sess.record_action_map_provider_response_actionability(
                        &turn_context,
                        snapshot,
                        ActionMapProviderResponseActionabilityInput {
                            response_actionability: response_actionability.as_str().to_string(),
                            end_turn,
                            saw_actionable_output,
                            assistant_message_present,
                            recovery_action: recovery_action.to_string(),
                            last_agent_message_preview: taskspace_last_message_preview(
                                last_agent_message.as_deref(),
                            ),
                        },
                    )
                    .await;
                }
                if response_actionability.needs_recovery()
                    && (budget_pressure_follow_up_intent
                        || budget_pressure_silent_action_transition)
                    && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
                {
                    let trigger = if budget_pressure_silent_action_transition {
                        "budget_pressure_silent_action"
                    } else {
                        "budget_pressure_follow_up_intent"
                    };
                    match snapshot.node_kind.as_deref() {
                        Some("inspect_code_context") => {
                            match sess
                                .force_finish_action_map_inspect_for_provider_budget(
                                    &turn_context,
                                    snapshot,
                                    trigger,
                                )
                                .await
                            {
                                Ok(true) => {
                                    taskspace_no_action_recovery_item = Some(
                                        build_taskspace_forced_inspect_transition_recovery_item(),
                                    );
                                }
                                Ok(false) => {}
                                Err(error) => {
                                    sess.send_event(
                                        &turn_context,
                                        EventMsg::Warning(WarningEvent {
                                            message: format!(
                                                "TaskSpaceForcedInspectTransitionFailedV1 trigger={trigger} error={error}"
                                            ),
                                        }),
                                    )
                                    .await;
                                }
                            }
                        }
                        Some("implement_solution") => {
                            record_taskspace_observed_implement_edit(
                                &sess,
                                &turn_context,
                                &turn_diff_tracker,
                                snapshot.request_count,
                            )
                            .await;
                            match sess
                                .force_finish_action_map_implement_for_provider_budget(
                                    &turn_context,
                                    snapshot,
                                    trigger,
                                )
                                .await
                            {
                                Ok(true) => {
                                    taskspace_no_action_recovery_item = Some(
                                        build_taskspace_forced_implement_transition_recovery_item(),
                                    );
                                }
                                Ok(false) => {}
                                Err(error) => {
                                    sess.send_event(
                                        &turn_context,
                                        EventMsg::Warning(WarningEvent {
                                            message: format!(
                                                "TaskSpaceForcedImplementTransitionFailedV1 trigger={trigger} error={error}"
                                            ),
                                        }),
                                    )
                                    .await;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if final_response_rejected {
                    last_agent_message = None;
                }
                break Ok(SamplingRequestResult {
                    needs_follow_up,
                    last_agent_message,
                    taskspace_no_action_recovery_item,
                });
            }
            ResponseEvent::OutputTextDelta(delta) => {
                // In review child threads, suppress assistant text deltas; the
                // UI will show a selection popup from the final ReviewOutput.
                if let Some(active) = active_item.as_ref() {
                    let item_id = active.id();
                    if matches!(active, TurnItem::AgentMessage(_)) {
                        let parsed = assistant_message_stream_parsers.parse_delta(&item_id, &delta);
                        emit_streamed_assistant_text_delta(
                            &sess,
                            &turn_context,
                            plan_mode_state.as_mut(),
                            &item_id,
                            parsed,
                        )
                        .await;
                    } else {
                        let event = AgentMessageContentDeltaEvent {
                            thread_id: sess.conversation_id.to_string(),
                            turn_id: turn_context.sub_id.clone(),
                            item_id,
                            delta,
                        };
                        sess.send_event(&turn_context, EventMsg::AgentMessageContentDelta(event))
                            .await;
                    }
                } else {
                    error_or_panic("OutputTextDelta without active item".to_string());
                }
            }
            ResponseEvent::ToolCallInputDelta {
                item_id: _,
                call_id,
                delta,
            } => {
                let Some((active_call_id, consumer)) = active_tool_argument_diff_consumer.as_mut()
                else {
                    continue;
                };
                let call_id = match call_id {
                    Some(call_id) if call_id.as_str() != active_call_id.as_str() => continue,
                    Some(call_id) => call_id,
                    None => active_call_id.clone(),
                };
                if let Some(event) = consumer.consume_diff(turn_context.as_ref(), call_id, &delta) {
                    sess.send_event(&turn_context, event).await;
                }
            }
            ResponseEvent::ReasoningSummaryDelta {
                delta,
                summary_index,
            } => {
                if let Some(active) = active_item.as_ref() {
                    let event = ReasoningContentDeltaEvent {
                        thread_id: sess.conversation_id.to_string(),
                        turn_id: turn_context.sub_id.clone(),
                        item_id: active.id(),
                        delta,
                        summary_index,
                    };
                    sess.send_event(&turn_context, EventMsg::ReasoningContentDelta(event))
                        .await;
                } else {
                    error_or_panic("ReasoningSummaryDelta without active item".to_string());
                }
            }
            ResponseEvent::ReasoningSummaryPartAdded { summary_index } => {
                if let Some(active) = active_item.as_ref() {
                    let event =
                        EventMsg::AgentReasoningSectionBreak(AgentReasoningSectionBreakEvent {
                            item_id: active.id(),
                            summary_index,
                        });
                    sess.send_event(&turn_context, event).await;
                } else {
                    error_or_panic("ReasoningSummaryPartAdded without active item".to_string());
                }
            }
            ResponseEvent::ReasoningContentDelta {
                delta,
                content_index,
            } => {
                if let Some(active) = active_item.as_ref() {
                    let event = ReasoningRawContentDeltaEvent {
                        thread_id: sess.conversation_id.to_string(),
                        turn_id: turn_context.sub_id.clone(),
                        item_id: active.id(),
                        delta,
                        content_index,
                    };
                    sess.send_event(&turn_context, EventMsg::ReasoningRawContentDelta(event))
                        .await;
                } else {
                    error_or_panic("ReasoningRawContentDelta without active item".to_string());
                }
            }
        }
    };

    flush_assistant_text_segments_all(
        &sess,
        &turn_context,
        plan_mode_state.as_mut(),
        &mut assistant_message_stream_parsers,
    )
    .await;

    drain_in_flight(&mut in_flight, sess.clone(), turn_context.clone()).await?;
    if let Ok(result) = &mut outcome
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && snapshot.node_kind.as_deref() == Some("implement_solution")
    {
        let recorded_diff_edit = record_taskspace_observed_implement_edit(
            &sess,
            &turn_context,
            &turn_diff_tracker,
            snapshot.request_count,
        )
        .await;
        let has_successful_edit = recorded_diff_edit
            || sess
                .action_map_current_main_node_has_successful_action(ActionClass::Edit)
                .await;
        if has_successful_edit {
            match sess
                .force_finish_action_map_implement_for_provider_budget(
                    &turn_context,
                    snapshot,
                    "implement_observed_edit_after_tool_drain",
                )
                .await
            {
                Ok(true) => {
                    result.taskspace_no_action_recovery_item =
                        Some(build_taskspace_forced_implement_transition_recovery_item());
                }
                Ok(false) => {}
                Err(error) => {
                    sess.send_event(
                        &turn_context,
                        EventMsg::Warning(WarningEvent {
                            message: format!(
                                "TaskSpaceForcedImplementTransitionFailedV1 trigger=implement_observed_edit_after_tool_drain error={error}"
                            ),
                        }),
                    )
                    .await;
                }
            }
        }
    }
    if let Ok(result) = &mut outcome
        && result
            .taskspace_no_action_recovery_item
            .as_ref()
            .is_some_and(|item| {
                response_item_text_contains(item, TASKSPACE_VALIDATION_NEEDS_TEST_MARKER)
            })
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && matches!(
            snapshot.node_kind.as_deref(),
            Some("smoke_test" | "regression_test")
        )
        && let Some(command) = taskspace_validation_required_command_from_gate_recovery(
            result.last_agent_message.as_deref(),
        )
    {
        run_taskspace_validation_required_command_bootstrap(
            tool_runtime.clone(),
            sess.clone(),
            turn_context.clone(),
            snapshot.request_count,
            &command,
            cancellation_token.child_token(),
        )
        .await?;
        result.taskspace_no_action_recovery_item = None;
    }
    if let Ok(result) = &mut outcome
        && result.taskspace_no_action_recovery_item.is_none()
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && matches!(
            snapshot.node_kind.as_deref(),
            Some("smoke_test" | "regression_test")
        )
    {
        match sess
            .force_finish_action_map_validation_after_successful_tool(
                &turn_context,
                snapshot,
                "validation_success_after_tool_drain",
            )
            .await
        {
            Ok(true) => {
                result.taskspace_no_action_recovery_item =
                    Some(build_taskspace_forced_validation_closeout_recovery_item());
            }
            Ok(false) => {}
            Err(error) => {
                sess.send_event(
                    &turn_context,
                    EventMsg::Warning(WarningEvent {
                        message: format!(
                            "TaskSpaceForcedValidationCloseoutFailedV1 trigger=validation_success_after_tool_drain error={error}"
                        ),
                    }),
                )
                .await;
            }
        }
    }
    if let Ok(result) = &mut outcome
        && result.taskspace_no_action_recovery_item.is_none()
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && snapshot.node_kind.as_deref() == Some("implement_solution")
        && sess
            .action_map_current_implement_progress_needs_edit()
            .await
    {
        let evidence_summary = sess.action_map_current_working_evidence_summary().await;
        let failed_edit_summary = sess.action_map_current_recent_failed_edit_summary().await;
        result.taskspace_no_action_recovery_item =
            Some(build_taskspace_implementation_recovery_item(
                result.last_agent_message.as_deref(),
                evidence_summary.as_deref(),
                failed_edit_summary.as_deref(),
            ));
    }
    if let Ok(result) = &mut outcome
        && result.taskspace_no_action_recovery_item.is_none()
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && snapshot.node_kind.as_deref() == Some("inspect_code_context")
        && sess
            .action_map_current_inspect_progress_ready_for_transition()
            .await
    {
        match sess
            .force_finish_action_map_inspect_for_provider_budget(
                &turn_context,
                snapshot,
                "inspect_progress_convergence",
            )
            .await
        {
            Ok(true) => {
                result.taskspace_no_action_recovery_item =
                    Some(build_taskspace_forced_inspect_transition_recovery_item());
            }
            Ok(false) => {}
            Err(error) => {
                sess.send_event(
                    &turn_context,
                    EventMsg::Warning(WarningEvent {
                        message: format!(
                            "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_progress_convergence error={error}"
                        ),
                    }),
                )
                .await;
            }
        }
    }
    if let Ok(result) = &mut outcome
        && result.needs_follow_up
        && result.taskspace_no_action_recovery_item.is_none()
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && matches!(
            snapshot.node_kind.as_deref(),
            Some("smoke_test" | "regression_test")
        )
        && sess
            .action_map_current_validation_node_has_local_infra_failure()
            .await
    {
        result.taskspace_no_action_recovery_item =
            Some(build_taskspace_validation_infra_recovery_item());
    }
    if let Ok(result) = &mut outcome
        && result.needs_follow_up
        && result.taskspace_no_action_recovery_item.is_none()
        && taskspace_progress_before_request.is_some()
    {
        let taskspace_progress_after_request =
            sess.action_map_current_main_node_progress_signature().await;
        if taskspace_progress_after_request == taskspace_progress_before_request {
            if taskspace_message_has_gate_recovery(result.last_agent_message.as_deref()) {
                let mut forced_transition = false;
                let mut repeated_block_bootstrap = false;
                if taskspace_message_has_inspect_duplicate_successful_evidence(
                    result.last_agent_message.as_deref(),
                ) && let Some(snapshot) = provider_budget_snapshot
                    .as_ref()
                    .filter(|snapshot| {
                        snapshot.node_kind.as_deref() == Some("inspect_code_context")
                    })
                    .cloned()
                {
                    let duplicate_read_search_gate = taskspace_message_has_gate_recovery_reason(
                        result.last_agent_message.as_deref(),
                        "inspect_duplicate_successful_read_or_search",
                    );
                    let repeated_blocked_action = taskspace_message_has_repeated_blocked_action(
                        result.last_agent_message.as_deref(),
                    );
                    if repeated_blocked_action && !duplicate_read_search_gate {
                        match sess
                            .force_finish_action_map_inspect_for_provider_budget(
                                &turn_context,
                                snapshot.clone(),
                                "inspect_repeated_blocked_action_with_evidence",
                            )
                            .await
                        {
                            Ok(true) => {
                                result.taskspace_no_action_recovery_item =
                                    Some(build_taskspace_forced_inspect_transition_recovery_item());
                                forced_transition = true;
                            }
                            Ok(false) => {
                                run_taskspace_inspect_bootstrap(
                                    tool_runtime.clone(),
                                    sess.clone(),
                                    turn_context.clone(),
                                    snapshot.request_count,
                                    TASKSPACE_REPEATED_BLOCKED_INSPECT_BOOTSTRAP_CALL_ID,
                                    taskspace_repeated_blocked_inspect_bootstrap_command(),
                                    "TaskSpaceRepeatedBlockedInspectBootstrapV1 executed bounded source/test reads after progress stayed unchanged on a repeated blocked diagnostic command.",
                                    cancellation_token.child_token(),
                                )
                                .await?;
                                repeated_block_bootstrap = true;
                            }
                            Err(error) => {
                                sess.send_event(
                                    &turn_context,
                                    EventMsg::Warning(WarningEvent {
                                        message: format!(
                                            "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_repeated_blocked_action_with_evidence error={error}"
                                        ),
                                    }),
                                )
                                .await;
                            }
                        }
                    } else {
                        let trigger = taskspace_inspect_duplicate_successful_evidence_trigger(
                            result.last_agent_message.as_deref(),
                        );
                        match sess
                            .force_finish_action_map_inspect_for_provider_budget(
                                &turn_context,
                                snapshot.clone(),
                                trigger,
                            )
                            .await
                        {
                            Ok(true) => {
                                result.taskspace_no_action_recovery_item =
                                    Some(build_taskspace_forced_inspect_transition_recovery_item());
                                forced_transition = true;
                            }
                            Ok(false) => {
                                if taskspace_repeated_duplicate_read_search_should_bootstrap(
                                    result.last_agent_message.as_deref(),
                                ) {
                                    if let Some(forced_item) =
                                        run_taskspace_duplicate_read_search_bootstrap_then_force(
                                            tool_runtime.clone(),
                                            sess.clone(),
                                            turn_context.clone(),
                                            snapshot.clone(),
                                            cancellation_token.child_token(),
                                        )
                                        .await?
                                    {
                                        result.taskspace_no_action_recovery_item =
                                            Some(forced_item);
                                        forced_transition = true;
                                    }
                                    repeated_block_bootstrap = true;
                                }
                            }
                            Err(error) => {
                                sess.send_event(
                                        &turn_context,
                                        EventMsg::Warning(WarningEvent {
                                            message: format!(
                                                "TaskSpaceForcedInspectTransitionFailedV1 trigger={trigger} error={error}"
                                            ),
                                        }),
                                    )
                                    .await;
                            }
                        }
                    }
                }
                if !forced_transition && !repeated_block_bootstrap {
                    result.taskspace_no_action_recovery_item = Some(
                        if taskspace_message_has_inspect_duplicate_successful_evidence(
                            result.last_agent_message.as_deref(),
                        ) {
                            build_taskspace_duplicate_inspect_successful_evidence_recovery_item(
                                result.last_agent_message.as_deref(),
                            )
                        } else {
                            build_taskspace_no_action_recovery_item(
                                result.last_agent_message.as_deref(),
                            )
                        },
                    );
                }
            } else if let Some(snapshot) = provider_budget_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.node_kind.as_deref() == Some("inspect_code_context"))
                .cloned()
            {
                if taskspace_message_requests_validation(result.last_agent_message.as_deref()) {
                    run_taskspace_inspect_bootstrap(
                        tool_runtime.clone(),
                        sess.clone(),
                        turn_context.clone(),
                        snapshot.request_count,
                        TASKSPACE_INSPECT_TEST_BOOTSTRAP_CALL_ID,
                        "python -m pytest -q",
                        "TaskSpaceInspectTestBootstrapV1 executed `python -m pytest -q` after inspect_code_context requested validation but produced no tool call.",
                        cancellation_token.child_token(),
                    )
                    .await?;
                } else {
                    let forced_transition = match sess
                        .force_finish_action_map_inspect_for_provider_budget(
                            &turn_context,
                            snapshot.clone(),
                            "inspect_no_action_with_evidence",
                        )
                        .await
                    {
                        Ok(true) => {
                            result.taskspace_no_action_recovery_item =
                                Some(build_taskspace_forced_inspect_transition_recovery_item());
                            true
                        }
                        Ok(false) => false,
                        Err(error) => {
                            sess.send_event(
                                &turn_context,
                                EventMsg::Warning(WarningEvent {
                                    message: format!(
                                        "TaskSpaceForcedInspectTransitionFailedV1 trigger=inspect_no_action_with_evidence error={error}"
                                    ),
                                }),
                            )
                            .await;
                            false
                        }
                    };
                    if !forced_transition {
                        let (call_id_prefix, command, event_message) = (
                            TASKSPACE_INSPECT_BOOTSTRAP_CALL_ID,
                            "rg --files",
                            "TaskSpaceInspectBootstrapV1 executed `rg --files` after inspect_code_context produced no effective action.",
                        );
                        run_taskspace_inspect_bootstrap(
                            tool_runtime.clone(),
                            sess.clone(),
                            turn_context.clone(),
                            snapshot.request_count,
                            call_id_prefix,
                            command,
                            event_message,
                            cancellation_token.child_token(),
                        )
                        .await?;
                    }
                }
            } else {
                result.taskspace_no_action_recovery_item = Some(
                    build_taskspace_no_action_recovery_item(result.last_agent_message.as_deref()),
                );
            }
        }
    }

    if cancellation_token.is_cancelled() {
        return Err(CodexErr::TurnAborted);
    }

    if should_emit_turn_diff {
        let unified_diff = {
            let mut tracker = turn_diff_tracker.lock().await;
            tracker.get_unified_diff()
        };
        if let Ok(Some(unified_diff)) = unified_diff {
            let msg = EventMsg::TurnDiff(TurnDiffEvent { unified_diff });
            sess.clone().send_event(&turn_context, msg).await;
        }
    }

    outcome
}

pub(crate) fn get_last_assistant_message_from_turn(responses: &[ResponseItem]) -> Option<String> {
    for item in responses.iter().rev() {
        if let Some(message) = last_assistant_message_from_item(item, /*plan_mode*/ false) {
            return Some(message);
        }
    }
    None
}
