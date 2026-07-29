use serde_json::Value;

pub(crate) fn provider_response_failure_provenance<'a>(
    call_ids: impl IntoIterator<Item = &'a str>,
) -> Value {
    let affected_call_ids = call_ids
        .into_iter()
        .filter(|call_id| !call_id.trim().is_empty())
        .collect::<Vec<_>>();
    let first_call_id = affected_call_ids.first().copied().unwrap_or("unpaired");
    serde_json::json!({
        "scope": "provider_response",
        "copy_group_id": format!("provider_response:{first_call_id}"),
        "zero_dispatch": true,
        "affected_call_ids": affected_call_ids,
    })
}

pub(crate) fn skipped_call_failure_provenance(call_id: &str, cause_call_id: &str) -> Value {
    serde_json::json!({
        "scope": "tool_sequence_skip",
        "copy_group_id": format!("tool_sequence_skip:{cause_call_id}"),
        "zero_dispatch": true,
        "affected_call_ids": [call_id],
        "cause_call_id": cause_call_id,
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
        assert_eq!(
            value["affected_call_ids"],
            serde_json::json!(["call-2", "call-3"])
        );
    }

    #[test]
    fn skipped_call_provenance_names_the_causal_call() {
        let value = skipped_call_failure_provenance("call-2", "call-1");
        assert_eq!(value["scope"], "tool_sequence_skip");
        assert_eq!(value["cause_call_id"], "call-1");
        assert_eq!(value["affected_call_ids"], serde_json::json!(["call-2"]));
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
