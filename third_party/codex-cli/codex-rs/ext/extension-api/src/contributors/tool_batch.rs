use codex_tools::ToolName;
use codex_tools::ToolPayload;

use crate::ExtensionData;
use crate::ExtensionFuture;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolBatchCall {
    pub call_id: String,
    pub tool_name: ToolName,
    pub payload: ToolPayload,
}

/// Complete, ordered set of tool calls declared by one model response.
pub struct ToolBatchPreflightInput<'a> {
    pub session_store: &'a ExtensionData,
    pub thread_store: &'a ExtensionData,
    pub turn_store: &'a ExtensionData,
    pub calls: &'a [ToolBatchCall],
}

/// Extension-owned protocol failure that prevents every call in the batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolBatchPreflightFailure {
    pub code: String,
    pub message: String,
}

impl ToolBatchPreflightFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn model_message(&self) -> String {
        format!("tool batch rejected [{}]: {}", self.code, self.message)
    }
}

/// Gate invoked after a response is complete and before any declared tool call
/// begins execution. Contributors run in registration order; the first failure
/// rejects the entire batch. With no contributor installed this is a no-op.
pub trait ToolBatchPreflightContributor: Send + Sync {
    fn preflight<'a>(
        &'a self,
        input: ToolBatchPreflightInput<'a>,
    ) -> ExtensionFuture<'a, Result<(), ToolBatchPreflightFailure>>;
}
