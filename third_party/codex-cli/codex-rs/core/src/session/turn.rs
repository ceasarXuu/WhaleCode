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
const TASKSPACE_ACTIVE_COMPACT_PROJECTION_MARKER: &str =
    "TaskSpace v0.0.5 active compact projection.";
const TASKSPACE_ACTIVE_THIN_PROJECTION_MARKER: &str = "TaskSpace v0.0.5 active thin projection.";
const TASKSPACE_ACTIVE_PROJECTION_MARKER: &str = "ContextProjectionV1 active replacement:";
const TASKSPACE_SHADOW_PROJECTION_MARKER: &str =
    "ContextProjectionV1 shadow (not active replacement):";
const TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_ITEMS: usize = 3;
const TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_CHARS: usize = 2400;
const TASKSPACE_ACTION_CONTRACT_MAX_SEQUENCE_ACTIONS: usize = 8;
const TASKSPACE_DEEPSEEK_CACHE_ANCHOR_LINES: usize = 4200;
const TASKSPACE_IMPLEMENT_PROGRESS_BEFORE_EDIT_HINT: usize = 10;
#[cfg(test)]
const TASKSPACE_NO_ACTION_RECOVERY_MARKER: &str = "TaskSpaceNoActionRecoveryV1:";
const TASKSPACE_GATE_RECOVERY_MARKER: &str = "TaskSpaceGateRecoveryV1:";
#[cfg(test)]
const TASKSPACE_INSPECT_TRANSITION_AVAILABLE_MARKER: &str =
    "TaskSpaceInspectTransitionAvailableV1:";
#[cfg(test)]
const TASKSPACE_IMPLEMENT_VALIDATION_AVAILABLE_MARKER: &str =
    "TaskSpaceImplementValidationAvailableV1:";
#[cfg(test)]
const TASKSPACE_VALIDATION_CLOSEOUT_AVAILABLE_MARKER: &str =
    "TaskSpaceValidationCloseoutAvailableV1:";
#[cfg(test)]
const TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER: &str = "TaskSpaceImplementNeedsEditRecoveryV1:";
#[cfg(test)]
const TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER: &str =
    "TaskSpaceValidationReworkDuplicateReadRecoveryV1:";
#[cfg(test)]
const TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER: &str =
    "TaskSpaceValidationReworkPatchOnlyRecoveryV1:";
#[cfg(test)]
const TASKSPACE_EDIT_FAILURE_MARKER: &str = "TaskSpaceEditFailureRecoveryV1:";
#[cfg(test)]
const TASKSPACE_APPLY_PATCH_FORMAT_MARKER: &str = "TaskSpaceApplyPatchFormatRecoveryV1:";
#[cfg(test)]
const TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER: &str =
    "TaskSpaceApplyPatchMissingTargetRecoveryV1:";
#[cfg(test)]
const TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER: &str =
    "TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1:";
#[cfg(test)]
const TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER: &str = "TaskSpaceApplyPatchNativeHunkRecoveryV1:";
#[cfg(test)]
const TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER: &str =
    "TaskSpaceApplyPatchReplacementRequiredRecoveryV1:";
#[cfg(test)]
const TASKSPACE_PATCH_INTENT_FORMAT_MARKER: &str = "TaskSpacePatchIntentFormatRecoveryV1:";
#[cfg(test)]
const TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER: &str = "TaskSpaceValidationInfraRecoveryV1:";
#[cfg(test)]
const TASKSPACE_VALIDATION_NODE_FEEDBACK_MARKER: &str = "TaskSpaceValidationNodeFeedbackV1:";
const TASKSPACE_FINAL_READINESS_RECOVERY_MARKER: &str = "TaskSpaceFinalReadinessRecoveryV1:";
#[cfg(test)]
const TASKSPACE_PATH_CORRECTION_MARKER: &str = "TaskSpacePathCorrectionRecoveryV1:";
const TASKSPACE_TOOL_FEEDBACK_MARKER: &str = "TaskSpaceToolFeedbackV1:";
const TASKSPACE_ACTIVE_MAX_RAW_TOOL_OUTPUT_CHARS: usize = 12_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskspaceProviderResponseActionability {
    Actionable,
    NoActionFollowUp,
    ToolFeedbackRecovery,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct TaskSpaceActionSequenceV1 {
    schema_version: String,
    actions: Vec<TaskSpaceActionV1>,
    #[serde(default)]
    rationale: Option<String>,
}

impl TaskspaceProviderResponseActionability {
    fn as_str(self) -> &'static str {
        match self {
            Self::Actionable => "actionable",
            Self::NoActionFollowUp => "no_action_follow_up",
            Self::ToolFeedbackRecovery => "tool_feedback_recovery",
            Self::EmptyFollowUp => "empty_follow_up",
            Self::FinalCandidate => "final_candidate",
            Self::FinalRejected => "final_rejected",
        }
    }

    #[cfg(test)]
    fn needs_recovery(self) -> bool {
        matches!(
            self,
            Self::NoActionFollowUp
                | Self::ToolFeedbackRecovery
                | Self::EmptyFollowUp
                | Self::FinalRejected
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
                } = sampling_request_output;
                can_drain_pending_input = true;
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
    _deepseek_chat: bool,
    configured: &str,
) -> TaskspaceProviderTransportMode {
    match configured.trim().to_ascii_lowercase().as_str() {
        "action_contract"
        | "cache_optimized_action_contract"
        | "cache-optimized-action-contract" => {
            TaskspaceProviderTransportMode::CacheOptimizedActionContract
        }
        _ => TaskspaceProviderTransportMode::NativeTools,
    }
}

fn taskspace_static_action_contract_instructions() -> &'static str {
    "TaskSpaceActionContractV1:
You are running in TaskSpace cache-optimized action-contract transport.
Provider-native tools are intentionally disabled for this request.
Return either one taskspace-action-v1 JSON object or one taskspace-action-sequence-v1 JSON envelope as the assistant message body.
Do not emit markdown fences, DSML tool calls, XML tags, prose before JSON, or prose after JSON.
Single-action JSON shape:
{\"schema_version\":\"taskspace-action-v1\",\"action\":\"<action>\",\"node_id\":\"<active node id or null>\",\"args\":{},\"rationale\":\"short reason\"}
Sequence JSON shape:
{\"schema_version\":\"taskspace-action-sequence-v1\",\"actions\":[{\"schema_version\":\"taskspace-action-v1\",\"action\":\"<action>\",\"node_id\":\"<active node id or null>\",\"args\":{},\"rationale\":\"short reason\"}]}
Sequence execution:
- Runtime executes actions in listed order, using the latest TaskSpace state before each action.
- Runtime stops the sequence after the first rejected action, failed edit/test tool result, failed tool runtime dispatch, blocked terminal action, or accepted terminal final_answer/blocked action.
- Runtime does not reorder, infer, merge, or skip actions.
Action argument rules:
- list_files args: {\"path\":\".\"}
- search args: {\"pattern\":\"literal or regex\",\"path\":\".\"}
- read_file args: {\"path\":\"relative/path\"}; reads text files and supported structured previews such as .parquet with bounded rows/schema.
- apply_patch args: {\"patch\":\"*** Begin Patch\\n...\\n*** End Patch\\n\"}; create new files with native `*** Add File: <path>` plus `+` content lines, and update existing files with `*** Update File: <path>` hunks.
- run_test args: {\"command\":\"test command\",\"timeout_ms\":120000}
- taskspace_control args: {\"action\":\"start_task|finish_node|create_node|bind_node|block_node|record_fact|record_fact_source|record_output_contract|record_success_criteria|state_commit\",...}; use canonical key \"action\", not \"action_name\" or \"command\", for lifecycle commands.
- create_node args include {\"kind\":\"inspect_code_context|implement_solution|smoke_test|regression_test|final_synthesis\",\"title\":\"short title\",\"context_summary\":\"scope\",\"dependency_node_ids\":[\"node-id\"],\"bind_current\":true}
- finish_node args include {\"result_summary\":\"what was completed\",\"next_node_kind\":\"implement_solution|smoke_test|regression_test|final_synthesis\",\"next_node_title\":\"short title\",\"next_node_context_summary\":\"scope\",\"next_dependency_node_ids\":[\"node-id\"]} when the same action should create and bind a next node.
- final_answer args: {\"message\":\"user-facing final answer\"}
- blocked args: {\"reason\":\"exact missing evidence or blocker\"}
Validation invariants:
- Unknown actions or malformed action arguments are protocol errors and no tool will execute.
- If provider-native tool-call markup appears after the JSON object, TaskSpace ignores that markup and executes only the JSON action or action sequence."
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
            "\nhard_state: active_task_without_active_node.\nordinary_tool_boundary: ordinary tools require an active node binding and lease.\nstate_transition_fact: creating or binding another node changes the task path.",
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
No active TaskSpace task exists yet.\n\
required_protocol: taskspace_control(action=start_task) creates the initial task path.\n\
ordinary_tool_boundary: ordinary tools require an active task path, current node binding, and lease."
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
hard_state: active_task_without_active_node.\n\
closure_reason: validation_closed_by_local_infrastructure_evidence.\n\
recorded_blocker_source: exact validator infrastructure blocker and implementation evidence already recorded."
        .to_string();
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_closed_validation_blocker_applies(
    has_blocked_validation_result: bool,
    has_accepted_successful_validation_result: bool,
    has_ready_recovery_node: bool,
) -> bool {
    has_blocked_validation_result
        && !has_accepted_successful_validation_result
        && !has_ready_recovery_node
}

#[cfg(test)]
fn taskspace_completed_task_action_should_force_final_answer(action: &TaskSpaceActionV1) -> bool {
    action.action != "final_answer"
}

fn taskspace_action_contract_tool_runtime_bootstrap_failure_item() -> ResponseItem {
    let text = "TaskSpaceActionContractToolRuntimeBootstrapFailureV1:\n\
Existing TaskSpace task has no active bound node because ordinary tools are blocked by sandbox/tool runtime bootstrap failure evidence.\n\
hard_state: active_task_without_active_node.\n\
closure_reason: ordinary_tools_blocked_by_sandbox_or_tool_runtime_bootstrap_failure.\n\
recorded_blocker_source: exact sandbox/tool runtime blocker and tool failure evidence already recorded."
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
hard_state: inspect_script_reference_without_matching_read_event.\n\
Observed script refs without matching read events:",
    );
    for script in scripts {
        text.push_str("\n- ");
        text.push_str(script);
    }
    if let Some(first) = scripts.first() {
        text.push_str("\nfirst_observed_unread_script_ref: `");
        text.push_str(first);
        text.push_str("`");
    }
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_no_action_recovery_item(last_message: Option<&str>) -> ResponseItem {
    let previous = last_message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("(no assistant text was captured)");
    let gate_recovery = taskspace_gate_recovery_context(previous);
    let text = format!(
        "{TASKSPACE_NO_ACTION_RECOVERY_MARKER}\n\
The previous assistant message requested follow-up but did not produce an actionable TaskSpace item: no tool result, taskspace_control transition, or final response accepted by TaskSpace was recorded.\n\
Previous assistant message: {previous}\n\
{gate_recovery}\
TaskSpace progress forms accepted by the runtime:\n\
- a tool call that records a successful or failed tool result;\n\
- taskspace_control that records a lifecycle or ledger state transition;\n\
- a final response, or a blocked-with-evidence response, accepted by TaskSpace.\n\
This recovery item does not add task semantics beyond the captured previous message and tool feedback."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_final_readiness_recovery_item(last_message: Option<&str>) -> ResponseItem {
    let previous = last_message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("(no final-readiness rejection text was captured)");
    let text = format!(
        "{TASKSPACE_FINAL_READINESS_RECOVERY_MARKER}\n\
final_answer_rejected: true\n\
captured_rejection:\n{previous}\n\
state_baseline:\n\
- final_answer remains rejected until the missing ledger items in captured_rejection are satisfied or waived with evidence.\n\
- This recovery item does not satisfy, waive, or choose evidence for any ledger item.\n\
available_state_record_shape:\n\
- taskspace_control action=state_commit schema_version=taskspace-state-commit-v1\n\
- success_criteria entries may cite explicit id, status, description, kind, and evidence_refs.\n\
- result_validities entries may cite explicit result_id, validity, validity_reason, claims, evidence_refs, changed_artifacts, and validator_refs.\n\
- decisions entries may record an explicit decision and supporting result ids when the agent judges synthesis is ready.\n\
This item preserves the final-readiness gate output as model-visible state feedback."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
This is preserved as the most recent blocked-tool feedback; it does not add guidance beyond the captured tool output.\n"
    )
}

fn taskspace_message_has_gate_recovery(message: Option<&str>) -> bool {
    message.is_some_and(|message| message.contains(TASKSPACE_GATE_RECOVERY_MARKER))
}

#[cfg(test)]
fn taskspace_message_has_state_machine_rejection(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        message.contains("TaskSpaceActionV1 rejected:")
            || message.contains("TaskSpaceFinalAnswerRejectedV1:")
            || message.contains("TaskSpaceBlockedResponseRejectedV1:")
    })
}

#[cfg(test)]
fn taskspace_message_has_gate_recovery_reason(message: Option<&str>, reason: &str) -> bool {
    message.is_some_and(|message| {
        message.contains(TASKSPACE_GATE_RECOVERY_MARKER) && message.contains(reason)
    })
}

#[cfg(test)]
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
Tool feedback facts:\n\
- Native apply_patch `*** Add File` is only for files that do not already exist.\n\
- Existing files use `*** Update File: <path>` hunks with exact existing context and replacement lines.\n\
- Unified diff input for existing files uses `--- a/<path>` and `+++ b/<path>`, never `--- /dev/null`.\n\
Feedback boundary: this item preserves patch-format facts and target locator data; it does not select the next action."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
Tool feedback facts:\n\
- Missing files are created with native `*** Add File: <relative/path>` syntax and `+`-prefixed content lines.\n\
- Already inspected existing artifacts are modified with native `*** Update File: <relative/path>` syntax.\n\
- Native apply_patch add-file syntax does not use `--- /dev/null`, `+++ b/<path>`, or `@@ -0,0 +...` unified-diff headers.\n\
Feedback boundary: this item preserves patch-format facts and target locator data; it does not select whether the target should be created, updated, re-read, or blocked."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
Tool feedback facts:\n\
- In-place native updates need existing context lines plus exact `-old` / `+new` replacement lines.\n\
- Native complete replacement grammar uses `*** Delete File: <path>` followed by `*** Add File: <path>`.\n\
- Shell, Python, or JSON transformation commands are not valid apply_patch payload content.\n\
Feedback boundary: this item preserves patch-format facts and target locator data; it does not select the next action."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
Tool feedback facts for complete replacement:\n\
- Whole-file native replacement grammar uses `*** Delete File: <relative/path>` followed by `*** Add File: <relative/path>` with replacement file contents.\n\
- Every added replacement line must be prefixed with `+`.\n\
- Native replacement payloads do not contain `--- a/...`, `+++ b/...`, or `@@ -old,+new @@` unified-diff range headers.\n\
Feedback boundary: this item preserves patch-format facts and target locator data; it does not select the next action."
    } else {
        "\
Tool feedback facts for native update:\n\
- Native updates use `*** Update File: <relative/path>` with `@@` plus exact existing context and exact `-old` / `+new` lines.\n\
- Unified-diff markers such as `--- a/...`, `+++ b/...`, or `@@ -old,+new @@` do not belong after `*** Update File`.\n\
- Native Update File scaffold when the target line is known:\n\
```text\n\
*** Begin Patch\n\
*** Update File: <relative/path>\n\
@@\n\
 exact existing context line\n\
-old exact line\n\
+new exact line\n\
*** End Patch\n\
```\n\
- Native complete replacement grammar uses `*** Delete File: <relative/path>` followed by `*** Add File: <relative/path>`.\n\
Feedback boundary: this item preserves patch-format facts and target locator data; it does not select whether to update, replace, re-read, or block."
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct TaskspacePathCorrection {
    failed_path: String,
    suggested_relative_path: String,
}

fn taskspace_path_correction_from_response_item(
    item: &ResponseItem,
) -> Option<TaskspacePathCorrection> {
    let output = match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            function_call_output_body_text(&output.body)
        }
        _ => return None,
    };
    taskspace_path_correction_from_text(&output)
}

fn taskspace_path_correction_from_text(text: &str) -> Option<TaskspacePathCorrection> {
    let normalized = text.to_ascii_lowercase();
    let mentions_missing_path = normalized.contains("no such file or directory")
        || normalized.contains("can't read")
        || normalized.contains("cannot access")
        || normalized.contains("could not find item")
        || normalized.contains("cannot find the path");
    if !mentions_missing_path {
        return None;
    }

    ["/data", "/app"]
        .iter()
        .filter_map(|prefix| taskspace_first_absolute_workspace_path(text, prefix))
        .filter_map(|failed_path| {
            taskspace_relative_candidate_for_absolute_workspace_path(&failed_path).map(
                |suggested_relative_path| TaskspacePathCorrection {
                    failed_path,
                    suggested_relative_path,
                },
            )
        })
        .next()
}

fn taskspace_first_absolute_workspace_path(text: &str, prefix: &str) -> Option<String> {
    for (start, _) in text.match_indices(prefix) {
        let tail = &text[start..];
        if let Some(next) = tail[prefix.len()..].chars().next()
            && !taskspace_absolute_workspace_path_boundary(next)
        {
            continue;
        }
        let mut end = tail.len();
        for (index, character) in tail.char_indices() {
            if index > 0
                && (character.is_whitespace()
                    || matches!(
                        character,
                        '"' | '\'' | '`' | '<' | '>' | '|' | '\r' | '\n' | '\t'
                    ))
            {
                end = index;
                break;
            }
        }
        let candidate = tail[..end]
            .trim_matches(|character| {
                matches!(
                    character,
                    ':' | ',' | ';' | '.' | ')' | '(' | '[' | ']' | '{' | '}'
                )
            })
            .to_string();
        if candidate.len() >= prefix.len() {
            return Some(candidate);
        }
    }
    None
}

fn taskspace_absolute_workspace_path_boundary(character: char) -> bool {
    character == '/'
        || character.is_whitespace()
        || matches!(
            character,
            ':' | ','
                | ';'
                | '.'
                | ')'
                | '('
                | '['
                | ']'
                | '{'
                | '}'
                | '"'
                | '\''
                | '`'
                | '<'
                | '>'
                | '|'
                | '\r'
                | '\n'
                | '\t'
        )
}

fn taskspace_relative_candidate_for_absolute_workspace_path(path: &str) -> Option<String> {
    if path == "/app" {
        return Some(".".to_string());
    }
    if path == "/data" {
        return Some("data".to_string());
    }
    path.strip_prefix("/app/")
        .map(ToString::to_string)
        .or_else(|| {
            path.strip_prefix("/data/")
                .map(|relative| format!("data/{relative}"))
        })
        .filter(|path| !path.trim().is_empty())
}

#[cfg(test)]
fn build_taskspace_path_correction_recovery_item(
    correction: &TaskspacePathCorrection,
    node_kind: Option<&str>,
) -> ResponseItem {
    let candidate_path_context = match node_kind {
        Some("inspect_code_context") => format!(
            "inspect_code_context candidate path `{}` can be evaluated as a file or directory path by a state-machine-legal evidence action",
            correction.suggested_relative_path
        ),
        Some("smoke_test" | "regression_test") => format!(
            "validation candidate command/path component `{}` is a workspace-relative replacement for the failed absolute mount",
            correction.suggested_relative_path
        ),
        Some("implement_solution") => format!(
            "implementation candidate path `{}` is a workspace-relative replacement for the failed absolute mount",
            correction.suggested_relative_path
        ),
        _ => format!(
            "workspace-relative candidate path `{}` is available as failed-path feedback",
            correction.suggested_relative_path
        ),
    };
    let text = format!(
        "{TASKSPACE_PATH_CORRECTION_MARKER}\n\
failure_kind: path_not_found_with_relative_candidate\n\
failed_path: {failed_path}\n\
suggested_relative_path: {suggested_relative_path}\n\
The previous tool failed because it used an absolute container path that is not visible in the current workspace. This is failed tool feedback, not successful evidence.\n\
Path correction facts, not a runtime-selected strategy:\n\
- Workspace-relative candidate: `{suggested_relative_path}`.\n\
- Candidate path context: {candidate_path_context}.\n\
- ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
- TaskSpace will not block other state-machine-legal tool actions only because they differ from this suggestion; further failures remain ordinary tool feedback for the Agent to interpret.\n",
        failed_path = correction.failed_path,
        suggested_relative_path = correction.suggested_relative_path,
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn taskspace_action_contract_rejection_followup(reason: &str) -> String {
    format!(
        "TaskSpaceActionV1 rejected: {reason}. Return exactly one valid taskspace-action-v1 JSON object or one valid taskspace-action-sequence-v1 envelope."
    )
}

#[cfg(test)]
fn taskspace_path_correction_retry_reject_reason(
    action: &TaskSpaceActionV1,
    correction: &TaskspacePathCorrection,
) -> Option<String> {
    let _ = (action, correction);
    None
}

#[cfg(test)]
fn taskspace_path_correction_retry_advisory_reason(
    action: &TaskSpaceActionV1,
    correction: &TaskspacePathCorrection,
) -> Option<String> {
    if !matches!(
        action.action.as_str(),
        "list_files" | "read_file" | "search"
    ) {
        return None;
    }
    let path = taskspace_action_arg_string(&action.args, "path")?;
    let suggested_relative_path = if taskspace_same_workspace_path(&path, &correction.failed_path) {
        correction.suggested_relative_path.clone()
    } else if taskspace_workspace_alias_root(&path)
        .is_some_and(|root| Some(root) == taskspace_workspace_alias_root(&correction.failed_path))
    {
        taskspace_relative_candidate_for_absolute_workspace_path(&taskspace_normalize_retry_path(
            &path,
        ))?
    } else if taskspace_path_correction_action_drifted_from_suggestion(
        &path,
        &correction.suggested_relative_path,
    ) {
        correction.suggested_relative_path.clone()
    } else {
        return None;
    };
    Some(format!(
        "path_correction_advisory:{failed_path}:suggested_relative_path={suggested_relative_path}",
        failed_path = taskspace_normalize_retry_path(&path),
    ))
}

#[cfg(test)]
fn taskspace_path_correction_action_drifted_from_suggestion(path: &str, suggestion: &str) -> bool {
    let path = taskspace_normalize_retry_path(path);
    if taskspace_workspace_alias_root(&path).is_some() {
        return false;
    }
    let suggestion = taskspace_normalize_retry_path(suggestion);
    if path.is_empty() || suggestion.is_empty() || path == suggestion {
        return false;
    }
    if path
        .strip_prefix(suggestion.trim_end_matches('/'))
        .is_some_and(|suffix| suffix.starts_with('/'))
    {
        return false;
    }
    matches!(path.as_str(), "." | "./")
}

fn taskspace_action_can_clear_path_correction_feedback(action: &TaskSpaceActionV1) -> bool {
    matches!(
        action.action.as_str(),
        "list_files" | "read_file" | "search"
    )
}

fn taskspace_should_refill_path_correction_from_failed_read_summary(
    tool_path_correction_feedback_present: bool,
    path_correction_cleared_this_request: bool,
    progress_before_request: Option<usize>,
    progress_after_request: Option<usize>,
) -> bool {
    !tool_path_correction_feedback_present
        && !path_correction_cleared_this_request
        && progress_after_request == progress_before_request
}

#[cfg(test)]
fn taskspace_same_workspace_path(left: &str, right: &str) -> bool {
    taskspace_normalize_retry_path(left) == taskspace_normalize_retry_path(right)
}

#[cfg(test)]
fn taskspace_normalize_retry_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches(['"', '\'', '`']);
    if trimmed == "/" {
        return trimmed.to_string();
    }
    trimmed.trim_end_matches('/').to_string()
}

#[cfg(test)]
fn taskspace_workspace_alias_root(path: &str) -> Option<&'static str> {
    let normalized = taskspace_normalize_retry_path(path);
    ["/data", "/app"]
        .into_iter()
        .find(|root| normalized == *root || normalized.starts_with(&format!("{root}/")))
}

#[cfg(test)]
fn build_taskspace_apply_patch_replacement_required_recovery_item(
    targets: &str,
    evidence_summary: Option<&str>,
) -> ResponseItem {
    let targets = targets.trim();
    let targets = if targets.is_empty() {
        "(unknown replacement target)"
    } else {
        targets
    };
    let complete_target_replacement =
        if taskspace_evidence_has_full_visible_validation_rework_target_read(evidence_summary) {
            format!(
                "\nComplete target-read visibility facts:\n\
- The validation rework target has full visible content (content_visibility=full_content_visible).\n\
- The visible target path is `{targets}`.\n\
- Native whole-file replacement grammar for that path is `*** Delete File: {targets}` followed by `*** Add File: {targets}` with `+`-prefixed added lines.\n"
            )
        } else {
            String::new()
        };
    let text = format!(
        "{TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER}\n\
The previous apply_patch used `*** Update File` for: {targets}\n\
The previous validation rework feedback recorded this target as a whole-file replacement candidate.\n\
Tool feedback facts:\n\
- Whole-file native replacement grammar uses `*** Delete File: <relative/path>` followed by `*** Add File: <relative/path>` with replacement file contents.\n\
- Every added replacement line must be prefixed with `+`.\n\
- Native `*** Update File` grammar includes exact existing context and exact `-old` / `+new` lines.\n\
- Native apply_patch payloads do not contain `*** Context Lines`, `---`, `--- a/...`, `+++ b/...`, `--- Update File:`, or `@@ -old,+new @@` unified-diff headers.\n\
Feedback boundary: this item preserves patch-format facts and target locator data; it does not select the next action.\
{complete_target_replacement}"
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
Tool feedback facts:\n\
- TaskSpace accepted neither prose nor a raw patch because this turn requires exactly one valid taskspace-action-v1 JSON object.\n\
- taskspace-action-v1 JSON is the required transport shape; the rejected patch intent remains visible as raw evidence.\n\
- The JSON object must not be wrapped in markdown fences, prose before or after the object, or a second action.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
Already visible evidence remains available as ordinary input."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_inspect_transition_available_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_INSPECT_TRANSITION_AVAILABLE_MARKER}\n\
TaskSpace has recorded inspect_code_context evidence that appears sufficient for a concrete implementation step.\n\
Runtime boundary:\n\
- TaskSpace did not finish the inspect node or create an implementation node automatically.\n\
- The agent remains responsible for the next explicit state-machine-legal action.\n\
state_transition_contract: finish_node can bind implement_solution only if the agent chooses to close inspect_code_context.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
visible_evidence_status: inspected evidence remains available; no implementation node was created by runtime."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_implement_validation_available_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_IMPLEMENT_VALIDATION_AVAILABLE_MARKER}\n\
TaskSpace has recorded a successful implementation edit on the current implement_solution node.\n\
Runtime boundary:\n\
- TaskSpace did not finish the implementation node or create a validation node automatically.\n\
- The agent remains responsible for the next explicit state-machine-legal action.\n\
state_transition_contract: finish_node can bind smoke_test or regression_test only if the agent chooses to close implement_solution.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
visible_evidence_status: successful edit evidence remains available; no validation node was created by runtime."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_validation_closeout_available_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_VALIDATION_CLOSEOUT_AVAILABLE_MARKER}\n\
TaskSpace has recorded a successful validation tool result on the current smoke_test/regression_test node.\n\
Runtime boundary:\n\
- TaskSpace did not finish the validation node or emit final_answer automatically.\n\
- The agent remains responsible for the next explicit state-machine-legal action.\n\
State facts:\n\
- validation node closeout is governed by the active node contract and hard state baseline.\n\
- final_answer is governed by the no-active-node state baseline.\n\
- additional validation commands, if any, are ordinary Agent-selected tool actions."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_validation_infra_recovery_item() -> ResponseItem {
    let text = format!(
        "{TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER}\n\
The latest validation command failed because local validator infrastructure or the host shell failed, not because new code evidence was found.\n\
hard_state: local_validation_infrastructure_failure.\n\
state_record_fact: the failed validation result can be recorded as invalid infrastructure evidence.\n\
recorded_blocker_source: exact local infrastructure evidence, such as Bash/Service/CreateInstance/E_ACCESSDENIED.\n\
duplicate_evidence_boundary: repeated bash, PowerShell, Docker, or shell-discovery commands for the same local validator failure are duplicate infrastructure evidence unless new state/tool evidence changes the failure."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
        "{TASKSPACE_VALIDATION_NODE_FEEDBACK_MARKER}\n\
The current node is smoke_test/regression_test.\n\
{gate_context}\
{last}\
TaskSpace validation-node feedback:\n\
- ordinary tool results remain recorded under the current validation node;\n\
- taskspace_control records remain available for node lifecycle or notes;\n\
- blocked-with-evidence remains available when validation cannot run.\n\
This item preserves prior validation feedback without selecting a command for the Agent."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
            format!("\nAlready inspected evidence visible in current context:\n{bullets}\n")
        })
        .unwrap_or_default();
    let text = format!(
        "{TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER}\n\
TaskSpace implement_solution progress fact: no successful edit has been recorded on the current node yet.\n\
{evidence}\
Runtime boundary:\n\
- This is a state/progress fact, not a closed action space and not a command to edit.\n\
- TaskSpace has not executed or rejected any tool solely because of this fact.\n\
- ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
- Already visible evidence and any later tool results are preserved as ordinary inputs to the agent's next state-machine-legal action."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn build_taskspace_validation_rework_duplicate_read_recovery_item(
    last_message: Option<&str>,
    evidence_summary: Option<&str>,
    failed_edit_summary: Option<&str>,
) -> ResponseItem {
    let previous = last_message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("(no blocked read feedback was captured)");
    let previous_excerpt = taskspace_previous_feedback_excerpt(previous, 2200);
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
feedback_semantics: exact duplicate complete read_file request only\n\
target_artifact: {artifact}\n\
previous_read_result: {previous_result}\n\
{repair_contract}\
{failed_edit}\
The previous action was blocked because this validation rework node already has a complete successful read_file result for the same failure artifact.\n\
Projection boundary:\n\
- This item preserves the blocked tool feedback and visible evidence; it does not select an implementation strategy.\n\
- The prior complete read_file result remains available as `{previous_result}`.\n\
- repair_contract, when present, is copied as an evidence fact rather than a strategy instruction.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
duplicate_complete_read_signal: the same complete read_file request has no recorded new state/tool evidence delta yet.\n\
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

#[cfg(test)]
fn build_taskspace_validation_rework_patch_only_recovery_item(
    last_message: Option<&str>,
    evidence_summary: Option<&str>,
    failed_edit_summary: Option<&str>,
) -> ResponseItem {
    let previous = last_message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("(no blocked action feedback was captured)");
    let previous_excerpt = taskspace_previous_feedback_excerpt(previous, 1600);
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
                "\nComplete target-read visibility facts:\n\
- The target file is already fully visible (content_visibility=full_content_visible).\n\
- Native whole-file replacement grammar for `{target_artifact_label}` is `*** Delete File: {target_artifact_label}` followed by `*** Add File: {target_artifact_label}` with `+`-prefixed added lines.\n"
            )
        } else {
            String::new()
        };
    let schema_contract_facts = taskspace_validation_rework_schema_contract_facts(evidence_summary);
    let schema_contract_visible = if schema_contract_facts.is_empty() {
        String::new()
    } else {
        "schema_contract_evidence_visible=true\n".to_string()
    };
    let text = format!(
        "{TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER}\n\
failure_kind: validation_rework_evidence_after_target_read\n\
target_artifacts: {target_artifact_label}\n\
boundary_mode: evidence_only\n\
{schema_contract_visible}\
TaskSpace preserves the validation failure, target-read result, and schema contract evidence below without selecting a repair strategy.\n\
Available evidence facts:\n\
- The visible validation failure, schema contract, and validation_rework_target_read evidence already shown in context remain available.\n\
- If the validation_rework_target_read evidence says content_visibility=full_content_visible, no additional file lines are hidden for that target in the current projection.\n\
- Exact duplicate read_file for `{target_artifact_label}` is a low-information evidence signal when it adds no new state/tool delta.\n\
- ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
{schema_contract_facts}\
Apply_patch grammar facts:\n\
- Native apply_patch starts with `*** Begin Patch` and ends with `*** End Patch`.\n\
- Native update grammar uses `*** Update File: <target>` plus context lines with `+`/`-` edits.\n\
- Native complete replacement grammar uses `*** Delete File: <target>` followed by `*** Add File: <target>`.\n\
- Patch payload grammar contains patch sections and changed file lines, not markdown fences, shell commands, JSON generation scripts, or prose.\n\
{complete_target_replacement}\
Previous blocked feedback:\n{previous_excerpt}\n\
{failed_edit}\
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

#[cfg(test)]
fn taskspace_validation_rework_schema_contract_facts(evidence_summary: Option<&str>) -> String {
    let Some(evidence_summary) = evidence_summary else {
        return String::new();
    };

    let snippets = evidence_summary
        .split(['\n', '|'])
        .map(str::trim)
        .filter(|segment| {
            !segment.is_empty()
                && (segment.contains("validation_schema_repair_contract")
                    || segment.contains("missing_required_properties")
                    || segment.contains("schema_property_rename_hints")
                    || segment.contains("schema_type_mismatches"))
        })
        .take(6)
        .map(|segment| format!("- {segment}"))
        .collect::<Vec<_>>();

    if snippets.is_empty() {
        return String::new();
    }

    format!(
        "Schema contract evidence snippets copied from current context:\n{}\n",
        snippets.join("\n")
    )
}

#[cfg(test)]
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

#[cfg(test)]
fn build_taskspace_edit_failure_recovery_item(
    failure_summary: Option<&str>,
    evidence_summary: Option<&str>,
) -> ResponseItem {
    let should_force_complete_rewrite = taskspace_failure_expected_lines_mismatch(failure_summary)
        && taskspace_evidence_has_full_visible_validation_rework_target_read(evidence_summary);
    let complete_rewrite = if should_force_complete_rewrite {
        "\nComplete target-read visibility facts:\n- The validation rework target already has full visible target content (content_visibility=full_content_visible).\n- The previous apply_patch failed to find expected lines in that real file snapshot.\n- Native whole-file replacement grammar is `*** Delete File: <path>` followed by `*** Add File: <path>` and `+`-prefixed added lines.\n"
    } else {
        ""
    };
    let patch_format_facts = if should_force_complete_rewrite {
        "- failed_edit_observation: previous hunk failed against a full visible file snapshot.\n\
- patch_format_facts: native apply_patch payloads do not use unified/range hunk headers (`@@ -...`), placeholder hunk headers, markdown fences, shell commands, or prose.\n"
    } else {
        "- failed_edit_observation: previous edit failed; target/context freshness is a fact to derive from visible read summaries and raw tool feedback.\n\
- patch_format_facts: native apply_patch accepts Add File / Update File / Delete File sections according to its grammar; exact tool failure text remains below.\n"
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
	The previous edit tool call failed. This item preserves the raw tool failure, target locator facts, visible-context facts, and patch grammar facts without selecting the next action.\n\
	{failure}\
	{structured_failure}\
	{evidence}\
	{complete_rewrite}\
        Feedback boundary:\n\
- The failed edit result remains part of the visible tool-result history.\n\
- This recovery item does not close the action space; it only exposes the tool failure and grammar facts.\n\
{patch_format_facts}\
		- If the failure says the target file is missing, the already listed/read existing path remains available evidence."
    );

    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
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
                    "path_correction_candidate: corrected=`{corrected}` rejected=`{target}`"
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
        lines.push(
            "tool_feedback_facts: the failed hunk did not match the target snapshot".to_string(),
        );
        lines.push("tool_feedback_locator: failed_hunk_target_snapshot_mismatch".to_string());
        lines.push(
            "context_freshness_source: visible read summaries and raw failed-edit output"
                .to_string(),
        );
    }
    if lines.iter().any(|line| {
        line.contains("apply_patch_native_hunk_header")
            || line.contains("apply_patch_unified_hunk_header_in_native_patch")
    }) {
        lines.push("tool_feedback_facts: native apply_patch rejected unified-diff markers (`--- a/...`, `+++ b/...`, `@@ -old,+new @@`)".to_string());
        lines.push("tool_feedback_locator: native_patch_grammar_rejection".to_string());
    }
    format!("\nStructured failed-edit contract:\n{}\n", lines.join("\n"))
}

#[cfg(test)]
fn taskspace_tool_feedback_field(text: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix(&prefix)?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

#[cfg(test)]
fn taskspace_failure_expected_lines_mismatch(failure_summary: Option<&str>) -> bool {
    failure_summary
        .map(|value| value.to_ascii_lowercase())
        .is_some_and(|value| {
            value.contains("failed to find expected lines")
                || value.contains("apply_patch_expected_lines_mismatch")
                || value.contains("apply_patch_context_mismatch")
        })
}

#[cfg(test)]
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

#[cfg(test)]
fn is_taskspace_no_action_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_NO_ACTION_RECOVERY_MARKER)
}

#[cfg(test)]
fn is_taskspace_path_correction_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_PATH_CORRECTION_MARKER)
}

#[cfg(test)]
fn is_taskspace_validation_rework_duplicate_read_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER)
}

#[cfg(test)]
fn is_taskspace_validation_rework_patch_only_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER)
}

#[cfg(test)]
fn is_taskspace_apply_patch_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_FORMAT_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER)
        || response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER)
}

#[cfg(test)]
fn taskspace_recovery_snapshot_node_key(
    snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) -> String {
    snapshot
        .and_then(|snapshot| snapshot.node_id.as_deref())
        .unwrap_or("unknown-node")
        .to_string()
}

#[cfg(test)]
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

#[cfg(test)]
fn is_taskspace_plain_implement_needs_edit_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER)
}

#[cfg(test)]
fn is_taskspace_implement_needs_edit_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER)
        || response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER)
        || response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER)
        || response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER)
        || response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER)
}

#[cfg(test)]
fn taskspace_implement_recovery_advisory_warning_message(
    item: &ResponseItem,
    attempt: usize,
) -> String {
    if response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER) {
        format!(
            "TaskSpace inserted TaskSpacePatchIntentFormatRecoveryV1 because an apply_patch intent was rejected for non-strict JSON. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_FORMAT_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchFormatRecoveryV1 because apply_patch tried to add an existing file. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchMissingTargetRecoveryV1 because apply_patch tried to update a missing file. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1 because apply_patch used an unanchored Update File patch. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchReplacementRequiredRecoveryV1 because a replacement-oriented patch correction was generated for this target. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceApplyPatchNativeHunkRecoveryV1 because apply_patch mixed native grammar with unified/range hunk syntax. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceEditFailureRecoveryV1 because the previous edit tool call failed. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER)
        && taskspace_duplicate_read_recovery_preserves_patch_grammar_failure(item)
    {
        format!(
            "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1 because validation rework repeated an already-read artifact after patch grammar feedback. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadRecoveryV1 because validation rework already has the target file contents. Advisory recovery attempt {attempt} is being used."
        )
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER) {
        format!(
            "TaskSpace inserted TaskSpaceValidationReworkPatchOnlyRecoveryV1 because validation rework target contents and repair facts are already visible. Advisory recovery attempt {attempt} is being used."
        )
    } else {
        format!(
            "TaskSpace inserted TaskSpaceImplementNeedsEditRecoveryV1 as a progress fact because implementation has no successful edit yet. Fact count for this node: {attempt}."
        )
    }
}

#[cfg(test)]
fn taskspace_duplicate_read_recovery_preserves_patch_grammar_failure(item: &ResponseItem) -> bool {
    response_item_text_contains(item, "Most recent failed edit feedback to preserve")
        && response_item_texts_contain(item, &|text| {
            text.contains("apply_patch_mixed_native_unified")
                || text.contains("apply_patch_native_hunk_header")
                || text.contains("apply_patch_unanchored_update")
                || text.contains("apply_patch_replacement_required")
        })
}

#[cfg(test)]
fn taskspace_special_recovery_warning_message(item: &ResponseItem) -> String {
    if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_FORMAT_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchFormatRecoveryV1 after apply_patch tried to add an existing file. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_MISSING_TARGET_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchMissingTargetRecoveryV1 after apply_patch tried to update a missing file. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_UNANCHORED_UPDATE_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1 after apply_patch used an unanchored Update File patch. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchReplacementRequiredRecoveryV1 after prior validation rework feedback preferred whole-file replacement. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER) {
        "TaskSpace inserted TaskSpaceApplyPatchNativeHunkRecoveryV1 after apply_patch mixed native grammar with unified/range hunk syntax. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_EDIT_FAILURE_MARKER) {
        "TaskSpace inserted TaskSpaceEditFailureRecoveryV1 after an edit tool call failed. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER) {
        "TaskSpace inserted TaskSpaceValidationReworkDuplicateReadRecoveryV1 after a validation rework node repeated an already-read failure artifact. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER) {
        "TaskSpace inserted TaskSpaceValidationReworkPatchOnlyRecoveryV1 after a validation rework node received already-visible target contents and repair facts. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER) {
        "TaskSpace inserted TaskSpaceImplementNeedsEditRecoveryV1 as a non-blocking progress fact. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_PATCH_INTENT_FORMAT_MARKER) {
        "TaskSpace inserted TaskSpacePatchIntentFormatRecoveryV1 after an apply_patch intent was rejected for non-strict JSON. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_INFRA_RECOVERY_MARKER) {
        "TaskSpace inserted TaskSpaceValidationInfraRecoveryV1 after local validator infrastructure failed. This guidance does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_VALIDATION_NODE_FEEDBACK_MARKER) {
        "TaskSpace inserted validation-node feedback after a rejected validation-node action. This feedback does not consume the no-action recovery allowance.".to_string()
    } else if response_item_text_contains(item, TASKSPACE_FINAL_READINESS_RECOVERY_MARKER) {
        "TaskSpace inserted TaskSpaceFinalReadinessRecoveryV1 after final readiness rejected a final_answer. This guidance does not consume the no-action recovery allowance.".to_string()
    } else {
        "TaskSpace inserted non-cap TaskSpace recovery guidance. This guidance does not consume the no-action recovery allowance.".to_string()
    }
}

#[cfg(test)]
fn taskspace_message_hit_implementation_needs_edit(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        message.contains("implementation_needs_edit")
            || message.contains("validation_rework_patch_only_after_target_read")
            || message.contains("validation_rework_evidence_after_target_read")
            || message.contains("validation_rework_closed_action_space_read_disallowed")
            || taskspace_text_mentions_validation_rework_duplicate_artifact_read(message)
            || message.contains("has enough read/search evidence and no successful edit")
    })
}

#[cfg(test)]
fn taskspace_evidence_has_validation_rework_target_read(evidence_summary: Option<&str>) -> bool {
    evidence_summary.is_some_and(|text| text.contains("validation_rework_target_read"))
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn taskspace_message_hit_validation_needs_test(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        (message.contains("node_policy_violation:smoke_test:")
            || message.contains("node_policy_violation:regression_test:"))
            && (message.contains(":list_files")
                || message.contains(":read_file")
                || message.contains(":search"))
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

#[cfg(test)]
fn taskspace_message_hit_apply_patch_intent_format_rejection(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        message.contains("action_contract_output_not_strict_json:apply_patch_intent")
    })
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

fn classify_taskspace_provider_response_actionability(
    needs_follow_up: bool,
    saw_actionable_output: bool,
    assistant_message_present: bool,
    gate_recovery_message_present: bool,
    tool_failure_recovery_message_present: bool,
    final_response_rejected: bool,
    _provider_budget_exhausted_followup: bool,
) -> TaskspaceProviderResponseActionability {
    if final_response_rejected {
        TaskspaceProviderResponseActionability::FinalRejected
    } else if saw_actionable_output
        && (gate_recovery_message_present || tool_failure_recovery_message_present)
    {
        TaskspaceProviderResponseActionability::ToolFeedbackRecovery
    } else if gate_recovery_message_present {
        TaskspaceProviderResponseActionability::NoActionFollowUp
    } else if tool_failure_recovery_message_present {
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
        let tool_visibility = match transport_mode {
            TaskspaceProviderTransportMode::NativeTools => TaskspaceProviderToolVisibility::All,
            TaskspaceProviderTransportMode::CacheOptimizedActionContract => {
                TaskspaceProviderToolVisibility::None
            }
        };
        if let Some(snapshot) = provider_budget_snapshot.as_ref()
            && transport_mode == TaskspaceProviderTransportMode::CacheOptimizedActionContract
        {
            prompt_input.push(taskspace_action_contract_state_item(snapshot));
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
    _current_node_kind: Option<&str>,
) -> Vec<ResponseItem> {
    let mut latest_user_input: Option<(usize, ResponseItem)> = None;
    let mut latest_taskspace_context: Option<(usize, ResponseItem)> = None;
    let mut tool_outputs: Vec<(usize, ResponseItem)> = Vec::new();

    for (index, item) in items.into_iter().enumerate() {
        if is_taskspace_active_context_item(&item) {
            latest_taskspace_context = Some((index, item));
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
    if let Some((_, item)) = latest_taskspace_context {
        prepared.push(item);
    }
    let recent_tool_outputs = tool_outputs
        .into_iter()
        .filter_map(|(index, item)| (index > latest_user_index).then_some(item))
        .collect::<Vec<_>>();
    if let Some(item) = taskspace_action_contract_recent_tool_outputs_item(&recent_tool_outputs) {
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
        || response_item_text_contains_taskspace_active_marker(item)
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
    let action_contract_tool_output = response_item_tool_call_id(item)
        .is_some_and(|call_id| call_id.starts_with("taskspace-action-contract-"));
    is_tool_output_item(item)
        && (action_contract_tool_output
            || !is_legacy_taskspace_tool_output(item)
            || is_actionable_taskspace_gate_feedback_output(item))
}

fn is_actionable_taskspace_gate_feedback_output(item: &ResponseItem) -> bool {
    (response_item_text_contains(item, "high-signal inspected evidence")
        && response_item_text_contains(item, "uncovered"))
        || response_item_texts_contain(item, &|text| {
            taskspace_text_mentions_current_validation_test_required(text)
        })
        || (response_item_texts_contain(item, &|text| {
            taskspace_missing_command_script_from_text(text).is_some()
        }))
        || response_item_text_contains(item, "required_validator:python scripts/validate.py")
        || (response_item_text_contains(item, "still unreviewed")
            && response_item_text_contains(item, "result_validities"))
        || (response_item_text_contains(
            item,
            "cannot be completed without a recorded successful edit action",
        ) && response_item_text_contains(item, "Execute the edit in this node"))
        || response_item_texts_contain(item, &|text| {
            taskspace_output_mentions_local_validator_infra_state_commit(text)
        })
}

fn taskspace_text_mentions_obsolete_runtime_boundary_strategy_feedback(text: &str) -> bool {
    (text.contains("cannot be blocked for a missing diagnostic prerequisite")
        && text.contains("already recorded successful diagnostic evidence"))
        || (text.contains("cannot be blocked for an internal node-policy")
            && text.contains("inspected implementation evidence is already available"))
        || (text.contains("cannot be blocked for missing source visibility")
            && (text.contains("already recorded implementation source evidence")
                || text.contains(
                    "dependency evidence already identifies the implementation artifact or validation rework target",
                )))
        || (text.contains("cannot be blocked for validator procedure")
            && text.contains("implementation failure"))
        || (text.contains("cannot be blocked for editable validation failure")
            && text.contains("failed validation evidence"))
}

#[cfg(test)]
fn taskspace_previous_feedback_excerpt(previous: &str, max_chars: usize) -> String {
    let trimmed = previous.trim();
    if taskspace_text_mentions_obsolete_runtime_boundary_strategy_feedback(trimmed)
        || taskspace_text_mentions_projection_strategy_injection(trimmed)
    {
        return "Previous runtime-boundary strategy text omitted because it selected an implementation strategy. Structured fields above preserve the failure kind, target artifact, prior result, repair contract, and evidence references that remain valid."
            .to_string();
    }

    trimmed.chars().take(max_chars).collect::<String>()
}

#[cfg(test)]
fn taskspace_text_mentions_projection_strategy_injection(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let snake_next_action = ["next_valid", "_action:"].concat();
    let prose_next_action = ["next valid", " action:"].concat();
    ((lower.contains(&snake_next_action) || lower.contains(&prose_next_action))
        && (lower.contains("apply_patch")
            || lower.contains("run_test")
            || lower.contains("taskspace_control")
            || lower.contains("block_node")))
        || (lower.contains("do not call")
            && (lower.contains("read_file")
                || lower.contains("list_files")
                || lower.contains("search")
                || lower.contains("apply_patch")))
        || (lower.contains("current required behavior:") && lower.contains("apply_patch"))
        || lower.contains("read/search is no longer a valid next action")
}

fn taskspace_action_contract_recent_tool_outputs_item(
    items: &[ResponseItem],
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
    let mut remaining_chars = TASKSPACE_ACTION_CONTRACT_MAX_RECENT_TOOL_OUTPUT_CHARS;
    let mut sections = Vec::new();
    for (call_id, text) in summaries.into_iter().rev() {
        if remaining_chars == 0 {
            break;
        }
        let char_count = text.chars().count();
        let allowed_chars = remaining_chars;
        let mut output = text.chars().take(remaining_chars).collect::<String>();
        let truncated = char_count > remaining_chars;
        let omitted_chars = char_count.saturating_sub(allowed_chars);
        if truncated {
            output.push_str("\n[truncated]");
            output = append_taskspace_tool_tail_sentinels(output, &text);
            remaining_chars = 0;
        } else {
            remaining_chars = remaining_chars.saturating_sub(char_count);
        }
        sections.push(format!(
            "call_id: {call_id}\noutput_chars: {char_count}\noutput_visible_chars: {}\noutput_truncated: {truncated}\noutput_omitted_chars: {omitted_chars}\noutput:\n{output}",
            if truncated { allowed_chars } else { char_count }
        ));
    }
    if sections.is_empty() {
        return None;
    }

    let text = format!(
        "TaskSpaceActionContractRecentToolOutputsV1:\n{}",
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
    let (call_id, text) = match item {
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        }
        | ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => (
            call_id.as_str(),
            function_call_output_body_text(&output.body),
        ),
        _ => return None,
    };
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    Some((call_id.to_string(), text.to_string()))
}

#[cfg(test)]
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
hard_state: validation_node_without_current_test_or_build_result.\n\
tool_feedback_facts: the current validation node has no current same-node test/build result recorded.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
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
hard_state: validation_node_without_current_test_or_build_result.\n\
tool_feedback_facts: the current validation node has no current same-node test/build result recorded.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
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
feedback_semantics: duplicate evidence only; the previous result already contains the target contents for `{artifact}`.\n\
available_evidence: previous_read_result `{previous_result}` remains visible.\n\
raw_output:\n{text}"
        );
    }
    if taskspace_text_mentions_obsolete_runtime_boundary_strategy_feedback(text) {
        return format!(
            "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: blocked\n\
failure_kind: obsolete_runtime_boundary_strategy_feedback\n\
feedback_semantics: a previous runtime version rejected block_node using implementation-strategy heuristics. Current projection omits that obsolete strategy text and does not select the next action.\n\
raw_output_omitted: true"
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
tool_feedback_facts: no successful edit result is recorded on the current implementation node.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
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
tool_feedback_facts: native apply_patch grammar rejected a mixed unified-diff hunk/header for the listed target(s).\n\
tool_feedback_locator: target_path={targets}; raw_error_preserved=true; grammar_error=native_hunk_header\n\
patch_format_facts: native apply_patch grammar uses `*** Begin Patch` / file sections / `*** End Patch`; unified-diff range headers are rejected by this tool.\n\
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
tool_feedback_facts: apply_patch tried to update a target file that the tool could not read.\n\
tool_feedback_locator: target_path={target}; raw_error_preserved=true; target_read_status=missing_for_update\n\
patch_format_facts: native `*** Add File` and `*** Update File` are different tool grammar forms; this field does not decide which semantic action applies.\n\
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
tool_feedback_facts: apply_patch could not find the expected existing lines in `{target}`.\n\
tool_feedback_locator: target_path={target}; raw_error_preserved=true; expected_lines_present_in_tool_error=true\n\
content_visibility_source: current read summaries and raw tool feedback; this field does not infer whether context is stale.\n\
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
tool_feedback_facts: apply_patch context did not match `{target}`; unified-diff range headers are not native apply_patch hunks.\n\
tool_feedback_locator: target_path={target}; raw_error_preserved=true; context_mismatch_reported=true\n\
content_visibility_source: current read summaries and raw tool feedback; this field does not infer whether context is stale.\n\
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
tool_feedback_facts: apply_patch rejected a unified-diff range header in native patch input.\n\
tool_feedback_locator: raw_error_preserved=true; grammar_error=unified_hunk_header_in_native_patch\n\
patch_format_facts: native apply_patch grammar rejects unified-diff range headers; this field does not select the next action.\n\
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
tool_feedback_facts: the validation command references a script path that the shell cannot open.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
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
hard_state: result_validity_unreviewed_for_dependent_record.\n\
tool_feedback_facts: result `{result_id}` is still unreviewed in node `{node_id}`.\n\
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
hard_state: implementation_node_without_successful_edit_result.\n\
tool_feedback_facts: finish_node for this implementation node has no successful edit result recorded.\n\
raw_output:\n{text}"
        );
    }
    format!(
        "{TASKSPACE_TOOL_FEEDBACK_MARKER}\n\
tool_source: action_contract_internal\n\
tool_action: {action}\n\
tool_result: failed\n\
failure_kind: tool_execution_failed\n\
feedback_semantics: the tool result is a failed action result and remains available as raw evidence.\n\
ordinary_tool_boundary: ordinary tool use remains governed by the active node binding and lease.\n\
raw_output:\n{text}"
    )
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
    FinalReadinessRecovery,
    CurrentTaskspaceRuntimeFeedback,
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

    let current_feedback_tool_call_ids = items
        .iter()
        .filter(|item| is_current_taskspace_runtime_feedback_item(item))
        .filter_map(response_item_tool_call_id)
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let classified_items = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let category = classify_provider_visible_item(&item);
            let action = if response_item_tool_call_id(&item)
                .is_some_and(|call_id| current_feedback_tool_call_ids.contains(call_id))
            {
                ProviderVisibleHistoryAction::Include
            } else {
                provider_visible_history_action(&category)
            };
            (index, item, category, action)
        })
        .collect::<Vec<_>>();
    let paired_omitted_tool_call_ids =
        omitted_provider_visible_tool_call_ids(classified_items.as_slice());
    let latest_final_readiness_recovery_index = classified_items
        .iter()
        .filter(|(_, _, category, _)| {
            matches!(
                category,
                ProviderVisibleItemCategory::FinalReadinessRecovery
            )
        })
        .map(|(index, _, _, _)| *index)
        .next_back();
    let latest_active_projection_item = classified_items
        .iter()
        .filter(|(_, _, category, _)| {
            matches!(category, ProviderVisibleItemCategory::ActiveProjection)
        })
        .map(|(_, item, _, _)| item.clone())
        .next_back();

    let mut prepared = Vec::with_capacity(classified_items.len());
    let mut latest_final_readiness_recovery_item: Option<ResponseItem> = None;
    let mut decisions = Vec::with_capacity(classified_items.len());
    for (index, item, category, base_action) in classified_items {
        let mut action =
            provider_visible_history_pair_action(&item, base_action, &paired_omitted_tool_call_ids);
        if matches!(
            category,
            ProviderVisibleItemCategory::FinalReadinessRecovery
        ) {
            if latest_final_readiness_recovery_index == Some(index) {
                if taskspace_final_readiness_recovery_still_applies(
                    &item,
                    latest_active_projection_item.as_ref(),
                ) {
                    latest_final_readiness_recovery_item = Some(item);
                } else {
                    action = ProviderVisibleHistoryAction::Omit(
                        "stale_final_readiness_recovery_satisfied_by_projection",
                    );
                }
            } else {
                action =
                    ProviderVisibleHistoryAction::Omit("stale_final_readiness_recovery_replaced");
            }
            decisions.push(ProviderVisibleHistoryDecision {
                index,
                category,
                action,
            });
            continue;
        }
        if matches!(action, ProviderVisibleHistoryAction::Include) {
            prepared.push(item);
        }
        decisions.push(ProviderVisibleHistoryDecision {
            index,
            category,
            action,
        });
    }
    if let Some(item) = latest_final_readiness_recovery_item {
        prepared.push(item);
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
        | ProviderVisibleItemCategory::FinalReadinessRecovery
        | ProviderVisibleItemCategory::CurrentTaskspaceRuntimeFeedback
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
    if is_taskspace_final_readiness_recovery_item(item) {
        return ProviderVisibleItemCategory::FinalReadinessRecovery;
    }
    if is_current_taskspace_runtime_feedback_item(item) {
        return ProviderVisibleItemCategory::CurrentTaskspaceRuntimeFeedback;
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
    response_item_text_contains(item, TASKSPACE_ACTIVE_PROJECTION_MARKER)
}

fn response_item_text_contains_taskspace_active_marker(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_ACTIVE_PROFILE_MARKER)
        || response_item_text_contains(item, TASKSPACE_ACTIVE_COMPACT_PROJECTION_MARKER)
        || response_item_text_contains(item, TASKSPACE_ACTIVE_THIN_PROJECTION_MARKER)
}

fn is_protected_user_input(item: &ResponseItem) -> bool {
    matches!(item, ResponseItem::Message { role, .. } if role == "user")
}

fn is_protected_developer_or_system_input(item: &ResponseItem) -> bool {
    matches!(item, ResponseItem::Message { role, .. } if role == "developer" || role == "system")
}

fn is_legacy_taskspace_instruction(item: &ResponseItem) -> bool {
    if is_taskspace_final_readiness_recovery_item(item) {
        return false;
    }
    response_item_text_contains(item, "TaskSpace mode is now active")
        || response_item_text_contains(item, "TaskSpace final answer gate rejected")
        || response_item_text_contains(item, "taskspace_control(")
}

fn is_taskspace_final_readiness_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_FINAL_READINESS_RECOVERY_MARKER)
}

fn taskspace_final_readiness_recovery_still_applies(
    recovery: &ResponseItem,
    active_context: Option<&ResponseItem>,
) -> bool {
    let Some(recovery_text) = response_item_text(recovery) else {
        return true;
    };
    let missing_ids = taskspace_final_readiness_missing_ledger_ids(&recovery_text);
    if missing_ids.is_empty() {
        return true;
    }
    let Some(context_text) = active_context.and_then(response_item_text) else {
        return true;
    };
    !taskspace_final_readiness_missing_ids_closed_in_projection(&missing_ids, &context_text)
}

fn taskspace_final_readiness_missing_ledger_ids(text: &str) -> Vec<String> {
    let Some((_, after_marker)) = text.split_once("Missing ledger items:") else {
        return Vec::new();
    };
    let relevant = after_marker
        .split("Recent result refs")
        .next()
        .unwrap_or(after_marker);
    let mut ids = Vec::new();
    for token in relevant
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | ',' | '.'))
        .filter_map(|token| token.strip_prefix("id="))
    {
        let id = token.trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ':' | ')' | '('));
        if !id.is_empty() && !ids.iter().any(|existing| existing == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

fn taskspace_final_readiness_missing_ids_closed_in_projection(
    missing_ids: &[String],
    projection_text: &str,
) -> bool {
    missing_ids.iter().all(|id| {
        projection_text.lines().any(|line| {
            let line = line.trim_start();
            if !line.starts_with("- ") {
                return false;
            }
            let mut parts = line.trim_start_matches("- ").split_whitespace();
            let Some(line_id) = parts.next() else {
                return false;
            };
            if line_id != id {
                return false;
            }
            parts.any(|part| matches!(part, "status=satisfied" | "status=waived"))
        })
    })
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

fn is_current_taskspace_runtime_feedback_item(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. }
    ) && [
        TASKSPACE_GATE_RECOVERY_MARKER,
        "TaskSpaceFinalAnswerRejectedV1:",
        "TaskSpaceBlockedRejectedV1:",
    ]
    .iter()
    .any(|marker| response_item_text_contains(item, marker))
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

fn taskspace_sequence_failure_feedback_from_response_item(
    action_name: &str,
    item: &ResponseItem,
) -> Option<String> {
    let text = response_item_text(item)?;
    let failed = match action_name {
        "apply_patch" => text.contains("apply_patch verification failed"),
        "run_test" => taskspace_shell_output_has_nonzero_exit_code(&text),
        _ => false,
    };
    failed.then(|| text.chars().take(2200).collect::<String>())
}

fn taskspace_shell_output_has_nonzero_exit_code(text: &str) -> bool {
    text.lines().any(|line| {
        let Some(code) = line.trim().strip_prefix("Exit code: ") else {
            return false;
        };
        code.split_whitespace()
            .next()
            .is_some_and(|code| code != "0")
    })
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
            route_mode: Some("thin".to_string()),
            profile_name: Some("taskspace-v005-active".to_string()),
            request_phase: Some("model_sampling".to_string()),
            provider_request_context_missing_reason: None,
            request_count: 1,
            max_requests: 8,
            node_request_count: 0,
            max_model_requests_per_node: 3,
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
    fn taskspace_action_contract_parser_accepts_action_sequence() {
        let actions = parse_taskspace_actions_v1(
            r#"{"schema_version":"taskspace-action-sequence-v1","actions":[{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/lib.rs"},"rationale":"inspect lib"},{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/main.rs"},"rationale":"inspect main"}],"rationale":"read independent files"}"#,
        )
        .expect("valid action sequence");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "read_file");
        assert_eq!(
            taskspace_action_arg_string(&actions[1].args, "path").as_deref(),
            Some("src/main.rs")
        );
        assert_eq!(
            parse_taskspace_action_v1(
                r#"{"schema_version":"taskspace-action-sequence-v1","actions":[{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/lib.rs"}},{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/main.rs"}}]}"#
            )
            .expect_err("multi-action envelope is not a single action"),
            "action_sequence_not_single_action"
        );
    }

    #[test]
    fn taskspace_action_contract_sequence_call_ids_are_unique() {
        let snapshot = provider_snapshot("inspect_code_context");
        let first = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/lib.rs"}}"#,
        )
        .expect("valid first action");
        let second = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"src/main.rs"}}"#,
        )
        .expect("valid second action");

        let first_call =
            taskspace_action_to_tool_call_with_sequence_index(&first, &snapshot, Some(1))
                .expect("first action maps")
                .expect("first tool call");
        let second_call =
            taskspace_action_to_tool_call_with_sequence_index(&second, &snapshot, Some(2))
                .expect("second action maps")
                .expect("second tool call");

        assert_ne!(first_call.call_id, second_call.call_id);
        assert_eq!(
            taskspace_action_contract_action_name_from_call_id(&first_call.call_id),
            Some("read_file")
        );
        assert_eq!(
            taskspace_action_contract_action_name_from_call_id(&second_call.call_id),
            Some("read_file")
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
        assert!(text.contains("hard_state: active_task_without_active_node"));
        assert!(text.contains("ordinary_tool_boundary: ordinary tools require an active node"));
        assert!(text.contains("state_transition_fact:"));
        assert!(!text.contains("action=start_task"));
        assert!(!text.contains("state_machine_allowed_actions"));
        assert!(!text.contains("Do not create"));
    }

    #[test]
    fn taskspace_action_contract_closed_validation_forbids_new_nodes() {
        let text = item_text(taskspace_action_contract_closed_validation_item());

        assert!(text.contains("TaskSpaceActionContractClosedValidationV1"));
        assert!(text.contains("hard_state: active_task_without_active_node"));
        assert!(
            text.contains("closure_reason: validation_closed_by_local_infrastructure_evidence")
        );
        assert!(text.contains("recorded_blocker_source"));
        assert!(!text.contains("state_machine_allowed_actions"));
        assert!(!text.contains("rejected_by_state_baseline"));
        assert!(!text.contains("Do not call start_task"));
        assert!(text.contains("validator infrastructure blocker"));
    }

    #[test]
    fn closed_validation_blocker_is_suppressed_after_successful_validation() {
        assert!(taskspace_closed_validation_blocker_applies(
            true, false, false
        ));
        assert!(!taskspace_closed_validation_blocker_applies(
            true, true, false
        ));
        assert!(!taskspace_closed_validation_blocker_applies(
            true, false, true
        ));
        assert!(!taskspace_closed_validation_blocker_applies(
            false, false, false
        ));
    }

    #[test]
    fn taskspace_action_contract_tool_runtime_bootstrap_failure_forbids_new_nodes() {
        let text = item_text(taskspace_action_contract_tool_runtime_bootstrap_failure_item());

        assert!(text.contains("TaskSpaceActionContractToolRuntimeBootstrapFailureV1"));
        assert!(text.contains("hard_state: active_task_without_active_node"));
        assert!(text.contains(
            "closure_reason: ordinary_tools_blocked_by_sandbox_or_tool_runtime_bootstrap_failure"
        ));
        assert!(text.contains("recorded_blocker_source"));
        assert!(!text.contains("state_machine_allowed_actions"));
        assert!(!text.contains("rejected_by_state_baseline"));
        assert!(!text.contains("Do not call start_task"));
        assert!(text.contains("sandbox/tool runtime blocker"));
    }

    #[test]
    fn taskspace_action_contract_inspect_missing_scripts_narrows_to_read_file() {
        let scripts = vec!["generate_report.sh".to_string()];
        let text = item_text(taskspace_action_contract_inspect_unread_scripts_item(
            &scripts,
        ));

        assert!(text.contains("TaskSpaceActionContractInspectMissingScriptsV1"));
        assert!(text.contains("hard_state: inspect_script_reference_without_matching_read_event"));
        assert!(text.contains("generate_report.sh"));
        assert!(text.contains("first_observed_unread_script_ref"));
        assert!(!text.contains("state_machine_allowed_actions"));
        assert!(!text.contains("rejected_by_state_baseline"));
        assert!(!text.contains("The next action must be read_file"));
        assert!(!text.contains("Do not call list_files"));
    }

    #[test]
    fn taskspace_boundary_feedback_items_do_not_emit_strategy_labels() {
        let mut validation_snapshot = provider_snapshot("smoke_test");
        validation_snapshot.node_id = Some("node-9".to_string());
        let samples = vec![
            item_text(taskspace_action_contract_state_item(&validation_snapshot)),
            item_text(taskspace_action_contract_closed_validation_item()),
            item_text(taskspace_action_contract_tool_runtime_bootstrap_failure_item()),
            item_text(taskspace_action_contract_inspect_unread_scripts_item(&[
                "generate_report.sh".to_string(),
            ])),
            item_text(build_taskspace_inspect_transition_available_item()),
            item_text(build_taskspace_implement_validation_available_item()),
            item_text(build_taskspace_validation_closeout_available_item()),
        ];
        let forbidden = [
            "Available next actions",
            "Do not call",
            "Current request allowed actions are narrowed",
            "Next valid action",
            "Preferred fix",
            "rejected_by_state_baseline",
            "state_machine_allowed_actions",
            "state_machine_requirement",
            "validation_command_source",
            "validation_needs_test",
            "action_space_source",
            "Action-space source",
            "The next action must",
            "Suggested recovery",
            "Suggested action",
        ];

        for text in samples {
            for phrase in forbidden {
                assert!(
                    !text.contains(phrase),
                    "boundary feedback contains strategy phrase `{phrase}` in:\n{text}"
                );
            }
        }
    }

    #[test]
    fn taskspace_action_contract_state_is_mechanical_after_implementation_progress() {
        let mut snapshot = provider_snapshot("implement_solution");

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Active node id: node-1"));
        assert!(text.contains("Active node kind: implement_solution"));
        assert!(!text.contains("Implementation node event status"));
        assert!(!text.contains("implementation_needs_edit"));
        let text_after_edit = item_text(taskspace_action_contract_state_item(&snapshot));
        assert!(!text_after_edit.contains("no successful edit event is recorded"));
    }

    #[test]
    fn taskspace_action_contract_state_does_not_interpret_dependency_evidence() {
        let mut snapshot = provider_snapshot("implement_solution");

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Active node kind: implement_solution"));
        assert!(!text.contains("Implementation node event status"));

        let read = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_report.sh"},"rationale":"read more"}"#,
        )
        .expect("valid read action");
        let call = taskspace_action_to_tool_call(&read, &snapshot)
            .expect("implementation evidence fact should not close read action space")
            .expect("read_file maps to shell command");
        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn taskspace_action_contract_state_does_not_project_mandatory_evidence_strategy() {
        let mut snapshot = provider_snapshot("implement_solution");

        let text = item_text(taskspace_action_contract_state_item(&snapshot));

        assert!(text.contains("Active node kind: implement_solution"));
        assert!(!text.contains("Uncovered mandatory evidence refs"));
        assert!(!text.contains("patch target coverage"));

        let read = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"collect_data.sh"},"rationale":"read more"}"#,
        )
        .expect("valid read action");
        let call = taskspace_action_to_tool_call(&read, &snapshot)
            .expect("mandatory evidence pressure should not close read action space")
            .expect("read_file maps to shell command");
        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn taskspace_apply_patch_is_not_rejected_by_semantic_evidence_coverage() {
        let mut snapshot = provider_snapshot("implement_solution");

        let text = item_text(taskspace_action_contract_state_item(&snapshot));
        assert!(!text.contains("Uncovered mandatory evidence refs"));
        assert!(!text.contains("generate_report.sh (invalid_shebang, result-13)"));

        let wrong = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n*** Update File: report_generation.sh\n@@\n-#!/bin/nonexistent\n+#!/bin/bash\n*** End Patch\n"},"rationale":"fix shebang"}"#,
        )
        .expect("valid wrong patch action");
        taskspace_action_to_tool_call(&wrong, &snapshot)
            .expect("semantic evidence coverage must not reject Agent patch intent")
            .expect("tool call");

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
    fn taskspace_action_contract_bootstrap_accepts_top_level_start_task_alias() {
        let start_task = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"start_task","node_id":null,"args":{"initial_success_criteria":["Generate /app/merged_users.parquet","Generate /app/conflicts.json"],"initial_output_contracts":["/app/merged_users.parquet","/app/conflicts.json"],"initial_fact_sources":["/data/source_a/users.json","/data/source_b/users.csv","/data/source_c/users.parquet"]},"rationale":"Bootstrap the merge task."}"#,
        )
        .expect("valid top-level lifecycle action");
        let call = taskspace_bootstrap_action_to_tool_call(&start_task)
            .expect("top-level start_task should normalize to taskspace_control")
            .expect("start_task should execute");

        assert_eq!(call.tool_name.name, "taskspace_control");
        match call.payload {
            ToolPayload::Function { arguments } => {
                let value: serde_json::Value = serde_json::from_str(&arguments).expect("json");
                assert_eq!(value["action"], "start_task");
                assert_eq!(
                    value["initial_output_contracts"][0],
                    "/app/merged_users.parquet"
                );
                assert_eq!(
                    value["initial_fact_sources"][2],
                    "/data/source_c/users.parquet"
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_canonicalizes_natural_start_task_aliases() {
        let start_task = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"taskspace_control","node_id":null,"args":{"action":"start_task","task_description":"Create a JSON processor that transforms CSV data into organization.json following schema.json.","initial_criteria":["Read schema.json","Verify organization.json structure matches schema"],"initial_contracts":["organization.json file with correct structure and data"],"initial_fact_sources":["schema.json","departments.csv"],"first_node_kind":"inspect_code_context","first_node_description":"Explore the provided CSV files and schema.json."},"rationale":"Start task and inspect inputs."}"#,
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
                assert_eq!(
                    value["task_objective"],
                    "Create a JSON processor that transforms CSV data into organization.json following schema.json."
                );
                assert_eq!(value["node_kind"], "inspect_code_context");
                assert_eq!(
                    value["node_context_summary"],
                    "Explore the provided CSV files and schema.json."
                );
                assert!(value.get("task_description").is_none());
                assert!(value.get("initial_criteria").is_none());
                assert!(value.get("initial_contracts").is_none());
                assert!(value.get("first_node_kind").is_none());
                assert!(value.get("first_node_description").is_none());
                assert!(value["initial_success_criteria"].is_array());
                assert!(value["initial_output_contracts"].is_array());
                assert!(value["initial_fact_sources"].is_array());
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
    fn taskspace_action_contract_parser_recovers_sequence_after_prose_with_braces() {
        let actions = parse_taskspace_actions_v1(
            r#"I can see the issue: `format_depth()` returns `f"depth: {count_stack_depth()}"`.

{"schema_version":"taskspace-action-sequence-v1","actions":[{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-2","args":{"patch":"*** Update File: src/call_stack_counter.py\n@@\n def format_depth() -> str:\n-    return f\"depth: {count_stack_depth()}\"\n+    return f\"CALL_STACK_DEPTH={count_stack_depth()}\"\n"},"rationale":"fix output"},{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-2","args":{"command":"python scripts/validate.py","timeout_ms":30000},"rationale":"validate"}]}"#,
        )
        .expect("json sequence after prose prefix should be recoverable");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "apply_patch");
        assert_eq!(actions[1].action, "run_test");
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
    fn taskspace_action_contract_policy_allows_tests_in_implementation_node() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"run_test","node_id":"node-1","args":{"command":"cargo test"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("implementation nodes allow ordinary test execution feedback")
            .expect("run_test should map to shell command");

        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn taskspace_action_contract_keeps_late_implementation_reads_open() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_report.sh"}}"#,
        )
        .expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("implementation convergence fact should not block reads")
            .expect("read_file maps to a shell command");
        assert_eq!(call.tool_name.name, "shell_command");
        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("successful edit keeps read action available")
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

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("named validation rework target read should be allowed")
            .expect("read_file maps to a shell command");
        assert_eq!(call.tool_name.name, "shell_command");

        let broad_read = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"schema.json"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&broad_read, &snapshot)
            .expect("non-target read remains available under implement_solution")
            .expect("read_file maps to shell command");
        assert_eq!(call.tool_name.name, "shell_command");

        let state_text = item_text(taskspace_action_contract_state_item(&snapshot));
        assert!(state_text.contains("generate_org.py"));
        assert!(state_text.contains("Associated validation rework artifact refs"));
        assert!(!state_text.contains("already associated with this node"));
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
        assert!(response_item_text_contains(&item, "progress fact"));
        assert!(response_item_text_contains(
            &item,
            "not a closed action space"
        ));
        assert!(response_item_text_contains(
            &item,
            "list_files, search, read_file"
        ));
        assert!(response_item_text_contains(&item, "#!/bin/nonexistent"));
    }

    #[test]
    fn implementation_needs_edit_progress_fact_is_non_terminal() {
        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "validation_schema_repair_contract: target_artifacts=generate_organization.py",
        ));

        assert!(is_taskspace_plain_implement_needs_edit_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
        assert!(!taskspace_special_recovery_warning_message(&item).contains("HardStop"));
    }

    #[test]
    fn validation_rework_duplicate_read_rejection_uses_edit_recovery() {
        assert!(taskspace_message_hit_implementation_needs_edit(Some(
            "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `recover.py` in result `result-12` and no successful edit has been recorded after that read. The previous complete read result remains available as duplicate evidence; choose any state-machine-legal action using the visible facts, or record blocked with the exact blocker."
        )));

        let item = build_taskspace_implement_needs_edit_recovery_item(Some(
            "result-12: recover.py current binary scan recovered 2 rows",
        ));
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_IMPLEMENT_NEEDS_EDIT_MARKER));
        assert!(text.contains("recover.py"));
        assert!(text.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!text.contains("Action-space source of truth"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn validation_rework_duplicate_read_recovery_preserves_patch_only_contract() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-10` and no successful edit has been recorded after that read. The previous complete read result remains available as duplicate evidence; choose any state-machine-legal action using the visible facts, or record blocked with the exact blocker. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | schema_required_groups=schema.json:properties.statistics requires averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService | target_artifacts=process.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-10` or choose another state-machine-legal action\"]}";
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
        assert!(
            text.contains("feedback_semantics: exact duplicate complete read_file request only")
        );
        assert!(text.contains("Projection boundary:"));
        assert!(text.contains("does not select an implementation strategy"));
        assert!(text.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!text.contains("action_space_source"));
        assert!(text.contains("duplicate_complete_read_signal"));
        assert!(!text.contains("Current required behavior:"));
        assert!(!text.contains("read_file/context refresh is not a valid recovery"));
        let boundary_pos = text
            .find("Projection boundary:")
            .expect("projection boundary heading");
        let evidence_pos = text
            .find("Already inspected evidence available to use now:")
            .expect("evidence heading");
        assert!(
            boundary_pos < evidence_pos,
            "duplicate-read semantics must precede long evidence block:\n{text}"
        );
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn validation_rework_duplicate_read_recovery_preserves_failed_patch_grammar() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-10` or choose another state-machine-legal action\"]}";
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
        assert!(text.contains("Projection boundary:"));
        assert!(text.contains("does not select an implementation strategy"));
        assert!(!text.contains("correct that patch grammar now"));
        assert!(!text.contains("read_file/context refresh is not a valid recovery"));
        assert!(!text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(
            taskspace_implement_recovery_advisory_warning_message(&item, 7)
                .contains("TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1")
        );
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn validation_rework_duplicate_read_recovery_preserves_unanchored_patch_feedback() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-10` or choose another state-machine-legal action\"]}";
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
        assert!(text.contains("Projection boundary:"));
        assert!(!text.contains("correct that patch grammar now"));
        assert!(!text.contains("read_file/context refresh is not a valid recovery"));
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
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-10` or choose another state-machine-legal action\"]}";
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
    fn implementation_recovery_preserves_rework_evidence_after_target_read() {
        let last_message = "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit. Return exactly one valid taskspace-action-v1 JSON object.";
        let evidence = "validation_rework: smoke_test `node-3` failed result `result-10`: missing_required_properties: members, averageDepartmentBudget \
| validation_rework_target_read result=result-12 artifact=generate_org.py excerpt: member_ids -> members \
| result-2 artifacts=schema.json: required members and averageDepartmentBudget";
        let item =
            build_taskspace_implementation_recovery_item(Some(last_message), Some(evidence), None);
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(text.contains("failure_kind: validation_rework_evidence_after_target_read"));
        assert!(text.contains("boundary_mode: evidence_only"));
        assert!(text.contains("target_artifacts: generate_org.py"));
        assert!(text.contains("no additional file lines are hidden"));
        assert!(text.contains("Schema contract evidence snippets copied from current context:"));
        assert!(text.contains("schema_contract_evidence_visible=true"));
        assert!(text.contains("missing_required_properties: members, averageDepartmentBudget"));
        assert!(text.contains("Apply_patch grammar facts:"));
        assert!(!text.contains("Schema validation facts from current failure:"));
        assert!(!text.contains("Missing required output properties captured"));
        assert!(!text.contains("schema_repair_fact_summary=true"));
        assert!(text.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!text.contains("action_space_source"));
        assert!(!text.contains("Patch construction scaffold:"));
        assert!(!text.contains("Final action lock:"));
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
        assert!(!taskspace_special_recovery_warning_message(&item).contains("HardStop"));
    }

    #[test]
    fn implementation_recovery_preserves_schema_type_mismatch_facts() {
        let last_message = "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit. Return exactly one valid taskspace-action-v1 JSON object.";
        let evidence = "validation_rework: smoke_test `node-5` failed result `result-14`: schema_type_mismatches=skillDistribution expected object \
| validation_rework_target_read result=result-16 artifact=generate_organization.py read_context: complete_read eof_reached=true content_visibility: full_content_visible \
| validation_schema_repair_contract: schema_type_mismatches=skillDistribution expected object | target_artifacts=generate_organization.py";
        let item =
            build_taskspace_implementation_recovery_item(Some(last_message), Some(evidence), None);
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(text.contains("Schema contract evidence snippets copied from current context:"));
        assert!(text.contains("schema_contract_evidence_visible=true"));
        assert!(text.contains("schema_type_mismatches=skillDistribution expected object"));
        assert!(!text.contains("Schema validation facts from current failure:"));
        assert!(text.contains("target_artifacts: generate_organization.py"));
        assert!(is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
    }

    #[test]
    fn implementation_recovery_preserves_array_item_type_facts() {
        let last_message = "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit. Return exactly one valid taskspace-action-v1 JSON object.";
        let evidence = "validation_rework: smoke_test `node-5` failed result `result-14`: schema_type_mismatches=members expected string items \
| validation_rework_target_read result=result-16 artifact=generate_organization.py read_context: complete_read eof_reached=true content_visibility: full_content_visible \
| validation_schema_repair_contract: schema_type_mismatches=members expected string items | target_artifacts=generate_organization.py";
        let item =
            build_taskspace_implementation_recovery_item(Some(last_message), Some(evidence), None);
        let text = item_text(item);

        assert!(text.contains("schema_type_mismatches=members expected string items"));
        assert!(text.contains("schema_contract_evidence_visible=true"));
        assert!(!text.contains("schema_repair_fact_summary=true"));
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
    fn implementation_recovery_preserves_evidence_after_visible_target_read() {
        let last_message = "TaskSpaceActionContractStateV1: implementation_needs_edit after visible validation_rework_target_read.";
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
        assert!(text.contains("schema_contract_evidence_visible=true"));
        assert!(text.contains("boundary_mode: evidence_only"));
        assert!(text.contains("no additional file lines are hidden"));
        assert!(text.contains("Schema contract evidence snippets copied from current context:"));
        assert!(text.contains("missing_required_properties=members, averageDepartmentBudget"));
        assert!(text.contains("averageYearsOfService"));
        assert!(text.contains("schema_property_rename_hints=member_ids->members"));
        assert!(text.contains("Complete target-read visibility facts"));
        assert!(text.contains("*** Delete File: generate_organization.py"));
        assert!(text.contains("*** Add File: generate_organization.py"));
        assert!(
            text.find("Complete target-read visibility facts")
                < text.find("Previous blocked feedback")
        );
        assert!(text.contains("Patch payload grammar contains patch sections"));
        assert!(!text.contains("Do not put markdown fences"));
        assert!(!text.contains("schema_repair_fact_summary=true"));
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
        assert!(text.contains("tool_feedback_facts: the failed hunk did not match"));
        assert!(text.contains("tool_feedback_locator: failed_hunk_target_snapshot_mismatch"));
        assert!(text.contains("Complete target-read visibility facts"));
        assert!(text.contains("*** Delete File"));
        assert!(text.contains("*** Add File"));
        assert!(text.contains("Feedback boundary:"));
        assert!(text.contains("The failed edit result remains part"));
        assert!(!text.contains("correction_options:"));
        assert!(!text.contains("Available correction paths include"));
        assert!(!text.contains("Do not emit `*** Update File`"));
        assert!(!text.contains("Do not call read_file"));
        assert!(!text.contains("Emit exactly one recovery action now"));
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
    fn validation_rework_patch_only_schema_repair_remains_advisory_without_hard_stop() {
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
        let boundary_pos = recovery_text
            .find("Feedback boundary:")
            .or_else(|| recovery_text.find("boundary_mode: evidence_only"))
            .expect("feedback boundary heading");
        let evidence_pos = recovery_text
            .find("Already inspected evidence available to use now:")
            .expect("evidence heading");
        assert!(
            boundary_pos < evidence_pos,
            "boundary facts must precede long evidence block:\n{recovery_text}"
        );
        assert!(!recovery_text.contains("Current required behavior:"));
        assert!(!recovery_text.contains("Do not call read_file"));
        assert!(!recovery_text.contains("HardStop"));
        assert!(is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
    }

    #[test]
    fn validation_rework_patch_only_without_schema_repair_stays_advisory() {
        let item = build_taskspace_validation_rework_patch_only_recovery_item(
            Some(
                "TaskSpaceActionV1 rejected: node_policy_violation:implement_solution:read_file:implementation_needs_edit",
            ),
            Some("validation_rework_target_read result=result-12 artifact=generate_org.py"),
            None,
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(!text.contains("schema_repair_rediscovery_grace=true"));
        assert!(!text.contains("HardStop"));
        assert!(is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
    }

    #[test]
    fn validation_rework_patch_only_omits_legacy_strategy_feedback_without_hard_stop() {
        let next_valid_action = ["next_valid", "_action:"].concat();
        let next_valid_action_prose = ["Next valid", " action:"].concat();
        let previous = format!(
            "TaskSpaceToolFeedbackV1:\n\
tool_source: action_contract_internal\n\
tool_action: block_node\n\
tool_result: blocked\n\
failure_kind: missing_source_visibility_blocker_rejected\n\
{next_valid_action} emit exactly one apply_patch action. Do not block for missing source visibility.\n\
raw_output:\n\
TaskSpace implement_solution node `node-6` cannot be blocked for missing source visibility because dependency evidence already identifies the implementation artifact or validation rework target. {next_valid_action_prose} retry apply_patch using existing complete validation rework target evidence plus failed validation/tool feedback; do not block for source visibility and do not refresh read when complete_read/eof_reached=true."
        );
        let item = build_taskspace_validation_rework_patch_only_recovery_item(
            Some(&previous),
            Some(
                "validation_rework_target_read result=result-16 artifact=generate_organization.py read_context: complete_read eof_reached=true content_visibility: full_content_visible | validation_rework: smoke_test `node-5` has blocked validation evidence `result-13`: KeyError: 'id'",
            ),
            None,
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_PATCH_ONLY_MARKER));
        assert!(text.contains("Previous runtime-boundary strategy text omitted"));
        assert!(!text.contains("missing_source_visibility_blocker_rejected"));
        assert!(!text.contains(
            "retry apply_patch using existing complete validation rework target evidence"
        ));
        assert!(!text.contains("HardStop"));
        assert!(is_taskspace_validation_rework_patch_only_recovery_item(
            &item
        ));
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
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-10` and no successful edit has been recorded after that read. TaskSpaceGateRecoveryV1: {\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-10` or choose another state-machine-legal action\"]}";
        let item = build_taskspace_implementation_recovery_item(
            Some(last_message),
            Some("result-10 artifacts=generate_org.py"),
            Some("TaskSpaceActionV1 rejected: apply_patch_unanchored_update:generate_org.py"),
        );
        let text = item_text(item);

        assert!(text.contains(TASKSPACE_VALIDATION_REWORK_DUPLICATE_READ_MARKER));
        assert!(text.contains("apply_patch_unanchored_update:generate_org.py"));
        assert!(text.contains("Most recent failed edit feedback to preserve"));
        assert!(!text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
    }

    #[test]
    fn implementation_recovery_selects_duplicate_rework_from_gate_text_without_reason() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate.py` in result `result-11` and no successful edit has been recorded after that read. The previous complete read result remains available as duplicate evidence; choose any state-machine-legal action using the visible facts, or record blocked with the exact blocker. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | target_artifacts=generate.py";
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
        assert!(text.contains("KeyError"));
        assert!(text.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!text.contains("Action-space source of truth"));
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
    fn taskspace_patch_intent_format_recovery_preserves_strict_json_boundary() {
        let item = build_taskspace_patch_intent_format_recovery_item(
            Some("result-9: task_deps/generator.log traceback"),
            Some("{\"schema_version\":\"taskspace-action-v1\",\"action\":\"apply_patch\"} extra"),
        );
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_PATCH_INTENT_FORMAT_MARKER));
        assert!(text.contains("not exactly one taskspace-action-v1 JSON object"));
        assert!(text.contains("requires exactly one valid taskspace-action-v1 JSON object"));
        assert!(text.contains("choose another state-machine-legal action"));
        assert!(text.contains("must not be wrapped in markdown fences"));
        assert!(!text.contains("Do not call read_file, list_files, search"));
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
        assert!(summary.contains("tool_feedback_facts: apply_patch could not find"));
        assert!(summary.contains("tool_feedback_locator: target_path=generate_org.py"));
        assert!(summary.contains("content_visibility_source: current read summaries"));
        assert!(!summary.contains("correction_options:"));
        assert!(!summary.contains("refresh `generate_org.py`"));

        let recovery = build_taskspace_edit_failure_recovery_item(
            Some(
                "result-11: apply_patch verification failed: Failed to find expected lines in generate_org.py",
            ),
            Some("result-10 artifacts=generate_org.py excerpt truncated"),
        );
        let text = item_text(recovery);
        assert!(text.contains(TASKSPACE_EDIT_FAILURE_MARKER));
        assert!(text.contains("patch_format_facts: native apply_patch accepts"));
        assert!(text.contains("Feedback boundary:"));
        assert!(text.contains("The failed edit result remains part"));
        assert!(!text.contains("Available correction paths include"));
        assert!(!text.contains("a narrow read_file when existing context is stale or truncated"));
    }

    #[test]
    fn complete_validation_rework_expected_lines_failure_exposes_replacement_facts() {
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
        assert!(text.contains("Complete target-read visibility facts"));
        assert!(text.contains("content_visibility=full_content_visible"));
        assert!(text.contains("*** Delete File: <path>"));
        assert!(text.contains("*** Add File: <path>"));
        assert!(text.contains("tool_feedback_locator: failed_hunk_target_snapshot_mismatch"));
        assert!(!text.contains("Available correction paths include"));
        assert!(!text.contains("do not refresh read"));
        assert!(!text.contains("Do not emit `*** Update File`"));
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
        assert!(!text.contains("Complete target-read visibility facts"));
        assert!(text.contains("patch_format_facts: native apply_patch accepts"));
        assert!(!text.contains("a narrow read_file when existing context is stale or truncated"));
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
        assert!(text.contains("Missing files are created with native"));
        assert!(text.contains("does not use `--- /dev/null`"));
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
    fn action_contract_failed_validation_finish_is_not_reinterpreted() {
        let snapshot = provider_snapshot("smoke_test");
        let args = serde_json::json!({
            "action": "finish_node",
            "status": "failed",
            "reason": "Test failed with IndentationError on process.py line 1."
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("mechanical control shape should parse");

        assert_eq!(normalized, args);
        assert!(normalized.get("node_id").is_none());
        assert!(normalized.get("blocker_summary").is_none());
    }

    #[test]
    fn action_contract_feedback_preserves_stale_validation_block_verbatim() {
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
        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item])
            .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("cannot be blocked as failed validation"));
        assert!(text.contains("cannot be blocked as failed validation"));
        assert!(!summary.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(!text.contains("progress_fact:"));
    }

    #[test]
    fn action_contract_feedback_preserves_validation_finish_rejection_verbatim() {
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
        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item])
            .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("cannot be completed without a recorded successful test"));
        assert!(text.contains("cannot be completed without a recorded successful test"));
        assert!(!summary.contains(TASKSPACE_TOOL_FEEDBACK_MARKER));
        assert!(!text.contains("progress_fact:"));
    }

    #[test]
    fn action_contract_feedback_preserves_duplicate_rework_read_verbatim() {
        let tool_call = ToolCall {
            tool_name: ToolName::plain("shell_command"),
            call_id: "taskspace-action-contract-19-read_file".to_string(),
            payload: ToolPayload::Function {
                arguments: "{\"command\":\"sed -n '1,240p' -- process.py\"}".to_string(),
            },
        };
        let err = CodexErr::Fatal(
            "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `process.py` in result `result-10` and no successful edit has been recorded after that read. The previous complete read result remains available as duplicate evidence; choose any state-machine-legal action using the visible facts, or record blocked with the exact blocker. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | schema_required_groups=schema.json:properties.statistics requires averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService | target_artifacts=process.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-10` or choose another state-machine-legal action\"]}".to_string(),
        );
        let response_input = response_input_for_taskspace_action_tool_error(&tool_call, &err);
        let response_item: ResponseItem = response_input.into();
        let (_, summary) = taskspace_action_contract_tool_output_summary(&response_item)
            .expect("failed action-contract output summarizes");
        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item])
            .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains("already read failure artifact `process.py`"));
        assert!(summary.contains("in result `result-10`"));
        assert!(
            summary.contains("Validation repair contract: missing_required_properties=members")
        );
        assert!(summary.contains("projectStatusDistribution"));
        assert!(text.contains("already read failure artifact `process.py`"));
        assert!(!summary.contains("feedback_semantics:"));
        assert!(!text.contains("progress_fact:"));
    }

    #[test]
    fn action_contract_feedback_does_not_reinterpret_implementation_rejection() {
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
        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item])
            .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(summary.contains(
            "node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        ));
        assert!(
            summary.contains("Validation repair contract: missing_required_properties=members")
        );
        assert!(summary.contains("projectStatusDistribution"));
        assert!(text.contains(
            "node_policy_violation:implement_solution:read_file:implementation_needs_edit"
        ));
        assert!(!summary.contains("feedback_semantics:"));
        assert!(!text.contains("progress_fact:"));
    }

    #[test]
    fn taskspace_action_contract_run_test_preserves_bare_pytest_command() {
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
                assert_eq!(value["command"], "pytest test_tax_calc.py -v");
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
                assert_eq!(value["command"], "python -m pytest test_tax_calc.py -v");
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_run_test_preserves_shell_command() {
        let cases = [
            ("./run_pipeline.sh", "./run_pipeline.sh"),
            (
                "cd /app && ./run_pipeline.sh",
                "cd /app && ./run_pipeline.sh",
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
    fn taskspace_action_contract_run_test_preserves_and_chain() {
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
                assert_eq!(
                    value["command"],
                    "python merge_users.py && python -c 'print(\"ok\")'"
                );
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_run_test_preserves_or_chain() {
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
                assert_eq!(value["command"], command);
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
    fn taskspace_action_contract_apply_patch_drops_unified_function_context_header() {
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"apply_patch","node_id":"node-1","args":{"patch":"*** Begin Patch\n--- a/src/call_stack_counter.py\n+++ b/src/call_stack_counter.py\n@@ -4,7 +4,7 @@ def count_stack_depth() -> int:\n     return len(inspect.stack())\n \n def format_depth() -> str:\n-    return f\"depth: {count_stack_depth()}\"\n+    return f\"CALL_STACK_DEPTH={count_stack_depth()}\"\n*** End Patch\n"}}"#,
        )
        .expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("policy ok")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/call_stack_counter.py"));
                assert!(input.contains("\n@@\n"));
                assert!(input.contains(" def format_depth() -> str:\n"));
                assert!(input.contains("-    return f\"depth: {count_stack_depth()}\""));
                assert!(input.contains("+    return f\"CALL_STACK_DEPTH={count_stack_depth()}\""));
                assert!(!input.contains("     return len(inspect.stack())"));
                assert!(!input.contains(" def main() -> None:"));
                assert!(!input.contains("@@ def count_stack_depth() -> int:"));
                assert!(!input.contains("@@ -4,7 +4,7 @@"));
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
    fn taskspace_action_contract_normalizes_unified_diff_with_trailing_end_only() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: src/call_stack_counter.py\n--- a/src/call_stack_counter.py\n+++ b/src/call_stack_counter.py\n@@ -5,7 +5,7 @@ import inspect\n def count_stack_depth() -> int:\n     return len(inspect.stack())\n \n-def format_depth() -> str:\n-    return f\"depth: {count_stack_depth()}\"\n+def format_depth() -> str:\n+    return f\"CALL_STACK_DEPTH={count_stack_depth()}\"\n \n def main() -> None:\n     print(format_depth())\n*** End Patch\n"
            },
            "rationale": "fix format"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("trailing End Patch without Begin Patch should normalize")
            .expect("tool call");

        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert_eq!(input.matches("*** End Patch").count(), 1);
                assert_eq!(
                    input
                        .matches("*** Update File: src/call_stack_counter.py")
                        .count(),
                    1
                );
                assert!(!input.contains("--- a/src/call_stack_counter.py"));
                assert!(!input.contains("+++ b/src/call_stack_counter.py"));
                assert!(!input.contains("@@ -5,7 +5,7 @@"));
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
    fn taskspace_action_contract_allows_native_update_for_rework_target() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: process.py\n@@\n-print('bad')\n+print('fixed')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("state-machine-legal native update should dispatch for rework target")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
    }

    #[test]
    fn taskspace_action_contract_normalizes_mixed_native_unified_patch() {
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

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("mechanically convertible native/unified patch should dispatch")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/recover.py"));
                assert!(input.contains("@@\n-print('bad')\n+print('fixed')"));
                assert!(!input.contains("--- a/recover.py"));
                assert!(!input.contains("+++ b/recover.py"));
                assert!(!input.contains("@@ -1,1 +1,1 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_allows_mechanically_actionable_rework_target_mixed_patch() {
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

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("mechanically actionable rework mixed patch should dispatch")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/process_csv.py"));
                assert!(input.contains("@@\n-print('bad')\n+print('fixed')"));
                assert!(!input.contains("--- a/process_csv.py"));
                assert!(!input.contains("+++ b/process_csv.py"));
                assert!(!input.contains("@@ -1,1 +1,1 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_keeps_unanchored_update_feedback_for_rework_target() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: process.py\n+print('replacement is still required')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");

        let err = taskspace_action_to_tool_call(&action, &snapshot)
            .expect_err("unanchored rework target Update File must keep grammar feedback");

        assert_eq!(err, "apply_patch_unanchored_update:process.py");
    }

    #[test]
    fn taskspace_action_contract_keeps_placeholder_range_feedback_for_rework_target() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: process.py\n@@ -... +... @@\n-old\n+new\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");

        let err = taskspace_action_to_tool_call(&action, &snapshot)
            .expect_err("placeholder range hunks are not mechanically actionable");

        assert_eq!(err, "apply_patch_mixed_native_unified:process.py");
    }

    #[test]
    fn taskspace_action_contract_keeps_placeholder_ellipsis_feedback_for_rework_target() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: process.py\n@@ ... @@\n-old\n+new\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");
        let mut snapshot = provider_snapshot("implement_solution");

        let err = taskspace_action_to_tool_call(&action, &snapshot)
            .expect_err("placeholder ellipsis hunks are not mechanically actionable");

        assert_eq!(err, "apply_patch_mixed_native_unified:process.py");
    }

    #[test]
    fn taskspace_action_contract_keeps_generic_unanchored_update_for_non_rework_target() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Begin Patch\n*** Update File: recover.py\n+print('fixed')\n*** End Patch\n"
            },
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("non-rework target should keep generic unanchored update feedback");

        assert_eq!(err, "apply_patch_unanchored_update:recover.py");
    }

    #[test]
    fn taskspace_action_contract_normalizes_live_wrapped_mixed_native_unified_patch() {
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

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("live wrapped native/unified patch should normalize")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/csv2json.py"));
                assert!(input.contains("@@\n #!/usr/bin/env python3\n- \"\"\"CSV"));
                assert!(!input.contains("--- a/csv2json.py"));
                assert!(!input.contains("+++ b/csv2json.py"));
                assert!(!input.contains("@@ -1,2 +1,2 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_live_unwrapped_mixed_native_unified_patch() {
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

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("live unwrapped native/unified patch should normalize")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/csv2json.py"));
                assert!(input.contains("@@\n- #!/usr/bin/env python3\n+#!/usr/bin/env python3"));
                assert!(!input.contains("--- a/csv2json.py"));
                assert!(!input.contains("+++ b/csv2json.py"));
                assert!(!input.contains("@@ -1,2 +1,2 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_normalizes_sample_unwrapped_mixed_native_unified_patch() {
        let raw = serde_json::json!({
            "schema_version": "taskspace-action-v1",
            "action": "apply_patch",
            "node_id": "node-1",
            "args": {
                "patch": "*** Update File: merge_users.py\n--- a/merge_users.py\n+++ b/merge_users.py\n@@ -7,3 +7,3 @@\n-DATA_DIR = Path('/data')\n+DATA_DIR = Path('data')\n OUT_DIR = Path('.')"
            },
            "rationale": "fix data dir"
        })
        .to_string();
        let action = parse_taskspace_action_v1(&raw).expect("valid json");

        let call = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect("sample native/unified patch should normalize")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.starts_with("*** Begin Patch\n"));
                assert!(input.ends_with("*** End Patch\n"));
                assert!(input.contains("*** Update File: src/merge_users.py"));
                assert!(input.contains(
                    "@@\n-DATA_DIR = Path('/data')\n+DATA_DIR = Path('data')\n OUT_DIR = Path('.')"
                ));
                assert!(!input.contains("--- a/merge_users.py"));
                assert!(!input.contains("+++ b/merge_users.py"));
                assert!(!input.contains("@@ -7,3 +7,3 @@"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
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
            .expect("separator update sections should normalize to native hunks")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/process.py"));
                assert!(input.contains(
                    "@@\n-    'member_ids': [m.strip() for m in p['member_ids'].split(';')],\n+    'members': [m.strip() for m in p['member_ids'].split(';')],"
                ));
                assert!(input.contains(
                    "@@\n-    'total_employees': total_employees,\n+    'totalEmployees': total_employees,"
                ));
                assert!(!input.contains("\n---\n"));
            }
            other => panic!("expected custom payload, got {other:?}"),
        }
    }

    #[test]
    fn taskspace_action_contract_rejects_duplicate_unwrapped_update_wrapper() {
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

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("duplicate mixed wrapper must be rejected before tool dispatch");

        assert_eq!(
            err,
            "apply_patch_mixed_native_unified:(missing patch target)"
        );
    }

    #[test]
    fn taskspace_action_contract_rejects_misordered_begin_update_mixed_patch() {
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

        let err = taskspace_action_to_tool_call(&action, &provider_snapshot("implement_solution"))
            .expect_err("misordered mixed wrapper must be rejected before tool dispatch");

        assert_eq!(err, "apply_patch_mixed_native_unified:csv2json.py");
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
            .expect("live mixed placeholder hunk should normalize when context is concrete")
            .expect("tool call");

        assert_eq!(call.tool_name.name, "apply_patch");
        match call.payload {
            ToolPayload::Custom { input } => {
                assert!(input.contains("*** Update File: src/generate_json.py"));
                assert!(input.contains(
                    "@@\n def build_organization(departments, employees, projects):\n+    for emp in employees:"
                ));
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
        assert!(text.contains("Feedback boundary: this item preserves patch-format facts"));
        assert!(!text.contains("Available correction paths"));
        assert!(text.contains("`--- Update File:`"));
        assert!(text.contains("*** Delete File: <relative/path>"));
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
        assert!(text.contains("Tool feedback facts for native update"));
        assert!(text.contains("`--- Update File:`"));
        assert!(text.contains("Native Update File scaffold"));
        assert!(text.contains("*** Update File: <relative/path>"));
        assert!(text.contains("-old exact line"));
        assert!(text.contains("+new exact line"));
        assert!(warning.contains("TaskSpaceApplyPatchNativeHunkRecoveryV1"));
        assert!(!warning.contains("TaskSpaceImplementNeedsEditRecoveryV1"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn path_correction_detects_data_and_app_absolute_paths() {
        let data_correction = taskspace_path_correction_from_text(
            "sed: can't read /data/source_b/users.csv: No such file or directory",
        )
        .expect("data path correction");
        assert_eq!(data_correction.failed_path, "/data/source_b/users.csv");
        assert_eq!(
            data_correction.suggested_relative_path,
            "data/source_b/users.csv"
        );

        let app_correction = taskspace_path_correction_from_text(
            "bash: /app/run_pipeline.sh: No such file or directory",
        )
        .expect("app path correction");
        assert_eq!(app_correction.failed_path, "/app/run_pipeline.sh");
        assert_eq!(app_correction.suggested_relative_path, "run_pipeline.sh");
    }

    #[test]
    fn path_correction_detects_action_map_failed_read_summary() {
        let summary = "result-1: Main tool call tool: shell_command call_id: taskspace-action-contract-1-read_file action_class: read success: false preview: TaskSpaceToolInvocationV1: tool: shell_command command: sed -n '1,240p' -- /data/source_a/users.json raw_output: sed: can't read /data/source_a/users.json: No such file or directory";
        let correction =
            taskspace_path_correction_from_text(summary).expect("summary path correction");

        assert_eq!(correction.failed_path, "/data/source_a/users.json");
        assert_eq!(
            correction.suggested_relative_path,
            "data/source_a/users.json"
        );
    }

    #[test]
    fn path_correction_detects_absolute_workspace_root_directory_failure() {
        let summary = "result-1: Main tool call tool: shell_command call_id: taskspace-action-contract-1-list_files action_class: read success: false preview: TaskSpaceToolInvocationV1: tool: shell_command command: rg --files /data raw_output: rg: /data: IO error for operation on /data: No such file or directory (os error 2)";
        let correction =
            taskspace_path_correction_from_text(summary).expect("root path correction");

        assert_eq!(correction.failed_path, "/data");
        assert_eq!(correction.suggested_relative_path, "data");
    }

    #[test]
    fn path_correction_does_not_match_non_workspace_prefix_word() {
        assert!(
            taskspace_path_correction_from_text(
                "rg: /database: IO error for operation on /database: No such file or directory"
            )
            .is_none()
        );
    }

    #[test]
    fn path_correction_recovery_item_is_advisory_feedback() {
        let output = tool_output(
            "rg: /data/source_a: IO error for operation on /data/source_a: No such file or directory",
        );
        let correction =
            taskspace_path_correction_from_response_item(&output).expect("path correction");
        let item = build_taskspace_path_correction_recovery_item(
            &correction,
            Some("inspect_code_context"),
        );
        let text = item_text(item);

        assert!(text.contains(TASKSPACE_PATH_CORRECTION_MARKER));
        assert!(text.contains("failure_kind: path_not_found_with_relative_candidate"));
        assert!(text.contains("failed_path: /data/source_a"));
        assert!(text.contains("suggested_relative_path: data/source_a"));
        assert!(text.contains("Path correction facts, not a runtime-selected strategy"));
        assert!(text.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!text.contains("action_space_source"));
        assert!(text.contains("Workspace-relative candidate: `data/source_a`"));
        assert!(!text.contains("Suggested recovery"));
        assert!(
            text.contains("inspect_code_context candidate path `data/source_a` can be evaluated")
        );
        assert!(text.contains("will not block other state-machine-legal tool actions"));
        assert!(!text.contains(TASKSPACE_GATE_RECOVERY_MARKER));
        assert!(!text.contains("\"reason\":\"path_correction_retry_forbidden\""));
    }

    #[test]
    fn path_correction_retry_is_advisory_not_action_contract_rejection() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data".to_string(),
            suggested_relative_path: "data".to_string(),
        };
        let broad_root = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"list_files","args":{"path":"."}}"#,
        )
        .expect("valid action");
        let absolute_retry = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"list_files","args":{"path":"/data"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_path_correction_retry_reject_reason(&broad_root, &correction).is_none());
        assert!(
            taskspace_path_correction_retry_reject_reason(&absolute_retry, &correction).is_none()
        );
        assert_eq!(
            taskspace_path_correction_retry_advisory_reason(&broad_root, &correction).as_deref(),
            Some("path_correction_advisory:.:suggested_relative_path=data")
        );
        assert_eq!(
            taskspace_path_correction_retry_advisory_reason(&absolute_retry, &correction)
                .as_deref(),
            Some("path_correction_advisory:/data:suggested_relative_path=data")
        );
    }

    #[test]
    fn path_correction_recovery_does_not_hard_stop() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data/source_a/users.json".to_string(),
            suggested_relative_path: "data/source_a/users.json".to_string(),
        };
        let item = build_taskspace_path_correction_recovery_item(
            &correction,
            Some("inspect_code_context"),
        );
        assert!(is_taskspace_path_correction_recovery_item(&item));
        let text = item_text(item);
        assert!(!text.contains("HardStop"));
        assert!(text.contains("will not block other state-machine-legal tool actions"));
    }

    #[test]
    fn path_correction_feedback_clears_after_successful_read_surface_action() {
        for action_name in ["list_files", "read_file", "search"] {
            let action = TaskSpaceActionV1 {
                schema_version: "taskspace-action-v1".to_string(),
                action: action_name.to_string(),
                node_id: Some("node-1".to_string()),
                args: serde_json::json!({ "path": "data/source_a/users.json" }),
                rationale: None,
            };
            assert!(taskspace_action_can_clear_path_correction_feedback(&action));
        }

        let edit_action = TaskSpaceActionV1 {
            schema_version: "taskspace-action-v1".to_string(),
            action: "apply_patch".to_string(),
            node_id: Some("node-1".to_string()),
            args: serde_json::json!({ "patch": "*** Begin Patch\n*** End Patch\n" }),
            rationale: None,
        };
        assert!(!taskspace_action_can_clear_path_correction_feedback(
            &edit_action
        ));
    }

    #[test]
    fn path_correction_failed_read_bridge_skips_after_new_progress() {
        assert!(
            taskspace_should_refill_path_correction_from_failed_read_summary(
                false,
                false,
                Some(3),
                Some(3),
            )
        );
        assert!(
            !taskspace_should_refill_path_correction_from_failed_read_summary(
                false,
                false,
                Some(3),
                Some(4),
            )
        );
        assert!(
            !taskspace_should_refill_path_correction_from_failed_read_summary(
                true,
                false,
                Some(3),
                Some(3),
            )
        );
        assert!(
            !taskspace_should_refill_path_correction_from_failed_read_summary(
                false,
                true,
                Some(3),
                Some(3),
            )
        );
    }

    #[test]
    fn path_correction_allows_repeated_absolute_action_path() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data".to_string(),
            suggested_relative_path: "data".to_string(),
        };
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"list_files","args":{"path":"/data/"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_path_correction_retry_reject_reason(&action, &correction).is_none());
        assert_eq!(
            taskspace_path_correction_retry_advisory_reason(&action, &correction).as_deref(),
            Some("path_correction_advisory:/data:suggested_relative_path=data")
        );
    }

    #[test]
    fn path_correction_allows_absolute_child_after_alias_root_failure() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data".to_string(),
            suggested_relative_path: "data".to_string(),
        };
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","args":{"path":"/data/source_a/users.json"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_path_correction_retry_reject_reason(&action, &correction).is_none());
        assert_eq!(
            taskspace_path_correction_retry_advisory_reason(&action, &correction).as_deref(),
            Some(
                "path_correction_advisory:/data/source_a/users.json:suggested_relative_path=data/source_a/users.json"
            )
        );
    }

    #[test]
    fn path_correction_allows_broad_root_after_relative_candidate() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data".to_string(),
            suggested_relative_path: "data".to_string(),
        };
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"list_files","args":{"path":"."}}"#,
        )
        .expect("valid action");

        assert!(taskspace_path_correction_retry_reject_reason(&action, &correction).is_none());
        assert_eq!(
            taskspace_path_correction_retry_advisory_reason(&action, &correction).as_deref(),
            Some("path_correction_advisory:.:suggested_relative_path=data")
        );
    }

    #[test]
    fn path_correction_allows_suggested_relative_action_path() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data/source_a/users.json".to_string(),
            suggested_relative_path: "data/source_a/users.json".to_string(),
        };
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","args":{"path":"data/source_a/users.json"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_path_correction_retry_reject_reason(&action, &correction).is_none());
    }

    #[test]
    fn path_correction_allows_child_of_suggested_relative_path() {
        let correction = TaskspacePathCorrection {
            failed_path: "/data".to_string(),
            suggested_relative_path: "data".to_string(),
        };
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","args":{"path":"data/source_a/users.json"}}"#,
        )
        .expect("valid action");

        assert!(taskspace_path_correction_retry_reject_reason(&action, &correction).is_none());
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
        assert!(text.contains("Whole-file native replacement"));
        assert!(text.contains("*** Delete File: <relative/path>"));
        assert!(text.contains("*** Add File: <relative/path>"));
        assert!(text.contains("Tool feedback facts for complete replacement"));
        assert!(!text.contains("Use native `*** Update File"));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn apply_patch_replacement_required_recovery_preserves_replacement_semantics() {
        let targets = taskspace_replacement_required_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_replacement_required:generate_organization.py. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("target parsed");
        let item = build_taskspace_apply_patch_replacement_required_recovery_item(&targets, None);
        let text = item_text(item.clone());
        let warning = taskspace_implement_recovery_advisory_warning_message(&item, 4);
        let special_warning = taskspace_special_recovery_warning_message(&item);

        assert_eq!(targets, "generate_organization.py");
        assert!(text.contains(TASKSPACE_APPLY_PATCH_REPLACEMENT_REQUIRED_MARKER));
        assert!(text.contains("Whole-file native replacement grammar"));
        assert!(text.contains("*** Delete File: <relative/path>"));
        assert!(text.contains("*** Add File: <relative/path>"));
        assert!(text.contains("Native `*** Update File` grammar includes exact existing context"));
        assert!(text.contains("Native apply_patch payloads do not contain `*** Context Lines`"));
        assert!(!text.contains("A syntactically valid native `*** Update File` is still accepted"));
        assert!(!text.contains(TASKSPACE_APPLY_PATCH_NATIVE_HUNK_MARKER));
        assert!(warning.contains("TaskSpaceApplyPatchReplacementRequiredRecoveryV1"));
        assert!(!warning.contains("TaskSpaceApplyPatchNativeHunkRecoveryV1"));
        assert!(special_warning.contains("TaskSpaceApplyPatchReplacementRequiredRecoveryV1"));
        assert!(is_taskspace_apply_patch_recovery_item(&item));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
        assert!(!text.contains("HardStop"));
    }

    #[test]
    fn apply_patch_replacement_required_recovery_uses_full_visible_target_scaffold() {
        let targets = taskspace_replacement_required_targets_from_rejection(Some(
            "TaskSpaceActionV1 rejected: apply_patch_replacement_required:csv_to_json.py. Return exactly one valid taskspace-action-v1 JSON object.",
        ))
        .expect("target parsed");
        let item = build_taskspace_apply_patch_replacement_required_recovery_item(
            &targets,
            Some(
                "validation_rework_target_read result=result-19 artifact=csv_to_json.py read_context: complete_read eof_reached=true content_visibility: full_content_visible",
            ),
        );
        let text = item_text(item);

        assert!(text.contains("Complete target-read visibility facts"));
        assert!(text.contains("*** Delete File: csv_to_json.py"));
        assert!(text.contains("*** Add File: csv_to_json.py"));
        assert!(text.contains("content_visibility=full_content_visible"));
        assert!(text.contains("replacement grammar"));
        assert!(!text.contains("Reconstruct the complete corrected file"));
    }

    #[test]
    fn apply_patch_recovery_remains_advisory_after_repeated_same_node_failures() {
        let item = build_taskspace_apply_patch_unanchored_update_recovery_item("recover.py");
        let text = item_text(item.clone());

        assert!(is_taskspace_apply_patch_recovery_item(&item));
        assert!(text.contains("TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1"));
        assert!(!text.contains("HardStop"));
        assert!(is_taskspace_implement_needs_edit_recovery_item(&item));
    }

    #[test]
    fn apply_patch_native_hunk_recovery_remains_advisory() {
        let item = build_taskspace_apply_patch_native_hunk_recovery_item("recover.py", false);
        let text = item_text(item.clone());

        assert!(is_taskspace_apply_patch_recovery_item(&item));
        assert!(text.contains("TaskSpaceApplyPatchNativeHunkRecoveryV1"));
        assert!(!text.contains("HardStop"));
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
        assert!(patch.contains("@@\n"));
        assert!(!patch.contains("@@ def calculate_tax(subtotal, region):"));
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
    fn taskspace_provider_transport_defaults_deepseek_to_native_tools() {
        assert_eq!(
            taskspace_provider_transport_mode_for_request(true, ""),
            TaskspaceProviderTransportMode::NativeTools
        );
        assert_eq!(
            taskspace_provider_transport_mode_for_request(true, "native_tools"),
            TaskspaceProviderTransportMode::NativeTools
        );
        assert_eq!(
            taskspace_provider_transport_mode_for_request(true, "action_contract"),
            TaskspaceProviderTransportMode::CacheOptimizedActionContract
        );
        assert_eq!(
            taskspace_provider_transport_mode_for_request(false, ""),
            TaskspaceProviderTransportMode::NativeTools
        );
    }

    #[test]
    fn taskspace_static_contract_exposes_duplicate_validation_rework_baseline() {
        let instructions = taskspace_static_action_contract_instructions();

        assert!(instructions.contains("taskspace-action-sequence-v1"));
        assert!(instructions.contains("Runtime executes actions in listed order"));
        assert!(!instructions.contains("Return one taskspace-action-v1 JSON object"));
        assert!(instructions.contains("validation rework duplicate baseline"));
        assert!(instructions.contains("exact duplicate read_file"));
        assert!(instructions.contains("low-information evidence signal"));
        assert!(instructions.contains("read_file remains governed by the active node action set"));
        assert!(!instructions.contains("may be rejected as duplicate evidence"));
        assert!(!instructions.contains("apply_patch if enough evidence is available"));
        assert!(!instructions.contains("validation rework override"));
        assert!(
            !instructions.contains("read_file/list_files/search/schema inspection are not valid")
        );
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
        assert!(taskspace_action_allowed_for_node(
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
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_organization.py"}}"#,
        )
        .expect("valid action");

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("first target read should be allowed")
            .expect("read_file maps to shell command");

        assert_eq!(call.tool_name.name, "shell_command");
    }

    #[test]
    fn taskspace_action_contract_allows_duplicate_validation_rework_target_read() {
        let mut snapshot = provider_snapshot("implement_solution");
        let action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"read_file","node_id":"node-1","args":{"path":"generate_organization.py"}}"#,
        )
        .expect("valid action");

        let call = taskspace_action_to_tool_call(&action, &snapshot)
            .expect("duplicate target read remains state-machine legal")
            .expect("read_file maps to shell command");

        assert_eq!(call.tool_name.name, "shell_command");
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
    fn completed_task_final_answer_conversion_includes_blocked_action() {
        let blocked_action = parse_taskspace_action_v1(
            r#"{"schema_version":"taskspace-action-v1","action":"blocked","args":{"reason":"stale local validator infrastructure blocker"}}"#,
        )
        .expect("valid blocked action");
        let final_action = taskspace_final_answer_action("Validation passed.");

        assert!(taskspace_completed_task_action_should_force_final_answer(
            &blocked_action
        ));
        assert!(!taskspace_completed_task_action_should_force_final_answer(
            &final_action
        ));
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

        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item])
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
        assert!(joined.contains("tool_feedback_facts: apply_patch tried to update"));
        assert!(
            joined.contains("tool_feedback_locator: target_path=call_stack_counter/__main__.py")
        );
        assert!(joined.contains("patch_format_facts: native `*** Add File`"));
        assert!(!joined.contains("correction_options:"));
        assert!(!joined.contains("next_valid_action: emit exactly one apply_patch"));
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
        assert!(joined.contains("tool_feedback_facts: apply_patch could not find"));
        assert!(joined.contains("tool_feedback_locator: target_path=convert.py"));
        assert!(joined.contains("content_visibility_source: current read summaries"));
        assert!(!joined.contains("correction_options:"));
        assert!(!joined.contains("*** Delete File: convert.py"));
        assert!(!joined.contains("*** Add File: convert.py"));
        assert!(!joined.contains("Do not repeat the same context hunk"));
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
        assert!(joined.contains("tool_feedback_facts: apply_patch context did not match"));
        assert!(joined.contains("tool_feedback_locator: target_path=recover.py"));
        assert!(joined.contains("content_visibility_source: current read summaries"));
        assert!(!joined.contains("correction_options:"));
        assert!(!joined.contains("Do not repeat the same context hunk"));
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
        assert!(joined.contains("tool_feedback_facts: apply_patch rejected"));
        assert!(joined.contains("tool_feedback_locator: raw_error_preserved=true"));
        assert!(joined.contains("patch_format_facts: native apply_patch grammar rejects"));
        assert!(!joined.contains("correction_options:"));
        assert!(!joined.contains("prefix every added file content line with `+`"));
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
    fn action_contract_prompt_preserves_local_validator_coverage_failure_raw() {
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
        assert!(joined.contains("failure_kind: tool_execution_failed"));
        assert!(joined.contains("validation_test_missing_local_validator_coverage"));
        assert!(joined.contains("raw_output:"));
        assert!(!joined.contains("failure_kind: validation_test_missing_local_validator_coverage"));
        assert!(!joined.contains("required_validator: python scripts/validate.py"));
        assert!(!joined.contains("progress_fact: A previous run_test was rejected"));
    }

    #[test]
    fn action_contract_prompt_preserves_changed_artifact_coverage_failure_raw() {
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
        assert!(joined.contains("failure_kind: tool_execution_failed"));
        assert!(joined.contains("validation_test_missing_changed_artifact_coverage"));
        assert!(joined.contains("python generate_organization.py"));
        assert!(joined.contains("raw_output:"));
        assert!(
            !joined.contains("failure_kind: validation_test_missing_changed_artifact_coverage")
        );
        assert!(!joined.contains("required_command: python generate_organization.py"));
        assert!(!joined.contains("progress_fact: A previous run_test was rejected"));
    }

    #[test]
    fn action_contract_prompt_preserves_output_contract_coverage_failure_raw() {
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
        assert!(joined.contains("failure_kind: tool_execution_failed"));
        assert!(joined.contains("validation_test_missing_output_contract_coverage"));
        assert!(joined.contains(
            "python generate_json.py && python -m jsonschema -i organization.json schema.json"
        ));
        assert!(joined.contains("raw_output:"));
        assert!(!joined.contains("failure_kind: validation_test_missing_output_contract_coverage"));
        assert!(!joined.contains("required_command: python generate_json.py"));
        assert!(!joined.contains("progress_fact: A previous run_test was rejected"));
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
            joined.contains("progress_fact: The previous run_test did not start the validator")
        );
        assert!(
            joined.contains("tool_feedback_facts: the command referenced a missing script path")
        );
        assert!(joined.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!joined.contains("action_space_source"));
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
        assert!(joined.contains("progress_fact: A previous ordinary tool was blocked"));
        assert!(joined.contains("hard_state: result_validity_unreviewed_for_dependent_record"));
        assert!(joined.contains("tool_feedback_facts: result `result-8` is still unreviewed"));
        assert!(!joined.contains("action_space_source"));
        assert!(!joined.contains("taskspace_control action=state_commit is available"));
        assert!(!joined.contains("Next action must be taskspace_control"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_obsolete_completed_diagnostic_strategy() {
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
        assert!(joined.contains("failure_kind: obsolete_runtime_boundary_strategy_feedback"));
        assert!(joined.contains("raw_output_omitted: true"));
        assert!(!joined.contains("next_valid_action: emit exactly one apply_patch action"));
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
        assert!(joined.contains("progress_fact: A previous finish_node action was rejected"));
        assert!(joined.contains("hard_state: implementation_node_without_successful_edit_result"));
        assert!(joined.contains("tool_feedback_facts: finish_node for this implementation node"));
        assert!(!joined.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_obsolete_internal_policy_blocker_strategy() {
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
        assert!(joined.contains("failure_kind: obsolete_runtime_boundary_strategy_feedback"));
        assert!(joined.contains("raw_output_omitted: true"));
        assert!(!joined.contains("next_valid_action: emit exactly one apply_patch action"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_obsolete_missing_source_blocker_strategy() {
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
        assert!(joined.contains("failure_kind: obsolete_runtime_boundary_strategy_feedback"));
        assert!(joined.contains("raw_output_omitted: true"));
        assert!(!joined.contains("apply_patch is available"));
        assert!(!joined.contains("failed patch feedback"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_obsolete_validation_rework_missing_source_strategy() {
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
        assert!(joined.contains("failure_kind: obsolete_runtime_boundary_strategy_feedback"));
        assert!(joined.contains("raw_output_omitted: true"));
        assert!(!joined.contains("apply_patch is available"));
        assert!(!joined.contains("failed validation feedback"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_obsolete_validator_procedure_strategy() {
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
        assert!(joined.contains("failure_kind: obsolete_runtime_boundary_strategy_feedback"));
        assert!(joined.contains("raw_output_omitted: true"));
        assert!(!joined.contains("validator procedure or test-command setup"));
        assert!(!joined.contains("apply_patch for the implementation artifact"));
        assert!(!joined.contains("failure_kind: tool_execution_failed"));
    }

    #[test]
    fn action_contract_prompt_omits_obsolete_editable_validation_failure_strategy() {
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
        assert!(joined.contains("failure_kind: obsolete_runtime_boundary_strategy_feedback"));
        assert!(joined.contains("raw_output_omitted: true"));
        assert!(!joined.contains("whole-file or narrow patch"));
        assert!(!joined.contains("apply_patch for the implementation artifact"));
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
        assert!(joined.contains("will not finish the implementation node automatically"));
        assert!(joined.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!joined.contains("Action-space source of truth"));
        assert!(!joined.contains("taskspace_control action=finish_node is available"));
    }

    #[test]
    fn action_contract_prompt_structures_uncovered_high_signal_finish_rejection() {
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
        assert!(
            joined.contains("hard_state: mandatory_inspected_evidence_uncovered_by_edit_result")
        );
        assert!(!joined.contains("Action-space source of truth"));
        assert!(!joined.contains("taskspace_control action=finish_node is available"));
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
        let recent = taskspace_action_contract_recent_tool_outputs_item(&[response_item])
            .expect("recent feedback should be produced");
        let text = item_text(recent);

        assert!(text.contains("high-signal inspected evidence remains uncovered"));
        assert!(text.contains("generate_report.sh"));
        assert!(!text.contains("hard_state:"));
        assert!(!text.contains("progress_fact:"));
    }

    #[test]
    fn action_contract_control_keeps_block_node_fields_verbatim() {
        let snapshot = provider_snapshot("smoke_test");
        let args = serde_json::json!({
            "action": "block_node",
            "reason": "validation failed with IndentationError"
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("block node args should remain structurally valid");

        assert_eq!(normalized, args);
        assert!(normalized.get("node_id").is_none());
        assert!(normalized.get("blocker_summary").is_none());
    }

    #[test]
    fn action_contract_control_keeps_failed_validation_finish_verbatim() {
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
            .expect("failed validation finish should remain structurally valid");

        assert_eq!(normalized, args);
    }

    #[test]
    fn action_contract_control_does_not_rewrite_create_node_fields() {
        let args = serde_json::json!({
            "action": "create_node",
            "node_kind": "implement_solution",
            "node_title": "Fix failed validation",
            "description": "Patch merge_users.py after smoke_test failure",
            "bind_current": true
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, None)
            .expect("create node args should remain structurally valid");

        assert_eq!(normalized, args);
    }

    #[test]
    fn action_contract_control_does_not_default_create_node_fields() {
        let mut snapshot = provider_snapshot("unknown");
        snapshot.node_id = None;
        snapshot.node_kind = None;
        let args = serde_json::json!({
            "action": "create_node",
            "kind": "inspect_code_context",
            "label": "Inspect source files"
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("create node args should remain structurally valid");

        assert_eq!(normalized, args);
        assert!(normalized.get("title").is_none());
        assert!(normalized.get("context_summary").is_none());
        assert!(normalized.get("bind_current").is_none());
    }

    #[test]
    fn action_contract_control_does_not_rewrite_bind_node_intent() {
        let mut snapshot = provider_snapshot("unknown");
        snapshot.node_id = None;
        snapshot.node_kind = None;
        let args = serde_json::json!({
            "action": "bind_node",
            "child_kind": "inspect_code_context",
            "child_name": "inspect_sources"
        });

        let normalized = normalize_taskspace_action_contract_control_args(&args, Some(&snapshot))
            .expect("bind node args should remain structurally valid");

        assert_eq!(normalized, args);
        assert_eq!(normalized["action"], "bind_node");
        assert!(normalized.get("node_id").is_none());
    }

    #[test]
    fn action_contract_prompt_structures_state_commit_after_local_validator_infra_failure() {
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
        assert!(joined.contains("State ledger can record"));
        assert!(joined.contains("invalid infrastructure evidence"));
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
        assert!(joined.contains("State ledger can record"));
        assert!(joined.contains("invalid infrastructure evidence"));
    }

    #[test]
    fn action_contract_prompt_structures_recorded_local_validator_infra_failure() {
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
        assert!(joined.contains("Repeating shell discovery"));
        assert!(joined.contains("ordinary_tool_boundary: ordinary tool use remains governed"));
        assert!(!joined.contains("Action-space source of truth"));
        assert!(!joined.contains("blocked with the exact infrastructure evidence"));
    }

    #[test]
    fn action_contract_prompt_structures_platform_compatible_rework_after_recorded_local_infra() {
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
        assert!(joined.contains("State baseline rejects reusing the same infrastructure failure"));
        assert!(!joined.contains("platform-compatible validation evidence"));
        assert!(!joined.contains("blocked with the exact local validator infrastructure evidence"));
    }

    #[test]
    fn action_contract_prompt_structures_unrecoverable_local_infra_in_rework_context() {
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
        assert!(joined.contains("State baseline treats this as infrastructure evidence"));
        assert!(!joined.contains("blocked with exact infrastructure evidence is available"));
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
        assert!(text.contains("did not produce an actionable TaskSpace item"));
        assert!(text.contains("no tool result"));
        assert!(text.contains("TaskSpace progress forms accepted by the runtime"));
        assert!(text.contains("does not add task semantics"));
        assert!(!text.contains("call shell_command with `rg --files` now"));
        assert!(!text.contains("finish the inspect node into implement_solution"));
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
        assert!(text.contains("most recent blocked-tool feedback"));
        assert!(!text.contains("obey the `next_valid_actions`"));
    }

    #[test]
    fn no_action_recovery_continues_after_advisory_threshold() {
        let item = build_taskspace_no_action_recovery_item(Some("Let me inspect that next."));

        assert!(is_taskspace_no_action_recovery_item(&item));
        assert!(
            !taskspace_special_recovery_warning_message(&item).contains("HardStop"),
            "no-action recovery must remain feedback, not a provider-sampling stop"
        );
        assert!(!item_text(item).contains("Stop provider sampling for this turn"));
    }

    #[test]
    fn validation_rework_duplicate_read_remains_recoverable_after_one_recovery() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-11` and no successful edit has been recorded after that read. The previous complete read result remains available as duplicate evidence; choose any state-machine-legal action using the visible facts, or record blocked with the exact blocker. Validation repair contract: missing_required_properties=members, averageDepartmentBudget | target_artifacts=generate_org.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"reuse `result-11` or choose another state-machine-legal action\"]}";
        let recovery = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some("validation_rework result-11 artifacts=generate_org.py"),
            None,
        );

        assert!(!item_text(recovery.clone()).contains("HardStop"));
        assert!(
            !taskspace_special_recovery_warning_message(&recovery).contains("HardStop"),
            "duplicate-read recovery must not become a runtime stop"
        );
        let text = item_text(recovery);
        assert!(text.contains("target_artifact: generate_org.py"));
        assert!(text.contains("previous_read_result: result-11"));
        assert!(!text.contains("Stop provider sampling for this turn"));
    }

    #[test]
    fn validation_rework_duplicate_read_complete_context_remains_recoverable() {
        let last_message = "TaskSpace recorded duplicate read evidence because validation rework node `node-4` already read failure artifact `process.py` in result `result-11` and no successful edit has been recorded after that read. Result `result-11` is a complete read_file context (TaskSpaceReadFileSummaryV1 eof_reached=true; no additional file lines are hidden). The previous complete read result remains available as duplicate evidence. Validation repair contract: missing_required_properties=id, members, averageDepartmentBudget | target_artifacts=process.py\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"next_valid_actions\":[\"state-machine legal actions remain available\"]}";
        let recovery = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some(
                "validation_rework_target_read result=result-11 artifact=process.py | read_context: complete_read; TaskSpaceReadFileSummaryV1: path=process.py lines_read=97 eof_reached=true max_lines=240",
            ),
            None,
        );

        let text = item_text(recovery);
        assert!(!text.contains("HardStop"));
        assert!(!text.contains("Stop provider sampling for this turn"));
        assert!(text.contains("target_artifact: process.py"));
        assert!(text.contains("previous_read_result: result-11"));
        assert!(text.contains("complete read_file context"));
    }

    #[test]
    fn validation_rework_duplicate_read_repeated_gate_remains_recoverable() {
        let last_message = "TaskSpace blocked this read because validation rework node `node-4` already read failure artifact `generate_org.py` in result `result-11` and no successful edit has been recorded after that read. The previous complete read result remains available as duplicate evidence; choose any state-machine-legal action using the visible facts, or record blocked with the exact blocker.\nTaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_rework_duplicate_artifact_read\",\"blocking_items\":[\"current_node:node-4:implement_solution\",\"repeated_blocked_action:validation_rework_duplicate_artifact_read|shell_command|read|sed -n 1,240p -- generate_org.py\"],\"next_valid_actions\":[\"reuse `result-11` or choose another state-machine-legal action\"],\"repeated_blocked_action\":{\"fingerprint\":\"validation_rework_duplicate_artifact_read|shell_command|read|sed -n 1,240p -- generate_org.py\",\"repeat_count\":2,\"same_action_allowed\":false}}";
        let recovery = build_taskspace_validation_rework_duplicate_read_recovery_item(
            Some(last_message),
            Some("validation_rework result-11 artifacts=generate_org.py"),
            None,
        );

        let text = item_text(recovery);
        assert!(text.contains("failure_kind: validation_rework_duplicate_artifact_read"));
        assert!(text.contains("repeated_blocked_action"));
        assert!(!text.contains("HardStop"));
    }

    #[test]
    fn gate_recovery_message_is_preserved_by_generic_no_action_recovery() {
        let blocked_output = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_output_contract_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_json.py && python -m jsonschema -i organization.json schema.json`\"]}";

        assert!(taskspace_message_has_gate_recovery(Some(blocked_output)));
        assert!(taskspace_message_has_gate_recovery_reason(
            Some(blocked_output),
            "validation_test_missing_output_contract_coverage"
        ));
        assert!(!taskspace_message_has_gate_recovery_reason(
            Some("validation_test_missing_output_contract_coverage"),
            "validation_test_missing_output_contract_coverage"
        ));
        assert!(!taskspace_message_has_gate_recovery(Some(
            "Run python -m pytest -q now."
        )));
        let recovery = item_text(build_taskspace_no_action_recovery_item(Some(
            blocked_output,
        )));
        assert!(recovery.contains("validation_test_missing_output_contract_coverage"));
        assert!(recovery.contains("python -m jsonschema"));
        assert!(recovery.contains("most recent blocked-tool feedback"));
        assert!(!recovery.contains("obey the `next_valid_actions`"));
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
        assert!(text.contains("hard_state: local_validation_infrastructure_failure"));
        assert!(text.contains("recorded_blocker_source: exact local infrastructure evidence"));
        assert!(text.contains("Bash/Service/CreateInstance/E_ACCESSDENIED"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
    }

    #[test]
    fn validation_needs_test_recovery_blocks_discovery_loop() {
        let last = "TaskSpaceActionV1 rejected: node_policy_violation:smoke_test:list_files. Return exactly one valid taskspace-action-v1 JSON object.";
        let item = build_taskspace_validation_needs_test_recovery_item(Some(last));
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_VALIDATION_NODE_FEEDBACK_MARKER));
        assert!(text.contains("TaskSpace validation-node feedback"));
        assert!(text.contains("ordinary tool results remain recorded"));
        assert!(text.contains("preserves prior validation feedback"));
        assert!(!text.contains("Emit exactly one run_test action now"));
        assert!(!text.contains("Do not call list_files"));
        assert!(!text.contains("python scripts/validate.py"));
        assert!(!text.contains("validation_needs_test"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
        assert!(taskspace_message_hit_validation_needs_test(Some(last)));
    }

    #[test]
    fn validation_coverage_gate_alone_does_not_trigger_needs_test_recovery() {
        let last = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_changed_artifact_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_organization.py` to execute changed artifact `generate_organization.py`\"]}";
        let output_contract = "TaskSpace blocked this validation command.\n\
TaskSpaceGateRecoveryV1: {\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"reason\":\"validation_test_missing_output_contract_coverage\",\"next_valid_actions\":[\"run_test with command `python generate_json.py && python -m jsonschema -i organization.json schema.json` to execute the changed artifact and validate declared output contract(s)\"]}";

        assert!(!taskspace_message_hit_validation_needs_test(Some(last)));
        assert!(!taskspace_message_hit_validation_needs_test(Some(
            output_contract
        )));
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
    fn provider_response_actionability_classifies_final_gate_rejection_as_recovery() {
        let classification = classify_taskspace_provider_response_actionability(
            true, false, true, false, false, true, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::FinalRejected
        );
        assert!(classification.needs_recovery());
        assert_eq!(classification.as_str(), "final_rejected");
    }

    #[test]
    fn final_gate_rejection_item_is_provider_visible_mechanical_feedback() {
        let active_projection = message(
            "developer",
            &format!(
                "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\ncurrent_node: node-1 kind=inspect_code_context status=running"
            ),
        );
        let rejection = taskspace_final_answer_gate_rejection_item(
            "TaskSpace final response is unavailable while node `node-1` is running. hard_state: active_node_open.",
        )
        .expect("rejection item");

        let composition = compose_provider_visible_history(vec![active_projection, rejection]);
        let joined = item_texts(&composition.items).join("\n");

        assert!(joined.contains("TaskSpaceFinalAnswerRejectedV1"));
        assert!(joined.contains("hard_state: active_node_open"));
        assert!(joined.contains("TaskSpace state is unchanged"));
        assert!(!joined.contains("next action"));
        assert!(!joined.contains("must call"));
    }

    #[test]
    fn terminal_gate_rejection_feedback_is_neutral_state_error() {
        let final_feedback = taskspace_final_answer_gate_rejection_followup("missing criterion");
        let blocked_feedback = taskspace_blocked_gate_rejection_followup("missing evidence");

        assert!(final_feedback.contains("TaskSpaceFinalAnswerRejectedV1"));
        assert!(final_feedback.contains("accepted: false"));
        assert!(final_feedback.contains("state_effect: final_answer was not recorded"));
        assert!(!final_feedback.contains("Continue the same task"));
        assert!(!final_feedback.contains("Correct the specific rejection reason"));

        assert!(blocked_feedback.contains("TaskSpaceBlockedResponseRejectedV1"));
        assert!(blocked_feedback.contains("accepted: false"));
        assert!(blocked_feedback.contains("state_effect: blocked response was not recorded"));
        assert!(!blocked_feedback.contains("Continue the same task"));
        assert!(!blocked_feedback.contains("Correct the specific rejection reason"));
    }

    #[test]
    fn provider_response_actionability_final_rejection_overrides_actionable_output() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, false, false, true, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::FinalRejected
        );
        assert!(classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_classifies_no_action_follow_up() {
        let classification = classify_taskspace_provider_response_actionability(
            true, false, true, false, false, false, false,
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
            false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::FinalCandidate
        );
        assert!(!classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_keeps_actionable_response_out_of_recovery() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, false, false, false, false,
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
            true, true, true, true, false, false, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::ToolFeedbackRecovery
        );
        assert!(classification.needs_recovery());
        assert_eq!(classification.as_str(), "tool_feedback_recovery");
    }

    #[test]
    fn provider_response_actionability_treats_gate_recovery_without_tool_output_as_no_action() {
        let classification = classify_taskspace_provider_response_actionability(
            true, false, true, true, false, false, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::NoActionFollowUp
        );
        assert!(classification.needs_recovery());
        assert_eq!(classification.as_str(), "no_action_follow_up");
    }

    #[test]
    fn provider_response_actionability_treats_tool_failure_feedback_as_tool_feedback_recovery() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, false, true, false, false,
        );

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::ToolFeedbackRecovery
        );
        assert!(classification.needs_recovery());
        assert_eq!(classification.as_str(), "tool_feedback_recovery");
    }

    #[test]
    fn action_sequence_failure_feedback_detects_failed_edit_and_test() {
        let failed_patch = tool_output_with_call_id(
            "patch-call",
            "apply_patch verification failed: Failed to find expected lines in src/lib.rs",
        );
        let failed_test =
            tool_output_with_call_id("test-call", "Exit code: 1\nOutput:\nfailed assertion");
        let successful_test = tool_output_with_call_id("test-call", "Exit code: 0\nOutput:\nok");

        assert!(
            taskspace_sequence_failure_feedback_from_response_item("apply_patch", &failed_patch)
                .is_some()
        );
        assert!(
            taskspace_sequence_failure_feedback_from_response_item("run_test", &failed_test)
                .is_some()
        );
        assert!(
            taskspace_sequence_failure_feedback_from_response_item("run_test", &successful_test)
                .is_none()
        );
        assert!(
            taskspace_sequence_failure_feedback_from_response_item("read_file", &failed_test)
                .is_none()
        );
    }

    #[test]
    fn provider_response_actionability_treats_profile_hint_overrun_as_actionable() {
        let classification = classify_taskspace_provider_response_actionability(
            true, true, true, false, false, false, true,
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
    fn active_context_replacement_recognizes_thin_projection_without_legacy_profile_marker() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_THIN_PROJECTION_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\ncurrent_node: node-1 kind=inspect_code_context"
        );
        let items = vec![
            message(
                "developer",
                "TaskSpace mode is now active.\nBootstrap status: no TaskSpace task exists. taskspace_control(action=start_task) is required before ordinary tools.",
            ),
            message("developer", &active_projection),
            message("user", "Preserve the current bug-fix requirement."),
        ];

        let prepared = prepare_provider_visible_prompt_items(items);
        let texts = item_texts(&prepared);
        let joined = texts.join("\n");

        assert_eq!(prepared.len(), 2);
        assert!(joined.contains(TASKSPACE_ACTIVE_THIN_PROJECTION_MARKER));
        assert!(joined.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER));
        assert!(!joined.contains("TaskSpaceAgentContextBundleV1:"));
        assert!(joined.contains("Preserve the current bug-fix requirement."));
        assert!(!joined.contains("Bootstrap status: no TaskSpace task exists"));
        assert!(!joined.contains("TaskSpace mode is now active."));
        assert!(!joined.contains("taskspace_control(action=start_task)"));
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
    fn active_context_replacement_preserves_current_gate_feedback_pair() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix the bug"
        );
        let items = vec![
            message("developer", &active_projection),
            tool_call("shell_command", "blocked-call"),
            tool_output_with_call_id(
                "blocked-call",
                "TaskSpace blocked this tool call.\nTaskSpaceGateRecoveryV1: {\"allowed\":false,\"gate_class\":\"state_machine\",\"reason\":\"current_node_binding_missing\"}",
            ),
            message("user", "Keep the direct user requirement."),
        ];

        let composition = compose_provider_visible_history(items);
        let texts = item_texts(&composition.items);
        let joined = texts.join("\n");

        assert!(composition.items.iter().any(|item| matches!(
            item,
            ResponseItem::FunctionCall { call_id, .. }
                if call_id == "blocked-call"
        )));
        assert!(joined.contains("TaskSpaceGateRecoveryV1"));
        assert!(joined.contains("Keep the direct user requirement."));
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Include
        );
        assert_eq!(
            composition.decisions[2].action,
            ProviderVisibleHistoryAction::Include
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

    #[test]
    fn final_readiness_recovery_preserves_ledger_update_shape() {
        let rejection = taskspace_final_answer_gate_rejection_followup(
            "TaskSpace final answer cannot be emitted until every success criterion and output contract is satisfied or waived with evidence. Missing ledger items: success_criterion id=criterion-1 kind=test status=open evidence_refs=1 description=\"run validator\". Recent result refs available to cite: result-8:test. State record schema available: taskspace_control action=state_commit schema_version=taskspace-state-commit-v1 accepts success_criteria/output_contracts entries with explicit id, status, kind, description, and evidence_refs. Final readiness is evaluated from the latest ledger state.",
        );
        let text = item_text(build_taskspace_final_readiness_recovery_item(Some(
            &rejection,
        )));

        assert!(text.contains(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER));
        assert!(text.contains("final_answer_rejected: true"));
        assert!(text.contains("criterion-1"));
        assert!(text.contains("taskspace_control action=state_commit"));
        assert!(text.contains("success_criteria entries"));
        assert!(text.contains("result_validities entries"));
        assert!(!text.contains("then retry final_answer"));
        assert!(!text.contains(TASKSPACE_NO_ACTION_RECOVERY_MARKER));
    }

    #[test]
    fn active_context_replacement_preserves_final_readiness_recovery() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: finish synthesis"
        );
        let rejection = taskspace_final_answer_gate_rejection_followup(
            "TaskSpace final answer cannot be emitted until every success criterion and output contract is satisfied or waived with evidence. Missing ledger items: output_contract id=contract-1 status=open evidence_refs=1 description=\"summarize changed files\". State record schema available: taskspace_control action=state_commit schema_version=taskspace-state-commit-v1 accepts success_criteria/output_contracts entries with explicit id, status, kind, description, and evidence_refs.",
        );
        let recovery = build_taskspace_final_readiness_recovery_item(Some(&rejection));
        let items = vec![message("developer", &active_projection), recovery];

        let composition = compose_provider_visible_history(items);
        let joined = item_texts(&composition.items).join("\n");

        assert!(joined.contains(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER));
        assert!(joined.contains("contract-1"));
        assert!(joined.contains("taskspace_control action=state_commit"));
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Include
        );
    }

    #[test]
    fn active_context_replacement_omits_stale_final_readiness_recovery_after_ledger_satisfied() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\n\
active_objective: finish synthesis\n\
success_criteria:\n\
  - criterion-1 status=satisfied run validator\n\
  - output-contract-1 status=satisfied summarize changed files"
        );
        let rejection = taskspace_final_answer_gate_rejection_followup(
            "TaskSpace final answer cannot be emitted until every success criterion and output contract is satisfied or waived with evidence. Missing ledger items: success_criterion id=criterion-1 kind=test status=open evidence_refs=1 description=\"run validator\"; output_contract id=output-contract-1 kind=artifact status=open evidence_refs=1 description=\"summarize changed files\". Recent result refs available to cite: result-8:test. State record schema available: taskspace_control action=state_commit schema_version=taskspace-state-commit-v1 accepts success_criteria/output_contracts entries with explicit id, status, kind, description, and evidence_refs.",
        );
        let recovery = build_taskspace_final_readiness_recovery_item(Some(&rejection));
        let items = vec![message("developer", &active_projection), recovery];

        let composition = compose_provider_visible_history(items);
        let joined = item_texts(&composition.items).join("\n");

        assert!(!joined.contains(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER));
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Omit(
                "stale_final_readiness_recovery_satisfied_by_projection"
            )
        );
    }

    #[test]
    fn active_context_replacement_places_latest_final_readiness_recovery_after_projection() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: finish synthesis"
        );
        let stale_recovery =
            build_taskspace_final_readiness_recovery_item(Some("stale missing id=old-criterion"));
        let latest_recovery =
            build_taskspace_final_readiness_recovery_item(Some("latest missing id=new-criterion"));
        let items = vec![
            message("user", "Keep the user task."),
            stale_recovery,
            message("developer", &active_projection),
            latest_recovery,
        ];

        let composition = compose_provider_visible_history(items);
        let texts = item_texts(&composition.items);
        let joined = texts.join("\n");
        let projection_pos = texts
            .iter()
            .position(|text| text.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER))
            .expect("projection should remain visible");
        let recovery_pos = texts
            .iter()
            .position(|text| text.contains(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER))
            .expect("latest recovery should remain visible");

        assert!(recovery_pos > projection_pos);
        assert_eq!(
            joined
                .matches(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER)
                .count(),
            1
        );
        assert!(!joined.contains("old-criterion"));
        assert!(joined.contains("new-criterion"));
        assert_eq!(
            composition.decisions[1].action,
            ProviderVisibleHistoryAction::Omit("stale_final_readiness_recovery_replaced")
        );
        assert_eq!(
            composition.decisions[3].action,
            ProviderVisibleHistoryAction::Include
        );
    }

    #[test]
    fn action_contract_prompt_preserves_latest_final_readiness_recovery_after_projection() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: finish synthesis"
        );
        let stale_recovery =
            build_taskspace_final_readiness_recovery_item(Some("stale missing id=old-criterion"));
        let latest_recovery =
            build_taskspace_final_readiness_recovery_item(Some("latest missing id=new-criterion"));
        let items = vec![
            message("user", "Keep the user task."),
            stale_recovery,
            message("developer", &active_projection),
            latest_recovery,
        ];

        let prepared =
            prepare_taskspace_action_contract_prompt_items_for_node(items, Some("final_synthesis"));
        let texts = item_texts(&prepared);
        let joined = texts.join("\n");
        let projection_pos = texts
            .iter()
            .position(|text| text.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER))
            .expect("projection should remain visible");
        let recovery_pos = texts
            .iter()
            .position(|text| text.contains(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER))
            .expect("latest recovery should remain visible");

        assert!(recovery_pos > projection_pos);
        assert_eq!(
            joined
                .matches(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER)
                .count(),
            1
        );
        assert!(!joined.contains("old-criterion"));
        assert!(joined.contains("new-criterion"));
    }

    #[test]
    fn action_contract_prompt_omits_stale_final_readiness_recovery_after_projection() {
        let active_projection = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\n\
active_objective: finish synthesis\n\
success_criteria:\n\
  - criterion-1 status=satisfied run validator"
        );
        let recovery = build_taskspace_final_readiness_recovery_item(Some(
            "TaskSpace final answer cannot be emitted until every success criterion and output contract is satisfied or waived with evidence. Missing ledger items: success_criterion id=criterion-1 kind=test status=open evidence_refs=1 description=\"run validator\". Recent result refs available to cite: result-8:test. State record schema available: taskspace_control action=state_commit schema_version=taskspace-state-commit-v1 accepts success_criteria entries.",
        ));
        let items = vec![
            message("user", "Keep the user task."),
            message("developer", &active_projection),
            recovery,
        ];

        let prepared =
            prepare_taskspace_action_contract_prompt_items_for_node(items, Some("final_synthesis"));
        let joined = item_texts(&prepared).join("\n");

        assert!(joined.contains("Keep the user task."));
        assert!(joined.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER));
        assert!(!joined.contains(TASKSPACE_FINAL_READINESS_RECOVERY_MARKER));
    }
}

#[derive(Debug)]
struct SamplingRequestResult {
    needs_follow_up: bool,
    last_agent_message: Option<String>,
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

fn parse_taskspace_action_v1(text: &str) -> Result<TaskSpaceActionV1, String> {
    let mut actions = parse_taskspace_actions_v1(text)?;
    if actions.len() != 1 {
        return Err("action_sequence_not_single_action".to_string());
    }
    Ok(actions.remove(0))
}

fn parse_taskspace_actions_v1(text: &str) -> Result<Vec<TaskSpaceActionV1>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty_action_contract_output".to_string());
    }
    if trimmed.starts_with("```") {
        if let Some(fenced_json) = taskspace_single_fenced_json_body(trimmed) {
            return parse_taskspace_actions_v1(fenced_json);
        }
        return Err("action_contract_output_not_strict_json".to_string());
    }
    if !trimmed.starts_with('{') {
        if let Some(json_start) = taskspace_prefixed_action_json_start(trimmed) {
            return parse_taskspace_actions_v1(&trimmed[json_start..]);
        }
        return taskspace_action_from_deepseek_dsml(trimmed)
            .map(|action| vec![action])
            .ok_or_else(|| "action_contract_output_not_strict_json".to_string());
    }
    let json_end = taskspace_leading_json_object_end(trimmed)
        .ok_or_else(|| "malformed_action_json:unterminated_object".to_string())?;
    let value = serde_json::from_str::<serde_json::Value>(&trimmed[..json_end])
        .map_err(|err| format!("malformed_action_json:{err}"))?;
    let suffix = trimmed[json_end..].trim();
    let action_name_for_suffix = value
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !suffix.is_empty()
        && !suffix.contains("DSML")
        && !(suffix == "\"" && action_name_for_suffix == "apply_patch")
    {
        return Err("action_contract_output_not_strict_json".to_string());
    }
    let schema_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let actions = if schema_version == "taskspace-action-sequence-v1" {
        let sequence = serde_json::from_value::<TaskSpaceActionSequenceV1>(value)
            .map_err(|err| format!("malformed_action_json:{err}"))?;
        if sequence.actions.is_empty() {
            return Err("empty_action_sequence".to_string());
        }
        if sequence.actions.len() > TASKSPACE_ACTION_CONTRACT_MAX_SEQUENCE_ACTIONS {
            return Err("action_sequence_too_long".to_string());
        }
        sequence.actions
    } else {
        vec![
            serde_json::from_value::<TaskSpaceActionV1>(value)
                .map_err(|err| format!("malformed_action_json:{err}"))?,
        ]
    };
    for action in &actions {
        if action.schema_version != "taskspace-action-v1" {
            return Err("unsupported_action_schema_version".to_string());
        }
        if action.action.trim().is_empty() {
            return Err("missing_action".to_string());
        }
    }
    Ok(actions)
}

fn taskspace_prefixed_action_json_start(text: &str) -> Option<usize> {
    text.match_indices('{').find_map(|(index, _)| {
        let candidate = text.get(index..)?;
        let after_open = candidate.strip_prefix('{')?.trim_start();
        after_open
            .starts_with("\"schema_version\"")
            .then_some(index)
    })
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
                | "run_test"
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

#[cfg(test)]
fn taskspace_action_is_finish_node_control(action: &TaskSpaceActionV1) -> bool {
    taskspace_action_control_action(action) == Some("finish_node")
}

#[cfg(test)]
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

#[cfg(test)]
fn taskspace_action_control_action(action: &TaskSpaceActionV1) -> Option<&str> {
    if let Some(control_action) = taskspace_top_level_control_action(action.action.as_str()) {
        return Some(control_action);
    }
    let root = action.args.as_object()?;
    taskspace_control_action_from_root(root)
}

fn taskspace_top_level_control_action(action: &str) -> Option<&'static str> {
    match action {
        "start_task" => Some("start_task"),
        "create_node" => Some("create_node"),
        "bind_node" => Some("bind_node"),
        "finish_node" => Some("finish_node"),
        "block_node" => Some("block_node"),
        "record_fact" => Some("record_fact"),
        "record_fact_source" => Some("record_fact_source"),
        "record_output_contract" => Some("record_output_contract"),
        "record_success_criteria" => Some("record_success_criteria"),
        _ => None,
    }
}

fn taskspace_canonical_action_name(action: &str) -> &str {
    if taskspace_top_level_control_action(action).is_some() {
        "taskspace_control"
    } else {
        action
    }
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

#[cfg(test)]
fn taskspace_action_control_creates_validation_node(action: &TaskSpaceActionV1) -> bool {
    if taskspace_action_control_action(action) != Some("create_node") {
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

#[cfg(test)]
fn taskspace_action_control_creates_final_synthesis(action: &TaskSpaceActionV1) -> bool {
    taskspace_action_control_action(action) == Some("create_node")
        && action
            .args
            .get("node_kind")
            .or_else(|| action.args.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("final_synthesis")
}

#[cfg(test)]
fn taskspace_final_answer_action(message: &str) -> TaskSpaceActionV1 {
    TaskSpaceActionV1 {
        schema_version: "taskspace-action-v1".to_string(),
        action: "final_answer".to_string(),
        node_id: None,
        args: serde_json::json!({ "message": message }),
        rationale: Some("Thin TaskSpace path is complete after validation.".to_string()),
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

#[cfg(test)]
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

#[cfg(test)]
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
    } else if lines.last() == Some(&"*** End Patch") {
        lines.pop();
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

        let mut file_hunk_lines = Vec::new();
        while index < lines.len() {
            if lines[index].starts_with("--- ")
                && lines
                    .get(index + 1)
                    .is_some_and(|candidate| candidate.starts_with("+++ "))
            {
                break;
            }
            file_hunk_lines.push(lines[index]);
            index += 1;
        }
        let normalized_hunk_lines =
            normalize_taskspace_unified_diff_hunks_for_native_patch(&file_hunk_lines);
        if !normalized_hunk_lines.iter().any(|line| line == "@@") {
            return None;
        }
        rewritten.extend(normalized_hunk_lines);
        converted_files += 1;
    }
    if converted_files == 0 {
        return None;
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
}

fn normalize_taskspace_unified_diff_hunks_for_native_patch(lines: &[&str]) -> Vec<String> {
    let mut rewritten = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        if !lines[index].starts_with("@@") {
            rewritten.push(normalize_taskspace_unified_hunk_line(lines[index]));
            index += 1;
            continue;
        }
        let hunk_start = index;
        index += 1;
        while index < lines.len() && !lines[index].starts_with("@@") {
            index += 1;
        }
        rewritten.extend(normalize_taskspace_unified_diff_hunk_for_native_patch(
            &lines[hunk_start..index],
        ));
    }
    rewritten
}

fn normalize_taskspace_unified_diff_hunk_for_native_patch(hunk: &[&str]) -> Vec<String> {
    let Some(header) = hunk.first() else {
        return Vec::new();
    };
    let body = &hunk[1..];
    let has_add = body
        .iter()
        .any(|line| line.starts_with('+') && !line.starts_with("+++"));
    let has_delete = body
        .iter()
        .any(|line| line.starts_with('-') && !line.starts_with("---"));
    if !has_add || !has_delete {
        return hunk
            .iter()
            .map(|line| normalize_taskspace_unified_hunk_line(line))
            .collect();
    }

    let Some(first_change) = body
        .iter()
        .position(|line| taskspace_unified_hunk_change_line(line))
    else {
        return vec![normalize_taskspace_unified_hunk_line(header)];
    };
    let Some(last_change) = body
        .iter()
        .rposition(|line| taskspace_unified_hunk_change_line(line))
    else {
        return vec![normalize_taskspace_unified_hunk_line(header)];
    };

    let leading_anchor = body[..first_change]
        .iter()
        .rposition(|line| taskspace_unified_hunk_nonempty_context_line(line));
    let trailing_anchor = (leading_anchor.is_none()).then(|| {
        body[last_change.saturating_add(1)..]
            .iter()
            .position(|line| taskspace_unified_hunk_nonempty_context_line(line))
            .map(|offset| last_change + 1 + offset)
    });

    let mut rewritten = vec![normalize_taskspace_unified_hunk_line(header)];
    if let Some(anchor_index) = leading_anchor {
        rewritten.push(body[anchor_index].to_string());
    }
    rewritten.extend(body[first_change..=last_change].iter().map(|line| {
        if line.starts_with("@@") {
            normalize_taskspace_unified_hunk_line(line)
        } else {
            (*line).to_string()
        }
    }));
    if let Some(Some(anchor_index)) = trailing_anchor {
        rewritten.push(body[anchor_index].to_string());
    }
    rewritten
}

fn taskspace_unified_hunk_change_line(line: &str) -> bool {
    (line.starts_with('+') && !line.starts_with("+++"))
        || (line.starts_with('-') && !line.starts_with("---"))
}

fn taskspace_unified_hunk_nonempty_context_line(line: &str) -> bool {
    line.starts_with(' ') && !line.trim().is_empty()
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
    if line.starts_with("@@") {
        "@@".to_string()
    } else {
        line.to_string()
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

fn taskspace_apply_patch_misordered_begin_targets(patch: &str) -> Vec<String> {
    let lines = patch.lines().collect::<Vec<_>>();
    if lines.first().is_some_and(|line| *line == "*** Begin Patch") {
        return Vec::new();
    }
    if !lines.iter().any(|line| *line == "*** Begin Patch") {
        return Vec::new();
    }
    let targets = taskspace_apply_patch_declared_targets(patch);
    if targets.is_empty() {
        vec!["(missing patch target)".to_string()]
    } else {
        targets
    }
}

fn taskspace_apply_patch_placeholder_range_hunk_targets(patch: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut current_target = None::<String>;
    let mut pending_ellipsis_target = None::<String>;
    let mut pending_ellipsis_has_context = false;
    let finish_pending_ellipsis =
        |targets: &mut Vec<String>,
         pending_ellipsis_target: &mut Option<String>,
         pending_ellipsis_has_context: &mut bool| {
            if let Some(target) = pending_ellipsis_target.take()
                && !*pending_ellipsis_has_context
                && !targets.iter().any(|existing| existing == &target)
            {
                targets.push(target);
            }
            *pending_ellipsis_has_context = false;
        };
    for line in patch.lines() {
        if let Some(target) = line.strip_prefix("*** Update File: ").map(str::trim) {
            finish_pending_ellipsis(
                &mut targets,
                &mut pending_ellipsis_target,
                &mut pending_ellipsis_has_context,
            );
            current_target = (!target.is_empty()).then(|| target.to_string());
            continue;
        }
        if line.starts_with("*** Add File: ")
            || line.starts_with("*** Delete File: ")
            || line.starts_with("*** End Patch")
        {
            finish_pending_ellipsis(
                &mut targets,
                &mut pending_ellipsis_target,
                &mut pending_ellipsis_has_context,
            );
            current_target = None;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("@@") {
            finish_pending_ellipsis(
                &mut targets,
                &mut pending_ellipsis_target,
                &mut pending_ellipsis_has_context,
            );
            if trimmed.contains("...")
                && (trimmed.contains("-...") || trimmed.contains("+..."))
                && let Some(target) = current_target.as_ref()
                && !targets.iter().any(|existing| existing == target)
            {
                targets.push(target.clone());
            } else if matches!(trimmed, "@@ ... @@" | "@@...@@") {
                pending_ellipsis_target = current_target.clone();
            }
            continue;
        }
        if pending_ellipsis_target.is_some() && line.starts_with(' ') && !line.trim().is_empty() {
            pending_ellipsis_has_context = true;
        }
    }
    finish_pending_ellipsis(
        &mut targets,
        &mut pending_ellipsis_target,
        &mut pending_ellipsis_has_context,
    );
    targets
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

#[cfg(test)]
fn taskspace_action_to_tool_call(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
) -> Result<Option<ToolCall>, String> {
    taskspace_action_to_tool_call_with_sequence_index(action, snapshot, None)
}

fn taskspace_action_to_tool_call_with_sequence_index(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sequence_index: Option<usize>,
) -> Result<Option<ToolCall>, String> {
    let raw_action_name = action.action.as_str();
    let action_name = taskspace_canonical_action_name(raw_action_name);
    if action_name == "final_answer" {
        return Ok(None);
    }
    if let (Some(expected), Some(actual)) = (snapshot.node_id.as_deref(), action.node_id.as_deref())
        && expected != actual
    {
        return Err("node_id_mismatch".to_string());
    }
    let args = &action.args;
    let call_id = taskspace_action_contract_call_id(
        snapshot.request_count.saturating_add(1),
        raw_action_name,
        sequence_index,
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
            let raw_patch = taskspace_action_arg_string(args, "patch")
                .ok_or_else(|| "missing_apply_patch_patch".to_string())?;
            let misordered_begin_targets =
                taskspace_apply_patch_misordered_begin_targets(&raw_patch);
            if !misordered_begin_targets.is_empty() {
                return Err(format!(
                    "apply_patch_mixed_native_unified:{}",
                    misordered_begin_targets.join(",")
                ));
            }
            if taskspace_apply_patch_missing_unified_header_target(&raw_patch) {
                return Err("apply_patch_mixed_native_unified:(missing patch target)".to_string());
            }
            let existing_add_targets =
                taskspace_unified_diff_add_targets_existing_files(&raw_patch);
            if !existing_add_targets.is_empty() {
                return Err(format!(
                    "apply_patch_existing_file_as_add:{}",
                    existing_add_targets.join(",")
                ));
            }
            let malformed_native_operation_targets =
                taskspace_apply_patch_malformed_native_operation_targets(&raw_patch);
            if !malformed_native_operation_targets.is_empty() {
                return Err(format!(
                    "apply_patch_native_hunk_header:{}",
                    malformed_native_operation_targets.join(",")
                ));
            }
            let native_hunk_header_targets =
                taskspace_apply_patch_native_hunk_header_targets(&raw_patch);
            if !native_hunk_header_targets.is_empty() {
                return Err(format!(
                    "apply_patch_native_hunk_header:{}",
                    native_hunk_header_targets.join(",")
                ));
            }
            let placeholder_range_hunk_targets =
                taskspace_apply_patch_placeholder_range_hunk_targets(&raw_patch);
            if !placeholder_range_hunk_targets.is_empty() {
                return Err(format!(
                    "apply_patch_mixed_native_unified:{}",
                    placeholder_range_hunk_targets.join(",")
                ));
            }
            let patch = normalize_taskspace_unified_diff_patch(&raw_patch)
                .unwrap_or_else(|| normalize_taskspace_apply_patch(&raw_patch));
            let native_hunk_header_targets =
                taskspace_apply_patch_native_hunk_header_targets(&patch);
            if !native_hunk_header_targets.is_empty() {
                return Err(format!(
                    "apply_patch_native_hunk_header:{}",
                    native_hunk_header_targets.join(",")
                ));
            }
            let placeholder_range_hunk_targets =
                taskspace_apply_patch_placeholder_range_hunk_targets(&patch);
            if !placeholder_range_hunk_targets.is_empty() {
                return Err(format!(
                    "apply_patch_mixed_native_unified:{}",
                    placeholder_range_hunk_targets.join(",")
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
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("apply_patch"),
                call_id,
                payload: ToolPayload::Custom { input: patch },
            }))
        }
        "run_test" => {
            let command = taskspace_action_arg_string(args, "command")
                .ok_or_else(|| "missing_run_test_command".to_string())?;
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
                if let Some(control_action) = taskspace_top_level_control_action(raw_action_name)
                    && let Some(root) = normalized.as_object_mut()
                {
                    root.entry("action".to_string())
                        .or_insert_with(|| serde_json::Value::String(control_action.to_string()));
                }
                if let (Some(root), Some(node_id)) =
                    (normalized.as_object_mut(), action.node_id.as_deref())
                    && !root.contains_key("node_id")
                {
                    root.insert(
                        "node_id".to_string(),
                        serde_json::Value::String(node_id.to_string()),
                    );
                }
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

fn taskspace_action_contract_call_id(
    request_index: usize,
    raw_action_name: &str,
    sequence_index: Option<usize>,
) -> String {
    if let Some(sequence_index) = sequence_index {
        format!("taskspace-action-contract-{request_index}-{sequence_index}-{raw_action_name}")
    } else {
        format!("taskspace-action-contract-{request_index}-{raw_action_name}")
    }
}

const TASKSPACE_PARQUET_PREVIEW_SCRIPT: &str = r#"import json
import sys

path = sys.argv[1]
try:
    import pandas as pd
except Exception as exc:
    print(f"TaskSpaceStructuredFilePreviewV1: path={path} format=parquet status=dependency_unavailable error={type(exc).__name__}: {exc}")
    print(f"TaskSpaceReadFileSummaryV1: path={path} lines_read=0 eof_reached=true max_lines=20 structured_preview=false")
    raise SystemExit(1)

try:
    df = pd.read_parquet(path)
except Exception as exc:
    print(f"TaskSpaceStructuredFilePreviewV1: path={path} format=parquet status=read_error error={type(exc).__name__}: {exc}")
    print(f"TaskSpaceReadFileSummaryV1: path={path} lines_read=0 eof_reached=true max_lines=20 structured_preview=false")
    raise SystemExit(1)

preview = df.head(20)
columns = [str(column) for column in df.columns]
print(f"TaskSpaceStructuredFilePreviewV1: path={path} format=parquet status=ok rows={len(df)} columns={json.dumps(columns, ensure_ascii=False)}")
print(preview.to_json(orient="records", date_format="iso"))
print(f"TaskSpaceReadFileSummaryV1: path={path} lines_read={len(preview)} eof_reached=true max_lines=20 structured_preview=true")
"#;

fn taskspace_read_file_command(path: &str) -> String {
    if taskspace_path_has_extension(path, ".parquet") {
        return taskspace_parquet_read_file_command(path);
    }
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

fn taskspace_path_has_extension(path: &str, extension: &str) -> bool {
    path.trim().to_ascii_lowercase().ends_with(extension)
}

fn taskspace_parquet_read_file_command(path: &str) -> String {
    let path_arg = codex_shell_command::parse_command::shlex_join(&[path.to_string()]);
    if cfg!(windows) {
        let ps_path = path.replace('"', "`\"");
        format!(
            "@'\n{script}\n'@ | python - \"{ps_path}\"",
            script = TASKSPACE_PARQUET_PREVIEW_SCRIPT
        )
    } else {
        format!(
            "if command -v python >/dev/null 2>&1; then python - {path_arg} <<'PY'\n{script}\nPY\n\
elif command -v python3 >/dev/null 2>&1; then python3 - {path_arg} <<'PY'\n{script}\nPY\n\
else printf 'TaskSpaceStructuredFilePreviewV1: path=%s format=parquet status=python_unavailable\\nTaskSpaceReadFileSummaryV1: path=%s lines_read=0 eof_reached=true max_lines=20 structured_preview=false\\n' {path_arg} {path_arg}; fi",
            script = TASKSPACE_PARQUET_PREVIEW_SCRIPT
        )
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
    _snapshot: Option<&crate::action_map::ActionMapProviderRequestBudgetSnapshot>,
) -> Result<serde_json::Value, String> {
    let mut normalized = args.clone();
    let Some(root) = normalized.as_object_mut() else {
        return Err(TASKSPACE_CONTROL_ARGS_NOT_OBJECT_ERROR.to_string());
    };
    canonicalize_taskspace_control_action_arg(root)?;
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
        "start_task" => {
            move_taskspace_json_alias(root, "task_name", "task_title");
            move_taskspace_json_alias(root, "task_description", "task_objective");
            move_taskspace_json_alias(root, "summary", "task_objective");
            move_taskspace_json_alias(root, "first_node", "node_title");
            move_taskspace_json_alias(root, "first_node_kind", "node_kind");
            move_taskspace_json_alias(root, "initial_node_kind", "node_kind");
            move_taskspace_json_alias(root, "first_node_id", "node_title");
            move_taskspace_json_alias(root, "first_node_title", "node_title");
            move_taskspace_json_alias(root, "first_node_description", "node_context_summary");
            move_taskspace_json_alias(root, "first_node_context", "node_context_summary");
            move_taskspace_json_alias(root, "initial_node_context", "node_context_summary");
            move_taskspace_json_alias(root, "description", "node_context_summary");
            move_taskspace_json_alias(root, "success_criteria", "initial_success_criteria");
            move_taskspace_json_alias(root, "initial_criteria", "initial_success_criteria");
            move_taskspace_json_alias(root, "criteria", "initial_success_criteria");
            move_taskspace_json_alias(root, "output_contracts", "initial_output_contracts");
            move_taskspace_json_alias(root, "initial_contracts", "initial_output_contracts");
            move_taskspace_json_alias(root, "contracts", "initial_output_contracts");
            move_taskspace_json_alias(root, "fact_sources", "initial_fact_sources");
        }
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

#[cfg(test)]
fn taskspace_bootstrap_action_to_tool_call(
    action: &TaskSpaceActionV1,
) -> Result<Option<ToolCall>, String> {
    taskspace_bootstrap_action_to_tool_call_with_sequence_index(action, None)
}

fn taskspace_bootstrap_action_to_tool_call_with_sequence_index(
    action: &TaskSpaceActionV1,
    sequence_index: Option<usize>,
) -> Result<Option<ToolCall>, String> {
    let raw_action_name = action.action.as_str();
    let action_name = taskspace_canonical_action_name(raw_action_name);
    if !taskspace_action_allowed_for_node(action_name, None) {
        return Err(format!("bootstrap_policy_violation:{raw_action_name}"));
    }
    match action_name {
        "taskspace_control" => {
            let mut args = action.args.clone();
            if let Some(control_action) = taskspace_top_level_control_action(raw_action_name)
                && let Some(root) = args.as_object_mut()
            {
                root.entry("action".to_string())
                    .or_insert_with(|| serde_json::Value::String(control_action.to_string()));
            }
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("taskspace_control"),
                call_id: taskspace_action_contract_bootstrap_call_id(
                    raw_action_name,
                    sequence_index,
                ),
                payload: ToolPayload::Function {
                    arguments: normalize_taskspace_action_contract_control_args(&args, None)?
                        .to_string(),
                },
            }))
        }
        "blocked" => Ok(None),
        _ => Err(format!("unsupported_bootstrap_action:{raw_action_name}")),
    }
}

fn taskspace_action_contract_bootstrap_call_id(
    raw_action_name: &str,
    sequence_index: Option<usize>,
) -> String {
    if let Some(sequence_index) = sequence_index {
        format!("taskspace-action-contract-bootstrap-{sequence_index}-{raw_action_name}")
    } else {
        format!("taskspace-action-contract-bootstrap-{raw_action_name}")
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
        "TaskSpaceFinalAnswerRejectedV1:\n\
accepted: false\n\
rejection_reason: {error}\n\
state_effect: final_answer was not recorded; TaskSpace state is unchanged."
    )
}

fn taskspace_final_answer_gate_rejection_item(error: &str) -> Option<ResponseItem> {
    crate::context_manager::updates::build_developer_update_item(vec![
        taskspace_final_answer_gate_rejection_followup(error),
    ])
}

fn taskspace_blocked_gate_rejection_followup(error: &str) -> String {
    format!(
        "TaskSpaceBlockedResponseRejectedV1:\n\
accepted: false\n\
rejection_reason: {error}\n\
state_effect: blocked response was not recorded; TaskSpace state is unchanged."
    )
}

fn taskspace_blocked_gate_rejection_item(error: &str) -> Option<ResponseItem> {
    crate::context_manager::updates::build_developer_update_item(vec![
        taskspace_blocked_gate_rejection_followup(error),
    ])
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
    let provider_request_budget = provider_budget_snapshot
        .as_ref()
        .map(|snapshot| {
            let transport_mode =
                taskspace_provider_transport_mode(&turn_context, Some(snapshot), true);
            let budget_state = match transport_mode {
                TaskspaceProviderTransportMode::NativeTools => snapshot.budget_state.clone(),
                TaskspaceProviderTransportMode::CacheOptimizedActionContract => String::new(),
            };
            ProviderRequestBudgetContext::enabled_with_attribution(
                ProviderRequestBudgetLimits {
                    request_count: snapshot.request_count,
                    max_requests: snapshot.max_requests,
                    node_request_count: snapshot.node_request_count,
                    max_model_requests_per_node: snapshot.max_model_requests_per_node,
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
    let mut tool_path_correction_feedback: Option<TaskspacePathCorrection> = None;
    let mut path_correction_cleared_this_request = false;
    if action_contract_mode
        && let Some(failed_read_summary) = sess
            .action_map_current_recent_failed_read_summary()
            .await
            .filter(|summary| !summary.trim().is_empty())
    {
        tool_path_correction_feedback = taskspace_path_correction_from_text(&failed_read_summary);
    }
    let mut saw_actionable_output = false;
    let mut active_item: Option<TurnItem> = None;
    let mut active_tool_argument_diff_consumer: Option<(
        String,
        Box<dyn ToolArgumentDiffConsumer>,
    )> = None;
    let mut should_emit_turn_diff = false;
    let mut taskspace_terminal_action_observed_in_request = false;
    let mut taskspace_final_response_rejected_in_request = false;
    let plan_mode = turn_context.collaboration_mode.mode == ModeKind::Plan;
    let mut assistant_message_stream_parsers = AssistantMessageStreamParsers::new(plan_mode);
    let mut plan_mode_state = plan_mode.then(|| PlanModeStreamState::new(&turn_context.sub_id));
    let receiving_span = trace_span!("receiving_stream");
    let outcome: CodexResult<SamplingRequestResult> = loop {
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
                    match parse_taskspace_actions_v1(raw_text) {
                        Ok(actions) => {
                            let action_count = actions.len();
                            for (action_index, action) in actions.into_iter().enumerate() {
                                let sequence_index = (action_count > 1).then_some(action_index + 1);
                                let action_name =
                                    taskspace_canonical_action_name(action.action.as_str())
                                        .to_string();
                                let final_message = taskspace_action_final_message(&action);
                                let current_snapshot = sess
                                    .action_map_provider_request_budget_snapshot()
                                    .await
                                    .or_else(|| provider_budget_snapshot.clone());
                                let tool_call = if let Some(snapshot) = current_snapshot.as_ref() {
                                    taskspace_action_to_tool_call_with_sequence_index(
                                        &action,
                                        snapshot,
                                        sequence_index,
                                    )
                                } else {
                                    taskspace_bootstrap_action_to_tool_call_with_sequence_index(
                                        &action,
                                        sequence_index,
                                    )
                                };
                                match tool_call {
                                    Ok(Some(tool_call)) => {
                                        let call_item =
                                            response_item_for_taskspace_action_tool_call(
                                                &tool_call,
                                            );
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
                                        let mut tool_failed_result_message: Option<String> = None;
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
                                                if let Some(correction) =
                                                    taskspace_path_correction_from_response_item(
                                                        &response_item,
                                                    )
                                                {
                                                    tool_path_correction_feedback =
                                                        Some(correction);
                                                } else if taskspace_action_can_clear_path_correction_feedback(
                                                    &action,
                                                ) {
                                                    tool_path_correction_feedback = None;
                                                    path_correction_cleared_this_request = true;
                                                }
                                                tool_gate_recovery_message =
                                                    taskspace_gate_recovery_from_response_item(
                                                        &response_item,
                                                    );
                                                tool_failed_result_message =
                                                    taskspace_sequence_failure_feedback_from_response_item(
                                                        action_name.as_str(),
                                                        &response_item,
                                                    );
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
                                                tool_error_message = Some(format!(
                                                    "TaskSpace tool call failed: {err}"
                                                ));
                                                tool_path_correction_feedback =
                                                    taskspace_path_correction_from_text(
                                                        &err.to_string(),
                                                    );
                                            }
                                        }
                                        if let Some(message) = tool_gate_recovery_message {
                                            last_agent_message = Some(message);
                                            break;
                                        } else if let Some(message) = tool_failed_result_message {
                                            last_agent_message = Some(message);
                                            break;
                                        } else if let Some(message) = tool_error_message {
                                            last_agent_message = Some(message);
                                            break;
                                        } else if let Some(rationale) = action.rationale.as_deref()
                                            && !rationale.trim().is_empty()
                                        {
                                            last_agent_message = Some(rationale.to_string());
                                        }
                                    }
                                    Ok(None) if final_message.is_some() => {
                                        let final_message = final_message.unwrap_or_default();
                                        let terminal_gate_error = if action.action == "final_answer"
                                        {
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
                                                .validate_action_map_terminal_blocker(
                                                    &final_message,
                                                )
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
                                            saw_actionable_output = false;
                                            let (feedback, feedback_item) = if action.action
                                                == "final_answer"
                                            {
                                                taskspace_final_response_rejected_in_request = true;
                                                (
                                                    taskspace_final_answer_gate_rejection_followup(
                                                        &error,
                                                    ),
                                                    taskspace_final_answer_gate_rejection_item(
                                                        &error,
                                                    ),
                                                )
                                            } else {
                                                (
                                                    taskspace_blocked_gate_rejection_followup(
                                                        &error,
                                                    ),
                                                    taskspace_blocked_gate_rejection_item(&error),
                                                )
                                            };
                                            if let Some(item) = feedback_item {
                                                sess.record_conversation_items(
                                                    &turn_context,
                                                    std::slice::from_ref(&item),
                                                )
                                                .await;
                                            }
                                            last_agent_message = Some(feedback);
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
                                        break;
                                    }
                                    Ok(None) => {}
                                    Err(reason) => {
                                        needs_follow_up = true;
                                        let rejection =
                                            taskspace_action_contract_rejection_followup(&reason);
                                        sess.record_action_map_runtime_feedback(
                                            &turn_context,
                                            "action_contract_rejection",
                                            false,
                                            rejection.clone(),
                                        )
                                        .await;
                                        last_agent_message = Some(rejection);
                                        break;
                                    }
                                }
                            }
                        }
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
                            let rejection = if patch_intent_suffix.is_empty() {
                                taskspace_action_contract_rejection_followup(&reason)
                            } else {
                                format!(
                                    "TaskSpaceActionV1 rejected: {reason}{patch_intent_suffix}. Return exactly one valid taskspace-action-v1 JSON object or one valid taskspace-action-sequence-v1 envelope."
                                )
                            };
                            sess.record_action_map_runtime_feedback(
                                &turn_context,
                                "action_contract_rejection",
                                false,
                                rejection.clone(),
                            )
                            .await;
                            last_agent_message = Some(rejection);
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
                let mut final_response_rejected = taskspace_final_response_rejected_in_request;
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
                            let feedback = taskspace_final_answer_gate_rejection_followup(&error);
                            if let Some(item) = taskspace_final_answer_gate_rejection_item(&error) {
                                sess.record_conversation_items(
                                    &turn_context,
                                    std::slice::from_ref(&item),
                                )
                                .await;
                            }
                            last_agent_message = Some(feedback);
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
                let taskspace_progress_after_request =
                    sess.action_map_current_main_node_progress_signature().await;
                if taskspace_should_refill_path_correction_from_failed_read_summary(
                    tool_path_correction_feedback.is_some(),
                    path_correction_cleared_this_request,
                    taskspace_progress_before_request,
                    taskspace_progress_after_request,
                ) && let Some(failed_read_summary) =
                    sess.action_map_current_recent_failed_read_summary().await
                {
                    tool_path_correction_feedback =
                        taskspace_path_correction_from_text(&failed_read_summary);
                }
                let response_actionability = classify_taskspace_provider_response_actionability(
                    needs_follow_up,
                    saw_actionable_output,
                    assistant_message_present,
                    taskspace_message_has_gate_recovery(last_agent_message.as_deref()),
                    tool_path_correction_feedback.is_some(),
                    final_response_rejected,
                    false,
                );
                if let Some(snapshot) = current_budget_snapshot.as_ref() {
                    sess.record_action_map_provider_response_actionability(
                        &turn_context,
                        snapshot.clone(),
                        ActionMapProviderResponseActionabilityInput {
                            response_actionability: response_actionability.as_str().to_string(),
                            end_turn,
                            saw_actionable_output,
                            assistant_message_present,
                            recovery_action: "none".to_string(),
                            last_agent_message_preview: taskspace_last_message_preview(
                                last_agent_message.as_deref(),
                            ),
                        },
                    )
                    .await;
                }
                if final_response_rejected {
                    last_agent_message = None;
                }
                break Ok(SamplingRequestResult {
                    needs_follow_up,
                    last_agent_message,
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
    if outcome.is_ok()
        && let Some(snapshot) = sess.action_map_provider_request_budget_snapshot().await
        && snapshot.node_kind.as_deref() == Some("implement_solution")
    {
        let _ = record_taskspace_observed_implement_edit(
            &sess,
            &turn_context,
            &turn_diff_tracker,
            snapshot.request_count,
        )
        .await;
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
