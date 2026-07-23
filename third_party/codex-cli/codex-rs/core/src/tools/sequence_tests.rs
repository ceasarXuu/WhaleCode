use super::*;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallExecution;
use crate::tools::provider_tool_declaration::ProviderToolDeclaration;
use crate::tools::sequence_preflight::REQUEST_MULTIPLE_PATCHES_CODE;
use crate::tools::sequence_preflight::TASKSPACE_AFTER_BOUNDARY_REQUIRES_CONTROL_CODE;
use crate::tools::sequence_preflight::TASKSPACE_BOUNDARY_REQUIRES_ACTION_CODE;
use crate::tools::sequence_preflight::TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE;
use crate::tools::sequence_preflight::TASKSPACE_CONTROL_BINDING_FORBIDDEN_CODE;
use crate::tools::sequence_preflight::TASKSPACE_TOOL_SHAPE_UNSUPPORTED_CODE;
use crate::tools::sequence_preflight::validate_tool_sequence;
use codex_protocol::models::ResponseItem;
use codex_tools::ToolName;

fn function_call(name: &str, call_id: &str) -> ToolCall {
    function_call_with_arguments(name, call_id, "{}")
}

fn function_call_with_arguments(name: &str, call_id: &str, arguments: &str) -> ToolCall {
    ToolCall {
        tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Function {
            arguments: arguments.to_string(),
        },
        taskspace_binding: None,
    }
}

fn bound_call(name: &str, call_id: &str, binding: &str) -> ToolCall {
    let mut call = function_call(name, call_id);
    call.taskspace_binding = Some(binding.into());
    call
}

fn boundary_control(action: &str, call_id: &str) -> ToolCall {
    let arguments = match action {
        "initialize_map" => {
            r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Work"},"finish_identity":{"id":"finish"},"additional_work_nodes":[],"edges":[]}"#
        }
        "bind_node" => r#"{"action":"bind_node","expected_revision":2,"node_id":"work"}"#,
        "complete_then_continue" => {
            r#"{"action":"complete_then_continue","expected_revision":2,"current_node_id":"work","next_node_id":"verify"}"#
        }
        other => panic!("unsupported boundary action fixture: {other}"),
    };
    function_call_with_arguments("taskspace_control", call_id, arguments)
}

#[test]
fn preserves_provider_order_around_state_barriers() {
    let calls = vec![
        function_call("read_file", "read-1"),
        function_call("read_file", "read-2"),
        function_call("taskspace_control", "finish"),
        function_call("apply_patch", "edit"),
        function_call("taskspace_control", "finish-2"),
        function_call("exec_command", "test"),
    ];

    assert_eq!(
        sequence_segments(&calls),
        vec![
            SequenceSegment::Parallel { start: 0, end: 2 },
            SequenceSegment::Barrier {
                index: 2,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Barrier {
                index: 3,
                kind: BarrierKind::ApplyPatch,
            },
            SequenceSegment::Barrier {
                index: 4,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Parallel { start: 5, end: 6 },
        ]
    );
}

#[test]
fn leaves_ordinary_only_response_as_one_parallel_segment() {
    let calls = vec![
        function_call("read_file", "read-1"),
        function_call("exec_command", "read-2"),
    ];
    assert_eq!(
        sequence_segments(&calls),
        vec![SequenceSegment::Parallel { start: 0, end: 2 }]
    );
}

#[test]
fn tool_sequence_identity_is_stable_and_order_sensitive() {
    let first = vec![
        bound_call("read_file", "read-1", "active"),
        bound_call("exec_command", "test-1", "active"),
    ];
    let reversed = vec![first[1].clone(), first[0].clone()];

    assert_eq!(tool_sequence_sha256(&first), tool_sequence_sha256(&first));
    assert_ne!(
        tool_sequence_sha256(&first),
        tool_sequence_sha256(&reversed)
    );
    assert_eq!(tool_sequence_call_ids(&first), "read-1,test-1");
}

#[test]
fn provider_build_failure_closes_all_pairings_before_factual_feedback() {
    let declarations = vec![
        ProviderToolDeclaration::ready(function_call("read_file", "ready-prefix")),
        ProviderToolDeclaration::build_failed(
            &ResponseItem::ToolSearchCall {
                id: None,
                call_id: Some("malformed-search".to_string()),
                status: Some("completed".to_string()),
                execution: "client".to_string(),
                arguments: serde_json::json!({"query": 7}),
            },
            "failed to parse tool_search arguments: query must be a string",
        ),
    ];

    let outcome = invalid_provider_declaration_outcome(&declarations, Some(11));

    assert_eq!(outcome.outputs.len(), 3);
    assert!(matches!(
        &outcome.outputs[0],
        ResponseInputItem::FunctionCallOutput { call_id, output }
            if call_id == "ready-prefix" && output.success == Some(false)
    ));
    assert!(matches!(
        &outcome.outputs[1],
        ResponseInputItem::ToolSearchOutput { call_id, status, tools, .. }
            if call_id == "malformed-search" && status == "completed" && tools.is_empty()
    ));
    let ResponseInputItem::Message { content, .. } = &outcome.outputs[2] else {
        panic!("response-level factual feedback must follow all call pairings");
    };
    let text = content
        .iter()
        .find_map(|item| match item {
            codex_protocol::models::ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .expect("factual failure payload");
    assert!(text.contains("ProviderToolResponsePreflightV1"));
    assert!(text.contains("malformed-search"));
    assert!(text.contains("query must be a string"));
    assert!(text.contains(r#""executed_tool_call_count":0"#));
}

#[test]
fn skipped_output_preserves_call_id_and_failure_status() {
    let call = function_call("apply_patch", "edit-call");
    let output = ToolCallRuntime::skipped_responses(&call, "finish-call").remove(0);
    assert_eq!(response_input_call_id(&output), "edit-call");
    assert!(!response_input_succeeded(&output));
    let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
        panic!("expected function output");
    };
    assert!(
        output
            .body
            .to_text()
            .is_some_and(|text| text.contains("skipped_due_to_prior_failure"))
    );
}

#[test]
fn tool_search_failure_stops_later_segments_despite_completed_pairing_status() {
    let execution = ToolCallExecution {
        response: ResponseInputItem::ToolSearchOutput {
            call_id: "search-failed".into(),
            status: "completed".into(),
            execution: "client".into(),
            tools: Vec::new(),
        },
        supplemental_responses: Vec::new(),
        succeeded: false,
        taskspace_terminal_carrier: None,
    };

    assert_eq!(execution_failure_call_id(&execution), Some("search-failed"));
    assert!(
        response_input_succeeded(&execution.response),
        "provider pairing status remains completed independently of execution success"
    );
}

#[test]
fn tool_pairing_outputs_precede_supplemental_failure_facts() {
    let mut pairing = Vec::new();
    let mut supplemental = Vec::new();
    let search = ToolCall {
        tool_name: ToolName::plain("tool_search"),
        call_id: "search-failed".into(),
        payload: ToolPayload::ToolSearch {
            arguments: codex_protocol::models::SearchToolCallParams {
                query: String::new(),
                limit: None,
            },
        },
        taskspace_binding: Some("active".into()),
    };

    append_pairing_and_supplemental(
        &mut pairing,
        &mut supplemental,
        ToolCallRuntime::skipped_responses(&search, "prior-call"),
    );
    append_pairing_and_supplemental(
        &mut pairing,
        &mut supplemental,
        ToolCallRuntime::skipped_responses(
            &function_call("taskspace_control", "control-skipped"),
            "prior-call",
        ),
    );
    pairing.extend(supplemental);

    assert!(matches!(
        pairing[0],
        ResponseInputItem::ToolSearchOutput { .. }
    ));
    assert!(matches!(
        pairing[1],
        ResponseInputItem::FunctionCallOutput { .. }
    ));
    assert!(matches!(pairing[2], ResponseInputItem::Message { .. }));
}

#[test]
fn invalid_taskspace_control_arguments_reject_the_complete_response() {
    let calls = [
        bound_call("exec_command", "inspect", "active"),
        function_call_with_arguments("taskspace_control", "invalid-bootstrap", r#"{"action":7}"#),
    ];

    let failure = validate_tool_sequence(&calls, true)
        .expect_err("mechanically invalid control must fail before execution");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE
    );
    let outputs = failure.outputs(&calls, Some(7));
    assert_eq!(outputs.len(), calls.len());
    for output in outputs {
        let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
            panic!("expected function output");
        };
        let value: serde_json::Value =
            serde_json::from_str(output.body.to_text().as_deref().expect("preflight payload"))
                .expect("preflight json");
        assert_eq!(value["request"]["executed_tool_call_count"], 0);
        assert_eq!(value["state_commit"], false);
        assert_eq!(value["canonical_revision"], 7);
    }
}

#[test]
fn taskspace_control_binding_is_rejected_by_response_preflight() {
    let mut control =
        function_call_with_arguments("taskspace_control", "read-map", r#"{"action":"read_map"}"#);
    control.taskspace_binding = Some("active".into());
    let calls = [bound_call("exec_command", "inspect", "active"), control];

    let failure =
        validate_tool_sequence(&calls, true).expect_err("control binding must fail preflight");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_CONTROL_BINDING_FORBIDDEN_CODE
    );
}

#[test]
fn unsupported_provider_payload_rejects_the_complete_response() {
    let calls = vec![
        bound_call("ordinary", "ordinary-call", "active"),
        ToolCall {
            tool_name: ToolName::plain("unknown_custom"),
            call_id: "custom-call".into(),
            payload: ToolPayload::Custom {
                input: "raw".into(),
            },
            taskspace_binding: None,
        },
    ];

    let failure = validate_tool_sequence(&calls, true).expect_err("custom payload must fail");
    assert_eq!(failure.reason_code, TASKSPACE_TOOL_SHAPE_UNSUPPORTED_CODE);
    let outputs = failure.outputs(&calls, Some(11));
    assert_eq!(outputs.len(), 2);
    for output in outputs {
        assert!(!response_input_succeeded(&output));
        let text = match output {
            ResponseInputItem::FunctionCallOutput { output, .. }
            | ResponseInputItem::CustomToolCallOutput { output, .. } => {
                output.body.to_text().expect("preflight text")
            }
            other => panic!("expected function-compatible output, got {other:?}"),
        };
        assert!(text.contains("\"executed_tool_call_count\":0"));
        assert!(text.contains("\"state_commit\":false"));
    }
}

#[test]
fn preflight_keeps_all_call_pairings_before_tool_search_failure_facts() {
    let search = ToolCall {
        tool_name: ToolName::plain("tool_search"),
        call_id: "search-invalid".into(),
        payload: ToolPayload::ToolSearch {
            arguments: codex_protocol::models::SearchToolCallParams {
                query: "tools".into(),
                limit: None,
            },
        },
        taskspace_binding: Some("active".into()),
    };
    let calls = vec![
        search,
        function_call_with_arguments("taskspace_control", "control-invalid", r#"{"action":7}"#),
    ];

    let outputs = validate_tool_sequence(&calls, true)
        .expect_err("invalid control must reject the complete response")
        .outputs(&calls, None);

    assert!(matches!(
        outputs[0],
        ResponseInputItem::ToolSearchOutput { .. }
    ));
    assert!(matches!(
        outputs[1],
        ResponseInputItem::FunctionCallOutput { .. }
    ));
    assert!(matches!(outputs[2], ResponseInputItem::Message { .. }));
}

#[test]
fn multi_patch_preflight_closes_every_call_without_execution_claims() {
    let calls = vec![
        function_call("read_file", "read"),
        function_call("apply_patch", "patch-1"),
        function_call("apply_patch", "patch-2"),
    ];
    let failure = validate_tool_sequence(&calls, false).expect_err("two patches must fail");
    assert_eq!(failure.reason_code, REQUEST_MULTIPLE_PATCHES_CODE);

    let outputs = failure.outputs(&calls, None);
    assert_eq!(outputs.len(), calls.len());
    for (output, call) in outputs.iter().zip(&calls) {
        assert_eq!(response_input_call_id(output), call.call_id);
        assert!(!response_input_succeeded(output));
        let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
            panic!("expected function output");
        };
        let value: serde_json::Value =
            serde_json::from_str(output.body.to_text().as_deref().expect("preflight payload"))
                .expect("preflight json");
        assert_eq!(value["error"]["code"], REQUEST_MULTIPLE_PATCHES_CODE);
        assert_eq!(value["request"]["executed_tool_call_count"], 0);
        assert_eq!(value["request"]["patch_call_count"], 2);
    }
}

#[test]
fn one_patch_with_follow_up_tools_passes_standard_preflight() {
    let calls = vec![
        function_call("apply_patch", "patch"),
        function_call("exec_command", "test"),
    ];
    let manifest = validate_tool_sequence(&calls, false).expect("valid sequence");
    assert_eq!(manifest.request_patch_count, 1);
}

#[test]
fn boundary_and_bound_action_stay_in_one_valid_response() {
    let calls = vec![
        boundary_control("complete_then_continue", "handoff"),
        bound_call("apply_patch", "patch", "after_boundary"),
        bound_call("exec_command", "test", "active"),
        bound_call("read_file", "inspect", "active"),
    ];

    let manifest = validate_tool_sequence(&calls, true).expect("valid merged response");
    assert_eq!(manifest.entries.len(), 4);
    assert_eq!(manifest.request_patch_count, 1);
    assert_eq!(
        sequence_segments(&calls),
        vec![
            SequenceSegment::Barrier {
                index: 0,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Barrier {
                index: 1,
                kind: BarrierKind::ApplyPatch,
            },
            SequenceSegment::Parallel { start: 2, end: 4 },
        ]
    );
}

#[test]
fn multiple_boundary_pairs_preserve_declared_sequence() {
    let calls = vec![
        boundary_control("bind_node", "bind"),
        bound_call("read_file", "read", "after_boundary"),
        boundary_control("complete_then_continue", "handoff"),
        bound_call("exec_command", "test", "after_boundary"),
    ];

    validate_tool_sequence(&calls, true).expect("two valid boundary pairs");
    assert_eq!(
        sequence_segments(&calls),
        vec![
            SequenceSegment::Barrier {
                index: 0,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Parallel { start: 1, end: 2 },
            SequenceSegment::Barrier {
                index: 2,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Parallel { start: 3, end: 4 },
        ]
    );
}

#[test]
fn standalone_boundary_is_rejected_before_execution() {
    let calls = vec![boundary_control("complete_then_continue", "handoff")];
    let failure = validate_tool_sequence(&calls, true).expect_err("action pair is mandatory");
    assert_eq!(failure.reason_code, TASKSPACE_BOUNDARY_REQUIRES_ACTION_CODE);
    let output = failure.outputs(&calls, None).remove(0);
    let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
        panic!("expected function output");
    };
    let value: serde_json::Value =
        serde_json::from_str(output.body.to_text().as_deref().expect("preflight payload"))
            .expect("preflight json");
    assert_eq!(
        value["request"]["actual_sequence"][0],
        serde_json::json!({
            "tool": "taskspace_control",
            "control_action": "complete_then_continue",
            "taskspace_binding": null,
            "payload_kind": "function",
        })
    );
    assert_eq!(
        value["request"]["expected_sequence"]["immediately_after_boundary"],
        serde_json::json!({
            "tool_kind": "ordinary_tool",
            "taskspace_binding": "after_boundary",
        })
    );
}

#[test]
fn orphan_after_boundary_is_rejected_before_execution() {
    let calls = vec![bound_call("exec_command", "test", "after_boundary")];
    let failure = validate_tool_sequence(&calls, true).expect_err("boundary is mandatory");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_AFTER_BOUNDARY_REQUIRES_CONTROL_CODE
    );
}

#[test]
fn active_binding_preserves_parallel_segments() {
    let calls = vec![
        bound_call("read_file", "read-1", "active"),
        bound_call("exec_command", "read-2", "active"),
    ];

    validate_tool_sequence(&calls, true).expect("active binding sequence");
    assert_eq!(
        sequence_segments(&calls),
        vec![SequenceSegment::Parallel { start: 0, end: 2 }]
    );
}
