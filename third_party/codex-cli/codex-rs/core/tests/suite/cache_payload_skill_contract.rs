use super::cache_payload_contract::completed_response_stream;
use super::cache_payload_contract::stabilize_fixture_inputs;
use super::cache_payload_contract::submit_turn;
use super::cache_payload_contract::value_contains_text;
use codex_model_provider_info::WireApi;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use std::path::PathBuf;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

const SKILL_NAME: &str = "skill-creator";
async fn submit_first_turn(
    test: &core_test_support::test_codex::TestCodex,
    skill_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let mut items = vec![UserInput::Text {
        text: "skill contract turn one".to_string(),
        text_elements: Vec::new(),
    }];
    if let Some(path) = skill_path {
        items.push(UserInput::Skill {
            name: SKILL_NAME.to_string(),
            path,
        });
    }
    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items,
            final_output_json_schema: None,
            cwd: test.config.cwd.to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            permission_profile: None,
            model: test.session_configured.model.clone(),
            effort: None,
            summary: None,
            service_tier: None,
            collaboration_mode: None,
            personality: None,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}

async fn capture_skill_request_pair(selected: bool) -> anyhow::Result<Value> {
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    completed_response_stream("resp-skill-contract"),
                    "text/event-stream",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;
    let test = test_codex()
        .with_config(|config| {
            config.model_provider.wire_api = WireApi::Responses;
            config.model = Some("deepseek-v4-flash".to_string());
            config.cwd =
                AbsolutePathBuf::try_from(PathBuf::from("/tmp")).expect("fixed Skill contract cwd");
        })
        .build(&server)
        .await?;
    let skill_path = test
        .codex_home_path()
        .join("skills")
        .join(".system")
        .join(SKILL_NAME)
        .join("SKILL.md");
    assert!(skill_path.is_file(), "bundled Skill fixture must exist");
    submit_first_turn(&test, selected.then_some(skill_path)).await?;
    submit_turn(&test, "skill contract turn two").await?;

    let requests = server
        .received_requests()
        .await
        .expect("Skill final-wire requests");
    assert_eq!(requests.len(), 2);
    let first = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    let second = FinalWireEvidence::from_raw_body(&requests[1].body)?;
    let mut snapshot = serde_json::json!({
        "provider_identity": {
            "provider_id": "deepseek",
            "wire_api": "responses",
            "endpoint_path": "/v1/responses"
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
    Ok(snapshot)
}

fn is_selected_skill_message(value: &Value) -> bool {
    value_contains_text(value, "<skill>")
        && value_contains_text(value, "<name>skill-creator</name>")
}

fn remove_selected_skill_messages(value: &mut Value) {
    match value {
        Value::Array(values) => {
            values.retain(|value| !is_selected_skill_message(value));
            values.iter_mut().for_each(remove_selected_skill_messages);
        }
        Value::Object(values) => values.values_mut().for_each(remove_selected_skill_messages),
        _ => {}
    }
}

fn selected_skill_count(request: &Value) -> usize {
    request["input"]
        .as_array()
        .expect("request input")
        .iter()
        .filter(|item| value_contains_text(item, "<name>skill-creator</name>"))
        .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_bundled_skill_is_the_only_wire_difference() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let unselected = capture_skill_request_pair(false).await?;
    let selected = capture_skill_request_pair(true).await?;

    assert_eq!(selected_skill_count(&selected["request_1"]), 1);
    assert_eq!(selected_skill_count(&selected["request_2"]), 1);
    assert_eq!(selected_skill_count(&unselected["request_1"]), 0);
    assert_eq!(selected_skill_count(&unselected["request_2"]), 0);
    let mut normalized_selected = selected.clone();
    remove_selected_skill_messages(&mut normalized_selected);
    assert_eq!(normalized_selected, unselected);

    insta::assert_snapshot!(
        "selected_bundled_skill_two_request_final_wire",
        serde_json::to_string_pretty(&selected)?
    );
    Ok(())
}
