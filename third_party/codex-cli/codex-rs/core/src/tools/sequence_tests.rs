use super::*;
use crate::action_map::ActionMapActionReservation;
use crate::action_map::ActionMapExecuteTransaction;
use crate::action_map::ActionMapInitialize;
use crate::action_map::ActionMapReservationInput;
use crate::action_map::ActionMapReservationRelease;
use crate::action_map::ActionMapResponsePrepareError;
use crate::action_map::ActionMapStateRejection;
use crate::action_map::ActionMapViolationCode;
use crate::action_map::CompletionRecord;
use crate::action_map::MapEdge;
use crate::action_map::NodeMutation;
use crate::action_map::action_map_node;
use crate::action_map::execute_action_map_transaction;
use crate::action_map::initialize_action_map;
use crate::action_map::release_action_map_reservation;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallExecution;
use crate::tools::provider_tool_declaration::ProviderToolDeclaration;
use crate::tools::sequence_preflight::REQUEST_MULTIPLE_PATCHES_CODE;
use crate::tools::sequence_preflight::TASKSPACE_ACTION_COUNT_MISMATCH_CODE;
use crate::tools::sequence_preflight::TASKSPACE_ACTION_TOOL_MISMATCH_CODE;
use crate::tools::sequence_preflight::TASKSPACE_CONTROL_MUST_BE_FIRST_CODE;
use crate::tools::sequence_preflight::TASKSPACE_CONTROL_ONLY_ACTION_HAS_SIBLINGS_CODE;
use crate::tools::sequence_preflight::TASKSPACE_CONTROL_REQUIRED_CODE;
use crate::tools::sequence_preflight::ToolSequencePlan;
use crate::tools::sequence_preflight::validate_tool_sequence;
use codex_protocol::models::ResponseItem;
use codex_tools::ToolName;

fn function_call(name: &str, call_id: &str) -> ToolCall {
    function_call_with_arguments(name, call_id, "{}")
}

fn function_call_with_arguments(name: &str, call_id: &str, arguments: &str) -> ToolCall {
    ToolCall {
        provider_tool_name: ToolName::plain(name),
        dispatch_tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Function {
            arguments: arguments.to_string(),
        },
    }
}

fn tool_search_call(call_id: &str, query: &str) -> ToolCall {
    ToolCall {
        provider_tool_name: ToolName::plain("tool_search"),
        dispatch_tool_name: ToolName::plain("tool_search"),
        call_id: call_id.to_string(),
        payload: ToolPayload::ToolSearch {
            arguments: codex_protocol::models::SearchToolCallParams {
                query: query.to_string(),
                limit: None,
            },
        },
    }
}

fn execute_control(call_id: &str, actions: serde_json::Value) -> ToolCall {
    function_call_with_arguments(
        "taskspace_control",
        call_id,
        &serde_json::json!({
            "action": "execute",
            "expected_revision": 7,
            "mutations": [],
            "actions": actions,
        })
        .to_string(),
    )
}

fn initialize_and_execute_control(call_id: &str, actions: serde_json::Value) -> ToolCall {
    function_call_with_arguments(
        "taskspace_control",
        call_id,
        &serde_json::json!({
            "action": "initialize_and_execute",
            "root": {"node_id": "root", "goal": "Start"},
            "work_nodes": [{"node_id": "inspect", "goal": "Inspect"}],
            "finish": {"node_id": "finish", "goal": "Finish"},
            "edges": [
                {"from": "root", "to": "inspect"},
                {"from": "inspect", "to": "finish"}
            ],
            "actions": actions,
        })
        .to_string(),
    )
}

fn reopen_control(call_id: &str, actions: serde_json::Value) -> ToolCall {
    function_call_with_arguments(
        "taskspace_control",
        call_id,
        &serde_json::json!({
            "action": "reopen_map",
            "expected_revision": 9,
            "work_nodes": [{"node_id": "address-feedback", "goal": "Address feedback"}],
            "edges": [
                {"from": "root", "to": "address-feedback"},
                {"from": "address-feedback", "to": "finish"}
            ],
            "actions": actions,
        })
        .to_string(),
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
fn taskspace_execute_plan_declares_sibling_native_calls_without_payload_decoration() {
    let calls = vec![
        initialize_and_execute_control(
            "control",
            serde_json::json!([
                {"node_id": "inspect", "tool": "read_file"},
                {"node_id": "inspect", "tool": "exec_command"}
            ]),
        ),
        function_call_with_arguments("read_file", "read", r#"{"path":"README.md"}"#),
        function_call_with_arguments("exec_command", "test", r#"{"cmd":"cargo test"}"#),
    ];

    let (manifest, plan) = validate_tool_sequence(&calls, true).expect("valid TaskSpace execute");
    assert_eq!(manifest.entries.len(), 3);
    let ToolSequencePlan::TaskSpaceExecute {
        control_index,
        declared_calls,
        ..
    } = plan
    else {
        panic!("expected TaskSpaceExecute plan");
    };
    assert_eq!(control_index, 0);
    assert_eq!(declared_calls.len(), 2);
    assert_eq!(declared_calls[0].call_id, "read");
    assert_eq!(declared_calls[0].call_index, 0);
    assert_eq!(declared_calls[0].node_id, "inspect");
    assert_eq!(declared_calls[0].tool_name, "read_file");
    let ToolPayload::Function { arguments } = &calls[1].payload else {
        panic!("expected native function payload");
    };
    assert_eq!(arguments, r#"{"path":"README.md"}"#);
}

#[test]
fn taskspace_execute_requires_action_count_to_match_sibling_calls() {
    let calls = vec![
        execute_control(
            "control",
            serde_json::json!([{"node_id": "inspect", "tool": "read_file"}]),
        ),
        function_call("read_file", "read"),
        function_call("exec_command", "test"),
    ];

    let failure = validate_tool_sequence(&calls, true).expect_err("count mismatch must fail");
    assert_eq!(failure.reason_code, TASKSPACE_ACTION_COUNT_MISMATCH_CODE);
    assert_eq!(failure.outputs(&calls, Some(7)).len(), calls.len() + 1);
}

#[test]
fn taskspace_execute_requires_action_tool_to_match_sibling_call() {
    let calls = vec![
        execute_control(
            "control",
            serde_json::json!([{"node_id": "inspect", "tool": "exec_command"}]),
        ),
        function_call("read_file", "read"),
    ];

    let failure = validate_tool_sequence(&calls, true).expect_err("tool mismatch must fail");
    assert_eq!(failure.reason_code, TASKSPACE_ACTION_TOOL_MISMATCH_CODE);
}

#[test]
fn reopen_map_is_a_prepared_response_with_matching_native_siblings() {
    let calls = vec![
        reopen_control(
            "reopen",
            serde_json::json!([
                {"node_id": "address-feedback", "tool": "read_file"}
            ]),
        ),
        function_call_with_arguments("read_file", "read", r#"{"path":"README.md"}"#),
    ];

    let (_, plan) = validate_tool_sequence(&calls, true).expect("valid reopen response");
    let ToolSequencePlan::TaskSpaceExecute {
        args,
        declared_calls,
        ..
    } = plan
    else {
        panic!("expected prepared reopen response");
    };
    assert_eq!(args.action_name(), "reopen_map");
    assert_eq!(declared_calls.len(), 1);
    assert_eq!(declared_calls[0].node_id, "address-feedback");
}

#[test]
fn reopen_map_cannot_be_a_standalone_control_call() {
    let calls = vec![reopen_control(
        "reopen",
        serde_json::json!([
            {"node_id": "address-feedback", "tool": "read_file"}
        ]),
    )];

    let failure = validate_tool_sequence(&calls, true).expect_err("reopen needs sibling calls");
    assert_eq!(failure.reason_code, TASKSPACE_ACTION_COUNT_MISMATCH_CODE);
}

#[test]
fn taskspace_control_only_plan_rejects_sibling_native_calls() {
    let calls = vec![
        function_call_with_arguments("taskspace_control", "read-map", r#"{"action":"read_map"}"#),
        function_call("read_file", "read"),
    ];

    let failure = validate_tool_sequence(&calls, true).expect_err("control-only has sibling");
    assert_eq!(
        failure.reason_code,
        TASKSPACE_CONTROL_ONLY_ACTION_HAS_SIBLINGS_CODE
    );
}

#[test]
fn taskspace_control_only_plan_stays_with_control_handler() {
    let calls = vec![function_call_with_arguments(
        "taskspace_control",
        "read-map",
        r#"{"action":"read_map"}"#,
    )];

    let (_, plan) = validate_tool_sequence(&calls, true).expect("valid control-only");
    let ToolSequencePlan::TaskSpaceControlOnly = plan else {
        panic!("expected control-only plan");
    };
}

#[test]
fn taskspace_response_requires_control_manifest_first() {
    let calls = vec![
        function_call("read_file", "read"),
        execute_control("control", serde_json::json!([])),
    ];

    let failure = validate_tool_sequence(&calls, true).expect_err("control must be first");
    assert_eq!(failure.reason_code, TASKSPACE_CONTROL_MUST_BE_FIRST_CODE);
}

#[test]
fn taskspace_response_requires_control_manifest() {
    let calls = vec![function_call("read_file", "read")];

    let failure = validate_tool_sequence(&calls, true).expect_err("control is mandatory");
    assert_eq!(failure.reason_code, TASKSPACE_CONTROL_REQUIRED_CODE);
}

#[test]
fn tool_sequence_identity_is_stable_and_order_sensitive() {
    let first = vec![
        function_call("read_file", "read-1"),
        function_call("exec_command", "test-1"),
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
fn tool_sequence_identity_uses_provider_names_before_shared_dispatch_aliases() {
    let exec = ToolCall {
        provider_tool_name: ToolName::plain("exec_command"),
        dispatch_tool_name: ToolName::plain("shell_command"),
        call_id: "same-call".to_string(),
        payload: ToolPayload::Function {
            arguments: r#"{"command":"true"}"#.to_string(),
        },
    };
    let read = ToolCall {
        provider_tool_name: ToolName::plain("read_file"),
        dispatch_tool_name: ToolName::plain("shell_command"),
        call_id: "same-call".to_string(),
        payload: ToolPayload::Function {
            arguments: r#"{"command":"cat README.md"}"#.to_string(),
        },
    };

    assert_ne!(tool_sequence_sha256(&[exec]), tool_sequence_sha256(&[read]));
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
    assert!(text.contains("ProviderToolResponsePreflightV2"));
    assert!(text.contains(r#""copy_group_id":"provider_response:ready-prefix""#));
    assert!(text.contains("malformed-search"));
    assert!(text.contains("query must be a string"));
    assert!(text.contains(r#""executed_tool_call_count":0"#));
}

#[test]
fn taskspace_state_commit_failure_closes_every_call_without_dispatch() {
    let calls = vec![
        execute_control(
            "control",
            serde_json::json!([{"node_id": "inspect", "tool": "read_file"}]),
        ),
        function_call("read_file", "read"),
    ];

    let error = ActionMapResponsePrepareError::state(ActionMapStateRejection::one(
        9,
        ActionMapViolationCode::StaleRevision,
        "expected_revision",
    ));
    let outcome = taskspace_prepare_failure_outcome(&calls, Some(9), &error);

    assert_eq!(outcome.outputs.len(), calls.len() + 1);
    assert!(outcome.terminal_completion.is_none());
    for (output, call) in outcome.outputs.iter().take(calls.len()).zip(&calls) {
        assert_eq!(response_input_call_id(output), call.call_id);
        assert!(!response_input_succeeded(output));
        let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
            panic!("expected function output");
        };
        let value: serde_json::Value =
            serde_json::from_str(output.body.to_text().as_deref().expect("failure payload"))
                .expect("failure json");
        assert_eq!(
            value["error"]["code"],
            ACTION_MAP_RESPONSE_STATE_COMMIT_FAILED_CODE
        );
        assert_eq!(value["executed_tool_call_count"], 0);
        assert_eq!(value["state_commit"], false);
        assert_eq!(value["canonical_revision"], 9);
        assert_eq!(value["current_revision"], 9);
        assert_eq!(value["error"]["violations"][0]["code"], "stale_revision");
        assert!(value["error"].get("detail").is_none());
    }
    assert!(matches!(
        outcome.outputs.last(),
        Some(ResponseInputItem::Message { .. })
    ));
}

#[test]
fn taskspace_complete_then_reserve_rejection_closes_tool_search_without_dispatch() {
    let calls = vec![
        execute_control(
            "control",
            serde_json::json!([{"node_id": "verify", "tool": "tool_search"}]),
        ),
        tool_search_call("search", "read_file"),
    ];
    let initial_reservation = "initial-reservation".to_string();
    let initialized = initialize_action_map(ActionMapInitialize {
        map_id: "state-tool-search".into(),
        root: action_map_node("root", "solve", Vec::new()),
        work_nodes: vec![action_map_node("verify", "verify", Vec::new())],
        finish: action_map_node("finish", "finish", Vec::new()),
        edges: vec![
            MapEdge {
                from: "root".into(),
                to: "verify".into(),
            },
            MapEdge {
                from: "verify".into(),
                to: "finish".into(),
            },
        ],
        reservations: vec![ActionMapReservationInput {
            reservation_id: initial_reservation.clone(),
            reservation: ActionMapActionReservation {
                action_id: "initial-action".into(),
                node_id: "verify".into(),
                tool_name: "read_file".into(),
                response_call_index: 0,
            },
        }],
    })
    .expect("initialize map")
    .map;
    let canonical = release_action_map_reservation(
        &initialized,
        ActionMapReservationRelease {
            expected_revision: initialized.revision,
            reservation_id: initial_reservation,
            result_refs: Vec::new(),
            evidence_refs: Vec::new(),
        },
    )
    .expect("release initial reservation")
    .map;
    let rejection = execute_action_map_transaction(
        &canonical,
        ActionMapExecuteTransaction {
            expected_revision: canonical.revision,
            graph: Default::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "verify".into(),
                record: CompletionRecord {
                    action_id: "complete-verify".into(),
                    result_ref_ids: Vec::new(),
                    evidence_ref_ids: Vec::new(),
                },
            }],
            reservations: vec![ActionMapReservationInput {
                reservation_id: "search-reservation".into(),
                reservation: ActionMapActionReservation {
                    action_id: "search-action".into(),
                    node_id: "verify".into(),
                    tool_name: "tool_search".into(),
                    response_call_index: 0,
                },
            }],
        },
    )
    .expect_err("completing and reserving the same node must be rejected");
    assert_eq!(
        rejection.violations[0].code,
        ActionMapViolationCode::NodeStateInvalid
    );
    let error = ActionMapResponsePrepareError::state(rejection);

    let outcome = taskspace_prepare_failure_outcome(&calls, Some(canonical.revision), &error);

    assert_eq!(outcome.outputs.len(), calls.len() + 1);
    assert!(outcome.terminal_completion.is_none());
    let ResponseInputItem::FunctionCallOutput { call_id, output } = &outcome.outputs[0] else {
        panic!("control call must receive its native pairing");
    };
    assert_eq!(call_id, "control");
    assert_eq!(output.success, Some(false));
    let control_failure: serde_json::Value =
        serde_json::from_str(output.body.to_text().as_deref().expect("failure payload"))
            .expect("failure JSON");
    assert_eq!(control_failure["state_commit"], false);
    assert_eq!(control_failure["rejected_candidate_committed"], false);
    assert_eq!(control_failure["executed_tool_call_count"], 0);
    assert_eq!(
        control_failure["failure_provenance"]["affected_call_ids"],
        serde_json::json!(["control", "search"])
    );
    let violation = &control_failure["error"]["violations"][0];
    assert_eq!(violation["canonical_before_transaction"]["state"], "ready");
    assert_eq!(
        violation["rejected_candidate_at_violation"]["state"],
        "completed"
    );
    assert_eq!(
        violation["rejected_candidate_at_violation"]["allowed_states"],
        serde_json::json!(["ready", "in_flight"])
    );

    assert!(matches!(
        &outcome.outputs[1],
        ResponseInputItem::ToolSearchOutput {
            call_id,
            status,
            tools,
            ..
        } if call_id == "search" && status == "completed" && tools.is_empty()
    ));
    let ResponseInputItem::Message { role, content } = &outcome.outputs[2] else {
        panic!("response-level failure fact must follow both pairings");
    };
    assert_eq!(role, "developer");
    let fact = content
        .iter()
        .find_map(|item| match item {
            codex_protocol::models::ContentItem::InputText { text } => Some(text),
            _ => None,
        })
        .expect("response-level failure fact");
    let fact: serde_json::Value = serde_json::from_str(fact).expect("factual failure JSON");
    assert_eq!(
        fact["failure_provenance"]["affected_call_ids"],
        serde_json::json!(["control", "search"])
    );
    assert_eq!(fact["executed_tool_call_count"], 0);
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
        provider_tool_name: ToolName::plain("tool_search"),
        dispatch_tool_name: ToolName::plain("tool_search"),
        call_id: "search-failed".into(),
        payload: ToolPayload::ToolSearch {
            arguments: codex_protocol::models::SearchToolCallParams {
                query: String::new(),
                limit: None,
            },
        },
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
fn multi_patch_preflight_closes_every_call_without_execution_claims() {
    let calls = vec![
        function_call("read_file", "read"),
        function_call("apply_patch", "patch-1"),
        function_call("apply_patch", "patch-2"),
    ];
    let failure = validate_tool_sequence(&calls, false).expect_err("two patches must fail");
    assert_eq!(failure.reason_code, REQUEST_MULTIPLE_PATCHES_CODE);

    let outputs = failure.outputs(&calls, None);
    assert_eq!(outputs.len(), calls.len() + 1);
    for (output, call) in outputs.iter().take(calls.len()).zip(&calls) {
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
    assert!(matches!(
        outputs.last(),
        Some(ResponseInputItem::Message { .. })
    ));
}

#[test]
fn one_patch_with_follow_up_tools_passes_standard_preflight() {
    let calls = vec![
        function_call("apply_patch", "patch"),
        function_call("exec_command", "test"),
    ];
    let (manifest, plan) = validate_tool_sequence(&calls, false).expect("valid sequence");
    assert_eq!(manifest.request_patch_count, 1);
    assert!(matches!(plan, ToolSequencePlan::Standard));
}
