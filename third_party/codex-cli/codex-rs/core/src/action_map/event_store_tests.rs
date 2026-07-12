use super::*;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ImageDetail;
use codex_protocol::models::LocalShellAction;
use codex_protocol::models::LocalShellExecAction;
use codex_protocol::models::LocalShellStatus;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ReasoningItemReasoningSummary;
use codex_protocol::models::WebSearchAction;

fn fixtures() -> Vec<ResponseItem> {
    vec![
        ResponseItem::Message {
            id: Some("msg-provider-id".into()),
            role: "user".into(),
            content: vec![
                ContentItem::InputText {
                    text: "task".into(),
                },
                ContentItem::InputImage {
                    image_url: "data:image/png;base64,AA==".into(),
                    detail: Some(ImageDetail::Original),
                },
            ],
            end_turn: Some(false),
            phase: Some(MessagePhase::Commentary),
        },
        ResponseItem::Reasoning {
            id: "reasoning-provider-id".into(),
            summary: vec![ReasoningItemReasoningSummary::SummaryText {
                text: "summary".into(),
            }],
            content: Some(vec![ReasoningItemContent::ReasoningText {
                text: "content".into(),
            }]),
            encrypted_content: Some("encrypted".into()),
        },
        ResponseItem::LocalShellCall {
            id: Some("shell-provider-id".into()),
            call_id: Some("shell-call".into()),
            status: LocalShellStatus::Completed,
            action: LocalShellAction::Exec(LocalShellExecAction {
                command: vec!["pwd".into()],
                timeout_ms: Some(1_000),
                working_directory: Some("/workspace".into()),
                env: None,
                user: None,
            }),
        },
        ResponseItem::FunctionCall {
            id: Some("function-provider-id".into()),
            name: "exec_command".into(),
            namespace: Some("core".into()),
            arguments: r#"{"cmd":"pwd"}"#.into(),
            call_id: "function-call".into(),
        },
        ResponseItem::ToolSearchCall {
            id: Some("search-provider-id".into()),
            call_id: Some("search-call".into()),
            status: Some("completed".into()),
            execution: "client".into(),
            arguments: serde_json::json!({"query":"tool"}),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "function-call".into(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::ContentItems(vec![
                    FunctionCallOutputContentItem::InputText {
                        text: "output-ref://sha256/abc\n[truncated]".into(),
                    },
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "data:image/png;base64,AA==".into(),
                        detail: Some(ImageDetail::High),
                    },
                ]),
                success: Some(false),
            },
        },
        ResponseItem::CustomToolCall {
            id: Some("custom-provider-id".into()),
            status: Some("completed".into()),
            call_id: "custom-call".into(),
            name: "apply_patch".into(),
            input: "*** Begin Patch".into(),
        },
        ResponseItem::CustomToolCallOutput {
            call_id: "custom-call".into(),
            name: Some("apply_patch".into()),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text("Done".into()),
                success: Some(true),
            },
        },
        ResponseItem::ToolSearchOutput {
            call_id: Some("search-call".into()),
            status: "completed".into(),
            execution: "client".into(),
            tools: vec![serde_json::json!({"name":"exec_command"})],
        },
        ResponseItem::WebSearchCall {
            id: Some("web-provider-id".into()),
            status: Some("completed".into()),
            action: Some(WebSearchAction::Search {
                query: Some("query".into()),
                queries: None,
            }),
        },
        ResponseItem::ImageGenerationCall {
            id: "image-call".into(),
            status: "completed".into(),
            revised_prompt: Some("prompt".into()),
            result: "image-result".into(),
        },
        ResponseItem::Compaction {
            encrypted_content: "checkpoint".into(),
        },
    ]
}

#[test]
fn response_items_round_trip_without_field_loss() {
    for (index, item) in fixtures().into_iter().enumerate() {
        let event = TaskSpaceEvent::from_response_item(
            format!("event-{}", index + 1),
            u64::try_from(index + 1).unwrap(),
            if index == 0 {
                TaskSpaceEventOwner::Root
            } else {
                TaskSpaceEventOwner::Node("node-1".into())
            },
            (index > 2).then(|| "parent-control".into()),
            &item,
            100 + i64::try_from(index).unwrap(),
        )
        .unwrap();
        let persisted = serde_json::to_value(&event).unwrap();
        let restored_event: TaskSpaceEvent = serde_json::from_value(persisted).unwrap();
        assert_eq!(restored_event.to_response_item().unwrap(), item);
    }
}

#[test]
fn codec_preserves_global_sequence_and_parent_link() {
    let events = fixtures()
        .into_iter()
        .take(3)
        .enumerate()
        .map(|(index, item)| {
            TaskSpaceEvent::from_response_item(
                format!("event-{}", index + 1),
                u64::try_from(index + 1).unwrap(),
                TaskSpaceEventOwner::Node("node-1".into()),
                Some("outer-call".into()),
                &item,
                1,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(
        events
            .iter()
            .all(|event| event.parent_call_id.as_deref() == Some("outer-call"))
    );
}

#[test]
fn unsupported_and_corrupt_items_fail_explicitly() {
    let error = TaskSpaceEvent::from_response_item(
        "event-other",
        1,
        TaskSpaceEventOwner::Root,
        None,
        &ResponseItem::Other,
        1,
    )
    .unwrap_err();
    assert_eq!(error, TaskSpaceEventCodecError::UnsupportedItem("other"));

    let mut event = TaskSpaceEvent::from_response_item(
        "event-message",
        1,
        TaskSpaceEventOwner::Root,
        None,
        &fixtures().remove(0),
        1,
    )
    .unwrap();
    event.original_role = Some("assistant".into());
    assert_eq!(
        event.to_response_item().unwrap_err(),
        TaskSpaceEventCodecError::MetadataMismatch("original_role")
    );
}

#[test]
fn store_preserves_order_and_call_pair_owner() {
    let mut store = TaskSpaceEventStore::new();
    let call = ResponseItem::FunctionCall {
        id: None,
        name: "taskspace_control".into(),
        namespace: None,
        arguments: "{}".into(),
        call_id: "control-call".into(),
    };
    let output = ResponseItem::FunctionCallOutput {
        call_id: "control-call".into(),
        output: FunctionCallOutputPayload::from_text("ok".into()),
    };
    store.record_item(&call, None, None, 1).unwrap();
    store
        .record_item(&output, Some("node-created-by-call"), None, 2)
        .unwrap();
    assert_eq!(store.linearize(), vec![call, output]);
    assert_eq!(store.events()[0].owner, TaskSpaceEventOwner::Root);
    assert_eq!(store.events()[1].owner, TaskSpaceEventOwner::Root);
}

#[test]
fn store_marks_global_items_without_reordering_them() {
    let mut store = TaskSpaceEventStore::new();
    let developer = ResponseItem::Message {
        id: None,
        role: "developer".into(),
        content: vec![ContentItem::InputText {
            text: "policy".into(),
        }],
        end_turn: None,
        phase: None,
    };
    let user = ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText {
            text: "task".into(),
        }],
        end_turn: None,
        phase: None,
    };
    store.record_item(&developer, None, None, 1).unwrap();
    store.record_item(&user, None, None, 2).unwrap();
    assert_eq!(store.events()[0].owner, TaskSpaceEventOwner::Global);
    assert_eq!(store.events()[1].owner, TaskSpaceEventOwner::Root);
    assert_eq!(store.linearize(), vec![developer, user]);
}

#[test]
fn store_restore_and_rollback_are_mechanical() {
    let mut store = TaskSpaceEventStore::new();
    for item in [
        ResponseItem::Message {
            id: None,
            role: "user".into(),
            content: vec![ContentItem::InputText {
                text: "first".into(),
            }],
            end_turn: None,
            phase: None,
        },
        ResponseItem::Message {
            id: None,
            role: "assistant".into(),
            content: vec![ContentItem::OutputText {
                text: "done".into(),
            }],
            end_turn: None,
            phase: None,
        },
        ResponseItem::Message {
            id: None,
            role: "user".into(),
            content: vec![ContentItem::InputText {
                text: "second".into(),
            }],
            end_turn: None,
            phase: None,
        },
    ] {
        store.record_item(&item, None, None, 1).unwrap();
    }
    let mut restored = TaskSpaceEventStore::restore(store.events().to_vec()).unwrap();
    restored.drop_last_n_user_turns(1);
    assert_eq!(restored.events().len(), 2);

    let mut corrupt = store.events().to_vec();
    corrupt[1].sequence = 9;
    assert_eq!(
        TaskSpaceEventStore::restore(corrupt).unwrap_err(),
        TaskSpaceEventCodecError::SequenceConflict {
            expected: 2,
            actual: 9
        }
    );
}
