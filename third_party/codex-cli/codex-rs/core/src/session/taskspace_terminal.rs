use super::Session;
use super::TurnContext;
use crate::action_map::ActionMapTerminalOutcome;

pub(crate) enum FinishActionMapError {
    Rejected(String),
    Persistence(String),
}

impl Session {
    pub(crate) async fn finish_action_map(
        &self,
        turn_context: &TurnContext,
        expected_revision: u64,
        terminal_node_id: String,
        final_summary: String,
        source_event_ref: String,
    ) -> Result<ActionMapTerminalOutcome, FinishActionMapError> {
        let (outcome, events) = self
            .mutate_canonical_action_map("finish_map", |runtime, principal| {
                match runtime.finish_map_for_main(
                    principal,
                    expected_revision,
                    terminal_node_id,
                    final_summary,
                    source_event_ref,
                ) {
                    Ok((outcome, events)) => (Ok(outcome), events),
                    Err(error) => (Err(error), Vec::new()),
                }
            })
            .await
            .map_err(FinishActionMapError::Persistence)?;
        let outcome = outcome.map_err(FinishActionMapError::Rejected)?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(outcome)
    }
}
