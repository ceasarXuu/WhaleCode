use std::collections::HashSet;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use codex_exec_server::CopyOptions;
use codex_exec_server::CreateDirectoryOptions;
use codex_exec_server::ExecutorFileSystem;
use codex_exec_server::FileMetadata;
use codex_exec_server::FileSystemResult;
use codex_exec_server::FileSystemSandboxContext;
use codex_exec_server::LOCAL_FS;
use codex_exec_server::ReadDirectoryEntry;
use codex_exec_server::RemoveOptions;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::test_support::PathExt;
use tempfile::tempdir;

use super::commit_patch;
use super::prepare_patch;
use super::validate_preconditions;
use crate::parse_patch;

struct FailingFileSystem {
    inner: Arc<dyn ExecutorFileSystem>,
    write_calls: AtomicUsize,
    remove_calls: AtomicUsize,
    fail_writes: Mutex<HashSet<usize>>,
    fail_removes: Mutex<HashSet<usize>>,
}

impl FailingFileSystem {
    fn new(fail_writes: &[usize], fail_removes: &[usize]) -> Self {
        Self {
            inner: LOCAL_FS.clone(),
            write_calls: AtomicUsize::new(0),
            remove_calls: AtomicUsize::new(0),
            fail_writes: Mutex::new(fail_writes.iter().copied().collect()),
            fail_removes: Mutex::new(fail_removes.iter().copied().collect()),
        }
    }

    fn should_fail(counter: &AtomicUsize, failures: &Mutex<HashSet<usize>>) -> bool {
        let call = counter.fetch_add(1, Ordering::SeqCst) + 1;
        failures.lock().unwrap().contains(&call)
    }
}

#[async_trait]
impl ExecutorFileSystem for FailingFileSystem {
    async fn read_file(
        &self,
        path: &AbsolutePathBuf,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<Vec<u8>> {
        self.inner.read_file(path, sandbox).await
    }

    async fn write_file(
        &self,
        path: &AbsolutePathBuf,
        contents: Vec<u8>,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        if Self::should_fail(&self.write_calls, &self.fail_writes) {
            return Err(io::Error::other("injected write failure"));
        }
        self.inner.write_file(path, contents, sandbox).await
    }

    async fn create_directory(
        &self,
        path: &AbsolutePathBuf,
        options: CreateDirectoryOptions,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        self.inner.create_directory(path, options, sandbox).await
    }

    async fn get_metadata(
        &self,
        path: &AbsolutePathBuf,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<FileMetadata> {
        self.inner.get_metadata(path, sandbox).await
    }

    async fn read_directory(
        &self,
        path: &AbsolutePathBuf,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<Vec<ReadDirectoryEntry>> {
        self.inner.read_directory(path, sandbox).await
    }

    async fn remove(
        &self,
        path: &AbsolutePathBuf,
        options: RemoveOptions,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        if Self::should_fail(&self.remove_calls, &self.fail_removes) {
            return Err(io::Error::other("injected remove failure"));
        }
        self.inner.remove(path, options, sandbox).await
    }

    async fn copy(
        &self,
        source_path: &AbsolutePathBuf,
        destination_path: &AbsolutePathBuf,
        options: CopyOptions,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        self.inner
            .copy(source_path, destination_path, options, sandbox)
            .await
    }
}

#[tokio::test]
async fn commit_failure_rolls_back_current_and_prior_writes() {
    let dir = tempdir().unwrap();
    let cwd = dir.path().abs();
    let first = dir.path().join("first.txt");
    let second = dir.path().join("second.txt");
    std::fs::write(&first, "first before\n").unwrap();
    std::fs::write(&second, "second before\n").unwrap();
    let patch = parse_patch(
        "*** Begin Patch\n*** Update File: first.txt\n@@\n-first before\n+first after\n*** Update File: second.txt\n@@\n-second before\n+second after\n*** End Patch",
    )
    .unwrap();
    let fs = FailingFileSystem::new(&[2], &[]);

    let prepared = prepare_patch(&patch.hunks, &cwd, &fs, None).await.unwrap();
    validate_preconditions(&prepared, &fs, None).await.unwrap();
    let error = commit_patch(&prepared, &fs, None).await.unwrap_err();

    assert_eq!(std::fs::read_to_string(first).unwrap(), "first before\n");
    assert_eq!(std::fs::read_to_string(second).unwrap(), "second before\n");
    assert_eq!(
        error.committed_paths,
        vec![std::path::PathBuf::from("first.txt")]
    );
    assert!(error.rollback_failed_paths.is_empty());
    assert!(
        error
            .to_string()
            .contains("rollback_status=best_effort_restored")
    );
}

#[tokio::test]
async fn rollback_failure_is_reported_without_hiding_committed_paths() {
    let dir = tempdir().unwrap();
    let cwd = dir.path().abs();
    let first = dir.path().join("first.txt");
    let second = dir.path().join("second.txt");
    std::fs::write(&first, "first before\n").unwrap();
    std::fs::write(&second, "second before\n").unwrap();
    let patch = parse_patch(
        "*** Begin Patch\n*** Update File: first.txt\n@@\n-first before\n+first after\n*** Update File: second.txt\n@@\n-second before\n+second after\n*** End Patch",
    )
    .unwrap();
    let fs = FailingFileSystem::new(&[2, 3], &[]);

    let prepared = prepare_patch(&patch.hunks, &cwd, &fs, None).await.unwrap();
    validate_preconditions(&prepared, &fs, None).await.unwrap();
    let error = commit_patch(&prepared, &fs, None).await.unwrap_err();

    assert_eq!(std::fs::read_to_string(first).unwrap(), "first before\n");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "second before\n");
    assert!(error.rollback_failed_paths.contains(&second));
    assert!(
        error
            .to_string()
            .contains("rollback_status=best_effort_partial")
    );
}

#[tokio::test]
async fn move_remove_failure_restores_destination_and_preserves_source() {
    let dir = tempdir().unwrap();
    let cwd = dir.path().abs();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("destination.txt");
    std::fs::write(&source, "source before\n").unwrap();
    std::fs::write(&destination, "destination before\n").unwrap();
    let patch = parse_patch(
        "*** Begin Patch\n*** Update File: source.txt\n*** Move to: destination.txt\n@@\n-source before\n+source after\n*** End Patch",
    )
    .unwrap();
    let fs = FailingFileSystem::new(&[], &[1]);

    let prepared = prepare_patch(&patch.hunks, &cwd, &fs, None).await.unwrap();
    validate_preconditions(&prepared, &fs, None).await.unwrap();
    let error = commit_patch(&prepared, &fs, None).await.unwrap_err();

    assert_eq!(std::fs::read_to_string(source).unwrap(), "source before\n");
    assert_eq!(
        std::fs::read_to_string(destination).unwrap(),
        "destination before\n"
    );
    assert!(error.rollback_failed_paths.is_empty());
}

#[tokio::test]
async fn precondition_change_is_rejected_before_commit() {
    let dir = tempdir().unwrap();
    let cwd = dir.path().abs();
    let path = dir.path().join("changed.txt");
    std::fs::write(&path, "before\n").unwrap();
    let patch = parse_patch(
        "*** Begin Patch\n*** Update File: changed.txt\n@@\n-before\n+after\n*** End Patch",
    )
    .unwrap();

    let prepared = prepare_patch(&patch.hunks, &cwd, LOCAL_FS.as_ref(), None)
        .await
        .unwrap();
    std::fs::write(&path, "external change\n").unwrap();
    let error = validate_preconditions(&prepared, LOCAL_FS.as_ref(), None)
        .await
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("File changed after patch preparation")
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "external change\n");
}
