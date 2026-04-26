use serde::{Deserialize, Serialize};

use crate::ModelStreamEvent;

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
            "Bootstrap agent accepted the task: {}\n\nRepository observation:\n{}\n\nThis local bootstrap mode does not call DeepSeek or edit files. Use whale run --allow-write for the live patch-safe tool loop.",
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
