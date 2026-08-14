use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_state::EnqueueTaskSpacePendingProviderActionRequest;

use super::session::Session;
use crate::action_map::rooted_dag::ActionOutcome;
use crate::tools::taskspace_exec::TaskSpacePendingProviderActionFact;

pub(crate) const TASKSPACE_PENDING_PROVIDER_ACTIONS_MARKER: &str =
    "TaskSpacePendingProviderActionsR8V1:";

impl Session {
    pub(crate) async fn persist_pending_provider_actions(
        &self,
        facts: Vec<TaskSpacePendingProviderActionFact>,
    ) -> Result<(), String> {
        if facts.is_empty() {
            return Ok(());
        }
        let state_db = self.require_taskspace_state_db()?;
        for fact in facts {
            state_db
                .enqueue_taskspace_pending_provider_action(
                    EnqueueTaskSpacePendingProviderActionRequest {
                        action_id: fact.action_id.clone(),
                        origin_thread_id: self.conversation_id,
                        map_id: Some(fact.map_id.clone()),
                        provider_response_id: fact.provider_response_id.clone(),
                        provider_action_key: fact.provider_action_key.clone(),
                        tool_name: fact.tool.clone(),
                        outcome: protocol_outcome(fact.outcome)?,
                    },
                )
                .await
                .map_err(|error| {
                    format!("TaskSpace pending Provider Action persistence failed: {error}")
                })?;
            tracing::info!(
                target: "codex_core::taskspace",
                event_name = "taskspace.provider_action_pending",
                map_id = fact.map_id,
                action_id = fact.action_id,
                provider_response_id = fact.provider_response_id,
                tool = fact.tool,
                outcome = ?fact.outcome,
                "persisted Provider Action awaiting Agent node attribution"
            );
        }
        Ok(())
    }

    pub(crate) async fn pending_provider_actions_context(
        &self,
        map_id: &str,
    ) -> Result<Option<String>, String> {
        let pending = self
            .require_taskspace_state_db()?
            .load_taskspace_pending_provider_actions(self.conversation_id, Some(map_id))
            .await
            .map_err(|error| format!("TaskSpace pending Provider Action read failed: {error}"))?;
        if pending.is_empty() {
            return Ok(None);
        }
        let actions = pending
            .into_iter()
            .map(|action| {
                serde_json::json!({
                    "action_id": action.action_id,
                    "tool": action.tool_name,
                    "outcome": outcome_name(action.outcome),
                })
            })
            .collect::<Vec<_>>();
        let body = serde_json::to_string(&serde_json::json!({ "actions": actions }))
            .map_err(|error| format!("pending Provider Action rendering failed: {error}"))?;
        Ok(Some(format!(
            "{TASKSPACE_PENDING_PROVIDER_ACTIONS_MARKER}\n{body}\nTaskSpacePendingProviderActionsR8V1 end.\n"
        )))
    }
}

fn protocol_outcome(outcome: ActionOutcome) -> Result<TaskSpaceActionOutcome, String> {
    match outcome {
        ActionOutcome::Succeeded => Ok(TaskSpaceActionOutcome::Succeeded),
        ActionOutcome::Failed => Ok(TaskSpaceActionOutcome::Failed),
        ActionOutcome::Cancelled => Ok(TaskSpaceActionOutcome::Cancelled),
        ActionOutcome::Pending => {
            Err("Provider Action completed with a non-terminal outcome".to_string())
        }
    }
}

fn outcome_name(outcome: TaskSpaceActionOutcome) -> &'static str {
    match outcome {
        TaskSpaceActionOutcome::Pending => "pending",
        TaskSpaceActionOutcome::Succeeded => "succeeded",
        TaskSpaceActionOutcome::Failed => "failed",
        TaskSpaceActionOutcome::Cancelled => "cancelled",
    }
}
