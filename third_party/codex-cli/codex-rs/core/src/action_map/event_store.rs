#![allow(dead_code)]

use super::checkpoint_refs::checkpoint_output_refs;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub(crate) enum TaskSpaceEventOwner {
    Global,
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
    #[error("taskspace event sequence conflict: expected {expected}, got {actual}")]
    SequenceConflict { expected: u64, actual: u64 },
    #[error("taskspace event owner is invalid")]
    InvalidOwner,
    #[error("taskspace event type is invalid")]
    InvalidEventType,
    #[error("response item type is not task context: {0}")]
    UnsupportedItem(&'static str),
    #[error("taskspace event payload is invalid: {0}")]
    InvalidPayload(String),
    #[error("taskspace event metadata does not match its raw payload: {0}")]
    MetadataMismatch(&'static str),
    #[error("taskspace compaction checkpoint is invalid: {0}")]
    InvalidCheckpoint(String),
    #[error("taskspace compaction checkpoint hash mismatch")]
    CheckpointHashMismatch,
}

const TASKSPACE_CHECKPOINT_SCHEMA: &str = "TaskSpaceCompactionCheckpointV1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TaskSpaceCompactionCheckpoint {
    schema_version: String,
    checkpoint_id: String,
    covered_sequence_start: u64,
    covered_sequence_end: u64,
    covered_event_count: usize,
    covered_events_sha256: String,
    output_refs: Vec<String>,
    omission_reason: String,
    replacement_items: Vec<ResponseItem>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TaskSpaceEventStore {
    events: Vec<TaskSpaceEvent>,
    call_owners: HashMap<String, TaskSpaceEventOwner>,
    next_sequence: u64,
}

impl TaskSpaceEventStore {
    pub(crate) fn new() -> Self {
        Self {
            events: Vec::new(),
            call_owners: HashMap::new(),
            next_sequence: 1,
        }
    }

    pub(crate) fn events(&self) -> &[TaskSpaceEvent] {
        &self.events
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(crate) fn event_id_for_call(&self, call_id: &str) -> Option<String> {
        self.events
            .iter()
            .find(|event| event.call_id.as_deref() == Some(call_id))
            .map(|event| event.id.clone())
    }

    pub(crate) fn initialization_source_event_ids(&self, call_id: &str) -> Vec<String> {
        let call_event = self
            .events
            .iter()
            .find(|event| event.call_id.as_deref() == Some(call_id));
        let Some(call_event) = call_event else {
            return Vec::new();
        };
        let mut source_event_ids = self
            .events
            .iter()
            .rev()
            .find(|event| {
                event.sequence < call_event.sequence
                    && event.original_role.as_deref() == Some("user")
            })
            .map(|event| vec![event.id.clone()])
            .unwrap_or_default();
        source_event_ids.push(call_event.id.clone());
        source_event_ids
    }

    pub(crate) fn record_item(
        &mut self,
        item: &ResponseItem,
        current_node_id: Option<&str>,
        parent_call_id: Option<String>,
        created_at_ms: i64,
    ) -> Result<TaskSpaceEvent, TaskSpaceEventCodecError> {
        let owner = self.owner_for_item(item, current_node_id);
        let sequence = self.next_sequence;
        let event = TaskSpaceEvent::from_response_item(
            format!("task-event-{sequence}"),
            sequence,
            owner.clone(),
            parent_call_id,
            item,
            created_at_ms,
        )?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        if is_call_item(item)
            && let Some(call_id) = event.call_id.as_ref()
        {
            self.call_owners.insert(call_id.clone(), owner);
        }
        self.events.push(event.clone());
        Ok(event)
    }

    pub(crate) fn install_compaction_checkpoint(
        &mut self,
        replacement_items: Vec<ResponseItem>,
        created_at_ms: i64,
    ) -> Result<TaskSpaceEvent, TaskSpaceEventCodecError> {
        let first = self.events.first().ok_or_else(|| {
            TaskSpaceEventCodecError::InvalidCheckpoint("no events to compact".to_string())
        })?;
        let last = self.events.last().expect("checked non-empty event store");
        let replacement_items = replacement_items
            .into_iter()
            .filter(|item| !is_taskspace_runtime_context_item(item))
            .collect::<Vec<_>>();
        let checkpoint = TaskSpaceCompactionCheckpoint {
            schema_version: TASKSPACE_CHECKPOINT_SCHEMA.to_string(),
            checkpoint_id: format!("task-checkpoint-{}", self.next_sequence),
            covered_sequence_start: first.sequence,
            covered_sequence_end: last.sequence,
            covered_event_count: self.events.len(),
            covered_events_sha256: checkpoint_hash(&self.events)?,
            output_refs: checkpoint_output_refs(&self.events),
            omission_reason: "context_compaction".to_string(),
            replacement_items,
        };
        let item = ResponseItem::Compaction {
            encrypted_content: serde_json::to_string(&checkpoint)
                .map_err(|error| TaskSpaceEventCodecError::InvalidCheckpoint(error.to_string()))?,
        };
        self.record_item(&item, None, None, created_at_ms)
    }

    pub(crate) fn restore(events: Vec<TaskSpaceEvent>) -> Result<Self, TaskSpaceEventCodecError> {
        let mut store = Self::new();
        for event in events {
            if event.sequence != store.next_sequence {
                return Err(TaskSpaceEventCodecError::SequenceConflict {
                    expected: store.next_sequence,
                    actual: event.sequence,
                });
            }
            let item = event.to_response_item()?;
            if let Some(checkpoint) = checkpoint_from_item(&item)? {
                validate_checkpoint(&checkpoint, &store.events, event.sequence)?;
            }
            if is_call_item(&item)
                && let Some(call_id) = event.call_id.as_ref()
            {
                store
                    .call_owners
                    .insert(call_id.clone(), event.owner.clone());
            }
            store.next_sequence = store.next_sequence.saturating_add(1);
            store.events.push(event);
        }
        Ok(store)
    }

    pub(crate) fn linearize(&self) -> Vec<ResponseItem> {
        let checkpoint = self
            .events
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, event)| {
                let item = event
                    .to_response_item()
                    .expect("TaskSpaceEventStore only contains validated events");
                checkpoint_from_item(&item)
                    .expect("TaskSpaceEventStore only contains valid checkpoints")
                    .map(|checkpoint| (index, checkpoint))
            });
        let (mut items, suffix_start) = match checkpoint {
            Some((index, checkpoint)) => {
                let mut items = vec![checkpoint_context_item(&checkpoint)];
                items.extend(checkpoint.replacement_items);
                (items, index + 1)
            }
            None => (Vec::new(), 0),
        };
        items.extend(
            self.events[suffix_start..]
                .iter()
                .map(|event| {
                    event
                        .to_response_item()
                        .expect("TaskSpaceEventStore only contains validated events")
                })
                .filter(|item| {
                    checkpoint_from_item(item)
                        .expect("TaskSpaceEventStore only contains valid checkpoints")
                        .is_none()
                }),
        );
        items
    }

    pub(crate) fn take_linearized(&mut self) -> Vec<ResponseItem> {
        let items = self
            .linearize()
            .into_iter()
            .filter(|item| !is_taskspace_runtime_context_item(item))
            .collect();
        *self = Self::new();
        items
    }

    pub(crate) fn drop_last_n_user_turns(&mut self, num_turns: u32) {
        if num_turns == 0 {
            return;
        }
        let user_indices = self
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| {
                event
                    .to_response_item()
                    .ok()
                    .filter(crate::context_manager::is_user_turn_boundary)
                    .map(|_| index)
            })
            .collect::<Vec<_>>();
        let drop_count = usize::try_from(num_turns).unwrap_or(usize::MAX);
        let cutoff = user_indices
            .len()
            .checked_sub(drop_count)
            .and_then(|index| user_indices.get(index).copied())
            .or_else(|| user_indices.first().copied());
        if let Some(cutoff) = cutoff {
            self.events.truncate(cutoff);
            self.rebuild_indexes();
        }
    }

    fn owner_for_item(
        &self,
        item: &ResponseItem,
        current_node_id: Option<&str>,
    ) -> TaskSpaceEventOwner {
        if let Some(call_id) = output_call_id(item)
            && let Some(owner) = self.call_owners.get(call_id)
        {
            return owner.clone();
        }
        if matches!(item, ResponseItem::Message { role, .. } if role == "developer" || role == "system")
        {
            return TaskSpaceEventOwner::Global;
        }
        current_node_id
            .map(|node_id| TaskSpaceEventOwner::Node(node_id.to_string()))
            .unwrap_or(TaskSpaceEventOwner::Root)
    }

    fn rebuild_indexes(&mut self) {
        self.call_owners.clear();
        for event in &self.events {
            if matches!(
                event.event_type,
                TaskSpaceEventType::LocalShellCall
                    | TaskSpaceEventType::FunctionCall
                    | TaskSpaceEventType::ToolSearchCall
                    | TaskSpaceEventType::CustomToolCall
            ) && let Some(call_id) = event.call_id.as_ref()
            {
                self.call_owners
                    .insert(call_id.clone(), event.owner.clone());
            }
        }
        self.next_sequence = self
            .events
            .last()
            .map(|event| event.sequence.saturating_add(1))
            .unwrap_or(1);
    }
}

fn checkpoint_from_item(
    item: &ResponseItem,
) -> Result<Option<TaskSpaceCompactionCheckpoint>, TaskSpaceEventCodecError> {
    let ResponseItem::Compaction { encrypted_content } = item else {
        return Ok(None);
    };
    let Ok(value) = serde_json::from_str::<Value>(encrypted_content) else {
        return Ok(None);
    };
    if value.get("schema_version").and_then(Value::as_str) != Some(TASKSPACE_CHECKPOINT_SCHEMA) {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|error| TaskSpaceEventCodecError::InvalidCheckpoint(error.to_string()))
}

fn checkpoint_hash(events: &[TaskSpaceEvent]) -> Result<String, TaskSpaceEventCodecError> {
    let bytes = serde_json::to_vec(events)
        .map_err(|error| TaskSpaceEventCodecError::InvalidCheckpoint(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_checkpoint(
    checkpoint: &TaskSpaceCompactionCheckpoint,
    preceding_events: &[TaskSpaceEvent],
    checkpoint_sequence: u64,
) -> Result<(), TaskSpaceEventCodecError> {
    if checkpoint.schema_version != TASKSPACE_CHECKPOINT_SCHEMA
        || checkpoint.omission_reason != "context_compaction"
        || checkpoint.covered_sequence_start == 0
        || checkpoint.covered_sequence_start > checkpoint.covered_sequence_end
        || checkpoint.covered_sequence_end >= checkpoint_sequence
    {
        return Err(TaskSpaceEventCodecError::InvalidCheckpoint(
            "invalid schema, range, or omission reason".to_string(),
        ));
    }
    let covered = preceding_events
        .iter()
        .filter(|event| {
            event.sequence >= checkpoint.covered_sequence_start
                && event.sequence <= checkpoint.covered_sequence_end
        })
        .cloned()
        .collect::<Vec<_>>();
    if covered.len() != checkpoint.covered_event_count
        || covered.first().map(|event| event.sequence) != Some(checkpoint.covered_sequence_start)
        || covered.last().map(|event| event.sequence) != Some(checkpoint.covered_sequence_end)
    {
        return Err(TaskSpaceEventCodecError::InvalidCheckpoint(
            "covered event range is incomplete".to_string(),
        ));
    }
    if checkpoint_hash(&covered)? != checkpoint.covered_events_sha256 {
        return Err(TaskSpaceEventCodecError::CheckpointHashMismatch);
    }
    Ok(())
}

fn checkpoint_context_item(checkpoint: &TaskSpaceCompactionCheckpoint) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![codex_protocol::models::ContentItem::InputText {
            text: format!(
                "TaskSpaceCompactionCheckpointV1:\ncheckpoint_id: {}\ncovered_sequence_range: {}-{}\ncovered_event_count: {}\ncovered_events_sha256: {}\noutput_refs: {}\nomission_reason: {}",
                checkpoint.checkpoint_id,
                checkpoint.covered_sequence_start,
                checkpoint.covered_sequence_end,
                checkpoint.covered_event_count,
                checkpoint.covered_events_sha256,
                if checkpoint.output_refs.is_empty() {
                    "none".to_string()
                } else {
                    checkpoint.output_refs.join(",")
                },
                checkpoint.omission_reason,
            ),
        }],
        end_turn: None,
        phase: None,
    }
}

pub(super) fn output_call_id(item: &ResponseItem) -> Option<&str> {
    match item {
        ResponseItem::FunctionCallOutput { call_id, .. }
        | ResponseItem::CustomToolCallOutput { call_id, .. } => Some(call_id),
        ResponseItem::ToolSearchOutput { call_id, .. } => call_id.as_deref(),
        _ => None,
    }
}

pub(super) fn is_call_item(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::CustomToolCall { .. }
    )
}

pub(super) fn is_taskspace_runtime_context_item(item: &ResponseItem) -> bool {
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };
    if role != "developer" && role != "system" {
        return false;
    }
    content.iter().any(|content| {
        let text = match content {
            codex_protocol::models::ContentItem::InputText { text }
            | codex_protocol::models::ContentItem::OutputText { text } => text,
            codex_protocol::models::ContentItem::InputImage { .. } => return false,
        };
        text.contains("TaskSpaceAgentContextBundleV1:")
            || text.contains("ContextProjectionV1 epoch snapshot")
            || text.contains("TaskSpace mode is now active.")
    })
}

#[cfg(test)]
#[path = "event_store_tests.rs"]
mod tests;
