use sha2::Digest;
use sha2::Sha256;

use codex_protocol::protocol::MapRuntimeMode;

pub(crate) const TASKSPACE_CONTRACT_MANIFEST_ID: &str = "r7-taskspace-five-layer-production-v1";
pub(crate) const TASKSPACE_CONTRACT_MANIFEST_VERSION: &str = "1.0.11";
pub(crate) const TASKSPACE_CONTRACT_MANIFEST_SHA256: &str =
    "1fa11df96094a5db26002f6172e1efef4466df253c0b43feed07b272004d192e";

const TASKSPACE_CONTRACT_MANIFEST: &str =
    include_str!("prompts/taskspace_contract_manifest_v1.json");
pub(crate) const TASKSPACE_CORE_PROTOCOL_VERSION: &str = "taskspace-core-v2.6";
pub(crate) const TASKSPACE_CORE_PROTOCOL_SHA256: &str =
    "1faf64648dbf31237d1eabc0153470023b48f225df5daf9d6d313fd05e3f035d";
pub(crate) const TASKSPACE_CORE_PROTOCOL: &str =
    include_str!("prompts/taskspace_core_protocol_v2.md");

pub(crate) fn taskspace_contract_manifest_matches() -> bool {
    sha256(TASKSPACE_CONTRACT_MANIFEST) == TASKSPACE_CONTRACT_MANIFEST_SHA256
}

pub(crate) fn taskspace_core_protocol(mode: MapRuntimeMode) -> Option<&'static str> {
    match mode {
        MapRuntimeMode::Standard => None,
        MapRuntimeMode::Experiment => Some(TASKSPACE_CORE_PROTOCOL),
    }
}

pub(crate) fn taskspace_core_protocol_matches() -> bool {
    sha256(TASKSPACE_CORE_PROTOCOL) == TASKSPACE_CORE_PROTOCOL_SHA256
}

fn sha256(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn production_manifest_matches_its_identity() {
        let manifest: Value = serde_json::from_str(TASKSPACE_CONTRACT_MANIFEST)
            .expect("TaskSpace contract manifest must be valid JSON");

        assert_eq!(
            manifest["contract_id"].as_str(),
            Some(TASKSPACE_CONTRACT_MANIFEST_ID)
        );
        assert_eq!(
            manifest["manifest_version"].as_str(),
            Some(TASKSPACE_CONTRACT_MANIFEST_VERSION)
        );
        assert!(taskspace_contract_manifest_matches());
    }

    #[test]
    fn taskspace_core_protocol_is_exact_and_mode_scoped() {
        assert!(taskspace_core_protocol_matches());
        assert_eq!(taskspace_core_protocol(MapRuntimeMode::Standard), None);
        assert_eq!(
            taskspace_core_protocol(MapRuntimeMode::Experiment),
            Some(TASKSPACE_CORE_PROTOCOL)
        );
    }
}
