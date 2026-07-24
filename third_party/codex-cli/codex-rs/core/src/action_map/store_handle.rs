use codex_protocol::ThreadId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapStoreHandle {
    pub(crate) map_id: String,
    pub(crate) owner_thread_id: ThreadId,
    pub(crate) store_revision: u64,
    pub(crate) graph_revision: u64,
    pub(crate) snapshot_sha256: String,
}

impl From<&codex_state::TaskSpaceMapRecord> for ActionMapStoreHandle {
    fn from(record: &codex_state::TaskSpaceMapRecord) -> Self {
        Self {
            map_id: record.map_id.clone(),
            owner_thread_id: record.owner_thread_id,
            store_revision: record.store_revision,
            graph_revision: record.graph_revision,
            snapshot_sha256: record.snapshot_sha256.clone(),
        }
    }
}
