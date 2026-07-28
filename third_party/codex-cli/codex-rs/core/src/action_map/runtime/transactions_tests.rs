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
    let prepared_receipt = runtime
        .response_final_receipt_for_main(&prepared, "control-call-2")
        .expect("prepared receipt");
    assert!(!prepared_receipt.complete());
    assert_eq!(prepared_receipt.attributed_result_count, 0);
    assert_eq!(prepared_receipt.outstanding_reservation_count, 2);
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
    let final_receipt = runtime
        .response_final_receipt_for_main(&prepared, "control-call-2")
        .expect("final receipt");
    assert!(final_receipt.complete());
    assert_eq!(final_receipt.canonical_revision, Some(map.revision));
    assert_eq!(final_receipt.attributed_result_count, 2);
    assert_eq!(final_receipt.outstanding_reservation_count, 0);
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

#[test]
fn runtime_close_reopen_close_preserves_one_map_and_terminal_history() {
    let owner = ThreadId::new();
    let mut runtime = ActionMapRuntimeState::default();
    runtime
        .restore_store_map("store-map-lifecycle", owner, None)
        .expect("restore empty identity");
    let (prepared, _) = runtime
        .prepare_response_for_main(
            owner,
            "control-initialize",
            initialize_operation(),
            vec![declared_call("call-inspect", "inspect", "read_file")],
        )
        .expect("initialize");
    runtime
        .release_main_action_result(
            owner,
            &prepared.prepared_calls[0],
            true,
            "tool-result://call/call-inspect".into(),
        )
        .expect("release inspect");
    let revision = runtime.canonical_map_for_store().expect("map").revision;
    let (first_close, _) = runtime
        .finish_map_for_main(
            owner,
            revision,
            "finish".into(),
            vec!["inspect".into()],
            "Initial work complete".into(),
            "terminal-action-1".into(),
        )
        .expect("first close");
    assert_eq!(first_close.completed_work_node_ids, vec!["inspect"]);

    let revision = runtime
        .canonical_map_for_store()
        .expect("closed map")
        .revision;
    let (reopened, _) = runtime
        .prepare_response_for_main(
            owner,
            "control-reopen",
            ActionMapResponseOperation::Reopen {
                expected_revision: revision,
                work_nodes: vec![map_node(
                    "address-feedback",
                    "address user feedback",
                    Vec::new(),
                )],
                edges: vec![
                    MapEdge {
                        from: "root".into(),
                        to: "address-feedback".into(),
                    },
                    MapEdge {
                        from: "address-feedback".into(),
                        to: "finish".into(),
                    },
                ],
            },
            vec![declared_call(
                "call-feedback",
                "address-feedback",
                "read_file",
            )],
        )
        .expect("reopen");
    assert_eq!(reopened.action, "reopen_map");
    runtime
        .release_main_action_result(
            owner,
            &reopened.prepared_calls[0],
            true,
            "tool-result://call/call-feedback".into(),
        )
        .expect("release feedback action");

    let reopened_map = runtime.canonical_map_for_store().expect("reopened map");
    assert_eq!(reopened_map.map_id, "store-map-lifecycle");
    assert!(reopened_map.terminal_record.is_none());
    assert_eq!(reopened_map.terminal_history.len(), 1);
    assert!(reopened_map.completion_records.contains_key("inspect"));
    assert!(!reopened_map.completion_records.contains_key("root"));
    assert!(!reopened_map.completion_records.contains_key("finish"));

    let revision = reopened_map.revision;
    let (second_close, _) = runtime
        .finish_map_for_main(
            owner,
            revision,
            "finish".into(),
            vec!["address-feedback".into()],
            "Feedback addressed".into(),
            "terminal-action-2".into(),
        )
        .expect("second close");
    assert_eq!(second_close.map_id, "store-map-lifecycle");
    let closed_map = runtime.canonical_map_for_store().expect("closed map");
    assert_eq!(closed_map.terminal_history.len(), 1);
    assert_eq!(
        closed_map
            .terminal_record
            .as_ref()
            .expect("current terminal")
            .summary_ref,
        "Feedback addressed"
    );
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
