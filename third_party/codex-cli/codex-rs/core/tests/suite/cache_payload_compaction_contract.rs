use super::cache_payload_contract::configure_deepseek_responses;
use super::cache_payload_contract::provider_identity;
use super::cache_payload_contract::stabilize_fixture_inputs;
use super::cache_payload_contract::submit_turn;
use super::cache_payload_contract::value_contains_text;
use codex_core::compact::SUMMARIZATION_PROMPT;
use codex_core::compact::SUMMARY_PREFIX;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use std::path::PathBuf;

const FIRST_USER: &str = "compaction contract first turn";
const FIRST_ASSISTANT: &str = "PRE_COMPACTION_ASSISTANT_HISTORY";
const SUMMARY: &str = "COMPACTED_CONTRACT_SUMMARY";
const FOLLOW_UP_USER: &str = "compaction contract follow-up";

fn protected_request_controls(request: &Value) -> Value {
    serde_json::json!({
        "instructions": request["instructions"],
        "model": request["model"],
        "parallel_tool_calls": request["parallel_tool_calls"],
        "reasoning": request["reasoning"],
        "text": request["text"],
        "tool_choice": request["tool_choice"],
        "tools": request["tools"],
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_compaction_has_an_independent_final_wire_contract() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("msg-before-compact", FIRST_ASSISTANT),
                ev_completed("resp-before-compact"),
            ]),
            sse(vec![
                ev_assistant_message("msg-compact-summary", SUMMARY),
                ev_completed("resp-compact-summary"),
            ]),
            sse(vec![ev_completed("resp-after-compact")]),
        ],
    )
    .await;

    let test = test_codex()
        .with_config(|config| {
            configure_deepseek_responses(config);
            config.cwd = AbsolutePathBuf::try_from(PathBuf::from("/tmp"))
                .expect("fixed compaction cache contract cwd");
            config.compact_prompt = Some(SUMMARIZATION_PROMPT.to_string());
        })
        .build(&server)
        .await?;

    submit_turn(&test, FIRST_USER).await?;
    test.codex.submit(Op::Compact).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    submit_turn(&test, FOLLOW_UP_USER).await?;

    let requests = request_log.requests();
    assert_eq!(requests.len(), 3);
    let mut before = FinalWireEvidence::from_raw_body(&requests[0].body_bytes())?.structured_body;
    let mut compact = FinalWireEvidence::from_raw_body(&requests[1].body_bytes())?.structured_body;
    let mut after = FinalWireEvidence::from_raw_body(&requests[2].body_bytes())?.structured_body;

    assert!(value_contains_text(&before["input"], FIRST_USER));
    assert!(!value_contains_text(&before["input"], FIRST_ASSISTANT));
    assert!(value_contains_text(&compact["input"], FIRST_USER));
    assert!(value_contains_text(&compact["input"], FIRST_ASSISTANT));
    assert!(value_contains_text(&compact["input"], SUMMARIZATION_PROMPT));
    assert!(value_contains_text(&after["input"], FIRST_USER));
    assert!(value_contains_text(
        &after["input"],
        &format!("{SUMMARY_PREFIX}\n{SUMMARY}")
    ));
    assert!(value_contains_text(&after["input"], FOLLOW_UP_USER));
    assert!(!value_contains_text(&after["input"], FIRST_ASSISTANT));
    assert!(!value_contains_text(&after["input"], SUMMARIZATION_PROMPT));

    let controls = protected_request_controls(&before);
    assert_eq!(protected_request_controls(&after), controls);
    for key in ["instructions", "model", "reasoning", "text", "tool_choice"] {
        assert_eq!(compact[key], before[key], "compact request changed {key}");
    }
    assert_eq!(compact["parallel_tool_calls"], false);
    assert!(
        compact["tools"].as_array().is_some_and(Vec::is_empty),
        "summarization request must not expose execution tools"
    );

    let codex_home = test.codex_home_path().to_string_lossy();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    for request in [&mut before, &mut compact, &mut after] {
        stabilize_fixture_inputs(
            request,
            &[
                (codex_home.as_ref(), "<CODEX_HOME>"),
                (&source_root, "<CODEX_SOURCE_ROOT>"),
            ],
        );
    }
    let snapshot = serde_json::json!({
        "provider_identity": provider_identity(&test.config),
        "before_compaction": before,
        "compaction_request": compact,
        "after_compaction": after,
    });
    insta::assert_snapshot!(
        "local_compaction_three_request_final_wire",
        serde_json::to_string_pretty(&snapshot)?
    );
    Ok(())
}
