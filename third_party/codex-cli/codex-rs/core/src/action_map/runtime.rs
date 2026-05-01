use codex_protocol::protocol::MapRuntimeMode;

#[derive(Debug, Clone)]
pub(crate) struct ActionMapRuntimeState {
    mode: MapRuntimeMode,
    pending_transition_notice: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SetMapRuntimeModeOutcome {
    pub(crate) previous_mode: MapRuntimeMode,
    pub(crate) current_mode: MapRuntimeMode,
    pub(crate) changed: bool,
}

impl Default for ActionMapRuntimeState {
    fn default() -> Self {
        Self {
            mode: MapRuntimeMode::Standard,
            pending_transition_notice: None,
        }
    }
}

impl ActionMapRuntimeState {
    #[cfg(test)]
    pub(crate) fn mode(&self) -> MapRuntimeMode {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: MapRuntimeMode) -> SetMapRuntimeModeOutcome {
        let previous_mode = self.mode;
        self.mode = mode;
        if previous_mode != mode {
            self.pending_transition_notice = Some(transition_notice(previous_mode, mode));
        }
        SetMapRuntimeModeOutcome {
            previous_mode,
            current_mode: self.mode,
            changed: previous_mode != self.mode,
        }
    }

    pub(crate) fn restore_mode(&mut self, mode: MapRuntimeMode) {
        self.mode = mode;
        self.pending_transition_notice = None;
    }

    pub(crate) fn take_pending_transition_notice(&mut self) -> Option<String> {
        self.pending_transition_notice.take()
    }
}

fn transition_notice(previous_mode: MapRuntimeMode, current_mode: MapRuntimeMode) -> String {
    match (previous_mode, current_mode) {
        (MapRuntimeMode::Standard, MapRuntimeMode::Experiment) => {
            "Action Map experiment mode is now active.\n\
Previous standard-mode conversation remains background context only.\n\
Before taking multi-agent action, create or bind an Action Map and a ready node.\n\
Future subagent work must be map/node driven."
                .to_string()
        }
        (MapRuntimeMode::Experiment, MapRuntimeMode::Standard) => {
            "Action Map experiment mode is now disabled.\n\
Existing maps, nodes, leases, and results remain historical context only.\n\
Do not continue maintaining the map, require node binding, or follow map-driven protocol unless the user enables experiment mode again.\n\
Continue with the standard Codex multi-agent behavior."
                .to_string()
        }
        _ => format!(
            "Action Map runtime mode changed from {previous_mode} to {current_mode}."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_standard() {
        let state = ActionMapRuntimeState::default();

        assert_eq!(state.mode(), MapRuntimeMode::Standard);
    }

    #[test]
    fn set_mode_reports_whether_state_changed() {
        let mut state = ActionMapRuntimeState::default();

        let changed = state.set_mode(MapRuntimeMode::Experiment);
        assert_eq!(changed.previous_mode, MapRuntimeMode::Standard);
        assert_eq!(changed.current_mode, MapRuntimeMode::Experiment);
        assert!(changed.changed);
        assert!(
            state
                .take_pending_transition_notice()
                .expect("transition notice")
                .contains("experiment mode is now active")
        );

        let unchanged = state.set_mode(MapRuntimeMode::Experiment);
        assert_eq!(unchanged.previous_mode, MapRuntimeMode::Experiment);
        assert_eq!(unchanged.current_mode, MapRuntimeMode::Experiment);
        assert!(!unchanged.changed);
        assert!(state.take_pending_transition_notice().is_none());
    }

    #[test]
    fn restore_mode_does_not_create_transition_notice() {
        let mut state = ActionMapRuntimeState::default();

        state.restore_mode(MapRuntimeMode::Experiment);

        assert_eq!(state.mode(), MapRuntimeMode::Experiment);
        assert!(state.take_pending_transition_notice().is_none());
    }
}
