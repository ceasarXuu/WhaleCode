use anyhow::Context;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;

fn canonicalize_json(value: &Value) -> Value {
    canonicalize_json_value(value, None)
}

fn canonicalize_json_value(value: &Value, field_name: Option<&str>) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            canonicalize_json_value(value, Some(key.as_str())),
                        )
                    })
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| canonicalize_json_value(item, field_name))
                .collect(),
        ),
        Value::String(text) if field_name == Some("x-codex-turn-metadata") => {
            Value::String(canonicalize_serialized_json(text))
        }
        Value::String(text) if field_name == Some("description") => {
            Value::String(canonicalize_fenced_json(text))
        }
        _ => value.clone(),
    }
}

fn canonicalize_serialized_json(text: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return text.to_string();
    };
    serde_json::to_string(&canonicalize_json(&value)).expect("canonical JSON value must serialize")
}

fn canonicalize_fenced_json(text: &str) -> String {
    const OPENING_FENCE: &str = "```json\n";
    const CLOSING_FENCE: &str = "\n```";
    let mut rendered = String::with_capacity(text.len());
    let mut remainder = text;
    while let Some(opening_offset) = remainder.find(OPENING_FENCE) {
        let json_start = opening_offset + OPENING_FENCE.len();
        let Some(closing_offset) = remainder[json_start..].find(CLOSING_FENCE) else {
            break;
        };
        let json_end = json_start + closing_offset;
        rendered.push_str(&remainder[..json_start]);
        let embedded = &remainder[json_start..json_end];
        match serde_json::from_str::<Value>(embedded) {
            Ok(value) => rendered.push_str(
                &serde_json::to_string(&canonicalize_json(&value))
                    .expect("canonical JSON value must serialize"),
            ),
            Err(_) => rendered.push_str(embedded),
        }
        rendered.push_str(CLOSING_FENCE);
        remainder = &remainder[json_end + CLOSING_FENCE.len()..];
    }
    rendered.push_str(remainder);
    rendered
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalWireEvidence {
    pub raw_body_sha256: String,
    pub structured_body: Value,
}

impl FinalWireEvidence {
    pub fn from_raw_body(raw_body: &[u8]) -> anyhow::Result<Self> {
        let structured_body = serde_json::from_slice(raw_body)
            .context("final-wire request body must be valid JSON")?;
        Ok(Self {
            raw_body_sha256: format!("{:x}", Sha256::digest(raw_body)),
            structured_body,
        })
    }

    pub fn render(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(&serde_json::json!({
            "raw_body_sha256": self.raw_body_sha256,
            "structured_body": self.structured_body,
        }))
        .context("final-wire evidence must be serializable")
    }
}

pub fn render_cache_snapshot(scenario_id: &str, value: &Value) -> anyhow::Result<String> {
    if scenario_id.is_empty()
        || !scenario_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        anyhow::bail!(
            "cache snapshot scenario id must use lowercase ASCII, digits, or underscores"
        );
    }
    // serde_json's `preserve_order` feature is unified when core is tested with
    // app-server/TUI. Sort object keys explicitly, including JSON encoded in
    // headers and fenced tool examples, so core-only and full-workspace cache
    // evidence are identical.
    let rendered = serde_json::to_string_pretty(&canonicalize_json(value))
        .context("cache snapshot evidence must be serializable")?;
    if let Some(report_dir) = std::env::var_os("WHALE_CACHE_CHANGE_REPORT_DIR") {
        let report_dir = Path::new(&report_dir);
        std::fs::create_dir_all(report_dir).context("create cache change report directory")?;
        std::fs::write(report_dir.join(format!("{scenario_id}.json")), &rendered)
            .context("write cache change report candidate")?;
    }
    Ok(rendered)
}
