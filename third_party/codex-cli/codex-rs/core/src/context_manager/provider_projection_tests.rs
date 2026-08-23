use super::*;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ReasoningItemReasoningSummary;

fn function_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "tool".into(),
        namespace: None,
        arguments: "{}".into(),
        encrypted_function_args: Some(vec!["private-args".into()]),
        call_id: call_id.into(),
        internal_chat_message_metadata_passthrough: None,
    }
}

fn encrypted_output(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        id: None,
        call_id: call_id.into(),
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::ContentItems(vec![
                FunctionCallOutputContentItem::EncryptedContent {
                    encrypted_content: "private-output".into(),
                },
            ]),
            success: Some(true),
        },
        internal_chat_message_metadata_passthrough: None,
    }
}

#[test]
fn non_openai_projection_removes_opaque_fields_without_mutating_source() {
    let source = vec![
        ResponseItem::Reasoning {
            id: None,
            summary: vec![ReasoningItemReasoningSummary::SummaryText {
                text: "readable".into(),
            }],
            content: None,
            encrypted_content: Some("openai-private".into()),
            internal_chat_message_metadata_passthrough: None,
        },
        function_call("call-readable"),
        ResponseItem::FunctionCallOutput {
            id: None,
            call_id: "call-readable".into(),
            output: FunctionCallOutputPayload::from_text("result".into()),
            internal_chat_message_metadata_passthrough: None,
        },
    ];
    let canonical = source.clone();

    let projected = project_history_for_provider(source.clone(), false, &[InputModality::Text]);

    assert_eq!(source, canonical);
    assert!(matches!(
        &projected[0],
        ResponseItem::Reasoning {
            encrypted_content: None,
            ..
        }
    ));
    assert!(matches!(
        &projected[1],
        ResponseItem::FunctionCall {
            encrypted_function_args: None,
            ..
        }
    ));
}

#[test]
fn openai_projection_preserves_private_history_for_switch_back() {
    let source = vec![function_call("call-1"), encrypted_output("call-1")];
    assert_eq!(
        project_history_for_provider(source.clone(), true, &[InputModality::Text]),
        source
    );
}

#[test]
fn private_only_output_removes_its_call_as_a_pair() {
    let source = vec![function_call("private"), encrypted_output("private")];
    let projected = project_history_for_provider(source, false, &[InputModality::Text]);
    assert!(projected.is_empty());
}

#[test]
fn hosted_items_and_opaque_compaction_are_removed() {
    let source = vec![
        ResponseItem::WebSearchCall {
            id: None,
            status: Some("completed".into()),
            action: None,
            internal_chat_message_metadata_passthrough: None,
        },
        ResponseItem::Compaction {
            id: None,
            encrypted_content: "provider-private".into(),
            internal_chat_message_metadata_passthrough: None,
        },
    ];
    assert!(project_history_for_provider(source, false, &[InputModality::Text]).is_empty());
}

#[test]
fn missing_output_is_closed_after_projection() {
    let projected = project_history_for_provider(
        vec![function_call("aborted")],
        false,
        &[InputModality::Text],
    );
    assert_eq!(projected.len(), 2);
    assert!(matches!(
        &projected[1],
        ResponseItem::FunctionCallOutput { call_id, .. } if call_id == "aborted"
    ));
}

#[test]
fn target_modalities_are_applied_to_the_projected_copy() {
    let source = vec![ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputImage {
            image_url: "data:image/png;base64,private".into(),
            detail: None,
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }];
    let canonical = source.clone();

    let projected = project_history_for_provider(source.clone(), false, &[InputModality::Text]);

    assert_eq!(source, canonical);
    assert!(
        !serde_json::to_string(&projected)
            .expect("projection should serialize")
            .contains("data:image")
    );
}
