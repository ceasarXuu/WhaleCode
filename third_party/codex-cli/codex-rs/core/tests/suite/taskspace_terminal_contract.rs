#![cfg(not(target_os = "windows"))]

use anyhow::Ok;
use codex_protocol::items::AgentMessageContent;
use codex_protocol::items::TurnItem;
use codex_protocol::models::MessagePhase;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::user_input::UserInput;
use codex_state::CommitTaskSpaceMapRequest;
use codex_state::TaskSpaceMapWriteOutcome;
use core_test_support::responses::ResponseMock;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_message_item_added;
use core_test_support::responses::ev_output_text_delta;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;

const FINAL_SUMMARY: &str = "Exact Agent terminal summary.";
const PLAIN_PROVIDER_TEXT: &str = "Provider tried to finish without finish_map.";
const TASKSPACE_CORE_PROTOCOL: &str =
    include_str!("../../src/context/prompts/taskspace_core_protocol_v2.md");

fn initialize_arguments() -> String {
    json!({
        "action": "initialize_map",
        "root": {"id": "root", "goal": "Complete the test task"},
        "initial_work": {"id": "work", "goal": "Inspect the workspace"},
        "additional_work": [
            {"id": "verify", "goal": "Verify the result"}
        ],
        "finish_id": "finish",
        "edges": [
            {"from": "root", "to": "work"},
            {"from": "work", "to": "verify"},
            {"from": "verify", "to": "finish"}
        ]
    })
    .to_string()
}

fn transition_arguments() -> String {
    json!({
        "action": "complete_then_continue",
        "expected_revision": 2,
        "current_node_id": "work",
        "next_node_id": "verify"
    })
    .to_string()
}

fn finish_arguments() -> String {
    json!({
        "action": "finish_map",
        "expected_revision": 3,
        "terminal_node_id": "verify",
        "final_summary": FINAL_SUMMARY
    })
    .to_string()
}

async fn enable_taskspace(test: &TestCodex) {
    test.codex
        .submit(Op::SetMapRuntimeMode {
            mode: MapRuntimeMode::Experiment,
        })
        .await
        .expect("enable TaskSpace");
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(_)))
    })
    .await;
}

async fn submit_and_collect(test: &TestCodex) -> Vec<EventMsg> {
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "exercise the terminal contract".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submit test turn");

    let mut events = Vec::new();
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        let complete = matches!(event, EventMsg::TurnComplete(_));
        events.push(event);
        if complete {
            return events;
        }
    }
}

fn bound_exec_arguments(test: &TestCodex, binding: &str) -> String {
    json!({
        "cmd": "pwd",
        "workdir": test.cwd_path().display().to_string(),
        "taskspace_binding": {"action": binding},
    })
    .to_string()
}

fn initializing_exec_arguments(test: &TestCodex) -> String {
    json!({
        "cmd": "pwd",
        "workdir": test.cwd_path().display().to_string(),
        "taskspace_binding": serde_json::from_str::<serde_json::Value>(&initialize_arguments())
            .expect("initialization arguments"),
    })
    .to_string()
}

fn common_responses(test: &TestCodex) -> Vec<String> {
    vec![
        sse(vec![
            ev_response_created("init-response"),
            ev_function_call(
                "init-action",
                "exec_command",
                &initializing_exec_arguments(test),
            ),
            ev_completed("init-response"),
        ]),
        sse(vec![
            ev_response_created("complete-response"),
            ev_function_call(
                "complete-control",
                "taskspace_control",
                &transition_arguments(),
            ),
            ev_function_call(
                "complete-action",
                "exec_command",
                &bound_exec_arguments(test, "after_boundary"),
            ),
            ev_completed("complete-response"),
        ]),
    ]
}

fn assert_taskspace_request_shapes(responses: &ResponseMock) {
    let requests = responses.requests();
    assert_eq!(
        requests.len(),
        3,
        "terminal handling must not add a request"
    );
    for request in requests {
        let body = request.body_json();
        assert_eq!(
            body["instructions"].as_str(),
            Some(codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            "TaskSpace request must carry the complete TaskSpace base instructions"
        );
        let developer_texts = request.message_input_texts("developer");
        assert_eq!(
            developer_texts.first().map(String::as_str),
            Some(TASKSPACE_CORE_PROTOCOL),
            "TaskSpace core protocol must be the first stable developer section"
        );
        assert_eq!(
            developer_texts
                .iter()
                .filter(|text| text.contains("<taskspace_core_protocol"))
                .count(),
            1,
            "TaskSpace core protocol must appear exactly once"
        );
        let tool_choice = body["tool_choice"].clone();
        let tool_names = body["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            tool_choice,
            json!("auto"),
            "TaskSpace request changed tool choice: tools={tool_names:?}"
        );
        assert_eq!(
            body["tools"].as_array().map(Vec::len),
            Some(12),
            "TaskSpace must preserve the immutable preflight-capable tool surface: {tool_names:?}"
        );
        assert_eq!(
            tool_names.first().copied(),
            Some("taskspace_control"),
            "the immutable TaskSpace tool surface must keep control first"
        );
        assert!(
            !tool_names.contains(&"update_plan"),
            "TaskSpace request must hide the linear plan tool"
        );
        for unsupported in ["local_shell", "web_search", "image_generation"] {
            assert!(
                !tool_names.contains(&unsupported),
                "TaskSpace must hide provider-native tools that cannot enter client preflight: \
                 {unsupported}"
            );
        }
    }
    let initialization_output = responses
        .function_call_output_text("init-action")
        .expect("missing initialization carrier output");
    assert!(
        initialization_output.contains("TaskSpaceInitializationCarrierResultV1"),
        "{initialization_output}"
    );
    assert!(
        initialization_output.contains("TaskSpaceControlResultV2"),
        "{initialization_output}"
    );
    assert!(
        initialization_output.contains("\"state_commit\":true"),
        "{initialization_output}"
    );

    let complete_output = responses
        .function_call_output_text("complete-control")
        .expect("missing complete control output");
    assert!(complete_output.contains("TaskSpaceControlResultV2"));
    assert!(complete_output.contains("\"state_commit\":true"));
    let complete_action_output = responses
        .function_call_output_text("complete-action")
        .expect("missing complete action output");
    assert!(
        !complete_action_output.contains("TaskSpaceInitializationCarrierResultV1"),
        "{complete_action_output}"
    );
}

fn error_messages(events: &[EventMsg]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            EventMsg::Error(error) => Some(error.message.as_str()),
            _ => None,
        })
        .collect()
}

fn control_output_diagnostics(responses: &ResponseMock) -> Vec<(&'static str, String)> {
    ["init-action", "complete-control", "finish-call"]
        .into_iter()
        .filter_map(|call_id| {
            responses
                .function_call_output_text(call_id)
                .map(|output| (call_id, output))
        })
        .collect()
}

fn agent_text(item: &TurnItem) -> Option<(String, Option<MessagePhase>)> {
    let TurnItem::AgentMessage(message) = item else {
        return None;
    };
    let text = message
        .content
        .iter()
        .map(|content| match content {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect::<String>();
    Some((text, message.phase.clone()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_finish_control_is_the_only_taskspace_final() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            config.taskspace_projection_policy = Some(TaskSpaceProjectionPolicy::MapAppend);
        })
        .build(&server)
        .await?;
    enable_taskspace(&test).await;

    let mut bodies = common_responses(&test);
    bodies.push(sse(vec![
        ev_response_created("finish-response"),
        ev_function_call("finish-call", "taskspace_control", &finish_arguments()),
        ev_completed("finish-response"),
    ]));
    let responses = mount_sse_sequence(&server, bodies).await;
    let events = submit_and_collect(&test).await;
    let errors = error_messages(&events);
    assert!(
        errors.is_empty(),
        "unexpected errors: {errors:?}; control outputs: {:?}",
        control_output_diagnostics(&responses)
    );

    let completed_messages = events
        .iter()
        .filter_map(|event| match event {
            EventMsg::ItemCompleted(ItemCompletedEvent { item, .. }) => agent_text(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        completed_messages,
        vec![(FINAL_SUMMARY.to_string(), Some(MessagePhase::FinalAnswer))]
    );
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::TurnComplete(completed)
            if completed.last_agent_message.as_deref() == Some(FINAL_SUMMARY)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::MapRuntime(MapRuntimeEvent::GraphRevisionCommitted(committed))
            if committed.operation == "finish_map" && committed.revision == 4
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::MapRuntime(MapRuntimeEvent::StoreCommitted(committed))
            if committed.operation == "finish_map"
                && committed.store_revision >= 4
                && committed.graph_revision == 4
    )));

    let state = codex_state::StateRuntime::init(
        test.codex_home_path().to_path_buf(),
        "test-provider".to_string(),
    )
    .await?;
    let (stored, binding) = state
        .load_taskspace_map_for_thread(test.session_configured.session_id)
        .await?
        .expect("TaskSpace thread must remain bound to its canonical Store map");
    let stored_map = stored.snapshot.map.as_ref().expect("stored rooted map");
    assert!(stored.complete);
    assert!(stored_map.complete);
    assert_eq!(stored.graph_revision, 4);
    assert_eq!(stored_map.revision, 4);
    assert_eq!(binding.map_id, stored.map_id);

    let mut externally_committed = stored.snapshot.clone();
    externally_committed.routing_required = !stored.snapshot.routing_required;
    let external_commit = state
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: stored.map_id.clone(),
            expected_store_revision: stored.store_revision,
            snapshot: externally_committed,
            commit_id: "external-app-server-read-test".to_string(),
            operation: "external_app_server_read_test".to_string(),
            actor_thread_id: test.session_configured.session_id,
            binding: None,
        })
        .await?;
    assert!(matches!(
        external_commit,
        TaskSpaceMapWriteOutcome::Applied(_)
    ));
    let refreshed = test
        .codex
        .action_map_snapshot()
        .await
        .map_err(anyhow::Error::msg)?;
    assert_ne!(
        refreshed.routing_required, stored.snapshot.routing_required,
        "public snapshot reads must refresh a stale Session cache from the canonical Store"
    );

    let rollout_path = test.codex.rollout_path().expect("rollout path");
    let rollout = tokio::fs::read_to_string(rollout_path).await?;
    assert!(!rollout.contains("\"snapshot_updated\""));
    assert!(!rollout.contains("\"snapshot_delta\""));
    assert_taskspace_request_shapes(&responses);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plain_provider_final_is_nonterminal_and_does_not_retry() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            config.taskspace_projection_policy = Some(TaskSpaceProjectionPolicy::MapAppend);
        })
        .build(&server)
        .await?;
    enable_taskspace(&test).await;

    let mut bodies = common_responses(&test);
    bodies.push(sse(vec![
        ev_response_created("plain-response"),
        ev_message_item_added("plain-message", "Provider tried "),
        ev_output_text_delta("to finish without finish_map."),
        ev_assistant_message("plain-message", PLAIN_PROVIDER_TEXT),
        ev_completed("plain-response"),
    ]));
    let responses = mount_sse_sequence(&server, bodies).await;
    let events = submit_and_collect(&test).await;

    let completed_messages = events
        .iter()
        .filter_map(|event| match event {
            EventMsg::ItemCompleted(ItemCompletedEvent { item, .. }) => agent_text(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        completed_messages,
        vec![(
            PLAIN_PROVIDER_TEXT.to_string(),
            Some(MessagePhase::Commentary)
        )]
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        EventMsg::AgentMessageDelta(_) | EventMsg::AgentMessageContentDelta(_)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::Error(error)
            if error.message.contains("taskspace_terminal_protocol_violation")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::TurnComplete(completed) if completed.last_agent_message.is_none()
    )));
    assert_taskspace_request_shapes(&responses);
    Ok(())
}
