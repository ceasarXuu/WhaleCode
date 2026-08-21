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
