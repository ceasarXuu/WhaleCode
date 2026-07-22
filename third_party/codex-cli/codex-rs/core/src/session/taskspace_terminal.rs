use super::Session;
use super::TurnContext;
use crate::action_map::ActionMapTerminalOutcome;
use crate::action_map::snapshot_sha256;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeGraphRevisionCommittedEvent;
use codex_protocol::protocol::MapRuntimeTerminalCommittedEvent;
use codex_protocol::protocol::MapRuntimeTraceEventRecordedEvent;
use codex_protocol::protocol::RolloutItem;
use tracing::debug;
use tracing::error;
use tracing::info;

pub(crate) enum FinishActionMapError {
    Rejected(String),
    Persistence(String),
    Internal(String),
}

impl Session {
    pub(crate) async fn complete_active_work_then_end_action_map(
        &self,
        turn_context: &TurnContext,
        expected_revision: u64,
        current_node_id: String,
        final_summary: String,
        source_event_ref: String,
    ) -> Result<ActionMapTerminalOutcome, FinishActionMapError> {
        let (outcome, terminal_event, candidate) = {
            let state = self.state.lock().await;
            let mut candidate = state.action_map_runtime.clone();
            let (outcome, events) = candidate
                .complete_active_work_then_end_for_main(
                    self.conversation_id,
                    expected_revision,
                    current_node_id,
                    final_summary,
                    source_event_ref,
                )
                .map_err(FinishActionMapError::Rejected)?;
            let terminal_event = Self::terminal_commit_event(events, candidate.snapshot())
                .map_err(FinishActionMapError::Internal)?;
            (outcome, terminal_event, candidate)
        };
        self.install_terminal_candidate(turn_context, expected_revision, terminal_event, candidate)
            .await?;
        Ok(outcome)
    }

    pub(crate) async fn close_finish_with_no_active_work_action_map(
        &self,
        turn_context: &TurnContext,
        expected_revision: u64,
        final_summary: String,
    ) -> Result<ActionMapTerminalOutcome, FinishActionMapError> {
        let (outcome, terminal_event, candidate) = {
            let state = self.state.lock().await;
            let mut candidate = state.action_map_runtime.clone();
            let (outcome, events) = candidate
                .close_finish_with_no_active_work_for_main(
                    self.conversation_id,
                    expected_revision,
                    final_summary,
                )
                .map_err(FinishActionMapError::Rejected)?;
            let terminal_event = Self::terminal_commit_event(events, candidate.snapshot())
                .map_err(FinishActionMapError::Internal)?;
            (outcome, terminal_event, candidate)
        };
        self.install_terminal_candidate(turn_context, expected_revision, terminal_event, candidate)
            .await?;
        Ok(outcome)
    }

    async fn install_terminal_candidate(
        &self,
        turn_context: &TurnContext,
        expected_revision: u64,
        terminal_event: MapRuntimeTerminalCommittedEvent,
        candidate: crate::action_map::ActionMapRuntimeState,
    ) -> Result<(), FinishActionMapError> {
        let terminal_map = terminal_event.snapshot.map.as_ref().ok_or_else(|| {
            FinishActionMapError::Internal(
                "TaskSpace terminal transaction lost its canonical map before persistence. hard_state: terminal_transaction_invalid."
                    .to_string(),
            )
        })?;
        let terminal_map_id = terminal_map.id.clone();
        let terminal_revision = terminal_map.revision;
        self.persist_terminal_commit(&terminal_event)
            .await
            .map_err(FinishActionMapError::Persistence)?;
        {
            let mut state = self.state.lock().await;
            let live_snapshot = state.action_map_runtime.snapshot();
            let live_state_is_precommit = live_snapshot.map.as_ref().is_some_and(|map| {
                map.id == terminal_map_id && map.revision == expected_revision && !map.complete
            });
            if !live_state_is_precommit {
                error!(
                    target: "codex_core::taskspace",
                    map_id = terminal_map_id,
                    revision = terminal_revision,
                    live_map_id = ?live_snapshot.map.as_ref().map(|map| map.id.as_str()),
                    live_revision = ?live_snapshot.map.as_ref().map(|map| map.revision),
                    reason_code = "terminal_live_state_drift",
                    "taskspace_terminal_durable_state_overrides_live_drift"
                );
            }
            state.action_map_runtime = candidate;
            state.action_map_checkpoint.install(
                terminal_event.checkpoint_id.clone(),
                terminal_event.snapshot_sha256.clone(),
                terminal_event.snapshot.clone(),
            );
        }
        self.send_persisted_event(
            turn_context,
            EventMsg::MapRuntime(MapRuntimeEvent::TerminalCommitted(Box::new(terminal_event))),
        )
        .await;
        Ok(())
    }

    pub(super) fn terminal_commit_event(
        events: Vec<MapRuntimeEvent>,
        snapshot: ActionMapSnapshot,
    ) -> Result<MapRuntimeTerminalCommittedEvent, String> {
        let mut graph_revision = None;
        let mut trace_event = None;
        for event in events {
            match event {
                MapRuntimeEvent::GraphRevisionCommitted(event)
                    if matches!(
                        event.operation.as_str(),
                        "close_finish_with_no_active_work" | "complete_active_work_then_end"
                    ) && graph_revision.is_none() =>
                {
                    graph_revision = Some(event);
                }
                MapRuntimeEvent::TaskspaceTraceEventRecorded(event)
                    if event.kind == "terminal_committed" && trace_event.is_none() =>
                {
                    trace_event = Some(event);
                }
                _ => {
                    return Err(
                        "TaskSpace terminal transaction produced an unexpected event set. hard_state: terminal_transaction_invalid."
                            .to_string(),
                    );
                }
            }
        }
        let graph_revision: MapRuntimeGraphRevisionCommittedEvent = graph_revision.ok_or_else(|| {
            "TaskSpace terminal transaction is missing its graph revision. hard_state: terminal_transaction_invalid."
                .to_string()
        })?;
        let trace_event: MapRuntimeTraceEventRecordedEvent = trace_event.ok_or_else(|| {
            "TaskSpace terminal transaction is missing its trace event. hard_state: terminal_transaction_invalid."
                .to_string()
        })?;
        let map = snapshot.map.as_ref().ok_or_else(|| {
            "TaskSpace terminal transaction is missing its canonical map snapshot. hard_state: terminal_transaction_invalid."
                .to_string()
        })?;
        if !map.complete
            || map.id != graph_revision.map_id
            || map.revision != graph_revision.revision
            || trace_event.map_id != graph_revision.map_id
        {
            return Err(
                "TaskSpace terminal transaction identity does not match its canonical snapshot. hard_state: terminal_transaction_invalid."
                    .to_string(),
            );
        }
        let snapshot_sha256 = snapshot_sha256(&snapshot).map_err(|error| {
            format!(
                "TaskSpace terminal transaction snapshot is not serializable. hard_state: terminal_transaction_invalid. {error}"
            )
        })?;
        let checkpoint_id = format!("map-terminal-{}", &snapshot_sha256[..16]);
        Ok(MapRuntimeTerminalCommittedEvent {
            checkpoint_id,
            snapshot_sha256,
            snapshot,
            graph_revision,
            trace_event,
        })
    }

    async fn persist_terminal_commit(
        &self,
        event: &MapRuntimeTerminalCommittedEvent,
    ) -> Result<(), String> {
        let Some(live_thread) = self.live_thread() else {
            debug!(
                target: "codex_core::taskspace",
                map_id = event.graph_revision.map_id,
                revision = event.graph_revision.revision,
                "taskspace_terminal_persistence_unavailable"
            );
            return Ok(());
        };
        let item = RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::TerminalCommitted(
            Box::new(event.clone()),
        )));
        live_thread.append_items(&[item]).await.map_err(|error| {
            error!(
                target: "codex_core::taskspace",
                map_id = event.graph_revision.map_id,
                revision = event.graph_revision.revision,
                reason_code = "terminal_persistence_append_failed",
                %error,
                "taskspace_terminal_persistence_failed"
            );
            format!(
                "TaskSpace terminal transaction could not be queued for persistence. hard_state: terminal_persistence_failed. {error}"
            )
        })?;
        live_thread.persist().await.map_err(|error| {
            error!(
                target: "codex_core::taskspace",
                map_id = event.graph_revision.map_id,
                revision = event.graph_revision.revision,
                reason_code = "terminal_persistence_flush_failed",
                %error,
                "taskspace_terminal_persistence_failed"
            );
            format!(
                "TaskSpace terminal transaction could not cross its durability barrier. hard_state: terminal_persistence_failed. {error}"
            )
        })?;
        info!(
            target: "codex_core::taskspace",
            map_id = event.graph_revision.map_id,
            revision = event.graph_revision.revision,
            checkpoint_id = event.checkpoint_id,
            snapshot_sha256 = event.snapshot_sha256,
            "taskspace_terminal_transaction_persisted"
        );
        Ok(())
    }
}
