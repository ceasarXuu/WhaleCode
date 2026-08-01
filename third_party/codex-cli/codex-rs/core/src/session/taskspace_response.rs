use super::Session;
use super::TurnContext;
use crate::action_map::ActionMapDeclaredCall;
use crate::action_map::ActionMapPreparedCall;
use crate::action_map::ActionMapPreparedResponse;
use crate::action_map::ActionMapResponseOperation;
use crate::action_map::ActionMapResponsePrepareError;
use crate::action_map::ActionMapResponseSettlement;
use crate::action_map::BlockRecord;
use crate::action_map::CompletionRecord;
use crate::action_map::GraphMutation;
use crate::action_map::MapEdge;
use crate::action_map::MapNode;
use crate::action_map::NodeMutation;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphEdgeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphNodeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceMutationArgs;
use crate::tools::sequence_preflight::TaskSpaceDeclaredCall;

impl Session {
    pub(crate) async fn taskspace_response_settlement(
        &self,
        prepared: &ActionMapPreparedResponse,
    ) -> Result<ActionMapResponseSettlement, String> {
        let prepared = prepared.clone();
        self.read_canonical_action_map("response_settlement", move |runtime, _| {
            runtime.response_settlement_for_main(&prepared)
        })
        .await?
    }

    pub(crate) async fn prepare_taskspace_response(
        &self,
        turn_context: &TurnContext,
        control_call_id: &str,
        args: TaskSpaceControlArgs,
        declared_calls: Vec<TaskSpaceDeclaredCall>,
    ) -> Result<ActionMapPreparedResponse, ActionMapResponsePrepareError> {
        let source_refs = {
            let state = self.state.lock().await;
            for call in &declared_calls {
                crate::action_map::TaskSpaceEventStore::validate_call_owner(
                    &call.call_id,
                    &call.node_id,
                )
                .map_err(|error| {
                    ActionMapResponsePrepareError::protocol(
                        "taskspace_call_attribution_invalid",
                        format!(
                            "TaskSpace call attribution preflight failed for `{}`: {error}",
                            call.call_id
                        ),
                    )
                })?;
            }
            state
                .taskspace_events
                .initialization_source_event_ids(control_call_id)
        };
        let operation =
            response_operation(args, control_call_id, &source_refs).map_err(|error| {
                ActionMapResponsePrepareError::protocol(
                    "taskspace_control_operation_invalid",
                    error,
                )
            })?;
        let declared_calls = declared_calls
            .into_iter()
            .map(|call| ActionMapDeclaredCall {
                call_id: call.call_id,
                call_index: call.call_index,
                node_id: call.node_id,
                tool_name: call.tool_name,
            })
            .collect();
        let (prepared, events) = self
            .mutate_canonical_action_map("prepare_response", |runtime, principal| {
                match runtime.prepare_response_for_main(
                    principal,
                    control_call_id,
                    operation,
                    declared_calls,
                ) {
                    Ok((prepared, events)) => (Ok(prepared), events),
                    Err(error) => (Err(error), Vec::new()),
                }
            })
            .await
            .map_err(|error| {
                ActionMapResponsePrepareError::resource(
                    "taskspace_canonical_store_unavailable",
                    error,
                )
            })?;
        let prepared = prepared.map_err(ActionMapResponsePrepareError::state)?;
        {
            let mut state = self.state.lock().await;
            for call in &prepared.prepared_calls {
                state
                    .taskspace_events
                    .bind_validated_call_owner(call.call_id.clone(), call.node_id.clone());
            }
        }
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(prepared)
    }

    pub(crate) async fn record_taskspace_bound_tool_result(
        &self,
        turn_context: &TurnContext,
        prepared: &ActionMapPreparedCall,
        success: bool,
        result_ref_id: String,
    ) -> Result<(), String> {
        let prepared = prepared.clone();
        let (result, events) = self
            .mutate_canonical_action_map("release_action_reservation", |runtime, principal| {
                match runtime.release_main_action_result(
                    principal,
                    &prepared,
                    success,
                    result_ref_id,
                ) {
                    Ok(events) => (Ok(()), events),
                    Err(error) => (Err(error), Vec::new()),
                }
            })
            .await?;
        result?;
        self.emit_action_map_events_for_turn(turn_context, events)
            .await;
        Ok(())
    }
}

fn response_operation(
    args: TaskSpaceControlArgs,
    action_id: &str,
    source_refs: &[String],
) -> Result<ActionMapResponseOperation, String> {
    let action_name = args.action_name();
    match args {
        TaskSpaceControlArgs::InitializeAndExecute {
            root,
            work_nodes,
            finish,
            edges,
            ..
        } => Ok(ActionMapResponseOperation::Initialize {
            root: map_node(root, source_refs.to_vec()),
            work_nodes: work_nodes
                .into_iter()
                .map(|node| map_node(node, Vec::new()))
                .collect(),
            finish: map_node(finish, Vec::new()),
            edges: edges.into_iter().map(map_edge).collect(),
        }),
        TaskSpaceControlArgs::Execute {
            expected_revision,
            mutations,
            ..
        } => {
            let mut graph = GraphMutation::default();
            let mut node_mutations = Vec::new();
            for mutation in mutations {
                match mutation {
                    TaskSpaceMutationArgs::AddWorkNodes { work_nodes } => {
                        graph.add_work_nodes.extend(
                            work_nodes
                                .into_iter()
                                .map(|node| map_node(node, Vec::new())),
                        );
                    }
                    TaskSpaceMutationArgs::AddEdges { edges } => {
                        graph.add_edges.extend(edges.into_iter().map(map_edge));
                    }
                    TaskSpaceMutationArgs::RemoveEdges { edges } => {
                        graph.remove_edges.extend(edges.into_iter().map(map_edge));
                    }
                    TaskSpaceMutationArgs::CompleteNode { node_id } => {
                        node_mutations.push(NodeMutation::Complete {
                            node_id,
                            record: CompletionRecord {
                                action_id: action_id.to_string(),
                                result_ref_ids: Vec::new(),
                                evidence_ref_ids: Vec::new(),
                            },
                        });
                    }
                    TaskSpaceMutationArgs::BlockNode { node_id } => {
                        node_mutations.push(NodeMutation::Block {
                            node_id,
                            record: BlockRecord {
                                action_id: action_id.to_string(),
                                reason_ref: format!("taskspace-control:{action_id}"),
                            },
                        });
                    }
                    TaskSpaceMutationArgs::UnblockNode { node_id } => {
                        node_mutations.push(NodeMutation::Unblock { node_id });
                    }
                }
            }
            Ok(ActionMapResponseOperation::Execute {
                expected_revision,
                graph,
                node_mutations,
            })
        }
        TaskSpaceControlArgs::ReopenMap {
            expected_revision,
            work_nodes,
            edges,
            ..
        } => Ok(ActionMapResponseOperation::Reopen {
            expected_revision,
            work_nodes: work_nodes
                .into_iter()
                .map(|node| map_node(node, Vec::new()))
                .collect(),
            edges: edges.into_iter().map(map_edge).collect(),
        }),
        TaskSpaceControlArgs::ReadMap
        | TaskSpaceControlArgs::ReadOutputRef { .. }
        | TaskSpaceControlArgs::FinishMap { .. } => Err(format!(
            "`{action_name}` is not a response execution action"
        )),
    }
}

fn map_node(args: TaskSpaceGraphNodeArgs, source_refs: Vec<String>) -> MapNode {
    MapNode {
        node_id: args.node_id,
        goal: args.goal,
        source_refs,
    }
}

fn map_edge(args: TaskSpaceGraphEdgeArgs) -> MapEdge {
    MapEdge {
        from: args.from,
        to: args.to,
    }
}
