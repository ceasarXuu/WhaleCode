use anyhow::Context;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;

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
    let rendered = serde_json::to_string_pretty(value)
        .context("cache snapshot evidence must be serializable")?;
    if let Some(report_dir) = std::env::var_os("WHALE_CACHE_CHANGE_REPORT_DIR") {
        let report_dir = Path::new(&report_dir);
        std::fs::create_dir_all(report_dir).context("create cache change report directory")?;
        std::fs::write(report_dir.join(format!("{scenario_id}.json")), &rendered)
            .context("write cache change report candidate")?;
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::FinalWireEvidence;
    use super::render_cache_snapshot;
    use serde_json::json;

    #[test]
    fn identical_raw_bodies_produce_identical_evidence() -> anyhow::Result<()> {
        let raw = br#"{"messages":[{"role":"system"},{"role":"user"}]}"#;

        let first = FinalWireEvidence::from_raw_body(raw)?;
        let second = FinalWireEvidence::from_raw_body(raw)?;

        assert_eq!(first, second);
        assert_eq!(first.render()?, second.render()?);
        Ok(())
    }

    #[test]
    fn raw_hash_and_structured_body_cover_different_changes() -> anyhow::Result<()> {
        let compact = br#"{"items":["first","second"]}"#;
        let spaced = br#"{ "items": ["first", "second"] }"#;
        let reordered = br#"{"items":["second","first"]}"#;
        let changed_field = br#"{"items":["first","changed"]}"#;

        let compact = FinalWireEvidence::from_raw_body(compact)?;
        let spaced = FinalWireEvidence::from_raw_body(spaced)?;
        let reordered = FinalWireEvidence::from_raw_body(reordered)?;
        let changed_field = FinalWireEvidence::from_raw_body(changed_field)?;

        assert_ne!(compact.raw_body_sha256, spaced.raw_body_sha256);
        assert_eq!(compact.structured_body, spaced.structured_body);
        assert_ne!(compact.structured_body, reordered.structured_body);
        assert_ne!(compact.structured_body, changed_field.structured_body);
        Ok(())
    }

    #[test]
    fn invalid_json_is_rejected() {
        let error = FinalWireEvidence::from_raw_body(b"not-json")
            .expect_err("invalid final-wire JSON must fail");

        assert!(error.to_string().contains("must be valid JSON"));
    }

    #[test]
    fn cache_snapshot_scenario_id_rejects_path_syntax() {
        let error = render_cache_snapshot("../invalid", &json!({"stable": true}))
            .expect_err("path syntax must be rejected");

        assert!(error.to_string().contains("lowercase ASCII"));
    }
}
