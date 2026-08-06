use std::collections::HashMap;

use codex_protocol::ThreadId;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;

use crate::action_map::map::ActionMapId;
use crate::action_map::map::ActionMapInstance;
use crate::action_map::rooted_dag;

use super::types::SetTaskSpaceModeOutcome;
use super::types::TaskSpaceModeTransition;

#[derive(Debug, Clone)]
pub(crate) struct ActionMapRuntimeState {
    pub(crate) mode: MapRuntimeMode,
    pub(crate) active_map_id: Option<ActionMapId>,
    pub(crate) maps: HashMap<ActionMapId, ActionMapInstance>,
}

impl Default for ActionMapRuntimeState {
    fn default() -> Self {
        Self {
            mode: MapRuntimeMode::Standard,
            active_map_id: None,
            maps: HashMap::new(),
        }
    }
}

impl ActionMapRuntimeState {
    pub(crate) fn set_mode_for_session(
        &mut self,
        mode: MapRuntimeMode,
        owner_session_id: ThreadId,
    ) -> (SetTaskSpaceModeOutcome, Vec<MapRuntimeEvent>) {
        let previous_mode = self.mode;
        self.mode = mode;
        if mode == MapRuntimeMode::Experiment && self.active_map_id.is_none() {
            self.active_map_id = Some(format!("map-{owner_session_id}"));
        }
        let changed = previous_mode != mode;
        let events = changed
            .then_some(MapRuntimeEvent::ModeChanged(MapRuntimeModeChangedEvent {
                previous_mode,
                current_mode: mode,
            }))
            .into_iter()
            .collect();
        (
            SetTaskSpaceModeOutcome {
                mode: TaskSpaceModeTransition {
                    previous_mode,
                    current_mode: mode,
                    changed,
                },
                active_map_id: self.active_map_id.clone(),
            },
            events,
        )
    }

    pub(crate) fn mode(&self) -> MapRuntimeMode {
        self.mode
    }

    #[cfg(test)]
    pub(crate) fn active_map_id(&self) -> Option<&str> {
        self.active_map_id.as_deref()
    }

    pub(crate) fn active_map(&self) -> Option<&ActionMapInstance> {
        self.active_map_id
            .as_deref()
            .and_then(|map_id| self.maps.get(map_id))
    }

    pub(crate) fn export_canonical_map(&self) -> Option<TaskSpaceCanonicalMap> {
        self.active_map().map(|map| map.canonical_map().clone())
    }

    pub(crate) fn canonical_map_for_store(&self) -> Option<TaskSpaceCanonicalMap> {
        self.export_canonical_map()
    }

    fn restore_canonical_map(&mut self, map: TaskSpaceCanonicalMap) {
        let map_id = map.map_id.clone();
        self.maps
            .insert(map_id.clone(), ActionMapInstance::new(map));
        self.active_map_id = Some(map_id);
    }

    pub(crate) fn restore_store_map(
        &mut self,
        map_id: &str,
        _owner_session_id: ThreadId,
        canonical_map: Option<TaskSpaceCanonicalMap>,
    ) -> Result<(), String> {
        match canonical_map {
            Some(map) => {
                if map.map_id != map_id {
                    return Err(format!(
                        "TaskSpace Store map `{map_id}` does not match canonical map `{}`.",
                        map.map_id
                    ));
                }
                let violations = rooted_dag::validate(&map);
                if !violations.is_empty() {
                    let details = violations
                        .iter()
                        .map(|violation| {
                            if violation.subjects.is_empty() {
                                violation.code.as_str().to_string()
                            } else {
                                format!(
                                    "{}({})",
                                    violation.code.as_str(),
                                    violation.subjects.join(",")
                                )
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(";");
                    return Err(format!(
                        "TaskSpace Store map `{map_id}` canonical Map is invalid: {details}"
                    ));
                }
                self.mode = MapRuntimeMode::Experiment;
                self.restore_canonical_map(map);
            }
            None => {
                self.mode = MapRuntimeMode::Experiment;
                self.maps.remove(map_id);
                self.active_map_id = Some(map_id.to_string());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_map::rooted_dag::NodeState;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::rooted_dag::new_map;

    fn valid_map(map_id: &str) -> TaskSpaceCanonicalMap {
        new_map(
            map_id.into(),
            map_node("root", "solve", NodeState::InFlight, "", vec![]),
            vec![map_node(
                "work",
                "work",
                NodeState::Ready,
                "",
                vec!["root".into()],
            )],
            map_node(
                "finish",
                "finish",
                NodeState::Waiting,
                "",
                vec!["work".into()],
            ),
        )
    }

    #[test]
    fn restore_store_map_accepts_valid_canonical_map() {
        let owner = ThreadId::new();
        let map = valid_map("map-1");
        let mut runtime = ActionMapRuntimeState::default();

        runtime
            .restore_store_map("map-1", owner, Some(map.clone()))
            .unwrap();

        assert_eq!(runtime.mode(), MapRuntimeMode::Experiment);
        assert_eq!(runtime.active_map_id(), Some("map-1"));
        assert_eq!(runtime.canonical_map_for_store(), Some(map));
    }

    #[test]
    fn restore_rejects_invalid_map_without_mutating_current_state() {
        let owner = ThreadId::new();
        let existing = valid_map("existing");
        let mut runtime = ActionMapRuntimeState::default();
        runtime
            .restore_store_map("existing", owner, Some(existing.clone()))
            .unwrap();

        let mut invalid = valid_map("invalid");
        invalid.work_nodes[0].parents = vec!["finish".into()];
        let error = runtime
            .restore_store_map("invalid", owner, Some(invalid))
            .unwrap_err();

        assert!(error.contains("invalid"));
        assert_eq!(runtime.active_map_id(), Some("existing"));
        assert_eq!(runtime.canonical_map_for_store(), Some(existing));
    }

    #[test]
    fn empty_store_identity_requires_bootstrap() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();
        runtime.restore_store_map("map-empty", owner, None).unwrap();
        assert_eq!(runtime.active_map_id(), Some("map-empty"));
        assert!(runtime.canonical_map_for_store().is_none());
    }
}
