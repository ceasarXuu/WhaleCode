use super::*;
use crate::tools::context::ToolPayload;
use crate::tools::sequence_preflight::REQUEST_MULTIPLE_PATCHES_CODE;
use crate::tools::sequence_preflight::TASKSPACE_REQUIRED_NEXT_CALL_MISSING_CODE;
use crate::tools::sequence_preflight::TASKSPACE_REQUIRED_ORDINARY_TOOL_INVALID_CODE;
use crate::tools::sequence_preflight::TASKSPACE_REQUIRED_PATCH_ARGUMENTS_INVALID_CODE;
use crate::tools::sequence_preflight::TASKSPACE_REQUIRED_PATCH_INVALID_CODE;
use crate::tools::sequence_preflight::validate_tool_sequence;
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
    }
}

fn custom_call(name: &str, call_id: &str, input: &str) -> ToolCall {
    ToolCall {
        tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Custom {
            input: input.to_string(),
        },
    }
}

fn handoff_call(call_id: &str, required_next_call: &str) -> ToolCall {
    function_call_with_arguments(
        "taskspace_control",
        call_id,
        &format!(
            r#"{{"action":"complete_then_continue","expected_revision":2,"current_node_id":"edit","next_node_id":"verify","required_next_call":"{required_next_call}"}}"#,
        ),
    )
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
fn preserves_adjacent_finish_barriers_before_follow_up_action() {
    let calls = vec![
        function_call("taskspace_control", "finish-1"),
        function_call("taskspace_control", "finish-2"),
        function_call("exec_command", "test"),
    ];
    assert_eq!(
        sequence_segments(&calls),
        vec![
            SequenceSegment::Barrier {
                index: 0,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Barrier {
                index: 1,
                kind: BarrierKind::TaskSpaceControl,
            },
            SequenceSegment::Parallel { start: 2, end: 3 },
        ]
    );
}

#[test]
fn skipped_output_preserves_call_id_and_failure_status() {
    let call = function_call("apply_patch", "edit-call");
    let output = ToolCallRuntime::skipped_response(&call, "finish-call");
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
fn invalid_taskspace_arguments_are_owned_by_the_tool_handler() {
    let call = function_call_with_arguments(
        "taskspace_control",
        "invalid-bootstrap",
        r#"{"action":"initialize_map"}"#,
    );

    let manifest = validate_tool_sequence(&[call]).expect("preflight must not own tool arguments");
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(manifest.request_patch_count, 0);
}

#[test]
fn multi_patch_preflight_closes_every_call_without_execution_claims() {
    let calls = vec![
        function_call("read_file", "read"),
        function_call("apply_patch", "patch-1"),
        function_call("apply_patch", "patch-2"),
    ];
    let failure = validate_tool_sequence(&calls).expect_err("two patches must fail");
    assert_eq!(failure.reason_code, REQUEST_MULTIPLE_PATCHES_CODE);

    let outputs = failure.outputs(&calls);
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
fn one_patch_with_follow_up_tools_passes_manifest_preflight() {
    let calls = vec![
        function_call("apply_patch", "patch"),
        function_call("exec_command", "test"),
    ];
    let manifest = validate_tool_sequence(&calls).expect("valid sequence");
    assert_eq!(manifest.request_patch_count, 1);
}

#[test]
fn declared_patch_and_follow_up_tools_stay_in_one_valid_response() {
    let calls = vec![
        handoff_call("handoff", "apply_patch"),
        custom_call("apply_patch", "patch", "*** Begin Patch\n*** End Patch"),
        function_call("exec_command", "test"),
        function_call("read_file", "inspect"),
    ];

    let manifest = validate_tool_sequence(&calls).expect("valid merged response");
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
fn declared_ordinary_tool_requirement_passes_in_the_same_response() {
    let calls = vec![
        handoff_call("handoff", "ordinary_tool"),
        function_call("exec_command", "test"),
        function_call("read_file", "inspect"),
    ];

    validate_tool_sequence(&calls).expect("valid ordinary sibling response");
}

#[test]
fn declared_next_call_requires_an_immediate_sibling() {
    let failure = validate_tool_sequence(&[handoff_call("handoff", "ordinary_tool")])
        .expect_err("missing sibling must fail");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_REQUIRED_NEXT_CALL_MISSING_CODE
    );
}

#[test]
fn declared_patch_rejects_a_non_patch_immediate_sibling() {
    let calls = vec![
        handoff_call("handoff", "apply_patch"),
        function_call("exec_command", "test"),
    ];
    let failure = validate_tool_sequence(&calls).expect_err("wrong sibling must fail");
    assert_eq!(failure.reason_code, TASKSPACE_REQUIRED_PATCH_INVALID_CODE);
}

#[test]
fn declared_patch_rejects_malformed_direct_arguments_before_execution() {
    let calls = vec![
        handoff_call("handoff", "apply_patch"),
        function_call_with_arguments("apply_patch", "patch", r#"{"input":"patch"}}"#),
    ];
    let failure = validate_tool_sequence(&calls).expect_err("malformed patch must fail");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_REQUIRED_PATCH_ARGUMENTS_INVALID_CODE
    );
    assert!(failure.outputs(&calls).iter().all(|output| {
        !response_input_succeeded(output)
            && matches!(
                output,
                ResponseInputItem::FunctionCallOutput { output, .. }
                    if output.body.to_text().is_some_and(|text| text.contains(
                        "\"executed_tool_call_count\":0"
                    ))
            )
    }));
}

#[test]
fn ordinary_tool_does_not_ambiguously_accept_apply_patch() {
    let calls = vec![
        handoff_call("handoff", "ordinary_tool"),
        custom_call("apply_patch", "patch", "*** Begin Patch\n*** End Patch"),
    ];
    let failure = validate_tool_sequence(&calls).expect_err("ambiguous patch sibling must fail");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_REQUIRED_ORDINARY_TOOL_INVALID_CODE
    );
}
