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
    include_str!("../../src/context/prompts/taskspace_core_protocol_v3.md");

fn initialize_arguments() -> String {
    json!({
        "action": "initialize_and_execute",
        "root": {"node_id": "root", "goal": "Complete the test task"},
        "work_nodes": [{"node_id": "work", "goal": "Inspect the workspace"}],
        "finish": {"node_id": "finish", "goal": "Verify and summarize"},
        "edges": [
            {"from": "root", "to": "work"},
            {"from": "work", "to": "finish"}
        ],
        "actions": [{"node_id": "work", "tool": "shell_command"}]
    })
    .to_string()
}

fn finish_arguments() -> String {
    json!({
        "action": "finish_map",
        "expected_revision": 2,
        "finish_node_id": "finish",
        "complete_work_node_ids": ["work"],
        "exact_summary": FINAL_SUMMARY
    })
    .to_string()
}

fn two_action_initialize_arguments() -> String {
    json!({
        "action": "initialize_and_execute",
        "root": {"node_id": "root", "goal": "Complete the test task"},
        "work_nodes": [
            {"node_id": "inspect", "goal": "Inspect the workspace"},
            {"node_id": "verify", "goal": "Verify the workspace"}
        ],
        "finish": {"node_id": "finish", "goal": "Finish"},
        "edges": [
            {"from": "root", "to": "inspect"},
            {"from": "root", "to": "verify"},
            {"from": "inspect", "to": "finish"},
            {"from": "verify", "to": "finish"}
        ],
        "actions": [
            {"node_id": "inspect", "tool": "exec_command"},
            {"node_id": "verify", "tool": "exec_command"}
        ]
    })
    .to_string()
}

fn execute_arguments() -> String {
    json!({
        "action": "execute",
        "expected_revision": 2,
        "mutations": [],
        "actions": [{"node_id": "work", "tool": "exec_command"}]
    })
    .to_string()
}

fn reopen_arguments() -> String {
    json!({
        "action": "reopen_map",
        "expected_revision": 3,
        "work_nodes": [{"node_id": "follow-up", "goal": "Address user feedback"}],
        "edges": [
            {"from": "root", "to": "follow-up"},
            {"from": "follow-up", "to": "finish"}
        ],
        "actions": [{"node_id": "follow-up", "tool": "exec_command"}]
    })
    .to_string()
}

fn plain_response(response_id: &str) -> String {
    let message_id = format!("{response_id}-message");
    let text = "The invalid tool response was rejected.";
    sse(vec![
        ev_response_created(response_id),
        ev_message_item_added(&message_id, "The invalid tool "),
        ev_output_text_delta("response was rejected."),
        ev_assistant_message(&message_id, text),
        ev_completed(response_id),
    ])
}

async fn enable_taskspace(test: &TestCodex) -> anyhow::Result<()> {
    test.codex
        .submit(Op::SetMapRuntimeMode {
            mode: MapRuntimeMode::Experiment,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(_)))
    })
    .await;
    Ok(())
}

async fn submit_and_collect(test: &TestCodex) -> anyhow::Result<Vec<EventMsg>> {
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
        .await?;

    let mut events = Vec::new();
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        let complete = matches!(event, EventMsg::TurnComplete(_));
        events.push(event);
        if complete {
            return Ok(events);
        }
    }
}

async fn submit_and_wait_for_store(test: &TestCodex) -> anyhow::Result<()> {
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "exercise invalid TaskSpace identity handling".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    Ok(())
}

fn common_responses(test: &TestCodex) -> Vec<String> {
    vec![sse(vec![
        ev_response_created("init-response"),
        ev_function_call("init-control", "taskspace_control", &initialize_arguments()),
        ev_function_call(
            "init-action",
            "exec_command",
            &json!({
                "cmd": "pwd",
                "workdir": test.cwd_path().display().to_string(),
            })
            .to_string(),
        ),
        ev_completed("init-response"),
    ])]
}

fn assert_taskspace_request_shapes(responses: &ResponseMock, expected_count: usize) {
    let requests = responses.requests();
    assert_eq!(requests.len(), expected_count);
    for request in requests {
        let body = request.body_json();
        assert_eq!(
            body["instructions"].as_str(),
            Some(codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_TASKSPACE)
        );
        let developer_texts = request.message_input_texts("developer");
        assert_eq!(
            developer_texts.first().map(String::as_str),
            Some(TASKSPACE_CORE_PROTOCOL)
        );
        assert_eq!(
            developer_texts
                .iter()
                .filter(|text| text.contains("<taskspace_core_protocol"))
                .count(),
            1
        );
        assert_eq!(body["tool_choice"], json!("auto"));
        let tool_names = body["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(tool_names.first().copied(), Some("taskspace_control"));
        assert!(!tool_names.contains(&"update_plan"));
    }
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
    enable_taskspace(&test).await?;

    let mut bodies = common_responses(&test);
    bodies.push(sse(vec![
        ev_response_created("finish-response"),
        ev_function_call("finish-call", "taskspace_control", &finish_arguments()),
        ev_completed("finish-response"),
    ]));
    let responses = mount_sse_sequence(&server, bodies).await;
    let events = submit_and_collect(&test).await?;
    let errors = events
        .iter()
        .filter_map(|event| match event {
            EventMsg::Error(error) => Some(error.message.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");

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
        EventMsg::MapRuntime(MapRuntimeEvent::StoreCommitted(committed))
            if committed.operation == "finish_map" && committed.map_revision == 3
    )));

    let state = codex_state::StateRuntime::init(
        test.codex_home_path().to_path_buf(),
        "test-provider".to_string(),
    )
    .await?;
    let (stored, binding) = state
        .load_taskspace_map_for_thread(test.session_configured.session_id)
        .await?
        .expect("TaskSpace thread remains bound to its canonical map");
    let map = stored.canonical_map.expect("stored canonical map");
    assert!(stored.terminal);
    assert_eq!(stored.map_revision, 3);
    assert_eq!(binding.map_id, stored.map_id);
    assert_eq!(
        map.terminal_record.expect("current terminal").summary_ref,
        FINAL_SUMMARY
    );
    assert!(map.completion_records.contains_key("work"));
    assert!(!map.completion_records.contains_key("root"));
    assert!(!map.completion_records.contains_key("finish"));
    assert_taskspace_request_shapes(&responses, 2);
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
    enable_taskspace(&test).await?;

    let mut bodies = common_responses(&test);
    bodies.push(sse(vec![
        ev_response_created("plain-response"),
        ev_message_item_added("plain-message", "Provider tried "),
        ev_output_text_delta("to finish without finish_map."),
        ev_assistant_message("plain-message", PLAIN_PROVIDER_TEXT),
        ev_completed("plain-response"),
    ]));
    let responses = mount_sse_sequence(&server, bodies).await;
    let events = submit_and_collect(&test).await?;

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
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::Error(error)
            if error.message.contains("taskspace_terminal_protocol_violation")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        EventMsg::TurnComplete(completed) if completed.last_agent_message.is_none()
    )));
    assert_taskspace_request_shapes(&responses, 2);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_initialize_call_ids_leave_store_empty_and_dispatch_nothing() -> anyhow::Result<()>
{
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            config.taskspace_projection_policy = Some(TaskSpaceProjectionPolicy::MapAppend);
        })
        .build(&server)
        .await?;
    enable_taskspace(&test).await?;
    let marker = test.cwd_path().join("duplicate-init-dispatched");
    let invalid = sse(vec![
        ev_response_created("invalid-init"),
        ev_function_call(
            "init-control",
            "taskspace_control",
            &two_action_initialize_arguments(),
        ),
        ev_function_call(
            "duplicate",
            "exec_command",
            &json!({"cmd": format!("touch {}", marker.display())}).to_string(),
        ),
        ev_function_call(
            "duplicate",
            "exec_command",
            &json!({"cmd": format!("touch {}", marker.display())}).to_string(),
        ),
        ev_completed("invalid-init"),
    ]);
    mount_sse_sequence(&server, vec![invalid, plain_response("after-invalid-init")]).await;

    submit_and_wait_for_store(&test).await?;

    assert!(!marker.exists(), "invalid sibling calls must not dispatch");
    let state = codex_state::StateRuntime::init(
        test.codex_home_path().to_path_buf(),
        "test-provider".to_string(),
    )
    .await?;
    let (stored, _) = state
        .load_taskspace_map_for_thread(test.session_configured.session_id)
        .await?
        .expect("mechanical TaskSpace identity");
    assert_eq!(stored.map_revision, 0);
    assert!(stored.canonical_map.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_execute_call_id_preserves_active_store_revision() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            config.taskspace_projection_policy = Some(TaskSpaceProjectionPolicy::MapAppend);
        })
        .build(&server)
        .await?;
    enable_taskspace(&test).await?;
    let marker = test.cwd_path().join("empty-execute-dispatched");
    let invalid = sse(vec![
        ev_response_created("invalid-execute"),
        ev_function_call("execute-control", "taskspace_control", &execute_arguments()),
        ev_function_call(
            " ",
            "exec_command",
            &json!({"cmd": format!("touch {}", marker.display())}).to_string(),
        ),
        ev_completed("invalid-execute"),
    ]);
    let mut responses = common_responses(&test);
    responses.extend([invalid, plain_response("after-invalid-execute")]);
    mount_sse_sequence(&server, responses).await;

    submit_and_wait_for_store(&test).await?;

    assert!(!marker.exists(), "invalid sibling call must not dispatch");
    let state = codex_state::StateRuntime::init(
        test.codex_home_path().to_path_buf(),
        "test-provider".to_string(),
    )
    .await?;
    let (stored, _) = state
        .load_taskspace_map_for_thread(test.session_configured.session_id)
        .await?
        .expect("active canonical map");
    let map = stored.canonical_map.expect("initialized canonical map");
    assert_eq!(stored.map_revision, 2);
    assert_eq!(map.revision, 2);
    assert!(map.action_reservations.is_empty());
    assert_eq!(map.result_refs.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_reopen_call_id_preserves_closed_store_and_terminal() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            config.taskspace_projection_policy = Some(TaskSpaceProjectionPolicy::MapAppend);
        })
        .build(&server)
        .await?;
    enable_taskspace(&test).await?;
    let marker = test.cwd_path().join("duplicate-reopen-dispatched");
    let invalid = sse(vec![
        ev_response_created("invalid-reopen"),
        ev_function_call("duplicate", "taskspace_control", &reopen_arguments()),
        ev_function_call(
            "duplicate",
            "exec_command",
            &json!({"cmd": format!("touch {}", marker.display())}).to_string(),
        ),
        ev_completed("invalid-reopen"),
    ]);
    let mut responses = common_responses(&test);
    responses.push(sse(vec![
        ev_response_created("finish-response"),
        ev_function_call("finish-call", "taskspace_control", &finish_arguments()),
        ev_completed("finish-response"),
    ]));
    responses.extend([invalid, plain_response("after-invalid-reopen")]);
    mount_sse_sequence(&server, responses).await;

    submit_and_collect(&test).await?;
    submit_and_wait_for_store(&test).await?;

    assert!(!marker.exists(), "invalid reopen sibling must not dispatch");
    let state = codex_state::StateRuntime::init(
        test.codex_home_path().to_path_buf(),
        "test-provider".to_string(),
    )
    .await?;
    let (stored, _) = state
        .load_taskspace_map_for_thread(test.session_configured.session_id)
        .await?
        .expect("closed canonical map");
    let map = stored.canonical_map.expect("closed canonical facts");
    assert_eq!(stored.map_revision, 3);
    assert!(stored.terminal);
    assert!(map.terminal_record.is_some());
    assert!(map.terminal_history.is_empty());
    assert!(map.action_reservations.is_empty());
    Ok(())
}
