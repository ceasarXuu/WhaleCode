use codex_protocol::models::ResponseItem;
use codex_tools::ToolSpec;

use crate::action_map::rooted_dag::ActionOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum HostedToolKind {
    WebSearch,
    ImageGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedToolIdentity {
    pub(super) kind: HostedToolKind,
    pub(super) native_name: String,
}

impl HostedToolIdentity {
    pub(super) fn from_spec(spec: &ToolSpec) -> Option<Self> {
        let kind = match spec {
            ToolSpec::WebSearch { .. } => HostedToolKind::WebSearch,
            ToolSpec::ImageGeneration { .. } => HostedToolKind::ImageGeneration,
            _ => return None,
        };
        Some(Self {
            kind,
            native_name: spec.name().to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedToolFact {
    pub(crate) tool: String,
    pub(crate) outcome: ActionOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HostedResponseFact {
    pub(super) kind: HostedToolKind,
    pub(super) outcome: ActionOutcome,
}

pub(super) fn hosted_response_fact(
    item: &ResponseItem,
) -> Result<Option<HostedResponseFact>, String> {
    let observed = match item {
        ResponseItem::WebSearchCall { status, .. } => {
            Some((HostedToolKind::WebSearch, status.as_deref()))
        }
        ResponseItem::ImageGenerationCall { status, .. } => {
            Some((HostedToolKind::ImageGeneration, Some(status.as_str())))
        }
        _ => None,
    };
    let Some((kind, status)) = observed else {
        return Ok(None);
    };
    let outcome = match status {
        Some("completed") => ActionOutcome::Succeeded,
        Some("failed") => ActionOutcome::Failed,
        Some("cancelled" | "canceled") => ActionOutcome::Cancelled,
        _ => {
            return Err("provider-hosted response item has non-terminal status".to_string());
        }
    };
    Ok(Some(HostedResponseFact { kind, outcome }))
}

pub(super) fn merge_hosted_outcome(current: ActionOutcome, next: ActionOutcome) -> ActionOutcome {
    if hosted_outcome_rank(next) > hosted_outcome_rank(current) {
        next
    } else {
        current
    }
}

fn hosted_outcome_rank(outcome: ActionOutcome) -> u8 {
    match outcome {
        ActionOutcome::Succeeded => 3,
        ActionOutcome::Failed => 2,
        ActionOutcome::Cancelled => 1,
        ActionOutcome::Pending => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::WebSearchAction;

    #[test]
    fn spec_and_response_items_share_the_same_hosted_identity() {
        let web_spec = ToolSpec::WebSearch {
            external_web_access: Some(true),
            filters: None,
            user_location: None,
            search_context_size: None,
            search_content_types: None,
        };
        let image_spec = ToolSpec::ImageGeneration {
            output_format: "png".into(),
        };
        let web_identity = HostedToolIdentity::from_spec(&web_spec).unwrap();
        let image_identity = HostedToolIdentity::from_spec(&image_spec).unwrap();
        assert_eq!(web_identity.native_name, web_spec.name());
        assert_eq!(image_identity.native_name, image_spec.name());

        let web = hosted_response_fact(&ResponseItem::WebSearchCall {
            id: Some("ws-1".into()),
            status: Some("failed".into()),
            action: Some(WebSearchAction::Search {
                query: Some("query".into()),
                queries: None,
            }),
        })
        .unwrap()
        .unwrap();
        let image = hosted_response_fact(&ResponseItem::ImageGenerationCall {
            id: "ig-1".into(),
            status: "cancelled".into(),
            revised_prompt: None,
            result: String::new(),
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            (web.kind, web.outcome),
            (web_identity.kind, ActionOutcome::Failed)
        );
        assert_eq!(
            (image.kind, image.outcome),
            (image_identity.kind, ActionOutcome::Cancelled)
        );
    }

    #[test]
    fn hosted_outcome_ignores_failed_internal_steps_after_success() {
        assert_eq!(
            merge_hosted_outcome(ActionOutcome::Succeeded, ActionOutcome::Failed),
            ActionOutcome::Succeeded
        );
        assert_eq!(
            merge_hosted_outcome(ActionOutcome::Cancelled, ActionOutcome::Failed),
            ActionOutcome::Failed
        );
    }
}
