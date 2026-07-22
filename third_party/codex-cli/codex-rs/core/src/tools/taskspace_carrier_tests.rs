use super::*;
use codex_protocol::models::FunctionCallOutputPayload;

#[test]
fn committed_transition_is_prepended_without_rewriting_tool_text() {
    let original = "exact tool output";
    let mut response = ResponseInputItem::FunctionCallOutput {
        call_id: "call".into(),
        output: FunctionCallOutputPayload::from_text(original.into()),
    };
    wrap_carrier_response(
        &mut response,
        &CarrierAction::TransitionCommitted(
            r#"{"action":"bind_node","state_commit":true,"committed_revision":3}"#.into(),
        ),
        true,
    );

    let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
        panic!("function output");
    };
    let text = output.body.to_text().expect("text");
    assert!(text.contains("TaskSpaceCarrierResultV2"));
    assert!(text.contains("action_result"));
    assert!(text.ends_with(original));
}

#[test]
fn transition_rejection_records_that_tool_was_not_dispatched() {
    let mut response = ResponseInputItem::FunctionCallOutput {
        call_id: "call".into(),
        output: FunctionCallOutputPayload::from_text("transition rejected".into()),
    };
    wrap_carrier_response(
        &mut response,
        &CarrierAction::Rejected("stale revision".into()),
        false,
    );
    let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
        panic!("function output");
    };
    let text = output.body.to_text().expect("text");
    assert!(text.contains("\"tool_dispatched\":false"));
    assert_eq!(text.matches("stale revision").count(), 1);
}

#[test]
fn validated_continuation_does_not_pollute_the_tool_result() {
    let original = "exact tool output";
    let mut response = ResponseInputItem::FunctionCallOutput {
        call_id: "call".into(),
        output: FunctionCallOutputPayload::from_text(original.into()),
    };

    wrap_carrier_response(&mut response, &CarrierAction::ContinueValidated, true);

    let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
        panic!("function output");
    };
    assert_eq!(output.body.to_text().as_deref(), Some(original));
}

#[test]
fn continue_current_accepts_exact_revision_and_binding() {
    assert_eq!(
        continue_current_failure(7, "implement", Some((7, Some("implement")))),
        None
    );
}

#[test]
fn continue_current_reports_exact_mechanical_mismatch() {
    let cases = [
        (
            continue_current_failure(7, "implement", None),
            "TASKSPACE_NO_ACTIVE_MAP",
        ),
        (
            continue_current_failure(7, "implement", Some((8, Some("implement")))),
            "TASKSPACE_REVISION_MISMATCH",
        ),
        (
            continue_current_failure(7, "implement", Some((7, Some("explore")))),
            "TASKSPACE_BINDING_MISMATCH",
        ),
    ];

    for (failure, expected_code) in cases {
        let value: serde_json::Value =
            serde_json::from_str(&failure.expect("expected rejection")).expect("valid JSON");
        assert_eq!(value["error"]["code"], expected_code);
        assert_eq!(value["state_commit"], false);
    }
}
