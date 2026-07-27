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
fn agent_manifest_owner_applies_to_call_and_output_without_a_current_node() {
    let call = fixtures()
        .into_iter()
        .find(|item| {
            matches!(
                item,
                ResponseItem::FunctionCall { call_id, .. } if call_id == "function-call"
            )
        })
        .unwrap();
    let output = fixtures()
        .into_iter()
        .find(|item| {
            matches!(
                item,
                ResponseItem::FunctionCallOutput { call_id, .. } if call_id == "function-call"
            )
        })
        .unwrap();
    let mut store = TaskSpaceEventStore::new();
    store.record_item(&call, None, None, 1).unwrap();
    store
        .bind_call_owner("function-call", "node-from-agent-manifest")
        .unwrap();
    store.record_item(&output, None, None, 2).unwrap();

    assert!(store.events().iter().all(|event| {
        event.owner == TaskSpaceEventOwner::Node("node-from-agent-manifest".to_string())
    }));
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
fn initialization_sources_reference_user_and_control_events() {
    let mut store = TaskSpaceEventStore::new();
    let user = ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText {
            text: "task".into(),
        }],
        end_turn: None,
        phase: None,
    };
    let call = ResponseItem::FunctionCall {
        id: None,
        name: "taskspace_control".into(),
        namespace: None,
        arguments: r#"{"action":"initialize_map"}"#.into(),
        call_id: "control-call".into(),
    };
    store.record_item(&user, None, None, 1).unwrap();
    store.record_item(&call, None, None, 2).unwrap();

    assert_eq!(
        store.initialization_source_event_ids("control-call"),
        vec!["task-event-1", "task-event-2"]
    );
    assert_eq!(
        store.event_id_for_call("control-call").as_deref(),
        Some("task-event-2")
    );
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

#[test]
fn compaction_checkpoint_preserves_raw_events_and_exposes_one_replacement_view() {
    let mut store = TaskSpaceEventStore::new();
    let user = ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText {
            text: "raw task body".into(),
        }],
        end_turn: None,
        phase: None,
    };
    let output_ref = format!("output-ref://sha256/{}", "a".repeat(64));
    let assistant = ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText {
            text: format!("raw assistant body {output_ref}"),
        }],
        end_turn: None,
        phase: None,
    };
    store.record_item(&user, None, None, 1).unwrap();
    store.record_item(&assistant, None, None, 2).unwrap();
    let summary = ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText {
            text: "checkpoint summary".into(),
        }],
        end_turn: None,
        phase: None,
    };

    let checkpoint = store
        .install_compaction_checkpoint(vec![summary.clone()], 3)
        .unwrap();
    let suffix = ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText {
            text: "after checkpoint".into(),
        }],
        end_turn: None,
        phase: None,
    };
    store.record_item(&suffix, None, None, 4).unwrap();

    assert_eq!(store.events().len(), 4);
    assert_eq!(checkpoint.event_type, TaskSpaceEventType::Compaction);
    let visible = store.linearize();
    assert_eq!(visible.len(), 3);
    assert!(
        matches!(&visible[0], ResponseItem::Message { role, content, .. }
        if role == "developer"
            && matches!(&content[0], ContentItem::InputText { text }
                if text.contains("covered_sequence_range: 1-2")
                    && text.contains(&output_ref)))
    );
    assert_eq!(visible[1], summary);
    assert_eq!(visible[2], suffix);
    let visible_text = serde_json::to_string(&visible).unwrap();
    assert!(!visible_text.contains("raw task body"));
    assert!(!visible_text.contains("raw assistant body"));

    let restored = TaskSpaceEventStore::restore(store.events().to_vec()).unwrap();
    assert_eq!(restored.linearize(), visible);
}

#[test]
fn compaction_checkpoint_drops_stale_taskspace_runtime_context() {
    let mut store = TaskSpaceEventStore::new();
    store
        .record_item(
            &ResponseItem::Message {
                id: None,
                role: "user".into(),
                content: vec![ContentItem::InputText {
                    text: "task".into(),
                }],
                end_turn: None,
                phase: None,
            },
            None,
            None,
            1,
        )
        .unwrap();
    let stale_projection = ResponseItem::Message {
        id: None,
        role: "developer".into(),
        content: vec![ContentItem::InputText {
            text: "TaskSpaceMapProjectionR7V1:\n- map: none".into(),
        }],
        end_turn: None,
        phase: None,
    };
    store
        .install_compaction_checkpoint(vec![stale_projection], 2)
        .unwrap();

    let visible = serde_json::to_string(&store.linearize()).unwrap();
    assert!(!visible.contains("TaskSpaceMapProjectionR7V1"));
    assert!(visible.contains("TaskSpaceCompactionCheckpointV1"));
}

#[test]
fn restore_rejects_checkpoint_when_covered_raw_event_changed() {
    let mut store = TaskSpaceEventStore::new();
    store
        .record_item(
            &ResponseItem::Message {
                id: None,
                role: "user".into(),
                content: vec![ContentItem::InputText {
                    text: "task".into(),
                }],
                end_turn: None,
                phase: None,
            },
            None,
            None,
            1,
        )
        .unwrap();
    store.install_compaction_checkpoint(Vec::new(), 2).unwrap();
    let mut events = store.events().to_vec();
    events[0].raw_payload["content"][0]["text"] = serde_json::json!("changed");

    assert_eq!(
        TaskSpaceEventStore::restore(events).unwrap_err(),
        TaskSpaceEventCodecError::CheckpointHashMismatch
    );
}

fn terminal_control_call(final_summary: &str, call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "taskspace_control".into(),
        namespace: None,
        arguments: serde_json::json!({
            "action": "finish_map",
            "expected_revision": 4,
            "terminal_node_id": "finish",
            "final_summary": final_summary
        })
        .to_string(),
        call_id: call_id.into(),
    }
}

fn taskspace_control_output(call_id: &str, status: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: call_id.into(),
        output: FunctionCallOutputPayload::from_text(
            serde_json::json!({"status": status, "steps": []}).to_string(),
        ),
    }
}

fn assistant_final_answer(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText { text: text.into() }],
        end_turn: Some(true),
        phase: Some(MessagePhase::FinalAnswer),
    }
}

#[test]
fn terminal_success_keeps_control_feedback_visible() {
    let mut store = TaskSpaceEventStore::new();
    let call = terminal_control_call("done", "terminal-control");
    let output = taskspace_control_output("terminal-control", "committed");
    let final_answer = assistant_final_answer("done");
    store.record_item(&call, None, None, 1).unwrap();
    store.record_item(&output, None, None, 2).unwrap();
    store.record_item(&final_answer, None, None, 3).unwrap();

    assert_eq!(store.events().len(), 3);
    assert_eq!(
        store.linearize(),
        vec![call.clone(), output.clone(), final_answer.clone()]
    );
    assert_eq!(store.take_linearized(), vec![call, output, final_answer]);
}

#[test]
fn terminal_control_pair_stays_visible_without_matching_successful_final() {
    for (output_status, final_text, expected_len) in [
        ("committed", None, 2),
        ("committed", Some("different"), 3),
        ("state_machine_failed", Some("done"), 3),
    ] {
        let mut store = TaskSpaceEventStore::new();
        let call = terminal_control_call("done", "terminal-control");
        let output = taskspace_control_output("terminal-control", output_status);
        store.record_item(&call, None, None, 1).unwrap();
        store.record_item(&output, None, None, 2).unwrap();
        if let Some(text) = final_text {
            store
                .record_item(&assistant_final_answer(text), None, None, 3)
                .unwrap();
        }

        let visible = store.linearize();
        assert_eq!(visible.len(), expected_len);
        assert_eq!(visible[0], call);
        assert_eq!(visible[1], output);
    }
}
