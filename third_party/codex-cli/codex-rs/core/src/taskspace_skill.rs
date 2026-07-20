use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::SkillScope;
use codex_protocol::protocol::TaskSpaceProjectionPolicy;
use codex_protocol::protocol::TaskSpaceSkillSnapshotIdentity;
use codex_utils_absolute_path::AbsolutePathBuf;
use sha2::Digest;
use sha2::Sha256;

use crate::SkillLoadOutcome;
use crate::SkillMetadata;
use crate::skills::TASKSPACE_ADVANCED_SKILL_NAME;
use crate::skills::TASKSPACE_ADVANCED_SKILL_VERSION;
use crate::skills::create_taskspace_advanced_snapshot;

pub(crate) struct TaskSpaceSkillFailureFact {
    pub(crate) status: &'static str,
    pub(crate) reason_code: &'static str,
    pub(crate) text: String,
}

struct TaskSpaceSkillCatalogObservation {
    status: &'static str,
    reason_code: &'static str,
    original_description_bytes: usize,
    rendered_skill_bytes: usize,
    rendered_catalog_bytes: usize,
    catalog_sha256: String,
}

pub(crate) fn explicit_load_failure_fact(
    name: &str,
    path: &str,
    expected_sha256: &str,
    error: &str,
) -> TaskSpaceSkillFailureFact {
    if !Path::new(path).is_file() {
        return TaskSpaceSkillFailureFact {
            status: "snapshot_missing",
            reason_code: "TASKSPACE_SKILL_SNAPSHOT_MISSING",
            text: format!(
                "TaskSpace skill snapshot unavailable: name={name} version={TASKSPACE_ADVANCED_SKILL_VERSION} sha256={expected_sha256} path={path}"
            ),
        };
    }

    TaskSpaceSkillFailureFact {
        status: "integrity_failed",
        reason_code: "TASKSPACE_SKILL_SNAPSHOT_INTEGRITY_FAILED",
        text: format!(
            "TaskSpace skill snapshot integrity check failed: name={name} version={TASKSPACE_ADVANCED_SKILL_VERSION} sha256={expected_sha256} path={path} error={error}"
        ),
    }
}

pub(crate) fn log_catalog_render(
    identity: &TaskSpaceSkillSnapshotIdentity,
    outcome: &SkillLoadOutcome,
    available: Option<&codex_core_skills::AvailableSkills>,
    rendered_catalog: Option<&str>,
) {
    let observation = catalog_render_observation(outcome, available, rendered_catalog);
    tracing::info!(
        target: "codex_core::taskspace",
        event_name = "taskspace.skill_catalog_rendered",
        load_trigger = "catalog",
        carrier = "catalog",
        name = identity.name,
        skill_version = identity.skill_version,
        body_sha256 = identity.body_sha256,
        immutable_snapshot_path = %identity.immutable_snapshot_path.display(),
        body_bytes = std::fs::metadata(&identity.immutable_snapshot_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0),
        original_description_bytes = observation.original_description_bytes,
        rendered_skill_bytes = observation.rendered_skill_bytes,
        rendered_catalog_bytes = observation.rendered_catalog_bytes,
        catalog_sha256 = observation.catalog_sha256,
        skill_load_status = observation.status,
        reason_code = observation.reason_code,
        "rendered TaskSpace advanced skill catalog metadata"
    );
}

fn catalog_render_observation(
    outcome: &SkillLoadOutcome,
    available: Option<&codex_core_skills::AvailableSkills>,
    rendered_catalog: Option<&str>,
) -> TaskSpaceSkillCatalogObservation {
    let selected = outcome.skills.iter().find(|skill| {
        skill.name == TASKSPACE_ADVANCED_SKILL_NAME && skill.scope == SkillScope::System
    });
    let line_prefix = format!("- {TASKSPACE_ADVANCED_SKILL_NAME}:");
    let rendered_line = available.and_then(|available| {
        available
            .skill_lines
            .iter()
            .find(|line| line.starts_with(&line_prefix))
    });
    let (status, reason_code) = match (selected, rendered_line) {
        (_, None) => ("catalog_not_visible", "metadata_budget"),
        (Some(skill), Some(line)) if !line.contains(&format!(": {} (file:", skill.description)) => {
            ("description_truncated", "metadata_budget")
        }
        _ => ("loaded", ""),
    };
    let original_description_bytes = selected.map_or(0, |skill| skill.description.len());
    let rendered_skill_bytes = rendered_line.map_or(0, String::len);
    let rendered_catalog_bytes = rendered_catalog.map_or(0, str::len);
    let catalog_sha256 = rendered_catalog
        .map(|catalog| format!("{:x}", Sha256::digest(catalog.as_bytes())))
        .unwrap_or_default();

    TaskSpaceSkillCatalogObservation {
        status,
        reason_code,
        original_description_bytes,
        rendered_skill_bytes,
        rendered_catalog_bytes,
        catalog_sha256,
    }
}

pub(crate) fn log_agent_file_read(
    outcome: &SkillLoadOutcome,
    skill: &SkillMetadata,
    success: bool,
    response_bytes: usize,
) {
    if skill.name != TASKSPACE_ADVANCED_SKILL_NAME {
        return;
    }
    let Some(body_sha256) = outcome.expected_body_sha256(skill) else {
        return;
    };
    let status = if success { "loaded" } else { "load_failed" };
    let reason_code = if success { "" } else { "TOOL_EXIT_NONZERO" };
    tracing::info!(
        target: "codex_core::taskspace",
        event_name = "taskspace.skill_load_completed",
        load_trigger = "agent_selection",
        carrier = "agent_file_read",
        name = skill.name,
        skill_version = TASKSPACE_ADVANCED_SKILL_VERSION,
        body_sha256,
        immutable_snapshot_path = %skill.path_to_skills_md.display(),
        body_bytes = std::fs::metadata(skill.path_to_skills_md.as_path())
            .map(|metadata| metadata.len())
            .unwrap_or(0),
        response_bytes,
        skill_load_status = status,
        reason_code,
        "Agent read TaskSpace advanced skill through an ordinary file tool"
    );
}

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
#[path = "taskspace_skill_tests.rs"]
mod tests;
