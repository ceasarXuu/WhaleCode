use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use whalecode_protocol::{AgentId, ArtifactId, WorkUnitId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOwnershipMode {
    Exclusive,
    AppendOnly,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOwnershipClaim {
    pub path: String,
    pub mode: FileOwnershipMode,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchArtifact {
    pub id: ArtifactId,
    pub work_unit_id: WorkUnitId,
    pub agent_id: AgentId,
    pub base_commit: String,
    pub touched_files: Vec<String>,
    pub ownership: Vec<FileOwnershipClaim>,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: String,
    pub sha256: String,
    pub len: u64,
    pub modified_unix_nanos: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchOperation {
    ReplaceOne { old: String, new: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchRequest {
    pub path: String,
    pub expected_snapshot: FileSnapshot,
    pub operation: PatchOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
    pub create_parent_dirs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPreview {
    pub status: WorkspacePatchStatus,
    pub touched_files: Vec<String>,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspacePatchStatus {
    Applied,
    Rejected { reason: PatchRejectReason },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchRejectReason {
    OutsideWorkspace,
    FileNotFound,
    HiddenPath,
    PathMismatch,
    StaleRead,
    OldStringMissing,
    OldStringNotUnique,
    NonUtf8,
}

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("workspace root does not exist: {0}")]
    MissingWorkspace(String),
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to inspect {path}: {source}")]
    Metadata {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("file is not valid UTF-8: {0}")]
    NonUtf8(PathBuf),
}

#[derive(Debug, Clone)]
pub struct WorkspacePatchEngine {
    workspace_root: PathBuf,
    canonical_root: PathBuf,
}

impl WorkspacePatchEngine {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, PatchError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let canonical_root = workspace_root
            .canonicalize()
            .map_err(|_| PatchError::MissingWorkspace(workspace_root.display().to_string()))?;
        Ok(Self {
            workspace_root,
            canonical_root,
        })
    }

    pub fn snapshot(&self, path: &str) -> Result<FileSnapshot, PatchError> {
        let path = self
            .resolve_existing(path)
            .map_err(|reason| PatchError::Read {
                path: self.workspace_root.join(path),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("{reason:?}"),
                ),
            })?;
        let content = fs::read(&path).map_err(|source| PatchError::Read {
            path: path.clone(),
            source,
        })?;
        let metadata = fs::metadata(&path).map_err(|source| PatchError::Metadata {
            path: path.clone(),
            source,
        })?;
        let modified_unix_nanos = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        Ok(FileSnapshot {
            path: self.relative_display(&path),
            sha256: sha256_hex(&content),
            len: content.len() as u64,
            modified_unix_nanos,
        })
    }

    pub fn dry_run_apply(&self, request: &PatchRequest) -> Result<PatchPreview, PatchError> {
        Ok(self.checked_edit(request)?.preview)
    }

    pub fn apply(&self, request: &PatchRequest) -> Result<PatchPreview, PatchError> {
        let checked = self.checked_edit(request)?;
        if checked.preview.status == WorkspacePatchStatus::Applied {
            fs::write(&checked.path, checked.updated).map_err(|source| PatchError::Write {
                path: checked.path,
                source,
            })?;
        }
        Ok(checked.preview)
    }

    pub fn write_file(&self, request: &WriteFileRequest) -> Result<PatchPreview, PatchError> {
        let target = match self.resolve_write_target(&request.path, request.create_parent_dirs) {
            Ok(target) => target,
            Err(reason) => {
                return Ok(PatchPreview {
                    status: WorkspacePatchStatus::Rejected { reason },
                    touched_files: vec![request.path.clone()],
                    diff: String::new(),
                });
            }
        };
        let before = match fs::read_to_string(&target.path) {
            Ok(content) => Some(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                return Err(PatchError::NonUtf8(target.path));
            }
            Err(source) => {
                return Err(PatchError::Read {
                    path: target.path,
                    source,
                });
            }
        };
        if let Some(parent) = target.path.parent() {
            fs::create_dir_all(parent).map_err(|source| PatchError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&target.path, request.content.as_bytes()).map_err(|source| {
            PatchError::Write {
                path: target.path.clone(),
                source,
            }
        })?;
        Ok(PatchPreview {
            status: WorkspacePatchStatus::Applied,
            touched_files: vec![target.relative_path.clone()],
            diff: write_file_diff(&target.relative_path, before.as_deref(), &request.content),
        })
    }

    fn checked_edit(&self, request: &PatchRequest) -> Result<CheckedEdit, PatchError> {
        if request.path != request.expected_snapshot.path {
            return Ok(CheckedEdit::rejected(
                request,
                self.workspace_root.join(&request.path),
                PatchRejectReason::PathMismatch,
            ));
        }
        let path = match self.resolve_existing(&request.path) {
            Ok(path) => path,
            Err(reason) => {
                return Ok(CheckedEdit::rejected(
                    request,
                    self.workspace_root.join(&request.path),
                    reason,
                ));
            }
        };
        let current = self.snapshot(&request.path)?;
        if current != request.expected_snapshot {
            return Ok(CheckedEdit::rejected(
                request,
                path,
                PatchRejectReason::StaleRead,
            ));
        }
        let content = self.read_utf8(&path)?;

        match &request.operation {
            PatchOperation::ReplaceOne { old, new } => {
                checked_replace(request, path, content, old, new)
            }
        }
    }

    fn read_utf8(&self, path: &Path) -> Result<String, PatchError> {
        fs::read_to_string(path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::InvalidData {
                PatchError::NonUtf8(path.to_path_buf())
            } else {
                PatchError::Read {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })
    }

    fn resolve_existing(&self, path: &str) -> Result<PathBuf, PatchRejectReason> {
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.canonical_root.join(path)
        };
        if candidate
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(PatchRejectReason::OutsideWorkspace);
        }
        if candidate
            .components()
            .any(|part| part.as_os_str().to_str().is_some_and(should_skip_name))
        {
            return Err(PatchRejectReason::HiddenPath);
        }
        let canonical = candidate.canonicalize().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                PatchRejectReason::FileNotFound
            } else {
                PatchRejectReason::OutsideWorkspace
            }
        })?;
        if !canonical.starts_with(&self.canonical_root) {
            return Err(PatchRejectReason::OutsideWorkspace);
        }
        Ok(canonical)
    }

    fn resolve_write_target(
        &self,
        path: &str,
        create_parent_dirs: bool,
    ) -> Result<WriteTarget, PatchRejectReason> {
        if path.trim().is_empty() {
            return Err(PatchRejectReason::OutsideWorkspace);
        }
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.canonical_root.join(path)
        };
        if candidate
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(PatchRejectReason::OutsideWorkspace);
        }
        if candidate
            .components()
            .any(|part| part.as_os_str().to_str().is_some_and(should_skip_name))
        {
            return Err(PatchRejectReason::HiddenPath);
        }
        let Some(parent) = candidate.parent() else {
            return Err(PatchRejectReason::OutsideWorkspace);
        };
        if !parent.exists() && !create_parent_dirs {
            return Err(PatchRejectReason::FileNotFound);
        }
        if !parent.exists()
            && !candidate.starts_with(&self.canonical_root)
            && !candidate.starts_with(&self.workspace_root)
        {
            return Err(PatchRejectReason::OutsideWorkspace);
        }
        if create_parent_dirs {
            fs::create_dir_all(parent).map_err(|_| PatchRejectReason::FileNotFound)?;
        }
        let canonical_parent = parent
            .canonicalize()
            .map_err(|_| PatchRejectReason::FileNotFound)?;
        if !canonical_parent.starts_with(&self.canonical_root) {
            return Err(PatchRejectReason::OutsideWorkspace);
        }
        let file_name = candidate
            .file_name()
            .ok_or(PatchRejectReason::OutsideWorkspace)?;
        let path = canonical_parent.join(file_name);
        let relative_path = self.relative_display(&path);
        Ok(WriteTarget {
            path,
            relative_path,
        })
    }

    fn relative_display(&self, path: &Path) -> String {
        path.strip_prefix(&self.canonical_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

struct CheckedEdit {
    path: PathBuf,
    updated: String,
    preview: PatchPreview,
}

struct WriteTarget {
    path: PathBuf,
    relative_path: String,
}

impl CheckedEdit {
    fn rejected(request: &PatchRequest, path: PathBuf, reason: PatchRejectReason) -> Self {
        Self {
            path,
            updated: String::new(),
            preview: PatchPreview {
                status: WorkspacePatchStatus::Rejected { reason },
                touched_files: vec![request.path.clone()],
                diff: String::new(),
            },
        }
    }
}

fn checked_replace(
    request: &PatchRequest,
    path: PathBuf,
    content: String,
    old: &str,
    new: &str,
) -> Result<CheckedEdit, PatchError> {
    let matches = content.matches(old).count();
    if matches == 0 {
        return Ok(CheckedEdit::rejected(
            request,
            path,
            PatchRejectReason::OldStringMissing,
        ));
    }
    if matches > 1 {
        return Ok(CheckedEdit::rejected(
            request,
            path,
            PatchRejectReason::OldStringNotUnique,
        ));
    }
    let updated = content.replacen(old, new, 1);
    Ok(CheckedEdit {
        path,
        preview: PatchPreview {
            status: WorkspacePatchStatus::Applied,
            touched_files: vec![request.path.clone()],
            diff: replacement_diff(&request.path, old, new, &content, &updated),
        },
        updated,
    })
}

fn replacement_diff(path: &str, old: &str, new: &str, before: &str, after: &str) -> String {
    format!(
        "--- a/{path}\n+++ b/{path}\n@@ replacement @@\n-{}\n+{}\n# before_sha256={}\n# after_sha256={}",
        single_line(old),
        single_line(new),
        sha256_hex(before.as_bytes()),
        sha256_hex(after.as_bytes())
    )
}

fn write_file_diff(path: &str, before: Option<&str>, after: &str) -> String {
    match before {
        Some(before) => format!(
            "--- a/{path}\n+++ b/{path}\n@@ write_file @@\n-{}\n+{}\n# before_sha256={}\n# after_sha256={}",
            single_line(before),
            single_line(after),
            sha256_hex(before.as_bytes()),
            sha256_hex(after.as_bytes())
        ),
        None => format!(
            "--- /dev/null\n+++ b/{path}\n@@ write_file @@\n+{}\n# after_sha256={}",
            single_line(after),
            sha256_hex(after.as_bytes())
        ),
    }
}

fn single_line(value: &str) -> String {
    value.replace('\n', "\\n")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn should_skip_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".whale"
            | ".claude"
            | ".codex"
            | ".opencode"
            | ".env"
            | ".env.local"
            | "target"
            | "tmp"
            | ".DS_Store"
    )
}
