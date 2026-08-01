use std::path::PathBuf;

use codex_exec_server::ExecutorFileSystem;
use codex_exec_server::FileSystemSandboxContext;
use codex_exec_server::RemoveOptions;
use codex_utils_absolute_path::AbsolutePathBuf;

use super::PreparedOperation;
use crate::write_file_with_missing_parent_retry;

pub(super) async fn rollback_operation(
    operation: &PreparedOperation,
    fs: &dyn ExecutorFileSystem,
    sandbox: Option<&FileSystemSandboxContext>,
    restored: &mut Vec<PathBuf>,
    failed: &mut Vec<PathBuf>,
) {
    match operation {
        PreparedOperation::Write {
            path,
            previous,
            missing_parents,
            ..
        } => {
            restore_path(fs, path, previous.as_deref(), sandbox, restored, failed).await;
            remove_created_parents(fs, missing_parents, sandbox, failed).await;
        }
        PreparedOperation::Delete { path, previous, .. } => {
            restore_path(fs, path, Some(previous), sandbox, restored, failed).await;
        }
        PreparedOperation::Move {
            source,
            destination,
            source_previous,
            destination_previous,
            missing_destination_parents,
            ..
        } => {
            restore_path(fs, source, Some(source_previous), sandbox, restored, failed).await;
            restore_path(
                fs,
                destination,
                destination_previous.as_deref(),
                sandbox,
                restored,
                failed,
            )
            .await;
            remove_created_parents(fs, missing_destination_parents, sandbox, failed).await;
        }
    }
}

async fn restore_path(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    previous: Option<&[u8]>,
    sandbox: Option<&FileSystemSandboxContext>,
    restored: &mut Vec<PathBuf>,
    failed: &mut Vec<PathBuf>,
) {
    let restored_path = if let Some(previous) = previous {
        write_file_with_missing_parent_retry(fs, path, previous.to_vec(), sandbox)
            .await
            .is_ok()
    } else {
        fs.remove(
            path,
            RemoveOptions {
                recursive: false,
                force: true,
            },
            sandbox,
        )
        .await
        .is_ok()
    };
    if restored_path {
        restored.push(path.to_path_buf());
    } else {
        failed.push(path.to_path_buf());
    }
}

async fn remove_created_parents(
    fs: &dyn ExecutorFileSystem,
    missing_parents: &[AbsolutePathBuf],
    sandbox: Option<&FileSystemSandboxContext>,
    failed: &mut Vec<PathBuf>,
) {
    for path in missing_parents {
        if fs
            .remove(
                path,
                RemoveOptions {
                    recursive: false,
                    force: true,
                },
                sandbox,
            )
            .await
            .is_err()
        {
            failed.push(path.to_path_buf());
        }
    }
}
