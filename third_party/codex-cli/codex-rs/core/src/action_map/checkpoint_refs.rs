use super::event_store::TaskSpaceEvent;
use serde_json::Value;
use std::collections::BTreeSet;

const OUTPUT_REF_PREFIX: &str = "output-ref://sha256/";
const SHA256_HEX_LEN: usize = 64;

pub(super) fn checkpoint_output_refs(events: &[TaskSpaceEvent]) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for event in events {
        collect_value_refs(&event.raw_payload, &mut refs);
    }
    refs.into_iter().collect()
}

fn collect_value_refs(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => collect_text_refs(text, refs),
        Value::Array(values) => {
            for value in values {
                collect_value_refs(value, refs);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_value_refs(value, refs);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn collect_text_refs(text: &str, refs: &mut BTreeSet<String>) {
    let mut remaining = text;
    while let Some(offset) = remaining.find(OUTPUT_REF_PREFIX) {
        let candidate = &remaining[offset..];
        let end = OUTPUT_REF_PREFIX.len() + SHA256_HEX_LEN;
        if candidate.len() >= end
            && candidate[OUTPUT_REF_PREFIX.len()..end]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            refs.insert(candidate[..end].to_string());
        }
        remaining = &candidate[OUTPUT_REF_PREFIX.len()..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_complete_content_addressed_refs() {
        let sha = "a".repeat(64);
        let value = serde_json::json!({
            "one": format!("before {OUTPUT_REF_PREFIX}{sha} after"),
            "two": [format!("{OUTPUT_REF_PREFIX}{sha}"), "output-ref://sha256/short"],
        });
        let mut refs = BTreeSet::new();
        collect_value_refs(&value, &mut refs);
        assert_eq!(refs, BTreeSet::from([format!("{OUTPUT_REF_PREFIX}{sha}")]));
    }
}
