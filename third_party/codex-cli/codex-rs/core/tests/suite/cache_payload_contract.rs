use codex_model_provider_info::WireApi;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use wiremock::Mock;
use wiremock::Respond;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

const CHAT_COMPLETION_STREAM: &str = concat!(
    "data: {\"id\":\"chatcmpl-cache-contract\",\"choices\":[{\"index\":0,",
    "\"delta\":{\"content\":\"turn complete\"},\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-cache-contract\",\"choices\":[{\"index\":0,",
    "\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":1,",
    "\"total_tokens\":11,\"prompt_tokens_details\":{\"cached_tokens\":0}}}\n\n",
    "data: [DONE]\n\n"
);

struct ChatSequenceResponder {
    next: AtomicUsize,
    bodies: Vec<String>,
}

impl Respond for ChatSequenceResponder {
    fn respond(&self, _: &wiremock::Request) -> ResponseTemplate {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(
                self.bodies
                    .get(index)
                    .unwrap_or_else(|| panic!("unexpected TaskSpace request {index}"))
                    .clone(),
            )
    }
}

fn chat_tool_stream(response_id: &str, calls: Vec<Value>) -> String {
    let chunk = serde_json::json!({
        "id": response_id,
        "choices": [{
            "index": 0,
            "delta": {"tool_calls": calls},
            "finish_reason": "tool_calls"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 1,
            "total_tokens": 11,
            "prompt_tokens_details": {"cached_tokens": 0}
        }
    });
    format!("data: {chunk}\n\ndata: [DONE]\n\n")
}

fn chat_tool_call(index: usize, call_id: &str, name: &str, arguments: Value) -> Value {
    serde_json::json!({
        "index": index,
        "id": call_id,
        "type": "function",
        "function": {"name": name, "arguments": arguments.to_string()}
    })
}

fn initialize_arguments() -> Value {
    serde_json::json!({
        "action": "initialize_and_execute",
        "root": {"node_id": "root", "goal": "Complete the cache contract task"},
        "work_nodes": [{"node_id": "work", "goal": "Run the deterministic check"}],
        "finish": {"node_id": "finish", "goal": "Verify and summarize"},
        "edges": [
            {"from": "root", "to": "work"},
            {"from": "work", "to": "finish"}
        ],
        "actions": [{"node_id": "work", "tool": "exec_command"}]
    })
}

fn finish_arguments() -> Value {
    serde_json::json!({
        "action": "finish_map",
        "expected_revision": 2,
        "finish_node_id": "finish",
        "complete_work_node_ids": ["work"],
        "exact_summary": "Cache contract task complete."
    })
}

async fn submit_turn(
    test: &core_test_support::test_codex::TestCodex,
    text: &str,
) -> anyhow::Result<()> {
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}

fn replace_tag_value(text: &str, tag: &str, replacement: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let Some(start) = text.find(&open) else {
        return text.to_string();
    };
    let value_start = start + open.len();
    let Some(relative_end) = text[value_start..].find(&close) else {
        return text.to_string();
    };
    let value_end = value_start + relative_end;
    format!(
        "{}{}{}",
        &text[..value_start],
        replacement,
        &text[value_end..]
    )
}

fn stabilize_projection_canonical_sha256(text: &str) -> String {
    const PREFIX: &str = "- canonical_sha256: ";
    if !text.contains("TaskSpaceMapProjectionR7V1:") {
        return text.to_string();
    }

    text.split_inclusive('\n')
        .map(|line| {
            let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
            let Some(hash) = line_without_newline.strip_prefix(PREFIX) else {
                return line.to_string();
            };
            assert_eq!(hash.len(), 64, "canonical projection hash length");
            assert!(
                hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "canonical projection hash format"
            );
            format!(
                "{PREFIX}<PROJECTION_CANONICAL_SHA256>{}",
                if line.ends_with('\n') { "\n" } else { "" }
            )
        })
        .collect()
}

fn stabilize_fixture_inputs(value: &mut Value, path_prefixes: &[(&str, &str)]) {
    match value {
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| stabilize_fixture_inputs(value, path_prefixes)),
        Value::Object(values) => values
            .values_mut()
            .for_each(|value| stabilize_fixture_inputs(value, path_prefixes)),
        Value::String(text) => {
            if text.contains("<environment_context>") {
                *text = replace_tag_value(text, "current_date", "FIXED_CURRENT_DATE");
                *text = replace_tag_value(text, "timezone", "FIXED_TIMEZONE");
            }
            *text = stabilize_projection_canonical_sha256(text);
            for (prefix, replacement) in path_prefixes {
                if prefix.is_empty() {
                    continue;
                }
                *text = text.replace(prefix, replacement);
            }
        }
        _ => {}
    }
}

#[test]
fn fixture_stabilization_ignores_empty_prefixes() {
    let mut value = Value::String("unchanged".to_string());
    stabilize_fixture_inputs(&mut value, &[("", "<EMPTY>")]);
    assert_eq!(value, Value::String("unchanged".to_string()));
}

#[test]
fn fixture_stabilization_replaces_only_valid_projection_canonical_hash() {
    let hash = "a".repeat(64);
    let mut value = Value::String(format!(
        "TaskSpaceMapProjectionR7V1:\n- canonical_sha256: {hash}\n- goal: keep"
    ));
    stabilize_fixture_inputs(&mut value, &[]);
    assert_eq!(
        value,
        Value::String(
            "TaskSpaceMapProjectionR7V1:\n- canonical_sha256: <PROJECTION_CANONICAL_SHA256>\n- goal: keep"
                .to_string()
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standard_request_pair_preserves_the_complete_prefix() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(CHAT_COMPLETION_STREAM, "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(|config| {
            config.model_provider.wire_api = WireApi::ChatCompletions;
            config.model = Some("deepseek-v4-flash".to_string());
            config.cwd =
                AbsolutePathBuf::try_from(PathBuf::from("/tmp")).expect("fixed cache contract cwd");
        })
        .build(&server)
        .await?;
    submit_turn(&test, "cache contract turn one").await?;
    submit_turn(&test, "cache contract turn two").await?;

    let requests = server
        .received_requests()
        .await
        .expect("two final-wire requests");
    assert_eq!(requests.len(), 2);
    let first = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    let second = FinalWireEvidence::from_raw_body(&requests[1].body)?;
    let first_messages = first.structured_body["messages"]
        .as_array()
        .expect("first messages");
    let second_messages = second.structured_body["messages"]
        .as_array()
        .expect("second messages");
    assert_eq!(&second_messages[..first_messages.len()], first_messages);
    assert_eq!(
        first.structured_body["tools"],
        second.structured_body["tools"]
    );
    assert_eq!(
        first.structured_body["tool_choice"],
        second.structured_body["tool_choice"]
    );
    let mut inserted_message = first.clone();
    inserted_message.structured_body["messages"]
        .as_array_mut()
        .expect("mutable messages")
        .insert(
            1,
            serde_json::json!({"role": "developer", "content": "inserted mutation"}),
        );
    assert_ne!(inserted_message, first);

    let mut snapshot = serde_json::json!({
        "provider_identity": {
            "provider_id": "deepseek",
            "wire_api": "chat_completions",
            "endpoint_path": "/v1/chat/completions"
        },
        "request_1": first.structured_body,
        "request_2": second.structured_body,
    });
    let codex_home = test.codex_home_path().to_string_lossy();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    stabilize_fixture_inputs(
        &mut snapshot,
        &[
            (codex_home.as_ref(), "<CODEX_HOME>"),
            (&source_root, "<CODEX_SOURCE_ROOT>"),
        ],
    );
    insta::assert_snapshot!(
        "standard_two_request_final_wire",
        serde_json::to_string_pretty(&snapshot)?
    );
    Ok(())
}

async fn capture_taskspace_request_pair(
    policy: TaskSpaceProjectionPolicy,
) -> anyhow::Result<Value> {
    let server = start_mock_server().await;
    let first_response = chat_tool_stream(
        "chatcmpl-taskspace-init",
        vec![
            chat_tool_call(
                0,
                "taskspace-init",
                "taskspace_control",
                initialize_arguments(),
            ),
            chat_tool_call(
                1,
                "taskspace-exec",
                "exec_command",
                serde_json::json!({"cmd": "printf taskspace-contract", "workdir": "/tmp"}),
            ),
        ],
    );
    let second_response = chat_tool_stream(
        "chatcmpl-taskspace-finish",
        vec![chat_tool_call(
            0,
            "taskspace-finish",
            "taskspace_control",
            finish_arguments(),
        )],
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ChatSequenceResponder {
            next: AtomicUsize::new(0),
            bodies: vec![first_response, second_response],
        })
        .expect(2)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(move |config| {
            config.model_provider.wire_api = WireApi::ChatCompletions;
            config.model = Some("deepseek-v4-flash".to_string());
            config.taskspace_projection_policy = Some(policy);
            config.cwd = AbsolutePathBuf::try_from(PathBuf::from("/tmp"))
                .expect("fixed TaskSpace cache contract cwd");
        })
        .build(&server)
        .await?;
    test.codex
        .submit(Op::SetMapRuntimeMode {
            mode: MapRuntimeMode::Experiment,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(_)))
    })
    .await;
    submit_turn(&test, "run the TaskSpace cache contract").await?;

    let requests = server
        .received_requests()
        .await
        .expect("TaskSpace final-wire requests");
    assert_eq!(requests.len(), 2);
    let first = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    let second = FinalWireEvidence::from_raw_body(&requests[1].body)?;
    let snapshot_map = test
        .codex
        .action_map_snapshot()
        .await
        .map_err(anyhow::Error::msg)?
        .map
        .expect("TaskSpace map");
    let map_id = snapshot_map.id;
    let task_id = snapshot_map.task_id.unwrap_or_default();
    let owner_id = snapshot_map
        .owner_session_id
        .map(|owner| owner.to_string())
        .unwrap_or_default();

    let mut snapshot = serde_json::json!({
        "provider_identity": {
            "provider_id": "deepseek",
            "wire_api": "chat_completions",
            "endpoint_path": "/v1/chat/completions"
        },
        "request_1": first.structured_body,
        "request_2": second.structured_body,
    });
    let codex_home = test.codex_home_path().to_string_lossy();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    stabilize_fixture_inputs(
        &mut snapshot,
        &[
            (codex_home.as_ref(), "<CODEX_HOME>"),
            (&source_root, "<CODEX_SOURCE_ROOT>"),
            (&map_id, "<MAP_ID>"),
            (&task_id, "<TASK_ID>"),
            (&owner_id, "<OWNER_THREAD_ID>"),
        ],
    );
    Ok(snapshot)
}

fn projection_count(request: &Value) -> usize {
    request["messages"]
        .as_array()
        .expect("request messages")
        .iter()
        .filter(|message| {
            message["content"]
                .as_str()
                .is_some_and(|content| content.contains("TaskSpaceMapProjectionR7V1:"))
        })
        .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn taskspace_projection_policies_have_independent_request_pairs() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let always = capture_taskspace_request_pair(TaskSpaceProjectionPolicy::MapAlways).await?;
    let append = capture_taskspace_request_pair(TaskSpaceProjectionPolicy::MapAppend).await?;
    let request = capture_taskspace_request_pair(TaskSpaceProjectionPolicy::MapRequest).await?;

    for request_key in ["request_1", "request_2"] {
        assert_eq!(always[request_key]["tools"], append[request_key]["tools"]);
        assert_eq!(always[request_key]["tools"], request[request_key]["tools"]);
        assert_eq!(
            always[request_key]["tool_choice"],
            append[request_key]["tool_choice"]
        );
        assert_eq!(
            always[request_key]["tool_choice"],
            request[request_key]["tool_choice"]
        );
    }
    assert_eq!(projection_count(&always["request_1"]), 1);
    assert_eq!(projection_count(&always["request_2"]), 1);
    assert_eq!(projection_count(&append["request_1"]), 1);
    assert_eq!(projection_count(&append["request_2"]), 2);
    assert_eq!(projection_count(&request["request_1"]), 0);
    assert_eq!(projection_count(&request["request_2"]), 0);

    insta::assert_snapshot!(
        "taskspace_map_always_two_request_final_wire",
        serde_json::to_string_pretty(&always)?
    );
    insta::assert_snapshot!(
        "taskspace_map_append_two_request_final_wire",
        serde_json::to_string_pretty(&append)?
    );
    insta::assert_snapshot!(
        "taskspace_map_request_two_request_final_wire",
        serde_json::to_string_pretty(&request)?
    );
    Ok(())
}
