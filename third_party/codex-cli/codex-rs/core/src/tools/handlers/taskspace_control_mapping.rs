use crate::action_map::ActionMapControlState;
use crate::action_map::ActionMapEdgeInput;
use crate::action_map::ActionMapInitializeFinishInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::action_map::NodeTransition;
use crate::tools::handlers::taskspace_control_args::TaskSpaceFinishNodeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphEdgeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphNodeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNodeTransition;

pub(super) fn control_state_has_active_binding(state: Option<&ActionMapControlState>) -> bool {
    state.is_some_and(|state| {
        state
            .current_node_id
            .as_ref()
            .is_some_and(|node_id| state.running_work_node_ids.contains(node_id))
    })
}

pub(super) fn map_node_input(node: TaskSpaceGraphNodeArgs) -> ActionMapInitializeNodeInput {
    ActionMapInitializeNodeInput {
        id: node.node_id,
        goal: node.goal,
    }
}

pub(super) fn map_finish_input(node: TaskSpaceFinishNodeArgs) -> ActionMapInitializeFinishInput {
    ActionMapInitializeFinishInput { id: node.node_id }
}

pub(super) fn map_edge_input(edge: TaskSpaceGraphEdgeArgs) -> ActionMapEdgeInput {
    ActionMapEdgeInput {
        from: edge.from,
        to: edge.to,
    }
}

pub(super) fn map_transition(transition: TaskSpaceNodeTransition) -> NodeTransition {
    match transition {
        TaskSpaceNodeTransition::Bind => NodeTransition::Bind,
        TaskSpaceNodeTransition::Complete => NodeTransition::Complete,
        TaskSpaceNodeTransition::Block => NodeTransition::Block,
        TaskSpaceNodeTransition::Unblock => NodeTransition::Unblock,
        TaskSpaceNodeTransition::Rework => NodeTransition::Rework,
    }
}
