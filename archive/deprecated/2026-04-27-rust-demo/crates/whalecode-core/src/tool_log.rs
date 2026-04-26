use serde_json::Value;

const MAX_TOOL_OUTPUT_LOG_BYTES: usize = 8 * 1024;

pub(crate) fn tool_log_preview(message: &str) -> (String, bool) {
    let redacted = redact_tool_message(message);
    if redacted.len() <= MAX_TOOL_OUTPUT_LOG_BYTES {
        return (redacted, false);
    }
    let mut boundary = MAX_TOOL_OUTPUT_LOG_BYTES;
    while !redacted.is_char_boundary(boundary) {
        boundary -= 1;
    }
    (format!("{}...[truncated]...", &redacted[..boundary]), true)
}

fn redact_tool_message(message: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(message) else {
        return message.to_owned();
    };
    serde_json::to_string(&redact_json_value(value)).unwrap_or_else(|_| message.to_owned())
}

fn redact_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String("[redacted]".to_owned()))
                    } else {
                        (key, redact_json_value(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_json_value).collect()),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("api_key")
        || key.contains("apikey")
        || key.contains("authorization")
        || key.contains("password")
        || key.contains("secret")
        || key.contains("token")
}
