use std::sync::Arc;

use codex_protocol::protocol::MapRuntimeMode;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::action_map::ActionMapPreparedResponse;
use crate::function_tool::FunctionCallError;
use crate::session::tests::make_session_and_context;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolCallSource;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use crate::tools::sequence_preflight::TaskSpaceDeclaredCall;
use crate::tools::taskspace_sequence_context::TaskSpaceSequenceInvocation;
use crate::tools::taskspace_sequence_context::TaskSpaceSequenceWorkBinding;
use crate::turn_diff_tracker::TurnDiffTracker;

#[derive(Default)]
struct ProbeObservation {
    dispatch_count: usize,
    prepared: Option<ActionMapPreparedResponse>,
    outer_call_id: Option<String>,
    item_id: Option<String>,
}

struct ControlSeamProbeHandler {
    observation: Arc<Mutex<ProbeObservation>>,
}

impl ToolHandler for ControlSeamProbeHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolCallSource::TaskSpaceSequence(sequence) = &invocation.source else {
            return Err(FunctionCallError::RespondToModel(
                "probe requires TaskSpace sequence metadata".to_string(),
            ));
        };
        let ToolPayload::Function { arguments } = &invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "probe requires function arguments".to_string(),
            ));
        };
        let args = parse_taskspace_control_args(arguments)?;
        let declared_calls = sequence
            .work_bindings
            .iter()
            .map(|binding| TaskSpaceDeclaredCall {
                call_id: binding.call_id.clone(),
                call_index: binding.call_index,
                node_id: binding.node_id.clone(),
                tool_name: binding.tool_name.clone(),
            })
            .collect();
        let prepared = invocation
            .session
            .prepare_taskspace_response_from_source(
                &invocation.turn,
                &invocation.call_id,
                &sequence.outer_call_id,
                args,
                declared_calls,
            )
            .await
            .map_err(|error| FunctionCallError::RespondToModel(format!("{error:?}")))?;

        let mut observation = self.observation.lock().await;
        observation.dispatch_count += 1;
        observation.prepared = Some(prepared);
        observation.outer_call_id = Some(sequence.outer_call_id.clone());
        observation.item_id = Some(sequence.item_id.clone());
        Ok(FunctionToolOutput::from_text(
            "prepared".to_string(),
            Some(true),
        ))
    }
}

fn taskspace_control_spec() -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".to_string(),
        description: String::new(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::default(),
        output_schema: None,
    })
}

fn control_call() -> ToolCall {
    ToolCall {
        provider_tool_name: ToolName::plain("taskspace_control"),
        dispatch_tool_name: ToolName::plain("taskspace_control"),
        call_id: "outer-1/control-1".to_string(),
        payload: ToolPayload::Function {
            arguments: serde_json::json!({
                "action": "initialize_and_execute",
                "root": {"node_id": "root", "goal": "Complete the task"},
                "work_nodes": [{"node_id": "work", "goal": "Perform the work"}],
                "finish": {"node_id": "finish", "goal": "Close the task"},
                "edges": [
                    {"from": "root", "to": "work"},
                    {"from": "work", "to": "finish"}
                ],
                "actions": [{"node_id": "legacy-copy", "tool": "legacy_copy"}]
            })
            .to_string(),
        },
    }
}

#[tokio::test]
async fn taskspace_sequence_control_metadata_reaches_one_router_handler_commit() {
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    let observation = Arc::new(Mutex::new(ProbeObservation::default()));
    let handler = Arc::new(ControlSeamProbeHandler {
        observation: Arc::clone(&observation),
    });
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec(taskspace_control_spec());
    builder.register_handler("taskspace_control", handler);
    let router = ToolRouter::from_builder_for_test(builder);
    let source = ToolCallSource::TaskSpaceSequence(TaskSpaceSequenceInvocation {
        outer_call_id: "outer-1".to_string(),
        item_id: "control-1".to_string(),
        node_id: None,
        work_bindings: vec![TaskSpaceSequenceWorkBinding {
            call_id: "outer-1/work-1".to_string(),
            call_index: 1,
            node_id: "work".to_string(),
            tool_name: "probe_work".to_string(),
        }]
        .into(),
    });

    let result = router
        .dispatch_tool_call_with_code_mode_result(
            Arc::clone(&session),
            turn,
            CancellationToken::new(),
            Arc::new(Mutex::new(TurnDiffTracker::new())),
            control_call(),
            source,
        )
        .await
        .expect("Router should dispatch the control probe");

    assert_eq!(result.call_id, "outer-1/control-1");
    let observation = observation.lock().await;
    assert_eq!(observation.dispatch_count, 1);
    assert_eq!(observation.outer_call_id.as_deref(), Some("outer-1"));
    assert_eq!(observation.item_id.as_deref(), Some("control-1"));
    let prepared = observation.prepared.as_ref().expect("prepared response");
    assert_eq!(prepared.prepared_calls.len(), 1);
    assert_eq!(prepared.prepared_calls[0].call_id, "outer-1/work-1");
    assert_eq!(prepared.prepared_calls[0].node_id, "work");
    assert_eq!(prepared.prepared_calls[0].tool_name, "probe_work");
    assert_ne!(prepared.prepared_calls[0].tool_name, "legacy_copy");
    assert!(
        prepared.prepared_calls[0]
            .reservation_id
            .contains("outer-1/control-1")
    );

    let snapshot = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot");
    let map = snapshot.map.expect("initialized map");
    assert_eq!(map.reservations.len(), 1);
    for expected_node_id in ["root", "work", "finish"] {
        assert!(map.nodes.iter().any(|node| node.id == expected_node_id));
    }
}
