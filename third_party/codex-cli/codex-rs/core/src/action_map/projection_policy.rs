use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use sha2::Digest;
use sha2::Sha256;

use super::projection::TASKSPACE_MAP_PROJECTION_END;
use super::projection::TASKSPACE_MAP_PROJECTION_MARKER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionTrigger {
    ProviderRequest {
        projection_is_current_tail: bool,
    },
    #[allow(dead_code)]
    ExplicitRead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionIdentity {
    pub(crate) map_id: Option<String>,
    pub(crate) revision: Option<u64>,
    pub(crate) canonical_sha256: Option<String>,
    pub(crate) projection_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProjectionCursor {
    pub(crate) last_emitted: Option<ProjectionIdentity>,
}

impl ProjectionCursor {
    pub(crate) fn from_items(items: &[ResponseItem]) -> Self {
        let last_emitted = items.iter().rev().find_map(|item| {
            let ResponseItem::Message { role, content, .. } = item else {
                return None;
            };
            if role != "developer" && role != "system" && role != "user" {
                return None;
            }
            content.iter().rev().find_map(|entry| match entry {
                ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                    projection_identity_from_context(text)
                }
                ContentItem::InputImage { .. } | ContentItem::InputAudio { .. } => None,
            })
        });
        Self { last_emitted }
    }
}

pub(crate) fn projection_identity_from_context(context: &str) -> Option<ProjectionIdentity> {
    let start = context.rfind(TASKSPACE_MAP_PROJECTION_MARKER)?;
    let candidate = &context[start..];
    let end = candidate.find(TASKSPACE_MAP_PROJECTION_END)? + TASKSPACE_MAP_PROJECTION_END.len();
    let block = &candidate[..end];
    let map_id = projection_field(block, "map_id").map(str::to_string);
    let revision = projection_field(block, "revision").and_then(|value| value.parse().ok());
    let canonical_sha256 = projection_field(block, "canonical_sha256").map(str::to_string);
    Some(ProjectionIdentity {
        map_id,
        revision,
        canonical_sha256,
        projection_sha256: format!("{:x}", Sha256::digest(block.as_bytes())),
    })
}

fn projection_field<'a>(projection: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("- {field}: ");
    projection
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionEmission {
    None,
    ReplaceLatest,
    AppendSnapshot,
    ReturnAsToolResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionDecision {
    pub(crate) emission: ProjectionEmission,
    pub(crate) next_cursor: ProjectionCursor,
}

pub(crate) fn decide_projection_emission(
    policy: TaskSpaceProjectionPolicy,
    trigger: ProjectionTrigger,
    cursor: &ProjectionCursor,
    candidate: Option<&ProjectionIdentity>,
) -> Result<ProjectionDecision, String> {
    let (emission, next_cursor) = match policy {
        TaskSpaceProjectionPolicy::MapAlways => match trigger {
            ProjectionTrigger::ProviderRequest { .. } => {
                (ProjectionEmission::ReplaceLatest, cursor.clone())
            }
            ProjectionTrigger::ExplicitRead => {
                (ProjectionEmission::ReturnAsToolResult, cursor.clone())
            }
        },
        TaskSpaceProjectionPolicy::MapAppend => match trigger {
            ProjectionTrigger::ExplicitRead => {
                (ProjectionEmission::ReturnAsToolResult, cursor.clone())
            }
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail,
            } => {
                let candidate = candidate.ok_or_else(|| {
                    format!("projection trigger `{trigger:?}` requires a rendered candidate")
                })?;
                if projection_is_current_tail {
                    (ProjectionEmission::None, cursor.clone())
                } else {
                    if let Some(last) = cursor.last_emitted.as_ref() {
                        match (
                            last.map_id.as_deref(),
                            last.revision,
                            candidate.map_id.as_deref(),
                            candidate.revision,
                        ) {
                            (Some(last_map), Some(last_revision), Some(map), Some(revision))
                                if last_map == map && last_revision > revision =>
                            {
                                return Err(format!(
                                    "append projection revision {revision} is older than emitted revision {last_revision}"
                                ));
                            }
                            (Some(last_map), _, None, None) => {
                                return Err(format!(
                                    "append projection cannot return to bootstrap after map `{last_map}`"
                                ));
                            }
                            _ => {}
                        }
                    }
                    (
                        ProjectionEmission::AppendSnapshot,
                        ProjectionCursor {
                            last_emitted: Some(candidate.clone()),
                        },
                    )
                }
            }
        },
        TaskSpaceProjectionPolicy::MapRequest => match trigger {
            ProjectionTrigger::ProviderRequest { .. } => (ProjectionEmission::None, cursor.clone()),
            ProjectionTrigger::ExplicitRead => {
                (ProjectionEmission::ReturnAsToolResult, cursor.clone())
            }
        },
    };
    Ok(ProjectionDecision {
        emission,
        next_cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::ContentItem;
    use codex_protocol::models::ResponseItem;

    #[test]
    fn map_always_replaces_on_every_provider_request() {
        let cursor = ProjectionCursor::default();
        let decision = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAlways,
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail: false,
            },
            &cursor,
            None,
        )
        .expect("map-always must be enabled");
        assert_eq!(decision.emission, ProjectionEmission::ReplaceLatest);
        assert_eq!(decision.next_cursor, cursor);
    }

    #[test]
    fn map_append_emits_on_each_request_after_new_history_and_rejects_regression() {
        let revision_3 = ProjectionIdentity {
            map_id: Some("map-1".into()),
            revision: Some(3),
            canonical_sha256: Some("canonical-3".into()),
            projection_sha256: "projection-3".into(),
        };
        let first = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail: false,
            },
            &ProjectionCursor::default(),
            Some(&revision_3),
        )
        .expect("first provider request should append");
        assert_eq!(first.emission, ProjectionEmission::AppendSnapshot);

        let same_revision_after_new_history = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail: false,
            },
            &first.next_cursor,
            Some(&revision_3),
        )
        .expect("same revision should append after new history");
        assert_eq!(
            same_revision_after_new_history.emission,
            ProjectionEmission::AppendSnapshot
        );

        let retry = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail: true,
            },
            &same_revision_after_new_history.next_cursor,
            Some(&revision_3),
        )
        .expect("provider retry should recognize the current tail");
        assert_eq!(retry.emission, ProjectionEmission::None);

        let revision_2 = ProjectionIdentity {
            revision: Some(2),
            ..revision_3
        };
        assert!(
            decide_projection_emission(
                TaskSpaceProjectionPolicy::MapAppend,
                ProjectionTrigger::ProviderRequest {
                    projection_is_current_tail: false,
                },
                &first.next_cursor,
                Some(&revision_2),
            )
            .is_err()
        );
    }

    #[test]
    fn map_append_provider_request_requires_a_rendered_candidate() {
        let decision = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail: false,
            },
            &ProjectionCursor::default(),
            None,
        );
        assert!(decision.is_err());
    }

    #[test]
    fn map_request_emits_only_for_explicit_read() {
        let cursor = ProjectionCursor::default();
        let provider = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapRequest,
            ProjectionTrigger::ProviderRequest {
                projection_is_current_tail: false,
            },
            &cursor,
            None,
        );
        assert_eq!(
            provider.expect("provider decision").emission,
            ProjectionEmission::None
        );

        let read = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapRequest,
            ProjectionTrigger::ExplicitRead,
            &cursor,
            None,
        );
        assert_eq!(
            read.expect("explicit read decision").emission,
            ProjectionEmission::ReturnAsToolResult
        );
    }

    #[test]
    fn cursor_restores_the_last_visible_projection_identity() {
        let item = ResponseItem::Message {
            id: None,
            role: "developer".into(),
            content: vec![ContentItem::InputText {
                text: format!(
                    "{TASKSPACE_MAP_PROJECTION_MARKER}\n- projection_kind: request_snapshot\n- map_id: map-1\n- revision: 4\n- canonical_sha256: canonical-4\n{TASKSPACE_MAP_PROJECTION_END}\n"
                ),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        };

        let cursor = ProjectionCursor::from_items(&[item]);
        let identity = cursor.last_emitted.expect("projection identity");
        assert_eq!(identity.map_id.as_deref(), Some("map-1"));
        assert_eq!(identity.revision, Some(4));
        assert_eq!(identity.canonical_sha256.as_deref(), Some("canonical-4"));
    }
}
