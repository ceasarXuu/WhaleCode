//! Session-wide mutable state.

use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::models::ResponseItem;
use codex_sandboxing::policy_transforms::merge_permission_profiles;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Deref;

use crate::action_map::ActionMapCheckpointState;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::ProjectionCursor;
use crate::action_map::TaskSpaceEvent;
use crate::action_map::TaskSpaceEventStore;
use crate::context_manager::ContextManager;
use crate::session::PreviousTurnSettings;
use crate::session::session::SessionConfiguration;
use crate::session_startup_prewarm::SessionStartupPrewarmHandle;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::TurnContextItem;
use codex_utils_output_truncation::TruncationPolicy;

/// Persistent, session-scoped state previously stored directly on `Session`.
pub(crate) struct SessionState {
    pub(crate) session_configuration: SessionConfiguration,
    pub(crate) history: ContextManager,
    pub(crate) latest_rate_limits: Option<RateLimitSnapshot>,
    pub(crate) server_reasoning_included: bool,
    pub(crate) dependency_env: HashMap<String, String>,
    pub(crate) mcp_dependency_prompted: HashSet<String>,
    /// Settings used by the latest regular user turn, used for turn-to-turn
    /// model/realtime handling on subsequent regular turns (including full-context
    /// reinjection after resume or `/compact`).
    previous_turn_settings: Option<PreviousTurnSettings>,
    /// Startup prewarmed session prepared during session initialization.
    pub(crate) startup_prewarm: Option<SessionStartupPrewarmHandle>,
    pub(crate) active_connector_selection: HashSet<String>,
    pub(crate) pending_session_start_source: Option<codex_hooks::SessionStartSource>,
    pub(crate) action_map_runtime: ActionMapRuntimeState,
    pub(crate) action_map_checkpoint: ActionMapCheckpointState,
    pub(crate) taskspace_events: TaskSpaceEventStore,
    pub(crate) taskspace_projection_cursor: ProjectionCursor,
    granted_permissions: Option<AdditionalPermissionProfile>,
    next_turn_is_first: bool,
}

impl SessionState {
    /// Create a new session state mirroring previous `State::default()` semantics.
    pub(crate) fn new(session_configuration: SessionConfiguration) -> Self {
        let history = ContextManager::new();
        Self {
            session_configuration,
            history,
            latest_rate_limits: None,
            server_reasoning_included: false,
            dependency_env: HashMap::new(),
            mcp_dependency_prompted: HashSet::new(),
            previous_turn_settings: None,
            startup_prewarm: None,
            active_connector_selection: HashSet::new(),
            pending_session_start_source: None,
            action_map_runtime: ActionMapRuntimeState::default(),
            action_map_checkpoint: ActionMapCheckpointState::default(),
            taskspace_events: TaskSpaceEventStore::new(),
            taskspace_projection_cursor: ProjectionCursor::default(),
            granted_permissions: None,
            next_turn_is_first: true,
        }
    }

    // History helpers
    pub(crate) fn record_items<I>(
        &mut self,
        items: I,
        policy: TruncationPolicy,
    ) -> Vec<TaskSpaceEvent>
    where
        I: IntoIterator,
        I::Item: std::ops::Deref<Target = ResponseItem>,
    {
        let items = items
            .into_iter()
            .map(|item| item.deref().clone())
            .collect::<Vec<_>>();
        if self.action_map_runtime.mode() != MapRuntimeMode::Experiment {
            self.history.record_items(items.iter(), policy);
            return Vec::new();
        }
        let current_node_id = self
            .action_map_runtime
            .context_owner_node_id()
            .map(str::to_string);
        let mut recorded = Vec::new();
        for item in &items {
            match self.taskspace_events.record_item(
                item,
                current_node_id.as_deref(),
                None,
                chrono::Utc::now().timestamp_millis(),
            ) {
                Ok(event) => recorded.push(event),
                Err(error) => panic!("TaskSpace canonical event record failed: {error}"),
            }
        }
        if let Some(identity) = ProjectionCursor::from_items(&items).last_emitted {
            self.taskspace_projection_cursor.last_emitted = Some(identity);
        }
        recorded
    }

    pub(crate) fn record_taskspace_child_item(
        &mut self,
        item: &ResponseItem,
        parent_call_id: String,
    ) -> Result<TaskSpaceEvent, String> {
        if self.action_map_runtime.mode() != MapRuntimeMode::Experiment {
            return Err("TaskSpace child items require TaskSpace mode.".to_string());
        }
        self.taskspace_events
            .record_item(
                item,
                self.action_map_runtime.context_owner_node_id(),
                Some(parent_call_id),
                chrono::Utc::now().timestamp_millis(),
            )
            .map_err(|error| format!("TaskSpace canonical child event record failed: {error}"))
    }

    pub(crate) fn previous_turn_settings(&self) -> Option<PreviousTurnSettings> {
        self.previous_turn_settings.clone()
    }
    pub(crate) fn set_previous_turn_settings(
        &mut self,
        previous_turn_settings: Option<PreviousTurnSettings>,
    ) {
        self.previous_turn_settings = previous_turn_settings;
    }

    pub(crate) fn set_next_turn_is_first(&mut self, value: bool) {
        self.next_turn_is_first = value;
    }

    pub(crate) fn take_next_turn_is_first(&mut self) -> bool {
        let is_first_turn = self.next_turn_is_first;
        self.next_turn_is_first = false;
        is_first_turn
    }

    pub(crate) fn clone_history(&self) -> ContextManager {
        let mut history = self.history.clone();
        if self.action_map_runtime.mode() == MapRuntimeMode::Experiment {
            history.replace(self.taskspace_events.linearize());
        }
        history
    }

    pub(crate) fn replace_history(
        &mut self,
        items: Vec<ResponseItem>,
        reference_context_item: Option<TurnContextItem>,
    ) {
        if self.action_map_runtime.mode() == MapRuntimeMode::Experiment {
            let mut store = TaskSpaceEventStore::new();
            let current_node_id = self.action_map_runtime.context_owner_node_id();
            for item in &items {
                store
                    .record_item(
                        item,
                        current_node_id,
                        None,
                        chrono::Utc::now().timestamp_millis(),
                    )
                    .expect("compacted TaskSpace history must be encodable");
            }
            self.taskspace_events = store;
            self.taskspace_projection_cursor =
                ProjectionCursor::from_items(&self.taskspace_events.linearize());
            self.history.replace(Vec::new());
        } else {
            self.history.replace(items);
        }
        self.history
            .set_reference_context_item(reference_context_item);
    }

    pub(crate) fn replace_compacted_history(
        &mut self,
        items: Vec<ResponseItem>,
        reference_context_item: Option<TurnContextItem>,
    ) -> Vec<TaskSpaceEvent> {
        if self.action_map_runtime.mode() != MapRuntimeMode::Experiment {
            self.history.replace(items);
            self.history
                .set_reference_context_item(reference_context_item);
            return Vec::new();
        }
        let checkpoint = self
            .taskspace_events
            .install_compaction_checkpoint(items, chrono::Utc::now().timestamp_millis())
            .expect("TaskSpace compaction checkpoint must be valid");
        self.taskspace_projection_cursor = ProjectionCursor::default();
        self.history.replace(Vec::new());
        self.history
            .set_reference_context_item(reference_context_item);
        vec![checkpoint]
    }

    pub(crate) fn activate_taskspace_context(&mut self) -> Vec<TaskSpaceEvent> {
        if !self.taskspace_events.is_empty() {
            return self.taskspace_events.events().to_vec();
        }
        let items = self.history.raw_items().to_vec();
        self.history.replace(Vec::new());
        let current_node_id = self
            .action_map_runtime
            .context_owner_node_id()
            .map(str::to_string);
        for item in &items {
            self.taskspace_events
                .record_item(
                    item,
                    current_node_id.as_deref(),
                    None,
                    chrono::Utc::now().timestamp_millis(),
                )
                .expect("existing history must be encodable as TaskSpace events");
        }
        self.taskspace_projection_cursor =
            ProjectionCursor::from_items(&self.taskspace_events.linearize());
        self.taskspace_events.events().to_vec()
    }

    pub(crate) fn deactivate_taskspace_context(&mut self) -> Vec<ResponseItem> {
        let items = self.taskspace_events.take_linearized();
        self.history.replace(items.clone());
        self.taskspace_projection_cursor = ProjectionCursor::default();
        items
    }

    pub(crate) fn restore_context(
        &mut self,
        history_items: Vec<ResponseItem>,
        taskspace_events: Vec<TaskSpaceEvent>,
        reference_context_item: Option<TurnContextItem>,
    ) {
        self.history.replace(history_items);
        self.history
            .set_reference_context_item(reference_context_item);
        self.taskspace_events = TaskSpaceEventStore::restore(taskspace_events)
            .expect("reconstructed TaskSpace events must be valid");
        self.taskspace_projection_cursor =
            ProjectionCursor::from_items(&self.taskspace_events.linearize());
    }

    pub(crate) fn restore_subagent_fork_context(
        &mut self,
        history_items: Vec<ResponseItem>,
        taskspace_events: Vec<TaskSpaceEvent>,
        reference_context_item: Option<TurnContextItem>,
    ) {
        let mut store = TaskSpaceEventStore::restore(taskspace_events)
            .expect("forked TaskSpace events must be valid");
        let mut items = history_items;
        items.extend(store.take_linearized());
        self.history.replace(items);
        self.history
            .set_reference_context_item(reference_context_item);
        self.taskspace_events = TaskSpaceEventStore::new();
        self.action_map_runtime = ActionMapRuntimeState::default();
        self.taskspace_projection_cursor = ProjectionCursor::default();
    }

    pub(crate) fn mutate_action_map<T>(
        &mut self,
        operation: impl FnOnce(&mut ActionMapRuntimeState) -> Result<(T, Vec<MapRuntimeEvent>), String>,
    ) -> Result<(T, Vec<MapRuntimeEvent>), String> {
        operation(&mut self.action_map_runtime)
    }

    pub(crate) fn set_token_info(&mut self, info: Option<TokenUsageInfo>) {
        self.history.set_token_info(info);
    }

    pub(crate) fn set_reference_context_item(&mut self, item: Option<TurnContextItem>) {
        self.history.set_reference_context_item(item);
    }

    pub(crate) fn reference_context_item(&self) -> Option<TurnContextItem> {
        self.history.reference_context_item()
    }

    // Token/rate limit helpers
    pub(crate) fn update_token_info_from_usage(
        &mut self,
        usage: &TokenUsage,
        model_context_window: Option<i64>,
    ) {
        self.history.update_token_info(usage, model_context_window);
    }

    pub(crate) fn token_info(&self) -> Option<TokenUsageInfo> {
        self.history.token_info()
    }

    pub(crate) fn set_rate_limits(&mut self, snapshot: RateLimitSnapshot) {
        self.latest_rate_limits = Some(merge_rate_limit_fields(
            self.latest_rate_limits.as_ref(),
            snapshot,
        ));
    }

    pub(crate) fn token_info_and_rate_limits(
        &self,
    ) -> (Option<TokenUsageInfo>, Option<RateLimitSnapshot>) {
        (self.token_info(), self.latest_rate_limits.clone())
    }

    pub(crate) fn set_token_usage_full(&mut self, context_window: i64) {
        self.history.set_token_usage_full(context_window);
    }

    pub(crate) fn get_total_token_usage(&self, server_reasoning_included: bool) -> i64 {
        self.history
            .get_total_token_usage(server_reasoning_included)
    }

    pub(crate) fn set_server_reasoning_included(&mut self, included: bool) {
        self.server_reasoning_included = included;
    }

    pub(crate) fn server_reasoning_included(&self) -> bool {
        self.server_reasoning_included
    }

    pub(crate) fn record_mcp_dependency_prompted<I>(&mut self, names: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.mcp_dependency_prompted.extend(names);
    }

    pub(crate) fn mcp_dependency_prompted(&self) -> HashSet<String> {
        self.mcp_dependency_prompted.clone()
    }

    pub(crate) fn set_dependency_env(&mut self, values: HashMap<String, String>) {
        for (key, value) in values {
            self.dependency_env.insert(key, value);
        }
    }

    pub(crate) fn dependency_env(&self) -> HashMap<String, String> {
        self.dependency_env.clone()
    }

    pub(crate) fn set_session_startup_prewarm(
        &mut self,
        startup_prewarm: SessionStartupPrewarmHandle,
    ) {
        self.startup_prewarm = Some(startup_prewarm);
    }

    pub(crate) fn take_session_startup_prewarm(&mut self) -> Option<SessionStartupPrewarmHandle> {
        self.startup_prewarm.take()
    }

    // Adds connector IDs to the active set and returns the merged selection.
    pub(crate) fn merge_connector_selection<I>(&mut self, connector_ids: I) -> HashSet<String>
    where
        I: IntoIterator<Item = String>,
    {
        self.active_connector_selection.extend(connector_ids);
        self.active_connector_selection.clone()
    }

    // Returns the current connector selection tracked on session state.
    pub(crate) fn get_connector_selection(&self) -> HashSet<String> {
        self.active_connector_selection.clone()
    }

    // Removes all currently tracked connector selections.
    pub(crate) fn clear_connector_selection(&mut self) {
        self.active_connector_selection.clear();
    }

    pub(crate) fn set_pending_session_start_source(
        &mut self,
        value: Option<codex_hooks::SessionStartSource>,
    ) {
        self.pending_session_start_source = value;
    }

    pub(crate) fn take_pending_session_start_source(
        &mut self,
    ) -> Option<codex_hooks::SessionStartSource> {
        self.pending_session_start_source.take()
    }

    pub(crate) fn record_granted_permissions(&mut self, permissions: AdditionalPermissionProfile) {
        self.granted_permissions =
            merge_permission_profiles(self.granted_permissions.as_ref(), Some(&permissions));
    }

    pub(crate) fn granted_permissions(&self) -> Option<AdditionalPermissionProfile> {
        self.granted_permissions.clone()
    }
}

// Sometimes new snapshots don't include credits or plan information.
// Preserve those from the previous snapshot when missing. For `limit_id`, treat
// missing values as the default `"codex"` bucket.
fn merge_rate_limit_fields(
    previous: Option<&RateLimitSnapshot>,
    mut snapshot: RateLimitSnapshot,
) -> RateLimitSnapshot {
    if snapshot.limit_id.is_none() {
        snapshot.limit_id = Some("codex".to_string());
    }
    if snapshot.credits.is_none() {
        snapshot.credits = previous.and_then(|prior| prior.credits.clone());
    }
    if snapshot.plan_type.is_none() {
        snapshot.plan_type = previous.and_then(|prior| prior.plan_type);
    }
    snapshot
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
