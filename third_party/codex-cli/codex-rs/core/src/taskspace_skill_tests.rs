use super::*;
use crate::SkillMetadata;
use codex_core_skills::AvailableSkills;
use codex_core_skills::SkillRenderReport;
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

fn history_with_identity(identity: TaskSpaceSkillSnapshotIdentity, forked: bool) -> InitialHistory {
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

fn available_with_line(line: Option<String>) -> AvailableSkills {
    AvailableSkills {
        skill_root_lines: Vec::new(),
        skill_lines: line.into_iter().collect(),
        report: SkillRenderReport {
            total_count: 1,
            included_count: 1,
            omitted_count: 0,
            truncated_description_chars: 0,
            truncated_description_count: 0,
        },
        warning_message: None,
    }
}

#[test]
fn catalog_observation_distinguishes_full_truncated_and_omitted_metadata() {
    let temp = tempdir().expect("temp dir");
    let path = AbsolutePathBuf::from_absolute_path(
        temp.path()
            .join("skills/.system/.snapshots/a/taskspace-advanced/SKILL.md"),
    )
    .expect("absolute path");
    let skill = bundled_skill(path.clone(), SkillScope::System);
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills.push(skill.clone());

    let full = available_with_line(Some(format!(
        "- {}: {} (file: {})",
        skill.name,
        skill.description,
        path.display()
    )));
    let full_observation = catalog_render_observation(&outcome, Some(&full), Some("full catalog"));
    assert_eq!(full_observation.status, "loaded");
    assert_eq!(full_observation.reason_code, "");
    assert_eq!(full_observation.rendered_catalog_bytes, 12);
    assert!(!full_observation.catalog_sha256.is_empty());

    let truncated = available_with_line(Some(format!(
        "- {}: advanced (file: {})",
        skill.name,
        path.display()
    )));
    let truncated_observation =
        catalog_render_observation(&outcome, Some(&truncated), Some("truncated catalog"));
    assert_eq!(truncated_observation.status, "description_truncated");
    assert_eq!(truncated_observation.reason_code, "metadata_budget");

    let omitted_observation = catalog_render_observation(&outcome, None, None);
    assert_eq!(omitted_observation.status, "catalog_not_visible");
    assert_eq!(omitted_observation.reason_code, "metadata_budget");

    let unavailable_observation =
        catalog_render_observation(&SkillLoadOutcome::default(), None, None);
    assert_eq!(unavailable_observation.status, "catalog_not_visible");
    assert_eq!(
        unavailable_observation.reason_code,
        "bundled_skill_unavailable"
    );
}

#[test]
fn explicit_load_failure_fact_is_exact_and_does_not_fallback() {
    let temp = tempdir().expect("temp dir");
    let missing_path = temp.path().join("missing/SKILL.md");
    let missing = explicit_load_failure_fact(
        TASKSPACE_ADVANCED_SKILL_NAME,
        &missing_path.display().to_string(),
        &"a".repeat(64),
        "not found",
    );
    assert_eq!(missing.status, "snapshot_missing");
    assert_eq!(missing.reason_code, "TASKSPACE_SKILL_SNAPSHOT_MISSING");
    assert_eq!(
        missing.text,
        format!(
            "TaskSpace skill snapshot unavailable: name={TASKSPACE_ADVANCED_SKILL_NAME} version={TASKSPACE_ADVANCED_SKILL_VERSION} sha256={} path={}",
            "a".repeat(64),
            missing_path.display()
        )
    );

    fs::create_dir_all(missing_path.parent().expect("snapshot parent"))
        .expect("create snapshot parent");
    fs::write(&missing_path, "tampered").expect("write tampered snapshot");
    let integrity = explicit_load_failure_fact(
        TASKSPACE_ADVANCED_SKILL_NAME,
        &missing_path.display().to_string(),
        &"a".repeat(64),
        "sha mismatch",
    );
    assert_eq!(integrity.status, "integrity_failed");
    assert_eq!(
        integrity.reason_code,
        "TASKSPACE_SKILL_SNAPSHOT_INTEGRITY_FAILED"
    );
    assert!(integrity.text.ends_with("error=sha mismatch"));
}

#[test]
fn new_taskspace_session_creates_exact_content_addressed_snapshot() {
    let temp = tempdir().expect("temp dir");
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).expect("absolute temp path");

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
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).expect("absolute temp path");
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
    bind_catalog_snapshot(&mut outcome, false, None).expect("bind Standard catalog");
    assert!(
        outcome
            .skills
            .iter()
            .all(|skill| skill.name != TASKSPACE_ADVANCED_SKILL_NAME)
    );
}

#[test]
fn standard_catalog_ignores_a_configured_taskspace_policy_snapshot() {
    let temp = tempdir().expect("temp dir");
    let source = AbsolutePathBuf::from_absolute_path(
        temp.path()
            .join("skills/.system/taskspace-advanced/SKILL.md"),
    )
    .expect("absolute source path");
    let identity = identity_at(temp.path(), &"a".repeat(64));
    let mut outcome = SkillLoadOutcome::default();
    outcome
        .skills
        .push(bundled_skill(source, SkillScope::System));

    bind_catalog_snapshot(&mut outcome, false, Some(&identity))
        .expect("Standard catalog filtering");
    assert!(
        outcome
            .skills
            .iter()
            .all(|skill| skill.name != TASKSPACE_ADVANCED_SKILL_NAME)
    );
}

#[test]
fn taskspace_session_continues_when_bundled_skills_are_disabled() {
    let temp = tempdir().expect("temp dir");
    let identity = identity_at(temp.path(), &"a".repeat(64));
    let mut outcome = SkillLoadOutcome::default();

    bind_catalog_snapshot(&mut outcome, true, Some(&identity))
        .expect("optional advanced skill must not block TaskSpace startup");
    assert!(outcome.skills.is_empty());
}

#[test]
fn resume_and_fork_restore_the_persisted_identity_without_materializing_latest() {
    let temp = tempdir().expect("temp dir");
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).expect("absolute temp path");
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
    let codex_home = AbsolutePathBuf::from_absolute_path(temp.path()).expect("absolute temp path");
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
    let snapshot = AbsolutePathBuf::from_absolute_path(identity.immutable_snapshot_path.clone())
        .expect("absolute snapshot path");
    let mut outcome = SkillLoadOutcome::default();
    outcome
        .skills
        .push(bundled_skill(source.clone(), SkillScope::System));

    bind_catalog_snapshot(&mut outcome, true, Some(&identity)).expect("bind snapshot");
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

    let error = bind_catalog_snapshot(&mut outcome, true, Some(&identity))
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
