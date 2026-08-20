use crate::JsonSchema;
use crate::TS;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMapRuntimeModeSetParams {
    pub thread_id: String,
    pub mode: MapRuntimeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub projection_policy: Option<TaskSpaceProjectionPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMapRuntimeModeSetResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadActionMapReadParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadActionMapReadResponse {
    pub snapshot: ActionMapSnapshot,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTaskSpaceReadParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTaskSpaceReadResponse {
    pub snapshot: ActionMapSnapshot,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TaskSpaceUpdatedNotification {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub map_id: String,
    pub revision: u64,
    pub operation: String,
}
