use codex_protocol::models::ResponseItem;
use codex_tools::ToolSpec;

use crate::action_map::rooted_dag::ActionOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum HostedToolKind {
    WebSearch,
    ImageGeneration,
}

impl HostedToolKind {
    pub(super) fn from_spec(spec: &ToolSpec) -> Option<Self> {
        match spec {
            ToolSpec::WebSearch { .. } => Some(Self::WebSearch),
            ToolSpec::ImageGeneration { .. } => Some(Self::ImageGeneration),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::WebSearch => "web_search",
            Self::ImageGeneration => "image_generation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedToolFact {
    pub(crate) tool: String,
    pub(crate) outcome: ActionOutcome,
}

pub(super) fn hosted_tool_fact(item: &ResponseItem) -> Result<Option<HostedToolFact>, String> {
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
            return Err(format!(
                "provider-hosted Tool `{}` has non-terminal status",
                kind.name()
            ));
        }
    };
    Ok(Some(HostedToolFact {
        tool: kind.name().to_string(),
        outcome,
    }))
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
        assert_eq!(
            HostedToolKind::from_spec(&web_spec).map(HostedToolKind::name),
            Some("web_search")
        );
        assert_eq!(
            HostedToolKind::from_spec(&image_spec).map(HostedToolKind::name),
            Some("image_generation")
        );

        let web = hosted_tool_fact(&ResponseItem::WebSearchCall {
            id: Some("ws-1".into()),
            status: Some("failed".into()),
            action: Some(WebSearchAction::Search {
                query: Some("query".into()),
                queries: None,
            }),
        })
        .unwrap()
        .unwrap();
        let image = hosted_tool_fact(&ResponseItem::ImageGenerationCall {
            id: "ig-1".into(),
            status: "cancelled".into(),
            revised_prompt: None,
            result: String::new(),
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            (web.tool.as_str(), web.outcome),
            ("web_search", ActionOutcome::Failed)
        );
        assert_eq!(
            (image.tool.as_str(), image.outcome),
            ("image_generation", ActionOutcome::Cancelled)
        );
    }

    #[test]
    fn logical_hosted_outcome_ignores_failed_internal_steps_after_success() {
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
