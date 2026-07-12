use super::*;

fn initialized_state(
    nodes: Vec<ActionMapInitializeNodeInput>,
    current_node_id: &str,
) -> (ActionMapRuntimeState, ThreadId, ActionMapInitializeOutcome) {
    let mut state = ActionMapRuntimeState::default();
    let owner = ThreadId::new();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    let (outcome, _) = state
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                task_title: "Agent-authored task".to_string(),
                source_event_ids: vec!["task-event-1".to_string()],
                nodes,
                current_node_id: current_node_id.to_string(),
            },
        )
        .expect("initialize map");
    (state, owner, outcome)
}

fn inspect_node(id: &str) -> ActionMapInitializeNodeInput {
    ActionMapInitializeNodeInput {
        id: id.to_string(),
        kind: NodeKind::InspectCodeContext,
        title: "Inspect".to_string(),
        context_summary: "Inspect the relevant implementation.".to_string(),
        dependency_node_ids: Vec::new(),
    }
}

#[test]
fn agent_initializes_explicit_graph_and_current_binding() {
    let implement = ActionMapInitializeNodeInput {
        id: "implement".to_string(),
        kind: NodeKind::ImplementSolution,
        title: "Implement".to_string(),
        context_summary: "Apply the chosen change.".to_string(),
        dependency_node_ids: vec!["inspect".to_string()],
    };
    let (state, _, outcome) =
        initialized_state(vec![inspect_node("inspect"), implement], "inspect");

    let map = state.maps.get(&outcome.map_id).expect("active map");
    assert_eq!(map.nodes.len(), 2);
    assert_eq!(map.edges.len(), 1);
    assert_eq!(state.current_main_node_id, Some(outcome.current_node_id));
}

#[test]
fn fork_rebinds_runtime_owner_and_main_lease() {
    let (mut state, original_owner, outcome) =
        initialized_state(vec![inspect_node("inspect")], "inspect");
    let fork_owner = ThreadId::new();

    let released_child_leases = state.rebind_after_fork(fork_owner);

    assert_eq!(released_child_leases, 0);
    let snapshot = state.snapshot();
    assert_ne!(fork_owner, original_owner);
    assert_eq!(snapshot.tasks[0].owner_session_id, Some(fork_owner));
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.id == outcome.map_id)
        .expect("forked map");
    assert_eq!(map.owner_session_id, Some(fork_owner));
    assert_eq!(map.leases.len(), 1);
    assert_eq!(map.leases[0].holder, "main");
    assert_eq!(map.leases[0].agent_thread_id, Some(fork_owner));
}

#[test]
fn snapshot_restore_preserves_maintenance_barrier() {
    let (state, _, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let mut snapshot = state.snapshot();
    snapshot.maintenance_barriers.push(
        codex_protocol::protocol::ActionMapSnapshotMaintenanceBarrier {
            map_id: outcome.map_id,
            node_id: outcome.current_node_id,
            reason: "node_tool_result_budget_exceeded".to_string(),
            result_count: 8,
            budget: 7,
        },
    );
    let expected = snapshot.maintenance_barriers.clone();
    let mut restored = ActionMapRuntimeState::default();

    restored.restore_snapshot(snapshot);

    assert_eq!(restored.snapshot().maintenance_barriers, expected);
}

#[test]
fn mechanical_blank_map_blocks_ordinary_tools() {
    let mut state = ActionMapRuntimeState::default();
    let owner = ThreadId::new();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);

    let error = state
        .prepare_main_tool_call(owner, ToolActionDescriptor::from("read_file"))
        .expect_err("blank map must block ordinary tools");

    assert!(error.to_string().contains("active_task_path_without_nodes"));
    assert!(error.to_string().contains("TaskSpaceGateResultV1"));
    let snapshot = state
        .provider_request_budget_snapshot()
        .expect("blank map provider snapshot");
    assert!(snapshot.map_requires_initialization);
}

#[test]
fn mechanical_blank_map_has_no_provider_developer_context() {
    let mut state = ActionMapRuntimeState::default();
    let owner = ThreadId::new();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);

    assert!(state.build_developer_context().is_none());
    assert!(state.take_pending_transition_notice().is_none());
}

#[test]
fn initialized_map_releases_provider_initialization_selection() {
    let (state, _, _) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let snapshot = state
        .provider_request_budget_snapshot()
        .expect("initialized map provider snapshot");
    assert!(!snapshot.map_requires_initialization);
}

#[test]
fn tool_result_is_recorded_under_current_node_by_canonical_event_ref() {
    let (mut state, owner, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let descriptor = ToolActionDescriptor::new("read_file", ActionClass::Read, "src/lib.rs")
        .with_call_id("call-1");
    state
        .prepare_main_tool_call(owner, descriptor)
        .expect("reserve tool call");
    let body = "line one\nline two\nraw failure-like word: error";
    let (event_id, _) = state
        .record_main_tool_result_with_class(
            owner,
            "call-1",
            "task-event-call-1".to_string(),
            "read_file",
            Some(ActionClass::Read),
            true,
            body.to_string(),
        )
        .expect("record tool result")
        .expect("taskspace event");

    let map = state.maps.get(&outcome.map_id).expect("active map");
    let event = map.node_events.get(&event_id).expect("node event");
    assert_eq!(event.node_id, outcome.current_node_id);
    assert_eq!(event.source_event_id.as_deref(), Some("task-event-call-1"));
    assert_eq!(event.tool_success, Some(true));
}

#[test]
fn missing_canonical_event_does_not_leave_completed_tool_in_flight() {
    let (mut state, owner, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    state
        .prepare_main_tool_call(
            owner,
            ToolActionDescriptor::new("exec_command", ActionClass::Read, "pwd")
                .with_call_id("nested-call"),
        )
        .expect("reserve nested tool call");

    let error = state
        .record_main_tool_result_with_class(
            owner,
            "nested-call",
            String::new(),
            "exec_command",
            Some(ActionClass::Read),
            true,
            "done".to_string(),
        )
        .expect_err("missing canonical event must remain explicit");
    assert!(error.contains("source_event_id"));

    state
        .finish_main_node_with_next(
            owner,
            &outcome.current_node_id,
            "Completed after attribution failure.".to_string(),
            None,
            None,
        )
        .expect("completed tool must not remain in flight");
}

#[test]
fn agent_can_finish_without_runtime_capability_inference() {
    let (mut state, owner, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let (finished, _) = state
        .finish_main_node_with_next(
            owner,
            &outcome.current_node_id,
            "Inspected the code.".to_string(),
            None,
            None,
        )
        .expect("finish node");
    assert!(finished.next_node_id.is_none());
    assert!(state.current_main_node_id.is_none());
}

#[test]
fn explicit_ready_target_is_claimed_and_finished_without_separate_bind() {
    let second = ActionMapInitializeNodeInput {
        id: "second".to_string(),
        kind: NodeKind::FinalSynthesis,
        title: "Second".to_string(),
        context_summary: "Record the second completed step.".to_string(),
        dependency_node_ids: vec!["first".to_string()],
    };
    let (mut state, owner, outcome) =
        initialized_state(vec![inspect_node("first"), second], "first");

    state
        .finish_main_node_with_next(
            owner,
            &outcome.current_node_id,
            "First complete.".to_string(),
            None,
            None,
        )
        .expect("finish current node");
    assert!(state.current_main_node_id.is_none());

    let (finished, events) = state
        .finish_main_node_with_next(owner, "second", "Second complete.".to_string(), None, None)
        .expect("claim and finish explicit ready target");

    assert!(finished.next_node_id.is_none());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, MapRuntimeEvent::LeaseCreated(_)))
    );
    assert!(state.current_main_node_id.is_none());
    let map = state.maps.get(&outcome.map_id).expect("active map");
    assert_eq!(
        map.nodes.get("second").expect("second node").status,
        NodeStatus::Completed
    );
}

#[test]
fn rejected_explicit_finish_does_not_leave_an_implicit_binding() {
    let second = ActionMapInitializeNodeInput {
        id: "second".to_string(),
        kind: NodeKind::InspectCodeContext,
        title: "Second".to_string(),
        context_summary: "Complete another prerequisite.".to_string(),
        dependency_node_ids: Vec::new(),
    };
    let final_node = ActionMapInitializeNodeInput {
        id: "final".to_string(),
        kind: NodeKind::FinalSynthesis,
        title: "Final".to_string(),
        context_summary: "Depends on both prerequisites.".to_string(),
        dependency_node_ids: vec!["first".to_string(), "second".to_string()],
    };
    let (mut state, owner, outcome) =
        initialized_state(vec![inspect_node("first"), second, final_node], "first");
    state
        .finish_main_node_with_next(
            owner,
            &outcome.current_node_id,
            "First complete.".to_string(),
            None,
            None,
        )
        .expect("finish first prerequisite");

    let error = state
        .finish_main_node_with_next(owner, "final", "Premature final.".to_string(), None, None)
        .expect_err("pending explicit target must be rejected");

    assert!(error.contains("target_node_dependencies_incomplete"));
    assert!(state.current_main_node_id.is_none());
    assert!(state.current_main_lease_id.is_none());
    let map = state.maps.get(&outcome.map_id).expect("active map");
    let final_node = map.nodes.get("final").expect("final node");
    assert_eq!(final_node.status, NodeStatus::Pending);
    assert!(final_node.active_lease.is_none());
}

#[test]
fn thin_projection_indexes_events_without_copying_raw_feedback() {
    let implement = ActionMapInitializeNodeInput {
        id: "implement".to_string(),
        kind: NodeKind::ImplementSolution,
        title: "Implement".to_string(),
        context_summary: "Apply the chosen change.".to_string(),
        dependency_node_ids: vec!["inspect".to_string()],
    };
    let (mut state, owner, outcome) =
        initialized_state(vec![inspect_node("inspect"), implement], "inspect");
    state
        .record_main_tool_result_with_class(
            owner,
            "call-1",
            "task-event-call-1".to_string(),
            "read_file",
            Some(ActionClass::Read),
            false,
            "command: cat private.txt\nraw_output:\nexact tool failure payload".to_string(),
        )
        .expect("record tool result");
    let event = state
        .maps
        .get_mut(&outcome.map_id)
        .and_then(|map| map.node_events.get_mut("node-event-1"))
        .expect("recorded node event");
    event.raw_ref = Some("output-ref-1".to_string());
    event.artifact_refs = vec!["src/private.rs".to_string()];

    let projection = state.build_developer_context().expect("projection");
    assert!(projection.contains("current_node_recent_events"));
    assert_eq!(projection.matches("task-event-call-1").count(), 1);
    assert_eq!(projection.matches("inspect kind=").count(), 1);
    assert_eq!(
        projection
            .matches("Inspect the relevant implementation.")
            .count(),
        1
    );
    assert!(projection.contains("map_edges:\n    - inspect->implement"));
    assert!(projection.contains("raw_ref=output-ref-1"));
    assert!(projection.contains("artifact_refs=src/private.rs"));
    assert!(!projection.contains("exact tool failure payload"));
    assert!(!projection.contains("command: cat private.txt"));
    assert!(!projection.contains("private.txt"));
    assert!(!projection.contains("excerpt:"));
    assert!(!projection.contains("raw_ref=none"));
    assert!(!projection.contains("artifacts=none"));
    assert!(!projection.contains("excerpt_truncated"));
    assert!(!projection.contains("current_node_dependencies"));
    assert!(!projection.contains("next_valid_actions"));
    assert!(!projection.contains("critical_artifact_evidence"));
    assert!(!projection.contains("fact_source_coverage"));
    assert!(!projection.contains("verified_input_evidence"));
}

#[test]
fn final_response_only_checks_mechanical_map_lifecycle() {
    let (mut state, owner, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let error = state
        .record_main_final_response(owner, "Done")
        .expect_err("open node must block final response");
    assert!(error.contains("active_node_open"));

    state
        .record_main_tool_result_with_class(
            owner,
            "call-1",
            "task-event-call-1".to_string(),
            "read_file",
            Some(ActionClass::Read),
            true,
            "observed source".to_string(),
        )
        .expect("record read result");
    state
        .finish_main_node_with_next(
            owner,
            &outcome.current_node_id,
            "Inspection complete.".to_string(),
            None,
            None,
        )
        .expect("finish node");

    let events = state
        .record_main_final_response(owner, "Done")
        .expect("final response completes task")
        .expect("completion events");
    assert!(events.iter().any(|event| matches!(
        event,
        MapRuntimeEvent::MapStatusChanged(event) if event.current_status == "completed"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        MapRuntimeEvent::TaskStatusChanged(event) if event.current_status == "completed"
    )));
    let snapshot = state.snapshot();
    assert_eq!(snapshot.maps[0].status, "completed");
    assert_eq!(snapshot.tasks[0].status, "completed");
    assert!(snapshot.active_map_id.is_none());
    assert!(snapshot.active_task_id.is_none());
}

#[test]
fn child_tool_result_metadata_is_extracted_without_retaining_body() {
    let child = ThreadId::new();
    let body = "*** Update File: core/src/lib.rs\nraw_output:\nsecret payload";

    assert_eq!(
        child_tool_source_event_ref(child, "call-child-1"),
        format!("thread:{child}/call:call-child-1")
    );
    assert_eq!(
        tool_result_artifact_refs(Some(ActionClass::Edit), true, body),
        vec!["core/src/lib.rs"]
    );
}

#[test]
fn terminal_candidate_commits_finish_and_final_gate_atomically() {
    let (mut state, owner, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let (finished, _) = state
        .finish_main_node_with_terminal_candidate(
            owner,
            &outcome.current_node_id,
            "Inspection complete.".to_string(),
            "Exact Agent final.",
        )
        .expect("terminal finish");

    assert!(finished.next_node_id.is_none());
    assert!(state.current_main_node_id.is_none());
}

#[test]
fn rejected_terminal_candidate_leaves_node_open() {
    let implement = ActionMapInitializeNodeInput {
        id: "implement".to_string(),
        kind: NodeKind::ImplementSolution,
        title: "Implement".to_string(),
        context_summary: "Apply the chosen change.".to_string(),
        dependency_node_ids: vec!["inspect".to_string()],
    };
    let (mut state, owner, outcome) =
        initialized_state(vec![inspect_node("inspect"), implement], "inspect");

    state
        .finish_main_node_with_terminal_candidate(
            owner,
            &outcome.current_node_id,
            "Premature finish.".to_string(),
            "Premature final.",
        )
        .expect_err("pending node must reject terminal candidate");

    assert_eq!(
        state.current_main_node_id,
        Some(outcome.current_node_id.clone())
    );
    let map = state.maps.get(&outcome.map_id).expect("active map");
    assert_eq!(
        map.nodes
            .get(&outcome.current_node_id)
            .expect("current node")
            .status,
        NodeStatus::Running
    );
}
