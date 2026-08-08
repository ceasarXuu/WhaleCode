use std::cmp::Ordering;

use codex_state::TaskSpaceMapRecord;

use super::Session;
use super::canonical_map_for_store;
use super::canonical_map_sha256;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::ActionMapStoreHandle;

#[derive(Debug, PartialEq, Eq)]
enum CacheInstallDecision {
    Install,
    SkipStale,
}

fn decide_cache_install(
    current: Option<&ActionMapStoreHandle>,
    incoming: &TaskSpaceMapRecord,
) -> Result<CacheInstallDecision, String> {
    let Some(current) = current else {
        return Ok(CacheInstallDecision::Install);
    };
    if current.map_id != incoming.map_id {
        return Err(format!(
            "TaskSpace cache binding changed from `{}` to `{}` during Store read.",
            incoming.map_id, current.map_id
        ));
    }
    match current.store_revision.cmp(&incoming.store_revision) {
        Ordering::Greater => Ok(CacheInstallDecision::SkipStale),
        Ordering::Less => Ok(CacheInstallDecision::Install),
        Ordering::Equal if current.canonical_sha256 != incoming.canonical_sha256 => Err(format!(
            "TaskSpace Store revision {} has conflicting canonical hashes.",
            incoming.store_revision
        )),
        Ordering::Equal => Ok(CacheInstallDecision::Install),
    }
}

impl Session {
    pub(in crate::session) async fn install_store_record(
        &self,
        record: &TaskSpaceMapRecord,
        candidate: ActionMapRuntimeState,
    ) -> Result<bool, String> {
        let candidate_map = canonical_map_for_store(&candidate);
        let candidate_sha256 = canonical_map_sha256(&candidate_map)
            .map_err(|error| format!("TaskSpace candidate hash failed: {error}"))?;
        if candidate_sha256 != record.canonical_sha256 {
            return Err("TaskSpace Store record does not match Runtime candidate.".to_string());
        }
        let mut state = self.state.lock().await;
        match decide_cache_install(state.action_map_store_handle.as_ref(), record)? {
            CacheInstallDecision::Install => {
                state.install_action_map_store_record(record, candidate);
                Ok(true)
            }
            CacheInstallDecision::SkipStale => {
                let current_revision = state
                    .action_map_store_handle
                    .as_ref()
                    .map_or(0, |handle| handle.store_revision);
                tracing::debug!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.map_store_stale_cache_install_skipped",
                    map_id = record.map_id,
                    incoming_store_revision = record.store_revision,
                    current_store_revision = current_revision,
                    "skipped stale canonical Map cache installation"
                );
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use codex_protocol::ThreadId;

    use super::*;

    fn handle(map_id: &str, store_revision: u64, hash: &str) -> ActionMapStoreHandle {
        ActionMapStoreHandle {
            map_id: map_id.to_string(),
            owner_thread_id: ThreadId::new(),
            store_revision,
            canonical_sha256: hash.to_string(),
        }
    }

    fn record(map_id: &str, store_revision: u64, hash: &str) -> TaskSpaceMapRecord {
        TaskSpaceMapRecord {
            map_id: map_id.to_string(),
            owner_thread_id: ThreadId::new(),
            canonical_map: None,
            canonical_sha256: hash.to_string(),
            store_revision,
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    #[test]
    fn cache_install_never_replaces_a_newer_revision() {
        let current = handle("map-1", 3, "new");
        let stale = record("map-1", 2, "old");

        assert_eq!(
            decide_cache_install(Some(&current), &stale).expect("decision"),
            CacheInstallDecision::SkipStale
        );
    }

    #[test]
    fn cache_install_rejects_same_revision_with_different_hash() {
        let current = handle("map-1", 3, "first");
        let conflicting = record("map-1", 3, "second");

        let error = decide_cache_install(Some(&current), &conflicting)
            .expect_err("same revision with another hash must fail");
        assert!(error.contains("conflicting canonical hashes"));
    }

    #[test]
    fn cache_install_accepts_newer_or_matching_records() {
        let current = handle("map-1", 3, "same");
        let matching = record("map-1", 3, "same");
        let newer = record("map-1", 4, "new");

        assert_eq!(
            decide_cache_install(Some(&current), &matching).expect("matching decision"),
            CacheInstallDecision::Install
        );
        assert_eq!(
            decide_cache_install(Some(&current), &newer).expect("newer decision"),
            CacheInstallDecision::Install
        );
    }

    #[test]
    fn cache_install_rejects_a_changed_map_binding() {
        let current = handle("map-2", 1, "current");
        let incoming = record("map-1", 2, "incoming");

        let error =
            decide_cache_install(Some(&current), &incoming).expect_err("changed binding must fail");
        assert!(error.contains("cache binding changed"));
    }

    #[tokio::test]
    async fn stale_store_read_cannot_replace_newer_session_cache() {
        let (session, _) = crate::session::tests::make_session_and_context().await;
        let hash = canonical_map_sha256(&None).expect("hash empty canonical Map");
        let newer = record("map-1", 3, &hash);
        let newer_runtime = super::super::runtime_from_record(&newer).expect("newer runtime");
        session
            .state
            .lock()
            .await
            .install_action_map_store_record(&newer, newer_runtime);

        let stale = record("map-1", 2, &hash);
        let stale_runtime = super::super::runtime_from_record(&stale).expect("stale runtime");
        let installed = session
            .install_store_record(&stale, stale_runtime)
            .await
            .expect("stale install decision");

        assert!(!installed);
        let state = session.state.lock().await;
        assert_eq!(
            state
                .action_map_store_handle
                .as_ref()
                .expect("newer handle remains")
                .store_revision,
            3
        );
    }
}
