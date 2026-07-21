use codex_api::ResponsesApiRequest;
use codex_api::ToolChoice;
use codex_api::build_chat_completions_body;
use serde_json::Value;
use std::collections::BTreeSet;

pub fn deepseek_function_names(responses_tools: Vec<Value>) -> Result<BTreeSet<String>, String> {
    let request = ResponsesApiRequest {
        model: "deepseek-v4-pro".into(),
        instructions: String::new(),
        input: Vec::new(),
        tools: responses_tools,
        tool_choice: ToolChoice::Auto,
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        include: Vec::new(),
        service_tier: None,
        prompt_cache_key: None,
        text: None,
        client_metadata: None,
    };
    let body = build_chat_completions_body(&request);
    body.get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|tool| {
            tool.pointer("/function/name")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .ok_or_else(|| format!("DeepSeek mapper returned an unnamed tool: {tool}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_mapper_projects_only_supported_responses_tools() {
        let names = deepseek_function_names(vec![
            serde_json::json!({"type": "web_search"}),
            serde_json::json!({"type": "custom", "name": "apply_patch"}),
            serde_json::json!({"type": "image_generation"}),
        ])
        .expect("production mapper");
        assert_eq!(
            names,
            BTreeSet::from(["apply_patch".to_string(), "web_search".to_string()])
        );
    }
}
