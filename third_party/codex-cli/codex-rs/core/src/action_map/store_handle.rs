use codex_protocol::ThreadId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapStoreHandle {
    pub(crate) map_id: String,
    pub(crate) owner_thread_id: ThreadId,
    pub(crate) store_revision: u64,
    pub(crate) canonical_sha256: String,
}

impl From<&codex_state::TaskSpaceMapRecord> for ActionMapStoreHandle {
    fn from(record: &codex_state::TaskSpaceMapRecord) -> Self {
        Self {
            map_id: record.map_id.clone(),
            owner_thread_id: record.owner_thread_id,
            store_revision: record.store_revision,
            canonical_sha256: record.canonical_sha256.clone(),
        }
    }
}
