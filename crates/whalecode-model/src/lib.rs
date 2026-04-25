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
    ToolCallDelta {
        name: String,
        arguments_delta: String,
    },
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model: String,
    pub task: String,
    pub tool_summaries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResponse {
    pub events: Vec<ModelStreamEvent>,
    pub final_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapModelRuntime {
    pub model: String,
}

impl BootstrapModelRuntime {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }

    pub fn complete(&self, request: ModelRequest) -> ModelResponse {
        let observation = if request.tool_summaries.is_empty() {
            "No repository tools were executed.".to_owned()
        } else {
            request.tool_summaries.join("\n")
        };
        let final_text = format!(
            "Bootstrap agent accepted the task: {}\n\nRepository observation:\n{}\n\nLive DeepSeek execution and patch-safe writes are not enabled in this slice yet.",
            request.task, observation
        );

        ModelResponse {
            events: vec![
                ModelStreamEvent::ReasoningDelta(
                    "Build a replayable turn before enabling mutating tools.".to_owned(),
                ),
                ModelStreamEvent::TextDelta(final_text.clone()),
                ModelStreamEvent::Finished,
            ],
            final_text,
        }
    }
}
