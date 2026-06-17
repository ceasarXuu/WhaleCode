use crate::acl::revoke_ace;
use crate::deny_read_acl::apply_deny_read_acls;
use crate::deny_read_acl::lexical_path_key;
use crate::setup::sandbox_dir;
use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::ffi::c_void;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const DENY_READ_ACL_STATE_FILE: &str = "deny_read_acl_state.json";

#[derive(Default, Deserialize, Serialize)]
struct PersistentDenyReadAclState {
    principals: BTreeMap<String, Vec<PathBuf>>,
}

/// Reconciles the persistent deny-read ACEs owned by one sandbox principal.
///
/// Workspace-write and elevated sandbox sessions intentionally leave ACLs in
/// place after a command exits, because descendants may outlive the launcher.
/// That makes the ACL set stateful across runs. Persist the paths applied for
/// each SID, apply the new desired set first, and only then revoke stale paths
/// from the same SID so profile changes do not leave old deny-read ACEs behind.
///
/// # Safety
/// Caller must pass a valid SID pointer matching `principal_sid`.
pub unsafe fn sync_persistent_deny_read_acls(
    codex_home: &Path,
    principal_sid: &str,
    desired_paths: &[PathBuf],
    psid: *mut c_void,
) -> Result<Vec<PathBuf>> {
    let state_path = sandbox_dir(codex_home).join(DENY_READ_ACL_STATE_FILE);
    let mut state = load_state(&state_path)?;
    let previous_paths = state
        .principals
        .get(principal_sid)
        .cloned()
        .unwrap_or_default();

    let applied_paths = unsafe { apply_deny_read_acls(desired_paths, psid) }?;
    let desired_keys = applied_paths
        .iter()
        .map(|path| lexical_path_key(path))
        .collect::<HashSet<_>>();

    for path in previous_paths {
        if !desired_keys.contains(&lexical_path_key(&path)) {
            revoke_ace(&path, psid);
        }
    }

    if applied_paths.is_empty() {
        state.principals.remove(principal_sid);
    } else {
        state
            .principals
            .insert(principal_sid.to_string(), applied_paths.clone());
    }
    store_state(&state_path, &state)?;

    Ok(applied_paths)
}

fn load_state(path: &Path) -> Result<PersistentDenyReadAclState> {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(state) => Ok(state),
            Err(err) => recover_corrupt_state(path, &bytes)
                .with_context(|| {
                    format!(
                        "recover corrupt deny-read ACL state {} after parse error: {err}",
                        path.display()
                    )
                }),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistentDenyReadAclState::default())
        }
        Err(err) => Err(err).with_context(|| format!("read deny-read ACL state {}", path.display())),
    }
}

fn recover_corrupt_state(path: &Path, bytes: &[u8]) -> Result<PersistentDenyReadAclState> {
    let backup_path = corrupt_state_backup_path(path);
    std::fs::write(&backup_path, bytes)
        .with_context(|| format!("backup corrupt deny-read ACL state {}", backup_path.display()))?;
    Ok(PersistentDenyReadAclState::default())
}

fn corrupt_state_backup_path(path: &Path) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .map(|value| value.to_string_lossy())
        .unwrap_or_else(|| "deny_read_acl_state.json".into());
    path.with_file_name(format!("{file_name}.corrupt-{millis}"))
}

fn store_state(path: &Path, state: &PersistentDenyReadAclState) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(state).context("serialize deny-read ACL state")?;
    std::fs::write(path, bytes)
        .with_context(|| format!("write deny-read ACL state {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_state_recovers_corrupt_json_with_backup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state_path = temp.path().join(DENY_READ_ACL_STATE_FILE);
        std::fs::write(&state_path, b"{not-json").expect("write corrupt state");

        let state = load_state(&state_path).expect("corrupt state recovers");

        assert!(state.principals.is_empty());
        let backups = std::fs::read_dir(temp.path())
            .expect("read tempdir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("deny_read_acl_state.json.corrupt-")
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            std::fs::read(backups[0].path()).expect("read backup"),
            b"{not-json"
        );
    }

    #[test]
    fn store_state_replaces_corrupt_state_after_recovery() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state_path = temp.path().join(DENY_READ_ACL_STATE_FILE);
        std::fs::write(&state_path, b"{not-json").expect("write corrupt state");

        let mut state = load_state(&state_path).expect("corrupt state recovers");
        state.principals.insert(
            "principal".to_string(),
            vec![PathBuf::from(r"C:\secret.txt")],
        );
        store_state(&state_path, &state).expect("store state");
        let restored = load_state(&state_path).expect("reload stored state");

        assert_eq!(restored.principals.len(), 1);
        assert_eq!(
            restored.principals.get("principal"),
            Some(&vec![PathBuf::from(r"C:\secret.txt")])
        );
    }
}
