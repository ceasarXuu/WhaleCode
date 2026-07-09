use super::map::NodeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeContract {
    pub(crate) max_main_tool_results_before_split_hint: usize,
}

pub(crate) fn contract_for(kind: NodeKind) -> NodeContract {
    match kind {
        NodeKind::InspectCodeContext => NodeContract {
            max_main_tool_results_before_split_hint: 10,
        },
        NodeKind::ImplementSolution => NodeContract {
            max_main_tool_results_before_split_hint: 10,
        },
        NodeKind::SmokeTest | NodeKind::RegressionTest => NodeContract {
            max_main_tool_results_before_split_hint: 8,
        },
        NodeKind::FinalSynthesis => NodeContract {
            max_main_tool_results_before_split_hint: 6,
        },
        NodeKind::Custom => NodeContract {
            max_main_tool_results_before_split_hint: 6,
        },
    }
}
