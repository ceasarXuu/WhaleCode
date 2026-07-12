use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use codex_exec_server::ExecutorFileSystem;
use codex_exec_server::FileSystemSandboxContext;
use codex_exec_server::RemoveOptions;
use codex_utils_absolute_path::AbsolutePathBuf;

use super::AffectedPaths;
use super::AppliedPatch;
use super::ApplyPatchError;
use super::Hunk;
use super::IoError;
use super::derive_new_contents_from_chunks;
use super::write_file_with_missing_parent_retry;

mod error;
mod rollback;
#[cfg(test)]
mod tests;

pub use error::PatchCommitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteKind {
    Add,
    Update,
}

#[derive(Debug)]
enum PreparedOperation {
    Write {
        kind: WriteKind,
        display_path: PathBuf,
        path: AbsolutePathBuf,
        contents: Vec<u8>,
        previous: Option<Vec<u8>>,
        missing_parents: Vec<AbsolutePathBuf>,
    },
    Delete {
        display_path: PathBuf,
        path: AbsolutePathBuf,
        previous: Vec<u8>,
    },
    Move {
        display_path: PathBuf,
        source: AbsolutePathBuf,
        destination: AbsolutePathBuf,
        contents: Vec<u8>,
        source_previous: Vec<u8>,
        destination_previous: Option<Vec<u8>>,
        missing_destination_parents: Vec<AbsolutePathBuf>,
    },
}

impl PreparedOperation {
    fn display_path(&self) -> &PathBuf {
        match self {
            Self::Write { display_path, .. }
            | Self::Delete { display_path, .. }
            | Self::Move { display_path, .. } => display_path,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedPatch {
    operations: Vec<PreparedOperation>,
}

pub(crate) async fn prepare_patch(
    hunks: &[Hunk],
    cwd: &AbsolutePathBuf,
    fs: &dyn ExecutorFileSystem,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<PreparedPatch, ApplyPatchError> {
    if hunks.is_empty() {
        return Err(validation_error("No files were modified."));
    }

    let mut operations = Vec::with_capacity(hunks.len());
    let mut claimed_paths = HashSet::new();
    for hunk in hunks {
        let display_path = hunk.path().to_path_buf();
        let source = hunk.resolve_path(cwd);
        match hunk {
            Hunk::AddFile { contents, .. } => {
                claim_path(&mut claimed_paths, &source)?;
                let missing_parents = missing_parent_directories(fs, &source, sandbox).await?;
                let previous = read_optional_regular_file(fs, &source, sandbox).await?;
                operations.push(PreparedOperation::Write {
                    kind: WriteKind::Add,
                    display_path,
                    path: source,
                    contents: contents.clone().into_bytes(),
                    previous,
                    missing_parents,
                });
            }
            Hunk::DeleteFile { .. } => {
                claim_path(&mut claimed_paths, &source)?;
                let previous =
                    read_required_regular_file(fs, &source, sandbox, "Failed to delete file")
                        .await?;
                operations.push(PreparedOperation::Delete {
                    display_path,
                    path: source,
                    previous,
                });
            }
            Hunk::UpdateFile {
                move_path, chunks, ..
            } => {
                claim_path(&mut claimed_paths, &source)?;
                reject_symlink_or_directory(fs, &source, sandbox, "update").await?;
                let AppliedPatch {
                    original_contents,
                    new_contents,
                } = derive_new_contents_from_chunks(&source, chunks, fs, sandbox).await?;
                if let Some(destination_path) = move_path {
                    let destination =
                        AbsolutePathBuf::resolve_path_against_base(destination_path, cwd);
                    if source == destination {
                        return Err(validation_error(format!(
                            "Patch move source and destination are identical: {}",
                            source.display()
                        )));
                    }
                    claim_path(&mut claimed_paths, &destination)?;
                    let missing_destination_parents =
                        missing_parent_directories(fs, &destination, sandbox).await?;
                    let destination_previous =
                        read_optional_regular_file(fs, &destination, sandbox).await?;
                    operations.push(PreparedOperation::Move {
                        display_path,
                        source,
                        destination,
                        contents: new_contents.into_bytes(),
                        source_previous: original_contents.into_bytes(),
                        destination_previous,
                        missing_destination_parents,
                    });
                } else {
                    operations.push(PreparedOperation::Write {
                        kind: WriteKind::Update,
                        display_path,
                        path: source,
                        contents: new_contents.into_bytes(),
                        previous: Some(original_contents.into_bytes()),
                        missing_parents: Vec::new(),
                    });
                }
            }
        }
    }
    Ok(PreparedPatch { operations })
}

pub(crate) async fn validate_preconditions(
    prepared: &PreparedPatch,
    fs: &dyn ExecutorFileSystem,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<(), ApplyPatchError> {
    for operation in &prepared.operations {
        match operation {
            PreparedOperation::Write { path, previous, .. } => {
                validate_preimage(fs, path, previous.as_deref(), sandbox).await?;
            }
            PreparedOperation::Delete { path, previous, .. } => {
                validate_preimage(fs, path, Some(previous), sandbox).await?;
            }
            PreparedOperation::Move {
                source,
                destination,
                source_previous,
                destination_previous,
                ..
            } => {
                validate_preimage(fs, source, Some(source_previous), sandbox).await?;
                validate_preimage(fs, destination, destination_previous.as_deref(), sandbox)
                    .await?;
            }
        }
    }
    Ok(())
}

pub(crate) async fn commit_patch(
    prepared: &PreparedPatch,
    fs: &dyn ExecutorFileSystem,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<AffectedPaths, PatchCommitError> {
    let mut committed: Vec<usize> = Vec::new();
    for (index, operation) in prepared.operations.iter().enumerate() {
        if let Err(error) = apply_operation(operation, fs, sandbox).await {
            let mut rollback_restored_paths = Vec::new();
            let mut rollback_failed_paths = Vec::new();
            rollback::rollback_operation(
                operation,
                fs,
                sandbox,
                &mut rollback_restored_paths,
                &mut rollback_failed_paths,
            )
            .await;
            for committed_index in committed.iter().rev().copied() {
                rollback::rollback_operation(
                    &prepared.operations[committed_index],
                    fs,
                    sandbox,
                    &mut rollback_restored_paths,
                    &mut rollback_failed_paths,
                )
                .await;
            }
            return Err(PatchCommitError {
                cause: error,
                committed_paths: committed
                    .iter()
                    .map(|committed_index| {
                        prepared.operations[*committed_index].display_path().clone()
                    })
                    .collect(),
                pending_paths: prepared.operations[index..]
                    .iter()
                    .map(|operation| operation.display_path().clone())
                    .collect(),
                rollback_restored_paths,
                rollback_failed_paths,
            });
        }
        committed.push(index);
    }
    Ok(affected_paths(&prepared.operations))
}

async fn apply_operation(
    operation: &PreparedOperation,
    fs: &dyn ExecutorFileSystem,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<(), String> {
    match operation {
        PreparedOperation::Write { path, contents, .. } => {
            write_file_with_missing_parent_retry(fs, path, contents.clone(), sandbox)
                .await
                .map_err(|error| error.to_string())
        }
        PreparedOperation::Delete { path, .. } => fs
            .remove(
                path,
                RemoveOptions {
                    recursive: false,
                    force: false,
                },
                sandbox,
            )
            .await
            .map_err(|error| format!("Failed to delete file {}: {error}", path.display())),
        PreparedOperation::Move {
            source,
            destination,
            contents,
            ..
        } => {
            write_file_with_missing_parent_retry(fs, destination, contents.clone(), sandbox)
                .await
                .map_err(|error| error.to_string())?;
            fs.remove(
                source,
                RemoveOptions {
                    recursive: false,
                    force: false,
                },
                sandbox,
            )
            .await
            .map_err(|error| format!("Failed to remove original {}: {error}", source.display()))
        }
    }
}

async fn validate_preimage(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    expected: Option<&[u8]>,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<(), ApplyPatchError> {
    let actual = read_optional_regular_file(fs, path, sandbox).await?;
    if actual.as_deref() != expected {
        return Err(validation_error(format!(
            "File changed after patch preparation: {}",
            path.display()
        )));
    }
    Ok(())
}

async fn read_required_regular_file(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    sandbox: Option<&FileSystemSandboxContext>,
    operation: &str,
) -> Result<Vec<u8>, ApplyPatchError> {
    let metadata = match fs.get_metadata(path, sandbox).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(validation_error(format!("{operation} {}", path.display())));
        }
        Err(source) => {
            return Err(ApplyPatchError::IoError(IoError {
                context: format!("{operation} {}", path.display()),
                source,
            }));
        }
    };
    if metadata.is_directory {
        return Err(validation_error(format!("{operation} {}", path.display())));
    }
    if metadata.is_symlink {
        return Err(validation_error(format!(
            "Patch path is a symbolic link and cannot be rolled back safely: {}",
            path.display()
        )));
    }
    fs.read_file(path, sandbox).await.map_err(|source| {
        ApplyPatchError::IoError(IoError {
            context: format!("{operation} {}", path.display()),
            source,
        })
    })
}

async fn read_optional_regular_file(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<Option<Vec<u8>>, ApplyPatchError> {
    match fs.get_metadata(path, sandbox).await {
        Ok(metadata) => {
            if metadata.is_directory {
                return Err(validation_error(format!(
                    "Patch path is a directory: {}",
                    path.display()
                )));
            }
            if metadata.is_symlink {
                return Err(validation_error(format!(
                    "Patch path is a symbolic link and cannot be rolled back safely: {}",
                    path.display()
                )));
            }
            fs.read_file(path, sandbox)
                .await
                .map(Some)
                .map_err(|source| {
                    ApplyPatchError::IoError(IoError {
                        context: format!("Failed to read patch path {}", path.display()),
                        source,
                    })
                })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ApplyPatchError::IoError(IoError {
            context: format!("Failed to inspect patch path {}", path.display()),
            source,
        })),
    }
}

async fn reject_symlink_or_directory(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    sandbox: Option<&FileSystemSandboxContext>,
    operation: &str,
) -> Result<(), ApplyPatchError> {
    let metadata = fs.get_metadata(path, sandbox).await.map_err(|source| {
        ApplyPatchError::IoError(IoError {
            context: format!("Failed to read file to {operation} {}", path.display()),
            source,
        })
    })?;
    if metadata.is_directory {
        return Err(validation_error(format!(
            "Patch path is a directory: {}",
            path.display()
        )));
    }
    if metadata.is_symlink {
        return Err(validation_error(format!(
            "Patch path is a symbolic link and cannot be rolled back safely: {}",
            path.display()
        )));
    }
    Ok(())
}

async fn missing_parent_directories(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    sandbox: Option<&FileSystemSandboxContext>,
) -> Result<Vec<AbsolutePathBuf>, ApplyPatchError> {
    let mut missing = Vec::new();
    let mut current = path.parent();
    while let Some(parent) = current {
        match fs.get_metadata(&parent, sandbox).await {
            Ok(metadata) if metadata.is_directory => break,
            Ok(_) => {
                return Err(validation_error(format!(
                    "Patch parent path is not a directory: {}",
                    parent.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                current = parent.parent();
                missing.push(parent);
            }
            Err(source) => {
                return Err(ApplyPatchError::IoError(IoError {
                    context: format!("Failed to inspect patch parent {}", parent.display()),
                    source,
                }));
            }
        }
    }
    Ok(missing)
}

fn claim_path(
    claimed_paths: &mut HashSet<PathBuf>,
    path: &AbsolutePathBuf,
) -> Result<(), ApplyPatchError> {
    if !claimed_paths.insert(path.to_path_buf()) {
        return Err(validation_error(format!(
            "Patch contains conflicting operations for {}",
            path.display()
        )));
    }
    Ok(())
}

fn affected_paths(operations: &[PreparedOperation]) -> AffectedPaths {
    let mut affected = AffectedPaths {
        added: Vec::new(),
        modified: Vec::new(),
        deleted: Vec::new(),
    };
    for operation in operations {
        match operation {
            PreparedOperation::Write {
                kind: WriteKind::Add,
                display_path,
                ..
            } => affected.added.push(display_path.clone()),
            PreparedOperation::Write {
                kind: WriteKind::Update,
                display_path,
                ..
            }
            | PreparedOperation::Move { display_path, .. } => {
                affected.modified.push(display_path.clone());
            }
            PreparedOperation::Delete { display_path, .. } => {
                affected.deleted.push(display_path.clone());
            }
        }
    }
    affected
}

fn validation_error(message: impl Into<String>) -> ApplyPatchError {
    ApplyPatchError::ComputeReplacements(message.into())
}
