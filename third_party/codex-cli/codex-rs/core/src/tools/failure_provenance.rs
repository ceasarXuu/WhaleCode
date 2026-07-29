use serde_json::Value;

pub(crate) fn provider_response_failure_provenance<'a>(
    call_ids: impl IntoIterator<Item = &'a str>,
) -> Value {
    let first_call_id = call_ids
        .into_iter()
        .find(|call_id| !call_id.trim().is_empty())
        .unwrap_or("unpaired");
    serde_json::json!({
        "scope": "provider_response",
        "copy_group_id": format!("provider_response:{first_call_id}"),
        "zero_dispatch": true,
    })
}

pub(crate) fn exact_failure_cause(message: &str) -> Value {
    serde_json::from_str(message).unwrap_or_else(|_| {
        serde_json::json!({
            "format": "text",
            "text": message,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_uses_the_first_nonempty_call_id() {
        let value = provider_response_failure_provenance(["", "call-2", "call-3"]);
        assert_eq!(
            value["copy_group_id"],
            serde_json::json!("provider_response:call-2")
        );
        assert_eq!(value["zero_dispatch"], serde_json::json!(true));
    }

    #[test]
    fn exact_cause_preserves_structured_and_text_failures() {
        assert_eq!(
            exact_failure_cause(r#"{"error":{"code":"failed"}}"#)["error"]["code"],
            serde_json::json!("failed")
        );
        assert_eq!(
            exact_failure_cause("plain failure"),
            serde_json::json!({"format": "text", "text": "plain failure"})
        );
    }
}
