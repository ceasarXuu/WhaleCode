use codex_protocol::protocol::TaskSpaceProjectionPolicy;

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
) -> Result<ProjectionDecision, String> {
    let emission = match policy {
        TaskSpaceProjectionPolicy::MapAlways => match trigger {
            ProjectionTrigger::ProviderRequest
            | ProjectionTrigger::CompactionEpochStart
            | ProjectionTrigger::Resume => ProjectionEmission::ReplaceLatest,
            ProjectionTrigger::RevisionCommit => ProjectionEmission::None,
            ProjectionTrigger::ExplicitRead => ProjectionEmission::ReturnAsToolResult,
        },
        TaskSpaceProjectionPolicy::MapAppend | TaskSpaceProjectionPolicy::MapRequest => {
            return Err(format!(
                "projection policy `{policy}` is not enabled in R7 Phase B"
            ));
        }
    };
    Ok(ProjectionDecision {
        emission,
        next_cursor: cursor.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_always_replaces_on_every_provider_visible_epoch() {
        let cursor = ProjectionCursor::default();
        for trigger in [
            ProjectionTrigger::ProviderRequest,
            ProjectionTrigger::CompactionEpochStart,
            ProjectionTrigger::Resume,
        ] {
            let decision =
                decide_projection_emission(TaskSpaceProjectionPolicy::MapAlways, trigger, &cursor)
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
        )
        .expect("map-always must be enabled");
        assert_eq!(decision.emission, ProjectionEmission::None);
    }

    #[test]
    fn later_policies_are_rejected_until_their_phase() {
        for policy in [
            TaskSpaceProjectionPolicy::MapAppend,
            TaskSpaceProjectionPolicy::MapRequest,
        ] {
            assert!(
                decide_projection_emission(
                    policy,
                    ProjectionTrigger::ProviderRequest,
                    &ProjectionCursor::default(),
                )
                .is_err()
            );
        }
    }
}
