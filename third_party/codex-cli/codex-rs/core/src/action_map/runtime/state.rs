use std::collections::HashMap;

use codex_protocol::ThreadId;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;

use crate::action_map::map::ActionMapId;
use crate::action_map::map::ActionMapInstance;

use super::types::ActionMapControlState;
use super::types::SetTaskSpaceModeOutcome;
use super::types::TaskSpaceModeTransition;

#[derive(Debug, Clone)]
pub(crate) struct ActionMapRuntimeState {
    pub(crate) mode: MapRuntimeMode,
    pub(crate) active_map_id: Option<ActionMapId>,
    pub(crate) maps: HashMap<ActionMapId, ActionMapInstance>,
}

#[cfg(test)]
mod tests {
    use codex_protocol::ThreadId;
    use codex_protocol::protocol::MapRuntimeMode;

    use crate::action_map::rooted_dag::MapEdge;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::rooted_dag::new_map;
    use crate::action_map::runtime::ActionMapRuntimeState;

    #[test]
    fn restore_store_map_accepts_canonical_map_without_snapshot() {
        let owner = ThreadId::new();
        let map = new_map(
            "map-1".into(),
            map_node("root", "solve", vec![]),
            vec![map_node("work", "work", vec![])],
            map_node("finish", "", vec![]),
            vec![
                MapEdge {
                    from: "root".into(),
                    to: "work".into(),
                },
                MapEdge {
                    from: "work".into(),
                    to: "finish".into(),
                },
            ],
        );
        let mut runtime = ActionMapRuntimeState::default();

        runtime
            .restore_store_map("map-1", owner, Some(map.clone()))
            .expect("canonical restore");

        assert_eq!(runtime.mode(), MapRuntimeMode::Experiment);
        assert_eq!(runtime.active_map_id(), Some("map-1"));
        assert_eq!(runtime.canonical_map_for_store(), Some(map));
    }

    #[test]
    fn experiment_mode_retains_empty_map_identity_without_canonical_map() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();

        runtime.set_mode_for_session(MapRuntimeMode::Experiment, owner);

        assert_eq!(runtime.mode(), MapRuntimeMode::Experiment);
        assert_eq!(
            runtime.active_map_id(),
            Some(format!("map-{owner}").as_str())
        );
        assert_eq!(runtime.canonical_map_for_store(), None);
    }

    #[test]
    fn restore_store_map_none_activates_explicit_empty_identity() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();

        runtime
            .restore_store_map("store-map-7", owner, None)
            .expect("empty store identity restore");

        assert_eq!(runtime.mode(), MapRuntimeMode::Experiment);
        assert_eq!(runtime.active_map_id(), Some("store-map-7"));
        assert_eq!(runtime.canonical_map_for_store(), None);
    }
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

    pub(crate) fn active_map_mut(&mut self) -> Option<&mut ActionMapInstance> {
        let map_id = self.active_map_id.clone()?;
        self.maps.get_mut(&map_id)
    }

    pub(crate) fn control_state(&self, map_id_hint: Option<&str>) -> Option<ActionMapControlState> {
        let map = match map_id_hint {
            Some(map_id) => self.maps.get(map_id)?,
            None => self.active_map()?,
        };
        Some(ActionMapControlState {
            map_id: map.map_id.clone(),
            owner_session_id: map.owner_session_id,
            revision: map.canonical_map().revision,
            complete: map.is_complete(),
            ready_work_node_count: map.ready_work_node_count(),
            inflight_work_node_count: map.inflight_work_node_count(),
            completed_work_node_count: map.completed_work_node_count(),
            finish_ready: map.finish_ready(),
        })
    }

    pub(crate) fn export_canonical_map(&self) -> Option<TaskSpaceCanonicalMap> {
        self.active_map().map(|map| map.canonical_map().clone())
    }

    pub(crate) fn canonical_map_for_store(&self) -> Option<TaskSpaceCanonicalMap> {
        self.export_canonical_map()
    }

    pub(crate) fn restore_canonical_map(
        &mut self,
        map: TaskSpaceCanonicalMap,
        owner_session_id: Option<ThreadId>,
    ) {
        let map_id = map.map_id.clone();
        self.maps.insert(
            map_id.clone(),
            ActionMapInstance::from_graph(map, Vec::new(), owner_session_id),
        );
        self.active_map_id = Some(map_id);
    }

    pub(crate) fn restore_store_map(
        &mut self,
        map_id: &str,
        owner_session_id: ThreadId,
        canonical_map: Option<TaskSpaceCanonicalMap>,
    ) -> Result<(), String> {
        self.mode = MapRuntimeMode::Experiment;
        match canonical_map {
            Some(map) => {
                if map.map_id != map_id {
                    return Err(format!(
                        "TaskSpace Store map `{map_id}` does not match canonical map `{}`.",
                        map.map_id
                    ));
                }
                self.restore_canonical_map(map, Some(owner_session_id));
            }
            None => {
                self.maps.remove(map_id);
                self.active_map_id = Some(map_id.to_string());
            }
        }
        Ok(())
    }
}
