use codex_protocol::ThreadId;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::taskspace::TaskSpaceTerminalRecord;
use std::collections::HashSet;

use crate::action_map::map::ActionMapInstance;
use crate::action_map::response::ActionMapDeclaredCall;
use crate::action_map::response::ActionMapPreparedCall;
use crate::action_map::response::ActionMapPreparedResponse;
use crate::action_map::response::ActionMapResponseOperation;
use crate::action_map::response::ActionMapResponseSettlement;
use crate::action_map::response::model_visible_state_violations;
use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::ActionReservation;
use crate::action_map::rooted_dag::CompletionRecord;
use crate::action_map::rooted_dag::EvidenceRefInput;
use crate::action_map::rooted_dag::FinalCompletion;
use crate::action_map::rooted_dag::FinishMap;
use crate::action_map::rooted_dag::GraphMutation;
use crate::action_map::rooted_dag::InitializeMap;
use crate::action_map::rooted_dag::MapEdge;
use crate::action_map::rooted_dag::MapNode;
use crate::action_map::rooted_dag::NodeMutation;
use crate::action_map::rooted_dag::Rejection;
use crate::action_map::rooted_dag::ReopenMap;
use crate::action_map::rooted_dag::ReservationInput;
use crate::action_map::rooted_dag::ReservationRelease;
use crate::action_map::rooted_dag::ResultRefInput;
use crate::action_map::rooted_dag::ViolationCode;

use super::state::ActionMapRuntimeState;
use super::types::ActionMapControlDelta;
use super::types::ActionMapTerminalOutcome;

impl ActionMapRuntimeState {
    pub(crate) fn response_settlement_for_main(
        &self,
        prepared: &ActionMapPreparedResponse,
    ) -> Result<ActionMapResponseSettlement, String> {
        let map = self
            .maps
            .get(&prepared.map_id)
            .ok_or_else(|| rejection_json(0, "map_missing", &prepared.map_id))?;
        Ok(ActionMapResponseSettlement::from_canonical_map(
            prepared,
            map.canonical_map(),
        ))
    }

    pub(crate) fn prepare_response_for_main(
        &mut self,
        owner_session_id: ThreadId,
        control_call_id: &str,
        operation: ActionMapResponseOperation,
        calls: Vec<ActionMapDeclaredCall>,
    ) -> Result<(ActionMapPreparedResponse, Vec<MapRuntimeEvent>), Rejection> {
        validate_declared_calls(control_call_id, &calls)?;
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
            ActionMapResponseOperation::Reopen {
                expected_revision,
                work_nodes,
                edges,
            } => {
                self.prepare_reopen(control_call_id, expected_revision, work_nodes, edges, calls)?
            }
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
        complete_work_node_ids: Vec<String>,
        exact_summary: String,
        action_id: String,
    ) -> Result<(ActionMapTerminalOutcome, Vec<MapRuntimeEvent>), String> {
        let map = self
            .active_map_mut()
            .ok_or_else(|| rejection_json(0, "map_missing", "active_map"))?;
        let final_completions = complete_work_node_ids
            .iter()
            .map(|node_id| FinalCompletion {
                node_id: node_id.clone(),
                record: CompletionRecord {
                    action_id: action_id.clone(),
                    result_ref_ids: Vec::new(),
                    evidence_ref_ids: Vec::new(),
                },
            })
            .collect();
        let commit = rooted_dag::finish_map(
            map.canonical_map(),
            FinishMap {
                expected_revision,
                finish_node_id: finish_node_id.clone(),
                final_completions,
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
                finish_node_id,
                completed_work_node_ids: complete_work_node_ids,
                revision: map.canonical_map().revision,
                exact_summary,
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
    ) -> Result<(String, u64, &'static str, Vec<ActionMapPreparedCall>), Rejection> {
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            Rejection::one(
                0,
                ViolationCode::MapIdentityInvalid,
                format!("map_identity_missing:{control_call_id}"),
            )
        })?;
        if self.maps.contains_key(&map_id) {
            return Err(Rejection::one(
                0,
                ViolationCode::TransitionInvalid,
                "map_exists",
            ));
        }
        let reservations = reservation_inputs(&map_id, control_call_id, &calls);
        let commit = rooted_dag::initialize(InitializeMap {
            map_id: map_id.clone(),
            root,
            work_nodes,
            finish,
            edges,
            reservations,
        })?;
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
    ) -> Result<(String, u64, &'static str, Vec<ActionMapPreparedCall>), Rejection> {
        let map = self.active_map_mut().ok_or_else(|| {
            Rejection::one(
                0,
                ViolationCode::MapIdentityInvalid,
                "map_missing:active_map",
            )
        })?;
        let map_id = map.map_id.clone();
        let commit = rooted_dag::execute(
            map.canonical_map(),
            rooted_dag::ExecuteTransaction {
                expected_revision,
                graph,
                node_mutations,
                reservations: reservation_inputs(&map_id, control_call_id, &calls),
            },
        )?;
        let revision = commit.map.revision;
        let prepared = prepared_calls(&map_id, revision, control_call_id, &calls);
        map.commit_graph(commit.map, commit.events);
        Ok((map_id, revision, "execute", prepared))
    }

    fn prepare_reopen(
        &mut self,
        control_call_id: &str,
        expected_revision: u64,
        work_nodes: Vec<MapNode>,
        edges: Vec<MapEdge>,
        calls: Vec<ActionMapDeclaredCall>,
    ) -> Result<(String, u64, &'static str, Vec<ActionMapPreparedCall>), Rejection> {
        let map = self.active_map_mut().ok_or_else(|| {
            Rejection::one(
                0,
                ViolationCode::MapIdentityInvalid,
                "map_missing:active_map",
            )
        })?;
        let map_id = map.map_id.clone();
        let commit = rooted_dag::reopen_map(
            map.canonical_map(),
            ReopenMap {
                expected_revision,
                add_work_nodes: work_nodes,
                add_edges: edges,
                reservations: reservation_inputs(&map_id, control_call_id, &calls),
            },
        )?;
        let revision = commit.map.revision;
        let prepared = prepared_calls(&map_id, revision, control_call_id, &calls);
        map.commit_graph(commit.map, commit.events);
        Ok((map_id, revision, "reopen_map", prepared))
    }
}

fn validate_declared_calls(
    control_call_id: &str,
    calls: &[ActionMapDeclaredCall],
) -> Result<(), Rejection> {
    if control_call_id.trim().is_empty() {
        return Err(Rejection::one(
            0,
            ViolationCode::ReservationInvalid,
            "empty_control_call_id",
        ));
    }
    if calls.is_empty() {
        return Err(Rejection::one(
            0,
            ViolationCode::ReservationInvalid,
            "empty_response",
        ));
    }
    let mut call_ids = HashSet::with_capacity(calls.len() + 1);
    call_ids.insert(control_call_id);
    for (index, call) in calls.iter().enumerate() {
        if call.call_id.trim().is_empty()
            || call.node_id.trim().is_empty()
            || call.tool_name.trim().is_empty()
            || call.call_index != index
        {
            return Err(Rejection::one(
                0,
                ViolationCode::ReservationInvalid,
                format!("call_identity:{index}"),
            ));
        }
        if !call_ids.insert(call.call_id.as_str()) {
            return Err(Rejection::one(
                0,
                ViolationCode::ReservationInvalid,
                format!("duplicate_call_id:{}", call.call_id),
            ));
        }
    }
    Ok(())
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
        "violations": model_visible_state_violations(&rejection),
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
#[path = "transactions_tests.rs"]
mod tests;
