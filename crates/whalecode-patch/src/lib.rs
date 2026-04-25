use serde::{Deserialize, Serialize};
use whalecode_protocol::{AgentId, ArtifactId, WorkUnitId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOwnershipMode {
    Exclusive,
    AppendOnly,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOwnershipClaim {
    pub path: String,
    pub mode: FileOwnershipMode,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchArtifact {
    pub id: ArtifactId,
    pub work_unit_id: WorkUnitId,
    pub agent_id: AgentId,
    pub base_commit: String,
    pub touched_files: Vec<String>,
    pub ownership: Vec<FileOwnershipClaim>,
    pub diff: String,
}
