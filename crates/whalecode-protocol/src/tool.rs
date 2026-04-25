use serde::{Deserialize, Serialize};

use crate::{ArtifactId, ToolCallId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Succeeded,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionMode {
    ReadOnly,
    Write,
    LongRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub execution_mode: ToolExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolEvent {
    CallStarted {
        call_id: ToolCallId,
        tool_name: String,
    },
    CallFinished {
        call_id: ToolCallId,
        status: ToolStatus,
        output_artifact: Option<ArtifactId>,
    },
}
