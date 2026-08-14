use std::sync::Arc;

use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MapRuntimeMode;
use uuid::Uuid;

use super::taskspace_action_settlement_tests::action_outcome;
use super::taskspace_action_settlement_tests::pending_map;

#[tokio::test]
async fn recovery_follows_large_feedback_output_reference() {
    let home = std::env::temp_dir().join(format!("taskspace-output-ref-{}", Uuid::new_v4()));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".into())
        .await
        .expect("initialize state DB");
    let (mut session, turn) = super::tests::make_session_and_context().await;
    session.services.state_db = Some(state_db);
    let rollout_path = super::tests::attach_thread_persistence(&mut session).await;
    let session = Arc::new(session);
    session.start_taskspace_action_settlements_for_test();
    let (activation, _) = session
        .set_persisted_action_map_mode(MapRuntimeMode::Experiment)
        .await
        .expect("activate TaskSpace");
    let map_id = activation.active_map_id.expect("Map identity");
    let candidate = pending_map(&map_id);
    let install_id = map_id.clone();
    session
        .mutate_canonical_action_map("test_install_pending", move |runtime, owner| {
            let result = runtime.restore_store_map(&install_id, owner, Some(candidate.clone()));
            (result, Vec::new())
        })
        .await
        .expect("persist pending Map")
        .0
        .expect("install pending Map");

    let raw_feedback = serde_json::json!({
        "kind": "taskspace_exec_result",
        "status": "completed",
        "outer_call_id": "outer",
        "map_id": map_id,
        "map_revision_at_dispatch": 2,
        "reads": [],
        "client_results": [{
            "call_index": 0,
            "action_id": "outer/taskspace/call/0",
            "node_id": "work",
            "tool": "inspect",
            "outcome": "succeeded"
        }],
        "provider_attributions": [],
        "large_native_tool_output": "x".repeat(64 * 1024)
    })
    .to_string();
    let artifact_ref = crate::tools::output_reference::write_output_artifact_for_rollout(
        Some(&rollout_path),
        raw_feedback.as_bytes(),
    )
    .await
    .expect("persist full output artifact")
    .expect("large output must use an artifact reference");
    let reference_text = crate::tools::output_reference::reference_text_for_raw_output(
        raw_feedback.as_bytes(),
        Some(&artifact_ref),
    )
    .expect("large output must be folded for the provider");
    session
        .record_conversation_items(
            &turn,
            &[ResponseItem::FunctionCallOutput {
                call_id: "outer".into(),
                output: FunctionCallOutputPayload::from_text(reference_text),
            }],
        )
        .await;

    session
        .recover_taskspace_action_settlements()
        .await
        .expect("recover settlement through output reference");
    session
        .await_taskspace_action_settlements()
        .await
        .expect("settlement barrier");
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "succeeded");
    let _ = tokio::fs::remove_dir_all(home).await;
}
