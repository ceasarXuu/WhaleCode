use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskspaceExecPlan {
    pub(crate) version: String,
    pub(crate) capability_id: String,
    pub(crate) calls: Vec<TaskspaceExecCall>,
    #[serde(default)]
    pub(crate) hosted_bindings: Vec<TaskspaceExecHostedBinding>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskspaceExecCall {
    pub(crate) item_id: String,
    pub(crate) tool: String,
    pub(crate) input: Value,
    #[serde(default)]
    pub(crate) node_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskspaceExecHostedBinding {
    pub(crate) tool: String,
    pub(crate) node_ids: Vec<String>,
}
