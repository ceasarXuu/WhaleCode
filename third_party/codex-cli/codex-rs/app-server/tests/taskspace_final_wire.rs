use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use codex_extension_api::ExtensionRegistryBuilder;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::ThreadId;
use codex_taskspace_extension::TaskSpaceMapBinding;
use codex_taskspace_extension::TaskSpaceMapCommit;
use codex_taskspace_extension::TaskSpaceMapRecord;
use codex_taskspace_extension::TaskSpaceMapRelation;
use codex_taskspace_extension::TaskSpaceMapWriteOutcome;
use codex_taskspace_extension::TaskSpaceStore;
use codex_taskspace_extension::TaskSpaceStoreFuture;
use codex_taskspace_extension::install;
use codex_taskspace_extension::model::MapEdge;
use codex_taskspace_extension::model::map_node;
use codex_taskspace_extension::model::new_map;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;

#[derive(Default)]
struct ToggleTaskSpaceStore {
    active: AtomicBool,
}

impl ToggleTaskSpaceStore {
    fn activate(&self) {
        self.active.store(true, Ordering::Release);
    }

    fn record(thread_id: ThreadId) -> TaskSpaceMapRecord {
        TaskSpaceMapRecord {
            map: new_map(
                "wire-map".into(),
                map_node(
                    "root",
                    "lock the request contract",
                    vec!["user-turn".into()],
                ),
                vec![map_node("work", "capture TaskSpace wire", Vec::new())],
                map_node("finish", "verify cache prefix", Vec::new()),
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
            ),
            owner_thread_id: thread_id,
            canonical_sha256: "mock-canonical-sha256".into(),
            store_revision: 1,
        }
    }
}

impl TaskSpaceStore for ToggleTaskSpaceStore {
    fn load_for_thread(
        &self,
        thread_id: ThreadId,
    ) -> TaskSpaceStoreFuture<'_, Option<(TaskSpaceMapRecord, TaskSpaceMapBinding)>> {
        Box::pin(async move {
            if !self.active.load(Ordering::Acquire) {
                return Ok(None);
            }
            let record = Self::record(thread_id);
            Ok(Some((
                record.clone(),
                TaskSpaceMapBinding {
                    thread_id,
                    map_id: record.map.map_id,
                    relation: TaskSpaceMapRelation::Owner,
                    parent_thread_id: None,
                },
            )))
        })
    }

    fn bind(&self, _binding: TaskSpaceMapBinding) -> TaskSpaceStoreFuture<'_, ()> {
        Box::pin(async { anyhow::bail!("final-wire fixture is read-only") })
    }

    fn compare_and_swap(
        &self,
        _commit: TaskSpaceMapCommit,
    ) -> TaskSpaceStoreFuture<'_, TaskSpaceMapWriteOutcome> {
        Box::pin(async { anyhow::bail!("final-wire fixture is read-only") })
    }
}

fn deepseek_provider(server: &wiremock::MockServer) -> ModelProviderInfo {
    let mut provider = ModelProviderInfo::create_deepseek_provider();
    provider.base_url = Some(format!("{}/v1", server.uri()));
    provider.env_key = None;
    provider
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deepseek_taskspace_final_wire_preserves_standard_cache_prefix() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("standard-message", "standard done"),
                ev_completed("standard-response"),
            ]),
            sse(vec![
                ev_assistant_message("taskspace-message", "taskspace done"),
                ev_completed("taskspace-response"),
            ]),
        ],
    )
    .await;
    let store = Arc::new(ToggleTaskSpaceStore::default());
    let mut extensions = ExtensionRegistryBuilder::new();
    install(&mut extensions, store.clone());
    let provider = deepseek_provider(&server);
    let test = test_codex()
        .with_extensions(Arc::new(extensions.build()))
        .with_config(move |config| {
            config.model = Some("deepseek-v4-flash".into());
            config.model_provider = provider;
        })
        .build(&server)
        .await
        .expect("create conversation");

    test.submit_text_turn("standard request").await.unwrap();
    store.activate();
    test.submit_text_turn("taskspace request").await.unwrap();

    let requests = request_log.requests();
    assert_eq!(requests.len(), 2);
    let standard = requests[0].body_json();
    let taskspace = requests[1].body_json();

    assert_eq!(standard["model"], "deepseek-v4-flash");
    assert_eq!(taskspace["model"], "deepseek-v4-flash");
    assert_eq!(
        standard["reasoning"],
        serde_json::json!({"effort": "standard"})
    );
    assert_eq!(taskspace["reasoning"], standard["reasoning"]);
    assert_eq!(taskspace["instructions"], standard["instructions"]);
    assert_eq!(taskspace["prompt_cache_key"], standard["prompt_cache_key"]);

    let standard_tools = standard["tools"].as_array().expect("standard tools");
    let taskspace_tools = taskspace["tools"].as_array().expect("TaskSpace tools");
    assert!(
        !standard_tools
            .iter()
            .any(|tool| tool["name"] == "taskspace_control")
    );
    assert!(
        taskspace_tools
            .iter()
            .any(|tool| tool["name"] == "taskspace_control")
    );
    let common_taskspace_tools = taskspace_tools
        .iter()
        .filter(|tool| tool["name"] != "taskspace_control")
        .collect::<Vec<_>>();
    assert_eq!(
        common_taskspace_tools,
        standard_tools.iter().collect::<Vec<_>>(),
        "TaskSpace must not rewrite or reorder common tool declarations"
    );

    let standard_input = standard["input"].as_array().expect("standard input");
    let taskspace_input = taskspace["input"].as_array().expect("TaskSpace input");
    assert_eq!(
        &taskspace_input[..standard_input.len()],
        standard_input,
        "activating TaskSpace must preserve the existing conversation prefix"
    );
    let taskspace_wire = serde_json::to_string(&taskspace).unwrap();
    assert!(
        !serde_json::to_string(&standard)
            .unwrap()
            .contains("<taskspace_map>")
    );
    assert!(taskspace_wire.contains("<taskspace_map>"));
    assert!(taskspace_wire.contains("taskspace-canonical-map-v2"));
    assert!(taskspace_wire.contains("wire-map"));
}
