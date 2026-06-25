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
const TASKSPACE_NO_ACTION_RECOVERY_MARKER: &str = "TaskSpaceNoActionRecoveryV1:";
const TASKSPACE_FORCED_INSPECT_TRANSITION_MARKER: &str =
    "TaskSpaceForcedInspectTransitionRecoveryV1:";
const TASKSPACE_FORCED_IMPLEMENT_TRANSITION_MARKER: &str =
    "TaskSpaceForcedImplementTransitionRecoveryV1:";
const TASKSPACE_INSPECT_BOOTSTRAP_CALL_ID: &str = "taskspace-inspect-bootstrap-rg-files";
const TASKSPACE_INSPECT_TEST_BOOTSTRAP_CALL_ID: &str = "taskspace-inspect-bootstrap-pytest";
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
                    let counts_against_no_action_cap =
                        is_taskspace_no_action_recovery_item(&recovery_item);
                    let no_action_recovery_cap = sess
                        .action_map_provider_request_budget_snapshot()
                        .await
                        .and_then(|snapshot| snapshot.node_kind)
                        .as_deref()
                        .map(taskspace_no_action_recovery_cap_for_node_kind)
                        .unwrap_or(1usize);
                    if counts_against_no_action_cap
                        && taskspace_no_action_recovery_count >= no_action_recovery_cap
                    {
                        sess.send_event(
                            &turn_context,
                            EventMsg::Error(ErrorEvent {
                                message: format!("TaskSpace stopped this turn because the model produced too many non-action assistant messages while requesting follow-up ({taskspace_no_action_recovery_count}/{no_action_recovery_cap} recoveries spent). It must emit a tool call, taskspace_control transition, or a final blocked-with-evidence answer instead of commentary-only output."),
                                codex_error_info: None,
                            }),
                        )
                        .await;
                        return None;
                    }
                    if counts_against_no_action_cap {
                        taskspace_no_action_recovery_count += 1;
                    }
                    sess.send_event(
                        &turn_context,
                        EventMsg::Warning(WarningEvent {
                            message: if counts_against_no_action_cap {
                                format!("TaskSpace inserted TaskSpaceNoActionRecoveryV1 because the provider response requested follow-up or was rejected by the final-response gate without an actionable tool/control/final result. Recovery attempt {}/{} is being used.", taskspace_no_action_recovery_count, no_action_recovery_cap)
                            } else if response_item_text_contains(
                                &recovery_item,
                                TASKSPACE_FORCED_IMPLEMENT_TRANSITION_MARKER,
                            ) {
                                "TaskSpace inserted TaskSpaceForcedImplementTransitionRecoveryV1 after a provider-budget forced implement transition. This guidance does not consume the no-action recovery allowance.".to_string()
                            } else {
                                "TaskSpace inserted TaskSpaceForcedInspectTransitionRecoveryV1 after a provider-budget forced inspect transition. This guidance does not consume the no-action recovery allowance.".to_string()
                            },
                        }),
                    )
                    .await;
                    sess.record_conversation_items(&turn_context, &[recovery_item])
                        .await;
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
- inspect_code_context: list_files, search, read_file, taskspace_control, blocked
- implement_solution: list_files, search, read_file, apply_patch, taskspace_control, blocked
- smoke_test/regression_test: run_test, read_file, search, taskspace_control, blocked
- final_synthesis: final_answer, taskspace_control, blocked
Action argument rules:
- list_files args: {\"path\":\".\"}
- search args: {\"pattern\":\"literal or regex\",\"path\":\".\"}
- read_file args: {\"path\":\"relative/path\"}
- apply_patch args: {\"patch\":\"*** Begin Patch\\n...\\n*** End Patch\\n\"}
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
    let text = format!(
        "TaskSpaceActionContractStateV1:\n\
Active node id: {node_id}\n\
Active node kind: {node_kind}"
    );
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

fn build_taskspace_no_action_recovery_item(last_message: Option<&str>) -> ResponseItem {
    let previous = last_message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("(no assistant text was captured)");
    let text = format!(
        "{TASKSPACE_NO_ACTION_RECOVERY_MARKER}\n\
The previous assistant message requested follow-up but did not produce effective TaskSpace progress: no successful tool result, taskspace_control transition, or final response accepted by TaskSpace was recorded.\n\
Previous assistant message: {previous}\n\
Required behavior for the next response:\n\
- Do not send commentary-only text such as \"let me check\".\n\
- Emit exactly one actionable operation now: a tool call, a taskspace_control finish/transition/state_commit, or a final blocked-with-evidence answer with the exact missing evidence.\n\
- If the current node is inspect_code_context and no source/test evidence has been read yet, call shell_command with `rg --files` now.\n\
- If inspect evidence is sufficient, finish the inspect node into implement_solution before more environment probing.\n\
- If a tool was blocked, follow that tool output's recovery instructions instead of repeating the same blocked action.\n\
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

fn is_taskspace_no_action_recovery_item(item: &ResponseItem) -> bool {
    response_item_text_contains(item, TASKSPACE_NO_ACTION_RECOVERY_MARKER)
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
        "先跑",
        "跑测试",
        "运行测试",
        "执行测试",
        "确认当前失败",
    ];
    FOLLOW_UP_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn classify_taskspace_provider_response_actionability(
    needs_follow_up: bool,
    saw_actionable_output: bool,
    assistant_message_present: bool,
    final_response_rejected: bool,
    _provider_budget_exhausted_followup: bool,
) -> TaskspaceProviderResponseActionability {
    if final_response_rejected {
        TaskspaceProviderResponseActionability::FinalRejected
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
                prepare_taskspace_action_contract_prompt_items(prompt_source)
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

fn prepare_taskspace_action_contract_prompt_items(items: Vec<ResponseItem>) -> Vec<ResponseItem> {
    let mut latest_user_input: Option<(usize, ResponseItem)> = None;
    let mut latest_taskspace_context: Option<ResponseItem> = None;
    let mut tool_outputs: Vec<(usize, ResponseItem)> = Vec::new();

    for (index, item) in items.into_iter().enumerate() {
        if is_taskspace_active_context_item(&item) {
            latest_taskspace_context = Some(item);
        } else if is_protected_user_input(&item) {
            latest_user_input = Some((index, item));
        } else if is_taskspace_action_contract_latest_tool_output_candidate(&item) {
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
    if let Some(item) = latest_taskspace_context {
        prepared.push(item);
    }
    let post_user_outputs = tool_outputs
        .into_iter()
        .filter_map(|(index, item)| {
            if index > latest_user_index {
                Some(item)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if let Some(item) = taskspace_action_contract_recent_tool_outputs_item(&post_user_outputs) {
        prepared.push(item);
    }
    prepared
}

fn is_taskspace_active_context_item(item: &ResponseItem) -> bool {
    is_active_context_projection_item(item)
        || response_item_text_contains(item, TASKSPACE_ACTIVE_PROFILE_MARKER)
        || response_item_text_contains(item, "TaskSpace mode is now active.")
}

fn prompt_contains_taskspace_active_context(prompt: &Prompt) -> bool {
    prompt.input.iter().any(is_taskspace_active_context_item)
}

fn is_taskspace_action_contract_latest_tool_output_candidate(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. }
    ) && !is_legacy_taskspace_tool_output(item)
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
    let edit_success_seen = summaries
        .iter()
        .any(|(_, text)| text.contains("Success. Updated the following files"));

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
            remaining_chars = 0;
        } else {
            remaining_chars = remaining_chars.saturating_sub(char_count);
        }
        sections.push(format!("call_id: {call_id}\noutput:\n{output}"));
    }
    if sections.is_empty() {
        return None;
    }

    let progress_hint = if edit_success_seen {
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
            prepared.push(item);
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

fn response_item_text_contains(item: &ResponseItem, needle: &str) -> bool {
    response_item_texts_contain(item, &|text| text.contains(needle))
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
    fn taskspace_action_contract_parser_rejects_non_json_text() {
        let err =
            parse_taskspace_action_v1("```json\n{}\n```").expect_err("must reject fenced text");
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
    fn taskspace_action_contract_policy_rejects_dsml_test_in_inspect_node() {
        let action = parse_taskspace_action_v1(
            r#"<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="shell_command">
<｜｜DSML｜｜parameter name="command" string="true">python -m pytest tests/test_tax_calc.py -v</｜｜DSML｜｜parameter>
</｜｜DSML｜｜invoke>
</｜｜DSML｜｜tool_calls>"#,
        )
        .expect("known DSML test command should map to run_test");
        let err =
            taskspace_action_to_tool_call(&action, &provider_snapshot("inspect_code_context"))
                .expect_err("inspect nodes cannot run tests");

        assert!(err.contains("node_policy_violation:inspect_code_context:run_test"));
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
            ),
            TaskspaceProviderResponseActionability::FinalCandidate
        );
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
        assert!(!taskspace_action_allowed_for_node(
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
    fn forced_transition_recovery_item_does_not_count_as_no_action_retry() {
        let item = build_taskspace_forced_inspect_transition_recovery_item();
        let text = item_text(item.clone());

        assert!(text.contains(TASKSPACE_FORCED_INSPECT_TRANSITION_MARKER));
        assert!(text.contains("Current required behavior"));
        assert!(!is_taskspace_no_action_recovery_item(&item));
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
        let classification =
            classify_taskspace_provider_response_actionability(true, false, true, true, false);

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::FinalRejected
        );
        assert!(classification.needs_recovery());
        assert_eq!(classification.as_str(), "final_rejected");
    }

    #[test]
    fn provider_response_actionability_classifies_no_action_follow_up() {
        let classification =
            classify_taskspace_provider_response_actionability(true, false, true, false, false);

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::NoActionFollowUp
        );
        assert!(classification.needs_recovery());
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
            Some("我已经看到了问题所在。先跑测试确认当前失败。")
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
        let classification =
            classify_taskspace_provider_response_actionability(true, true, true, false, false);

        assert_eq!(
            classification,
            TaskspaceProviderResponseActionability::Actionable
        );
        assert!(!classification.needs_recovery());
    }

    #[test]
    fn provider_response_actionability_treats_profile_hint_overrun_as_actionable() {
        let classification =
            classify_taskspace_provider_response_actionability(true, true, true, false, true);

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
                let response_item = response_input.into();
                sess.record_conversation_items(&turn_context, std::slice::from_ref(&response_item))
                    .await;
                mark_thread_memory_mode_polluted_if_external_context(
                    sess.as_ref(),
                    turn_context.as_ref(),
                    &response_item,
                )
                .await;
            }
            Err(err) => {
                error_or_panic(format!("in-flight tool future failed during drain: {err}"));
            }
        }
    }
    Ok(())
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
                call_id,
                payload: ToolPayload::Function { arguments },
            },
            cancellation_token,
        )
        .await?;
    let response_item: ResponseItem = response_input.into();
    sess.record_conversation_items(&turn_context, std::slice::from_ref(&response_item))
        .await;
    mark_thread_memory_mode_polluted_if_external_context(
        sess.as_ref(),
        turn_context.as_ref(),
        &response_item,
    )
    .await;
    Ok(())
}

fn parse_taskspace_action_v1(text: &str) -> Result<TaskSpaceActionV1, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty_action_contract_output".to_string());
    }
    if trimmed.starts_with("```") {
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
    if !suffix.is_empty() && !suffix.contains("DSML") {
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
            "list_files" | "search" | "read_file" | "taskspace_control" | "blocked"
        ),
        Some("implement_solution") => matches!(
            action,
            "list_files" | "read_file" | "search" | "apply_patch" | "taskspace_control" | "blocked"
        ),
        Some("smoke_test" | "regression_test") => matches!(
            action,
            "run_test" | "read_file" | "search" | "taskspace_control" | "blocked"
        ),
        Some("final_synthesis") => {
            matches!(action, "final_answer" | "taskspace_control" | "blocked")
        }
        _ => matches!(action, "taskspace_control" | "blocked"),
    }
}

async fn should_finish_node_after_successful_required_action(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if taskspace_action_is_finish_node_control(action) || taskspace_action_is_terminal(action) {
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

async fn should_answer_after_successful_validation_finish_node(
    action: &TaskSpaceActionV1,
    snapshot: &crate::action_map::ActionMapProviderRequestBudgetSnapshot,
    sess: &Session,
) -> bool {
    if !taskspace_action_is_finish_node_control(action) {
        return false;
    }
    if !matches!(
        snapshot.node_kind.as_deref(),
        Some("smoke_test" | "regression_test")
    ) {
        return false;
    }
    sess.action_map_current_main_node_has_successful_action(ActionClass::Test)
        .await
        || sess
            .action_map_current_main_node_has_successful_action(ActionClass::Build)
            .await
}

fn taskspace_action_is_finish_node_control(action: &TaskSpaceActionV1) -> bool {
    action.action == "taskspace_control"
        && taskspace_action_control_action(action) == Some("finish_node")
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
    if let Some(rewritten) = normalize_taskspace_bare_file_patch(&normalized) {
        return rewritten;
    }
    let lines = normalized.lines().collect::<Vec<_>>();
    if lines.len() < 5
        || lines.first() != Some(&"*** Begin Patch")
        || lines.last() != Some(&"*** End Patch")
        || !lines
            .iter()
            .any(|line| line.starts_with("*** Update File: "))
    {
        return normalized;
    }
    rewrite_taskspace_apply_patch_unique_update_paths(&normalized)
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
    let path =
        resolve_unique_existing_relative_path(raw_path).unwrap_or_else(|| raw_path.to_string());
    let mut rewritten = Vec::with_capacity(lines.len() + 1);
    rewritten.push("*** Begin Patch".to_string());
    rewritten.push(format!("*** Update File: {path}"));
    rewritten.push("@@".to_string());
    for line in lines.iter().skip(2).take(lines.len().saturating_sub(3)) {
        if line.trim().is_empty() && rewritten.last().is_some_and(|last| last == "@@") {
            continue;
        }
        rewritten.push((*line).to_string());
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
}

fn normalize_taskspace_unified_diff_patch(patch: &str) -> Option<String> {
    let normalized = patch.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    if lines.first() != Some(&"*** Begin Patch") || lines.last() != Some(&"*** End Patch") {
        return None;
    }
    let old_idx = lines.iter().position(|line| line.starts_with("--- "))?;
    let new_idx = lines.iter().position(|line| line.starts_with("+++ "))?;
    if new_idx != old_idx + 1 {
        return None;
    }
    let path = lines[new_idx].strip_prefix("+++ ")?.trim();
    let path = path.strip_prefix("b/").unwrap_or(path);
    if path.is_empty() {
        return None;
    }
    let path = resolve_unique_existing_relative_path(path).unwrap_or_else(|| path.to_string());
    let mut rewritten = Vec::with_capacity(lines.len());
    rewritten.push("*** Begin Patch".to_string());
    rewritten.push(format!("*** Update File: {path}"));
    for line in lines
        .iter()
        .skip(new_idx + 1)
        .take(lines.len().saturating_sub(new_idx + 2))
    {
        rewritten.push(normalize_taskspace_unified_hunk_line(line));
    }
    rewritten.push("*** End Patch".to_string());
    Some(rewritten.join("\n") + "\n")
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

fn rewrite_taskspace_apply_patch_unique_update_paths(patch: &str) -> String {
    let lines = patch.lines().collect::<Vec<_>>();
    let mut rewritten = Vec::with_capacity(lines.len());
    let mut changed = false;
    for line in lines {
        let Some(path) = line.strip_prefix("*** Update File: ") else {
            rewritten.push(line.to_string());
            continue;
        };
        let candidate = resolve_unique_existing_relative_path(path.trim());
        if let Some(candidate) = candidate {
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

fn resolve_unique_existing_relative_path(path: &str) -> Option<String> {
    resolve_unique_existing_relative_path_from(Path::new("."), path)
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
                "command": format!("Get-Content -LiteralPath {:?} -TotalCount 240", path),
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
            let patch = normalize_taskspace_unified_diff_patch(&patch)
                .unwrap_or_else(|| normalize_taskspace_apply_patch(&patch));
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
        "taskspace_control" => {
            let arguments =
                normalize_taskspace_action_contract_control_args(args, Some(snapshot))?.to_string();
            Ok(Some(ToolCall {
                tool_name: ToolName::plain("taskspace_control"),
                call_id,
                payload: ToolPayload::Function { arguments },
            }))
        }
        "blocked" => Ok(None),
        _ => Err(format!("unsupported_action:{action_name}")),
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
    if inner_action != "finish_node" {
        return Ok(normalized);
    }
    let Some(snapshot) = snapshot else {
        return Ok(normalized);
    };
    if !root.contains_key("node_id")
        && let Some(node_id) = snapshot.node_id.as_deref()
    {
        root.insert(
            "node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );
    }
    if snapshot.node_kind.as_deref() == Some("implement_solution") {
        root.entry("next_node_kind".to_string())
            .or_insert_with(|| serde_json::Value::String("smoke_test".to_string()));
        if root
            .get("next_node_kind")
            .and_then(serde_json::Value::as_str)
            == Some("smoke_test")
        {
            root.entry("next_node_title".to_string())
                .or_insert_with(|| serde_json::Value::String("Run focused validation".to_string()));
            root.entry("next_node_context_summary".to_string())
                .or_insert_with(|| {
                    serde_json::Value::String(
                        "Run the focused test command after the implementation edit.".to_string(),
                    )
                });
            if !root.contains_key("next_dependency_node_ids")
                && let Some(node_id) = snapshot.node_id.as_deref()
            {
                root.insert(
                    "next_dependency_node_ids".to_string(),
                    serde_json::json!([node_id]),
                );
            }
        }
    }
    Ok(normalized)
}

fn normalize_taskspace_action_contract_test_command(command: &str) -> String {
    let trimmed = command.trim();
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
            return if suffix.is_empty() {
                format!("pytest tests/{file}")
            } else {
                format!("pytest tests/{file} {suffix}")
            };
        }
        if file.ends_with(".py")
            && !Path::new(file).exists()
            && let Some(resolved) = resolve_unique_test_file_for_missing_pytest_path(file)
        {
            let suffix = suffix.trim();
            return if suffix.is_empty() {
                format!("pytest {resolved}")
            } else {
                format!("pytest {resolved} {suffix}")
            };
        }
    }
    trimmed.to_string()
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

async fn record_taskspace_observed_implement_edit(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    turn_diff_tracker: &SharedTurnDiffTracker,
    request_count: usize,
) -> bool {
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
                            let action = if let Some(snapshot) = provider_budget_snapshot.as_ref()
                                && should_finish_node_after_successful_required_action(
                                    &action,
                                    snapshot,
                                    sess.as_ref(),
                                )
                                .await
                            {
                                taskspace_finish_current_node_action(
                                    snapshot.node_id.as_deref(),
                                    "Required node work already succeeded; finishing node.",
                                )
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
                                && should_answer_after_successful_validation_finish_node(
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
                            in_flight.push_back(Box::pin(
                                tool_runtime
                                    .clone()
                                    .handle_tool_call(tool_call, cancellation_token.child_token()),
                            ));
                            if let Some(rationale) = action.rationale.as_deref()
                                && !rationale.trim().is_empty()
                            {
                                last_agent_message = Some(rationale.to_string());
                            }
                        }
                        Ok((_action, Some(final_message), None)) => {
                            apply_taskspace_terminal_action_message(
                                &mut needs_follow_up,
                                &mut saw_actionable_output,
                                &mut last_agent_message,
                                final_message,
                            );
                            taskspace_terminal_action_observed = true;
                        }
                        Ok((_action, None, None)) => {}
                        Err(reason) => {
                            needs_follow_up = true;
                            last_agent_message = Some(format!(
                                "TaskSpaceActionV1 rejected: {reason}. Return exactly one valid taskspace-action-v1 JSON object."
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
                if !needs_follow_up
                    && let Some(message) = last_agent_message.as_deref()
                    && sess
                        .record_action_map_main_final_response(&turn_context, message)
                        .await
                        .is_err()
                {
                    needs_follow_up = true;
                    final_response_rejected = true;
                }
                let current_budget_snapshot =
                    sess.action_map_provider_request_budget_snapshot().await;
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
                    final_response_rejected
                        || budget_pressure_follow_up_intent
                        || budget_pressure_silent_action_transition,
                    provider_budget_exhausted_followup,
                );
                let mut taskspace_no_action_recovery_item =
                    if response_actionability.needs_recovery() && current_budget_snapshot.is_some()
                    {
                        Some(build_taskspace_no_action_recovery_item(
                            last_agent_message.as_deref(),
                        ))
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
        && result.needs_follow_up
        && result.taskspace_no_action_recovery_item.is_none()
        && taskspace_progress_before_request.is_some()
    {
        let taskspace_progress_after_request =
            sess.action_map_current_main_node_progress_signature().await;
        if taskspace_progress_after_request == taskspace_progress_before_request {
            let inspect_bootstrap = provider_budget_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.node_kind.as_deref() == Some("inspect_code_context"))
                .cloned();
            if let Some(snapshot) = inspect_bootstrap {
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
