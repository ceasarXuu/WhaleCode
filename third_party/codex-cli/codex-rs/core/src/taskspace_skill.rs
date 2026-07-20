use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::SkillScope;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::protocol::TaskSpaceSkillSnapshotIdentity;
use codex_utils_absolute_path::AbsolutePathBuf;

use crate::SkillLoadOutcome;
use crate::skills::TASKSPACE_ADVANCED_SKILL_NAME;
use crate::skills::TASKSPACE_ADVANCED_SKILL_VERSION;
use crate::skills::create_taskspace_advanced_snapshot;

pub(crate) fn resolve_session_snapshot(
    policy: Option<TaskSpaceProjectionPolicy>,
    history: &InitialHistory,
    codex_home: &AbsolutePathBuf,
) -> Result<Option<TaskSpaceSkillSnapshotIdentity>> {
    if policy.is_none() {
        return Ok(None);
    }

    let identity = match history {
        InitialHistory::New | InitialHistory::Cleared => {
            let snapshot = create_taskspace_advanced_snapshot(codex_home)
                .context("failed to create TaskSpace advanced-skill snapshot")?;
            TaskSpaceSkillSnapshotIdentity {
                name: snapshot.name.to_string(),
                skill_version: snapshot.skill_version.to_string(),
                body_sha256: snapshot.body_sha256,
                immutable_snapshot_path: snapshot.immutable_snapshot_path.into_path_buf(),
            }
        }
        InitialHistory::Resumed(_) | InitialHistory::Forked(_) => history
            .taskspace_skill_snapshot()
            .context(
                "TaskSpace rollout has no pinned advanced-skill snapshot; R7 does not migrate pre-FLA-3 sessions",
            )?,
    };
    validate_identity(&identity)?;
    Ok(Some(identity))
}

pub(crate) fn bind_catalog_snapshot(
    outcome: &mut SkillLoadOutcome,
    identity: Option<&TaskSpaceSkillSnapshotIdentity>,
) -> Result<()> {
    let bundled_paths = outcome
        .skills
        .iter()
        .filter(|skill| {
            skill.name == TASKSPACE_ADVANCED_SKILL_NAME && skill.scope == SkillScope::System
        })
        .map(|skill| skill.path_to_skills_md.clone())
        .collect::<Vec<_>>();

    let Some(identity) = identity else {
        for path in bundled_paths {
            outcome.remove_skill_at_path(&path);
        }
        return Ok(());
    };

    if bundled_paths.len() != 1 {
        remove_reserved_name(outcome);
        bail!(
            "TaskSpace requires exactly one bundled `{TASKSPACE_ADVANCED_SKILL_NAME}` skill; found {}",
            bundled_paths.len()
        );
    }
    let source_path = &bundled_paths[0];
    let conflicts = outcome
        .skills
        .iter()
        .filter(|skill| {
            skill.name == TASKSPACE_ADVANCED_SKILL_NAME && skill.path_to_skills_md != *source_path
        })
        .map(|skill| skill.path_to_skills_md.display().to_string())
        .collect::<Vec<_>>();
    if !conflicts.is_empty() {
        remove_reserved_name(outcome);
        bail!(
            "reserved TaskSpace skill name `{TASKSPACE_ADVANCED_SKILL_NAME}` conflicts with: {}",
            conflicts.join(", ")
        );
    }

    let snapshot_path = match validate_identity(identity) {
        Ok(path) => path,
        Err(error) => {
            remove_reserved_name(outcome);
            return Err(error);
        }
    };
    outcome
        .rebind_skill_to_snapshot(
            source_path,
            snapshot_path.clone(),
            identity.body_sha256.clone(),
        )
        .map_err(anyhow::Error::msg)?;
    tracing::info!(
        target: "codex_core::taskspace",
        event_name = "taskspace.skill_snapshot_bound",
        load_trigger = "session_catalog",
        carrier = "catalog",
        name = identity.name,
        skill_version = identity.skill_version,
        body_sha256 = identity.body_sha256,
        immutable_snapshot_path = %snapshot_path.display(),
        status = if snapshot_path.as_path().is_file() { "available" } else { "snapshot_missing" },
        "bound TaskSpace advanced skill catalog entry to session snapshot"
    );
    Ok(())
}

fn remove_reserved_name(outcome: &mut SkillLoadOutcome) {
    let paths = outcome
        .skills
        .iter()
        .filter(|skill| skill.name == TASKSPACE_ADVANCED_SKILL_NAME)
        .map(|skill| skill.path_to_skills_md.clone())
        .collect::<Vec<_>>();
    for path in paths {
        outcome.remove_skill_at_path(&path);
    }
}

fn validate_identity(identity: &TaskSpaceSkillSnapshotIdentity) -> Result<AbsolutePathBuf> {
    if identity.name != TASKSPACE_ADVANCED_SKILL_NAME {
        bail!("invalid TaskSpace skill snapshot name `{}`", identity.name);
    }
    if identity.skill_version != TASKSPACE_ADVANCED_SKILL_VERSION {
        bail!(
            "unsupported TaskSpace skill snapshot version `{}`",
            identity.skill_version
        );
    }
    if identity.body_sha256.len() != 64
        || !identity
            .body_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("invalid TaskSpace skill snapshot SHA-256");
    }
    let path = AbsolutePathBuf::from_absolute_path(identity.immutable_snapshot_path.clone())
        .context("TaskSpace skill snapshot path must be absolute")?;
    let expected_suffix = Path::new(".snapshots")
        .join(&identity.body_sha256)
        .join(TASKSPACE_ADVANCED_SKILL_NAME)
        .join("SKILL.md");
    if !path.as_path().ends_with(&expected_suffix) {
        bail!(
            "TaskSpace skill snapshot path does not match its identity: {}",
            path.display()
        );
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkillMetadata;
    use codex_protocol::ThreadId;
    use codex_protocol::protocol::ResumedHistory;
    use codex_protocol::protocol::RolloutItem;
    use codex_protocol::protocol::SessionMeta;
    use codex_protocol::protocol::SessionMetaLine;
    use sha2::Digest;
    use std::fs;
    use tempfile::tempdir;

    fn identity_at(path: &Path, sha256: &str) -> TaskSpaceSkillSnapshotIdentity {
        TaskSpaceSkillSnapshotIdentity {
            name: TASKSPACE_ADVANCED_SKILL_NAME.to_string(),
            skill_version: TASKSPACE_ADVANCED_SKILL_VERSION.to_string(),
            body_sha256: sha256.to_string(),
            immutable_snapshot_path: path
                .join("skills/.system/.snapshots")
                .join(sha256)
                .join(TASKSPACE_ADVANCED_SKILL_NAME)
                .join("SKILL.md"),
        }
    }

    fn history_with_identity(
        identity: TaskSpaceSkillSnapshotIdentity,
        forked: bool,
    ) -> InitialHistory {
        let item = RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: ThreadId::new(),
                taskspace_skill_snapshot: Some(identity),
                ..Default::default()
            },
            git: None,
        });
        if forked {
            InitialHistory::Forked(vec![item])
        } else {
            InitialHistory::Resumed(ResumedHistory {
                conversation_id: ThreadId::new(),
                history: vec![item],
                rollout_path: None,
            })
        }
    }

    fn bundled_skill(path: AbsolutePathBuf, scope: SkillScope) -> SkillMetadata {
        SkillMetadata {
            name: TASKSPACE_ADVANCED_SKILL_NAME.to_string(),
            description: "advanced TaskSpace guidance".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: path,
            scope,
        }
    }

    #[test]
    fn new_taskspace_session_creates_exact_content_addressed_snapshot() {
        let temp = tempdir().expect("temp dir");
        let codex_home = AbsolutePathBuf::from_absolute_path(temp.path().to_path_buf())
            .expect("absolute temp path");

        let identity = resolve_session_snapshot(
            Some(TaskSpaceProjectionPolicy::MapRequest),
            &InitialHistory::New,
            &codex_home,
        )
        .expect("resolve snapshot")
        .expect("TaskSpace snapshot identity");

        assert!(identity.immutable_snapshot_path.is_file());
        let body = fs::read(&identity.immutable_snapshot_path).expect("read snapshot");
        assert_eq!(
            format!("{:x}", sha2::Sha256::digest(body)),
            identity.body_sha256
        );
    }

    #[test]
    fn standard_session_has_no_snapshot_or_catalog_entry() {
        let temp = tempdir().expect("temp dir");
        let codex_home = AbsolutePathBuf::from_absolute_path(temp.path().to_path_buf())
            .expect("absolute temp path");
        assert_eq!(
            resolve_session_snapshot(None, &InitialHistory::New, &codex_home)
                .expect("resolve Standard snapshot"),
            None
        );
        assert!(!temp.path().join("skills/.system/.snapshots").exists());

        let source = AbsolutePathBuf::from_absolute_path(
            temp.path()
                .join("skills/.system/taskspace-advanced/SKILL.md"),
        )
        .expect("absolute source path");
        let mut outcome = SkillLoadOutcome::default();
        outcome
            .skills
            .push(bundled_skill(source, SkillScope::System));
        bind_catalog_snapshot(&mut outcome, None).expect("bind Standard catalog");
        assert!(
            outcome
                .skills
                .iter()
                .all(|skill| skill.name != TASKSPACE_ADVANCED_SKILL_NAME)
        );
    }

    #[test]
    fn resume_and_fork_restore_the_persisted_identity_without_materializing_latest() {
        let temp = tempdir().expect("temp dir");
        let codex_home = AbsolutePathBuf::from_absolute_path(temp.path().to_path_buf())
            .expect("absolute temp path");
        let persisted = identity_at(temp.path(), &"a".repeat(64));

        for history in [
            history_with_identity(persisted.clone(), false),
            history_with_identity(persisted.clone(), true),
        ] {
            assert_eq!(
                resolve_session_snapshot(
                    Some(TaskSpaceProjectionPolicy::MapAlways),
                    &history,
                    &codex_home,
                )
                .expect("restore snapshot"),
                Some(persisted.clone())
            );
        }
        assert!(!persisted.immutable_snapshot_path.exists());
    }

    #[test]
    fn resumed_taskspace_session_without_identity_is_rejected() {
        let temp = tempdir().expect("temp dir");
        let codex_home = AbsolutePathBuf::from_absolute_path(temp.path().to_path_buf())
            .expect("absolute temp path");
        let history = InitialHistory::Forked(vec![RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: ThreadId::new(),
                ..Default::default()
            },
            git: None,
        })]);

        let error = resolve_session_snapshot(
            Some(TaskSpaceProjectionPolicy::MapAppend),
            &history,
            &codex_home,
        )
        .expect_err("missing identity must fail");
        assert!(error.to_string().contains("does not migrate pre-FLA-3"));
    }

    #[test]
    fn catalog_binding_uses_missing_persisted_snapshot_without_latest_fallback() {
        let temp = tempdir().expect("temp dir");
        let source = AbsolutePathBuf::from_absolute_path(
            temp.path()
                .join("skills/.system/taskspace-advanced/SKILL.md"),
        )
        .expect("absolute source path");
        let identity = identity_at(temp.path(), &"a".repeat(64));
        let snapshot =
            AbsolutePathBuf::from_absolute_path(identity.immutable_snapshot_path.clone())
                .expect("absolute snapshot path");
        let mut outcome = SkillLoadOutcome::default();
        outcome
            .skills
            .push(bundled_skill(source.clone(), SkillScope::System));

        bind_catalog_snapshot(&mut outcome, Some(&identity)).expect("bind snapshot");
        assert_eq!(outcome.skills[0].path_to_skills_md, snapshot);
        assert_eq!(
            outcome.expected_body_sha256(&outcome.skills[0]),
            Some(identity.body_sha256.as_str())
        );
        assert!(!outcome.skills[0].path_to_skills_md.exists());
        assert!(
            outcome
                .skills
                .iter()
                .all(|skill| skill.path_to_skills_md != source)
        );
    }

    #[test]
    fn reserved_name_conflict_is_rejected_and_removed() {
        let temp = tempdir().expect("temp dir");
        let source = AbsolutePathBuf::from_absolute_path(
            temp.path()
                .join("skills/.system/taskspace-advanced/SKILL.md"),
        )
        .expect("absolute source path");
        let conflict = AbsolutePathBuf::from_absolute_path(
            temp.path().join("repo-skill/taskspace-advanced/SKILL.md"),
        )
        .expect("absolute conflict path");
        let identity = identity_at(temp.path(), &"a".repeat(64));
        let mut outcome = SkillLoadOutcome::default();
        outcome
            .skills
            .push(bundled_skill(source, SkillScope::System));
        outcome
            .skills
            .push(bundled_skill(conflict, SkillScope::Repo));

        let error = bind_catalog_snapshot(&mut outcome, Some(&identity))
            .expect_err("reserved name conflict must fail");
        assert!(error.to_string().contains("reserved TaskSpace skill name"));
        assert!(
            outcome
                .skills
                .iter()
                .all(|skill| skill.name != TASKSPACE_ADVANCED_SKILL_NAME)
        );
    }

    #[test]
    fn persisted_identity_requires_content_addressed_snapshot_suffix() {
        let identity = TaskSpaceSkillSnapshotIdentity {
            name: TASKSPACE_ADVANCED_SKILL_NAME.to_string(),
            skill_version: TASKSPACE_ADVANCED_SKILL_VERSION.to_string(),
            body_sha256: "a".repeat(64),
            immutable_snapshot_path: Path::new("/tmp/.snapshots")
                .join("a".repeat(64))
                .join(TASKSPACE_ADVANCED_SKILL_NAME)
                .join("SKILL.md"),
        };
        assert!(validate_identity(&identity).is_ok());

        let mut mismatched = identity;
        mismatched.body_sha256 = "b".repeat(64);
        assert!(validate_identity(&mismatched).is_err());
    }
}
