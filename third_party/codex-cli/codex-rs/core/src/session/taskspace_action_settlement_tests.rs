use std::sync::Arc;
use std::sync::atomic::Ordering;

use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeAction;
use codex_protocol::taskspace::TaskSpaceNodeState;
use uuid::Uuid;

use super::Session;
use super::TaskSpaceActionSettlementFact;
use super::turn_context::TurnContext;

fn node(
    node_id: &str,
    parents: Vec<String>,
    state: TaskSpaceNodeState,
    actions: Vec<TaskSpaceNodeAction>,
) -> TaskSpaceMapNode {
    TaskSpaceMapNode {
        node_id: node_id.into(),
        goal: node_id.into(),
        state,
        content: String::new(),
        parents,
        actions,
    }
}

pub(super) fn pending_map(map_id: &str) -> TaskSpaceCanonicalMap {
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id: map_id.into(),
        root: node("root", Vec::new(), TaskSpaceNodeState::InFlight, Vec::new()),
        work_nodes: vec![node(
            "work",
            vec!["root".into()],
            TaskSpaceNodeState::Ready,
            vec![TaskSpaceNodeAction {
                action_id: "outer/taskspace/call/0".into(),
                tool_name: "inspect".into(),
                outcome: TaskSpaceActionOutcome::Pending,
            }],
        )],
        finish: node(
            "finish",
            vec!["work".into()],
            TaskSpaceNodeState::Waiting,
            Vec::new(),
        ),
        revision: 1,
    }
}

async fn persisted_session() -> (std::path::PathBuf, Arc<Session>, Arc<TurnContext>, String) {
    let home = std::env::temp_dir().join(format!("taskspace-recovery-{}", Uuid::new_v4()));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".into())
        .await
        .expect("initialize state DB");
    let (mut session, turn) = super::tests::make_session_and_context().await;
    session.services.state_db = Some(state_db);
    let session = Arc::new(session);
    session
        .taskspace_action_settlements
        .start(Arc::downgrade(&session));
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
    (home, session, Arc::new(turn), map_id)
}

fn persisted_feedback(map_id: &str, node_id: &str) -> ResponseItem {
    persisted_feedback_with_identity(
        map_id,
        node_id,
        "outer",
        "outer/taskspace/call/0",
        "succeeded",
    )
}

fn persisted_feedback_with_identity(
    map_id: &str,
    node_id: &str,
    outer_call_id: &str,
    action_id: &str,
    outcome: &str,
) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: "outer".into(),
        output: FunctionCallOutputPayload::from_text(
            serde_json::json!({
                "kind": "taskspace_exec_result",
                "status": "completed",
                "outer_call_id": outer_call_id,
                "map_id": map_id,
                "map_revision_at_dispatch": 2,
                "reads": [],
                "client_results": [{
                    "call_index": 0,
                    "action_id": action_id,
                    "node_id": node_id,
                    "tool": "inspect",
                    "outcome": outcome
                }],
                "provider_attributions": []
            })
            .to_string(),
        ),
    }
}

pub(super) fn action_outcome<'a>(
    map: &'a codex_protocol::protocol::ActionMapSnapshotMap,
    node_id: &str,
) -> &'a str {
    map.nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("target node")
        .actions
        .first()
        .expect("target Action")
        .outcome
        .as_str()
}

#[tokio::test]
async fn recovery_settles_only_persisted_pending_action_without_replaying_tool() {
    let (home, session, turn, map_id) = persisted_session().await;
    session
        .record_conversation_items(&turn, &[persisted_feedback(&map_id, "work")])
        .await;

    session
        .recover_taskspace_action_settlements()
        .await
        .expect("recover persisted result");
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

#[tokio::test]
async fn recovery_rejects_mismatched_attribution_before_map_mutation() {
    let (home, session, turn, map_id) = persisted_session().await;
    session
        .record_conversation_items(&turn, &[persisted_feedback(&map_id, "other")])
        .await;

    let error = session
        .recover_taskspace_action_settlements()
        .await
        .expect_err("wrong attribution must fail recovery");
    assert!(error.contains("attribution mismatch"));
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "pending");
    assert!(
        !session
            .taskspace_action_recovery_scanned
            .load(Ordering::SeqCst)
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn recovery_rejects_outer_and_action_identity_mismatch_before_map_mutation() {
    let (home, session, turn, map_id) = persisted_session().await;
    session
        .record_conversation_items(
            &turn,
            &[persisted_feedback_with_identity(
                &map_id,
                "work",
                "different-outer",
                "outer/taskspace/call/0",
                "succeeded",
            )],
        )
        .await;

    let error = session
        .recover_taskspace_action_settlements()
        .await
        .expect_err("outer/action mismatch must fail recovery");
    assert!(error.contains("identity mismatch"));
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "pending");
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn recovery_rejects_conflicting_terminal_facts_before_enqueue() {
    let (home, session, turn, map_id) = persisted_session().await;
    session
        .record_conversation_items(
            &turn,
            &[
                persisted_feedback_with_identity(
                    &map_id,
                    "work",
                    "outer",
                    "outer/taskspace/call/0",
                    "succeeded",
                ),
                persisted_feedback_with_identity(
                    &map_id,
                    "work",
                    "outer",
                    "outer/taskspace/call/0",
                    "failed",
                ),
            ],
        )
        .await;

    let error = session
        .recover_taskspace_action_settlements()
        .await
        .expect_err("conflicting terminal facts must fail recovery");
    assert!(error.contains("conflicting facts"));
    session
        .await_taskspace_action_settlements()
        .await
        .expect("validation must happen before enqueue");
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "pending");
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn settlement_survives_cancellation_of_the_producing_future() {
    let (home, session, _turn, map_id) = persisted_session().await;
    let (queued_tx, queued_rx) = tokio::sync::oneshot::channel();
    let producer_session = Arc::clone(&session);
    let producer = tokio::spawn(async move {
        producer_session
            .enqueue_taskspace_action_settlement(TaskSpaceActionSettlementFact {
                map_id,
                outer_call_id: "outer".into(),
                action_id: "outer/taskspace/call/0".into(),
                node_ids: vec!["work".into()],
                tool_name: "inspect".into(),
                outcome: TaskSpaceActionOutcome::Succeeded,
            })
            .expect("enqueue settlement");
        let _ = queued_tx.send(());
        std::future::pending::<()>().await;
    });
    queued_rx.await.expect("producer queued settlement");
    producer.abort();

    session
        .await_taskspace_action_settlements()
        .await
        .expect("Session-owned settlement must survive producer cancellation");
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "succeeded");
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn settlement_failure_remains_visible_at_every_request_barrier() {
    let (home, session, _turn, map_id) = persisted_session().await;
    session
        .enqueue_taskspace_action_settlement(TaskSpaceActionSettlementFact {
            map_id,
            outer_call_id: "outer".into(),
            action_id: "outer/taskspace/call/0".into(),
            node_ids: vec!["wrong".into()],
            tool_name: "inspect".into(),
            outcome: TaskSpaceActionOutcome::Succeeded,
        })
        .expect("enqueue invalid settlement fact");

    let first = session
        .await_taskspace_action_settlements()
        .await
        .expect_err("barrier must expose settlement failure");
    let second = session
        .await_taskspace_action_settlements()
        .await
        .expect_err("failure must remain visible");
    assert!(first.contains("attribution mismatch"));
    assert_eq!(second, first);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn provider_preparation_stops_on_permanent_settlement_failure() {
    let (home, session, turn, map_id) = persisted_session().await;
    session
        .enqueue_taskspace_action_settlement(TaskSpaceActionSettlementFact {
            map_id,
            outer_call_id: "outer".into(),
            action_id: "outer/taskspace/call/0".into(),
            node_ids: vec!["wrong".into()],
            tool_name: "inspect".into(),
            outcome: TaskSpaceActionOutcome::Succeeded,
        })
        .expect("enqueue invalid settlement fact");

    let initial_context = session.build_initial_context(&turn).await;
    let error = match session
        .prepare_provider_visible_prompt_items(&turn, initial_context)
        .await
    {
        Ok(_) => panic!("provider preparation must stop before transport"),
        Err(error) => error,
    };
    assert!(error.contains("TaskSpace settlement barrier failed"));
    assert!(error.contains("attribution mismatch"));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn graceful_shutdown_waits_for_producer_then_settlement() {
    let (home, session, _turn, map_id) = persisted_session().await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let producer_session = Arc::clone(&session);
    let producer = session
        .spawn_taskspace_action_producer(async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            producer_session
                .enqueue_taskspace_action_settlement(TaskSpaceActionSettlementFact {
                    map_id,
                    outer_call_id: "outer".into(),
                    action_id: "outer/taskspace/call/0".into(),
                    node_ids: vec!["work".into()],
                    tool_name: "inspect".into(),
                    outcome: TaskSpaceActionOutcome::Succeeded,
                })
                .expect("enqueue settlement before producer exit");
        })
        .expect("register producer");
    drop(producer);
    started_rx.await.expect("producer started");

    let shutdown_session = Arc::clone(&session);
    let shutdown = tokio::spawn(async move {
        super::handlers::shutdown(&shutdown_session, "shutdown".into()).await
    });
    tokio::task::yield_now().await;
    assert!(!shutdown.is_finished());
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            match session.spawn_taskspace_action_producer(async {}) {
                Ok(producer) => producer.await.expect("temporary producer"),
                Err(_) => break,
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("shutdown must close producer admission");
    release_tx.send(()).expect("release producer");
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), shutdown)
            .await
            .expect("shutdown timeout")
            .expect("shutdown task")
    );

    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "succeeded");
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn shutdown_flag_prevents_pending_work_from_starting_a_new_turn() {
    let (home, session, _turn, _map_id) = persisted_session().await;
    session
        .queue_response_items_for_next_turn(vec![ResponseInputItem::Message {
            role: "assistant".into(),
            content: vec![ContentItem::InputText {
                text: "queued before shutdown".into(),
            }],
        }])
        .await;
    session
        .shutting_down
        .store(true, std::sync::atomic::Ordering::SeqCst);

    session.maybe_start_turn_for_pending_work().await;

    assert!(session.active_turn.lock().await.is_none());
    assert!(session.has_queued_response_items_for_next_turn().await);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn graceful_shutdown_exits_after_reporting_settlement_failure() {
    let (home, session, _turn, map_id) = persisted_session().await;
    session
        .enqueue_taskspace_action_settlement(TaskSpaceActionSettlementFact {
            map_id,
            outer_call_id: "outer".into(),
            action_id: "outer/taskspace/call/0".into(),
            node_ids: vec!["wrong".into()],
            tool_name: "inspect".into(),
            outcome: TaskSpaceActionOutcome::Succeeded,
        })
        .expect("enqueue invalid settlement");

    assert!(super::handlers::shutdown(&session, "shutdown".into()).await);
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("Map snapshot")
        .map
        .expect("canonical Map");
    assert_eq!(action_outcome(&map, "work"), "pending");
    let _ = tokio::fs::remove_dir_all(home).await;
}
