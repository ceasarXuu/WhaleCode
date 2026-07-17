use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use sha2::Digest;
use sha2::Sha256;

const PROJECTION_START: &str = "TaskSpaceMapProjectionR7V1:";
const PROJECTION_END: &str = "TaskSpaceMapProjectionR7V1 end.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "later R7 phases activate the frozen trigger matrix"
)]
pub(crate) enum ProjectionTrigger {
    ProviderRequest,
    RevisionCommit,
    ExplicitRead,
    CompactionEpochStart,
    Resume,
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
            if role != "developer" && role != "system" {
                return None;
            }
            content.iter().rev().find_map(|entry| match entry {
                ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                    projection_identity_from_context(text)
                }
                ContentItem::InputImage { .. } => None,
            })
        });
        Self { last_emitted }
    }
}

pub(crate) fn projection_identity_from_context(context: &str) -> Option<ProjectionIdentity> {
    let start = context.rfind(PROJECTION_START)?;
    let candidate = &context[start..];
    let end = candidate.find(PROJECTION_END)? + PROJECTION_END.len();
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
#[allow(dead_code, reason = "Phase C activates append_revision")]
pub(crate) enum ProjectionEmission {
    None,
    ReplaceLatest,
    AppendRevision,
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
            ProjectionTrigger::ProviderRequest
            | ProjectionTrigger::CompactionEpochStart
            | ProjectionTrigger::Resume => (ProjectionEmission::ReplaceLatest, cursor.clone()),
            ProjectionTrigger::RevisionCommit => (ProjectionEmission::None, cursor.clone()),
            ProjectionTrigger::ExplicitRead => {
                (ProjectionEmission::ReturnAsToolResult, cursor.clone())
            }
        },
        TaskSpaceProjectionPolicy::MapAppend => match trigger {
            ProjectionTrigger::ProviderRequest => (ProjectionEmission::None, cursor.clone()),
            ProjectionTrigger::ExplicitRead => {
                (ProjectionEmission::ReturnAsToolResult, cursor.clone())
            }
            ProjectionTrigger::RevisionCommit
            | ProjectionTrigger::CompactionEpochStart
            | ProjectionTrigger::Resume => {
                let candidate = candidate.ok_or_else(|| {
                    format!("projection trigger `{trigger:?}` requires a rendered candidate")
                })?;
                let candidate_map = candidate
                    .map_id
                    .as_deref()
                    .ok_or_else(|| "append projection candidate has no map id".to_string())?;
                let candidate_revision = candidate.revision.ok_or_else(|| {
                    "append projection candidate has no committed revision".to_string()
                })?;
                let duplicate = cursor.last_emitted.as_ref().is_some_and(|last| {
                    last.map_id.as_deref() == Some(candidate_map)
                        && last.revision == Some(candidate_revision)
                });
                if duplicate {
                    (ProjectionEmission::None, cursor.clone())
                } else {
                    if let Some(last) = cursor.last_emitted.as_ref()
                        && last.map_id.as_deref() == Some(candidate_map)
                        && last
                            .revision
                            .is_some_and(|revision| revision > candidate_revision)
                    {
                        return Err(format!(
                            "append projection revision {candidate_revision} is older than emitted revision {}",
                            last.revision.expect("checked as some")
                        ));
                    }
                    (
                        ProjectionEmission::AppendRevision,
                        ProjectionCursor {
                            last_emitted: Some(candidate.clone()),
                        },
                    )
                }
            }
        },
        TaskSpaceProjectionPolicy::MapRequest => {
            return Err(format!(
                "projection policy `{policy}` is not enabled before R7 Phase D"
            ));
        }
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
    fn map_always_replaces_on_every_provider_visible_epoch() {
        let cursor = ProjectionCursor::default();
        for trigger in [
            ProjectionTrigger::ProviderRequest,
            ProjectionTrigger::CompactionEpochStart,
            ProjectionTrigger::Resume,
        ] {
            let decision = decide_projection_emission(
                TaskSpaceProjectionPolicy::MapAlways,
                trigger,
                &cursor,
                None,
            )
            .expect("map-always must be enabled");
            assert_eq!(decision.emission, ProjectionEmission::ReplaceLatest);
            assert_eq!(decision.next_cursor, cursor);
        }
    }

    #[test]
    fn map_always_does_not_emit_directly_on_revision_commit() {
        let decision = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAlways,
            ProjectionTrigger::RevisionCommit,
            &ProjectionCursor::default(),
            None,
        )
        .expect("map-always must be enabled");
        assert_eq!(decision.emission, ProjectionEmission::None);
    }

    #[test]
    fn map_append_emits_each_revision_once_and_rejects_regression() {
        let revision_3 = ProjectionIdentity {
            map_id: Some("map-1".into()),
            revision: Some(3),
            canonical_sha256: Some("canonical-3".into()),
            projection_sha256: "projection-3".into(),
        };
        let first = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::RevisionCommit,
            &ProjectionCursor::default(),
            Some(&revision_3),
        )
        .expect("first committed revision should append");
        assert_eq!(first.emission, ProjectionEmission::AppendRevision);

        let duplicate = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::RevisionCommit,
            &first.next_cursor,
            Some(&revision_3),
        )
        .expect("same revision retry should be suppressed");
        assert_eq!(duplicate.emission, ProjectionEmission::None);

        let revision_2 = ProjectionIdentity {
            revision: Some(2),
            ..revision_3
        };
        assert!(
            decide_projection_emission(
                TaskSpaceProjectionPolicy::MapAppend,
                ProjectionTrigger::RevisionCommit,
                &first.next_cursor,
                Some(&revision_2),
            )
            .is_err()
        );
    }

    #[test]
    fn map_append_provider_request_never_emits_directly() {
        let decision = decide_projection_emission(
            TaskSpaceProjectionPolicy::MapAppend,
            ProjectionTrigger::ProviderRequest,
            &ProjectionCursor::default(),
            None,
        )
        .expect("map-append provider request is enabled");
        assert_eq!(decision.emission, ProjectionEmission::None);
    }

    #[test]
    fn map_request_remains_rejected_until_phase_d() {
        assert!(
            decide_projection_emission(
                TaskSpaceProjectionPolicy::MapRequest,
                ProjectionTrigger::ProviderRequest,
                &ProjectionCursor::default(),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn cursor_restores_the_last_visible_projection_identity() {
        let item = ResponseItem::Message {
            id: None,
            role: "developer".into(),
            content: vec![ContentItem::InputText {
                text: "TaskSpaceMapProjectionR7V1:\n- projection_kind: revision_snapshot\n- map_id: map-1\n- revision: 4\n- canonical_sha256: canonical-4\nTaskSpaceMapProjectionR7V1 end.\n".into(),
            }],
            end_turn: None,
            phase: None,
        };

        let cursor = ProjectionCursor::from_items(&[item]);
        let identity = cursor.last_emitted.expect("projection identity");
        assert_eq!(identity.map_id.as_deref(), Some("map-1"));
        assert_eq!(identity.revision, Some(4));
        assert_eq!(identity.canonical_sha256.as_deref(), Some("canonical-4"));
    }
}
