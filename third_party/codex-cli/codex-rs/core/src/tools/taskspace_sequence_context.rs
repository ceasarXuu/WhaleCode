use std::sync::Arc;

/// Runtime-only attribution carried by the TaskSpace sequence container.
///
/// Native tool schemas and payloads never contain these fields. The outer
/// sequence runtime creates this metadata from the Agent-declared container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSpaceSequenceInvocation {
    pub outer_call_id: String,
    pub item_id: String,
    pub node_id: Option<String>,
    pub work_bindings: Arc<[TaskSpaceSequenceWorkBinding]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSpaceSequenceWorkBinding {
    pub call_id: String,
    pub call_index: usize,
    pub node_id: String,
    pub tool_name: String,
}
