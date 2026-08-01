use codex_utils_absolute_path::AbsolutePathBuf;
use include_dir::Dir;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;

use sha2::Digest;
use sha2::Sha256;
use thiserror::Error;

const SYSTEM_SKILLS_DIR: Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/assets/samples");

const SYSTEM_SKILLS_DIR_NAME: &str = ".system";
const SKILLS_DIR_NAME: &str = "skills";
const SYSTEM_SKILLS_MARKER_FILENAME: &str = ".codex-system-skills.marker";
const SYSTEM_SKILLS_MARKER_SALT: &str = "v1";
const SYSTEM_SKILLS_SNAPSHOTS_DIR_NAME: &str = ".snapshots";
const TASKSPACE_ADVANCED_SKILL_RELATIVE_PATH: &str = "taskspace-advanced/SKILL.md";

pub const TASKSPACE_ADVANCED_SKILL_NAME: &str = "taskspace-advanced";
pub const TASKSPACE_ADVANCED_SKILL_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpaceSkillSnapshot {
    pub name: &'static str,
    pub skill_version: &'static str,
    pub body_sha256: String,
    pub immutable_snapshot_path: AbsolutePathBuf,
    pub body_bytes: usize,
}

/// Returns the on-disk cache location for embedded system skills from an absolute CODEX_HOME.
pub fn system_cache_root_dir(codex_home: &AbsolutePathBuf) -> AbsolutePathBuf {
    codex_home
        .join(SKILLS_DIR_NAME)
        .join(SYSTEM_SKILLS_DIR_NAME)
}

/// Installs embedded system skills into `CODEX_HOME/skills/.system`.
///
/// Clears any existing system skills directory first and then writes the embedded
/// skills directory into place.
///
/// To avoid doing unnecessary work on every startup, a marker file is written
/// with a fingerprint of the embedded directory. When the marker matches, the
/// install is skipped.
pub fn install_system_skills(codex_home: &AbsolutePathBuf) -> Result<(), SystemSkillsError> {
    let skills_root_dir = codex_home.join(SKILLS_DIR_NAME);
    fs::create_dir_all(skills_root_dir.as_path())
        .map_err(|source| SystemSkillsError::io("create skills root dir", source))?;

    let dest_system = system_cache_root_dir(codex_home);

    let marker_path = dest_system.join(SYSTEM_SKILLS_MARKER_FILENAME);
    let expected_fingerprint = embedded_system_skills_fingerprint();
    if dest_system.as_path().is_dir()
        && read_marker(&marker_path).is_ok_and(|marker| marker == expected_fingerprint)
    {
        return Ok(());
    }

    clear_active_system_skills(&dest_system)?;

    write_embedded_dir(&SYSTEM_SKILLS_DIR, &dest_system)?;
    fs::write(marker_path.as_path(), format!("{expected_fingerprint}\n"))
        .map_err(|source| SystemSkillsError::io("write system skills marker", source))?;
    Ok(())
}

/// Materializes the selected TaskSpace skill body at an immutable content-addressed path.
///
/// Call this only for a new TaskSpace session. Resume and fork paths must use their persisted
/// identity and must not recreate or replace a missing snapshot from the active bundle.
pub fn create_taskspace_advanced_snapshot(
    codex_home: &AbsolutePathBuf,
) -> Result<TaskSpaceSkillSnapshot, SystemSkillsError> {
    let file = SYSTEM_SKILLS_DIR
        .get_file(TASKSPACE_ADVANCED_SKILL_RELATIVE_PATH)
        .ok_or(SystemSkillsError::EmbeddedTaskSpaceSkillMissing)?;
    let contents = file.contents();
    let body_sha256 = format!("{:x}", Sha256::digest(contents));
    let immutable_snapshot_path = system_cache_root_dir(codex_home)
        .join(SYSTEM_SKILLS_SNAPSHOTS_DIR_NAME)
        .join(&body_sha256)
        .join(TASKSPACE_ADVANCED_SKILL_RELATIVE_PATH);

    if immutable_snapshot_path.as_path().exists() {
        let existing = fs::read(immutable_snapshot_path.as_path())
            .map_err(|source| SystemSkillsError::io("read TaskSpace skill snapshot", source))?;
        let existing_sha256 = format!("{:x}", Sha256::digest(&existing));
        if existing_sha256 != body_sha256 {
            return Err(SystemSkillsError::TaskSpaceSkillSnapshotHashMismatch {
                path: immutable_snapshot_path,
                expected: body_sha256,
                actual: existing_sha256,
            });
        }
    } else {
        let parent = immutable_snapshot_path
            .parent()
            .expect("TaskSpace skill snapshot path has a parent");
        fs::create_dir_all(parent.as_path()).map_err(|source| {
            SystemSkillsError::io("create TaskSpace skill snapshot directory", source)
        })?;
        fs::write(immutable_snapshot_path.as_path(), contents)
            .map_err(|source| SystemSkillsError::io("write TaskSpace skill snapshot", source))?;
    }

    Ok(TaskSpaceSkillSnapshot {
        name: TASKSPACE_ADVANCED_SKILL_NAME,
        skill_version: TASKSPACE_ADVANCED_SKILL_VERSION,
        body_sha256,
        immutable_snapshot_path,
        body_bytes: contents.len(),
    })
}

fn clear_active_system_skills(dest_system: &AbsolutePathBuf) -> Result<(), SystemSkillsError> {
    if !dest_system.as_path().exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dest_system.as_path())
        .map_err(|source| SystemSkillsError::io("read existing system skills dir", source))?;
    for entry in entries {
        let entry = entry
            .map_err(|source| SystemSkillsError::io("read existing system skill entry", source))?;
        if entry.file_name() == SYSTEM_SKILLS_SNAPSHOTS_DIR_NAME {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|source| {
                SystemSkillsError::io("remove existing system skill dir", source)
            })?;
        } else {
            fs::remove_file(&path).map_err(|source| {
                SystemSkillsError::io("remove existing system skill file", source)
            })?;
        }
    }
    Ok(())
}

fn read_marker(path: &AbsolutePathBuf) -> Result<String, SystemSkillsError> {
    Ok(fs::read_to_string(path.as_path())
        .map_err(|source| SystemSkillsError::io("read system skills marker", source))?
        .trim()
        .to_string())
}

fn embedded_system_skills_fingerprint() -> String {
    let mut items = Vec::new();
    collect_fingerprint_items(&SYSTEM_SKILLS_DIR, &mut items);
    items.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let mut hasher = DefaultHasher::new();
    SYSTEM_SKILLS_MARKER_SALT.hash(&mut hasher);
    for (path, contents_hash) in items {
        path.hash(&mut hasher);
        contents_hash.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}

fn collect_fingerprint_items(dir: &Dir<'_>, items: &mut Vec<(String, Option<u64>)>) {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                items.push((subdir.path().to_string_lossy().to_string(), None));
                collect_fingerprint_items(subdir, items);
            }
            include_dir::DirEntry::File(file) => {
                let mut file_hasher = DefaultHasher::new();
                file.contents().hash(&mut file_hasher);
                items.push((
                    file.path().to_string_lossy().to_string(),
                    Some(file_hasher.finish()),
                ));
            }
        }
    }
}

/// Writes the embedded `include_dir::Dir` to disk under `dest`.
///
/// Preserves the embedded directory structure.
fn write_embedded_dir(dir: &Dir<'_>, dest: &AbsolutePathBuf) -> Result<(), SystemSkillsError> {
    fs::create_dir_all(dest.as_path())
        .map_err(|source| SystemSkillsError::io("create system skills dir", source))?;

    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                let subdir_dest = dest.join(subdir.path());
                fs::create_dir_all(subdir_dest.as_path()).map_err(|source| {
                    SystemSkillsError::io("create system skills subdir", source)
                })?;
                write_embedded_dir(subdir, dest)?;
            }
            include_dir::DirEntry::File(file) => {
                let path = dest.join(file.path());
                if let Some(parent) = path.as_path().parent() {
                    fs::create_dir_all(parent).map_err(|source| {
                        SystemSkillsError::io("create system skills file parent", source)
                    })?;
                }
                fs::write(path.as_path(), file.contents())
                    .map_err(|source| SystemSkillsError::io("write system skill file", source))?;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum SystemSkillsError {
    #[error("io error while {action}: {source}")]
    Io {
        action: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("embedded taskspace-advanced skill is missing")]
    EmbeddedTaskSpaceSkillMissing,
    #[error(
        "TaskSpace skill snapshot hash mismatch at {path}: expected {expected}, actual {actual}"
    )]
    TaskSpaceSkillSnapshotHashMismatch {
        path: AbsolutePathBuf,
        expected: String,
        actual: String,
    },
}

impl SystemSkillsError {
    fn io(action: &'static str, source: std::io::Error) -> Self {
        Self::Io { action, source }
    }
}

#[cfg(test)]
mod tests {
    use super::SYSTEM_SKILLS_DIR;
    use super::SYSTEM_SKILLS_SNAPSHOTS_DIR_NAME;
    use super::collect_fingerprint_items;
    use super::create_taskspace_advanced_snapshot;
    use super::install_system_skills;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use std::fs;

    #[test]
    fn fingerprint_traverses_nested_entries() {
        let mut items = Vec::new();
        collect_fingerprint_items(&SYSTEM_SKILLS_DIR, &mut items);
        let mut paths: Vec<String> = items.into_iter().map(|(path, _)| path).collect();
        paths.sort_unstable();

        assert!(
            paths
                .binary_search_by(|probe| probe.as_str().cmp("skill-creator/SKILL.md"))
                .is_ok()
        );
        assert!(
            paths
                .binary_search_by(|probe| probe.as_str().cmp("skill-creator/scripts/init_skill.py"))
                .is_ok()
        );
        assert!(
            paths
                .binary_search_by(|probe| probe.as_str().cmp("taskspace-advanced/SKILL.md"))
                .is_ok()
        );
    }

    #[test]
    fn taskspace_snapshot_is_content_addressed_and_survives_system_skill_refresh() {
        let temp = tempfile::tempdir().expect("temp dir");
        let codex_home = AbsolutePathBuf::from_absolute_path(temp.path().to_path_buf())
            .expect("absolute temp path");
        install_system_skills(&codex_home).expect("install system skills");

        let snapshot =
            create_taskspace_advanced_snapshot(&codex_home).expect("create TaskSpace snapshot");
        let snapshot_contents =
            fs::read(snapshot.immutable_snapshot_path.as_path()).expect("read snapshot");
        assert_eq!(snapshot.body_bytes, snapshot_contents.len());
        assert!(
            snapshot
                .immutable_snapshot_path
                .as_path()
                .components()
                .any(|component| component.as_os_str() == SYSTEM_SKILLS_SNAPSHOTS_DIR_NAME)
        );

        let marker =
            super::system_cache_root_dir(&codex_home).join(super::SYSTEM_SKILLS_MARKER_FILENAME);
        fs::write(marker.as_path(), "stale-marker\n").expect("stale marker");
        install_system_skills(&codex_home).expect("refresh system skills");

        assert_eq!(
            fs::read(snapshot.immutable_snapshot_path.as_path()).expect("read preserved snapshot"),
            snapshot_contents
        );
    }
}
