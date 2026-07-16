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
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Solve"},"initial_work_node":{"node_id":"node-1","goal":"Read"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[{"from":"root","to":"node-1"},{"from":"node-1","to":"finish"}],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    );

    let actions = taskspace_nested_actions(&call);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].tool_name(), "exec_command");
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
fn taskspace_patch_slot_and_top_level_patch_share_the_same_preflight_count() {
    let bootstrap = function_call_with_arguments(
        "taskspace_control",
        "bootstrap",
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Solve"},"initial_work_node":{"node_id":"edit","goal":"Edit"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[{"from":"root","to":"edit"},{"from":"edit","to":"finish"}],"continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch"}}}"#,
    );
    let calls = vec![bootstrap, function_call("apply_patch", "top-patch")];
    let manifest = ToolSequenceManifest::from_calls(&calls);
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
fn aggregate_does_not_duplicate_independently_visible_nested_tool_feedback() {
    let state = ResponseInputItem::FunctionCallOutput {
        call_id: "outer".into(),
        output: FunctionCallOutputPayload::from_text(
            serde_json::json!({
                "schema_version": "TaskSpaceControlResultR6V1",
                "status": "committed",
                "success": true,
                "state_commit": true,
                "committed_revision": 1,
                "delta": {
                    "map_id": "map-1",
                    "committed_revision": 1,
                    "graph_event_refs": [
                        {"revision": 1, "event_id": "event:5:map-1:1:0", "event_type": "map_initialized"},
                        {"revision": 1, "event_id": "event:5:map-1:1:1", "event_type": "readiness_changed"},
                        {"revision": 2, "event_id": "event:5:map-1:2:0", "event_type": "node_bound"}
                    ],
                    "node_detail_event_refs": []
                },
                "steps": [{
                    "kind": "map_initialized",
                    "map_id": "map-1",
                    "revision": 1
                }],
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
    )
    .expect("R6 batch response");
    let ResponseInputItem::FunctionCallOutput { call_id, output } = aggregated else {
        panic!("expected outer function output");
    };
    assert_eq!(call_id, "outer");
    assert_eq!(output.success, Some(true));
    let output_text = output.body.to_text().expect("text");
    let value: serde_json::Value = serde_json::from_str(&output_text).expect("batch json");
    assert_eq!(value["schema_version"], "TaskSpaceControlResultR6V1");
    assert_eq!(value["status"], "committed");
    assert_eq!(value["state_commit"], true);
    assert_eq!(value["steps"].as_array().expect("steps").len(), 1);
    assert_eq!(value["steps"][0]["map_id"], "map-1");
    assert!(!output_text.contains("task-event-7"));
    assert!(!output_text.contains("task-event-8"));
    assert!(
        output_text.len() <= 712,
        "initialization feedback with nested actions must stay at least 30% below the 1,018-byte E6 fixture, got {} bytes",
        output_text.len()
    );
}

#[test]
fn aggregate_rejects_non_r6_state_feedback_instead_of_rewriting_it() {
    let state = ResponseInputItem::FunctionCallOutput {
        call_id: "outer".into(),
        output: FunctionCallOutputPayload::from_text(
            serde_json::json!({
                "schema_version": "TaskSpaceControlResultV3",
                "success": true,
                "steps": [],
            })
            .to_string(),
        ),
    };

    let error = aggregate_taskspace_batch_response("outer", state, Vec::new(), true)
        .expect_err("legacy feedback must be fatal");

    assert!(
        error
            .to_string()
            .contains("unsupported state response schema")
    );
}
