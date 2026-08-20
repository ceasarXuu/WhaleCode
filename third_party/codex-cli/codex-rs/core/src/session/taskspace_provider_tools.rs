use super::session::Session;
use crate::action_map::rooted_dag::ProviderToolAction;
use crate::action_map::rooted_dag::record_provider_tool_actions;
use crate::tools::taskspace_exec::TaskSpaceProviderActionFact;

#[derive(Debug)]
enum ProviderActionRecordOutcome {
    Recorded { map_id: String, revision: u64 },
    Escaped { reason: String },
}

impl Session {
    pub(crate) async fn record_provider_actions(
        &self,
        facts: Vec<TaskSpaceProviderActionFact>,
    ) -> Result<(), String> {
        if facts.is_empty() {
            return Ok(());
        }
        let expected_map_id = facts[0].map_id.clone();
        let closure_map_id = expected_map_id.clone();
        let actions = facts
            .iter()
            .map(|fact| ProviderToolAction {
                action_id: fact.action_id.clone(),
                tool_name: fact.tool.clone(),
                outcome: fact.outcome,
            })
            .collect::<Vec<_>>();
        let failed_action_count = facts
            .iter()
            .filter(|fact| {
                matches!(
                    fact.outcome,
                    crate::action_map::rooted_dag::ActionOutcome::Failed
                        | crate::action_map::rooted_dag::ActionOutcome::Cancelled
                )
            })
            .count();
        let (outcome, _) = self
            .mutate_canonical_action_map("record_provider_actions", move |runtime, owner| {
                let Some(current) = runtime.canonical_map_for_store() else {
                    return (
                        ProviderActionRecordOutcome::Escaped {
                            reason: "TaskSpace Map is not initialized".to_string(),
                        },
                        Vec::new(),
                    );
                };
                if current.map_id != closure_map_id {
                    return (
                        ProviderActionRecordOutcome::Escaped {
                            reason: format!("response Map `{closure_map_id}` is no longer active"),
                        },
                        Vec::new(),
                    );
                }
                match record_provider_tool_actions(&current, &actions) {
                    Ok(commit) => {
                        let revision = commit.map.revision;
                        match runtime.restore_store_map(&closure_map_id, owner, Some(commit.map)) {
                            Ok(()) => (
                                ProviderActionRecordOutcome::Recorded {
                                    map_id: closure_map_id,
                                    revision,
                                },
                                Vec::new(),
                            ),
                            Err(reason) => {
                                (ProviderActionRecordOutcome::Escaped { reason }, Vec::new())
                            }
                        }
                    }
                    Err(rejection) => (
                        ProviderActionRecordOutcome::Escaped {
                            reason: rejection
                                .violations
                                .iter()
                                .map(|violation| violation.code.as_str())
                                .collect::<Vec<_>>()
                                .join(","),
                        },
                        Vec::new(),
                    ),
                }
            })
            .await?;

        match outcome {
            ProviderActionRecordOutcome::Recorded { map_id, revision } => {
                tracing::info!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.provider_actions_recorded",
                    map_id,
                    map_revision = revision,
                    provider_action_count = facts.len(),
                    provider_failed_action_count = failed_action_count,
                );
            }
            ProviderActionRecordOutcome::Escaped { reason } => {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.provider_actions_escaped",
                    map_id = expected_map_id,
                    provider_action_count = facts.len(),
                    provider_failed_action_count = failed_action_count,
                    reason,
                );
            }
        }
        Ok(())
    }
}
