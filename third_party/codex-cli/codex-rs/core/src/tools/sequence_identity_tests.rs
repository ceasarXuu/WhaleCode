use codex_protocol::models::ResponseInputItem;
use codex_tools::ToolName;

use super::validate_tool_sequence;
use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;

fn function_call(name: &str, call_id: &str, arguments: impl Into<String>) -> ToolCall {
    ToolCall {
        tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Function {
            arguments: arguments.into(),
        },
    }
}

fn control(action: &str, actions: serde_json::Value) -> ToolCall {
    let arguments = match action {
        "initialize_and_execute" => serde_json::json!({
            "action": action,
            "root": {"node_id": "root", "goal": "Complete the task"},
            "work_nodes": [
                {"node_id": "inspect", "goal": "Inspect"},
                {"node_id": "verify", "goal": "Verify"}
            ],
            "finish": {"node_id": "finish", "goal": "Finish"},
            "edges": [
                {"from": "root", "to": "inspect"},
                {"from": "root", "to": "verify"},
                {"from": "inspect", "to": "finish"},
                {"from": "verify", "to": "finish"}
            ],
            "actions": actions,
        }),
        "execute" => serde_json::json!({
            "action": action,
            "expected_revision": 7,
            "mutations": [],
            "actions": actions,
        }),
        "reopen_map" => serde_json::json!({
            "action": action,
            "expected_revision": 9,
            "work_nodes": [
                {"node_id": "inspect", "goal": "Inspect"},
                {"node_id": "verify", "goal": "Verify"}
            ],
            "edges": [
                {"from": "root", "to": "inspect"},
                {"from": "root", "to": "verify"},
                {"from": "inspect", "to": "finish"},
                {"from": "verify", "to": "finish"}
            ],
            "actions": actions,
        }),
        other => panic!("unsupported test action {other}"),
    };
    function_call("taskspace_control", "control", arguments.to_string())
}

fn assert_zero_commit_failure(
    failure: &crate::tools::sequence_preflight::ToolSequencePreflightFailure,
    calls: &[ToolCall],
) {
    let outputs = failure.outputs(calls, Some(17));
    assert_eq!(outputs.len(), calls.len());
    for output in outputs {
        let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
            panic!("test calls should receive function outputs");
        };
        let value: serde_json::Value =
            serde_json::from_str(output.body.to_text().as_deref().expect("failure body"))
                .expect("failure json");
        assert_eq!(value["state_commit"], false);
        assert_eq!(value["request"]["executed_tool_call_count"], 0);
        assert_eq!(
            value["error"]["code"], failure.reason_code,
            "feedback must preserve the exact mechanical reason"
        );
    }
}

#[test]
fn taskspace_preflight_rejects_empty_sibling_call_id_for_every_prepared_action() {
    for action in ["initialize_and_execute", "execute", "reopen_map"] {
        let calls = vec![
            control(
                action,
                serde_json::json!([{"node_id": "inspect", "tool": "read_file"}]),
            ),
            function_call("read_file", "", "{}"),
        ];

        let failure =
            validate_tool_sequence(&calls, true).expect_err("empty call_id must fail preflight");
        assert_eq!(
            failure.reason_code, "taskspace_empty_call_id",
            "wrong reason for {action}"
        );
        assert_zero_commit_failure(&failure, &calls);
    }
}

#[test]
fn taskspace_preflight_rejects_duplicate_call_id_for_every_prepared_action() {
    for action in ["initialize_and_execute", "execute", "reopen_map"] {
        let calls = vec![
            control(
                action,
                serde_json::json!([
                    {"node_id": "inspect", "tool": "read_file"},
                    {"node_id": "verify", "tool": "exec_command"}
                ]),
            ),
            function_call("read_file", "duplicate", "{}"),
            function_call("exec_command", "duplicate", r#"{"cmd":"true"}"#),
        ];

        let failure = validate_tool_sequence(&calls, true)
            .expect_err("duplicate call_id must fail preflight");
        assert_eq!(
            failure.reason_code, "taskspace_duplicate_call_id",
            "wrong reason for {action}"
        );
        assert_zero_commit_failure(&failure, &calls);
    }
}

#[test]
fn taskspace_preflight_rejects_control_and_sibling_with_same_call_id() {
    let calls = vec![
        function_call(
            "taskspace_control",
            "duplicate",
            serde_json::json!({
                "action": "execute",
                "expected_revision": 7,
                "mutations": [],
                "actions": [{"node_id": "inspect", "tool": "read_file"}],
            })
            .to_string(),
        ),
        function_call("read_file", "duplicate", "{}"),
    ];

    let failure =
        validate_tool_sequence(&calls, true).expect_err("response-wide duplicate must fail");
    assert_eq!(failure.reason_code, "taskspace_duplicate_call_id");
    assert_zero_commit_failure(&failure, &calls);
}
