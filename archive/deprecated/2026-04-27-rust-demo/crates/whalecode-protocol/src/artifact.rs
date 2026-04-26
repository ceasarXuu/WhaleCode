use serde::{Deserialize, Serialize};

use crate::ArtifactId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRef {
    pub name: String,
    pub version: u32,
}

pub type ArtifactSchemaRef = SchemaRef;
pub type EventSchemaRef = SchemaRef;
