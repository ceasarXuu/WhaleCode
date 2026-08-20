use codex_protocol::models::ResponseItem;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    None,
    Zstd,
}

pub(crate) fn attach_item_ids(payload_json: &mut Value, original_items: &[ResponseItem]) {
    let Some(input_value) = payload_json.get_mut("input") else {
        return;
    };
    let Value::Array(items) = input_value else {
        return;
    };

    for (value, item) in items.iter_mut().zip(original_items.iter()) {
        if let ResponseItem::Reasoning { id, .. }
        | ResponseItem::Message { id: Some(id), .. }
        | ResponseItem::WebSearchCall { id: Some(id), .. }
        | ResponseItem::FunctionCall { id: Some(id), .. }
        | ResponseItem::ToolSearchCall { id: Some(id), .. }
        | ResponseItem::LocalShellCall { id: Some(id), .. }
        | ResponseItem::CustomToolCall { id: Some(id), .. } = item
        {
            if id.is_empty() {
                continue;
            }

            if let Some(obj) = value.as_object_mut() {
                obj.insert("id".to_string(), Value::String(id.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use codex_protocol::models::WebSearchAction;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn hosted_item_ids_survive_responses_input_replay() {
        let original_items = vec![
            ResponseItem::WebSearchCall {
                id: Some("ws_123".to_string()),
                status: Some("completed".to_string()),
                action: Some(WebSearchAction::Search {
                    query: Some("TaskSpace protocol".to_string()),
                    queries: None,
                }),
            },
            ResponseItem::ImageGenerationCall {
                id: "ig_123".to_string(),
                status: "completed".to_string(),
                revised_prompt: Some("A dependency map".to_string()),
                result: "ZmFrZS1pbWFnZQ==".to_string(),
            },
        ];
        let mut payload = json!({"input": original_items});

        attach_item_ids(&mut payload, &original_items);

        assert_eq!(payload["input"][0]["id"], "ws_123");
        assert_eq!(payload["input"][1]["id"], "ig_123");
    }
}
