use serde::{Deserialize, Serialize};
use whalecode_protocol::{AgentId, AgentRole};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Busy,
    Blocked,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRuntime {
    pub id: AgentId,
    pub role: AgentRole,
    pub state: AgentState,
}

impl AgentRuntime {
    pub fn new(id: AgentId, role: AgentRole) -> Self {
        Self {
            id,
            role,
            state: AgentState::Idle,
        }
    }
}
