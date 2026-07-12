#![allow(dead_code)]

use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub(crate) enum TaskSpaceEventOwner {
    Root,
    Node(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskSpaceEventType {
    Message,
    Reasoning,
    LocalShellCall,
    FunctionCall,
    ToolSearchCall,
    FunctionCallOutput,
    CustomToolCall,
    CustomToolCallOutput,
    ToolSearchOutput,
    WebSearchCall,
    ImageGenerationCall,
    Compaction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TaskSpaceEvent {
    pub(crate) id: String,
    pub(crate) sequence: u64,
    pub(crate) owner: TaskSpaceEventOwner,
    pub(crate) event_type: TaskSpaceEventType,
    pub(crate) original_role: Option<String>,
    pub(crate) call_id: Option<String>,
    pub(crate) parent_call_id: Option<String>,
    pub(crate) tool_success: Option<bool>,
    pub(crate) raw_payload: Value,
    pub(crate) provider_item_id: Option<String>,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum TaskSpaceEventCodecError {
    #[error("taskspace event id must not be empty")]
    EmptyEventId,
    #[error("taskspace event sequence must be greater than zero")]
    InvalidSequence,
    #[error("taskspace node owner id must not be empty")]
    EmptyNodeOwner,
    #[error("response item type is not task context: {0}")]
    UnsupportedItem(&'static str),
    #[error("taskspace event payload is invalid: {0}")]
    InvalidPayload(String),
    #[error("taskspace event metadata does not match its raw payload: {0}")]
    MetadataMismatch(&'static str),
}

impl TaskSpaceEvent {
    pub(crate) fn from_response_item(
        id: impl Into<String>,
        sequence: u64,
        owner: TaskSpaceEventOwner,
        parent_call_id: Option<String>,
        item: &ResponseItem,
        created_at_ms: i64,
    ) -> Result<Self, TaskSpaceEventCodecError> {
        let id = id.into();
        validate_identity(&id, sequence, &owner)?;
        let event_type = event_type(item)?;
        let raw_payload = serde_json::to_value(item)
            .map_err(|error| TaskSpaceEventCodecError::InvalidPayload(error.to_string()))?;
        Ok(Self {
            id,
            sequence,
            owner,
            event_type,
            original_role: original_role(item),
            call_id: response_item_call_id(item),
            parent_call_id,
            tool_success: response_item_tool_success(item),
            raw_payload,
            provider_item_id: skipped_provider_item_id(item),
            created_at_ms,
        })
    }

    pub(crate) fn to_response_item(&self) -> Result<ResponseItem, TaskSpaceEventCodecError> {
        validate_identity(&self.id, self.sequence, &self.owner)?;
        let mut item: ResponseItem = serde_json::from_value(self.raw_payload.clone())
            .map_err(|error| TaskSpaceEventCodecError::InvalidPayload(error.to_string()))?;
        restore_provider_item_id(&mut item, self.provider_item_id.as_deref());
        restore_tool_success(&mut item, self.tool_success);
        if event_type(&item)? != self.event_type {
            return Err(TaskSpaceEventCodecError::MetadataMismatch("event_type"));
        }
        if original_role(&item) != self.original_role {
            return Err(TaskSpaceEventCodecError::MetadataMismatch("original_role"));
        }
        if response_item_call_id(&item) != self.call_id {
            return Err(TaskSpaceEventCodecError::MetadataMismatch("call_id"));
        }
        Ok(item)
    }
}

fn validate_identity(
    id: &str,
    sequence: u64,
    owner: &TaskSpaceEventOwner,
) -> Result<(), TaskSpaceEventCodecError> {
    if id.trim().is_empty() {
        return Err(TaskSpaceEventCodecError::EmptyEventId);
    }
    if sequence == 0 {
        return Err(TaskSpaceEventCodecError::InvalidSequence);
    }
    if matches!(owner, TaskSpaceEventOwner::Node(node_id) if node_id.trim().is_empty()) {
        return Err(TaskSpaceEventCodecError::EmptyNodeOwner);
    }
    Ok(())
}

fn event_type(item: &ResponseItem) -> Result<TaskSpaceEventType, TaskSpaceEventCodecError> {
    Ok(match item {
        ResponseItem::Message { .. } => TaskSpaceEventType::Message,
        ResponseItem::Reasoning { .. } => TaskSpaceEventType::Reasoning,
        ResponseItem::LocalShellCall { .. } => TaskSpaceEventType::LocalShellCall,
        ResponseItem::FunctionCall { .. } => TaskSpaceEventType::FunctionCall,
        ResponseItem::ToolSearchCall { .. } => TaskSpaceEventType::ToolSearchCall,
        ResponseItem::FunctionCallOutput { .. } => TaskSpaceEventType::FunctionCallOutput,
        ResponseItem::CustomToolCall { .. } => TaskSpaceEventType::CustomToolCall,
        ResponseItem::CustomToolCallOutput { .. } => TaskSpaceEventType::CustomToolCallOutput,
        ResponseItem::ToolSearchOutput { .. } => TaskSpaceEventType::ToolSearchOutput,
        ResponseItem::WebSearchCall { .. } => TaskSpaceEventType::WebSearchCall,
        ResponseItem::ImageGenerationCall { .. } => TaskSpaceEventType::ImageGenerationCall,
        ResponseItem::Compaction { .. } => TaskSpaceEventType::Compaction,
        ResponseItem::GhostSnapshot { .. } => {
            return Err(TaskSpaceEventCodecError::UnsupportedItem("ghost_snapshot"));
        }
        ResponseItem::Other => return Err(TaskSpaceEventCodecError::UnsupportedItem("other")),
    })
}

fn original_role(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::Message { role, .. } => Some(role.clone()),
        _ => None,
    }
}

fn response_item_call_id(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::LocalShellCall { call_id, .. }
        | ResponseItem::ToolSearchCall { call_id, .. }
        | ResponseItem::ToolSearchOutput { call_id, .. } => call_id.clone(),
        ResponseItem::FunctionCall { call_id, .. }
        | ResponseItem::FunctionCallOutput { call_id, .. }
        | ResponseItem::CustomToolCall { call_id, .. }
        | ResponseItem::CustomToolCallOutput { call_id, .. } => Some(call_id.clone()),
        _ => None,
    }
}

fn skipped_provider_item_id(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::Message { id, .. }
        | ResponseItem::LocalShellCall { id, .. }
        | ResponseItem::FunctionCall { id, .. }
        | ResponseItem::ToolSearchCall { id, .. }
        | ResponseItem::CustomToolCall { id, .. }
        | ResponseItem::WebSearchCall { id, .. } => id.clone(),
        ResponseItem::Reasoning { id, .. } => Some(id.clone()),
        _ => None,
    }
}

fn response_item_tool_success(item: &ResponseItem) -> Option<bool> {
    match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => output.success,
        _ => None,
    }
}

fn restore_provider_item_id(item: &mut ResponseItem, provider_item_id: Option<&str>) {
    match item {
        ResponseItem::Message { id, .. }
        | ResponseItem::LocalShellCall { id, .. }
        | ResponseItem::FunctionCall { id, .. }
        | ResponseItem::ToolSearchCall { id, .. }
        | ResponseItem::CustomToolCall { id, .. }
        | ResponseItem::WebSearchCall { id, .. } => {
            *id = provider_item_id.map(str::to_string);
        }
        ResponseItem::Reasoning { id, .. } => {
            *id = provider_item_id.unwrap_or_default().to_string();
        }
        _ => {}
    }
}

fn restore_tool_success(item: &mut ResponseItem, tool_success: Option<bool>) {
    match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => output.success = tool_success,
        _ => {}
    }
}

#[cfg(test)]
#[path = "event_store_tests.rs"]
mod tests;
