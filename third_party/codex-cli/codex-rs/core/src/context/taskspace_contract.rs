use sha2::Digest;
use sha2::Sha256;

pub(crate) const TASKSPACE_CONTRACT_MANIFEST_ID: &str = "r7-taskspace-five-layer-production-v1";
pub(crate) const TASKSPACE_CONTRACT_MANIFEST_VERSION: &str = "1.0.0";
pub(crate) const TASKSPACE_CONTRACT_MANIFEST_SHA256: &str =
    "b698d5c8ed8ef74790252ecc2c452034bdd31137c99a80478238325721a9a095";

const TASKSPACE_CONTRACT_MANIFEST: &str =
    include_str!("prompts/taskspace_contract_manifest_v1.json");

pub(crate) fn taskspace_contract_manifest_matches() -> bool {
    sha256(TASKSPACE_CONTRACT_MANIFEST) == TASKSPACE_CONTRACT_MANIFEST_SHA256
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
}
