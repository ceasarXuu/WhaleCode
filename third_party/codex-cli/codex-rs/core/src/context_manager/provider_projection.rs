use super::normalize;
use codex_history::ResponseItemEnvelope;
use codex_protocol::models::AgentMessageInputContent;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::InputModality;
use std::collections::HashSet;

/// Builds a request-only history view for the target provider.
///
/// Canonical history can contain opaque OpenAI state that another provider cannot interpret.
/// Projection owns its input, so filtering remains reversible when a later turn switches back.
pub(crate) fn project_history_for_provider(
    items: Vec<ResponseItem>,
    preserves_opaque_history: bool,
    input_modalities: &[InputModality],
) -> Vec<ResponseItem> {
    if preserves_opaque_history {
        return items;
    }

    let private_output_calls = private_only_output_call_ids(&items);
    let mut projected = items
        .into_iter()
        .filter_map(|mut item| {
            if should_drop_item(&item, &private_output_calls) {
                return None;
            }
            sanitize_item(&mut item);
            should_keep_sanitized_item(&item).then_some(item)
        })
        .map(ResponseItemEnvelope::new)
        .collect::<Vec<_>>();

    normalize::ensure_call_outputs_present(&mut projected);
    normalize::remove_orphan_outputs(&mut projected);
    normalize::strip_images_when_unsupported(input_modalities, &mut projected);
    normalize::strip_audio_when_unsupported(input_modalities, &mut projected);
    projected
        .into_iter()
        .map(|envelope| envelope.item)
        .collect()
}

fn private_only_output_call_ids(items: &[ResponseItem]) -> HashSet<String> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::FunctionCallOutput {
                call_id, output, ..
            } if output_is_private_only(&output.body) => call_id.clone(),
            ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } if output_is_private_only(&output.body) => Some(call_id.clone()),
            _ => None,
        })
        .collect()
}

fn output_is_private_only(body: &FunctionCallOutputBody) -> bool {
    matches!(
        body,
        FunctionCallOutputBody::ContentItems(content)
            if !content.is_empty()
                && content.iter().all(|part| matches!(
                    part,
                    FunctionCallOutputContentItem::EncryptedContent { .. }
                ))
    )
}

fn should_drop_item(item: &ResponseItem, private_output_calls: &HashSet<String>) -> bool {
    match item {
        ResponseItem::FunctionCall { call_id, .. } => private_output_calls.contains(call_id),
        ResponseItem::FunctionCallOutput { call_id, .. } => call_id
            .as_ref()
            .is_some_and(|call_id| private_output_calls.contains(call_id)),
        ResponseItem::CustomToolCall { call_id, .. }
        | ResponseItem::CustomToolCallOutput { call_id, .. } => {
            private_output_calls.contains(call_id)
        }
        ResponseItem::LocalShellCall {
            call_id: Some(call_id),
            ..
        } => private_output_calls.contains(call_id.as_str()),
        ResponseItem::ToolSearchCall { execution, .. }
        | ResponseItem::ToolSearchOutput { execution, .. } => execution == "server",
        ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::CompactionTrigger { .. }
        | ResponseItem::ContextCompaction { .. }
        | ResponseItem::Other => true,
        _ => false,
    }
}

fn sanitize_item(item: &mut ResponseItem) {
    item.clear_internal_chat_message_metadata_passthrough();
    match item {
        ResponseItem::AgentMessage { content, .. } => {
            content.retain(|part| matches!(part, AgentMessageInputContent::InputText { .. }));
        }
        ResponseItem::Reasoning {
            encrypted_content, ..
        } => *encrypted_content = None,
        ResponseItem::FunctionCall {
            encrypted_function_args,
            ..
        } => *encrypted_function_args = None,
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => {
            if let FunctionCallOutputBody::ContentItems(content) = &mut output.body {
                content.retain(|part| {
                    !matches!(part, FunctionCallOutputContentItem::EncryptedContent { .. })
                });
            }
        }
        _ => {}
    }
}

fn should_keep_sanitized_item(item: &ResponseItem) -> bool {
    match item {
        ResponseItem::AgentMessage { content, .. } => !content.is_empty(),
        ResponseItem::Reasoning {
            summary,
            content,
            encrypted_content,
            ..
        } => {
            !summary.is_empty()
                || content.as_ref().is_some_and(|content| !content.is_empty())
                || encrypted_content.is_some()
        }
        _ => true,
    }
}

#[cfg(test)]
#[path = "provider_projection_tests.rs"]
mod tests;
