use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ModelStreamEvent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedToolCall {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedModelOutput {
    pub text: String,
    pub reasoning: String,
    pub tool_calls: Vec<CollectedToolCall>,
    pub finished: bool,
}

#[derive(Debug, Clone, Default)]
struct ToolCallAccumulator {
    id: Option<String>,
    name: String,
    arguments: String,
}

pub fn collect_model_output(events: &[ModelStreamEvent]) -> CollectedModelOutput {
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut tool_calls = BTreeMap::<usize, ToolCallAccumulator>::new();
    let mut finished = false;

    for event in events {
        match event {
            ModelStreamEvent::TextDelta(delta) => text.push_str(delta),
            ModelStreamEvent::ReasoningDelta(delta) => reasoning.push_str(delta),
            ModelStreamEvent::ToolCallDelta {
                index,
                id,
                name,
                arguments_delta,
            } => {
                let call = tool_calls.entry(*index).or_default();
                if let Some(id) = id {
                    if !id.is_empty() {
                        call.id = Some(id.clone());
                    }
                }
                if !name.is_empty() {
                    call.name = name.clone();
                }
                call.arguments.push_str(arguments_delta);
            }
            ModelStreamEvent::Usage(_) => {}
            ModelStreamEvent::Finished => finished = true,
        }
    }

    CollectedModelOutput {
        text,
        reasoning,
        tool_calls: tool_calls
            .into_iter()
            .map(|(index, call)| CollectedToolCall {
                index,
                id: call.id.unwrap_or_else(|| format!("tool-{index}")),
                name: call.name,
                arguments: call.arguments,
            })
            .collect(),
        finished,
    }
}
