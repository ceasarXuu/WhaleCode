use serde::{Deserialize, Serialize};
use whalecode_protocol::ArtifactRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_tokens: u64,
    pub reserved_output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextFragment {
    Text(String),
    Artifact(ArtifactRef),
}
