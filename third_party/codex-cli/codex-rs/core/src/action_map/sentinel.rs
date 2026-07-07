use super::map::ActionMapId;
use super::map::MapNodeId;
use super::map::NodeResultId;
use super::map::TaskId;
use super::map::TaskSpaceTraceEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceSentinelWarning {
    pub(crate) id: String,
    pub(crate) warning_type: TaskSpaceSentinelWarningType,
    pub(crate) status: TaskSpaceSentinelWarningStatus,
    pub(crate) severity: TaskSpaceSentinelSeverity,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) result_id: Option<NodeResultId>,
    pub(crate) trace_event_ids: Vec<String>,
    pub(crate) reason: String,
    pub(crate) clearance_action: String,
    pub(crate) clear_action: Option<String>,
    pub(crate) created_at_ms: i64,
    pub(crate) cleared_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceSentinelWarningType {
    ValidatorFailure,
    UnclassifiedShellAction,
}

impl TaskSpaceSentinelWarningType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ValidatorFailure => "validator_failure",
            Self::UnclassifiedShellAction => "unclassified_shell_action",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "validator_failure" => Some(Self::ValidatorFailure),
            "unclassified_shell_action" => Some(Self::UnclassifiedShellAction),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceSentinelWarningStatus {
    Active,
    Cleared,
}

impl TaskSpaceSentinelWarningStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Cleared => "cleared",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "cleared" => Some(Self::Cleared),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceSentinelSeverity {
    Warning,
}

impl TaskSpaceSentinelSeverity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "warning" => Some(Self::Warning),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceSentinelWarningDraft {
    pub(crate) warning_type: TaskSpaceSentinelWarningType,
    pub(crate) severity: TaskSpaceSentinelSeverity,
    pub(crate) reason: &'static str,
    pub(crate) clearance_action: &'static str,
}

pub(crate) fn warning_drafts_for_trace_event(
    event: &TaskSpaceTraceEvent,
) -> Vec<TaskSpaceSentinelWarningDraft> {
    let mut drafts = Vec::new();
    if event.tags.iter().any(|tag| tag == "validator_failure") {
        drafts.push(TaskSpaceSentinelWarningDraft {
            warning_type: TaskSpaceSentinelWarningType::ValidatorFailure,
            severity: TaskSpaceSentinelSeverity::Warning,
            reason: "Validator-class action failed; inspect the node result before using it as accepted evidence.",
            clearance_action: "Run a successful validator, revise the contract, or explicitly accept the risk before final artifact audit.",
        });
    }
    if event
        .tags
        .iter()
        .any(|tag| tag == "unclassified_shell_action")
    {
        drafts.push(TaskSpaceSentinelWarningDraft {
            warning_type: TaskSpaceSentinelWarningType::UnclassifiedShellAction,
            severity: TaskSpaceSentinelSeverity::Warning,
            reason: "Shell action was not structurally classified; do not infer output, provenance, or final-artifact semantics from it.",
            clearance_action: "Record explicit output contract or fact source metadata before relying on this action as final evidence.",
        });
    }
    drafts
}
