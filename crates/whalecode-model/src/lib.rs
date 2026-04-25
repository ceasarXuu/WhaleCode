use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub model: String,
    pub context_window_tokens: u64,
    pub max_output_tokens: u64,
    pub supports_thinking: bool,
    pub supports_tool_calls: bool,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelStreamEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCallDelta { name: String, arguments_delta: String },
    Finished,
}
