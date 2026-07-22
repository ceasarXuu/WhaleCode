use super::*;

#[test]
fn parses_all_carrier_transitions_without_sibling_metadata() {
    let initialize = parse_taskspace_transition_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Solve"},"initial_work_node":{"node_id":"inspect","goal":"Inspect"},"finish_identity":{"id":"finish"},"additional_work_nodes":[],"edges":[{"from":"root","to":"inspect"},{"from":"inspect","to":"finish"}]}"#,
    )
    .expect("initialize");
    assert_eq!(initialize.action_name(), "initialize_map");

    let bind = parse_taskspace_transition_args(
        r#"{"action":"bind_node","expected_revision":2,"node_id":"verify"}"#,
    )
    .expect("bind");
    assert_eq!(bind.submitted_expected_revision(), Some(2));

    let handoff = parse_taskspace_transition_args(
        r#"{"action":"complete_then_continue","expected_revision":3,"current_node_id":"inspect","next_node_id":"verify"}"#,
    )
    .expect("handoff");
    assert_eq!(handoff.action_name(), "complete_then_continue");
}

#[test]
fn rejects_removed_sibling_field() {
    let error = parse_taskspace_transition_args(
        r#"{"action":"bind_node","expected_revision":2,"node_id":"verify","required_next_call":"ordinary_tool"}"#,
    )
    .expect_err("legacy sibling metadata must be rejected");
    assert!(error.to_string().contains("required_next_call"));
}
