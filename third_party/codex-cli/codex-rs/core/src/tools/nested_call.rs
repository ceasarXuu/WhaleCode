use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde_json::Value as JsonValue;

use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;

pub(crate) fn build_native_nested_tool_call(
    spec: &ToolSpec,
    tool_name: ToolName,
    call_id: String,
    input: Option<JsonValue>,
) -> Result<ToolCall, String> {
    let payload = match spec {
        ToolSpec::Function(_) => ToolPayload::Function {
            arguments: serialize_function_tool_arguments(&tool_name, input)?,
        },
        ToolSpec::Freeform(_) => match input {
            Some(JsonValue::String(input)) => ToolPayload::Custom { input },
            _ => return Err(format!("tool `{tool_name}` expects a string input")),
        },
        _ => {
            return Err(format!(
                "tool `{tool_name}` cannot be invoked as a nested native tool"
            ));
        }
    };

    Ok(ToolCall {
        provider_tool_name: tool_name.clone(),
        dispatch_tool_name: tool_name,
        call_id,
        payload,
    })
}

pub(crate) fn serialize_function_tool_arguments(
    tool_name: &ToolName,
    input: Option<JsonValue>,
) -> Result<String, String> {
    match input {
        None => Ok("{}".to_string()),
        Some(JsonValue::Object(map)) => serde_json::to_string(&JsonValue::Object(map))
            .map_err(|err| format!("failed to serialize tool `{tool_name}` arguments: {err}")),
        Some(_) => Err(format!(
            "tool `{tool_name}` expects a JSON object for arguments"
        )),
    }
}

#[cfg(test)]
mod tests {
    use codex_tools::FreeformTool;
    use codex_tools::FreeformToolFormat;
    use codex_tools::JsonSchema;
    use codex_tools::ResponsesApiTool;

    use super::*;

    #[test]
    fn nested_call_builder_preserves_native_tool_identity_and_payload_kind() {
        let function_name = ToolName::plain("inspect");
        let function = ToolSpec::Function(ResponsesApiTool {
            name: function_name.name.clone(),
            description: String::new(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::default(),
            output_schema: None,
        });
        let call = build_native_nested_tool_call(
            &function,
            function_name.clone(),
            "function-call".into(),
            Some(serde_json::json!({"path": "README.md"})),
        )
        .expect("function call");

        assert_eq!(call.provider_tool_name, function_name);
        assert_eq!(call.dispatch_tool_name, ToolName::plain("inspect"));
        assert!(matches!(call.payload, ToolPayload::Function { .. }));

        let freeform_name = ToolName::plain("patch");
        let freeform = ToolSpec::Freeform(FreeformTool {
            name: freeform_name.name.clone(),
            description: String::new(),
            format: FreeformToolFormat {
                r#type: "grammar".into(),
                syntax: "lark".into(),
                definition: String::new(),
            },
        });
        let call = build_native_nested_tool_call(
            &freeform,
            freeform_name.clone(),
            "freeform-call".into(),
            Some(JsonValue::String("patch body".into())),
        )
        .expect("freeform call");

        assert_eq!(call.provider_tool_name, freeform_name);
        assert_eq!(call.dispatch_tool_name, ToolName::plain("patch"));
        assert!(matches!(call.payload, ToolPayload::Custom { .. }));
    }
}
