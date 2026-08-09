use std::collections::BTreeMap;
use std::sync::Arc;

use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use pretty_assertions::assert_eq;

use super::*;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::map_node;
use crate::action_map::rooted_dag::new_map;

fn catalog() -> Arc<TaskSpaceExecCatalog> {
    Arc::new(
        TaskSpaceExecCatalog::build(&[ToolSpec::Function(ResponsesApiTool {
            name: "read_file".into(),
            description: "Read a file.".into(),
            strict: false,
            parameters: JsonSchema::object(
                BTreeMap::from([("path".into(), JsonSchema::string(None))]),
                Some(vec!["path".into()]),
                Some(AdditionalProperties::Boolean(false)),
            ),
            output_schema: None,
            defer_loading: None,
        })])
        .unwrap(),
    )
}

fn map(revision: u64) -> crate::action_map::rooted_dag::TaskSpaceMap {
    let mut map = new_map(
        "map-1".into(),
        map_node("root", "deliver", NodeState::InFlight, "", vec![]),
        vec![map_node(
            "work",
            "inspect",
            NodeState::Ready,
            "",
            vec!["root".into()],
        )],
        map_node(
            "finish",
            "close",
            NodeState::Waiting,
            "",
            vec!["work".into()],
        ),
    );
    map.revision = revision;
    map
}

fn arguments() -> &'static str {
    r#"{"calls":[{"tool":"read_file","node_id":"work","arguments":{"path":"a.rs"}}],"hosted_bindings":[]}"#
}

#[test]
fn request_context_captures_revision_and_catalog_without_agent_fields() {
    let map = map(7);
    let catalog = catalog();
    let context =
        TaskSpaceExecRequestContext::capture("map-1", Some(&map), catalog.clone()).unwrap();

    assert_eq!(context.map_id(), "map-1");
    assert_eq!(context.request_revision(), Some(7));
    assert!(
        context
            .clone()
            .decode_outer_call("outer", arguments())
            .is_ok()
    );
    let declaration = serde_json::to_string(&catalog.declaration().parameters).unwrap();
    for forbidden in ["expected_revision", "capability_id", "outer_call_id"] {
        assert!(!declaration.contains(forbidden));
    }
}

#[test]
fn blank_map_revision_is_distinct_from_initialized_revision() {
    let context = TaskSpaceExecRequestContext::capture("map-1", None, catalog()).unwrap();
    assert_eq!(context.request_revision(), None);
    assert_eq!(
        context.validate_current_map(Some(&map(1))),
        Err(TaskSpaceExecEnvelopeError::MapRevisionChanged {
            expected: None,
            current: Some(1)
        })
    );
}

#[test]
fn stale_concurrent_response_is_rejected_before_plan_use() {
    let request_map = map(3);
    let current_map = map(4);
    let context =
        TaskSpaceExecRequestContext::capture("map-1", Some(&request_map), catalog()).unwrap();

    assert_eq!(
        context.validate_current_map(Some(&current_map)),
        Err(TaskSpaceExecEnvelopeError::MapRevisionChanged {
            expected: Some(3),
            current: Some(4)
        })
    );
    assert_eq!(context.validate_current_map(Some(&request_map)), Ok(()));
}

#[test]
fn retry_reuses_request_snapshot_but_outer_call_identity_stays_response_local() {
    let request_map = map(5);
    let context =
        TaskSpaceExecRequestContext::capture("map-1", Some(&request_map), catalog()).unwrap();
    let retry = context.clone();
    let first = context
        .decode_outer_call("call-first", arguments())
        .unwrap();
    let second = retry.decode_outer_call("call-retry", arguments()).unwrap();

    assert_eq!(first.request().request_revision(), Some(5));
    assert_eq!(first.plan(), second.plan());
    assert_eq!(
        first.internal_call_id(0).unwrap().transport_id(),
        "call-first/taskspace/call/0"
    );
    assert_eq!(
        second.internal_call_id(0).unwrap().transport_id(),
        "call-retry/taskspace/call/0"
    );
}

#[test]
fn call_identity_is_bounded_by_the_decoded_outer_plan() {
    let request_map = map(2);
    let envelope = TaskSpaceExecRequestContext::capture("map-1", Some(&request_map), catalog())
        .unwrap()
        .decode_outer_call("outer-1", arguments())
        .unwrap();

    assert_eq!(envelope.outer_call_id(), "outer-1");
    assert_eq!(envelope.plan().calls.len(), 1);
    assert_eq!(
        envelope.internal_call_id(1),
        Err(TaskSpaceExecEnvelopeError::CallIndexOutOfRange {
            index: 1,
            call_count: 1
        })
    );
}

#[test]
fn map_identity_and_outer_identity_are_mechanical_hard_errors() {
    let current = map(1);
    let mismatched =
        TaskSpaceExecRequestContext::capture("other", Some(&current), catalog()).unwrap();
    assert_eq!(
        mismatched.validate_current_map(Some(&current)).unwrap_err(),
        TaskSpaceExecEnvelopeError::MapIdentityChanged {
            expected: "other".into(),
            current: "map-1".into()
        }
    );
    let context = TaskSpaceExecRequestContext::capture("map-1", Some(&current), catalog()).unwrap();
    assert_eq!(
        context.decode_outer_call("", arguments()).unwrap_err(),
        TaskSpaceExecEnvelopeError::EmptyOuterCallIdentity
    );
}
