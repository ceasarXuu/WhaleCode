use super::*;

fn initialized_state(
    nodes: Vec<ActionMapInitializeNodeInput>,
    current_node_key: &str,
) -> (ActionMapRuntimeState, ThreadId, ActionMapInitializeOutcome) {
    let mut state = ActionMapRuntimeState::default();
    let owner = ThreadId::new();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    let (outcome, _) = state
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                task_title: "Agent-authored task".to_string(),
                task_objective: "Complete the requested change".to_string(),
                nodes,
                current_node_key: current_node_key.to_string(),
            },
        )
        .expect("initialize map");
    (state, owner, outcome)
}

fn inspect_node(key: &str) -> ActionMapInitializeNodeInput {
    ActionMapInitializeNodeInput {
        key: key.to_string(),
        kind: NodeKind::InspectCodeContext,
        title: "Inspect".to_string(),
        context_summary: "Inspect the relevant implementation.".to_string(),
        dependency_keys: Vec::new(),
    }
}

#[test]
fn agent_initializes_explicit_graph_and_current_binding() {
    let implement = ActionMapInitializeNodeInput {
        key: "implement".to_string(),
        kind: NodeKind::ImplementSolution,
        title: "Implement".to_string(),
        context_summary: "Apply the chosen change.".to_string(),
        dependency_keys: vec!["inspect".to_string()],
    };
    let (state, _, outcome) =
        initialized_state(vec![inspect_node("inspect"), implement], "inspect");

    let map = state.maps.get(&outcome.map_id).expect("active map");
    assert_eq!(map.nodes.len(), 2);
    assert_eq!(map.edges.len(), 1);
    assert_eq!(state.current_main_node_id, Some(outcome.current_node_id));
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
    assert!(error.to_string().contains("TaskSpaceGateRecoveryV1"));
}

#[test]
fn tool_result_is_recorded_under_current_node_without_body_rewrite() {
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
    assert_eq!(event.body, body);
    assert_eq!(event.tool_success, Some(true));
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
fn thin_projection_keeps_raw_feedback_without_strategy_sections() {
    let (mut state, owner, _) = initialized_state(vec![inspect_node("inspect")], "inspect");
    state
        .record_main_tool_result_with_class(
            owner,
            "call-1",
            "read_file",
            Some(ActionClass::Read),
            false,
            "exact tool failure payload".to_string(),
        )
        .expect("record tool result");

    let projection = state.build_developer_context().expect("projection");
    assert!(projection.contains("exact tool failure payload"));
    assert!(projection.contains("current_node_recent_events"));
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

    assert_eq!(state.record_main_final_response(owner, "Done"), Ok(None));
}
