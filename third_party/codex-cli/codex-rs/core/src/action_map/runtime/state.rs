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

#[cfg(test)]
mod tests {
    use codex_protocol::ThreadId;
    use codex_protocol::protocol::MapRuntimeMode;

    use crate::action_map::rooted_dag::BlockRecord;
    use crate::action_map::rooted_dag::CompletionRecord;
    use crate::action_map::rooted_dag::MapEdge;
    use crate::action_map::rooted_dag::TaskSpaceMap;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::rooted_dag::new_map;
    use crate::action_map::runtime::ActionMapRuntimeState;

    fn valid_map(map_id: &str) -> TaskSpaceMap {
        new_map(
            map_id.into(),
            map_node("root", "solve", vec![]),
            vec![map_node("work", "work", vec![])],
            map_node("finish", "finish", vec![]),
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
        )
    }

    #[test]
    fn restore_store_map_accepts_canonical_map_without_snapshot() {
        let owner = ThreadId::new();
        let map = valid_map("map-1");
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

    #[test]
    fn restore_store_map_rejects_invalid_canonical_maps_without_mutation() {
        let owner = ThreadId::new();
        let existing = valid_map("existing-map");
        let mut runtime = ActionMapRuntimeState::default();
        runtime
            .restore_store_map("existing-map", owner, Some(existing.clone()))
            .expect("seed existing canonical Map");

        let mut cycle = valid_map("cycle-map");
        cycle.edges.push(MapEdge {
            from: "finish".into(),
            to: "root".into(),
        });
        let mut unreachable = valid_map("unreachable-map");
        unreachable.edges.remove(0);
        let mut fact_conflict = valid_map("fact-conflict-map");
        fact_conflict.completion_records.insert(
            "work".into(),
            CompletionRecord {
                action_id: "complete-work".into(),
                result_ref_ids: vec![],
                evidence_ref_ids: vec![],
            },
        );
        fact_conflict.block_records.insert(
            "work".into(),
            BlockRecord {
                action_id: "block-work".into(),
                reason_ref: "reason-1".into(),
            },
        );

        for invalid in [cycle, unreachable, fact_conflict] {
            let invalid_id = invalid.map_id.clone();
            let error = runtime
                .restore_store_map(&invalid_id, owner, Some(invalid))
                .expect_err("invalid canonical Map must be rejected");
            assert!(
                error.contains("invalid"),
                "restore error must identify invalid canonical Map: {error}"
            );
            assert_eq!(runtime.mode(), MapRuntimeMode::Experiment);
            assert_eq!(runtime.active_map_id(), Some("existing-map"));
            assert_eq!(runtime.canonical_map_for_store(), Some(existing.clone()));
        }
    }

    #[test]
    fn restore_store_map_id_mismatch_does_not_change_runtime() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();

        let error = runtime
            .restore_store_map("expected-map", owner, Some(valid_map("other-map")))
            .expect_err("Map identity mismatch");

        assert!(error.contains("does not match"));
        assert_eq!(runtime.mode(), MapRuntimeMode::Standard);
        assert_eq!(runtime.active_map_id(), None);
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

    pub(crate) fn export_canonical_map(&self) -> Option<TaskSpaceCanonicalMap> {
        self.active_map().map(|map| map.canonical_map().clone())
    }

    pub(crate) fn canonical_map_for_store(&self) -> Option<TaskSpaceCanonicalMap> {
        self.export_canonical_map()
    }

    fn restore_canonical_map(
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
                self.restore_canonical_map(map, Some(owner_session_id));
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
