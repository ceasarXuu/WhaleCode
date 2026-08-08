use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::Weak;
use std::sync::atomic::Ordering;

use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_state::SettleTaskSpaceActionRequest;
use codex_state::TaskSpaceMapWriteOutcome;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use super::record_map_revision;
use super::runtime_from_record;
use crate::action_map::rooted_dag;
use crate::session::session::Session;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TaskSpaceActionSettlementFact {
    pub(crate) map_id: String,
    pub(crate) outer_call_id: String,
    pub(crate) action_id: String,
    pub(crate) node_ids: Vec<String>,
    pub(crate) tool_name: String,
    pub(crate) outcome: TaskSpaceActionOutcome,
}

enum SettlementCommand {
    Settle(TaskSpaceActionSettlementFact),
    Barrier(oneshot::Sender<Result<(), String>>),
}

pub(crate) struct TaskSpaceActionSettlementQueue {
    sender: mpsc::UnboundedSender<SettlementCommand>,
    receiver: StdMutex<Option<mpsc::UnboundedReceiver<SettlementCommand>>>,
}

impl Default for TaskSpaceActionSettlementQueue {
    fn default() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: StdMutex::new(Some(receiver)),
        }
    }
}

impl TaskSpaceActionSettlementQueue {
    pub(crate) fn start(&self, session: Weak<Session>) {
        let receiver = self
            .receiver
            .lock()
            .expect("TaskSpace settlement receiver lock poisoned")
            .take();
        let Some(receiver) = receiver else {
            return;
        };
        tokio::spawn(run_settlement_worker(session, receiver));
    }

    fn enqueue(&self, fact: TaskSpaceActionSettlementFact) -> Result<(), String> {
        self.sender
            .send(SettlementCommand::Settle(fact))
            .map_err(|_| "TaskSpace Action settlement executor is unavailable.".to_string())
    }

    async fn barrier(&self) -> Result<(), String> {
        if self
            .receiver
            .lock()
            .expect("TaskSpace settlement receiver lock poisoned")
            .is_some()
        {
            return Ok(());
        }
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(SettlementCommand::Barrier(sender))
            .map_err(|_| "TaskSpace Action settlement executor is unavailable.".to_string())?;
        receiver.await.map_err(|_| {
            "TaskSpace Action settlement executor stopped before barrier.".to_string()
        })?
    }
}

impl Session {
    pub(crate) fn enqueue_taskspace_action_settlement(
        self: &Arc<Self>,
        fact: TaskSpaceActionSettlementFact,
    ) -> Result<(), String> {
        self.taskspace_action_settlements
            .start(Arc::downgrade(self));
        tracing::debug!(
            target: "codex_core::taskspace",
            event_name = "taskspace.action_settlement_queued",
            map_id = fact.map_id,
            outer_call_id = fact.outer_call_id,
            action_id = fact.action_id,
            tool = fact.tool_name,
            outcome = ?fact.outcome,
            "queued observed Tool outcome for canonical Map settlement"
        );
        self.taskspace_action_settlements.enqueue(fact)
    }

    pub(crate) async fn await_taskspace_action_settlements(&self) -> Result<(), String> {
        self.taskspace_action_settlements.barrier().await
    }

    pub(crate) async fn recover_taskspace_action_settlements(&self) -> Result<(), String> {
        if self
            .taskspace_action_recovery_scanned
            .load(Ordering::SeqCst)
        {
            return Ok(());
        }
        let result = self.recover_taskspace_action_settlements_once().await;
        if result.is_ok() {
            self.taskspace_action_recovery_scanned
                .store(true, Ordering::SeqCst);
        }
        result
    }

    async fn recover_taskspace_action_settlements_once(&self) -> Result<(), String> {
        let Some((map_id, pending)) = self.pending_taskspace_actions().await? else {
            return Ok(());
        };
        if pending.is_empty() {
            return Ok(());
        }
        let history = self.clone_history().await;
        let rollout_path = self
            .current_rollout_path()
            .await
            .map_err(|error| format!("TaskSpace settlement recovery path failed: {error}"))?;
        let mut recovered = BTreeSet::new();
        for item in history.raw_items().iter().rev() {
            let ResponseItem::FunctionCallOutput { output, .. } = item else {
                continue;
            };
            let Some(text) = output.text_content() else {
                continue;
            };
            let Some(envelope) = recovery_envelope(text, rollout_path.as_deref()).await? else {
                continue;
            };
            if envelope.map_id != map_id {
                continue;
            }
            for result in envelope.client_results {
                let Some(expected) = pending.get(&result.action_id) else {
                    continue;
                };
                if recovered.contains(&result.action_id) {
                    continue;
                }
                let outcome = parse_terminal_outcome(&result.outcome)?;
                if expected.tool_name != result.tool
                    || expected.node_ids.len() != 1
                    || expected.node_ids[0] != result.node_id
                {
                    return Err(format!(
                        "TaskSpace recovery attribution mismatch for Action `{}`.",
                        result.action_id
                    ));
                }
                self.taskspace_action_settlements
                    .enqueue(TaskSpaceActionSettlementFact {
                        map_id: map_id.clone(),
                        outer_call_id: envelope.outer_call_id.clone(),
                        action_id: result.action_id.clone(),
                        node_ids: expected.node_ids.clone(),
                        tool_name: expected.tool_name.clone(),
                        outcome,
                    })?;
                recovered.insert(result.action_id);
            }
        }
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.action_settlement_recovery_completed",
            map_id,
            pending_action_count = pending.len(),
            recovered_action_count = recovered.len(),
            "recovered persisted Tool outcomes without replaying Tools or rebuilding the Map"
        );
        Ok(())
    }

    async fn pending_taskspace_actions(
        &self,
    ) -> Result<Option<(String, BTreeMap<String, PendingAction>)>, String> {
        let state = self.state.lock().await;
        let Some(map) = state.action_map_runtime.canonical_map_for_store() else {
            return Ok(None);
        };
        let mut pending = BTreeMap::<String, PendingAction>::new();
        for node in &map.work_nodes {
            for action in &node.actions {
                if action.outcome != TaskSpaceActionOutcome::Pending {
                    continue;
                }
                let entry =
                    pending
                        .entry(action.action_id.clone())
                        .or_insert_with(|| PendingAction {
                            tool_name: action.tool_name.clone(),
                            node_ids: Vec::new(),
                        });
                if entry.tool_name != action.tool_name {
                    return Err(format!(
                        "TaskSpace pending Action `{}` has conflicting Tool identities.",
                        action.action_id
                    ));
                }
                entry.node_ids.push(node.node_id.clone());
            }
        }
        for action in pending.values_mut() {
            action.node_ids.sort();
        }
        Ok(Some((map.map_id, pending)))
    }

    async fn persist_taskspace_action_settlement(
        &self,
        fact: TaskSpaceActionSettlementFact,
    ) -> Result<(), String> {
        let _write_permit = self
            .taskspace_store_write_lock
            .acquire()
            .await
            .map_err(|_| "TaskSpace Store write serializer is closed.".to_string())?;
        let handle = {
            let mut state = self.state.lock().await;
            let Some(handle) = state.action_map_store_handle.clone() else {
                if state.action_map_runtime.mode() == MapRuntimeMode::Experiment {
                    #[cfg(not(test))]
                    return Err(
                        "TaskSpace operation requires a canonical Map Store handle.".to_string()
                    );
                    #[cfg(test)]
                    if self.services.state_db.is_some() {
                        return Err("TaskSpace operation requires a canonical Map Store handle."
                            .to_string());
                    }
                }
                settle_runtime_action(&mut state.action_map_runtime, &fact, self.conversation_id)?;
                return Ok(());
            };
            handle
        };
        if handle.map_id != fact.map_id {
            return Err(format!(
                "TaskSpace Action settlement targets Map `{}`, current Map is `{}`.",
                fact.map_id, handle.map_id
            ));
        }
        let mutation_id = settlement_mutation_id(&fact);
        let state_db = self.require_taskspace_state_db()?;
        let outcome = state_db
            .settle_taskspace_action_outcome(SettleTaskSpaceActionRequest {
                map_id: fact.map_id.clone(),
                commit_id: mutation_id.clone(),
                mutation_id: mutation_id.clone(),
                action_id: fact.action_id.clone(),
                node_ids: fact.node_ids.clone(),
                tool_name: fact.tool_name.clone(),
                outcome: fact.outcome,
                operation: "taskspace_exec_settle".to_string(),
                actor_thread_id: self.conversation_id,
            })
            .await
            .map_err(|error| format!("TaskSpace Action settlement failed: {error}"))?;
        let record = match outcome {
            TaskSpaceMapWriteOutcome::Applied(record)
            | TaskSpaceMapWriteOutcome::IdempotentReplay(record) => record,
            TaskSpaceMapWriteOutcome::Conflict { .. } => {
                return Err(
                    "TaskSpace Action settlement returned an impossible revision conflict."
                        .to_string(),
                );
            }
        };
        let runtime = runtime_from_record(&record).map_err(|error| error.to_string())?;
        self.install_store_record(&record, runtime).await?;
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.action_settlement_committed",
            map_id = record.map_id,
            outer_call_id = fact.outer_call_id,
            action_id = fact.action_id,
            tool = fact.tool_name,
            outcome = ?fact.outcome,
            store_revision = record.store_revision,
            map_revision = record_map_revision(&record),
            mutation_id,
            "committed observed Tool outcome to canonical TaskSpace Map"
        );
        Ok(())
    }
}

#[derive(Debug)]
struct PendingAction {
    tool_name: String,
    node_ids: Vec<String>,
}

#[derive(Deserialize)]
struct RecoveryEnvelope {
    kind: String,
    outer_call_id: String,
    map_id: String,
    #[serde(default)]
    client_results: Vec<RecoveryClientResult>,
}

#[derive(Deserialize)]
struct RecoveryClientResult {
    action_id: String,
    node_id: String,
    tool: String,
    outcome: String,
}

async fn recovery_envelope(
    text: &str,
    rollout_path: Option<&std::path::Path>,
) -> Result<Option<RecoveryEnvelope>, String> {
    let owned;
    let text = if text.starts_with("OutputReferenceV1:") {
        if !text.contains("taskspace_exec_result") {
            return Ok(None);
        }
        let output_ref = text
            .lines()
            .find_map(|line| line.strip_prefix("output_ref: "))
            .ok_or_else(|| "TaskSpace recovery output reference is missing.".to_string())?;
        let raw = crate::tools::output_reference::read_output_artifact_for_recovery(
            rollout_path,
            output_ref,
        )
        .await
        .map_err(|error| format!("TaskSpace recovery artifact read failed: {error}"))?;
        owned = String::from_utf8(raw)
            .map_err(|error| format!("TaskSpace recovery artifact is not UTF-8: {error}"))?;
        owned.as_str()
    } else {
        text
    };
    if !text.contains("\"kind\":\"taskspace_exec_result\"") {
        return Ok(None);
    }
    let envelope = serde_json::from_str::<RecoveryEnvelope>(text)
        .map_err(|error| format!("TaskSpace recovery feedback is malformed: {error}"))?;
    if envelope.kind != "taskspace_exec_result" {
        return Ok(None);
    }
    Ok(Some(envelope))
}

fn parse_terminal_outcome(value: &str) -> Result<TaskSpaceActionOutcome, String> {
    match value {
        "succeeded" => Ok(TaskSpaceActionOutcome::Succeeded),
        "failed" => Ok(TaskSpaceActionOutcome::Failed),
        "cancelled" => Ok(TaskSpaceActionOutcome::Cancelled),
        _ => Err(format!(
            "TaskSpace recovery result has non-terminal outcome `{value}`."
        )),
    }
}

async fn run_settlement_worker(
    session: Weak<Session>,
    mut receiver: mpsc::UnboundedReceiver<SettlementCommand>,
) {
    let mut first_error = None;
    while let Some(command) = receiver.recv().await {
        match command {
            SettlementCommand::Settle(fact) => {
                let Some(session) = session.upgrade() else {
                    break;
                };
                if let Err(error) = session.persist_taskspace_action_settlement(fact).await {
                    tracing::error!(
                        target: "codex_core::taskspace",
                        event_name = "taskspace.action_settlement_failed",
                        error = %error,
                        "failed to settle observed Tool outcome"
                    );
                    first_error.get_or_insert(error);
                }
            }
            SettlementCommand::Barrier(sender) => {
                let result = first_error.clone().map_or(Ok(()), Err);
                let failed = result.is_err();
                let _ = sender.send(result);
                tracing::debug!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.action_settlement_barrier_completed",
                    failed,
                    "completed TaskSpace Action settlement barrier"
                );
            }
        }
    }
}

fn settle_runtime_action(
    runtime: &mut crate::action_map::ActionMapRuntimeState,
    fact: &TaskSpaceActionSettlementFact,
    owner: codex_protocol::ThreadId,
) -> Result<(), String> {
    let current = runtime
        .canonical_map_for_store()
        .ok_or_else(|| "canonical Map disappeared during Tool settlement".to_string())?;
    let mut actual_nodes = current
        .work_nodes
        .iter()
        .filter(|node| {
            node.actions
                .iter()
                .any(|action| action.action_id == fact.action_id)
        })
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    actual_nodes.sort();
    let mut expected_nodes = fact.node_ids.clone();
    expected_nodes.sort();
    if actual_nodes != expected_nodes {
        return Err(format!(
            "TaskSpace Action `{}` node attribution mismatch.",
            fact.action_id
        ));
    }
    let commit =
        rooted_dag::settle_action(&current, &fact.action_id, &fact.tool_name, fact.outcome)
            .map_err(|error| format!("action settlement rejected: {error:?}"))?;
    runtime.restore_store_map(&fact.map_id, owner, Some(commit.map))
}

fn settlement_mutation_id(fact: &TaskSpaceActionSettlementFact) -> String {
    format!(
        "taskspace-settle/{}/{}/{}",
        fact.outer_call_id,
        fact.action_id,
        outcome_name(fact.outcome)
    )
}

fn outcome_name(outcome: TaskSpaceActionOutcome) -> &'static str {
    match outcome {
        TaskSpaceActionOutcome::Pending => "pending",
        TaskSpaceActionOutcome::Succeeded => "succeeded",
        TaskSpaceActionOutcome::Failed => "failed",
        TaskSpaceActionOutcome::Cancelled => "cancelled",
    }
}
