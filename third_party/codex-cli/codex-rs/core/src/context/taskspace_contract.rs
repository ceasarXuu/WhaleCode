use sha2::Digest;
use sha2::Sha256;

use codex_protocol::protocol::MapRuntimeMode;

pub(crate) const TASKSPACE_CONTRACT_MANIFEST_ID: &str = "r7-taskspace-five-layer-production-v1";
pub(crate) const TASKSPACE_CONTRACT_MANIFEST_VERSION: &str = "1.0.43";
pub(crate) const TASKSPACE_CONTRACT_MANIFEST_SHA256: &str =
    "2c2147357da7a829ac07c8ac6a67d3ab20a3b1525637971b9da45627d231980c";

const TASKSPACE_CONTRACT_MANIFEST: &str =
    include_str!("prompts/taskspace_contract_manifest_v1.json");
pub(crate) const TASKSPACE_CORE_PROTOCOL_VERSION: &str = "taskspace-core-v3.5";
pub(crate) const TASKSPACE_CORE_PROTOCOL_SHA256: &str =
    "45a013c2ce2b68d78b8dc8b01c7fd84b0965342c9b1fd61336854ece23672a14";
pub(crate) const TASKSPACE_CORE_PROTOCOL: &str =
    include_str!("prompts/taskspace_core_protocol_v3.md");

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
        assert!(TASKSPACE_CORE_PROTOCOL.contains("exact provider-visible Tool name"));
        assert!(TASKSPACE_CORE_PROTOCOL.contains("at most one `apply_patch` call"));
        assert!(
            TASKSPACE_CORE_PROTOCOL
                .contains("a node completed or blocked by this transaction cannot own them")
        );
    }
}
