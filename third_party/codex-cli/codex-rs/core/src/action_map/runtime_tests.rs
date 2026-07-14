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

fn initialized_five_node_chain() -> (ActionMapRuntimeState, ThreadId, ActionMapInitializeOutcome) {
    let nodes = (0..5)
        .map(|index| ActionMapInitializeNodeInput {
            id: format!("node-{index}"),
            kind: NodeKind::InspectCodeContext,
            title: format!("Node {index}"),
            context_summary: format!("Complete step {index}."),
            dependency_node_ids: (index > 0)
                .then(|| format!("node-{}", index - 1))
                .into_iter()
                .collect(),
        })
        .collect();
    let (mut state, owner, outcome) = initialized_state(nodes, "node-0");
    for index in 0..4 {
        let call_id = format!("call-read-{index}");
        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("read_file", ActionClass::Read, "src/lib.rs")
                    .with_call_id(call_id.clone()),
            )
            .expect("reserve chain read");
        state
            .record_main_tool_result_with_class(
                owner,
                &call_id,
                format!("task-event-read-{index}"),
                "read_file",
                Some(ActionClass::Read),
                true,
                format!("evidence for node {index}"),
            )
            .expect("record chain read")
            .expect("recorded chain event");
        state
            .finish_main_node_with_next(
                owner,
                &format!("node-{index}"),
                format!("task-event-finish-{index}"),
                Some(format!("node-{}", index + 1)),
                None,
            )
            .expect("advance chain");
    }
    (state, owner, outcome)
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
fn control_state_exposes_mechanical_open_map_state() {
    let implement = ActionMapInitializeNodeInput {
        id: "implement".to_string(),
        kind: NodeKind::ImplementSolution,
        title: "Implement".to_string(),
        context_summary: "Apply the chosen change.".to_string(),
        dependency_node_ids: vec!["inspect".to_string()],
    };
    let (state, _, outcome) =
        initialized_state(vec![inspect_node("inspect"), implement], "inspect");

    let control = state
        .control_state(Some(&outcome.map_id))
        .expect("control state");

    assert_eq!(control.task_id, outcome.task_id);
    assert_eq!(control.map_id, outcome.map_id);
    assert_eq!(control.current_node_id.as_deref(), Some("inspect"));
    assert_eq!(control.pending_node_ids, vec!["implement"]);
    assert_eq!(control.open_node_ids, vec!["inspect"]);
    assert!(control.blocked_node_ids.is_empty());
    assert_eq!(control.completed_node_count, 0);
    assert_eq!(control.total_node_count, 2);
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
fn node_detail_expansion_is_atomic_and_survives_repeated_snapshot_restore() {
    let (mut state, owner, outcome) = initialized_five_node_chain();

    let (expanded, events) = state
        .expand_node_details_for_main(
            owner,
            vec!["node-1".into()],
            "call-expand-1".into(),
            "task-event-expand-1".into(),
        )
        .expect("expand folded node");

    assert_eq!(expanded.len(), 1);
    assert!(matches!(
        events.as_slice(),
        [MapRuntimeEvent::NodeDetailExpanded(event)]
            if event.node_id == "node-1"
                && event.expansion_event_id == expanded[0].expansion_event_id
    ));
    let expected_event_id = expanded[0].expansion_event_id.clone();
    let mut snapshot = state.snapshot();
    for _ in 0..20 {
        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot);
        snapshot = restored.snapshot();
    }
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.id == outcome.map_id)
        .expect("restored map");
    assert!(map.node_events.iter().any(|event| {
        event.id == expected_event_id
            && event.event_kind == NODE_DETAIL_EXPANDED_EVENT_KIND
            && event.call_id.as_deref() == Some("call-expand-1")
            && event.source_event_id.as_deref() == Some("task-event-expand-1")
    }));
    assert!(
        map.nodes
            .iter()
            .find(|node| node.id == "node-1")
            .expect("expanded node")
            .node_event_ids
            .contains(&expected_event_id)
    );
}

#[test]
fn node_detail_expansion_rejects_mixed_batch_without_partial_commit() {
    let (mut state, owner, _) = initialized_five_node_chain();
    let before = state.snapshot();

    let error = state
        .expand_node_details_for_main(
            owner,
            vec!["node-1".into(), "node-2".into()],
            "call-expand-invalid".into(),
            "task-event-expand-invalid".into(),
        )
        .expect_err("full node makes the entire batch invalid");

    assert!(error.contains("node_detail_not_folded"));
    assert_eq!(state.snapshot(), before);
}

#[test]
fn s4_folds_only_remote_details_and_expand_restores_exact_baseline_details() {
    let (mut state, owner, _) = initialized_five_node_chain();

    let folded_context = state
        .build_developer_context()
        .expect("folded projection context");

    assert!(folded_context.contains("node-0 kind=inspect_code_context status=completed"));
    assert!(folded_context.contains("node-1 kind=inspect_code_context status=completed"));
    assert!(folded_context.contains("node-2 kind=inspect_code_context status=completed"));
    assert!(folded_context.contains("node-3 kind=inspect_code_context status=completed"));
    assert!(folded_context.contains("node-4 kind=inspect_code_context status=running"));
    assert!(folded_context.contains("node-0->node-1"));
    assert!(folded_context.contains("node-3->node-4"));
    let folded_node_line = folded_context
        .lines()
        .find(|line| line.contains("node-1 kind=inspect_code_context"))
        .expect("folded node skeleton");
    assert!(folded_node_line.contains("status=completed"));
    assert!(folded_node_line.contains("goal=\"Complete step 1.\""));
    assert!(folded_node_line.contains("event_count=1"));
    assert!(folded_node_line.contains("detail_state=folded frontier_distance=3"));
    assert!(folded_context.contains("detail_ref=taskspace-detail://sha256/"));
    assert!(!folded_context.contains("task-event-read-1 node=node-1"));
    assert!(folded_context.contains("task-event-read-2 node=node-2"));
    let snapshot = state.snapshot();
    let budget_event = snapshot
        .trace_events
        .iter()
        .rev()
        .find(|event| event.kind == "projection_budget")
        .expect("S4 projection budget event");
    let metric = |name: &str| {
        budget_event
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix(name))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("numeric projection metric")
    };
    assert_eq!(metric("folded_node_count:"), 1);
    assert!(
        metric("projection_bytes_before_strategy:") > metric("projection_bytes_after_strategy:")
    );
    assert!(
        metric("node_detail_bytes_before_strategy:") > metric("node_detail_bytes_after_strategy:")
    );
    assert_eq!(
        metric("skeleton_bytes_before_strategy:"),
        metric("skeleton_bytes_after_strategy:")
    );

    let (expanded, _) = state
        .expand_node_details_for_main(
            owner,
            vec!["node-1".into()],
            "call-expand-visible".into(),
            "task-event-expand-visible".into(),
        )
        .expect("expand folded details");
    assert!(folded_context.contains(&expanded[0].detail_ref));
    assert!(folded_context.contains(&expanded[0].detail_sha256));
    let expanded_context = state
        .build_developer_context()
        .expect("expanded projection context");
    assert!(expanded_context.contains(&format!(
        "detail_state=expanded expansion_event_id={}",
        expanded[0].expansion_event_id
    )));
    assert!(expanded_context.contains("task-event-read-1 node=node-1"));
    assert!(!expanded_context.contains("detail_state=folded frontier_distance=3"));
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
    assert!(projection.contains("node_details"));
    assert!(projection.contains("tier=D1 evidence=P0"));
    assert!(projection.contains("content_sha256="));
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
fn active_projection_tiers_node_details_by_graph_distance() {
    let nodes = vec![
        inspect_node("a"),
        ActionMapInitializeNodeInput {
            id: "b".into(),
            kind: NodeKind::ImplementSolution,
            title: "B".into(),
            context_summary: "B goal".into(),
            dependency_node_ids: vec!["a".into()],
        },
        ActionMapInitializeNodeInput {
            id: "c".into(),
            kind: NodeKind::SmokeTest,
            title: "C".into(),
            context_summary: "C goal".into(),
            dependency_node_ids: vec!["b".into()],
        },
        ActionMapInitializeNodeInput {
            id: "d".into(),
            kind: NodeKind::FinalSynthesis,
            title: "D".into(),
            context_summary: "D goal".into(),
            dependency_node_ids: vec!["c".into()],
        },
    ];
    let (mut state, owner, outcome) = initialized_state(nodes, "a");
    state.current_main_node_id = Some("d".into());
    let map = state.maps.get_mut(&outcome.map_id).expect("active map");
    for (index, node_id) in ["a", "b", "c", "d"].into_iter().enumerate() {
        let node = map.nodes.get_mut(node_id).expect("node");
        node.status = if node_id == "d" {
            NodeStatus::Running
        } else {
            NodeStatus::Completed
        };
        let event_id = format!("node-event-{}", index + 1);
        node.node_events.push(NodeEventRef {
            id: event_id.clone(),
            kind: "tool_result".into(),
        });
        map.node_events.insert(
            event_id.clone(),
            NodeEvent {
                id: event_id,
                map_id: outcome.map_id.clone(),
                node_id: node_id.into(),
                event_kind: "tool_result".into(),
                source: "main_tool".into(),
                action_class: Some(ActionClass::Read),
                tool_success: Some(true),
                content_sha256: format!("hash-{node_id}"),
                source_event_id: Some(format!("task-event-{node_id}")),
                raw_ref: Some(format!("output-ref-{node_id}")),
                artifact_refs: vec![format!("src/{node_id}.rs")],
                call_id: Some(format!("call-{node_id}")),
                source_thread_id: owner,
                created_at_ms: index as i64,
            },
        );
    }

    let projection = state.build_developer_context().expect("projection");
    for expected in [
        "task-event-a node=a tier=D3 evidence=P1",
        "task-event-b node=b tier=D2 evidence=P1",
        "task-event-c node=c tier=D1 evidence=P1",
        "task-event-d node=d tier=D1 evidence=P1",
    ] {
        assert!(
            projection.contains(expected),
            "missing {expected}: {projection}"
        );
    }
    assert!(projection.contains("active_frontier:\n    - d"));
}

#[test]
fn active_projection_reports_skeleton_over_budget_without_partial_map() {
    let (mut state, _, outcome) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let map = state.maps.get_mut(&outcome.map_id).expect("active map");
    for index in 0..1_000 {
        let id = format!("large-node-{index}");
        map.nodes.insert(
            id.clone(),
            MapNode {
                id,
                title: format!("Large node {index}"),
                kind: NodeKind::Custom,
                status: NodeStatus::Pending,
                context: NodeContext {
                    summary: format!("{}-{index}", "detailed-goal".repeat(12)),
                    source_refs: Vec::new(),
                },
                active_lease: None,
                result_context: Vec::new(),
                node_events: Vec::new(),
                origin_node_id: None,
            },
        );
    }

    let projection = state
        .build_developer_context()
        .expect("budget error context");
    assert!(projection.contains("TaskSpaceMapProjectionErrorV1"));
    assert!(projection.contains("error: map_skeleton_over_budget"));
    assert!(!projection.contains("ContextProjectionV1 epoch snapshot:"));
    assert!(!projection.contains("large-node-999"));
    let snapshot = state.snapshot();
    let budget_event = snapshot
        .trace_events
        .iter()
        .find(|event| event.kind == "projection_budget")
        .expect("projection budget trace event");
    for tag_prefix in [
        "projection_bytes:",
        "skeleton_projection_bytes:",
        "projection_header_bytes:",
        "projection_root_source_bytes:",
        "projection_frontier_bytes:",
        "projection_node_bytes:",
        "projection_edge_bytes:",
        "projection_detail_bytes:",
        "projection_footer_bytes:",
        "folded_node_count:",
        "node_detail_bytes_before_strategy:",
        "node_detail_bytes_after_strategy:",
        "skeleton_bytes_before_strategy:",
        "skeleton_bytes_after_strategy:",
    ] {
        assert!(
            budget_event
                .tags
                .iter()
                .any(|tag| tag.starts_with(tag_prefix)),
            "missing {tag_prefix} in {:?}",
            budget_event.tags
        );
    }
}

#[test]
fn expanded_details_over_budget_fail_explicitly_without_automatic_refold() {
    let (mut state, owner, outcome) = initialized_five_node_chain();
    state
        .expand_node_details_for_main(
            owner,
            vec!["node-1".into()],
            "call-expand-budget".into(),
            "task-event-expand-budget".into(),
        )
        .expect("expand folded details");
    let task = state.tasks.get(&outcome.task_id).expect("task");
    let map = state.maps.get(&outcome.map_id).expect("map");
    let mut unbounded_context = String::new();
    let unbounded = append_context_projection_active(
        &mut unbounded_context,
        task,
        map,
        state.current_main_node_id.as_deref(),
        None,
    )
    .expect("unbounded projection");
    assert!(unbounded.stats.estimated_tokens > unbounded.stats.skeleton_estimated_tokens);
    let max_projection_tokens = unbounded.stats.estimated_tokens - 1;
    state
        .active_budget
        .as_mut()
        .expect("active budget")
        .max_projection_tokens = max_projection_tokens;

    let context = state
        .build_developer_context()
        .expect("projection over-budget context");

    assert!(context.contains("error: map_projection_over_budget"));
    assert!(context.contains("automatic_refold_of_expanded_nodes: false"));
    assert!(!context.contains("ContextProjectionV1 epoch snapshot:"));
    let map = state.maps.get(&outcome.map_id).expect("map after failure");
    assert!(
        map.nodes["node-1"]
            .node_events
            .iter()
            .any(|event| event.kind == NODE_DETAIL_EXPANDED_EVENT_KIND)
    );
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

#[test]
fn terminal_finish_chain_commits_declared_order_atomically() {
    let verify = ActionMapInitializeNodeInput {
        id: "verify".to_string(),
        kind: NodeKind::RegressionTest,
        title: "Verify".to_string(),
        context_summary: "Run regression tests.".to_string(),
        dependency_node_ids: vec!["inspect".to_string()],
    };
    let (mut state, owner, outcome) =
        initialized_state(vec![inspect_node("inspect"), verify], "inspect");

    let (steps, _) = state
        .finish_main_node_chain_with_terminal_candidate(
            owner,
            &["inspect".to_string(), "verify".to_string()],
            "Agent conclusion event.".to_string(),
            "Exact Agent final.",
        )
        .expect("terminal chain");

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].0, "inspect");
    assert_eq!(steps[0].1.next_node_id.as_deref(), Some("verify"));
    assert_eq!(steps[1].0, "verify");
    assert!(steps[1].1.next_node_id.is_none());
    let map = state.maps.get(&outcome.map_id).expect("completed map");
    assert!(
        map.nodes
            .values()
            .all(|node| node.status == NodeStatus::Completed)
    );
    assert_eq!(map.status, MapStatus::Completed);
    let control = state
        .control_state(Some(&outcome.map_id))
        .expect("completed control state");
    assert_eq!(control.task_status, "completed");
    assert_eq!(control.map_status, "completed");
    assert!(control.current_node_id.is_none());
    assert!(control.open_node_ids.is_empty());
    assert_eq!(control.completed_node_count, 2);
}

#[test]
fn rejected_terminal_finish_chain_has_zero_partial_state() {
    let implement = ActionMapInitializeNodeInput {
        id: "implement".to_string(),
        kind: NodeKind::ImplementSolution,
        title: "Implement".to_string(),
        context_summary: "Apply the change.".to_string(),
        dependency_node_ids: vec!["inspect".to_string()],
    };
    let verify = ActionMapInitializeNodeInput {
        id: "verify".to_string(),
        kind: NodeKind::RegressionTest,
        title: "Verify".to_string(),
        context_summary: "Run regression tests.".to_string(),
        dependency_node_ids: vec!["implement".to_string()],
    };
    let (mut state, owner, _) =
        initialized_state(vec![inspect_node("inspect"), implement, verify], "inspect");
    let before = state.snapshot();

    state
        .finish_main_node_chain_with_terminal_candidate(
            owner,
            &["inspect".to_string(), "verify".to_string()],
            "Agent conclusion event.".to_string(),
            "Invalid final.",
        )
        .expect_err("pending verify must reject chain");

    assert_eq!(state.snapshot(), before);
}

#[test]
fn duplicate_terminal_finish_chain_has_zero_partial_state() {
    let (mut state, owner, _) = initialized_state(vec![inspect_node("inspect")], "inspect");
    let before = state.snapshot();

    let error = state
        .finish_main_node_chain_with_terminal_candidate(
            owner,
            &["inspect".to_string(), "inspect".to_string()],
            "Agent conclusion event.".to_string(),
            "Invalid final.",
        )
        .expect_err("duplicate chain");

    assert!(error.contains("duplicate node"));
    assert_eq!(state.snapshot(), before);
}
