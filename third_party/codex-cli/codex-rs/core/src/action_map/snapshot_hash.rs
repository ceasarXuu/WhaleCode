use codex_protocol::protocol::ActionMapSnapshot;
use sha2::Digest;
use sha2::Sha256;

pub(crate) fn snapshot_sha256(snapshot: &ActionMapSnapshot) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(snapshot)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
