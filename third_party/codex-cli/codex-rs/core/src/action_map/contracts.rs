use super::map::ActionClass;
use super::map::NodeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeContract {
    pub(crate) kind: NodeKind,
    pub(crate) allowed_actions: &'static [ActionClass],
    pub(crate) max_main_tool_results_before_split_hint: usize,
}

const INSPECT_ACTIONS: &[ActionClass] = &[
    ActionClass::Read,
    ActionClass::Search,
    ActionClass::Build,
    ActionClass::Test,
    ActionClass::Spawn,
    ActionClass::Wait,
    ActionClass::Review,
    ActionClass::Control,
];
const IMPLEMENT_ACTIONS: &[ActionClass] = &[
    ActionClass::Read,
    ActionClass::Search,
    ActionClass::Edit,
    ActionClass::Build,
    ActionClass::Spawn,
    ActionClass::Wait,
    ActionClass::Review,
    ActionClass::Control,
];
const TEST_ACTIONS: &[ActionClass] = &[
    ActionClass::Read,
    ActionClass::Build,
    ActionClass::Test,
    ActionClass::Spawn,
    ActionClass::Wait,
    ActionClass::Review,
    ActionClass::Control,
];
const FINAL_ACTIONS: &[ActionClass] = &[
    ActionClass::Read,
    ActionClass::Review,
    ActionClass::FinalResponse,
    ActionClass::Control,
];
const CUSTOM_ACTIONS: &[ActionClass] = &[
    ActionClass::Read,
    ActionClass::Search,
    ActionClass::Build,
    ActionClass::Spawn,
    ActionClass::Wait,
    ActionClass::Review,
    ActionClass::Control,
];

pub(crate) fn contract_for(kind: NodeKind) -> NodeContract {
    match kind {
        NodeKind::InspectCodeContext => NodeContract {
            kind,
            allowed_actions: INSPECT_ACTIONS,
            max_main_tool_results_before_split_hint: 6,
        },
        NodeKind::ImplementSolution => NodeContract {
            kind,
            allowed_actions: IMPLEMENT_ACTIONS,
            max_main_tool_results_before_split_hint: 10,
        },
        NodeKind::SmokeTest | NodeKind::RegressionTest => NodeContract {
            kind,
            allowed_actions: TEST_ACTIONS,
            max_main_tool_results_before_split_hint: 8,
        },
        NodeKind::FinalSynthesis => NodeContract {
            kind,
            allowed_actions: FINAL_ACTIONS,
            max_main_tool_results_before_split_hint: 6,
        },
        NodeKind::Custom => NodeContract {
            kind,
            allowed_actions: CUSTOM_ACTIONS,
            max_main_tool_results_before_split_hint: 6,
        },
    }
}

impl NodeContract {
    pub(crate) fn allows(self, action_class: ActionClass) -> bool {
        self.allowed_actions.contains(&action_class)
    }
}
