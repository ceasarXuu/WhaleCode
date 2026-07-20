use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::action_map::ActionClass;
use crate::action_map::ActionMapAssignment;
use crate::action_map::ActionMapExactPayloadScanEventInput;
use crate::action_map::ActionMapProviderRequestBudgetEventInput;
use crate::action_map::ActionMapProviderRequestBudgetSnapshot;
use crate::action_map::ActionMapProviderResponseActionabilityInput;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::TaskSpaceEvent;
use crate::action_map::ToolActionDescriptor;
use crate::action_map::build_snapshot_delta;
use crate::action_map::snapshot_sha256;
use crate::agent::AgentControl;
use crate::agent::AgentStatus;
use crate::agent::Mailbox;
use crate::agent::MailboxReceiver;
use crate::agent::agent_status_from_event;
use crate::agent::status::is_final;
use crate::build_available_skills;
use crate::client::ProviderProjectionIdentityExpectation;
use crate::client::ProviderRequestBudgetEvent;
use crate::commit_attribution::commit_message_trailer_instruction;
use crate::compact;
use crate::config::ManagedFeatures;
use crate::connectors;
use crate::context::ApprovedCommandPrefixSaved;
use crate::context::AppsInstructions;
use crate::context::AvailablePluginsInstructions;
use crate::context::AvailableSkillsInstructions;
use crate::context::CollaborationModeInstructions;
use crate::context::ContextualUserFragment;
use crate::context::NetworkRuleSaved;
use crate::context::PermissionsInstructions;
use crate::context::PersonalitySpecInstructions;
use crate::default_skill_metadata_budget;
use crate::environment_selection::selected_primary_environment;
use crate::environment_selection::validate_environment_selections;
use crate::exec_policy::ExecPolicyManager;
use crate::installation_id::resolve_installation_id;
use crate::parse_turn_item;
use crate::path_utils::normalize_for_native_workdir;
use crate::realtime_conversation::RealtimeConversationManager;
use crate::rollout::find_thread_name_by_id;
use crate::session_prefix::format_subagent_notification_message;
use crate::skills::SkillRenderSideEffects;
use crate::skills_load_input_from_config;
use crate::turn_metadata::TurnMetadataState;
use async_channel::Receiver;
use async_channel::Sender;
use chrono::Local;
use chrono::Utc;
use codex_analytics::AnalyticsEventsClient;
use codex_analytics::SubAgentThreadStartedInput;
use codex_app_server_protocol::McpServerElicitationRequest;
use codex_app_server_protocol::McpServerElicitationRequestParams;
use codex_config::types::OAuthCredentialsStoreMode;
use codex_exec_server::Environment;
use codex_exec_server::EnvironmentManager;
use codex_exec_server::FileSystemSandboxContext;
use codex_features::FEATURES;
use codex_features::Feature;
use codex_features::unstable_features_warning_event;
use codex_hooks::Hooks;
use codex_hooks::HooksConfig;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::auth_env_telemetry::collect_auth_env_telemetry;
use codex_login::default_client::originator;
use codex_mcp::McpConnectionManager;
use codex_mcp::McpRuntimeEnvironment;
use codex_mcp::ToolInfo;
use codex_mcp::codex_apps_tools_cache_key;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::manager::SharedModelsManager;
use codex_network_proxy::NetworkProxy;
use codex_network_proxy::NetworkProxyAuditMetadata;
use codex_network_proxy::normalize_host;
use codex_otel::current_span_trace_id;
use codex_otel::current_span_w3c_trace_context;
use codex_otel::set_parent_from_w3c_trace_context;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::ToolName;
use codex_protocol::account::PlanType as AccountPlanType;
use codex_protocol::approvals::ElicitationRequestEvent;
use codex_protocol::approvals::ExecPolicyAmendment;
use codex_protocol::approvals::NetworkPolicyAmendment;
use codex_protocol::approvals::NetworkPolicyRuleAction;
use codex_protocol::config_types::ApprovalsReviewer;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::dynamic_tools::DynamicToolResponse;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::items::TurnItem;
use codex_protocol::items::UserMessageItem;
use codex_protocol::mcp::CallToolResult;
use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxEnforcement;
use codex_protocol::models::format_allow_prefixes;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_protocol::protocol::FileChange;
use codex_protocol::protocol::HasLegacyEvent;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::ItemStartedEvent;
use codex_protocol::protocol::RawResponseItemEvent;
use codex_protocol::protocol::ReviewRequest;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::TurnAbortReason;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::protocol::TurnContextNetworkItem;
use codex_protocol::protocol::TurnEnvironmentSelection;
use codex_protocol::protocol::W3cTraceContext;
use codex_protocol::request_permissions::PermissionGrantScope;
use codex_protocol::request_permissions::RequestPermissionProfile;
use codex_protocol::request_permissions::RequestPermissionsArgs;
use codex_protocol::request_permissions::RequestPermissionsEvent;
use codex_protocol::request_permissions::RequestPermissionsResponse;
use codex_protocol::request_user_input::RequestUserInputArgs;
use codex_protocol::request_user_input::RequestUserInputResponse;
use codex_rmcp_client::ElicitationResponse;
use codex_rollout::state_db;
use codex_rollout_trace::AgentResultTracePayload;
use codex_rollout_trace::ThreadStartedTraceMetadata;
use codex_rollout_trace::ThreadTraceContext;
use codex_sandboxing::policy_transforms::intersect_permission_profiles;
use codex_shell_command::parse_command::parse_command;
use codex_terminal_detection::user_agent;
use codex_thread_store::CreateThreadParams;
use codex_thread_store::LiveThread;
use codex_thread_store::LiveThreadInitGuard;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::ResumeThreadParams;
use codex_thread_store::ThreadEventPersistenceMode;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_utils_output_truncation::TruncationPolicy;
use futures::future::BoxFuture;
use futures::future::Shared;
use futures::prelude::*;
use rmcp::model::ListResourceTemplatesResult;
use rmcp::model::ListResourcesResult;
use rmcp::model::PaginatedRequestParams;
use rmcp::model::ReadResourceRequestParams;
use rmcp::model::ReadResourceResult;
use rmcp::model::RequestId;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use toml::Value as TomlValue;
use tracing::Instrument;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::instrument;
use tracing::warn;
use uuid::Uuid;

use crate::client::ModelClient;
use crate::codex_thread::ThreadConfigSnapshot;
use crate::compact::collect_user_messages;
use crate::config::Config;
use crate::config::Constrained;
use crate::config::ConstraintResult;
use crate::config::GhostSnapshotConfig;
use crate::config::StartedNetworkProxy;
use crate::config::resolve_web_search_mode_for_turn;
use crate::context_manager::ContextManager;
use crate::context_manager::TotalTokenUsageBreakdown;
use crate::thread_rollout_truncation::initial_history_has_prior_user_turns;
use crate::tools::output_reference::reference_text_for_raw_output;
use crate::tools::output_reference::write_output_artifact_for_rollout;
use codex_config::CONFIG_TOML_FILE;
use codex_config::types::McpServerConfig;
use codex_config::types::ShellEnvironmentPolicy;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
#[cfg(test)]
use codex_protocol::exec_output::StreamOutput;

mod handlers;
mod mcp;
mod review;
mod rollout_reconstruction;
#[allow(clippy::module_inception)]
pub(crate) mod session;
mod taskspace_terminal;
pub(crate) mod turn;
pub(crate) mod turn_context;
#[cfg(test)]
use self::handlers::submission_dispatch_span;
use self::handlers::submission_loop;
use self::review::spawn_review_thread;
use self::session::AppServerClientMetadata;
use self::session::Session;
use self::session::SessionConfiguration;
pub(crate) use self::session::SessionSettingsUpdate;
pub(crate) use self::taskspace_terminal::FinishActionMapError;
#[cfg(test)]
use self::turn::AssistantMessageStreamParsers;
#[cfg(test)]
use self::turn::collect_explicit_app_ids_from_skill_items;
#[cfg(test)]
use self::turn::filter_connectors_for_input;
use self::turn::realtime_text_for_event;
use self::turn_context::TurnContext;
use self::turn_context::TurnSkillsContext;
#[cfg(test)]
mod rollout_reconstruction_tests;

#[derive(Debug, PartialEq)]
pub enum SteerInputError {
    NoActiveTurn(Vec<UserInput>),
    ExpectedTurnMismatch { expected: String, actual: String },
    ActiveTurnNotSteerable { turn_kind: NonSteerableTurnKind },
    EmptyInput,
}

impl SteerInputError {
    fn to_error_event(&self) -> ErrorEvent {
        match self {
            Self::NoActiveTurn(_) => ErrorEvent {
                message: "no active turn to steer".to_string(),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            },
            Self::ExpectedTurnMismatch { expected, actual } => ErrorEvent {
                message: format!("expected active turn id `{expected}` but found `{actual}`"),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            },
            Self::ActiveTurnNotSteerable { turn_kind } => {
                let turn_kind_label = match turn_kind {
                    NonSteerableTurnKind::Review => "review",
                    NonSteerableTurnKind::Compact => "compact",
                };
                ErrorEvent {
                    message: format!("cannot steer a {turn_kind_label} turn"),
                    codex_error_info: Some(CodexErrorInfo::ActiveTurnNotSteerable {
                        turn_kind: *turn_kind,
                    }),
                }
            }
            Self::EmptyInput => ErrorEvent {
                message: "input must not be empty".to_string(),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            },
        }
    }
}

/// Notes from the previous real user turn.
///
/// Conceptually this is the same role that `previous_model` used to fill, but
/// it can carry other prior-turn settings that matter when constructing
/// sensible state-change diffs or full-context reinjection, such as model
/// switches or detecting a prior `realtime_active -> false` transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviousTurnSettings {
    pub(crate) model: String,
    pub(crate) realtime_active: Option<bool>,
}

use crate::SkillError;
use crate::SkillLoadOutcome;
use crate::SkillMetadata;
use crate::SkillsManager;
use crate::action_map::ProjectionEmission;
use crate::action_map::ProjectionEnvelope;
use crate::action_map::ProjectionTrigger;
use crate::action_map::decide_projection_emission;
use crate::action_map::projection_identity_from_context;
use crate::agents_md::AgentsMdManager;
use crate::context::UserInstructions;
use crate::exec_policy::ExecPolicyUpdateError;
use crate::guardian::GuardianReviewSessionManager;
use crate::mcp::McpManager;
use crate::memories;
use crate::network_policy_decision::execpolicy_network_rule_amendment;
use crate::plugins::PluginsManager;
use crate::rollout::map_session_init_error;
use crate::session_startup_prewarm::SessionStartupPrewarmHandle;
use crate::shell;
use crate::shell_snapshot::ShellSnapshot;
use crate::skills_watcher::SkillsWatcher;
use crate::skills_watcher::SkillsWatcherEvent;
use crate::state::ActiveTurn;
use crate::state::MailboxDeliveryPhase;
use crate::state::PendingRequestPermissions;
use crate::state::SessionServices;
use crate::state::SessionState;
#[cfg(test)]
use crate::stream_events_utils::HandleOutputCtx;
#[cfg(test)]
use crate::stream_events_utils::handle_output_item_done;
use crate::tasks::GhostSnapshotTask;
use crate::tasks::ReviewTask;
use crate::tasks::SessionTask;
use crate::tasks::SessionTaskContext;
use crate::tools::network_approval::NetworkApprovalService;
use crate::tools::network_approval::build_blocked_request_observer;
use crate::tools::network_approval::build_network_policy_decider;
use crate::tools::sandboxing::ApprovalStore;
use crate::turn_timing::TurnTimingState;
use crate::turn_timing::record_turn_ttfm_metric;
use crate::unified_exec::UnifiedExecProcessManager;
use crate::windows_sandbox::WindowsSandboxLevelExt;
use codex_git_utils::get_git_repo_root;
use codex_mcp::compute_auth_statuses;
use codex_mcp::with_codex_apps_mcp;
use codex_otel::SessionTelemetry;
use codex_otel::THREAD_STARTED_METRIC;
use codex_otel::TelemetryAuthMode;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ApplyPatchApprovalRequestEvent;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::BackgroundEventEvent;
use codex_protocol::protocol::CodexErrorInfo;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::DeprecationNoticeEvent;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecApprovalRequestEvent;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeSnapshotUpdatedEvent;
use codex_protocol::protocol::McpServerRefreshConfig;
use codex_protocol::protocol::ModelRerouteEvent;
use codex_protocol::protocol::ModelRerouteReason;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::ModelVerificationEvent;
use codex_protocol::protocol::NetworkApprovalContext;
use codex_protocol::protocol::NonSteerableTurnKind;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::RequestUserInputEvent;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::SessionConfiguredEvent;
use codex_protocol::protocol::SessionNetworkProxyRuntime;
use codex_protocol::protocol::SkillDependencies as ProtocolSkillDependencies;
use codex_protocol::protocol::SkillErrorInfo;
use codex_protocol::protocol::SkillInterface as ProtocolSkillInterface;
use codex_protocol::protocol::SkillMetadata as ProtocolSkillMetadata;
use codex_protocol::protocol::SkillToolDependency as ProtocolSkillToolDependency;
use codex_protocol::protocol::StreamErrorEvent;
use codex_protocol::protocol::Submission;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::protocol::TaskSpaceSkillSnapshotIdentity;
use codex_protocol::protocol::TokenCountEvent;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::WarningEvent;
use codex_protocol::user_input::UserInput;
use codex_tools::ToolsConfig;
use codex_tools::ToolsConfigParams;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_readiness::Readiness;
use codex_utils_readiness::ReadinessFlag;
#[cfg(test)]
use codex_utils_stream_parser::ProposedPlanSegment;

/// The high-level interface to the Codex system.
/// It operates as a queue pair where you send submissions and receive events.
pub struct Codex {
    pub(crate) tx_sub: Sender<Submission>,
    pub(crate) rx_event: Receiver<Event>,
    // Last known status of the agent.
    pub(crate) agent_status: watch::Receiver<AgentStatus>,
    pub(crate) session: Arc<Session>,
    // Shared future for the background submission loop completion so multiple
    // callers can wait for shutdown.
    pub(crate) session_loop_termination: SessionLoopTermination,
}

pub(crate) type SessionLoopTermination = Shared<BoxFuture<'static, ()>>;

/// Wrapper returned by [`Codex::spawn`] containing the spawned [`Codex`] and
/// the unique session id.
pub struct CodexSpawnOk {
    pub codex: Codex,
    pub thread_id: ThreadId,
}

pub(crate) struct CodexSpawnArgs {
    pub(crate) config: Config,
    pub(crate) auth_manager: Arc<AuthManager>,
    pub(crate) models_manager: SharedModelsManager,
    pub(crate) environment_manager: Arc<EnvironmentManager>,
    pub(crate) skills_manager: Arc<SkillsManager>,
    pub(crate) plugins_manager: Arc<PluginsManager>,
    pub(crate) mcp_manager: Arc<McpManager>,
    pub(crate) skills_watcher: Arc<SkillsWatcher>,
    pub(crate) conversation_history: InitialHistory,
    pub(crate) session_source: SessionSource,
    pub(crate) agent_control: AgentControl,
    pub(crate) dynamic_tools: Vec<DynamicToolSpec>,
    pub(crate) persist_extended_history: bool,
    pub(crate) metrics_service_name: Option<String>,
    pub(crate) inherited_shell_snapshot: Option<Arc<ShellSnapshot>>,
    pub(crate) inherited_exec_policy: Option<Arc<ExecPolicyManager>>,
    /// Parent rollout trace used only to derive fresh spawned child traces.
    ///
    /// Root sessions and non-thread-spawn subagents pass a disabled context;
    /// `Session::new` creates the root trace itself when rollout tracing is enabled.
    pub(crate) parent_rollout_thread_trace: ThreadTraceContext,
    pub(crate) user_shell_override: Option<shell::Shell>,
    pub(crate) parent_trace: Option<W3cTraceContext>,
    pub(crate) environments: Vec<TurnEnvironmentSelection>,
    pub(crate) analytics_events_client: Option<AnalyticsEventsClient>,
    pub(crate) thread_store: Arc<dyn ThreadStore>,
}

pub(crate) const INITIAL_SUBMIT_ID: &str = "";
pub(crate) const SUBMISSION_CHANNEL_CAPACITY: usize = 512;
const CYBER_VERIFY_URL: &str = "https://chatgpt.com/cyber";
const CYBER_SAFETY_URL: &str = "https://developers.openai.com/codex/concepts/cyber-safety";

impl Codex {
    /// Spawn a new [`Codex`] and initialize the session.
    pub(crate) async fn spawn(args: CodexSpawnArgs) -> CodexResult<CodexSpawnOk> {
        let parent_trace = match args.parent_trace {
            Some(trace) => {
                if codex_otel::context_from_w3c_trace_context(&trace).is_some() {
                    Some(trace)
                } else {
                    warn!("ignoring invalid thread spawn trace carrier");
                    None
                }
            }
            None => None,
        };
        let thread_spawn_span = info_span!("thread_spawn", otel.name = "thread_spawn");
        if let Some(trace) = parent_trace.as_ref() {
            let _ = set_parent_from_w3c_trace_context(&thread_spawn_span, trace);
        }
        Self::spawn_internal(CodexSpawnArgs {
            parent_trace,
            ..args
        })
        .instrument(thread_spawn_span)
        .await
    }

    async fn spawn_internal(args: CodexSpawnArgs) -> CodexResult<CodexSpawnOk> {
        let CodexSpawnArgs {
            mut config,
            auth_manager,
            models_manager,
            environment_manager,
            skills_manager,
            plugins_manager,
            mcp_manager,
            skills_watcher,
            conversation_history,
            session_source,
            agent_control,
            dynamic_tools,
            persist_extended_history,
            metrics_service_name,
            inherited_shell_snapshot,
            user_shell_override,
            inherited_exec_policy,
            parent_rollout_thread_trace,
            parent_trace: _,
            environments,
            analytics_events_client,
            thread_store,
        } = args;
        let (tx_sub, rx_sub) = async_channel::bounded(SUBMISSION_CHANNEL_CAPACITY);
        let (tx_event, rx_event) = async_channel::unbounded();
        validate_environment_selections(environment_manager.as_ref(), &environments)?;
        let environment =
            selected_primary_environment(environment_manager.as_ref(), &environments)?;
        let fs = environment
            .as_ref()
            .map(|environment| environment.get_filesystem());
        let plugin_outcome = plugins_manager.plugins_for_config(&config).await;
        let effective_skill_roots = plugin_outcome.effective_skill_roots();
        let skills_input = skills_load_input_from_config(&config, effective_skill_roots);
        let taskspace_projection_policy = match &conversation_history {
            InitialHistory::New | InitialHistory::Cleared => config.taskspace_projection_policy,
            InitialHistory::Resumed(_) | InitialHistory::Forked(_) => {
                conversation_history.taskspace_projection_policy()
            }
        };
        let taskspace_skill_snapshot = crate::taskspace_skill::resolve_session_snapshot(
            taskspace_projection_policy,
            &conversation_history,
            &config.codex_home,
        )
        .map_err(|error| CodexErr::InvalidRequest(error.to_string()))?;
        let mut loaded_skills = skills_manager.skills_for_config(&skills_input, fs).await;
        crate::taskspace_skill::bind_catalog_snapshot(
            &mut loaded_skills,
            taskspace_skill_snapshot.as_ref(),
        )
        .map_err(|error| CodexErr::InvalidRequest(error.to_string()))?;

        for err in &loaded_skills.errors {
            error!(
                "failed to load skill {}: {}",
                err.path.display(),
                err.message
            );
        }

        if let SessionSource::SubAgent(SubAgentSource::ThreadSpawn { depth, .. }) = session_source
            && depth >= config.agent_max_depth
        {
            let _ = config.features.disable(Feature::SpawnCsv);
            let _ = config.features.disable(Feature::Collab);
        }

        let mut agents_md_warnings = Vec::new();
        let user_instructions = AgentsMdManager::new(&config)
            .user_instructions(environment.as_deref(), &mut agents_md_warnings)
            .await;
        config.startup_warnings.extend(agents_md_warnings);

        let exec_policy = if crate::guardian::is_guardian_reviewer_source(&session_source) {
            // Guardian review should rely on the built-in shell safety checks,
            // not on caller-provided exec-policy rules that could shape the
            // reviewer or silently auto-approve commands.
            Arc::new(ExecPolicyManager::default())
        } else if let Some(exec_policy) = &inherited_exec_policy {
            Arc::clone(exec_policy)
        } else {
            Arc::new(
                ExecPolicyManager::load(&config.config_layer_stack)
                    .await
                    .map_err(|err| CodexErr::Fatal(format!("failed to load rules: {err}")))?,
            )
        };

        let config = Arc::new(config);
        let refresh_strategy = match session_source {
            SessionSource::SubAgent(_) => codex_models_manager::manager::RefreshStrategy::Offline,
            _ => codex_models_manager::manager::RefreshStrategy::OnlineIfUncached,
        };
        if config.model.is_none()
            || !matches!(
                refresh_strategy,
                codex_models_manager::manager::RefreshStrategy::Offline
            )
        {
            let _ = models_manager.list_models(refresh_strategy).await;
        }
        let model = models_manager
            .get_default_model(&config.model, refresh_strategy)
            .await;

        // Resolve base instructions for the session. Priority order:
        // 1. config.base_instructions override
        // 2. conversation history => session_meta.base_instructions
        // 3. base_instructions for current model
        let model_info = models_manager
            .get_model_info(model.as_str(), &config.to_models_manager_config())
            .await;
        let base_instructions = config
            .base_instructions
            .clone()
            .or_else(|| conversation_history.get_base_instructions().map(|s| s.text))
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality));

        // Respect thread-start tools. When missing (resumed/forked threads), read from the db
        // first, then fall back to rollout-file tools.
        let persisted_tools = if dynamic_tools.is_empty() {
            let thread_id = match &conversation_history {
                InitialHistory::Resumed(resumed) => Some(resumed.conversation_id),
                InitialHistory::Forked(_) => conversation_history.forked_from_id(),
                InitialHistory::New | InitialHistory::Cleared => None,
            };
            match thread_id {
                Some(thread_id) => {
                    let state_db_ctx = state_db::get_state_db(&config).await;
                    state_db::get_dynamic_tools(state_db_ctx.as_deref(), thread_id, "codex_spawn")
                        .await
                }
                None => None,
            }
        } else {
            None
        };
        let dynamic_tools = if dynamic_tools.is_empty() {
            persisted_tools
                .or_else(|| conversation_history.get_dynamic_tools())
                .unwrap_or_default()
        } else {
            dynamic_tools
        };
        // TODO (aibrahim): Consolidate config.model and config.model_reasoning_effort into config.collaboration_mode
        // to avoid extracting these fields separately and constructing CollaborationMode here.
        let supported_reasoning_levels = model_info
            .supported_reasoning_levels
            .iter()
            .map(|preset| preset.effort)
            .collect::<Vec<_>>();
        let model_reasoning_effort = if supported_reasoning_levels.is_empty()
            && model_info.default_reasoning_level.is_none()
        {
            config.model_reasoning_effort
        } else {
            match config.model_reasoning_effort {
                Some(current) if supported_reasoning_levels.contains(&current) => Some(current),
                _ => supported_reasoning_levels
                    .get(supported_reasoning_levels.len().saturating_sub(1) / 2)
                    .copied()
                    .or(model_info.default_reasoning_level),
            }
        };
        let collaboration_mode = CollaborationMode {
            mode: ModeKind::Default,
            settings: Settings {
                model: model.clone(),
                reasoning_effort: model_reasoning_effort,
                developer_instructions: None,
            },
        };
        let account_plan_type = auth_manager
            .auth_cached()
            .and_then(|auth| auth.account_plan_type());
        let service_tier = get_service_tier(
            config.service_tier,
            config.notices.fast_default_opt_out.unwrap_or(false),
            account_plan_type,
            config.features.enabled(Feature::FastMode),
        );
        let session_configuration = SessionConfiguration {
            provider: config.model_provider.clone(),
            collaboration_mode,
            model_reasoning_summary: config.model_reasoning_summary,
            service_tier,
            taskspace_projection_policy,
            taskspace_skill_snapshot,
            developer_instructions: config.developer_instructions.clone(),
            user_instructions,
            personality: config.personality,
            base_instructions,
            compact_prompt: config.compact_prompt.clone(),
            approval_policy: config.permissions.approval_policy.clone(),
            approvals_reviewer: config.approvals_reviewer,
            sandbox_policy: config.permissions.sandbox_policy.clone(),
            file_system_sandbox_policy: config.permissions.file_system_sandbox_policy.clone(),
            network_sandbox_policy: config.permissions.network_sandbox_policy,
            windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
            cwd: config.cwd.clone(),
            codex_home: config.codex_home.clone(),
            thread_name: None,
            environments,
            original_config_do_not_use: Arc::clone(&config),
            metrics_service_name,
            app_server_client_name: None,
            app_server_client_version: None,
            session_source,
            dynamic_tools,
            persist_extended_history,
            inherited_shell_snapshot,
            user_shell_override,
        };

        // Generate a unique ID for the lifetime of this Codex session.
        let session_source_clone = session_configuration.session_source.clone();
        let (agent_status_tx, agent_status_rx) = watch::channel(AgentStatus::PendingInit);

        let session = Session::new(
            session_configuration,
            config.clone(),
            auth_manager.clone(),
            models_manager.clone(),
            exec_policy,
            tx_event.clone(),
            agent_status_tx.clone(),
            conversation_history,
            session_source_clone,
            skills_manager,
            plugins_manager,
            mcp_manager.clone(),
            skills_watcher,
            agent_control,
            environment_manager,
            analytics_events_client,
            thread_store,
            parent_rollout_thread_trace,
        )
        .await
        .map_err(|e| {
            error!("Failed to create session: {e:#}");
            map_session_init_error(&e, &config.codex_home)
        })?;
        let thread_id = session.conversation_id;

        // This task will run until Op::Shutdown is received.
        let session_for_loop = Arc::clone(&session);
        let session_loop_handle = tokio::spawn(async move {
            submission_loop(session_for_loop, config, rx_sub)
                .instrument(info_span!("session_loop", thread_id = %thread_id))
                .await;
        });
        let codex = Codex {
            tx_sub,
            rx_event,
            agent_status: agent_status_rx,
            session,
            session_loop_termination: session_loop_termination_from_handle(session_loop_handle),
        };

        Ok(CodexSpawnOk { codex, thread_id })
    }

    /// Submit the `op` wrapped in a `Submission` with a unique ID.
    pub async fn submit(&self, op: Op) -> CodexResult<String> {
        self.submit_with_trace(op, /*trace*/ None).await
    }

    pub async fn submit_with_trace(
        &self,
        op: Op,
        trace: Option<W3cTraceContext>,
    ) -> CodexResult<String> {
        let id = Uuid::now_v7().to_string();
        let sub = Submission {
            id: id.clone(),
            op,
            trace,
        };
        self.submit_with_id(sub).await?;
        Ok(id)
    }

    /// Use sparingly: prefer `submit()` so Codex is responsible for generating
    /// unique IDs for each submission.
    pub async fn submit_with_id(&self, mut sub: Submission) -> CodexResult<()> {
        if sub.trace.is_none() {
            sub.trace = current_span_w3c_trace_context();
        }
        self.tx_sub
            .send(sub)
            .await
            .map_err(|_| CodexErr::InternalAgentDied)?;
        Ok(())
    }

    /// Persist a thread-level memory mode update for the active session.
    ///
    /// This is a local-only operation that updates rollout metadata directly
    /// and does not involve the model.
    pub async fn set_thread_memory_mode(
        &self,
        mode: codex_protocol::protocol::ThreadMemoryMode,
    ) -> anyhow::Result<()> {
        handlers::persist_thread_memory_mode_update(&self.session, mode).await
    }

    pub async fn shutdown_and_wait(&self) -> CodexResult<()> {
        let session_loop_termination = self.session_loop_termination.clone();
        match self.submit(Op::Shutdown).await {
            Ok(_) => {}
            Err(CodexErr::InternalAgentDied) => {}
            Err(err) => return Err(err),
        }
        session_loop_termination.await;
        Ok(())
    }

    pub async fn next_event(&self) -> CodexResult<Event> {
        let event = self
            .rx_event
            .recv()
            .await
            .map_err(|_| CodexErr::InternalAgentDied)?;
        Ok(event)
    }

    pub async fn steer_input(
        &self,
        input: Vec<UserInput>,
        expected_turn_id: Option<&str>,
        responsesapi_client_metadata: Option<HashMap<String, String>>,
    ) -> Result<String, SteerInputError> {
        self.session
            .steer_input(input, expected_turn_id, responsesapi_client_metadata)
            .await
    }

    pub(crate) async fn set_app_server_client_info(
        &self,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
    ) -> ConstraintResult<()> {
        self.session
            .update_settings(SessionSettingsUpdate {
                app_server_client_name,
                app_server_client_version,
                ..Default::default()
            })
            .await
    }

    pub(crate) async fn agent_status(&self) -> AgentStatus {
        self.agent_status.borrow().clone()
    }

    pub(crate) async fn thread_config_snapshot(&self) -> ThreadConfigSnapshot {
        let state = self.session.state.lock().await;
        state.session_configuration.thread_config_snapshot()
    }

    pub(crate) fn state_db(&self) -> Option<state_db::StateDbHandle> {
        self.session.state_db()
    }

    pub(crate) fn enabled(&self, feature: Feature) -> bool {
        self.session.enabled(feature)
    }
}

fn get_service_tier(
    configured_service_tier: Option<ServiceTier>,
    fast_default_opt_out: bool,
    account_plan_type: Option<AccountPlanType>,
    fast_mode_enabled: bool,
) -> Option<ServiceTier> {
    if configured_service_tier.is_some() || fast_default_opt_out || !fast_mode_enabled {
        return configured_service_tier;
    }

    account_plan_type
        .is_some_and(is_enterprise_default_service_tier_plan)
        .then_some(ServiceTier::Fast)
}

fn is_enterprise_default_service_tier_plan(plan_type: AccountPlanType) -> bool {
    plan_type == AccountPlanType::Enterprise
        || plan_type.is_business_like()
        || plan_type.is_team_like()
}

#[cfg(test)]
pub(crate) fn completed_session_loop_termination() -> SessionLoopTermination {
    futures::future::ready(()).boxed().shared()
}

pub(crate) fn session_loop_termination_from_handle(
    handle: JoinHandle<()>,
) -> SessionLoopTermination {
    async move {
        let _ = handle.await;
    }
    .boxed()
    .shared()
}

async fn thread_title_from_state_db(
    state_db: Option<&state_db::StateDbHandle>,
    codex_home: &AbsolutePathBuf,
    conversation_id: ThreadId,
) -> Option<String> {
    if let Some(metadata) = state_db
        && let Some(metadata) = metadata.get_thread(conversation_id).await.ok().flatten()
    {
        let title = metadata.title.trim();
        if !title.is_empty() && metadata.first_user_message.as_deref().map(str::trim) != Some(title)
        {
            return Some(title.to_string());
        }
    }
    find_thread_name_by_id(codex_home, &conversation_id)
        .await
        .ok()
        .flatten()
}

pub(crate) struct PreparedProviderPromptItems {
    pub(crate) items: Vec<ResponseItem>,
    pub(crate) projection_identity: Option<ProviderProjectionIdentityExpectation>,
}

impl Session {
    pub(crate) async fn app_server_client_metadata(&self) -> AppServerClientMetadata {
        let state = self.state.lock().await;
        AppServerClientMetadata {
            client_name: state.session_configuration.app_server_client_name.clone(),
            client_version: state
                .session_configuration
                .app_server_client_version
                .clone(),
        }
    }

    pub(crate) async fn prepare_action_map_spawn_assignment(
        &self,
        turn_context: &TurnContext,
        task_name: &str,
        node_id: Option<&str>,
    ) -> Result<Option<ActionMapAssignment>, String> {
        let (assignment, events) = {
            let mut state = self.state.lock().await;
            state.mutate_action_map(|runtime| {
                runtime.prepare_spawn_assignment(self.conversation_id, task_name, node_id)
            })
        }
        .map_err(|err| {
            debug!(
                task_name,
                node_id,
                error = %err,
                "rejected TaskSpace spawn assignment"
            );
            err
        })?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(assignment)
    }

    pub(crate) async fn prepare_action_map_main_tool_call(
        &self,
        turn_context: &TurnContext,
        descriptor: impl Into<ToolActionDescriptor>,
    ) -> Result<(), String> {
        let result = {
            let mut state = self.state.lock().await;
            state
                .action_map_runtime
                .prepare_main_tool_call(self.conversation_id, descriptor)
        };
        match result {
            Ok(events) => {
                self.emit_action_map_events_for_turn(turn_context, events)
                    .await;
                Ok(())
            }
            Err(error) => {
                let (message, events) = error.into_parts();
                self.emit_action_map_events_for_turn(turn_context, events)
                    .await;
                Err(message)
            }
        }
    }

    pub(crate) async fn prepare_action_map_child_tool_call(
        &self,
        child_thread_id: ThreadId,
        descriptor: impl Into<ToolActionDescriptor>,
    ) -> Result<(), String> {
        let result = {
            let mut state = self.state.lock().await;
            state
                .action_map_runtime
                .prepare_child_tool_call(child_thread_id, descriptor)
        };
        match result {
            Ok(events) => {
                self.emit_action_map_events_raw(events).await;
                Ok(())
            }
            Err(error) => {
                let (message, events) = error.into_parts();
                self.emit_action_map_events_raw(events).await;
                Err(message)
            }
        }
    }

    pub(crate) async fn prepare_action_map_child_spawn(
        &self,
        child_thread_id: ThreadId,
    ) -> Result<(), String> {
        let result = {
            let state = self.state.lock().await;
            state
                .action_map_runtime
                .prepare_child_spawn(child_thread_id)
        };
        if let Err(error) = &result {
            debug!(
                child_thread_id = %child_thread_id,
                error = %error,
                "rejected TaskSpace subagent nested spawn"
            );
        }
        result
    }

    pub(crate) async fn begin_action_map_user_turn(&self, _turn_context: &TurnContext) {
        let changed = {
            let mut state = self.state.lock().await;
            state.action_map_runtime.begin_user_turn()
        };
        if changed {
            self.emit_action_map_delta().await;
        }
    }

    pub(crate) async fn record_action_map_main_tool_result(
        &self,
        turn_context: &TurnContext,
        call_id: &str,
        tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        preview: String,
    ) {
        let result = {
            let mut state = self.state.lock().await;
            let source_event_id = state.taskspace_events.event_id_for_call(call_id);
            state.action_map_runtime.record_main_tool_result_with_class(
                self.conversation_id,
                call_id,
                source_event_id.unwrap_or_default(),
                tool_name,
                action_class,
                success,
                preview,
            )
        };
        match result {
            Ok(Some((_, events))) => {
                self.emit_action_map_events_for_turn(turn_context, events)
                    .await;
            }
            Ok(None) => {}
            Err(error) => {
                warn!(
                    %error,
                    call_id,
                    tool_name,
                    "failed to record TaskSpace main tool result"
                );
            }
        }
    }

    pub(crate) async fn record_action_map_output_ref_trace_event(
        &self,
        turn_context: &TurnContext,
        kind: &str,
        call_id: Option<String>,
        artifact_ref: String,
        tags: Vec<String>,
    ) {
        let events = {
            let mut state = self.state.lock().await;
            state.action_map_runtime.record_output_ref_trace_event(
                kind,
                call_id,
                artifact_ref,
                tags,
            )
        };
        if let Some(events) = events {
            self.emit_action_map_events_for_turn(turn_context, events)
                .await;
        }
    }

    pub(crate) async fn action_map_provider_request_budget_snapshot(
        &self,
    ) -> Option<ActionMapProviderRequestBudgetSnapshot> {
        let state = self.state.lock().await;
        state.action_map_runtime.provider_request_budget_snapshot()
    }

    pub(crate) async fn record_action_map_provider_request_budget_events(
        &self,
        turn_context: &TurnContext,
        snapshot: ActionMapProviderRequestBudgetSnapshot,
        events: Vec<ProviderRequestBudgetEvent>,
    ) {
        if events.is_empty() {
            return;
        }
        for event in events.iter() {
            if Self::should_emit_provider_budget_warning(event) {
                self.send_event(
                    turn_context,
                    EventMsg::Warning(WarningEvent {
                        message: format!(
                            "TaskSpaceProviderRequestBudgetEventV1 status={} request_count={}->{} max={} state={}->{} phase={} node_role={} transport={} reason={}",
                            event.status,
                            event.request_count_before,
                            event.request_count_after,
                            event.max_requests,
                            event.budget_state_before,
                            event.budget_state_after,
                            event.request_phase.as_deref().unwrap_or("unknown"),
                            snapshot.node_role.as_deref().unwrap_or("unknown"),
                            event.transport,
                            event.budget_transition_reason,
                        ),
                    }),
                )
                .await;
            }
        }
        let inputs = events
            .into_iter()
            .map(|event| ActionMapProviderRequestBudgetEventInput {
                request_id: event.request_id,
                logical_request_id: event.logical_request_id,
                parent_request_id: event.parent_request_id,
                attempt_seq: event.attempt_seq,
                transport: event.transport,
                status: event.status,
                request_count_before: event.request_count_before,
                request_count_after: event.request_count_after,
                max_requests: event.max_requests,
                budget_state_before: event.budget_state_before,
                budget_state_after: event.budget_state_after,
                budget_transition_reason: event.budget_transition_reason,
                started_at_ms: event.started_at_ms,
                completed_at_ms: event.completed_at_ms,
                latency_ms: event.latency_ms,
                input_tokens: event.input_tokens,
                cached_input_tokens: event.cached_input_tokens,
                output_tokens: event.output_tokens,
                reasoning_output_tokens: event.reasoning_output_tokens,
                total_tokens: event.total_tokens,
                provider_payload_sha256: event.provider_payload_sha256,
                provider_payload_bytes: event.provider_payload_bytes,
                provider_wire_api: event.provider_wire_api,
                tools_count: event.tools_count,
                tools_present: event.tools_present,
                request_shape_classifier: event.request_shape_classifier,
                messages_hash: event.messages_hash,
                stable_prefix_hash: event.stable_prefix_hash,
                dynamic_suffix_hash: event.dynamic_suffix_hash,
                exact_payload_scan_passed: event.exact_payload_scan_passed,
                active_projection_present: event.active_projection_present,
                active_projection_count: event.active_projection_count,
                large_raw_output_tokens: event.large_raw_output_tokens,
                protected_items_present: event.protected_items_present,
                replacement_confirmed: event.replacement_confirmed,
                exact_payload_scan: event.exact_payload_scan.map(|scan| {
                    ActionMapExactPayloadScanEventInput {
                        scan_event_id: scan.scan_event_id,
                        request_id: scan.request_id,
                        provider_payload_sha256: scan.provider_payload_sha256,
                        scanner_version: scan.scanner_version,
                        matcher_version: scan.matcher_version,
                        checked_byte_ranges: scan.checked_byte_ranges,
                        negative_checks_performed: scan.negative_checks_performed,
                        projection_required: scan.projection_required,
                        active_projection_present: scan.active_projection_present,
                        active_projection_count: scan.active_projection_count,
                        projection_is_message_tail: scan.projection_is_message_tail,
                        large_raw_output_tokens: scan.large_raw_output_tokens,
                        runtime_boundary_forbidden_markers: scan.runtime_boundary_forbidden_markers,
                        protected_items_present: scan.protected_items_present,
                        projection_kind: scan.projection_kind,
                        projection_map_id_sha256: scan.projection_map_id_sha256,
                        projection_revision: scan.projection_revision,
                        projection_canonical_sha256: scan.projection_canonical_sha256,
                        projection_sha256: scan.projection_sha256,
                        projection_policy: scan.projection_policy,
                        expected_projection_kind: scan.expected_projection_kind,
                        expected_projection_map_id_sha256: scan.expected_projection_map_id_sha256,
                        expected_projection_revision: scan.expected_projection_revision,
                        expected_projection_canonical_sha256: scan
                            .expected_projection_canonical_sha256,
                        expected_projection_sha256: scan.expected_projection_sha256,
                        projection_identity_confirmed: scan.projection_identity_confirmed,
                        replacement_confirmed: scan.replacement_confirmed,
                        passed: scan.passed,
                        failure_reasons: scan.failure_reasons,
                    }
                }),
                task_id: event.task_id,
                map_id: event.map_id,
                node_id: event.node_id,
                request_phase: event.request_phase,
            })
            .collect::<Vec<_>>();
        let runtime_events = {
            let mut state = self.state.lock().await;
            state
                .action_map_runtime
                .record_provider_request_budget_events(&snapshot, inputs)
        };
        if let Some(runtime_events) = runtime_events {
            self.emit_action_map_events_for_turn(turn_context, runtime_events)
                .await;
        }
    }

    pub(crate) async fn record_action_map_provider_response_actionability(
        &self,
        turn_context: &TurnContext,
        snapshot: ActionMapProviderRequestBudgetSnapshot,
        input: ActionMapProviderResponseActionabilityInput,
    ) {
        if Self::should_emit_provider_response_actionability_warning(&snapshot, &input) {
            self.send_event(
                turn_context,
                EventMsg::Warning(WarningEvent {
                    message: format!(
                        "TaskSpaceProviderResponseActionabilityV1 actionability={} recovery_action={} request_count={}/{} phase={} node_role={} assistant_message_present={} saw_actionable_output={} end_turn={} preview={}",
                        input.response_actionability,
                        input.recovery_action,
                        snapshot.request_count,
                        snapshot.max_requests,
                        snapshot.request_phase.as_deref().unwrap_or("unknown"),
                        snapshot.node_role.as_deref().unwrap_or("unknown"),
                        input.assistant_message_present,
                        input.saw_actionable_output,
                        input.end_turn
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        input.last_agent_message_preview.as_deref().unwrap_or(""),
                    ),
                }),
            )
            .await;
        }
        let runtime_events = {
            let mut state = self.state.lock().await;
            state
                .action_map_runtime
                .record_provider_response_actionability(&snapshot, input)
        };
        if let Some(runtime_events) = runtime_events {
            self.emit_action_map_events_for_turn(turn_context, runtime_events)
                .await;
        }
    }

    pub(crate) async fn record_action_map_child_tool_result(
        &self,
        child_thread_id: ThreadId,
        call_id: &str,
        tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        preview: String,
    ) -> bool {
        let result = {
            let mut state = self.state.lock().await;
            state
                .action_map_runtime
                .record_child_tool_result_with_class(
                    child_thread_id,
                    call_id,
                    tool_name,
                    action_class,
                    success,
                    preview,
                )
        };
        match result {
            Ok(Some((_, events))) => {
                self.emit_action_map_events_raw(events).await;
                true
            }
            Ok(None) => false,
            Err(error) => {
                warn!(
                    %error,
                    child_thread_id = %child_thread_id,
                    call_id,
                    tool_name,
                    "failed to record TaskSpace subagent tool result"
                );
                false
            }
        }
    }

    pub(crate) async fn initialize_action_map_for_main(
        &self,
        turn_context: &TurnContext,
        input: crate::action_map::ActionMapInitializeInput,
    ) -> Result<crate::action_map::ActionMapInitializeOutcome, String> {
        let (outcome, events) = {
            let mut state = self.state.lock().await;
            state.mutate_action_map(|runtime| {
                runtime.initialize_map_for_main(self.conversation_id, input)
            })
        }?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(outcome)
    }

    pub(crate) async fn action_map_control_state(
        &self,
        map_id_hint: Option<&str>,
    ) -> Option<crate::action_map::ActionMapControlState> {
        let state = self.state.lock().await;
        state.action_map_runtime.control_state(map_id_hint)
    }

    pub(crate) async fn mutate_action_map_graph(
        &self,
        turn_context: &TurnContext,
        input: crate::action_map::ActionMapGraphMutationInput,
    ) -> Result<crate::action_map::ActionMapGraphMutationOutcome, String> {
        let (outcome, events) = {
            let mut state = self.state.lock().await;
            state.mutate_action_map(|runtime| {
                runtime.mutate_graph_for_main(self.conversation_id, input)
            })
        }?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(outcome)
    }

    pub(crate) async fn transition_action_map_node(
        &self,
        turn_context: &TurnContext,
        expected_revision: u64,
        node_id: String,
        transition: crate::action_map::NodeTransition,
        source_event_ref: String,
    ) -> Result<crate::action_map::ActionMapTransitionOutcome, String> {
        let (outcome, events) = {
            let mut state = self.state.lock().await;
            state.mutate_action_map(|runtime| {
                runtime.transition_node_for_main(
                    self.conversation_id,
                    expected_revision,
                    node_id,
                    transition,
                    source_event_ref,
                )
            })
        }?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(outcome)
    }

    pub(crate) async fn complete_then_bind_action_map_node(
        &self,
        turn_context: &TurnContext,
        expected_revision: u64,
        current_node_id: String,
        next_node_id: String,
        source_event_ref: String,
    ) -> Result<crate::action_map::ActionMapCompleteHandoffOutcome, String> {
        let (outcome, events) = {
            let mut state = self.state.lock().await;
            state.mutate_action_map(|runtime| {
                runtime.complete_then_bind_for_main(
                    self.conversation_id,
                    expected_revision,
                    current_node_id,
                    next_node_id,
                    source_event_ref,
                )
            })
        }?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(outcome)
    }

    pub(crate) async fn expand_action_map_node_details(
        &self,
        turn_context: &TurnContext,
        node_ids: Vec<String>,
        call_id: String,
        source_event_id: String,
    ) -> Result<Vec<crate::action_map::ActionMapNodeDetailExpansionOutcome>, String> {
        let (outcomes, events) = {
            let mut state = self.state.lock().await;
            state.action_map_runtime.expand_node_details_for_main(
                self.conversation_id,
                node_ids,
                call_id,
                source_event_id,
            )
        }?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(outcomes)
    }

    #[cfg(test)]
    pub(crate) async fn set_action_map_mode_for_test(
        &self,
        mode: codex_protocol::protocol::MapRuntimeMode,
    ) {
        let mut state = self.state.lock().await;
        state
            .action_map_runtime
            .set_mode_for_session(mode, self.conversation_id);
    }

    pub(crate) async fn attach_action_map_assignment(
        &self,
        turn_context: &TurnContext,
        lease_id: &str,
        thread_id: ThreadId,
        agent_path: Option<String>,
    ) {
        let event = {
            let mut state = self.state.lock().await;
            state
                .action_map_runtime
                .attach_agent_to_lease(lease_id, thread_id, agent_path)
        };
        self.emit_action_map_events_for_turn(turn_context, event.into_iter().collect())
            .await;
    }

    pub(crate) async fn release_action_map_assignment(
        &self,
        turn_context: &TurnContext,
        lease_id: &str,
        reason: &str,
    ) {
        let events = {
            let mut state = self.state.lock().await;
            match state
                .mutate_action_map(|runtime| Ok(((), runtime.release_lease(lease_id, reason))))
            {
                Ok(((), events)) => events,
                Err(error) => {
                    tracing::error!(
                        target: "codex_core::taskspace",
                        lease_id,
                        %error,
                        "rolled back TaskSpace lease release after projection capture failure"
                    );
                    Vec::new()
                }
            }
        };
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
    }

    pub(crate) async fn release_action_map_assignment_for_thread(
        &self,
        turn_context: &TurnContext,
        thread_id: ThreadId,
        reason: &str,
    ) {
        let events = {
            let mut state = self.state.lock().await;
            match state.mutate_action_map(|runtime| {
                Ok((
                    (),
                    runtime
                        .release_lease_for_thread(thread_id, reason)
                        .map(|(_, events)| events)
                        .unwrap_or_default(),
                ))
            }) {
                Ok(((), events)) => events,
                Err(error) => {
                    tracing::error!(
                        target: "codex_core::taskspace",
                        %thread_id,
                        %error,
                        "rolled back TaskSpace thread lease release after projection capture failure"
                    );
                    Vec::new()
                }
            }
        };
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
    }

    pub(crate) async fn record_action_map_child_result(
        &self,
        child_thread_id: ThreadId,
        status: &AgentStatus,
    ) -> Option<String> {
        let result = {
            let mut state = self.state.lock().await;
            match state.mutate_action_map(|runtime| {
                Ok(match runtime.record_child_result(child_thread_id, status) {
                    Some((result_id, events)) => (Some(result_id), events),
                    None => (None, Vec::new()),
                })
            }) {
                Ok((result_id, events)) => result_id.map(|result_id| (result_id, events)),
                Err(error) => {
                    tracing::error!(
                        target: "codex_core::taskspace",
                        %child_thread_id,
                        %error,
                        "rolled back TaskSpace child result after projection capture failure"
                    );
                    None
                }
            }
        };
        let (result_id, events) = result?;
        self.emit_action_map_events_raw(events).await;
        Some(result_id)
    }

    pub(crate) async fn record_final_action_map_child_result_if_needed(
        &self,
        child_thread_id: ThreadId,
    ) -> Option<String> {
        let status = self
            .services
            .agent_control
            .get_status(child_thread_id)
            .await;
        if !is_final(&status) {
            return None;
        }

        let result_id = self
            .record_action_map_child_result(child_thread_id, &status)
            .await;
        debug!(
            child_thread_id = %child_thread_id,
            ?status,
            result_id = result_id.as_deref().unwrap_or("none"),
            "checked final TaskSpace child status after lease attach"
        );
        result_id
    }

    pub(crate) async fn request_action_map_reborn(&self, turn_context: &TurnContext) {
        let events = {
            let mut state = self.state.lock().await;
            state.action_map_runtime.request_reborn()
        };
        if events.is_empty() {
            self.emit_action_map_checkpoint_for_turn(turn_context, "reborn_requested")
                .await;
        } else {
            self.emit_action_map_events_for_turn(turn_context, events)
                .await;
        }
    }

    pub(crate) async fn request_action_map_timeout_summaries(
        &self,
        turn_context: &TurnContext,
    ) -> usize {
        let (targets, parent_agent_path) = {
            let state = self.state.lock().await;
            let parent_agent_path = state
                .session_configuration
                .session_source
                .get_agent_path()
                .unwrap_or_else(AgentPath::root);
            (
                state.action_map_runtime.active_timeout_targets(),
                parent_agent_path,
            )
        };

        let mut requested = 0usize;
        let mut events = Vec::new();
        for target in targets {
            let Some(agent_path) = target.agent_path.clone() else {
                continue;
            };
            let _ = self
                .services
                .agent_control
                .interrupt_agent(target.thread_id)
                .await;
            let message = format!(
                "TaskSpace wait timeout reached for task path `{}` node `{}` lease `{}`. Stop the current work and return a concise current-progress summary as the node result. Do not start unrelated work.",
                target.map_id, target.node_id, target.lease_id
            );
            let communication = InterAgentCommunication::new(
                parent_agent_path.clone(),
                agent_path,
                Vec::new(),
                message,
                /*trigger_turn*/ true,
            );
            if self
                .services
                .agent_control
                .send_inter_agent_communication(target.thread_id, communication)
                .await
                .is_ok()
            {
                requested += 1;
                if let Some(event) = ActionMapRuntimeState::timeout_summary_requested_event(&target)
                {
                    events.push(event);
                }
            }
        }
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        requested
    }

    async fn emit_action_map_events_for_turn(
        &self,
        turn_context: &TurnContext,
        events: Vec<MapRuntimeEvent>,
    ) {
        let starts_map_lifecycle = events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::GraphRevisionCommitted(event)
                    if event.operation == "initialize_map" && event.revision == 1
            )
        });
        for event in events {
            self.send_event(turn_context, EventMsg::MapRuntime(event))
                .await;
        }
        if starts_map_lifecycle {
            self.emit_action_map_checkpoint_for_turn(turn_context, "map_lifecycle")
                .await;
        } else {
            self.emit_action_map_delta().await;
        }
    }

    fn should_emit_provider_budget_warning(event: &ProviderRequestBudgetEvent) -> bool {
        matches!(
            event.status.as_str(),
            "blocked" | "failed" | "response_failed" | "cancelled"
        ) || event.budget_state_before != "normal"
            || event.budget_state_after != "normal"
            || event.request_count_after.saturating_mul(2) >= event.max_requests
    }

    fn should_emit_provider_response_actionability_warning(
        snapshot: &ActionMapProviderRequestBudgetSnapshot,
        input: &ActionMapProviderResponseActionabilityInput,
    ) -> bool {
        input.recovery_action != "none"
            || input.response_actionability != "turn_complete"
            || snapshot.request_count.saturating_mul(2) >= snapshot.max_requests
    }

    pub(crate) async fn emit_action_map_checkpoint_for_turn(
        &self,
        turn_context: &TurnContext,
        reason: &str,
    ) {
        let (snapshot, snapshot_sha256, checkpoint_id) = {
            let mut state = self.state.lock().await;
            let snapshot = state.action_map_runtime.snapshot();
            let snapshot_sha256 = snapshot_sha256(&snapshot)
                .expect("ActionMapSnapshot must remain serializable for checkpoint persistence");
            let checkpoint_id = format!("map-checkpoint-{}", &snapshot_sha256[..16]);
            state.action_map_checkpoint.install(
                checkpoint_id.clone(),
                snapshot_sha256.clone(),
                snapshot.clone(),
            );
            (snapshot, snapshot_sha256, checkpoint_id)
        };
        let snapshot_bytes = serde_json::to_vec(&snapshot)
            .expect("ActionMapSnapshot must remain serializable for checkpoint persistence");
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.checkpoint_written",
            checkpoint_id,
            reason,
            snapshot_sha256,
            snapshot_bytes = snapshot_bytes.len(),
            map_present = snapshot.map.is_some(),
            map_revision = snapshot.map.as_ref().map(|map| map.revision),
            node_count = snapshot.map.as_ref().map_or(0, |map| map.nodes.len()),
            edge_count = snapshot.map.as_ref().map_or(0, |map| map.edges.len()),
            "TaskSpace map checkpoint persisted"
        );
        self.send_event(
            turn_context,
            EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
                MapRuntimeSnapshotUpdatedEvent {
                    checkpoint_id,
                    reason: reason.to_string(),
                    snapshot_sha256,
                    snapshot,
                },
            )),
        )
        .await;
    }

    async fn emit_action_map_delta(&self) {
        let delta = {
            let mut state = self.state.lock().await;
            let snapshot = state.action_map_runtime.snapshot();
            build_snapshot_delta(&mut state.action_map_checkpoint, &snapshot)
        };
        match delta {
            Ok(Some(delta)) => {
                let patch_bytes = serde_json::to_vec(&delta.patch)
                    .expect("snapshot delta patch must remain serializable")
                    .len();
                tracing::debug!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.snapshot_delta_written",
                    base_checkpoint_id = delta.base_checkpoint_id,
                    sequence = delta.sequence,
                    patch_bytes,
                    snapshot_sha256 = delta.snapshot_sha256,
                    "TaskSpace map snapshot delta persisted"
                );
                self.send_event_raw(Event {
                    id: self.next_internal_sub_id(),
                    msg: EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(delta)),
                })
                .await;
            }
            Ok(None) => {}
            Err(error) => panic!("failed to persist TaskSpace map snapshot delta: {error}"),
        }
    }

    async fn emit_action_map_checkpoint_raw(&self, reason: &str) {
        let turn_context = self.new_default_turn().await;
        self.emit_action_map_checkpoint_for_turn(&turn_context, reason)
            .await;
    }

    async fn emit_action_map_events_raw(&self, events: Vec<MapRuntimeEvent>) {
        for event in events {
            self.send_event_raw(Event {
                id: self.next_internal_sub_id(),
                msg: EventMsg::MapRuntime(event),
            })
            .await;
        }
        self.emit_action_map_delta().await;
    }

    fn managed_network_proxy_active_for_sandbox_policy(sandbox_policy: &SandboxPolicy) -> bool {
        !matches!(sandbox_policy, SandboxPolicy::DangerFullAccess)
    }

    /// Builds the `x-codex-beta-features` header value for this session.
    ///
    /// `ModelClient` is session-scoped and intentionally does not depend on the full `Config`, so
    /// we precompute the comma-separated list of enabled experimental feature keys at session
    /// creation time and thread it into the client.
    fn build_model_client_beta_features_header(config: &Config) -> Option<String> {
        let beta_features_header = FEATURES
            .iter()
            .filter_map(|spec| {
                if spec.stage.experimental_menu_description().is_some()
                    && config.features.enabled(spec.id)
                {
                    Some(spec.key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(",");

        if beta_features_header.is_empty() {
            None
        } else {
            Some(beta_features_header)
        }
    }

    async fn start_managed_network_proxy(
        spec: &crate::config::NetworkProxySpec,
        exec_policy: &codex_execpolicy::Policy,
        sandbox_policy: &SandboxPolicy,
        network_policy_decider: Option<Arc<dyn codex_network_proxy::NetworkPolicyDecider>>,
        blocked_request_observer: Option<Arc<dyn codex_network_proxy::BlockedRequestObserver>>,
        managed_network_requirements_enabled: bool,
        audit_metadata: NetworkProxyAuditMetadata,
    ) -> anyhow::Result<(StartedNetworkProxy, SessionNetworkProxyRuntime)> {
        let spec = spec
            .with_exec_policy_network_rules(exec_policy)
            .map_err(|err| {
                tracing::warn!(
                    "failed to apply execpolicy network rules to managed proxy; continuing with configured network policy: {err}"
                );
                err
            })
            .unwrap_or_else(|_| spec.clone());
        let network_proxy = spec
            .start_proxy(
                sandbox_policy,
                network_policy_decider,
                blocked_request_observer,
                managed_network_requirements_enabled,
                audit_metadata,
            )
            .await
            .map_err(|err| anyhow::anyhow!("failed to start managed network proxy: {err}"))?;
        let session_network_proxy = {
            let proxy = network_proxy.proxy();
            SessionNetworkProxyRuntime {
                http_addr: proxy.http_addr().to_string(),
                socks_addr: proxy.socks_addr().to_string(),
            }
        };
        Ok((network_proxy, session_network_proxy))
    }

    async fn refresh_managed_network_proxy_for_current_sandbox_policy(&self) {
        let Some(started_proxy) = self.services.network_proxy.as_ref() else {
            return;
        };
        let Ok(_refresh_guard) = self.managed_network_proxy_refresh_lock.acquire().await else {
            error!("managed network proxy refresh semaphore closed");
            return;
        };
        let session_configuration = {
            let state = self.state.lock().await;
            state.session_configuration.clone()
        };
        let Some(spec) = session_configuration
            .original_config_do_not_use
            .permissions
            .network
            .as_ref()
        else {
            return;
        };

        let spec = match spec
            .recompute_for_sandbox_policy(session_configuration.sandbox_policy.get())
        {
            Ok(spec) => spec,
            Err(err) => {
                warn!("failed to rebuild managed network proxy policy for sandbox change: {err}");
                return;
            }
        };
        let current_exec_policy = self.services.exec_policy.current();
        let spec = match spec.with_exec_policy_network_rules(current_exec_policy.as_ref()) {
            Ok(spec) => spec,
            Err(err) => {
                warn!(
                    "failed to apply execpolicy network rules while refreshing managed network proxy: {err}"
                );
                spec
            }
        };
        if let Err(err) = spec.apply_to_started_proxy(started_proxy).await {
            warn!("failed to refresh managed network proxy for sandbox change: {err}");
        }
    }

    pub(crate) async fn codex_home(&self) -> AbsolutePathBuf {
        let state = self.state.lock().await;
        state.session_configuration.codex_home().clone()
    }

    pub(crate) fn subscribe_out_of_band_elicitation_pause_state(&self) -> watch::Receiver<bool> {
        self.out_of_band_elicitation_paused.subscribe()
    }

    pub(crate) fn set_out_of_band_elicitation_pause_state(&self, paused: bool) {
        self.out_of_band_elicitation_paused.send_replace(paused);
    }

    fn start_skills_watcher_listener(self: &Arc<Self>) {
        let mut rx = self.services.skills_watcher.subscribe();
        let weak_sess = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(SkillsWatcherEvent::SkillsChanged { .. }) => {
                        let Some(sess) = weak_sess.upgrade() else {
                            break;
                        };
                        let event = Event {
                            id: sess.next_internal_sub_id(),
                            msg: EventMsg::SkillsUpdateAvailable,
                        };
                        sess.send_event_raw(event).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });
    }

    pub(crate) fn get_tx_event(&self) -> Sender<Event> {
        self.tx_event.clone()
    }

    pub(crate) fn state_db(&self) -> Option<state_db::StateDbHandle> {
        self.services.state_db.clone()
    }

    pub(crate) fn live_thread_for_persistence(
        &self,
        operation: &str,
    ) -> anyhow::Result<&LiveThread> {
        self.live_thread()
            .ok_or_else(|| anyhow::anyhow!("Session persistence is disabled; cannot {operation}."))
    }

    pub(crate) fn live_thread(&self) -> Option<&LiveThread> {
        self.services.live_thread.as_ref()
    }

    /// Flush rollout writes and return the final durability-barrier result.
    pub(crate) async fn flush_rollout(&self) -> std::io::Result<()> {
        if let Some(live_thread) = self.live_thread() {
            live_thread.flush().await.map_err(std::io::Error::other)
        } else {
            Ok(())
        }
    }

    pub(crate) async fn try_ensure_rollout_materialized(&self) -> std::io::Result<()> {
        if let Some(live_thread) = self.live_thread() {
            live_thread.persist().await.map_err(std::io::Error::other)?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_rollout_materialized(&self) {
        if let Err(e) = self.try_ensure_rollout_materialized().await {
            warn!("failed to materialize thread persistence: {e}");
        }
    }

    fn next_internal_sub_id(&self) -> String {
        let id = self
            .next_internal_sub_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("auto-compact-{id}")
    }

    pub(crate) async fn route_realtime_text_input(self: &Arc<Self>, text: String) {
        handlers::user_input_or_turn_inner(
            self,
            self.next_internal_sub_id(),
            Op::UserInput {
                environments: None,
                items: vec![UserInput::Text {
                    text,
                    text_elements: Vec::new(),
                }],
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
            },
            /*mirror_user_text_to_realtime*/ None,
        )
        .await;
    }

    pub(crate) async fn get_total_token_usage(&self) -> i64 {
        let state = self.state.lock().await;
        state.get_total_token_usage(state.server_reasoning_included())
    }

    pub(crate) async fn get_total_token_usage_breakdown(&self) -> TotalTokenUsageBreakdown {
        let state = self.state.lock().await;
        state.clone_history().get_total_token_usage_breakdown()
    }

    pub(crate) async fn total_token_usage(&self) -> Option<TokenUsage> {
        let state = self.state.lock().await;
        state.token_info().map(|info| info.total_token_usage)
    }

    /// Returns the complete token usage snapshot currently cached for this session.
    ///
    /// Resume and fork reconstruction seed this state from the last persisted rollout
    /// `TokenCount` event. Callers that need to replay restored usage to a client
    /// should use this accessor instead of `total_token_usage`, because the app-server
    /// notification includes both total and last-turn usage.
    pub(crate) async fn token_usage_info(&self) -> Option<TokenUsageInfo> {
        let state = self.state.lock().await;
        state.token_info()
    }

    pub(crate) async fn get_estimated_token_count(&self) -> Option<i64> {
        let state = self.state.lock().await;
        let base_instructions = crate::context::resolve_base_instructions(
            &state.session_configuration.base_instructions,
            state.action_map_runtime.mode(),
        );
        state
            .clone_history()
            .estimate_token_count_with_base_instructions(&base_instructions.instructions)
    }

    pub(crate) async fn get_base_instructions(&self) -> BaseInstructions {
        self.get_resolved_base_instructions().await.instructions
    }

    pub(crate) async fn get_resolved_base_instructions(
        &self,
    ) -> crate::context::ResolvedBaseInstructions {
        let state = self.state.lock().await;
        crate::context::resolve_base_instructions(
            &state.session_configuration.base_instructions,
            state.action_map_runtime.mode(),
        )
    }

    pub(crate) async fn get_standard_base_instructions(&self) -> BaseInstructions {
        let state = self.state.lock().await;
        BaseInstructions {
            text: state.session_configuration.base_instructions.clone(),
        }
    }

    // Merges connector IDs into the session-level explicit connector selection.
    pub(crate) async fn merge_connector_selection(
        &self,
        connector_ids: HashSet<String>,
    ) -> HashSet<String> {
        let mut state = self.state.lock().await;
        state.merge_connector_selection(connector_ids)
    }

    // Returns the connector IDs currently selected for this session.
    pub(crate) async fn get_connector_selection(&self) -> HashSet<String> {
        let state = self.state.lock().await;
        state.get_connector_selection()
    }

    // Clears connector IDs that were accumulated for explicit selection.
    pub(crate) async fn clear_connector_selection(&self) {
        let mut state = self.state.lock().await;
        state.clear_connector_selection();
    }

    async fn record_initial_history(
        &self,
        conversation_history: InitialHistory,
    ) -> Result<(), rollout_reconstruction::RolloutReconstructionError> {
        let turn_context = self.new_default_turn().await;
        let is_subagent = {
            let state = self.state.lock().await;
            matches!(
                state.session_configuration.session_source,
                SessionSource::SubAgent(_)
            )
        };
        let has_prior_user_turns = initial_history_has_prior_user_turns(&conversation_history);
        match conversation_history {
            InitialHistory::New | InitialHistory::Cleared => {
                // Defer initial context insertion until the first real turn starts so
                // turn/start overrides can be merged before we write model-visible context.
                self.set_previous_turn_settings(/*previous_turn_settings*/ None)
                    .await;
            }
            InitialHistory::Resumed(resumed_history) => {
                let rollout_items = resumed_history.history;
                let previous_turn_settings = self
                    .apply_rollout_reconstruction(&turn_context, &rollout_items, false, None)
                    .await?;

                // If resuming, warn when the last recorded model differs from the current one.
                let curr: &str = turn_context.model_info.slug.as_str();
                if let Some(prev) = previous_turn_settings
                    .as_ref()
                    .map(|settings| settings.model.as_str())
                    .filter(|model| *model != curr)
                {
                    warn!("resuming session with different model: previous={prev}, current={curr}");
                    self.send_event(
                        &turn_context,
                        EventMsg::Warning(WarningEvent {
                            message: format!(
                                "This session was recorded with model `{prev}` but is resuming with `{curr}`. \
                         Consider switching back to `{prev}` as it may affect Codex performance."
                            ),
                        }),
                    )
                    .await;
                }

                // Seed usage info from the recorded rollout so UIs can show token counts
                // immediately on resume/fork.
                if let Some(info) = Self::last_token_info_from_rollout(&rollout_items) {
                    let mut state = self.state.lock().await;
                    state.set_token_info(Some(info));
                }
                // Defer seeding the session's initial context until the first turn starts so
                // turn/start overrides can be merged before we write to the rollout.
                if !is_subagent {
                    let _ = self.flush_rollout().await;
                }
            }
            InitialHistory::Forked(rollout_items) => {
                self.apply_rollout_reconstruction(
                    &turn_context,
                    &rollout_items,
                    is_subagent,
                    (!is_subagent).then_some(self.conversation_id),
                )
                .await?;

                // Seed usage info from the recorded rollout so UIs can show token counts
                // immediately on resume/fork.
                if let Some(info) = Self::last_token_info_from_rollout(&rollout_items) {
                    let mut state = self.state.lock().await;
                    state.set_token_info(Some(info));
                }

                // If persisting, persist all rollout items as-is (the store filters).
                if !rollout_items.is_empty() {
                    self.persist_rollout_items(&rollout_items).await;
                }
                if !is_subagent {
                    self.emit_action_map_checkpoint_for_turn(&turn_context, "resume")
                        .await;
                }

                // Forked threads should remain file-backed immediately after startup.
                self.ensure_rollout_materialized().await;

                // Flush after seeding history and any persisted rollout copy.
                if !is_subagent {
                    let _ = self.flush_rollout().await;
                }
            }
        }
        {
            let mut state = self.state.lock().await;
            state.set_next_turn_is_first(!has_prior_user_turns);
        }
        Ok(())
    }

    async fn apply_rollout_reconstruction(
        &self,
        turn_context: &TurnContext,
        rollout_items: &[RolloutItem],
        linearize_taskspace_for_subagent: bool,
        fork_owner_session_id: Option<ThreadId>,
    ) -> Result<Option<PreviousTurnSettings>, rollout_reconstruction::RolloutReconstructionError>
    {
        let reconstructed_rollout = self
            .try_reconstruct_history_from_rollout(turn_context, rollout_items)
            .await?;
        if !linearize_taskspace_for_subagent
            && reconstructed_rollout.map_runtime_mode == MapRuntimeMode::Experiment
        {
            let policy = {
                let state = self.state.lock().await;
                state.session_configuration.taskspace_projection_policy
            };
            if policy.is_none() {
                return Err(rollout_reconstruction::RolloutReconstructionError {
                    phase: "taskspace_projection_policy_restore",
                    message: "TaskSpace rollout has no persisted projection policy; R7 does not migrate R6 sessions"
                        .to_string(),
                });
            }
        }
        let previous_turn_settings = reconstructed_rollout.previous_turn_settings.clone();
        {
            let mut state = self.state.lock().await;
            if linearize_taskspace_for_subagent {
                state.restore_subagent_fork_context(
                    reconstructed_rollout.history,
                    reconstructed_rollout.taskspace_events,
                    reconstructed_rollout.reference_context_item,
                );
            } else {
                state.restore_context(
                    reconstructed_rollout.history,
                    reconstructed_rollout.taskspace_events,
                    reconstructed_rollout.reference_context_item,
                );
                if let Some(snapshot) = reconstructed_rollout.map_runtime_snapshot {
                    state
                        .action_map_runtime
                        .restore_snapshot(snapshot)
                        .map_err(
                            |message| rollout_reconstruction::RolloutReconstructionError {
                                phase: "taskspace_snapshot_restore",
                                message,
                            },
                        )?;
                    state.action_map_checkpoint = reconstructed_rollout.map_runtime_checkpoint;
                } else {
                    state
                        .action_map_runtime
                        .restore_mode(reconstructed_rollout.map_runtime_mode);
                }
                if let Some(owner_session_id) = fork_owner_session_id {
                    let released_child_leases =
                        state.action_map_runtime.rebind_after_fork(owner_session_id);
                    tracing::info!(
                        %owner_session_id,
                        released_child_leases,
                        "rebound TaskSpace runtime after thread fork"
                    );
                }
            }
        }
        self.set_previous_turn_settings(previous_turn_settings.clone())
            .await;
        Ok(previous_turn_settings)
    }

    fn last_token_info_from_rollout(rollout_items: &[RolloutItem]) -> Option<TokenUsageInfo> {
        rollout_items.iter().rev().find_map(|item| match item {
            RolloutItem::EventMsg(EventMsg::TokenCount(ev)) => ev.info.clone(),
            _ => None,
        })
    }

    async fn previous_turn_settings(&self) -> Option<PreviousTurnSettings> {
        let state = self.state.lock().await;
        state.previous_turn_settings()
    }

    pub(crate) async fn set_previous_turn_settings(
        &self,
        previous_turn_settings: Option<PreviousTurnSettings>,
    ) {
        let mut state = self.state.lock().await;
        state.set_previous_turn_settings(previous_turn_settings);
    }

    fn maybe_refresh_shell_snapshot_for_cwd(
        &self,
        previous_cwd: &AbsolutePathBuf,
        next_cwd: &AbsolutePathBuf,
        codex_home: &AbsolutePathBuf,
        session_source: &SessionSource,
    ) {
        if previous_cwd == next_cwd {
            return;
        }

        if !self.features.enabled(Feature::ShellSnapshot) {
            return;
        }

        if matches!(
            session_source,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        ) {
            return;
        }

        ShellSnapshot::refresh_snapshot(
            codex_home.clone(),
            self.conversation_id,
            next_cwd.clone(),
            self.services.user_shell.as_ref().clone(),
            self.services.shell_snapshot_tx.clone(),
            self.services.session_telemetry.clone(),
        );
    }

    pub(crate) async fn update_settings(
        &self,
        updates: SessionSettingsUpdate,
    ) -> ConstraintResult<()> {
        let (previous_cwd, sandbox_policy_changed, next_cwd, codex_home, session_source) = {
            let mut state = self.state.lock().await;
            let updated = match state.session_configuration.apply(&updates) {
                Ok(updated) => updated,
                Err(err) => {
                    warn!("rejected session settings update: {err}");
                    return Err(err);
                }
            };

            let previous_cwd = state.session_configuration.cwd.clone();
            let sandbox_policy_changed =
                state.session_configuration.sandbox_policy != updated.sandbox_policy;
            let next_cwd = updated.cwd.clone();
            let codex_home = updated.codex_home.clone();
            let session_source = updated.session_source.clone();
            state.session_configuration = updated;
            (
                previous_cwd,
                sandbox_policy_changed,
                next_cwd,
                codex_home,
                session_source,
            )
        };

        self.maybe_refresh_shell_snapshot_for_cwd(
            &previous_cwd,
            &next_cwd,
            &codex_home,
            &session_source,
        );
        if sandbox_policy_changed {
            self.refresh_managed_network_proxy_for_current_sandbox_policy()
                .await;
        }

        Ok(())
    }

    pub(crate) async fn validate_settings(
        &self,
        updates: &SessionSettingsUpdate,
    ) -> ConstraintResult<()> {
        let state = self.state.lock().await;
        state.session_configuration.apply(updates).map(|_| ())
    }

    pub(crate) async fn set_session_startup_prewarm(
        &self,
        startup_prewarm: SessionStartupPrewarmHandle,
    ) {
        let mut state = self.state.lock().await;
        state.set_session_startup_prewarm(startup_prewarm);
    }

    pub(crate) async fn take_session_startup_prewarm(&self) -> Option<SessionStartupPrewarmHandle> {
        let mut state = self.state.lock().await;
        state.take_session_startup_prewarm()
    }

    pub(crate) async fn get_config(&self) -> std::sync::Arc<Config> {
        let state = self.state.lock().await;
        state
            .session_configuration
            .original_config_do_not_use
            .clone()
    }

    pub(crate) async fn provider(&self) -> ModelProviderInfo {
        let state = self.state.lock().await;
        state.session_configuration.provider.clone()
    }

    pub(crate) async fn reload_user_config_layer(&self) {
        let config_toml_path = {
            let state = self.state.lock().await;
            state
                .session_configuration
                .codex_home
                .join(CONFIG_TOML_FILE)
        };

        let user_config = match std::fs::read_to_string(&config_toml_path) {
            Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                Ok(config) => config,
                Err(err) => {
                    warn!("failed to parse user config while reloading layer: {err}");
                    return;
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                toml::Value::Table(Default::default())
            }
            Err(err) => {
                warn!("failed to read user config while reloading layer: {err}");
                return;
            }
        };

        let mut state = self.state.lock().await;
        let mut config = (*state.session_configuration.original_config_do_not_use).clone();
        config.config_layer_stack = config
            .config_layer_stack
            .with_user_config(&config_toml_path, user_config);
        state.session_configuration.original_config_do_not_use = Arc::new(config);
        self.services.skills_manager.clear_cache();
        self.services.plugins_manager.clear_cache();
    }

    async fn build_settings_update_items(
        &self,
        reference_context_item: Option<&TurnContextItem>,
        current_context: &TurnContext,
    ) -> Vec<ResponseItem> {
        // TODO: Make context updates a pure diff of persisted previous/current TurnContextItem
        // state so replay/backtracking is deterministic. Runtime inputs that affect model-visible
        // context (shell, exec policy, feature gates, previous-turn bridge) should be persisted
        // state or explicit non-state replay events.
        let previous_turn_settings = {
            let state = self.state.lock().await;
            state.previous_turn_settings()
        };
        let shell = self.user_shell();
        let exec_policy = self.services.exec_policy.current();
        crate::context_manager::updates::build_settings_update_items(
            reference_context_item,
            previous_turn_settings.as_ref(),
            current_context,
            shell.as_ref(),
            exec_policy.as_ref(),
            self.features.enabled(Feature::Personality),
        )
    }

    /// Persist the event to rollout and send it to clients.
    pub(crate) async fn send_event(&self, turn_context: &TurnContext, msg: EventMsg) {
        self.send_event_with_persistence(turn_context, msg, true)
            .await;
    }

    async fn send_persisted_event(&self, turn_context: &TurnContext, msg: EventMsg) {
        self.send_event_with_persistence(turn_context, msg, false)
            .await;
    }

    async fn send_event_with_persistence(
        &self,
        turn_context: &TurnContext,
        msg: EventMsg,
        persist: bool,
    ) {
        let legacy_source = msg.clone();
        self.services
            .rollout_thread_trace
            .record_codex_turn_event(&turn_context.sub_id, &legacy_source);
        self.services
            .rollout_thread_trace
            .record_tool_call_event(turn_context.sub_id.clone(), &legacy_source);
        let event = Event {
            id: turn_context.sub_id.clone(),
            msg,
        };
        if persist {
            self.send_event_raw(event).await;
        } else {
            self.dispatch_persisted_event_raw(event).await;
        }
        self.maybe_notify_parent_of_terminal_turn(turn_context, &legacy_source)
            .await;
        self.maybe_mirror_event_text_to_realtime(&legacy_source)
            .await;
        self.maybe_clear_realtime_handoff_for_event(&legacy_source)
            .await;

        let show_raw_agent_reasoning = self.show_raw_agent_reasoning();
        for legacy in legacy_source.as_legacy_events(show_raw_agent_reasoning) {
            let legacy_event = Event {
                id: turn_context.sub_id.clone(),
                msg: legacy,
            };
            self.send_event_raw(legacy_event).await;
        }
    }

    /// Forwards terminal turn events from spawned MultiAgentV2 children to their direct parent.
    async fn maybe_notify_parent_of_terminal_turn(
        &self,
        turn_context: &TurnContext,
        msg: &EventMsg,
    ) {
        if !self.enabled(Feature::MultiAgentV2) {
            return;
        }

        if !matches!(msg, EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_)) {
            return;
        }

        let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            agent_path: Some(child_agent_path),
            ..
        }) = &turn_context.session_source
        else {
            return;
        };

        let Some(status) = agent_status_from_event(msg) else {
            return;
        };
        if !is_final(&status) {
            return;
        }

        self.forward_child_completion_to_parent(
            turn_context,
            *parent_thread_id,
            child_agent_path,
            status,
        )
        .await;
    }

    /// Sends the standard completion envelope from a spawned MultiAgentV2 child to its parent.
    async fn forward_child_completion_to_parent(
        &self,
        turn_context: &TurnContext,
        parent_thread_id: ThreadId,
        child_agent_path: &codex_protocol::AgentPath,
        status: AgentStatus,
    ) {
        let Some(parent_agent_path) = child_agent_path
            .as_str()
            .rsplit_once('/')
            .and_then(|(parent, _)| codex_protocol::AgentPath::try_from(parent).ok())
        else {
            return;
        };

        let message = format_subagent_notification_message(child_agent_path.as_str(), &status);
        // `communication` owns the message. Keep a second copy only when the
        // recorder will actually need it after parent delivery succeeds.
        let trace_message = self
            .services
            .rollout_thread_trace
            .is_enabled()
            .then(|| message.clone());
        let communication = InterAgentCommunication::new(
            child_agent_path.clone(),
            parent_agent_path,
            Vec::new(),
            message,
            /*trigger_turn*/ false,
        );
        if let Err(err) = self
            .services
            .agent_control
            .send_inter_agent_communication(parent_thread_id, communication)
            .await
        {
            debug!("failed to notify parent thread {parent_thread_id}: {err}");
            return;
        }
        let _ = self
            .services
            .agent_control
            .record_action_map_child_result(parent_thread_id, self.conversation_id, &status)
            .await;
        if let Some(message) = trace_message {
            self.services
                .rollout_thread_trace
                .record_agent_result_interaction(
                    turn_context.sub_id.as_str(),
                    parent_thread_id,
                    &AgentResultTracePayload {
                        child_agent_path: child_agent_path.as_str(),
                        message: &message,
                        status: &status,
                    },
                );
        }
    }

    async fn maybe_mirror_event_text_to_realtime(&self, msg: &EventMsg) {
        let Some(text) = realtime_text_for_event(msg) else {
            return;
        };
        if self.conversation.running_state().await.is_none()
            || self.conversation.active_handoff_id().await.is_none()
        {
            return;
        }
        if let Err(err) = self.conversation.handoff_out(text).await {
            debug!("failed to mirror event text to realtime conversation: {err}");
        }
    }

    async fn maybe_clear_realtime_handoff_for_event(&self, msg: &EventMsg) {
        if !matches!(msg, EventMsg::TurnComplete(_)) {
            return;
        }
        if let Err(err) = self.conversation.handoff_complete().await {
            debug!("failed to finalize realtime handoff output: {err}");
        }
        self.conversation.clear_active_handoff().await;
    }

    pub(crate) async fn send_event_raw(&self, event: Event) {
        // Persist the event into rollout storage (the store filters as needed).
        let rollout_items = vec![RolloutItem::EventMsg(event.msg.clone())];
        self.persist_rollout_items(&rollout_items).await;
        self.dispatch_persisted_event_raw(event).await;
    }

    async fn dispatch_persisted_event_raw(&self, event: Event) {
        self.services
            .rollout_thread_trace
            .record_protocol_event(&event.msg);
        self.deliver_event_raw(event).await;
    }

    async fn deliver_event_raw(&self, event: Event) {
        // Record the last known agent status.
        if let Some(status) = agent_status_from_event(&event.msg) {
            self.agent_status.send_replace(status);
        }
        if let Err(e) = self.tx_event.send(event).await {
            debug!("dropping event because channel is closed: {e}");
        }
    }

    pub(crate) async fn emit_turn_item_started(&self, turn_context: &TurnContext, item: &TurnItem) {
        self.send_event(
            turn_context,
            EventMsg::ItemStarted(ItemStartedEvent {
                thread_id: self.conversation_id,
                turn_id: turn_context.sub_id.clone(),
                item: item.clone(),
            }),
        )
        .await;
    }

    pub(crate) async fn emit_turn_item_completed(
        &self,
        turn_context: &TurnContext,
        item: TurnItem,
    ) {
        record_turn_ttfm_metric(turn_context, &item).await;
        self.send_event(
            turn_context,
            EventMsg::ItemCompleted(ItemCompletedEvent {
                thread_id: self.conversation_id,
                turn_id: turn_context.sub_id.clone(),
                item,
            }),
        )
        .await;
    }

    /// Adds an execpolicy amendment to both the in-memory and on-disk policies so future
    /// commands can use the newly approved prefix.
    pub(crate) async fn persist_execpolicy_amendment(
        &self,
        amendment: &ExecPolicyAmendment,
    ) -> Result<(), ExecPolicyUpdateError> {
        let codex_home = self
            .state
            .lock()
            .await
            .session_configuration
            .codex_home()
            .clone();

        self.services
            .exec_policy
            .append_amendment_and_update(&codex_home, amendment)
            .await?;

        Ok(())
    }

    pub(crate) async fn turn_context_for_sub_id(&self, sub_id: &str) -> Option<Arc<TurnContext>> {
        let active = self.active_turn.lock().await;
        active
            .as_ref()
            .and_then(|turn| turn.tasks.get(sub_id))
            .map(|task| Arc::clone(&task.turn_context))
    }

    async fn active_turn_context_and_cancellation_token(
        &self,
    ) -> Option<(Arc<TurnContext>, CancellationToken)> {
        let active = self.active_turn.lock().await;
        let (_, task) = active.as_ref()?.tasks.first()?;
        Some((
            Arc::clone(&task.turn_context),
            task.cancellation_token.child_token(),
        ))
    }

    pub(crate) async fn record_execpolicy_amendment_message(
        &self,
        sub_id: &str,
        amendment: &ExecPolicyAmendment,
    ) {
        let Some(prefixes) = format_allow_prefixes(vec![amendment.command.clone()]) else {
            warn!("execpolicy amendment for {sub_id} had no command prefix");
            return;
        };
        let fragment = ApprovedCommandPrefixSaved::new(prefixes);
        let text = fragment.render();
        let message: ResponseItem = ContextualUserFragment::into(fragment);

        if let Some(turn_context) = self.turn_context_for_sub_id(sub_id).await {
            self.record_conversation_items(&turn_context, std::slice::from_ref(&message))
                .await;
            return;
        }

        if self
            .inject_response_items(vec![ResponseInputItem::Message {
                role: "developer".to_string(),
                content: vec![ContentItem::InputText { text }],
            }])
            .await
            .is_err()
        {
            warn!("no active turn found to record execpolicy amendment message for {sub_id}");
        }
    }

    pub(crate) async fn persist_network_policy_amendment(
        &self,
        amendment: &NetworkPolicyAmendment,
        network_approval_context: &NetworkApprovalContext,
    ) -> anyhow::Result<()> {
        let _refresh_guard = self
            .managed_network_proxy_refresh_lock
            .acquire()
            .await
            .map_err(|_| anyhow::anyhow!("managed network proxy refresh semaphore closed"))?;
        let host =
            Self::validated_network_policy_amendment_host(amendment, network_approval_context)?;
        let codex_home = self
            .state
            .lock()
            .await
            .session_configuration
            .codex_home()
            .clone();
        let execpolicy_amendment =
            execpolicy_network_rule_amendment(amendment, network_approval_context, &host);

        if let Some(started_network_proxy) = self.services.network_proxy.as_ref() {
            let proxy = started_network_proxy.proxy();
            match amendment.action {
                NetworkPolicyRuleAction::Allow => proxy
                    .add_allowed_domain(&host)
                    .await
                    .map_err(|err| anyhow::anyhow!("failed to update runtime allowlist: {err}"))?,
                NetworkPolicyRuleAction::Deny => proxy
                    .add_denied_domain(&host)
                    .await
                    .map_err(|err| anyhow::anyhow!("failed to update runtime denylist: {err}"))?,
            }
        }

        self.services
            .exec_policy
            .append_network_rule_and_update(
                &codex_home,
                &host,
                execpolicy_amendment.protocol,
                execpolicy_amendment.decision,
                Some(execpolicy_amendment.justification),
            )
            .await
            .map_err(|err| {
                anyhow::anyhow!("failed to persist network policy amendment to execpolicy: {err}")
            })?;

        Ok(())
    }

    fn validated_network_policy_amendment_host(
        amendment: &NetworkPolicyAmendment,
        network_approval_context: &NetworkApprovalContext,
    ) -> anyhow::Result<String> {
        let approved_host = normalize_host(&network_approval_context.host);
        let amendment_host = normalize_host(&amendment.host);
        if amendment_host != approved_host {
            return Err(anyhow::anyhow!(
                "network policy amendment host '{}' does not match approved host '{}'",
                amendment.host,
                network_approval_context.host
            ));
        }
        Ok(approved_host)
    }

    pub(crate) async fn record_network_policy_amendment_message(
        &self,
        sub_id: &str,
        amendment: &NetworkPolicyAmendment,
    ) {
        let fragment = NetworkRuleSaved::new(amendment);
        let text = fragment.render();
        let message: ResponseItem = ContextualUserFragment::into(fragment);

        if let Some(turn_context) = self.turn_context_for_sub_id(sub_id).await {
            self.record_conversation_items(&turn_context, std::slice::from_ref(&message))
                .await;
            return;
        }

        if self
            .inject_response_items(vec![ResponseInputItem::Message {
                role: "developer".to_string(),
                content: vec![ContentItem::InputText { text }],
            }])
            .await
            .is_err()
        {
            warn!("no active turn found to record network policy amendment message for {sub_id}");
        }
    }

    /// Emit an exec approval request event and await the user's decision.
    ///
    /// The request is keyed by `call_id` + `approval_id` so matching responses
    /// are delivered to the correct in-flight turn. If the pending approval is
    /// cleared before a response arrives, treat it as an abort so interrupted
    /// turns do not continue on a synthetic denial.
    ///
    /// Note that if `available_decisions` is `None`, then the other fields will
    /// be used to derive the available decisions via
    /// [ExecApprovalRequestEvent::default_available_decisions].
    #[allow(clippy::too_many_arguments)]
    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn request_command_approval(
        &self,
        turn_context: &TurnContext,
        call_id: String,
        approval_id: Option<String>,
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        reason: Option<String>,
        network_approval_context: Option<NetworkApprovalContext>,
        proposed_execpolicy_amendment: Option<ExecPolicyAmendment>,
        additional_permissions: Option<AdditionalPermissionProfile>,
        available_decisions: Option<Vec<ReviewDecision>>,
    ) -> ReviewDecision {
        //  command-level approvals use `call_id`.
        // `approval_id` is only present for subcommand callbacks (execve intercept)
        let effective_approval_id = approval_id.clone().unwrap_or_else(|| call_id.clone());
        // Add the tx_approve callback to the map before sending the request.
        let (tx_approve, rx_approve) = oneshot::channel();
        let prev_entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.insert_pending_approval(effective_approval_id.clone(), tx_approve)
                }
                None => None,
            }
        };
        if prev_entry.is_some() {
            warn!("Overwriting existing pending approval for call_id: {effective_approval_id}");
        }

        let parsed_cmd = parse_command(&command);
        let proposed_network_policy_amendments = network_approval_context.as_ref().map(|context| {
            vec![
                NetworkPolicyAmendment {
                    host: context.host.clone(),
                    action: NetworkPolicyRuleAction::Allow,
                },
                NetworkPolicyAmendment {
                    host: context.host.clone(),
                    action: NetworkPolicyRuleAction::Deny,
                },
            ]
        });
        let available_decisions = available_decisions.unwrap_or_else(|| {
            ExecApprovalRequestEvent::default_available_decisions(
                network_approval_context.as_ref(),
                proposed_execpolicy_amendment.as_ref(),
                proposed_network_policy_amendments.as_deref(),
                additional_permissions.as_ref(),
            )
        });
        let event = EventMsg::ExecApprovalRequest(ExecApprovalRequestEvent {
            call_id,
            approval_id,
            turn_id: turn_context.sub_id.clone(),
            command,
            cwd,
            reason,
            network_approval_context,
            proposed_execpolicy_amendment,
            proposed_network_policy_amendments,
            additional_permissions,
            available_decisions: Some(available_decisions),
            parsed_cmd,
        });
        self.send_event(turn_context, event).await;
        rx_approve.await.unwrap_or(ReviewDecision::Abort)
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn request_patch_approval(
        &self,
        turn_context: &TurnContext,
        call_id: String,
        changes: HashMap<PathBuf, FileChange>,
        reason: Option<String>,
        grant_root: Option<PathBuf>,
    ) -> oneshot::Receiver<ReviewDecision> {
        // Add the tx_approve callback to the map before sending the request.
        let (tx_approve, rx_approve) = oneshot::channel();
        let approval_id = call_id.clone();
        let prev_entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.insert_pending_approval(approval_id.clone(), tx_approve)
                }
                None => None,
            }
        };
        if prev_entry.is_some() {
            warn!("Overwriting existing pending approval for call_id: {approval_id}");
        }

        let event = EventMsg::ApplyPatchApprovalRequest(ApplyPatchApprovalRequestEvent {
            call_id,
            turn_id: turn_context.sub_id.clone(),
            changes,
            reason,
            grant_root,
        });
        self.send_event(turn_context, event).await;
        rx_approve
    }

    pub async fn request_permissions(
        self: &Arc<Self>,
        turn_context: &Arc<TurnContext>,
        call_id: String,
        args: RequestPermissionsArgs,
        cancellation_token: CancellationToken,
    ) -> Option<RequestPermissionsResponse> {
        self.request_permissions_for_cwd(
            turn_context,
            call_id,
            args,
            turn_context.cwd.clone(),
            cancellation_token,
        )
        .await
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub(crate) async fn request_permissions_for_cwd(
        self: &Arc<Self>,
        turn_context: &Arc<TurnContext>,
        call_id: String,
        args: RequestPermissionsArgs,
        cwd: AbsolutePathBuf,
        cancellation_token: CancellationToken,
    ) -> Option<RequestPermissionsResponse> {
        match turn_context.as_ref().approval_policy.value() {
            AskForApproval::Never => {
                return Some(RequestPermissionsResponse {
                    permissions: RequestPermissionProfile::default(),
                    scope: PermissionGrantScope::Turn,
                    strict_auto_review: false,
                });
            }
            AskForApproval::Granular(granular_config)
                if !granular_config.allows_request_permissions() =>
            {
                return Some(RequestPermissionsResponse {
                    permissions: RequestPermissionProfile::default(),
                    scope: PermissionGrantScope::Turn,
                    strict_auto_review: false,
                });
            }
            AskForApproval::OnFailure
            | AskForApproval::OnRequest
            | AskForApproval::UnlessTrusted
            | AskForApproval::Granular(_) => {}
        }

        let requested_permissions = args.permissions;

        if crate::guardian::routes_approval_to_guardian(turn_context.as_ref()) {
            let originating_turn_state = {
                let active = self.active_turn.lock().await;
                active.as_ref().map(|active| Arc::clone(&active.turn_state))
            };
            let review_id = crate::guardian::new_guardian_review_id();
            let session = Arc::clone(self);
            let turn = Arc::clone(turn_context);
            let request = crate::guardian::GuardianApprovalRequest::RequestPermissions {
                id: call_id,
                turn_id: turn_context.sub_id.clone(),
                reason: args.reason,
                permissions: requested_permissions.clone(),
            };
            let review_rx = crate::guardian::spawn_approval_request_review(
                session,
                turn,
                review_id,
                request,
                /*retry_reason*/ None,
                codex_analytics::GuardianApprovalRequestSource::MainTurn,
                cancellation_token.clone(),
            );
            let decision = tokio::select! {
                biased;
                _ = cancellation_token.cancelled() => return None,
                decision = review_rx => decision.unwrap_or(ReviewDecision::Denied),
            };
            let response = match decision {
                ReviewDecision::Approved | ReviewDecision::ApprovedExecpolicyAmendment { .. } => {
                    RequestPermissionsResponse {
                        permissions: requested_permissions.clone(),
                        scope: PermissionGrantScope::Turn,
                        strict_auto_review: false,
                    }
                }
                ReviewDecision::ApprovedForSession => RequestPermissionsResponse {
                    permissions: requested_permissions.clone(),
                    scope: PermissionGrantScope::Session,
                    strict_auto_review: false,
                },
                ReviewDecision::NetworkPolicyAmendment {
                    network_policy_amendment,
                } => match network_policy_amendment.action {
                    NetworkPolicyRuleAction::Allow => RequestPermissionsResponse {
                        permissions: requested_permissions.clone(),
                        scope: PermissionGrantScope::Turn,
                        strict_auto_review: false,
                    },
                    NetworkPolicyRuleAction::Deny => RequestPermissionsResponse {
                        permissions: RequestPermissionProfile::default(),
                        scope: PermissionGrantScope::Turn,
                        strict_auto_review: false,
                    },
                },
                ReviewDecision::Abort | ReviewDecision::Denied | ReviewDecision::TimedOut => {
                    RequestPermissionsResponse {
                        permissions: RequestPermissionProfile::default(),
                        scope: PermissionGrantScope::Turn,
                        strict_auto_review: false,
                    }
                }
            };
            let response = Self::normalize_request_permissions_response(
                requested_permissions,
                response,
                cwd.as_path(),
            );
            self.record_granted_request_permissions_for_turn(
                &response,
                originating_turn_state.as_ref(),
            )
            .await;
            return Some(response);
        }

        let (tx_response, rx_response) = oneshot::channel();
        let prev_entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.insert_pending_request_permissions(
                        call_id.clone(),
                        PendingRequestPermissions {
                            tx_response,
                            requested_permissions: requested_permissions.clone(),
                            cwd: cwd.clone(),
                        },
                    )
                }
                None => None,
            }
        };
        if prev_entry.is_some() {
            warn!("Overwriting existing pending request_permissions for call_id: {call_id}");
        }

        let event = EventMsg::RequestPermissions(RequestPermissionsEvent {
            call_id: call_id.clone(),
            turn_id: turn_context.sub_id.clone(),
            reason: args.reason,
            permissions: requested_permissions,
            cwd: Some(cwd),
        });
        self.send_event(turn_context.as_ref(), event).await;
        tokio::select! {
            biased;
            _ = cancellation_token.cancelled() => {
                let mut active = self.active_turn.lock().await;
                if let Some(at) = active.as_mut() {
                    let mut ts = at.turn_state.lock().await;
                    let _ = ts.remove_pending_request_permissions(&call_id);
                }
                None
            }
            response = rx_response => response.ok(),
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn request_user_input(
        &self,
        turn_context: &TurnContext,
        call_id: String,
        args: RequestUserInputArgs,
    ) -> Option<RequestUserInputResponse> {
        let sub_id = turn_context.sub_id.clone();
        let (tx_response, rx_response) = oneshot::channel();
        let event_id = sub_id.clone();
        let prev_entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.insert_pending_user_input(sub_id, tx_response)
                }
                None => None,
            }
        };
        if prev_entry.is_some() {
            warn!("Overwriting existing pending user input for sub_id: {event_id}");
        }

        let event = EventMsg::RequestUserInput(RequestUserInputEvent {
            call_id,
            turn_id: turn_context.sub_id.clone(),
            questions: args.questions,
        });
        self.send_event(turn_context, event).await;
        rx_response.await.ok()
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn notify_user_input_response(
        &self,
        sub_id: &str,
        response: RequestUserInputResponse,
    ) {
        let entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.remove_pending_user_input(sub_id)
                }
                None => None,
            }
        };
        match entry {
            Some(tx_response) => {
                tx_response.send(response).ok();
            }
            None => {
                warn!("No pending user input found for sub_id: {sub_id}");
            }
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn notify_request_permissions_response(
        &self,
        call_id: &str,
        response: RequestPermissionsResponse,
    ) {
        let (entry, originating_turn_state) = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    let entry = ts.remove_pending_request_permissions(call_id);
                    let originating_turn_state = entry.as_ref().map(|_| Arc::clone(&at.turn_state));
                    (entry, originating_turn_state)
                }
                None => (None, None),
            }
        };
        match entry {
            Some(entry) => {
                let response = Self::normalize_request_permissions_response(
                    entry.requested_permissions,
                    response,
                    entry.cwd.as_path(),
                );
                self.record_granted_request_permissions_for_turn(
                    &response,
                    originating_turn_state.as_ref(),
                )
                .await;
                entry.tx_response.send(response).ok();
            }
            None => {
                warn!("No pending request_permissions found for call_id: {call_id}");
            }
        }
    }

    fn normalize_request_permissions_response(
        requested_permissions: RequestPermissionProfile,
        response: RequestPermissionsResponse,
        cwd: &Path,
    ) -> RequestPermissionsResponse {
        if response.strict_auto_review && matches!(response.scope, PermissionGrantScope::Session) {
            return RequestPermissionsResponse {
                permissions: RequestPermissionProfile::default(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            };
        }

        if response.permissions.is_empty() {
            return response;
        }

        RequestPermissionsResponse {
            permissions: intersect_permission_profiles(
                requested_permissions.into(),
                response.permissions.into(),
                cwd,
            )
            .into(),
            scope: response.scope,
            strict_auto_review: response.strict_auto_review,
        }
    }

    async fn record_granted_request_permissions_for_turn(
        &self,
        response: &RequestPermissionsResponse,
        originating_turn_state: Option<&Arc<Mutex<crate::state::TurnState>>>,
    ) {
        if response.permissions.is_empty() {
            return;
        }
        match response.scope {
            PermissionGrantScope::Turn => {
                if let Some(turn_state) = originating_turn_state {
                    let mut ts = turn_state.lock().await;
                    let permissions: AdditionalPermissionProfile =
                        response.permissions.clone().into();
                    ts.record_granted_permissions(permissions);
                    if response.strict_auto_review {
                        ts.enable_strict_auto_review();
                    }
                }
            }
            PermissionGrantScope::Session => {
                let mut state = self.state.lock().await;
                state.record_granted_permissions(response.permissions.clone().into());
            }
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn reads must stay consistent with the matching turn state"
    )]
    pub(crate) async fn granted_turn_permissions(&self) -> Option<AdditionalPermissionProfile> {
        let active = self.active_turn.lock().await;
        let active = active.as_ref()?;
        let ts = active.turn_state.lock().await;
        ts.granted_permissions()
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn reads must stay consistent with the matching turn state"
    )]
    pub(crate) async fn strict_auto_review_enabled_for_turn(&self) -> bool {
        let active = self.active_turn.lock().await;
        let Some(active) = active.as_ref() else {
            return false;
        };
        let ts = active.turn_state.lock().await;
        ts.strict_auto_review_enabled()
    }

    pub(crate) async fn granted_session_permissions(&self) -> Option<AdditionalPermissionProfile> {
        let state = self.state.lock().await;
        state.granted_permissions()
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn notify_dynamic_tool_response(&self, call_id: &str, response: DynamicToolResponse) {
        let entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.remove_pending_dynamic_tool(call_id)
                }
                None => None,
            }
        };
        match entry {
            Some(tx_response) => {
                tx_response.send(response).ok();
            }
            None => {
                warn!("No pending dynamic tool call found for call_id: {call_id}");
            }
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn notify_approval(&self, approval_id: &str, decision: ReviewDecision) {
        let entry = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    ts.remove_pending_approval(approval_id)
                }
                None => None,
            }
        };
        match entry {
            Some(tx_approve) => {
                tx_approve.send(decision).ok();
            }
            None => {
                warn!("No pending approval found for call_id: {approval_id}");
            }
        }
    }

    /// Records provider items in the active canonical context backend and persists them once.
    pub(crate) async fn record_conversation_items(
        &self,
        turn_context: &TurnContext,
        items: &[ResponseItem],
    ) {
        let taskspace_events = self.record_into_context(items, turn_context).await;
        if taskspace_events.is_empty() {
            self.persist_rollout_response_items(items).await;
        } else {
            let rollout_items = taskspace_events
                .into_iter()
                .map(|event| {
                    RolloutItem::EventMsg(EventMsg::MapRuntime(
                        MapRuntimeEvent::TaskContextEventRecorded(event.to_protocol()),
                    ))
                })
                .collect::<Vec<_>>();
            self.persist_rollout_items(&rollout_items).await;
        }
        self.send_raw_response_items(turn_context, items).await;
    }

    /// Append ResponseItems to the in-memory conversation history only.
    pub(crate) async fn record_into_history(
        &self,
        items: &[ResponseItem],
        turn_context: &TurnContext,
    ) {
        let _ = self.record_into_context(items, turn_context).await;
    }

    async fn record_into_context(
        &self,
        items: &[ResponseItem],
        turn_context: &TurnContext,
    ) -> Vec<TaskSpaceEvent> {
        let mut state = self.state.lock().await;
        state.record_items(items.iter(), turn_context.truncation_policy)
    }

    pub(crate) async fn record_model_warning(&self, message: impl Into<String>, ctx: &TurnContext) {
        self.services
            .session_telemetry
            .counter("codex.model_warning", /*inc*/ 1, &[]);
        let item = ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: format!("Warning: {}", message.into()),
            }],
            end_turn: None,
            phase: None,
        };

        self.record_conversation_items(ctx, &[item]).await;
    }

    async fn maybe_warn_on_server_model_mismatch(
        self: &Arc<Self>,
        turn_context: &Arc<TurnContext>,
        server_model: String,
    ) -> bool {
        let requested_model = turn_context.model_info.slug.clone();
        let server_model_normalized = server_model.to_ascii_lowercase();
        let requested_model_normalized = requested_model.to_ascii_lowercase();
        if server_model_normalized == requested_model_normalized {
            info!("server reported model {server_model} (matches requested model)");
            return false;
        }

        warn!("server reported model {server_model} while requested model was {requested_model}");

        let warning_message = format!(
            "Your account was flagged for potentially high-risk cyber activity and this request was routed to gpt-5.2 as a fallback. To regain access to gpt-5.3-codex, apply for trusted access: {CYBER_VERIFY_URL} or learn more: {CYBER_SAFETY_URL}"
        );

        self.send_event(
            turn_context,
            EventMsg::ModelReroute(ModelRerouteEvent {
                from_model: requested_model.clone(),
                to_model: server_model.clone(),
                reason: ModelRerouteReason::HighRiskCyberActivity,
            }),
        )
        .await;

        self.send_event(
            turn_context,
            EventMsg::Warning(WarningEvent {
                message: warning_message.clone(),
            }),
        )
        .await;
        self.record_model_warning(warning_message, turn_context)
            .await;
        true
    }

    pub(crate) async fn emit_model_verification(
        self: &Arc<Self>,
        turn_context: &Arc<TurnContext>,
        verifications: Vec<ModelVerification>,
    ) {
        self.send_event(
            turn_context,
            EventMsg::ModelVerification(ModelVerificationEvent { verifications }),
        )
        .await;
    }

    pub(crate) async fn replace_history(
        &self,
        items: Vec<ResponseItem>,
        reference_context_item: Option<TurnContextItem>,
    ) {
        let mut state = self.state.lock().await;
        state.replace_history(items, reference_context_item);
    }

    pub(crate) async fn replace_compacted_history(
        &self,
        _turn_context: &TurnContext,
        items: Vec<ResponseItem>,
        reference_context_item: Option<TurnContextItem>,
        compacted_item: CompactedItem,
    ) {
        let checkpoint_events = {
            let mut state = self.state.lock().await;
            state.replace_compacted_history(items, reference_context_item.clone())
        };

        let taskspace_compacted = !checkpoint_events.is_empty();
        if taskspace_compacted {
            let rollout_items = checkpoint_events
                .into_iter()
                .map(|event| {
                    RolloutItem::EventMsg(EventMsg::MapRuntime(
                        MapRuntimeEvent::TaskContextEventRecorded(event.to_protocol()),
                    ))
                })
                .collect::<Vec<_>>();
            self.persist_rollout_items(&rollout_items).await;
            self.emit_action_map_checkpoint_raw("compaction").await;
        }

        if !taskspace_compacted {
            self.persist_rollout_items(&[RolloutItem::Compacted(compacted_item)])
                .await;
        }
        if let Some(turn_context_item) = reference_context_item {
            self.persist_rollout_items(&[RolloutItem::TurnContext(turn_context_item)])
                .await;
        }
        self.services.model_client.advance_window_generation();
    }

    async fn persist_rollout_response_items(&self, items: &[ResponseItem]) {
        let rollout_items: Vec<RolloutItem> = items
            .iter()
            .cloned()
            .map(RolloutItem::ResponseItem)
            .collect();
        self.persist_rollout_items(&rollout_items).await;
    }

    pub fn enabled(&self, feature: Feature) -> bool {
        self.features.enabled(feature)
    }

    pub(crate) fn features(&self) -> ManagedFeatures {
        self.features.clone()
    }

    pub(crate) async fn collaboration_mode(&self) -> CollaborationMode {
        let state = self.state.lock().await;
        state.session_configuration.collaboration_mode.clone()
    }

    async fn send_raw_response_items(&self, turn_context: &TurnContext, items: &[ResponseItem]) {
        for item in items {
            self.send_event(
                turn_context,
                EventMsg::RawResponseItem(RawResponseItemEvent { item: item.clone() }),
            )
            .await;
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "MCP app context rendering reads through the session-owned manager guard"
    )]
    pub(crate) async fn build_initial_context(
        &self,
        turn_context: &TurnContext,
    ) -> Vec<ResponseItem> {
        let mut developer_sections = Vec::<String>::with_capacity(8);
        let mut taskspace_developer_sections = Vec::<String>::with_capacity(1);
        let mut contextual_user_sections = Vec::<String>::with_capacity(2);
        let shell = self.user_shell();
        let (
            reference_context_item,
            previous_turn_settings,
            collaboration_mode,
            base_instructions,
            session_source,
            map_runtime_mode,
            taskspace_skill_snapshot,
        ) = {
            let state = self.state.lock().await;
            let map_runtime_mode = state.action_map_runtime.mode();
            (
                state.reference_context_item(),
                state.previous_turn_settings(),
                state.session_configuration.collaboration_mode.clone(),
                state.session_configuration.base_instructions.clone(),
                state.session_configuration.session_source.clone(),
                map_runtime_mode,
                state.session_configuration.taskspace_skill_snapshot.clone(),
            )
        };
        if let Some(core_protocol) = crate::context::taskspace_core_protocol(map_runtime_mode) {
            developer_sections.push(core_protocol.to_string());
        }
        if let Some(model_switch_message) =
            crate::context_manager::updates::build_model_instructions_update_item(
                previous_turn_settings.as_ref(),
                turn_context,
            )
        {
            developer_sections.push(model_switch_message);
        }
        if turn_context.config.include_permissions_instructions {
            developer_sections.push(
                PermissionsInstructions::from_policy(
                    turn_context.sandbox_policy.get(),
                    turn_context.approval_policy.value(),
                    turn_context.config.approvals_reviewer,
                    self.services.exec_policy.current().as_ref(),
                    &turn_context.cwd,
                    turn_context
                        .features
                        .enabled(Feature::ExecPermissionApprovals),
                    turn_context
                        .features
                        .enabled(Feature::RequestPermissionsTool),
                )
                .render(),
            );
        }
        let separate_guardian_developer_message =
            crate::guardian::is_guardian_reviewer_source(&session_source);
        // Keep the guardian policy prompt out of the aggregated developer bundle so it
        // stays isolated as its own top-level developer message for guardian subagents.
        if !separate_guardian_developer_message
            && let Some(developer_instructions) = turn_context.developer_instructions.as_deref()
            && !developer_instructions.is_empty()
        {
            developer_sections.push(developer_instructions.to_string());
        }
        // Add developer instructions for memories.
        if turn_context.features.enabled(Feature::MemoryTool)
            && turn_context.config.memories.use_memories
            && let Some(memory_prompt) =
                build_memory_tool_developer_instructions(&turn_context.config.codex_home).await
        {
            developer_sections.push(memory_prompt);
        }
        // Add developer instructions from collaboration_mode if they exist and are non-empty
        if let Some(collab_instructions) =
            CollaborationModeInstructions::from_collaboration_mode(&collaboration_mode)
        {
            developer_sections.push(collab_instructions.render());
        }
        let action_map_transition_notice = {
            let mut state = self.state.lock().await;
            state.action_map_runtime.take_pending_transition_notice()
        };
        if let Some(realtime_update) = crate::context_manager::updates::build_initial_realtime_item(
            reference_context_item.as_ref(),
            previous_turn_settings.as_ref(),
            turn_context,
        ) {
            developer_sections.push(realtime_update);
        }
        if self.features.enabled(Feature::Personality)
            && let Some(personality) = turn_context.personality
        {
            let model_info = turn_context.model_info.clone();
            let has_baked_personality = model_info.supports_personality()
                && base_instructions == model_info.get_model_instructions(Some(personality));
            if !has_baked_personality
                && let Some(personality_message) =
                    crate::context_manager::updates::personality_message_for(
                        &model_info,
                        personality,
                    )
            {
                developer_sections
                    .push(PersonalitySpecInstructions::new(personality_message).render());
            }
        }
        if turn_context.config.include_apps_instructions && turn_context.apps_enabled() {
            let mcp_connection_manager = self.services.mcp_connection_manager.read().await;
            let accessible_and_enabled_connectors =
                connectors::list_accessible_and_enabled_connectors_from_manager(
                    &mcp_connection_manager,
                    &turn_context.config,
                )
                .await;
            if let Some(apps_instructions) =
                AppsInstructions::from_connectors(&accessible_and_enabled_connectors)
            {
                developer_sections.push(apps_instructions.render());
            }
        }
        if turn_context.config.include_skill_instructions {
            let available_skills = build_available_skills(
                &turn_context.turn_skills.outcome,
                default_skill_metadata_budget(turn_context.model_info.context_window),
                SkillRenderSideEffects::ThreadStart {
                    session_telemetry: &self.services.session_telemetry,
                },
            );
            if let Some(available_skills) = available_skills {
                let warning_message = available_skills.warning_message.clone();
                if let Some(identity) = taskspace_skill_snapshot.as_ref() {
                    let rendered_catalog =
                        AvailableSkillsInstructions::from(available_skills.clone()).render();
                    crate::taskspace_skill::log_catalog_render(
                        identity,
                        &turn_context.turn_skills.outcome,
                        Some(&available_skills),
                        Some(&rendered_catalog),
                    );
                }
                let skills_instructions = AvailableSkillsInstructions::from(available_skills);
                if let Some(warning_message) = warning_message {
                    self.send_event_raw(Event {
                        id: String::new(),
                        msg: EventMsg::Warning(WarningEvent {
                            message: warning_message,
                        }),
                    })
                    .await;
                }
                developer_sections.push(skills_instructions.render());
            } else if let Some(identity) = taskspace_skill_snapshot.as_ref() {
                crate::taskspace_skill::log_catalog_render(
                    identity,
                    &turn_context.turn_skills.outcome,
                    None,
                    None,
                );
            }
        } else if let Some(identity) = taskspace_skill_snapshot.as_ref() {
            crate::taskspace_skill::log_catalog_render(
                identity,
                &turn_context.turn_skills.outcome,
                None,
                None,
            );
        }
        let loaded_plugins = self
            .services
            .plugins_manager
            .plugins_for_config(&turn_context.config)
            .await;
        if let Some(plugin_instructions) =
            AvailablePluginsInstructions::from_plugins(loaded_plugins.capability_summaries())
        {
            developer_sections.push(plugin_instructions.render());
        }
        if turn_context.features.enabled(Feature::CodexGitCommit)
            && let Some(commit_message_instruction) = commit_message_trailer_instruction(
                turn_context.config.commit_attribution.as_deref(),
            )
        {
            developer_sections.push(commit_message_instruction);
        }
        if let Some(action_map_transition_notice) = action_map_transition_notice {
            taskspace_developer_sections.push(action_map_transition_notice);
        }
        if let Some(user_instructions) = turn_context.user_instructions.as_deref() {
            contextual_user_sections.push(
                UserInstructions {
                    text: user_instructions.to_string(),
                    directory: turn_context.cwd.to_string_lossy().into_owned(),
                }
                .render(),
            );
        }
        if turn_context.config.include_environment_context {
            let subagents = self
                .services
                .agent_control
                .format_environment_context_subagents(self.conversation_id)
                .await;
            contextual_user_sections.push(
                crate::context::EnvironmentContext::from_turn_context(turn_context, shell.as_ref())
                    .with_subagents(subagents)
                    .render(),
            );
        }

        let mut items = Vec::with_capacity(4);
        if let Some(developer_message) =
            crate::context_manager::updates::build_developer_update_item(developer_sections)
        {
            items.push(developer_message);
        }
        if let Some(taskspace_developer_message) =
            crate::context_manager::updates::build_developer_update_item(
                taskspace_developer_sections,
            )
        {
            items.push(taskspace_developer_message);
        }
        if let Some(contextual_user_message) =
            crate::context_manager::updates::build_contextual_user_message(contextual_user_sections)
        {
            items.push(contextual_user_message);
        }
        // Emit the guardian policy prompt as a separate developer item so the guardian
        // subagent sees a distinct, easy-to-audit instruction block.
        if separate_guardian_developer_message
            && let Some(developer_instructions) = turn_context.developer_instructions.as_deref()
            && !developer_instructions.is_empty()
            && let Some(guardian_developer_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    developer_instructions.to_string(),
                ])
        {
            items.push(guardian_developer_message);
        }
        items
    }

    pub(crate) async fn persist_rollout_items(&self, items: &[RolloutItem]) {
        if let Some(live_thread) = self.live_thread()
            && let Err(e) = live_thread
                .append_items(&self.sanitize_rollout_items_for_persistence(items).await)
                .await
        {
            if matches!(e, ThreadStoreError::ThreadNotFound { .. })
                && self.shutting_down.load(Ordering::SeqCst)
            {
                debug!("skipped rollout append after thread persistence shutdown: {e:#}");
            } else {
                error!("failed to record rollout items: {e:#}");
            }
        }
    }

    async fn sanitize_rollout_items_for_persistence(
        &self,
        items: &[RolloutItem],
    ) -> Vec<RolloutItem> {
        let rollout_path = self.current_rollout_path().await.ok().flatten();
        let mut sanitized = Vec::with_capacity(items.len());
        for item in items {
            sanitized.push(match item {
                RolloutItem::ResponseItem(response_item) => RolloutItem::ResponseItem(
                    self.sanitize_rollout_response_item_for_persistence(
                        response_item,
                        rollout_path.as_deref(),
                    )
                    .await,
                ),
                _ => item.clone(),
            });
        }
        sanitized
    }

    async fn sanitize_rollout_response_item_for_persistence(
        &self,
        item: &ResponseItem,
        rollout_path: Option<&Path>,
    ) -> ResponseItem {
        match item {
            ResponseItem::FunctionCallOutput { call_id, output } => {
                ResponseItem::FunctionCallOutput {
                    call_id: call_id.clone(),
                    output: self
                        .sanitize_rollout_function_output_payload(output, rollout_path)
                        .await,
                }
            }
            ResponseItem::CustomToolCallOutput {
                call_id,
                name,
                output,
            } => ResponseItem::CustomToolCallOutput {
                call_id: call_id.clone(),
                name: name.clone(),
                output: self
                    .sanitize_rollout_function_output_payload(output, rollout_path)
                    .await,
            },
            _ => item.clone(),
        }
    }

    async fn sanitize_rollout_function_output_payload(
        &self,
        output: &FunctionCallOutputPayload,
        rollout_path: Option<&Path>,
    ) -> FunctionCallOutputPayload {
        let body = match &output.body {
            FunctionCallOutputBody::Text(text) => FunctionCallOutputBody::Text(
                self.sanitize_rollout_output_text_for_persistence(text, rollout_path)
                    .await,
            ),
            FunctionCallOutputBody::ContentItems(items)
                if items.iter().all(|item| {
                    matches!(item, FunctionCallOutputContentItem::InputText { .. })
                }) =>
            {
                let text = items
                    .iter()
                    .filter_map(|item| match item {
                        FunctionCallOutputContentItem::InputText { text } => Some(text.as_str()),
                        FunctionCallOutputContentItem::InputImage { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                FunctionCallOutputBody::Text(
                    self.sanitize_rollout_output_text_for_persistence(&text, rollout_path)
                        .await,
                )
            }
            FunctionCallOutputBody::ContentItems(items) => {
                let mut sanitized = Vec::with_capacity(items.len());
                for item in items {
                    sanitized.push(match item {
                        FunctionCallOutputContentItem::InputText { text } => {
                            FunctionCallOutputContentItem::InputText {
                                text: self
                                    .sanitize_rollout_output_text_for_persistence(
                                        text,
                                        rollout_path,
                                    )
                                    .await,
                            }
                        }
                        FunctionCallOutputContentItem::InputImage { .. } => item.clone(),
                    });
                }
                FunctionCallOutputBody::ContentItems(sanitized)
            }
        };

        FunctionCallOutputPayload {
            body,
            success: output.success,
        }
    }

    async fn sanitize_rollout_output_text_for_persistence(
        &self,
        text: &str,
        rollout_path: Option<&Path>,
    ) -> String {
        if text.contains("OutputReferenceV1") || text.contains("output-ref://") {
            return text.to_string();
        }

        let raw = text.as_bytes();
        let Ok(artifact_ref) = write_output_artifact_for_rollout(rollout_path, raw).await else {
            return text.to_string();
        };
        reference_text_for_raw_output(raw, artifact_ref.as_deref())
            .unwrap_or_else(|| text.to_string())
    }

    pub(crate) async fn clone_history(&self) -> ContextManager {
        let state = self.state.lock().await;
        state.clone_history()
    }

    pub(crate) async fn reference_context_item(&self) -> Option<TurnContextItem> {
        let state = self.state.lock().await;
        state.reference_context_item()
    }

    pub(crate) async fn action_map_snapshot(&self) -> ActionMapSnapshot {
        let state = self.state.lock().await;
        state.action_map_runtime.snapshot()
    }

    pub(crate) async fn read_action_map_projection(
        &self,
        turn_context: &TurnContext,
        call_id: &str,
    ) -> Result<String, String> {
        let (projection, projection_events) = {
            let mut state = self.state.lock().await;
            if state.action_map_runtime.mode() != MapRuntimeMode::Experiment {
                return Err("TaskSpace map read requires TaskSpace mode.".to_string());
            }
            let policy = state
                .session_configuration
                .taskspace_projection_policy
                .ok_or_else(|| "TaskSpace projection policy is missing.".to_string())?;
            let projection = state
                .action_map_runtime
                .build_developer_context(ProjectionEnvelope::CurrentProjection)
                .ok_or_else(|| "TaskSpace current Map projection is unavailable.".to_string())?;
            let identity = projection_identity_from_context(&projection).ok_or_else(|| {
                "TaskSpace current Map projection identity is invalid.".to_string()
            })?;
            let decision = decide_projection_emission(
                policy,
                ProjectionTrigger::ExplicitRead,
                &state.taskspace_projection_cursor,
                Some(&identity),
            )?;
            if decision.emission != ProjectionEmission::ReturnAsToolResult {
                return Err("TaskSpace explicit Map read did not select a tool result.".to_string());
            }
            state.taskspace_projection_cursor = decision.next_cursor;
            let mut events = state
                .action_map_runtime
                .take_pending_projection_trace_events();
            if let Some(read_events) = state.action_map_runtime.record_map_read_trace_event(
                call_id.to_string(),
                &policy.to_string(),
                identity.map_id.clone(),
                identity.revision,
                identity.canonical_sha256.clone(),
                identity.projection_sha256.clone(),
            ) {
                events.extend(read_events);
            }
            tracing::info!(
                target: "codex_core::taskspace",
                event_name = "taskspace.map_read_completed",
                call_id,
                policy = %policy,
                map_id = ?identity.map_id,
                revision = ?identity.revision,
                canonical_sha256 = ?identity.canonical_sha256,
                projection_sha256 = identity.projection_sha256,
                "returned exact current TaskSpace Map projection"
            );
            (projection, events)
        };
        if !projection_events.is_empty() {
            self.emit_action_map_events_for_turn(turn_context, projection_events)
                .await;
        }
        Ok(projection)
    }

    pub(crate) async fn taskspace_event_id_for_call(
        &self,
        call_id: &str,
    ) -> Result<String, String> {
        let state = self.state.lock().await;
        state
            .taskspace_events
            .event_id_for_call(call_id)
            .ok_or_else(|| format!("TaskSpace canonical event is missing for call `{call_id}`."))
    }

    pub(crate) async fn taskspace_initialization_source_event_ids(
        &self,
        call_id: &str,
    ) -> Result<Vec<String>, String> {
        let state = self.state.lock().await;
        let source_event_ids = state
            .taskspace_events
            .initialization_source_event_ids(call_id);
        if source_event_ids.is_empty() {
            Err(format!(
                "TaskSpace canonical initialization sources are missing for call `{call_id}`."
            ))
        } else {
            Ok(source_event_ids)
        }
    }

    pub(crate) async fn prepare_provider_visible_prompt_items(
        &self,
        turn_context: &TurnContext,
        mut items: Vec<ResponseItem>,
    ) -> PreparedProviderPromptItems {
        let (policy, projection_item, map_handle_item, projection_trace_events) = {
            let mut state = self.state.lock().await;
            if state.action_map_runtime.mode() != MapRuntimeMode::Experiment {
                return PreparedProviderPromptItems {
                    items,
                    projection_identity: None,
                };
            }
            let policy = state
                .session_configuration
                .taskspace_projection_policy
                .expect("TaskSpace mode requires an immutable projection policy");
            let context = match policy {
                TaskSpaceProjectionPolicy::MapAlways => state
                    .action_map_runtime
                    .build_developer_context(ProjectionEnvelope::CurrentProjection),
                TaskSpaceProjectionPolicy::MapAppend => state
                    .action_map_runtime
                    .build_developer_context(ProjectionEnvelope::RequestSnapshot),
                TaskSpaceProjectionPolicy::MapRequest => None,
            };
            let candidate = context
                .as_deref()
                .and_then(projection_identity_from_context);
            let projection_is_current_tail = candidate.as_ref().is_some_and(|candidate| {
                taskspace_projection_context(items.last()).is_some_and(|context| {
                    projection_identity_from_context(context).as_ref() == Some(candidate)
                })
            });
            let decision = decide_projection_emission(
                policy,
                ProjectionTrigger::ProviderRequest {
                    projection_is_current_tail,
                },
                &state.taskspace_projection_cursor,
                candidate.as_ref(),
            )
            .expect("configured TaskSpace projection policy must be enabled");
            state.taskspace_projection_cursor = decision.next_cursor;
            let projection_item = match decision.emission {
                ProjectionEmission::ReplaceLatest | ProjectionEmission::AppendSnapshot => context
                    .and_then(|projection| {
                        crate::context_manager::updates::build_developer_update_item(vec![
                            projection,
                        ])
                    }),
                ProjectionEmission::None => None,
                ProjectionEmission::ReturnAsToolResult => {
                    unreachable!("provider request cannot select a non-provider emission")
                }
            };
            let map_handle_item = (policy == TaskSpaceProjectionPolicy::MapRequest)
                .then(|| state.action_map_runtime.build_map_handle_context())
                .flatten()
                .and_then(|handle| {
                    crate::context_manager::updates::build_contextual_user_message(vec![handle])
                });
            tracing::debug!(
                target: "codex_core::taskspace",
                event_name = "taskspace.projection_emission_decided",
                policy = %policy,
                trigger = "provider_request",
                emission = ?decision.emission,
                projection_is_current_tail,
                "decided TaskSpace provider projection emission"
            );
            let events = state
                .action_map_runtime
                .take_pending_projection_trace_events();
            (policy, projection_item, map_handle_item, events)
        };
        if !projection_trace_events.is_empty() {
            self.emit_action_map_events_for_turn(turn_context, projection_trace_events)
                .await;
        }
        match policy {
            TaskSpaceProjectionPolicy::MapAlways => {
                items = remove_taskspace_projection_items(items);
                if let Some(projection_item) = projection_item {
                    items.push(projection_item);
                }
            }
            TaskSpaceProjectionPolicy::MapAppend => {
                if let Some(projection_item) = projection_item {
                    self.record_conversation_items(
                        turn_context,
                        std::slice::from_ref(&projection_item),
                    )
                    .await;
                    let projection_context = taskspace_projection_context(Some(&projection_item))
                        .expect("appended TaskSpace item must contain a projection");
                    let identity = projection_identity_from_context(projection_context)
                        .expect("appended TaskSpace projection must have an identity");
                    tracing::info!(
                        target: "codex_core::taskspace",
                        event_name = "taskspace.projection_request_tail_appended",
                        map_id = ?identity.map_id,
                        revision = ?identity.revision,
                        canonical_sha256 = ?identity.canonical_sha256,
                        projection_sha256 = identity.projection_sha256,
                        carrier_role = "user",
                        persistent = true,
                        request_tail = true,
                        "persisted latest TaskSpace projection at provider request tail"
                    );
                    items.push(projection_item);
                }
                rewrite_taskspace_projection_items_for_append(&mut items);
                debug_assert!(
                    taskspace_projection_context(items.last()).is_some(),
                    "map-append provider request must end with the latest projection"
                );
            }
            TaskSpaceProjectionPolicy::MapRequest => {
                debug_assert!(projection_item.is_none());
                items = remove_taskspace_map_handle_items(items);
                if let Some(map_handle_item) = map_handle_item {
                    let handle = taskspace_map_handle_context(Some(&map_handle_item))
                        .expect("map-request handle item must contain a handle");
                    tracing::info!(
                        target: "codex_core::taskspace",
                        event_name = "taskspace.map_handle_request_tail_emitted",
                        carrier_role = "user",
                        persistent = false,
                        bytes = handle.len(),
                        "emitted current non-persistent TaskSpace Map handle at provider request tail"
                    );
                    items.push(map_handle_item);
                }
            }
        }
        let projection_identity = match policy {
            TaskSpaceProjectionPolicy::MapRequest => {
                Some(ProviderProjectionIdentityExpectation::without_automatic_projection(policy))
            }
            TaskSpaceProjectionPolicy::MapAlways | TaskSpaceProjectionPolicy::MapAppend => {
                latest_taskspace_projection_context(&items).and_then(|context| {
                    ProviderProjectionIdentityExpectation::from_projection_context(policy, context)
                })
            }
        };
        PreparedProviderPromptItems {
            items,
            projection_identity,
        }
    }

    /// Persist the latest turn context snapshot for the first real user turn and for
    /// steady-state turns that emit model-visible context updates.
    ///
    /// When the reference snapshot is missing, this injects full initial context. Otherwise, it
    /// emits only settings diff items.
    ///
    /// If full context is injected and a model switch occurred, this prepends the
    /// `<model_switch>` developer message so model-specific instructions are not lost.
    ///
    /// This is the normal runtime path that establishes a new `reference_context_item`.
    /// Mid-turn compaction is the other path that can re-establish that baseline when it
    /// reinjects full initial context into replacement history. Other non-regular tasks
    /// intentionally do not update the baseline.
    pub(crate) async fn record_context_updates_and_set_reference_context_item(
        &self,
        turn_context: &TurnContext,
    ) {
        let reference_context_item = {
            let state = self.state.lock().await;
            state.reference_context_item()
        };
        let should_inject_full_context = reference_context_item.is_none();
        let context_items = if should_inject_full_context {
            self.build_initial_context(turn_context).await
        } else {
            // Steady-state path: append only context diffs to minimize token overhead.
            self.build_settings_update_items(reference_context_item.as_ref(), turn_context)
                .await
        };
        let turn_context_item = turn_context.to_turn_context_item();
        if !context_items.is_empty() {
            self.record_conversation_items(turn_context, &context_items)
                .await;
        }
        // Persist one `TurnContextItem` per real user turn so resume/lazy replay can recover the
        // latest durable baseline even when this turn emitted no model-visible context diffs.
        self.persist_rollout_items(&[RolloutItem::TurnContext(turn_context_item.clone())])
            .await;

        // Advance the in-memory diff baseline even when this turn emitted no model-visible
        // context items. This keeps later runtime diffing aligned with the current turn state.
        let mut state = self.state.lock().await;
        state.set_reference_context_item(Some(turn_context_item));
    }

    pub(crate) async fn update_token_usage_info(
        &self,
        turn_context: &TurnContext,
        token_usage: Option<&TokenUsage>,
    ) {
        if let Some(token_usage) = token_usage {
            let mut state = self.state.lock().await;
            state.update_token_info_from_usage(token_usage, turn_context.model_context_window());
        }
        self.send_token_count_event(turn_context).await;
    }

    pub(crate) async fn recompute_token_usage(&self, turn_context: &TurnContext) {
        let history = self.clone_history().await;
        let base_instructions = self.get_base_instructions().await;
        let Some(estimated_total_tokens) =
            history.estimate_token_count_with_base_instructions(&base_instructions)
        else {
            return;
        };
        {
            let mut state = self.state.lock().await;
            let mut info = state.token_info().unwrap_or(TokenUsageInfo {
                total_token_usage: TokenUsage::default(),
                last_token_usage: TokenUsage::default(),
                model_context_window: None,
            });

            info.last_token_usage = TokenUsage {
                input_tokens: 0,
                cached_input_tokens: 0,
                output_tokens: 0,
                reasoning_output_tokens: 0,
                total_tokens: estimated_total_tokens.max(0),
            };

            if let Some(model_context_window) = turn_context.model_context_window() {
                info.model_context_window = Some(model_context_window);
            }

            state.set_token_info(Some(info));
        }
        self.send_token_count_event(turn_context).await;
    }

    pub(crate) async fn update_rate_limits(
        &self,
        turn_context: &TurnContext,
        new_rate_limits: RateLimitSnapshot,
    ) {
        {
            let mut state = self.state.lock().await;
            state.set_rate_limits(new_rate_limits);
        }
        self.send_token_count_event(turn_context).await;
    }

    pub(crate) async fn mcp_dependency_prompted(&self) -> HashSet<String> {
        let state = self.state.lock().await;
        state.mcp_dependency_prompted()
    }

    pub(crate) async fn record_mcp_dependency_prompted<I>(&self, names: I)
    where
        I: IntoIterator<Item = String>,
    {
        let mut state = self.state.lock().await;
        state.record_mcp_dependency_prompted(names);
    }

    pub async fn dependency_env(&self) -> HashMap<String, String> {
        let state = self.state.lock().await;
        state.dependency_env()
    }

    pub async fn set_dependency_env(&self, values: HashMap<String, String>) {
        let mut state = self.state.lock().await;
        state.set_dependency_env(values);
    }

    pub(crate) async fn set_server_reasoning_included(&self, included: bool) {
        let mut state = self.state.lock().await;
        state.set_server_reasoning_included(included);
    }

    async fn send_token_count_event(&self, turn_context: &TurnContext) {
        let (info, rate_limits) = {
            let state = self.state.lock().await;
            state.token_info_and_rate_limits()
        };
        let event = EventMsg::TokenCount(TokenCountEvent { info, rate_limits });
        self.send_event(turn_context, event).await;
    }

    pub(crate) async fn set_total_tokens_full(&self, turn_context: &TurnContext) {
        if let Some(context_window) = turn_context.model_context_window() {
            let mut state = self.state.lock().await;
            state.set_token_usage_full(context_window);
        }
        self.send_token_count_event(turn_context).await;
    }

    pub(crate) async fn record_response_item_and_emit_turn_item(
        &self,
        turn_context: &TurnContext,
        response_item: ResponseItem,
    ) {
        // Add to conversation history and persist response item to rollout.
        self.record_conversation_items(turn_context, std::slice::from_ref(&response_item))
            .await;

        // Derive a turn item and emit lifecycle events if applicable.
        if let Some(item) = parse_turn_item(&response_item) {
            self.emit_turn_item_started(turn_context, &item).await;
            self.emit_turn_item_completed(turn_context, item).await;
        }
    }

    pub(crate) async fn record_user_prompt_and_emit_turn_item(
        &self,
        turn_context: &TurnContext,
        input: &[UserInput],
        response_item: ResponseItem,
    ) {
        // Persist the user message to history, but emit the turn item from `UserInput` so
        // UI-only `text_elements` are preserved. `ResponseItem::Message` does not carry
        // those spans, and `record_response_item_and_emit_turn_item` would drop them.
        self.record_conversation_items(turn_context, std::slice::from_ref(&response_item))
            .await;
        let turn_item = TurnItem::UserMessage(UserMessageItem::new(input));
        self.emit_turn_item_started(turn_context, &turn_item).await;
        self.emit_turn_item_completed(turn_context, turn_item).await;
        self.ensure_rollout_materialized().await;
    }

    pub(crate) async fn notify_background_event(
        &self,
        turn_context: &TurnContext,
        message: impl Into<String>,
    ) {
        let event = EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: message.into(),
        });
        self.send_event(turn_context, event).await;
    }

    pub(crate) async fn notify_stream_error(
        &self,
        turn_context: &TurnContext,
        message: impl Into<String>,
        codex_error: CodexErr,
    ) {
        let additional_details = codex_error.to_string();
        let codex_error_info = CodexErrorInfo::ResponseStreamDisconnected {
            http_status_code: codex_error.http_status_code_value(),
        };
        let event = EventMsg::StreamError(StreamErrorEvent {
            message: message.into(),
            codex_error_info: Some(codex_error_info),
            additional_details: Some(additional_details),
        });
        self.send_event(turn_context, event).await;
    }

    async fn maybe_start_ghost_snapshot(
        self: &Arc<Self>,
        turn_context: Arc<TurnContext>,
        cancellation_token: CancellationToken,
    ) {
        if !self.enabled(Feature::GhostCommit) {
            return;
        }
        let token = match turn_context.tool_call_gate.subscribe().await {
            Ok(token) => token,
            Err(err) => {
                warn!("failed to subscribe to ghost snapshot readiness: {err}");
                return;
            }
        };

        info!("spawning ghost snapshot task");
        let task = GhostSnapshotTask::new(token);
        Arc::new(task)
            .run(
                Arc::new(SessionTaskContext::new(self.clone())),
                turn_context.clone(),
                Vec::new(),
                cancellation_token,
            )
            .await;
    }

    /// Inject additional user input into the currently active turn.
    ///
    /// Returns the active turn id when accepted.
    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn steer_input(
        &self,
        input: Vec<UserInput>,
        expected_turn_id: Option<&str>,
        responsesapi_client_metadata: Option<HashMap<String, String>>,
    ) -> Result<String, SteerInputError> {
        if input.is_empty() {
            return Err(SteerInputError::EmptyInput);
        }

        let mut active = self.active_turn.lock().await;
        let Some(active_turn) = active.as_mut() else {
            return Err(SteerInputError::NoActiveTurn(input));
        };

        let Some((active_turn_id, _)) = active_turn.tasks.first() else {
            return Err(SteerInputError::NoActiveTurn(input));
        };

        if let Some(expected_turn_id) = expected_turn_id
            && expected_turn_id != active_turn_id
        {
            return Err(SteerInputError::ExpectedTurnMismatch {
                expected: expected_turn_id.to_string(),
                actual: active_turn_id.clone(),
            });
        }

        match active_turn.tasks.first().map(|(_, task)| task.kind) {
            Some(crate::state::TaskKind::Regular) => {}
            Some(crate::state::TaskKind::Review) => {
                return Err(SteerInputError::ActiveTurnNotSteerable {
                    turn_kind: NonSteerableTurnKind::Review,
                });
            }
            Some(crate::state::TaskKind::Compact) => {
                return Err(SteerInputError::ActiveTurnNotSteerable {
                    turn_kind: NonSteerableTurnKind::Compact,
                });
            }
            None => return Err(SteerInputError::NoActiveTurn(input)),
        }

        if let Some(responsesapi_client_metadata) = responsesapi_client_metadata
            && let Some((_, active_task)) = active_turn.tasks.first()
        {
            active_task
                .turn_context
                .turn_metadata_state
                .set_responsesapi_client_metadata(responsesapi_client_metadata);
        }

        let mut turn_state = active_turn.turn_state.lock().await;
        turn_state.push_pending_input(input.into());
        turn_state.accept_mailbox_delivery_for_current_turn();
        Ok(active_turn_id.clone())
    }

    /// Returns the input if there was no task running to inject into.
    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn inject_response_items(
        &self,
        input: Vec<ResponseInputItem>,
    ) -> Result<(), Vec<ResponseInputItem>> {
        let mut active = self.active_turn.lock().await;
        match active.as_mut() {
            Some(at) => {
                let mut ts = at.turn_state.lock().await;
                for item in input {
                    ts.push_pending_input(item);
                }
                Ok(())
            }
            None => Err(input),
        }
    }

    pub(crate) async fn defer_mailbox_delivery_to_next_turn(&self, sub_id: &str) {
        let turn_state = self.turn_state_for_sub_id(sub_id).await;
        let Some(turn_state) = turn_state else {
            return;
        };
        let mut turn_state = turn_state.lock().await;
        if turn_state.has_pending_input() {
            return;
        }
        turn_state.set_mailbox_delivery_phase(MailboxDeliveryPhase::NextTurn);
    }

    pub(crate) async fn accept_mailbox_delivery_for_current_turn(&self, sub_id: &str) {
        let turn_state = self.turn_state_for_sub_id(sub_id).await;
        let Some(turn_state) = turn_state else {
            return;
        };
        turn_state
            .lock()
            .await
            .set_mailbox_delivery_phase(MailboxDeliveryPhase::CurrentTurn);
    }

    pub(crate) async fn record_memory_citation_for_turn(&self, sub_id: &str) {
        let turn_state = self.turn_state_for_sub_id(sub_id).await;
        let Some(turn_state) = turn_state else {
            return;
        };
        turn_state.lock().await.has_memory_citation = true;
    }

    async fn turn_state_for_sub_id(
        &self,
        sub_id: &str,
    ) -> Option<Arc<tokio::sync::Mutex<crate::state::TurnState>>> {
        let active = self.active_turn.lock().await;
        active.as_ref().and_then(|active_turn| {
            active_turn
                .tasks
                .contains_key(sub_id)
                .then(|| Arc::clone(&active_turn.turn_state))
        })
    }

    pub(crate) fn subscribe_mailbox_seq(&self) -> watch::Receiver<u64> {
        self.mailbox.subscribe()
    }

    pub(crate) fn enqueue_mailbox_communication(&self, communication: InterAgentCommunication) {
        self.mailbox.send(communication);
    }

    pub(crate) async fn has_trigger_turn_mailbox_items(&self) -> bool {
        self.mailbox_rx.lock().await.has_pending_trigger_turn()
    }

    pub(crate) async fn has_pending_mailbox_items(&self) -> bool {
        self.mailbox_rx.lock().await.has_pending()
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn prepend_pending_input(&self, input: Vec<ResponseInputItem>) -> Result<(), ()> {
        let mut active = self.active_turn.lock().await;
        match active.as_mut() {
            Some(at) => {
                let mut ts = at.turn_state.lock().await;
                ts.prepend_pending_input(input);
                Ok(())
            }
            None => Err(()),
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state updates must remain atomic"
    )]
    pub async fn get_pending_input(&self) -> Vec<ResponseInputItem> {
        let (pending_input, accepts_mailbox_delivery) = {
            let mut active = self.active_turn.lock().await;
            match active.as_mut() {
                Some(at) => {
                    let mut ts = at.turn_state.lock().await;
                    (
                        ts.take_pending_input(),
                        ts.accepts_mailbox_delivery_for_current_turn(),
                    )
                }
                None => (Vec::new(), true),
            }
        };
        if !accepts_mailbox_delivery {
            return pending_input;
        }
        let mailbox_items = {
            let mut mailbox_rx = self.mailbox_rx.lock().await;
            mailbox_rx
                .drain()
                .into_iter()
                .map(|mail| mail.to_response_input_item())
                .collect::<Vec<_>>()
        };
        if pending_input.is_empty() {
            mailbox_items
        } else if mailbox_items.is_empty() {
            pending_input
        } else {
            let mut pending_input = pending_input;
            pending_input.extend(mailbox_items);
            pending_input
        }
    }

    /// Queue response items to be injected into the next active turn created for this session.
    pub(crate) async fn queue_response_items_for_next_turn(&self, items: Vec<ResponseInputItem>) {
        if items.is_empty() {
            return;
        }

        let mut idle_pending_input = self.idle_pending_input.lock().await;
        idle_pending_input.extend(items);
    }

    pub(crate) async fn take_queued_response_items_for_next_turn(&self) -> Vec<ResponseInputItem> {
        std::mem::take(&mut *self.idle_pending_input.lock().await)
    }

    pub(crate) async fn has_queued_response_items_for_next_turn(&self) -> bool {
        !self.idle_pending_input.lock().await.is_empty()
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "active turn checks and turn state reads must remain atomic"
    )]
    pub async fn has_pending_input(&self) -> bool {
        let (has_turn_pending_input, accepts_mailbox_delivery) = {
            let active = self.active_turn.lock().await;
            match active.as_ref() {
                Some(at) => {
                    let ts = at.turn_state.lock().await;
                    (
                        ts.has_pending_input(),
                        ts.accepts_mailbox_delivery_for_current_turn(),
                    )
                }
                None => (false, true),
            }
        };
        if has_turn_pending_input {
            return true;
        }
        if !accepts_mailbox_delivery {
            return false;
        }
        self.has_pending_mailbox_items().await
    }

    pub async fn interrupt_task(self: &Arc<Self>) {
        info!("interrupt received: abort current task, if any");
        let had_active_turn = self.active_turn.lock().await.is_some();
        // Even without an active task, interrupt handling pauses any active goal.
        self.abort_all_tasks(TurnAbortReason::Interrupted).await;
        if !had_active_turn {
            self.cancel_mcp_startup().await;
        }
    }

    pub(crate) fn hooks(&self) -> &Hooks {
        &self.services.hooks
    }

    pub(crate) fn user_shell(&self) -> Arc<shell::Shell> {
        Arc::clone(&self.services.user_shell)
    }

    pub(crate) async fn current_rollout_path(&self) -> anyhow::Result<Option<PathBuf>> {
        let Some(live_thread) = self.live_thread() else {
            return Ok(None);
        };
        live_thread.local_rollout_path().await.map_err(Into::into)
    }

    pub(crate) async fn hook_transcript_path(&self) -> Option<PathBuf> {
        self.ensure_rollout_materialized().await;
        match self.current_rollout_path().await {
            Ok(path) => path,
            Err(err) => {
                warn!("{err}");
                None
            }
        }
    }

    pub(crate) async fn take_pending_session_start_source(
        &self,
    ) -> Option<codex_hooks::SessionStartSource> {
        let mut state = self.state.lock().await;
        state.take_pending_session_start_source()
    }

    fn show_raw_agent_reasoning(&self) -> bool {
        self.services.show_raw_agent_reasoning
    }
}

pub(crate) fn emit_subagent_session_started(
    analytics_events_client: &AnalyticsEventsClient,
    client_metadata: AppServerClientMetadata,
    thread_id: ThreadId,
    parent_thread_id: Option<ThreadId>,
    thread_config: ThreadConfigSnapshot,
    subagent_source: SubAgentSource,
) {
    let AppServerClientMetadata {
        client_name,
        client_version,
    } = client_metadata;
    let (Some(client_name), Some(client_version)) = (client_name, client_version) else {
        tracing::warn!("skipping subagent thread analytics: missing inherited client metadata");
        return;
    };
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    analytics_events_client.track_subagent_thread_started(SubAgentThreadStartedInput {
        thread_id: thread_id.to_string(),
        parent_thread_id: parent_thread_id.map(|thread_id| thread_id.to_string()),
        product_client_id: client_name.clone(),
        client_name,
        client_version,
        model: thread_config.model,
        ephemeral: thread_config.ephemeral,
        subagent_source,
        created_at,
    });
}

fn skills_to_info(
    skills: &[SkillMetadata],
    disabled_paths: &HashSet<AbsolutePathBuf>,
) -> Vec<ProtocolSkillMetadata> {
    skills
        .iter()
        .map(|skill| ProtocolSkillMetadata {
            name: skill.name.clone(),
            description: skill.description.clone(),
            short_description: skill.short_description.clone(),
            interface: skill
                .interface
                .clone()
                .map(|interface| ProtocolSkillInterface {
                    display_name: interface.display_name,
                    short_description: interface.short_description,
                    icon_small: interface.icon_small,
                    icon_large: interface.icon_large,
                    brand_color: interface.brand_color,
                    default_prompt: interface.default_prompt,
                }),
            dependencies: skill.dependencies.clone().map(|dependencies| {
                ProtocolSkillDependencies {
                    tools: dependencies
                        .tools
                        .into_iter()
                        .map(|tool| ProtocolSkillToolDependency {
                            r#type: tool.r#type,
                            value: tool.value,
                            description: tool.description,
                            transport: tool.transport,
                            command: tool.command,
                            url: tool.url,
                        })
                        .collect(),
                }
            }),
            path: skill.path_to_skills_md.clone(),
            scope: skill.scope,
            enabled: !disabled_paths.contains(&skill.path_to_skills_md),
        })
        .collect()
}

fn errors_to_info(errors: &[SkillError]) -> Vec<SkillErrorInfo> {
    errors
        .iter()
        .map(|err| SkillErrorInfo {
            path: err.path.to_path_buf(),
            message: err.message.clone(),
        })
        .collect()
}

fn is_action_map_projection_developer_item(item: &ResponseItem) -> bool {
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };
    if role != "developer" {
        return false;
    }
    content.iter().any(|entry| {
        matches!(
            entry,
            ContentItem::InputText { text }
                if text.contains("TaskSpaceMapProjectionR7V1:")
        )
    })
}

fn remove_taskspace_projection_items(items: Vec<ResponseItem>) -> Vec<ResponseItem> {
    items
        .into_iter()
        .filter(|item| !is_action_map_projection_developer_item(item))
        .collect()
}

fn remove_taskspace_map_handle_items(items: Vec<ResponseItem>) -> Vec<ResponseItem> {
    items
        .into_iter()
        .filter(|item| taskspace_map_handle_context(Some(item)).is_none())
        .collect()
}

fn latest_taskspace_projection_context(items: &[ResponseItem]) -> Option<&str> {
    items
        .iter()
        .rev()
        .find_map(|item| taskspace_projection_context(Some(item)))
}

fn taskspace_projection_context(item: Option<&ResponseItem>) -> Option<&str> {
    let ResponseItem::Message { role, content, .. } = item? else {
        return None;
    };
    if !matches!(role.as_str(), "developer" | "system" | "user") {
        return None;
    }
    content.iter().rev().find_map(|entry| match entry {
        ContentItem::InputText { text } | ContentItem::OutputText { text }
            if text.contains("TaskSpaceMapProjectionR7V1:") =>
        {
            Some(text.as_str())
        }
        _ => None,
    })
}

fn taskspace_map_handle_context(item: Option<&ResponseItem>) -> Option<&str> {
    let ResponseItem::Message { role, content, .. } = item? else {
        return None;
    };
    if !matches!(role.as_str(), "developer" | "system" | "user") {
        return None;
    }
    content.iter().rev().find_map(|entry| match entry {
        ContentItem::InputText { text } | ContentItem::OutputText { text }
            if text.contains("TaskSpaceMapHandleR7V1:") =>
        {
            Some(text.as_str())
        }
        _ => None,
    })
}

fn rewrite_taskspace_projection_items_for_append(items: &mut [ResponseItem]) {
    for item in items {
        if taskspace_projection_context(Some(item)).is_none() {
            continue;
        }
        let ResponseItem::Message { role, .. } = item else {
            continue;
        };
        if matches!(role.as_str(), "developer" | "system") {
            *role = "user".to_string();
        }
    }
}

use crate::memories::prompts::build_memory_tool_developer_instructions;

#[cfg(test)]
pub(crate) mod tests;
