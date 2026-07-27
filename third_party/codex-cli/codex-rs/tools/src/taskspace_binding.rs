use crate::LoadableToolSpec;
use crate::ToolSpec;
use std::error::Error;
use std::fmt;

// Kept only until the B1X router cutover removes its existing public import.
// No TaskSpace provider schema or projection uses this field.
pub const TASKSPACE_BINDING_FIELD: &str = "taskspace_binding";

#[derive(Debug, Clone, PartialEq)]
pub enum TaskSpaceToolProjection {
    Visible(ToolSpec),
    Hidden {
        tool_name: String,
        tool_kind: &'static str,
        reason: &'static str,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TaskSpaceToolProjectionError {
    pub tool_name: String,
    pub field: &'static str,
}

impl fmt::Display for TaskSpaceToolProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TaskSpace ordinary Tool projection unexpectedly changed `{}` at `{}`",
            self.tool_name, self.field
        )
    }
}

impl Error for TaskSpaceToolProjectionError {}

pub fn project_taskspace_binding_tool(
    spec: ToolSpec,
) -> Result<TaskSpaceToolProjection, TaskSpaceToolProjectionError> {
    Ok(TaskSpaceToolProjection::Visible(spec))
}

pub fn project_taskspace_binding_loadable_tool(
    spec: LoadableToolSpec,
) -> Result<LoadableToolSpec, TaskSpaceToolProjectionError> {
    Ok(spec)
}

#[cfg(test)]
#[path = "taskspace_binding_tests.rs"]
mod tests;
