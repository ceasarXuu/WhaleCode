use codex_protocol::ThreadId;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::taskspace::TaskSpaceTerminalRecord;

use crate::action_map::map::ActionMapInstance;
use crate::action_map::response::ActionMapDeclaredCall;
use crate::action_map::response::ActionMapPreparedCall;
use crate::action_map::response::ActionMapPreparedResponse;
use crate::action_map::response::ActionMapResponseOperation;
use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::ActionReservation;
use crate::action_map::rooted_dag::EvidenceRefInput;
use crate::action_map::rooted_dag::FinalCompletion;
use crate::action_map::rooted_dag::FinishMap;
use crate::action_map::rooted_dag::GraphMutation;
use crate::action_map::rooted_dag::InitializeMap;
use crate::action_map::rooted_dag::MapEdge;
use crate::action_map::rooted_dag::MapNode;
use crate::action_map::rooted_dag::NodeMutation;
use crate::action_map::rooted_dag::ReservationInput;
use crate::action_map::rooted_dag::ReservationRelease;
use crate::action_map::rooted_dag::ResultRefInput;

use super::state::ActionMapRuntimeState;
use super::types::ActionMapControlDelta;
use super::types::ActionMapTerminalOutcome;

impl ActionMapRuntimeState {
    pub(crate) fn prepare_response_for_main(
        &mut self,
        owner_session_id: ThreadId,
        control_call_id: &str,
        operation: ActionMapResponseOperation,
        calls: Vec<ActionMapDeclaredCall>,
    ) -> Result<(ActionMapPreparedResponse, Vec<MapRuntimeEvent>), String> {
        if calls.is_empty() {
            return Err(rejection_json(0, "reservation_invalid", "empty_response"));
        }
        let revision_before = self
            .active_map()
            .map_or(0, |map| map.canonical_map().revision);
        let (map_id, revision_after, action, prepared_calls) = match operation {
            ActionMapResponseOperation::Initialize {
                root,
                work_nodes,
                finish,
                edges,
            } => self.prepare_initialize(
                owner_session_id,
                control_call_id,
                root,
                work_nodes,
                finish,
                edges,
                calls,
            )?,
            ActionMapResponseOperation::Execute {
                expected_revision,
                graph,
                node_mutations,
            } => self.prepare_execute(
                control_call_id,
                expected_revision,
                graph,
                node_mutations,
                calls,
            )?,
        };
        Ok((
            ActionMapPreparedResponse {
                map_id,
                revision_before,
                revision_after,
                action,
                prepared_calls,
            },
            Vec::new(),
        ))
    }

    pub(crate) fn release_main_action_result(
        &mut self,
        _owner: ThreadId,
        prepared: &ActionMapPreparedCall,
        success: bool,
        result_ref_id: String,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let map = self
            .maps
            .get_mut(&prepared.map_id)
            .ok_or_else(|| rejection_json(0, "map_missing", &prepared.map_id))?;
        let reservation = map
            .canonical_map()
            .action_reservations
            .get(&prepared.reservation_id)
            .ok_or_else(|| {
                rejection_json(
                    map.canonical_map().revision,
                    "reservation_invalid",
                    &prepared.reservation_id,
                )
            })?;
        if reservation.node_id != prepared.node_id
            || reservation.tool_name != prepared.tool_name
            || reservation.response_call_index != prepared.call_index as u32
        {
            return Err(rejection_json(
                map.canonical_map().revision,
                "reservation_invalid",
                &prepared.reservation_id,
            ));
        }
        let expected_action_id = action_id(prepared);
        if reservation.action_id != expected_action_id {
            return Err(rejection_json(
                map.canonical_map().revision,
                "reservation_invalid",
                &prepared.call_id,
            ));
        }
        let commit = rooted_dag::release_reservation(
            map.canonical_map(),
            ReservationRelease {
                expected_revision: map.canonical_map().revision,
                reservation_id: prepared.reservation_id.clone(),
                result_refs: vec![ResultRefInput {
                    result_ref_id,
                    is_error: !success,
                }],
                evidence_refs: Vec::<EvidenceRefInput>::new(),
            },
        )
        .map_err(rejection)?;
        map.commit_graph(commit.map, commit.events);
        Ok(Vec::new())
    }

    pub(crate) fn finish_map_for_main(
        &mut self,
        _owner: ThreadId,
        expected_revision: u64,
        finish_node_id: String,
        exact_summary: String,
        action_id: String,
    ) -> Result<(ActionMapTerminalOutcome, Vec<MapRuntimeEvent>), String> {
        let map = self
            .active_map_mut()
            .ok_or_else(|| rejection_json(0, "map_missing", "active_map"))?;
        let commit = rooted_dag::finish_map(
            map.canonical_map(),
            FinishMap {
                expected_revision,
                finish_node_id: finish_node_id.clone(),
                final_completions: Vec::<FinalCompletion>::new(),
                terminal: TaskSpaceTerminalRecord {
                    action_id,
                    summary_ref: exact_summary.clone(),
                },
            },
        )
        .map_err(rejection)?;
        map.commit_graph(commit.map, commit.events);
        Ok((
            ActionMapTerminalOutcome {
                map_id: map.map_id.clone(),
                terminal_node_id: finish_node_id,
                revision: map.canonical_map().revision,
                final_summary: exact_summary,
                delta: delta(map),
            },
            Vec::new(),
        ))
    }

    fn prepare_initialize(
        &mut self,
        owner: ThreadId,
        control_call_id: &str,
        root: MapNode,
        work_nodes: Vec<MapNode>,
        finish: MapNode,
        edges: Vec<MapEdge>,
        calls: Vec<ActionMapDeclaredCall>,
    ) -> Result<(String, u64, &'static str, Vec<ActionMapPreparedCall>), String> {
        let map_id = self
            .active_map_id
            .clone()
            .ok_or_else(|| rejection_json(0, "map_identity_missing", control_call_id))?;
        if self.maps.contains_key(&map_id) {
            return Err(rejection_json(0, "transition_invalid", "map_exists"));
        }
        let reservations = reservation_inputs(&map_id, control_call_id, &calls);
        let commit = rooted_dag::initialize(InitializeMap {
            map_id: map_id.clone(),
            root,
            work_nodes,
            finish,
            edges,
            reservations,
        })
        .map_err(rejection)?;
        let revision = commit.map.revision;
        let prepared = prepared_calls(&map_id, revision, control_call_id, &calls);
        self.maps.insert(
            map_id.clone(),
            ActionMapInstance::from_graph(commit.map, vec![commit.events], Some(owner)),
        );
        Ok((map_id, revision, "initialize_and_execute", prepared))
    }

    fn prepare_execute(
        &mut self,
        control_call_id: &str,
        expected_revision: u64,
        graph: GraphMutation,
        node_mutations: Vec<NodeMutation>,
        calls: Vec<ActionMapDeclaredCall>,
    ) -> Result<(String, u64, &'static str, Vec<ActionMapPreparedCall>), String> {
        let map = self
            .active_map_mut()
            .ok_or_else(|| rejection_json(0, "map_missing", "active_map"))?;
        let map_id = map.map_id.clone();
        let commit = rooted_dag::execute(
            map.canonical_map(),
            rooted_dag::ExecuteTransaction {
                expected_revision,
                graph,
                node_mutations,
                reservations: reservation_inputs(&map_id, control_call_id, &calls),
            },
        )
        .map_err(rejection)?;
        let revision = commit.map.revision;
        let prepared = prepared_calls(&map_id, revision, control_call_id, &calls);
        map.commit_graph(commit.map, commit.events);
        Ok((map_id, revision, "execute", prepared))
    }
}

fn reservation_inputs(
    map_id: &str,
    control_call_id: &str,
    calls: &[ActionMapDeclaredCall],
) -> Vec<ReservationInput> {
    calls
        .iter()
        .map(|call| {
            let prepared = prepared_call(map_id, 0, control_call_id, call);
            let action_id = action_id(&prepared);
            ReservationInput {
                reservation_id: prepared.reservation_id,
                reservation: ActionReservation {
                    action_id,
                    node_id: call.node_id.clone(),
                    tool_name: call.tool_name.clone(),
                    response_call_index: call.call_index as u32,
                },
            }
        })
        .collect()
}

fn prepared_calls(
    map_id: &str,
    revision: u64,
    control_call_id: &str,
    calls: &[ActionMapDeclaredCall],
) -> Vec<ActionMapPreparedCall> {
    calls
        .iter()
        .map(|call| prepared_call(map_id, revision, control_call_id, call))
        .collect()
}

fn prepared_call(
    map_id: &str,
    revision: u64,
    control_call_id: &str,
    call: &ActionMapDeclaredCall,
) -> ActionMapPreparedCall {
    ActionMapPreparedCall {
        map_id: map_id.to_string(),
        revision,
        call_id: call.call_id.clone(),
        call_index: call.call_index,
        node_id: call.node_id.clone(),
        tool_name: call.tool_name.clone(),
        reservation_id: format!(
            "{map_id}:reservation:{control_call_id}:{}:{}",
            call.call_index, call.call_id
        ),
    }
}

fn action_id(prepared: &ActionMapPreparedCall) -> String {
    format!(
        "{}:action:{}:{}",
        prepared.map_id, prepared.call_index, prepared.call_id
    )
}

fn delta(map: &ActionMapInstance) -> ActionMapControlDelta {
    ActionMapControlDelta {
        map_id: map.map_id.clone(),
        committed_revision: map.canonical_map().revision,
        graph_revision_batches: map.graph_events.clone(),
    }
}

fn rejection(rejection: rooted_dag::Rejection) -> String {
    serde_json::json!({
        "state_commit": rejection.state_commit,
        "current_revision": rejection.current_revision,
        "violations": rejection.violations.iter().map(|violation| {
            serde_json::json!({
                "code": violation.code.as_str(),
                "subjects": violation.subjects,
            })
        }).collect::<Vec<_>>()
    })
    .to_string()
}

fn rejection_json(current_revision: u64, code: &str, subject: &str) -> String {
    serde_json::json!({
        "state_commit": false,
        "current_revision": current_revision,
        "violations": [{"code": code, "subjects": [subject]}],
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use codex_protocol::ThreadId;

    use crate::action_map::response::ActionMapDeclaredCall;
    use crate::action_map::response::ActionMapResponseOperation;
    use crate::action_map::rooted_dag::MapEdge;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::runtime::ActionMapRuntimeState;

    #[test]
    fn prepare_initialize_uses_existing_active_identity() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();
        runtime
            .restore_store_map("store-map-9", owner, None)
            .expect("restore empty identity");

        let (prepared, events) = runtime
            .prepare_response_for_main(
                owner,
                "control-call-1",
                initialize_operation(),
                vec![declared_call("call-1", "inspect", "exec_command")],
            )
            .expect("initialize response commits");

        assert!(events.is_empty());
        assert_eq!(prepared.action, "initialize_and_execute");
        assert_eq!(prepared.map_id, "store-map-9");
        assert_eq!(runtime.active_map_id(), Some("store-map-9"));
        assert_eq!(
            runtime.canonical_map_for_store().unwrap().map_id,
            "store-map-9"
        );
        assert!(
            prepared.prepared_calls[0]
                .reservation_id
                .contains("control-call-1")
        );
    }

    #[test]
    fn release_rejects_prepared_call_metadata_mismatch() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();
        runtime
            .restore_store_map("store-map-10", owner, None)
            .expect("restore empty identity");
        let (prepared, _) = runtime
            .prepare_response_for_main(
                owner,
                "control-call-1",
                initialize_operation(),
                vec![declared_call("call-1", "inspect", "exec_command")],
            )
            .expect("initialize response commits");
        let mut tampered = prepared.prepared_calls[0].clone();
        tampered.tool_name = "different_tool".to_string();

        let error = runtime
            .release_main_action_result(owner, &tampered, true, "result-1".to_string())
            .expect_err("tampered prepared call is rejected");

        assert!(error.contains("reservation_invalid"));
        assert!(
            runtime
                .canonical_map_for_store()
                .unwrap()
                .result_refs
                .is_empty()
        );
    }

    #[test]
    fn multi_action_results_are_attributed_to_declared_nodes() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();
        runtime
            .restore_store_map("store-map-11", owner, None)
            .expect("restore empty identity");
        let (prepared, _) = runtime
            .prepare_response_for_main(
                owner,
                "control-call-2",
                parallel_initialize_operation(),
                vec![
                    declared_call_at(0, "call-read", "inspect", "read_file"),
                    declared_call_at(1, "call-test", "verify", "exec_command"),
                ],
            )
            .expect("parallel response commits");

        assert_eq!(prepared.prepared_calls.len(), 2);
        let revision_after_reservations = prepared.revision_after;
        runtime
            .release_main_action_result(
                owner,
                &prepared.prepared_calls[0],
                true,
                "tool-result://call/call-read".to_string(),
            )
            .expect("read result commits");
        runtime
            .release_main_action_result(
                owner,
                &prepared.prepared_calls[1],
                false,
                "tool-result://call/call-test".to_string(),
            )
            .expect("test result commits");

        let map = runtime.canonical_map_for_store().expect("canonical map");
        assert_eq!(map.revision, revision_after_reservations + 2);
        assert!(map.action_reservations.is_empty());
        assert_eq!(
            map.result_refs["tool-result://call/call-read"].node_id,
            "inspect"
        );
        assert!(!map.result_refs["tool-result://call/call-read"].is_error);
        assert_eq!(
            map.result_refs["tool-result://call/call-test"].node_id,
            "verify"
        );
        assert!(map.result_refs["tool-result://call/call-test"].is_error);
    }

    #[test]
    fn skipped_action_release_survives_canonical_map_restore() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();
        runtime
            .restore_store_map("store-map-12", owner, None)
            .expect("restore empty identity");
        let (prepared, _) = runtime
            .prepare_response_for_main(
                owner,
                "control-call-3",
                parallel_initialize_operation(),
                vec![
                    declared_call_at(0, "call-patch", "inspect", "apply_patch"),
                    declared_call_at(1, "call-verify", "verify", "exec_command"),
                ],
            )
            .expect("parallel response commits");

        runtime
            .release_main_action_result(
                owner,
                &prepared.prepared_calls[0],
                false,
                "tool-result://call/call-patch".to_string(),
            )
            .expect("failed patch result commits");
        runtime
            .release_main_action_result(
                owner,
                &prepared.prepared_calls[1],
                false,
                "tool-result://call/call-verify".to_string(),
            )
            .expect("skipped verify result commits");
        let stored = runtime
            .canonical_map_for_store()
            .expect("canonical map for Store");

        let mut restored = ActionMapRuntimeState::default();
        restored
            .restore_store_map("store-map-12", owner, Some(stored))
            .expect("restore persisted canonical map");
        let map = restored
            .canonical_map_for_store()
            .expect("restored canonical map");

        assert!(map.action_reservations.is_empty());
        assert!(map.result_refs["tool-result://call/call-patch"].is_error);
        assert!(map.result_refs["tool-result://call/call-verify"].is_error);
    }

    fn initialize_operation() -> ActionMapResponseOperation {
        ActionMapResponseOperation::Initialize {
            root: map_node("root", "solve", vec!["turn-1".to_string()]),
            work_nodes: vec![map_node("inspect", "inspect", Vec::new())],
            finish: map_node("finish", "close the task", Vec::new()),
            edges: vec![
                MapEdge {
                    from: "root".to_string(),
                    to: "inspect".to_string(),
                },
                MapEdge {
                    from: "inspect".to_string(),
                    to: "finish".to_string(),
                },
            ],
        }
    }

    fn declared_call(call_id: &str, node_id: &str, tool_name: &str) -> ActionMapDeclaredCall {
        declared_call_at(0, call_id, node_id, tool_name)
    }

    fn declared_call_at(
        call_index: usize,
        call_id: &str,
        node_id: &str,
        tool_name: &str,
    ) -> ActionMapDeclaredCall {
        ActionMapDeclaredCall {
            call_id: call_id.to_string(),
            call_index,
            node_id: node_id.to_string(),
            tool_name: tool_name.to_string(),
        }
    }

    fn parallel_initialize_operation() -> ActionMapResponseOperation {
        ActionMapResponseOperation::Initialize {
            root: map_node("root", "solve", vec!["turn-1".to_string()]),
            work_nodes: vec![
                map_node("inspect", "inspect", Vec::new()),
                map_node("verify", "verify", Vec::new()),
            ],
            finish: map_node("finish", "close the task", Vec::new()),
            edges: vec![
                MapEdge {
                    from: "root".to_string(),
                    to: "inspect".to_string(),
                },
                MapEdge {
                    from: "root".to_string(),
                    to: "verify".to_string(),
                },
                MapEdge {
                    from: "inspect".to_string(),
                    to: "finish".to_string(),
                },
                MapEdge {
                    from: "verify".to_string(),
                    to: "finish".to_string(),
                },
            ],
        }
    }
}
