use codex_protocol::ThreadId;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpaceMapRecord {
    pub map_id: String,
    pub owner_thread_id: ThreadId,
    pub canonical_map: Option<TaskSpaceCanonicalMap>,
    pub canonical_sha256: String,
    pub store_revision: u64,
    pub map_revision: u64,
    pub terminal: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSpaceMapRelation {
    Owner,
    Resume,
    Fork,
    Child,
}

impl TaskSpaceMapRelation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Resume => "resume",
            Self::Fork => "fork",
            Self::Child => "child",
        }
    }

    pub(crate) fn from_str(value: &str) -> anyhow::Result<Self> {
        match value {
            "owner" => Ok(Self::Owner),
            "resume" => Ok(Self::Resume),
            "fork" => Ok(Self::Fork),
            "child" => Ok(Self::Child),
            _ => anyhow::bail!("invalid TaskSpace map relation `{value}`"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpaceMapBindingRecord {
    pub thread_id: ThreadId,
    pub map_id: String,
    pub relation: TaskSpaceMapRelation,
    pub parent_thread_id: Option<ThreadId>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskSpaceMapRequest {
    pub map_id: String,
    pub owner_thread_id: ThreadId,
    pub canonical_map: Option<TaskSpaceCanonicalMap>,
    pub commit_id: String,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindTaskSpaceMapRequest {
    pub thread_id: ThreadId,
    pub map_id: String,
    pub relation: TaskSpaceMapRelation,
    pub parent_thread_id: Option<ThreadId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitTaskSpaceMapRequest {
    pub map_id: String,
    pub expected_store_revision: u64,
    pub canonical_map: Option<TaskSpaceCanonicalMap>,
    pub commit_id: String,
    pub operation: String,
    pub actor_thread_id: ThreadId,
    pub binding: Option<BindTaskSpaceMapRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpaceMapWriteOutcome {
    Applied(TaskSpaceMapRecord),
    IdempotentReplay(TaskSpaceMapRecord),
    Conflict { current: Option<TaskSpaceMapRecord> },
}

impl TaskSpaceMapWriteOutcome {
    pub fn record(&self) -> Option<&TaskSpaceMapRecord> {
        match self {
            Self::Applied(record) | Self::IdempotentReplay(record) => Some(record),
            Self::Conflict { current } => current.as_ref(),
        }
    }
}
