use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;
use std::collections::BTreeMap;

pub const WEB_FETCH_TOOL_NAME: &str = "web_fetch";

pub fn create_web_fetch_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "url".to_string(),
            JsonSchema::string(Some("HTTP or HTTPS URL to read.".to_string())),
        ),
        (
            "format".to_string(),
            JsonSchema::string_enum(
                vec![json!("markdown"), json!("text")],
                Some("Requested output format. Defaults to markdown.".to_string()),
            ),
        ),
        (
            "max_chars".to_string(),
            JsonSchema::integer(Some(
                "Maximum number of characters to return. Defaults to the configured limit."
                    .to_string(),
            )),
        ),
        (
            "reason".to_string(),
            JsonSchema::string(Some(
                "Brief reason this URL is needed for the current task.".to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: WEB_FETCH_TOOL_NAME.to_string(),
        description: "Reads the content of a previously discovered or user-provided HTTP(S) URL and returns markdown or text.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["url".to_string(), "reason".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}
