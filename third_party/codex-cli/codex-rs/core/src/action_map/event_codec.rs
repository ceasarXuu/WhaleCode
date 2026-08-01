use super::event_store::TaskSpaceEvent;
use super::event_store::TaskSpaceEventCodecError;
use super::event_store::TaskSpaceEventOwner;
use super::event_store::TaskSpaceEventType;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MapRuntimeTaskContextEventRecordedEvent;

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

    pub(crate) fn to_protocol(&self) -> MapRuntimeTaskContextEventRecordedEvent {
        let (owner_kind, owner_id) = match &self.owner {
            TaskSpaceEventOwner::Global => ("global", None),
            TaskSpaceEventOwner::Root => ("root", None),
            TaskSpaceEventOwner::Node(node_id) => ("node", Some(node_id.clone())),
        };
        MapRuntimeTaskContextEventRecordedEvent {
            id: self.id.clone(),
            sequence: self.sequence,
            owner_kind: owner_kind.to_string(),
            owner_id,
            event_type: self.event_type.as_str().to_string(),
            original_role: self.original_role.clone(),
            call_id: self.call_id.clone(),
            parent_call_id: self.parent_call_id.clone(),
            tool_success: self.tool_success,
            raw_payload: self.raw_payload.clone(),
            provider_item_id: self.provider_item_id.clone(),
            created_at_ms: self.created_at_ms,
        }
    }

    pub(crate) fn from_protocol(
        event: MapRuntimeTaskContextEventRecordedEvent,
    ) -> Result<Self, TaskSpaceEventCodecError> {
        let owner = match (event.owner_kind.as_str(), event.owner_id) {
            ("global", None) => TaskSpaceEventOwner::Global,
            ("root", None) => TaskSpaceEventOwner::Root,
            ("node", Some(node_id)) if !node_id.trim().is_empty() => {
                TaskSpaceEventOwner::Node(node_id)
            }
            _ => return Err(TaskSpaceEventCodecError::InvalidOwner),
        };
        let result = Self {
            id: event.id,
            sequence: event.sequence,
            owner,
            event_type: TaskSpaceEventType::from_str(&event.event_type)
                .ok_or(TaskSpaceEventCodecError::InvalidEventType)?,
            original_role: event.original_role,
            call_id: event.call_id,
            parent_call_id: event.parent_call_id,
            tool_success: event.tool_success,
            raw_payload: event.raw_payload,
            provider_item_id: event.provider_item_id,
            created_at_ms: event.created_at_ms,
        };
        result.to_response_item()?;
        Ok(result)
    }
}

impl TaskSpaceEventType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Reasoning => "reasoning",
            Self::LocalShellCall => "local_shell_call",
            Self::FunctionCall => "function_call",
            Self::ToolSearchCall => "tool_search_call",
            Self::FunctionCallOutput => "function_call_output",
            Self::CustomToolCall => "custom_tool_call",
            Self::CustomToolCallOutput => "custom_tool_call_output",
            Self::ToolSearchOutput => "tool_search_output",
            Self::WebSearchCall => "web_search_call",
            Self::ImageGenerationCall => "image_generation_call",
            Self::Compaction => "compaction",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "message" => Self::Message,
            "reasoning" => Self::Reasoning,
            "local_shell_call" => Self::LocalShellCall,
            "function_call" => Self::FunctionCall,
            "tool_search_call" => Self::ToolSearchCall,
            "function_call_output" => Self::FunctionCallOutput,
            "custom_tool_call" => Self::CustomToolCall,
            "custom_tool_call_output" => Self::CustomToolCallOutput,
            "tool_search_output" => Self::ToolSearchOutput,
            "web_search_call" => Self::WebSearchCall,
            "image_generation_call" => Self::ImageGenerationCall,
            "compaction" => Self::Compaction,
            _ => return None,
        })
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
