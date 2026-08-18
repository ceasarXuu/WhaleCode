use std::sync::Arc;

use crate::action_map::rooted_dag::TaskSpaceMap;

use super::TaskSpaceExecCatalog;
use super::TaskSpaceExecPlan;
use super::TaskSpaceExecPlanDecodeError;

#[derive(Debug, Clone)]
pub(crate) struct TaskSpaceExecRequestContext {
    map_id: String,
    request_revision: Option<u64>,
    catalog: Arc<TaskSpaceExecCatalog>,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskSpaceExecEnvelope {
    request: TaskSpaceExecRequestContext,
    outer_call_id: String,
    plan: TaskSpaceExecPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceExecInternalCallId {
    pub(crate) outer_call_id: String,
    pub(crate) index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecEnvelopeError {
    EmptyMapIdentity,
    EmptyOuterCallIdentity,
    MapIdentityChanged {
        expected: String,
        current: String,
    },
    MapRevisionChanged {
        expected: Option<u64>,
        current: Option<u64>,
    },
    CallIndexOutOfRange {
        index: usize,
        call_count: usize,
    },
    PlanDecode(TaskSpaceExecPlanDecodeError),
}

impl TaskSpaceExecRequestContext {
    #[cfg(test)]
    pub(crate) fn capture(
        map_id: impl Into<String>,
        current_map: Option<&TaskSpaceMap>,
        catalog: Arc<TaskSpaceExecCatalog>,
    ) -> Result<Self, TaskSpaceExecEnvelopeError> {
        let request_revision = current_map.map(|map| map.revision);
        Self::from_request_snapshot(map_id, request_revision, catalog)
    }

    pub(crate) fn from_request_snapshot(
        map_id: impl Into<String>,
        request_revision: Option<u64>,
        catalog: Arc<TaskSpaceExecCatalog>,
    ) -> Result<Self, TaskSpaceExecEnvelopeError> {
        let map_id = map_id.into();
        if map_id.trim().is_empty() {
            return Err(TaskSpaceExecEnvelopeError::EmptyMapIdentity);
        }
        Ok(Self {
            map_id,
            request_revision,
            catalog,
        })
    }

    pub(crate) fn map_id(&self) -> &str {
        &self.map_id
    }

    pub(crate) fn request_revision(&self) -> Option<u64> {
        self.request_revision
    }

    pub(super) fn catalog(&self) -> &TaskSpaceExecCatalog {
        &self.catalog
    }

    pub(crate) fn validate_current_map(
        &self,
        current_map: Option<&TaskSpaceMap>,
    ) -> Result<(), TaskSpaceExecEnvelopeError> {
        if let Some(current) = current_map
            && current.map_id != self.map_id
        {
            return Err(TaskSpaceExecEnvelopeError::MapIdentityChanged {
                expected: self.map_id.clone(),
                current: current.map_id.clone(),
            });
        }
        let current_revision = current_map.map(|map| map.revision);
        if current_revision != self.request_revision {
            return Err(TaskSpaceExecEnvelopeError::MapRevisionChanged {
                expected: self.request_revision,
                current: current_revision,
            });
        }
        Ok(())
    }

    pub(crate) fn decode_outer_call(
        self,
        outer_call_id: impl Into<String>,
        arguments: &str,
    ) -> Result<TaskSpaceExecEnvelope, TaskSpaceExecEnvelopeError> {
        let outer_call_id = outer_call_id.into();
        if outer_call_id.trim().is_empty() {
            return Err(TaskSpaceExecEnvelopeError::EmptyOuterCallIdentity);
        }
        let plan = self
            .catalog
            .decode_plan(arguments)
            .map_err(TaskSpaceExecEnvelopeError::PlanDecode)?;
        Ok(TaskSpaceExecEnvelope {
            request: self,
            outer_call_id,
            plan,
        })
    }
}

impl TaskSpaceExecEnvelope {
    pub(crate) fn request(&self) -> &TaskSpaceExecRequestContext {
        &self.request
    }

    #[cfg(test)]
    pub(crate) fn outer_call_id(&self) -> &str {
        &self.outer_call_id
    }

    pub(crate) fn plan(&self) -> &TaskSpaceExecPlan {
        &self.plan
    }

    pub(crate) fn internal_call_id(
        &self,
        index: usize,
    ) -> Result<TaskSpaceExecInternalCallId, TaskSpaceExecEnvelopeError> {
        if index >= self.plan.actions.len() {
            return Err(TaskSpaceExecEnvelopeError::CallIndexOutOfRange {
                index,
                call_count: self.plan.actions.len(),
            });
        }
        Ok(TaskSpaceExecInternalCallId {
            outer_call_id: self.outer_call_id.clone(),
            index,
        })
    }
}

impl TaskSpaceExecInternalCallId {
    pub(crate) fn transport_id(&self) -> String {
        format!("{}/taskspace/call/{}", self.outer_call_id, self.index)
    }
}
