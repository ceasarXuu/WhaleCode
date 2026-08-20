use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionEventSink;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::ExtensionWarning;
use codex_extension_api::NoopTurnItemEmitter;
use codex_extension_api::PreviousWorldStateSection;
use codex_extension_api::ThreadStartInput;
use codex_extension_api::ToolBatchCall;
use codex_extension_api::ToolBatchPreflightInput;
use codex_extension_api::ToolCallOutcome;
use codex_extension_api::ToolCallSource;
use codex_extension_api::ToolFinishInput;
use codex_extension_api::WorldStateContributionInput;
use codex_protocol::ThreadId;
use codex_protocol::ToolName;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_tools::ToolCall;
use codex_tools::ToolPayload;
use codex_utils_output_truncation::TruncationPolicy;

use crate::TaskSpaceMapBinding;
use crate::TaskSpaceMapCommit;
use crate::TaskSpaceMapRecord;
use crate::TaskSpaceMapRelation;
use crate::TaskSpaceMapWriteOutcome;
use crate::TaskSpaceStore;
use crate::TaskSpaceStoreFuture;
use crate::install;
use crate::model::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use crate::model::TaskSpaceMap;
use crate::model::TaskSpaceMapEdge;
use crate::model::TaskSpaceMapNode;
use crate::runtime::TaskSpaceRuntimeHandle;

#[derive(Default)]
struct FakeStore {
    records: Mutex<HashMap<ThreadId, (TaskSpaceMapRecord, TaskSpaceMapBinding)>>,
}

#[derive(Default)]
struct RecordingEventSink {
    events: Mutex<Vec<Event>>,
}

impl ExtensionEventSink for RecordingEventSink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }

    fn emit_warning(&self, _warning: ExtensionWarning) {}
}

impl FakeStore {
    fn seed(&self, thread_id: ThreadId, record: TaskSpaceMapRecord) {
        self.records.lock().unwrap().insert(
            thread_id,
            (
                record.clone(),
                TaskSpaceMapBinding {
                    thread_id,
                    map_id: record.map.map_id,
                    relation: TaskSpaceMapRelation::Owner,
                    parent_thread_id: None,
                },
            ),
        );
    }
}

impl TaskSpaceStore for FakeStore {
    fn load_for_thread(
        &self,
        thread_id: ThreadId,
    ) -> TaskSpaceStoreFuture<'_, Option<(TaskSpaceMapRecord, TaskSpaceMapBinding)>> {
        Box::pin(async move { Ok(self.records.lock().unwrap().get(&thread_id).cloned()) })
    }

    fn bind(&self, binding: TaskSpaceMapBinding) -> TaskSpaceStoreFuture<'_, ()> {
        Box::pin(async move {
            let mut records = self.records.lock().unwrap();
            let record = records
                .values()
                .find(|(record, _)| record.map.map_id == binding.map_id)
                .map(|(record, _)| record.clone())
                .ok_or_else(|| anyhow::anyhow!("missing map"))?;
            records.insert(binding.thread_id, (record, binding));
            Ok(())
        })
    }

    fn compare_and_swap(
        &self,
        commit: TaskSpaceMapCommit,
    ) -> TaskSpaceStoreFuture<'_, TaskSpaceMapWriteOutcome> {
        Box::pin(async move {
            let mut records = self.records.lock().unwrap();
            let current = records
                .values()
                .find(|(record, _)| record.map.map_id == commit.map.map_id)
                .map(|(record, _)| record.clone());
            let Some(current) = current else {
                if commit.expected_store_revision != 0 {
                    return Ok(TaskSpaceMapWriteOutcome::Conflict(None));
                }
                let applied = TaskSpaceMapRecord {
                    map: commit.map,
                    owner_thread_id: commit.owner_thread_id,
                    canonical_sha256: "test-created".into(),
                    store_revision: 1,
                };
                records.insert(
                    commit.owner_thread_id,
                    (
                        applied.clone(),
                        TaskSpaceMapBinding {
                            thread_id: commit.owner_thread_id,
                            map_id: applied.map.map_id.clone(),
                            relation: TaskSpaceMapRelation::Owner,
                            parent_thread_id: None,
                        },
                    ),
                );
                return Ok(TaskSpaceMapWriteOutcome::Applied(applied));
            };
            if current.store_revision != commit.expected_store_revision {
                return Ok(TaskSpaceMapWriteOutcome::Conflict(Some(current)));
            }
            let applied = TaskSpaceMapRecord {
                map: commit.map,
                owner_thread_id: commit.owner_thread_id,
                canonical_sha256: "test-updated".into(),
                store_revision: current.store_revision + 1,
            };
            for (record, _) in records.values_mut() {
                if record.map.map_id == applied.map.map_id {
                    *record = applied.clone();
                }
            }
            Ok(TaskSpaceMapWriteOutcome::Applied(applied))
        })
    }
}

fn record(map_id: &str, owner: ThreadId) -> TaskSpaceMapRecord {
    TaskSpaceMapRecord {
        map: TaskSpaceMap {
            schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
            map_id: map_id.into(),
            root: TaskSpaceMapNode {
                node_id: "root".into(),
                goal: "deliver".into(),
                source_refs: vec![],
            },
            work_nodes: vec![TaskSpaceMapNode {
                node_id: "work".into(),
                goal: "implement".into(),
                source_refs: vec![],
            }],
            finish: TaskSpaceMapNode {
                node_id: "finish".into(),
                goal: "close".into(),
                source_refs: vec![],
            },
            edges: vec![
                TaskSpaceMapEdge {
                    from: "root".into(),
                    to: "work".into(),
                },
                TaskSpaceMapEdge {
                    from: "work".into(),
                    to: "finish".into(),
                },
            ],
            completion_records: BTreeMap::new(),
            block_records: BTreeMap::new(),
            action_reservations: BTreeMap::new(),
            result_refs: BTreeMap::new(),
            evidence_refs: BTreeMap::new(),
            terminal_record: None,
            terminal_history: vec![],
            revision: 1,
        },
        owner_thread_id: owner,
        canonical_sha256: "test".into(),
        store_revision: 1,
    }
}

#[tokio::test]
async fn install_rehydrates_child_tool_and_world_state_without_core_hooks() {
    let parent = ThreadId::new();
    let child = ThreadId::new();
    let store = Arc::new(FakeStore::default());
    store.seed(parent, record("map-1", parent));
    let mut builder = ExtensionRegistryBuilder::<()>::new();
    install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(child.to_string());
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: parent,
        depth: 1,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
    });
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &source,
            forked_from_thread_id: None,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;

    let runtime = thread_store
        .get::<TaskSpaceRuntimeHandle>()
        .expect("runtime should exist");
    assert!(runtime.is_active());
    assert_eq!(runtime.record().await.unwrap().map.map_id, "map-1");
    assert_eq!(
        store
            .load_for_thread(child)
            .await
            .unwrap()
            .unwrap()
            .1
            .relation,
        TaskSpaceMapRelation::Child
    );

    let tools = registry.tool_contributors()[0].tools(&session_store, &thread_store);
    assert_eq!(tools.len(), 1);
    let payload = ToolPayload::Function {
        arguments: serde_json::json!({"action": "read_map"}).to_string(),
    };
    let output = tools[0]
        .handle(ToolCall {
            turn_id: "turn-1".into(),
            call_id: "call-1".into(),
            tool_name: ToolName::plain("taskspace_control"),
            model: "test".into(),
            codex_turn_metadata: None,
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: Default::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            environments: vec![],
            payload: payload.clone(),
        })
        .await
        .unwrap();
    assert_eq!(output.code_mode_result(&payload)["map"]["map_id"], "map-1");

    let turn_store = ExtensionData::new("turn-1");
    let sections = registry.context_contributors()[0]
        .contribute_world_state(WorldStateContributionInput {
            thread_id: child,
            turn_id: "turn-1",
            environments: &[],
            ready_selected_capability_roots: &[],
            executor_capability_discovery: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
        })
        .await;
    assert_eq!(sections.len(), 1);
    assert!(
        sections[0]
            .render_diff(PreviousWorldStateSection::Absent)
            .unwrap()
            .body()
            .contains("map-1")
    );
    assert!(
        sections[0]
            .render_diff(PreviousWorldStateSection::Known(sections[0].snapshot()))
            .is_none()
    );
}

#[tokio::test]
async fn regular_fork_inherits_with_fork_relation() {
    let source = ThreadId::new();
    let forked = ThreadId::new();
    let store = Arc::new(FakeStore::default());
    store.seed(source, record("map-fork", source));
    let mut builder = ExtensionRegistryBuilder::<()>::new();
    install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(forked.to_string());

    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &SessionSource::Cli,
            forked_from_thread_id: Some(source),
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;

    let runtime = thread_store
        .get::<TaskSpaceRuntimeHandle>()
        .expect("runtime should exist");
    assert!(runtime.is_active());
    let (_, binding) = store
        .load_for_thread(forked)
        .await
        .unwrap()
        .expect("fork binding should exist");
    assert_eq!(binding.relation, TaskSpaceMapRelation::Fork);
    assert_eq!(binding.parent_thread_id, Some(source));
}

#[tokio::test]
async fn unbound_standard_thread_exposes_no_taskspace_surface() {
    let store = Arc::new(FakeStore::default());
    let thread_id = ThreadId::new();
    let mut builder = ExtensionRegistryBuilder::<()>::new();
    install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(thread_id.to_string());
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &SessionSource::Cli,
            forked_from_thread_id: None,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;

    assert!(
        registry.tool_contributors()[0]
            .tools(&session_store, &thread_store)
            .is_empty()
    );
    let turn_store = ExtensionData::new("turn-1");
    assert!(
        registry.context_contributors()[0]
            .contribute_world_state(WorldStateContributionInput {
                thread_id,
                turn_id: "turn-1",
                environments: &[],
                ready_selected_capability_roots: &[],
                executor_capability_discovery: None,
                extension_metrics: None,
                session_store: &session_store,
                thread_store: &thread_store,
                turn_store: &turn_store,
            })
            .await
            .is_empty()
    );

    let calls = vec![
        batch_call(
            "control",
            "taskspace_control",
            serde_json::json!({"action": "read_map"}),
        ),
        batch_call("sibling", "read_file", serde_json::json!({"path": "x"})),
    ];
    registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &calls,
        })
        .await
        .expect("unbound Standard threads must retain the upstream path");
}

#[tokio::test]
async fn explicit_enable_initializes_map_and_first_action_atomically() {
    let store = Arc::new(FakeStore::default());
    let thread_id = ThreadId::new();
    let mut builder = ExtensionRegistryBuilder::<()>::new();
    let service = install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(thread_id.to_string());
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &SessionSource::Cli,
            forked_from_thread_id: None,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;
    assert!(
        registry.tool_contributors()[0]
            .tools(&session_store, &thread_store)
            .is_empty()
    );
    service.set_enabled(thread_id, true).await.unwrap();
    let control = registry.tool_contributors()[0]
        .tools(&session_store, &thread_store)
        .remove(0);
    let turn_store = ExtensionData::new("turn-1");
    let calls = vec![
        batch_call(
            "control",
            "taskspace_control",
            serde_json::json!({
                "action": "initialize_and_execute",
                "root": {"node_id": "root", "goal": "deliver"},
                "work_nodes": [{"node_id": "work", "goal": "implement"}],
                "finish": {"node_id": "finish", "goal": "close"},
                "edges": [
                    {"from": "root", "to": "work"},
                    {"from": "work", "to": "finish"}
                ],
                "actions": [{"node_id": "work", "tool": "read_file"}]
            }),
        ),
        batch_call("sibling", "read_file", serde_json::json!({"path": "x"})),
    ];
    registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &calls,
        })
        .await
        .unwrap();
    let created = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(created.map.revision, 1);
    assert_eq!(created.map.action_reservations.len(), 1);

    let payload = calls[0].payload.clone();
    let receipt = control
        .handle(ToolCall {
            turn_id: "turn-1".into(),
            call_id: "control".into(),
            tool_name: ToolName::plain("taskspace_control"),
            model: "test".into(),
            codex_turn_metadata: None,
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: Default::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            environments: vec![],
            payload: payload.clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        receipt.code_mode_result(&payload)["action"],
        "initialize_and_execute"
    );

    registry.tool_lifecycle_contributors()[0]
        .on_tool_finish(ToolFinishInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            turn_id: "turn-1",
            call_id: "sibling",
            tool_name: &ToolName::plain("read_file"),
            source: ToolCallSource::Direct,
            outcome: ToolCallOutcome::Completed { success: true },
        })
        .await;
    let released = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(released.map.revision, 2);
    assert!(released.map.action_reservations.is_empty());
    service.set_enabled(thread_id, false).await.unwrap();
    let service_state = service.read(thread_id).await.unwrap();
    assert!(!service_state.enabled);
    assert_eq!(service_state.record.unwrap().map.revision, 2);
    let runtime = thread_store.get::<TaskSpaceRuntimeHandle>().unwrap();
    runtime.refresh().await.unwrap();
    assert!(!runtime.is_enabled());
    assert!(
        registry.tool_contributors()[0]
            .tools(&session_store, &thread_store)
            .is_empty()
    );
}

#[tokio::test]
async fn active_taskspace_rejects_read_map_with_sibling_before_dispatch() {
    let store = Arc::new(FakeStore::default());
    let thread_id = ThreadId::new();
    store.seed(thread_id, record("map-1", thread_id));
    let mut builder = ExtensionRegistryBuilder::<()>::new();
    install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(thread_id.to_string());
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &SessionSource::Cli,
            forked_from_thread_id: None,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;
    let turn_store = ExtensionData::new("turn-1");
    let mismatched = vec![
        batch_call(
            "control-bad",
            "taskspace_control",
            serde_json::json!({
                "action": "execute",
                "expected_revision": 1,
                "actions": [{"node_id": "work", "tool": "exec_command"}],
            }),
        ),
        batch_call("sibling-bad", "read_file", serde_json::json!({"path": "x"})),
    ];
    let failure = registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &mismatched,
        })
        .await
        .unwrap_err();
    assert_eq!(failure.code, "taskspace_execute_manifest_invalid");
    assert_eq!(
        store
            .load_for_thread(thread_id)
            .await
            .unwrap()
            .unwrap()
            .0
            .map
            .revision,
        1
    );
    let calls = vec![
        batch_call(
            "control",
            "taskspace_control",
            serde_json::json!({"action": "read_map"}),
        ),
        batch_call("sibling", "read_file", serde_json::json!({"path": "x"})),
    ];

    let failure = registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &calls,
        })
        .await
        .expect_err("read_map sibling must reject the whole batch");

    assert_eq!(failure.code, "taskspace_read_map_has_siblings");
}

#[tokio::test]
async fn finish_and_reopen_commit_through_response_preflight() {
    let store = Arc::new(FakeStore::default());
    let thread_id = ThreadId::new();
    store.seed(thread_id, record("map-lifecycle", thread_id));
    let events = Arc::new(RecordingEventSink::default());
    let mut builder = ExtensionRegistryBuilder::<()>::with_event_sink(events.clone());
    install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(thread_id.to_string());
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &SessionSource::Cli,
            forked_from_thread_id: None,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;
    let control = registry.tool_contributors()[0]
        .tools(&session_store, &thread_store)
        .remove(0);
    let turn_store = ExtensionData::new("turn-lifecycle");
    let invalid_finish_calls = vec![
        batch_call(
            "invalid-finish-control",
            "taskspace_control",
            serde_json::json!({
                "action": "finish_map",
                "expected_revision": 1,
                "finish_node_id": "finish",
                "complete_work_node_ids": ["work"],
                "exact_summary": "must not commit"
            }),
        ),
        batch_call(
            "invalid-sibling",
            "read_file",
            serde_json::json!({"path": "x"}),
        ),
    ];
    let failure = registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &invalid_finish_calls,
        })
        .await
        .unwrap_err();
    assert_eq!(failure.code, "taskspace_finish_map_has_siblings");
    assert_eq!(
        store
            .load_for_thread(thread_id)
            .await
            .unwrap()
            .unwrap()
            .0
            .map
            .revision,
        1
    );
    let finish_calls = vec![batch_call(
        "finish-control",
        "taskspace_control",
        serde_json::json!({
            "action": "finish_map",
            "expected_revision": 1,
            "finish_node_id": "finish",
            "complete_work_node_ids": ["work"],
            "exact_summary": "completed exactly"
        }),
    )];
    registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &finish_calls,
        })
        .await
        .unwrap();
    let closed = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(closed.map.revision, 2);
    assert_eq!(
        closed.map.terminal_record.unwrap().summary_ref,
        "completed exactly"
    );
    let finish_payload = finish_calls[0].payload.clone();
    let finish_receipt = control
        .handle(ToolCall {
            turn_id: "turn-lifecycle".into(),
            call_id: "finish-control".into(),
            tool_name: ToolName::plain("taskspace_control"),
            model: "test".into(),
            codex_turn_metadata: None,
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: Default::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            environments: vec![],
            payload: finish_payload.clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        finish_receipt.code_mode_result(&finish_payload)["action"],
        "finish_map"
    );

    let reopen_calls = vec![
        batch_call(
            "reopen-control",
            "taskspace_control",
            serde_json::json!({
                "action": "reopen_map",
                "expected_revision": 2,
                "work_nodes": [{"node_id": "follow-up", "goal": "address feedback"}],
                "edges": [
                    {"from": "root", "to": "follow-up"},
                    {"from": "follow-up", "to": "finish"}
                ],
                "actions": [{"node_id": "follow-up", "tool": "read_file"}]
            }),
        ),
        batch_call(
            "reopen-sibling",
            "read_file",
            serde_json::json!({"path": "feedback.md"}),
        ),
    ];
    registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &reopen_calls,
        })
        .await
        .unwrap();
    let reopened = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(reopened.map.revision, 3);
    assert!(reopened.map.terminal_record.is_none());
    assert_eq!(reopened.map.terminal_history.len(), 1);
    assert_eq!(reopened.map.action_reservations.len(), 1);

    registry.tool_lifecycle_contributors()[0]
        .on_tool_finish(ToolFinishInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            turn_id: "turn-lifecycle",
            call_id: "reopen-sibling",
            tool_name: &ToolName::plain("read_file"),
            source: ToolCallSource::Direct,
            outcome: ToolCallOutcome::Completed { success: true },
        })
        .await;
    let released = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(released.map.revision, 4);
    assert!(released.map.action_reservations.is_empty());
    let recorded = events.events.lock().unwrap();
    let updates = recorded
        .iter()
        .filter_map(|event| match &event.msg {
            EventMsg::TaskSpaceUpdated(update) => Some((
                event.id.as_str(),
                update.revision,
                update.operation.as_str(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        updates,
        vec![
            ("taskspace:map-lifecycle:2", 2, "finish_map"),
            ("taskspace:map-lifecycle:3", 3, "reopen_map"),
            ("taskspace:map-lifecycle:4", 4, "action_release"),
        ]
    );
}

#[tokio::test]
async fn execute_preflight_commits_receipt_and_releases_sibling_reservation() {
    let store = Arc::new(FakeStore::default());
    let thread_id = ThreadId::new();
    store.seed(thread_id, record("map-1", thread_id));
    let mut builder = ExtensionRegistryBuilder::<()>::new();
    install(&mut builder, store.clone());
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new(thread_id.to_string());
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &(),
            session_source: &SessionSource::Cli,
            forked_from_thread_id: None,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;
    let turn_store = ExtensionData::new("turn-1");
    let calls = vec![
        batch_call(
            "control",
            "taskspace_control",
            serde_json::json!({
                "action": "execute",
                "expected_revision": 1,
                "mutations": [
                    {"action": "add_work_nodes", "work_nodes": [
                        {"node_id": "work-2", "goal": "verify"}
                    ]},
                    {"action": "add_edges", "edges": [
                        {"from": "root", "to": "work-2"},
                        {"from": "work-2", "to": "finish"}
                    ]}
                ],
                "actions": [{"node_id": "work-2", "tool": "read_file"}],
            }),
        ),
        batch_call("sibling", "read_file", serde_json::json!({"path": "x"})),
    ];
    registry
        .preflight_tool_batch(ToolBatchPreflightInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            calls: &calls,
        })
        .await
        .unwrap();
    let prepared = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(prepared.map.revision, 2);
    assert_eq!(prepared.map.work_nodes.len(), 2);
    assert_eq!(prepared.map.action_reservations.len(), 1);

    let control = registry.tool_contributors()[0]
        .tools(&session_store, &thread_store)
        .remove(0);
    let payload = calls[0].payload.clone();
    let receipt = control
        .handle(ToolCall {
            turn_id: "turn-1".into(),
            call_id: "control".into(),
            tool_name: ToolName::plain("taskspace_control"),
            model: "test".into(),
            codex_turn_metadata: None,
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: Default::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            environments: vec![],
            payload: payload.clone(),
        })
        .await
        .unwrap();
    assert_eq!(receipt.code_mode_result(&payload)["revisionAfter"], 2);

    registry.tool_lifecycle_contributors()[0]
        .on_tool_finish(ToolFinishInput {
            session_store: &session_store,
            thread_store: &thread_store,
            turn_store: &turn_store,
            turn_id: "turn-1",
            call_id: "sibling",
            tool_name: &ToolName::plain("read_file"),
            source: ToolCallSource::Direct,
            outcome: ToolCallOutcome::Completed { success: true },
        })
        .await;
    let released = store.load_for_thread(thread_id).await.unwrap().unwrap().0;
    assert_eq!(released.map.revision, 3);
    assert!(released.map.action_reservations.is_empty());
    assert_eq!(released.map.result_refs.len(), 1);
    assert!(!released.map.result_refs.values().next().unwrap().is_error);
}

fn batch_call(call_id: &str, name: &str, arguments: serde_json::Value) -> ToolBatchCall {
    ToolBatchCall {
        call_id: call_id.into(),
        tool_name: ToolName::plain(name),
        payload: ToolPayload::Function {
            arguments: arguments.to_string(),
        },
    }
}
