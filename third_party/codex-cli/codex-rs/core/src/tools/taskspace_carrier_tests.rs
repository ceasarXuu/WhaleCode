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
        &CarrierTransition::Committed(
            r#"{"action":"bind_node","state_commit":true,"committed_revision":3}"#.into(),
        ),
        true,
    );

    let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
        panic!("function output");
    };
    let text = output.body.to_text().expect("text");
    assert!(text.contains("TaskSpaceCarrierResultV1"));
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
        &CarrierTransition::Rejected("stale revision".into()),
        false,
    );
    let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
        panic!("function output");
    };
    assert!(
        output
            .body
            .to_text()
            .expect("text")
            .contains("\"tool_dispatched\":false")
    );
}
