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
pub(crate) struct HostedOutputFact {
    pub(crate) output_index: usize,
    pub(crate) provider_id: String,
    pub(crate) tool: String,
    pub(crate) outcome: ActionOutcome,
}

pub(super) fn hosted_output_fact(
    output_index: Option<usize>,
    item: &ResponseItem,
) -> Result<Option<HostedOutputFact>, String> {
    let observed = match item {
        ResponseItem::WebSearchCall { id, status, .. } => Some((
            HostedToolKind::WebSearch,
            id.clone().unwrap_or_default(),
            status.as_deref(),
        )),
        ResponseItem::ImageGenerationCall { id, status, .. } => Some((
            HostedToolKind::ImageGeneration,
            id.clone(),
            Some(status.as_str()),
        )),
        _ => None,
    };
    let Some((kind, provider_id, status)) = observed else {
        return Ok(None);
    };
    let output_index =
        output_index.ok_or_else(|| "provider-hosted output is missing output_index".to_string())?;
    let outcome = match status {
        Some("completed") => ActionOutcome::Succeeded,
        Some("failed") => ActionOutcome::Failed,
        Some("cancelled" | "canceled") => ActionOutcome::Cancelled,
        _ => {
            return Err(format!(
                "provider-hosted output {output_index} has non-terminal status"
            ));
        }
    };
    Ok(Some(HostedOutputFact {
        output_index,
        provider_id,
        tool: kind.name().to_string(),
        outcome,
    }))
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

        let web = hosted_output_fact(
            Some(1),
            &ResponseItem::WebSearchCall {
                id: Some("ws-1".into()),
                status: Some("failed".into()),
                action: Some(WebSearchAction::Search {
                    query: Some("query".into()),
                    queries: None,
                }),
            },
        )
        .unwrap()
        .unwrap();
        let image = hosted_output_fact(
            Some(2),
            &ResponseItem::ImageGenerationCall {
                id: "ig-1".into(),
                status: "cancelled".into(),
                revised_prompt: None,
                result: String::new(),
            },
        )
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
}
