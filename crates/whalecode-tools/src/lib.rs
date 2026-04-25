use serde::{Deserialize, Serialize};
use whalecode_protocol::{ToolExecutionMode, ToolSpec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultEnvelope {
    pub content: String,
    pub truncated: bool,
    pub original_len: usize,
}

pub fn builtin_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "read_file".to_owned(),
            description: "Read a UTF-8 file from the workspace".to_owned(),
            execution_mode: ToolExecutionMode::ReadOnly,
        },
        ToolSpec {
            name: "search_text".to_owned(),
            description: "Search workspace text".to_owned(),
            execution_mode: ToolExecutionMode::ReadOnly,
        },
        ToolSpec {
            name: "edit_file".to_owned(),
            description: "Edit a file through patch-safe workspace logic".to_owned(),
            execution_mode: ToolExecutionMode::Write,
        },
    ]
}
