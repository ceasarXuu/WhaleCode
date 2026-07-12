use super::*;
use crate::tools::sequence_manifest::ToolSequenceManifest;
use crate::tools::sequence_preflight::REQUEST_MULTIPLE_PATCHES_CODE;
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
            SequenceSegment::Barrier { index: 2 },
            SequenceSegment::Parallel { start: 3, end: 4 },
            SequenceSegment::Barrier { index: 4 },
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
            SequenceSegment::Barrier { index: 0 },
            SequenceSegment::Barrier { index: 1 },
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
fn extracts_bootstrap_nested_actions() {
    let call = function_call_with_arguments(
        "taskspace_control",
        "outer",
        r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"inspect_code_context","goal":"Read"}],"current_node_id":"node-1","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    );

    let actions = taskspace_nested_actions(&call);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].tool_name(), "exec_command");
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
fn taskspace_patch_slot_and_top_level_patch_share_the_same_preflight_count() {
    let bootstrap = function_call_with_arguments(
        "taskspace_control",
        "bootstrap",
        r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"edit","kind":"implement_solution","goal":"Edit"}],"current_node_id":"edit","continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch"}}}"#,
    );
    let calls = vec![bootstrap, function_call("apply_patch", "top-patch")];
    let manifest = ToolSequenceManifest::from_calls(&calls).expect("manifest");
    assert_eq!(manifest.request_patch_count, 2);
    assert!(validate_tool_sequence(&calls).is_err());
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
fn aggregate_references_canonical_nested_events_without_copying_output() {
    let state = ResponseInputItem::FunctionCallOutput {
        call_id: "outer".into(),
        output: FunctionCallOutputPayload::from_text(
            serde_json::json!({
                "schema_version": "TaskSpaceControlResultV1",
                "success": true,
                "steps": [{"kind": "finish", "success": true}],
            })
            .to_string(),
        ),
    };
    let aggregated = aggregate_taskspace_batch_response(
        "outer",
        state,
        vec![(
            "apply_patch".into(),
            "outer:nested:0".into(),
            true,
            "task-event-7".into(),
            "task-event-8".into(),
        )],
        true,
    );
    let ResponseInputItem::FunctionCallOutput { call_id, output } = aggregated else {
        panic!("expected outer function output");
    };
    assert_eq!(call_id, "outer");
    assert_eq!(output.success, Some(true));
    let value: serde_json::Value =
        serde_json::from_str(&output.body.to_text().expect("text")).expect("batch json");
    assert_eq!(value["steps"].as_array().expect("steps").len(), 2);
    assert_eq!(value["steps"][1]["call_event_ref"], "task-event-7");
    assert_eq!(value["steps"][1]["output_event_ref"], "task-event-8");
    assert!(value["steps"][1].get("response").is_none());
}
