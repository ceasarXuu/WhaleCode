use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use serde_json::Value;

pub(super) fn chat_messages_from_response_items(items: &[ResponseItem]) -> Vec<Value> {
    let mut messages = Vec::new();
    let mut pending_assistant = PendingAssistantMessage::default();

    for item in items {
        match item {
            ResponseItem::Reasoning { content, .. } => {
                pending_assistant.append_reasoning(reasoning_content_to_text(content));
            }
            ResponseItem::Message { role, content, .. } if role == "assistant" => {
                pending_assistant.append_content(content_items_to_text(content));
            }
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            }
            | ResponseItem::CustomToolCall {
                name,
                input: arguments,
                call_id,
                ..
            } => {
                pending_assistant.push_tool_call(name, arguments, call_id);
            }
            ResponseItem::Message { role, content, .. } => {
                pending_assistant.flush_into(&mut messages);
                let role = if role == "developer" { "system" } else { role };
                let text = content_items_to_text(content);
                if !text.trim().is_empty() {
                    messages.push(serde_json::json!({ "role": role, "content": text }));
                }
            }
            ResponseItem::FunctionCallOutput { call_id, output }
            | ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } => {
                pending_assistant.flush_into(&mut messages);
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": function_output_to_text(&output.body),
                }));
            }
            ResponseItem::ToolSearchOutput {
                call_id: Some(call_id),
                tools,
                ..
            } => {
                pending_assistant.flush_into(&mut messages);
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": serde_json::to_string(tools).unwrap_or_default(),
                }));
            }
            _ => {}
        }
    }

    pending_assistant.flush_into(&mut messages);
    messages
}

#[derive(Default)]
struct PendingAssistantMessage {
    reasoning_content: String,
    content: String,
    tool_calls: Vec<Value>,
}

impl PendingAssistantMessage {
    fn append_reasoning(&mut self, reasoning: String) {
        self.reasoning_content.push_str(&reasoning);
    }

    fn append_content(&mut self, content: String) {
        if content.trim().is_empty() {
            return;
        }
        if !self.content.is_empty() {
            self.content.push('\n');
        }
        self.content.push_str(&content);
    }

    fn push_tool_call(&mut self, name: &str, arguments: &str, call_id: &str) {
        self.tool_calls.push(serde_json::json!({
            "id": call_id,
            "type": "function",
            "function": {
                "name": name,
                "arguments": arguments,
            }
        }));
    }

    fn flush_into(&mut self, messages: &mut Vec<Value>) {
        if self.content.trim().is_empty() && self.tool_calls.is_empty() {
            self.reasoning_content.clear();
            return;
        }

        let mut message = serde_json::Map::new();
        message.insert("role".to_string(), Value::String("assistant".to_string()));
        message.insert(
            "content".to_string(),
            if self.content.trim().is_empty() {
                Value::Null
            } else {
                Value::String(std::mem::take(&mut self.content))
            },
        );
        if !self.reasoning_content.trim().is_empty() {
            message.insert(
                "reasoning_content".to_string(),
                Value::String(std::mem::take(&mut self.reasoning_content)),
            );
        }
        if !self.tool_calls.is_empty() {
            message.insert(
                "tool_calls".to_string(),
                Value::Array(std::mem::take(&mut self.tool_calls)),
            );
        }

        messages.push(Value::Object(message));
    }
}

fn reasoning_content_to_text(content: &Option<Vec<ReasoningItemContent>>) -> String {
    content
        .iter()
        .flatten()
        .map(|item| match item {
            ReasoningItemContent::ReasoningText { text } | ReasoningItemContent::Text { text } => {
                text.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn content_items_to_text(content: &[ContentItem]) -> String {
    content
        .iter()
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            ContentItem::InputImage { .. } => Some("[Image omitted: DeepSeek is text-only]"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn function_output_to_text(body: &FunctionCallOutputBody) -> String {
    body.to_text().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputPayload;
    use serde_json::json;

    #[test]
    fn preserves_reasoning_content_for_tool_calls() {
        let messages = chat_messages_from_response_items(&[
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "hi".to_string(),
                }],
                end_turn: None,
                phase: None,
            },
            ResponseItem::Reasoning {
                id: "reasoning-1".to_string(),
                summary: Vec::new(),
                content: Some(vec![ReasoningItemContent::ReasoningText {
                    text: "Need a directory listing.".to_string(),
                }]),
                encrypted_content: None,
            },
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "I'll inspect the directory.".to_string(),
                }],
                end_turn: None,
                phase: None,
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "shell_command".to_string(),
                namespace: None,
                arguments: r#"{"command":"Get-ChildItem"}"#.to_string(),
                call_id: "call_1".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call_1".to_string(),
                output: FunctionCallOutputPayload::from_text("ok".to_string()),
            },
        ]);

        assert_eq!(
            messages,
            vec![
                json!({"role": "user", "content": "hi"}),
                json!({
                    "role": "assistant",
                    "content": "I'll inspect the directory.",
                    "reasoning_content": "Need a directory listing.",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "shell_command",
                            "arguments": r#"{"command":"Get-ChildItem"}"#,
                        }
                    }]
                }),
                json!({
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "ok",
                })
            ]
        );
    }

    #[test]
    fn drops_unpaired_reasoning_content() {
        let messages = chat_messages_from_response_items(&[ResponseItem::Reasoning {
            id: "reasoning-only".to_string(),
            summary: Vec::new(),
            content: Some(vec![ReasoningItemContent::ReasoningText {
                text: "orphaned".to_string(),
            }]),
            encrypted_content: None,
        }]);

        assert!(messages.is_empty());
    }
}
