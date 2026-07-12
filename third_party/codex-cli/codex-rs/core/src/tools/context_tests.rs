use super::*;
use codex_protocol::models::DEFAULT_IMAGE_DETAIL;
use core_test_support::assert_regex_match;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn terminal_candidate_is_redacted_only_from_tool_logs() {
    let payload = ToolPayload::Function {
        arguments: serde_json::json!({
            "action": "finish_then_end",
            "terminal_finish": {},
            "final_candidate": "private final text"
        })
        .to_string(),
    };
    let logged = payload
        .log_payload_for_tool(&ToolName::plain("taskspace_control"))
        .into_owned();

    assert!(!logged.contains("private final text"));
    assert!(logged.contains("\"redacted\":true"));
    assert!(payload.log_payload().contains("private final text"));
}

#[test]
fn custom_tool_calls_should_roundtrip_as_custom_outputs() {
    let payload = ToolPayload::Custom {
        input: "patch".to_string(),
    };
    let response = FunctionToolOutput::from_text("patched".to_string(), Some(true))
        .to_response_item("call-42", &payload);

    match response {
        ResponseInputItem::CustomToolCallOutput {
            call_id, output, ..
        } => {
            assert_eq!(call_id, "call-42");
            assert_eq!(output.content_items(), None);
            assert_eq!(output.body.to_text().as_deref(), Some("patched"));
            assert_eq!(output.success, Some(true));
        }
        other => panic!("expected CustomToolCallOutput, got {other:?}"),
    }
}

#[test]
fn function_payloads_remain_function_outputs() {
    let payload = ToolPayload::Function {
        arguments: "{}".to_string(),
    };
    let response = FunctionToolOutput::from_text("ok".to_string(), Some(true))
        .to_response_item("fn-1", &payload);

    match response {
        ResponseInputItem::FunctionCallOutput { call_id, output } => {
            assert_eq!(call_id, "fn-1");
            assert_eq!(output.content_items(), None);
            assert_eq!(output.body.to_text().as_deref(), Some("ok"));
            assert_eq!(output.success, Some(true));
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}

#[test]
fn mcp_code_mode_result_serializes_full_call_tool_result() {
    let output = CallToolResult {
        content: vec![serde_json::json!({
            "type": "text",
            "text": "ignored",
        })],
        structured_content: Some(serde_json::json!({
            "threadId": "thread_123",
            "content": "done",
        })),
        is_error: Some(false),
        meta: Some(serde_json::json!({
            "source": "mcp",
        })),
    };

    let result = output.code_mode_result(&ToolPayload::Mcp {
        server: "server".to_string(),
        tool: "tool".to_string(),
        raw_arguments: "{}".to_string(),
    });

    assert_eq!(
        result,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": "ignored",
            }],
            "structuredContent": {
                "threadId": "thread_123",
                "content": "done",
            },
            "isError": false,
            "_meta": {
                "source": "mcp",
            },
        })
    );
}

#[test]
fn mcp_tool_output_response_item_includes_wall_time() {
    let output = McpToolOutput {
        result: CallToolResult {
            content: vec![serde_json::json!({
                "type": "text",
                "text": "done",
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        },
        tool_input: json!({}),
        wall_time: std::time::Duration::from_millis(1250),
        original_image_detail_supported: false,
        truncation_policy: TruncationPolicy::Bytes(1024),
    };

    let response = output.to_response_item(
        "mcp-call-1",
        &ToolPayload::Mcp {
            server: "server".to_string(),
            tool: "tool".to_string(),
            raw_arguments: "{}".to_string(),
        },
    );

    match response {
        ResponseInputItem::FunctionCallOutput { call_id, output } => {
            assert_eq!(call_id, "mcp-call-1");
            assert_eq!(output.success, Some(true));
            let Some(text) = output.body.to_text() else {
                panic!("MCP output should serialize as text");
            };
            let Some(payload) = text.strip_prefix("Wall time: 1.2500 seconds\nOutput:\n") else {
                panic!("MCP output should include wall-time header: {text}");
            };
            let parsed: serde_json::Value = serde_json::from_str(payload).unwrap_or_else(|err| {
                panic!("MCP output should serialize JSON content: {err}");
            });
            assert_eq!(
                parsed,
                json!([{
                    "type": "text",
                    "text": "done",
                }])
            );
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}

#[test]
fn mcp_tool_output_response_item_truncates_large_structured_content() {
    let output = McpToolOutput {
        result: CallToolResult {
            content: vec![serde_json::json!({
                "type": "text",
                "text": "ignored when structured content is present",
            })],
            structured_content: Some(serde_json::json!({
                "items": "large structured value ".repeat(1_000),
            })),
            is_error: Some(false),
            meta: None,
        },
        tool_input: json!({}),
        wall_time: std::time::Duration::from_millis(1250),
        original_image_detail_supported: false,
        truncation_policy: TruncationPolicy::Bytes(128),
    };

    let response = output.to_response_item(
        "mcp-call-large",
        &ToolPayload::Mcp {
            server: "server".to_string(),
            tool: "tool".to_string(),
            raw_arguments: "{}".to_string(),
        },
    );

    match response {
        ResponseInputItem::FunctionCallOutput { call_id, output } => {
            assert_eq!(call_id, "mcp-call-large");
            assert_eq!(output.success, Some(true));
            let text = output
                .body
                .to_text()
                .expect("MCP output should serialize as text");
            assert!(text.starts_with("Wall time: 1.2500 seconds\nOutput:\n"));
            assert!(text.contains("chars truncated"));
            assert!(!text.contains("ignored when structured content is present"));
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}

#[test]
fn mcp_tool_output_response_item_preserves_content_items() {
    let image_url = "data:image/png;base64,AAA";
    let output = McpToolOutput {
        result: CallToolResult {
            content: vec![serde_json::json!({
                "type": "image",
                "mimeType": "image/png",
                "data": "AAA",
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        },
        tool_input: json!({}),
        wall_time: std::time::Duration::from_millis(500),
        original_image_detail_supported: false,
        truncation_policy: TruncationPolicy::Bytes(1024),
    };

    let response = output.to_response_item(
        "mcp-call-2",
        &ToolPayload::Mcp {
            server: "server".to_string(),
            tool: "tool".to_string(),
            raw_arguments: "{}".to_string(),
        },
    );

    match response {
        ResponseInputItem::FunctionCallOutput { output, .. } => {
            assert_eq!(
                output.content_items(),
                Some(
                    vec![
                        FunctionCallOutputContentItem::InputText {
                            text: "Wall time: 0.5000 seconds\nOutput:".to_string(),
                        },
                        FunctionCallOutputContentItem::InputImage {
                            image_url: image_url.to_string(),
                            detail: Some(DEFAULT_IMAGE_DETAIL),
                        },
                    ]
                    .as_slice()
                )
            );
            assert_eq!(
                output.body.to_text().as_deref(),
                Some("Wall time: 0.5000 seconds\nOutput:")
            );
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}

#[test]
fn mcp_tool_output_code_mode_result_stays_raw_call_tool_result() {
    let large_content = "large structured value ".repeat(1_000);
    let output = McpToolOutput {
        result: CallToolResult {
            content: vec![serde_json::json!({
                "type": "text",
                "text": "ignored",
            })],
            structured_content: Some(serde_json::json!({
                "content": large_content,
            })),
            is_error: Some(false),
            meta: None,
        },
        tool_input: json!({}),
        wall_time: std::time::Duration::from_millis(1250),
        original_image_detail_supported: false,
        truncation_policy: TruncationPolicy::Bytes(64),
    };

    let result = output.code_mode_result(&ToolPayload::Mcp {
        server: "server".to_string(),
        tool: "tool".to_string(),
        raw_arguments: "{}".to_string(),
    });

    assert_eq!(
        result,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": "ignored",
            }],
            "structuredContent": {
                "content": "large structured value ".repeat(1_000),
            },
            "isError": false,
        })
    );
}

#[test]
fn custom_tool_calls_can_derive_text_from_content_items() {
    let payload = ToolPayload::Custom {
        input: "patch".to_string(),
    };
    let response = FunctionToolOutput::from_content(
        vec![
            FunctionCallOutputContentItem::InputText {
                text: "line 1".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
            FunctionCallOutputContentItem::InputText {
                text: "line 2".to_string(),
            },
        ],
        Some(true),
    )
    .to_response_item("call-99", &payload);

    match response {
        ResponseInputItem::CustomToolCallOutput {
            call_id, output, ..
        } => {
            let expected = vec![
                FunctionCallOutputContentItem::InputText {
                    text: "line 1".to_string(),
                },
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,AAA".to_string(),
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                },
                FunctionCallOutputContentItem::InputText {
                    text: "line 2".to_string(),
                },
            ];
            assert_eq!(call_id, "call-99");
            assert_eq!(output.content_items(), Some(expected.as_slice()));
            assert_eq!(output.body.to_text().as_deref(), Some("line 1\nline 2"));
            assert_eq!(output.success, Some(true));
        }
        other => panic!("expected CustomToolCallOutput, got {other:?}"),
    }
}

#[test]
fn tool_search_payloads_roundtrip_as_tool_search_outputs() {
    let payload = ToolPayload::ToolSearch {
        arguments: SearchToolCallParams {
            query: "calendar".to_string(),
            limit: None,
        },
    };
    let response = ToolSearchOutput {
        tools: vec![LoadableToolSpec::Function(codex_tools::ResponsesApiTool {
            name: "create_event".to_string(),
            description: String::new(),
            strict: false,
            defer_loading: Some(true),
            parameters: codex_tools::JsonSchema::object(
                /*properties*/ Default::default(),
                /*required*/ None,
                /*additional_properties*/ None,
            ),
            output_schema: None,
        })],
    }
    .to_response_item("search-1", &payload);

    match response {
        ResponseInputItem::ToolSearchOutput {
            call_id,
            status,
            execution,
            tools,
        } => {
            assert_eq!(call_id, "search-1");
            assert_eq!(status, "completed");
            assert_eq!(execution, "client");
            assert_eq!(
                tools,
                vec![json!({
                    "type": "function",
                    "name": "create_event",
                    "description": "",
                    "strict": false,
                    "defer_loading": true,
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                })]
            );
        }
        other => panic!("expected ToolSearchOutput, got {other:?}"),
    }
}

#[test]
fn log_preview_uses_content_items_when_plain_text_is_missing() {
    let output = FunctionToolOutput::from_content(
        vec![FunctionCallOutputContentItem::InputText {
            text: "preview".to_string(),
        }],
        Some(true),
    );

    assert_eq!(output.log_preview(), "preview");
    assert_eq!(
        function_call_output_content_items_to_text(&output.body),
        Some("preview".to_string())
    );
}

#[test]
fn model_visible_preview_uses_response_item_not_log_preview() {
    struct DivergentToolOutput;

    impl ToolOutput for DivergentToolOutput {
        fn log_preview(&self) -> String {
            "log-only-preview".to_string()
        }

        fn success_for_logging(&self) -> bool {
            true
        }

        fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
            function_tool_response(
                call_id,
                payload,
                vec![FunctionCallOutputContentItem::InputText {
                    text: "standard model-visible feedback".to_string(),
                }],
                Some(true),
            )
        }
    }

    let payload = ToolPayload::Function {
        arguments: "{}".to_string(),
    };
    let preview = tool_output_model_visible_preview(&DivergentToolOutput, "call-1", &payload);

    assert!(preview.contains("standard model-visible feedback"));
    assert!(!preview.contains("log-only-preview"));
}

#[test]
fn taskspace_preview_preserves_raw_exec_output_without_semantic_summary() {
    let raw_output = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        "{'name': 'Madrid', 'member_ids': ['E001']}: 'members' is a required property",
        "{'name': 'Ferrari', 'member_ids': ['E002']}: 'members' is a required property",
        "x".repeat(TELEMETRY_PREVIEW_MAX_BYTES + 256),
        "{'statistics': {}}: 'averageDepartmentBudget' is a required property",
        "{'statistics': {}}: 'totalEmployees' is a required property",
        "{'statistics': {}}: 'skillDistribution' is a required property",
        "{'statistics': {}}: 'departmentSizes' is a required property",
        "{'statistics': {}}: 'projectStatusDistribution' is a required property",
        "{'statistics': {}}: 'averageYearsOfService' is a required property",
    );
    let output = ExecCommandToolOutput {
        event_call_id: "event-1".to_string(),
        chunk_id: "chunk-1".to_string(),
        wall_time: std::time::Duration::from_millis(200),
        raw_output: raw_output.into_bytes(),
        artifact_ref: None,
        max_output_tokens: None,
        process_id: None,
        exit_code: Some(1),
        original_token_count: None,
        hook_command: None,
    };
    let payload = ToolPayload::Function {
        arguments: serde_json::json!({
            "command": "python generate_org.py && python -m jsonschema -i organization.json schema.json"
        })
        .to_string(),
    };

    let preview = tool_output_model_visible_preview(&output, "call-1", &payload);

    assert!(!preview.contains("TaskSpaceToolSemanticSummaryV1"));
    assert!(preview.contains("'members' is a required property"));
    assert!(preview.contains(TELEMETRY_PREVIEW_TRUNCATION_NOTICE));
}

#[test]
fn taskspace_preview_preserves_read_file_summary_after_telemetry_truncation() {
    let summary = "TaskSpaceReadFileSummaryV1: path=process_csv.py lines_read=240 eof_reached=false max_lines=240";
    let raw_output = format!(
        "import csv\n{}\n{summary}\n",
        "x".repeat(TELEMETRY_PREVIEW_MAX_BYTES + 256)
    );
    let output = ExecCommandToolOutput {
        event_call_id: "event-1".to_string(),
        chunk_id: "chunk-1".to_string(),
        wall_time: std::time::Duration::from_millis(200),
        raw_output: raw_output.into_bytes(),
        artifact_ref: None,
        max_output_tokens: None,
        process_id: None,
        exit_code: Some(0),
        original_token_count: None,
        hook_command: None,
    };
    let payload = ToolPayload::Function {
        arguments: serde_json::json!({
            "command": "sed -n '1,240p' -- process_csv.py && awk '{ printf \"TaskSpaceReadFileSummaryV1: path=%s\" }' process_csv.py"
        })
        .to_string(),
    };

    let preview = tool_output_model_visible_preview(&output, "call-1", &payload);

    assert!(preview.contains(TELEMETRY_PREVIEW_TRUNCATION_NOTICE));
    assert!(preview.contains("TaskSpaceToolTailSentinelV1"));
    assert!(preview.contains(summary));
    assert!(
        preview.rfind(summary).expect("summary present")
            > preview
                .find(TELEMETRY_PREVIEW_TRUNCATION_NOTICE)
                .expect("truncation present")
    );
}

#[test]
fn taskspace_preview_preserves_complete_read_file_content_beyond_telemetry_limit() {
    let summary = "TaskSpaceReadFileSummaryV1: path=generate_organization.py lines_read=95 eof_reached=true max_lines=240";
    let body = (0..90)
        .map(|idx| {
            format!(
                "line_{idx:02} = 'schema repair context averageDepartmentBudget members projects'"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let late_line = "line_94 = 'late project members and averageDepartmentBudget fix target'";
    let raw_output = format!("{body}\n{late_line}\n{summary}\n");
    assert!(raw_output.len() > TELEMETRY_PREVIEW_MAX_BYTES);
    let output = ExecCommandToolOutput {
        event_call_id: "event-1".to_string(),
        chunk_id: "chunk-1".to_string(),
        wall_time: std::time::Duration::from_millis(200),
        raw_output: raw_output.into_bytes(),
        artifact_ref: None,
        max_output_tokens: None,
        process_id: None,
        exit_code: Some(0),
        original_token_count: None,
        hook_command: None,
    };
    let payload = ToolPayload::Function {
        arguments: serde_json::json!({
            "command": "sed -n '1,240p' -- generate_organization.py && awk '{ printf \"TaskSpaceReadFileSummaryV1\" }' generate_organization.py"
        })
        .to_string(),
    };

    let preview = tool_output_model_visible_preview(&output, "call-1", &payload);

    assert!(!preview.contains(TELEMETRY_PREVIEW_TRUNCATION_NOTICE));
    assert!(preview.contains("line_00 = 'schema repair context"));
    assert!(preview.contains(late_line));
    assert!(preview.contains(summary));
}

#[test]
fn taskspace_preview_does_not_add_schema_summary_for_plain_exec_output() {
    let output = ExecCommandToolOutput {
        event_call_id: "event-1".to_string(),
        chunk_id: "chunk-1".to_string(),
        wall_time: std::time::Duration::from_millis(200),
        raw_output: format!(
            "ordinary failure\n{}",
            "x".repeat(TELEMETRY_PREVIEW_MAX_BYTES + 32)
        )
        .into_bytes(),
        artifact_ref: None,
        max_output_tokens: None,
        process_id: None,
        exit_code: Some(1),
        original_token_count: None,
        hook_command: None,
    };
    let payload = ToolPayload::Function {
        arguments: serde_json::json!({ "command": "pytest" }).to_string(),
    };

    let preview = tool_output_model_visible_preview(&output, "call-1", &payload);

    assert!(!preview.contains("TaskSpaceToolSemanticSummaryV1"));
    assert!(!preview.contains("missing_required_properties:"));
    assert!(preview.contains("ordinary failure"));
}

#[test]
fn telemetry_preview_returns_original_within_limits() {
    let content = "short output";
    assert_eq!(telemetry_preview(content), content);
}

#[test]
fn telemetry_preview_truncates_by_bytes() {
    let content = "x".repeat(TELEMETRY_PREVIEW_MAX_BYTES + 8);
    let preview = telemetry_preview(&content);

    assert!(preview.contains(TELEMETRY_PREVIEW_TRUNCATION_NOTICE));
    assert!(
        preview.len()
            <= TELEMETRY_PREVIEW_MAX_BYTES + TELEMETRY_PREVIEW_TRUNCATION_NOTICE.len() + 1
    );
}

#[test]
fn telemetry_preview_truncates_by_lines() {
    let content = (0..(TELEMETRY_PREVIEW_MAX_LINES + 5))
        .map(|idx| format!("line {idx}"))
        .collect::<Vec<_>>()
        .join("\n");

    let preview = telemetry_preview(&content);
    let lines: Vec<&str> = preview.lines().collect();

    assert!(lines.len() <= TELEMETRY_PREVIEW_MAX_LINES + 1);
    assert_eq!(lines.last(), Some(&TELEMETRY_PREVIEW_TRUNCATION_NOTICE));
}

#[test]
fn exec_command_tool_output_formats_truncated_response() {
    let payload = ToolPayload::Function {
        arguments: "{}".to_string(),
    };
    let response = ExecCommandToolOutput {
        event_call_id: "call-42".to_string(),
        chunk_id: "abc123".to_string(),
        wall_time: std::time::Duration::from_millis(1250),
        raw_output: b"token one token two token three token four token five".to_vec(),
        artifact_ref: None,
        max_output_tokens: Some(4),
        process_id: None,
        exit_code: Some(0),
        original_token_count: Some(10),
        hook_command: None,
    }
    .to_response_item("call-42", &payload);

    match response {
        ResponseInputItem::FunctionCallOutput { call_id, output } => {
            assert_eq!(call_id, "call-42");
            assert_eq!(output.success, Some(true));
            let text = output
                .body
                .to_text()
                .expect("exec output should serialize as text");
            assert_regex_match(
                r#"(?sx)
                    ^Chunk\ ID:\ abc123
                    \nWall\ time:\ \d+\.\d{4}\ seconds
                    \nProcess\ exited\ with\ code\ 0
                    \nOriginal\ token\ count:\ 10
                    \nOutput:
                    \n.*tokens\ truncated.*
                    $"#,
                &text,
            );
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}

#[test]
fn exec_command_tool_output_referenceizes_large_response() {
    let payload = ToolPayload::Function {
        arguments: "{}".to_string(),
    };
    let mut raw_output = Vec::new();
    raw_output.extend_from_slice(b"head-visible\n");
    raw_output.extend_from_slice("middle-secret-marker\n".repeat(4_000).as_bytes());
    raw_output.extend_from_slice(b"tail-visible\n");

    let response = ExecCommandToolOutput {
        event_call_id: "call-large".to_string(),
        chunk_id: "chunk-large".to_string(),
        wall_time: std::time::Duration::from_millis(250),
        raw_output,
        artifact_ref: Some("output-ref://sha256/test-large".to_string()),
        max_output_tokens: Some(100_000),
        process_id: None,
        exit_code: Some(0),
        original_token_count: Some(12_000),
        hook_command: None,
    }
    .to_response_item("call-large", &payload);

    match response {
        ResponseInputItem::FunctionCallOutput { call_id, output } => {
            assert_eq!(call_id, "call-large");
            let text = output
                .body
                .to_text()
                .expect("exec output should serialize as text");
            assert!(text.contains("OutputReferenceV1:"));
            assert!(text.contains("policy: referenced_large_output"));
            assert!(text.contains("artifact_ref: output-ref://sha256/test-large"));
            assert!(text.contains("sha256:"));
            assert!(text.contains("head-visible"));
            assert!(text.contains("tail-visible"));
            assert!(
                text.matches("middle-secret-marker").count() < 300,
                "large middle output should not replay inline"
            );
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}

#[test]
fn exec_command_tool_output_summarizes_medium_response() {
    let payload = ToolPayload::Function {
        arguments: "{}".to_string(),
    };
    let mut raw_output = Vec::new();
    raw_output.extend_from_slice(b"medium-head\n");
    raw_output.extend_from_slice("medium-middle-marker\n".repeat(900).as_bytes());
    raw_output.extend_from_slice(b"medium-tail\n");

    let response = ExecCommandToolOutput {
        event_call_id: "call-medium".to_string(),
        chunk_id: "chunk-medium".to_string(),
        wall_time: std::time::Duration::from_millis(250),
        raw_output,
        artifact_ref: Some("output-ref://sha256/test-medium".to_string()),
        max_output_tokens: Some(100_000),
        process_id: None,
        exit_code: Some(0),
        original_token_count: Some(4_000),
        hook_command: None,
    }
    .to_response_item("call-medium", &payload);

    match response {
        ResponseInputItem::FunctionCallOutput { call_id, output } => {
            assert_eq!(call_id, "call-medium");
            let text = output
                .body
                .to_text()
                .expect("exec output should serialize as text");
            assert!(text.contains("OutputReferenceV1:"));
            assert!(text.contains("policy: summarized_medium_output"));
            assert!(text.contains("artifact_ref: output-ref://sha256/test-medium"));
            assert!(text.contains("medium-head"));
            assert!(text.contains("medium-tail"));
            assert!(
                text.matches("medium-middle-marker").count() < 300,
                "medium output should not replay its full middle inline"
            );
        }
        other => panic!("expected FunctionCallOutput, got {other:?}"),
    }
}
