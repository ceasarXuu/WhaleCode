use tempfile::tempdir;
use whalecode_patch::{
    PatchOperation, PatchRejectReason, PatchRequest, WorkspacePatchEngine, WorkspacePatchStatus,
    WriteFileRequest,
};

#[test]
fn dry_run_replace_one_returns_diff_without_mutating_file() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("src.txt");
    std::fs::write(&path, "hello whale\n").expect("write file");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let snapshot = engine.snapshot("src.txt").expect("snapshot");

    let preview = engine
        .dry_run_apply(&PatchRequest {
            path: "src.txt".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "hello whale".to_owned(),
                new: "hello codex".to_owned(),
            },
        })
        .expect("dry run");

    assert_eq!(preview.status, WorkspacePatchStatus::Applied);
    assert_eq!(preview.touched_files, vec!["src.txt"]);
    assert!(preview.diff.contains("-hello whale"));
    assert!(preview.diff.contains("+hello codex"));
    assert_eq!(
        std::fs::read_to_string(path).expect("read file"),
        "hello whale\n"
    );
}

#[test]
fn apply_replace_one_mutates_file_after_snapshot_validation() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("src.txt");
    std::fs::write(&path, "hello whale\n").expect("write file");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let snapshot = engine.snapshot("src.txt").expect("snapshot");

    let preview = engine
        .apply(&PatchRequest {
            path: "src.txt".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "hello whale".to_owned(),
                new: "hello codex".to_owned(),
            },
        })
        .expect("apply");

    assert_eq!(preview.status, WorkspacePatchStatus::Applied);
    assert_eq!(
        std::fs::read_to_string(path).expect("read file"),
        "hello codex\n"
    );
}

#[test]
fn stale_read_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("src.txt");
    std::fs::write(&path, "old\n").expect("write file");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let snapshot = engine.snapshot("src.txt").expect("snapshot");
    std::fs::write(&path, "changed\n").expect("mutate after snapshot");

    let preview = engine
        .dry_run_apply(&PatchRequest {
            path: "src.txt".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "old".to_owned(),
                new: "new".to_owned(),
            },
        })
        .expect("dry run");

    assert_eq!(
        preview.status,
        WorkspacePatchStatus::Rejected {
            reason: PatchRejectReason::StaleRead
        }
    );
}

#[test]
fn apply_revalidates_stale_read_before_writing() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("src.txt");
    std::fs::write(&path, "old\n").expect("write file");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let snapshot = engine.snapshot("src.txt").expect("snapshot");
    std::fs::write(&path, "changed\n").expect("mutate after snapshot");

    let preview = engine
        .apply(&PatchRequest {
            path: "src.txt".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "old".to_owned(),
                new: "new".to_owned(),
            },
        })
        .expect("apply");

    assert_eq!(
        preview.status,
        WorkspacePatchStatus::Rejected {
            reason: PatchRejectReason::StaleRead
        }
    );
    assert_eq!(
        std::fs::read_to_string(path).expect("read file"),
        "changed\n"
    );
}

#[test]
fn non_unique_replacement_target_is_rejected() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("src.txt"), "same\nsame\n").expect("write file");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let snapshot = engine.snapshot("src.txt").expect("snapshot");

    let preview = engine
        .dry_run_apply(&PatchRequest {
            path: "src.txt".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "same".to_owned(),
                new: "other".to_owned(),
            },
        })
        .expect("dry run");

    assert_eq!(
        preview.status,
        WorkspacePatchStatus::Rejected {
            reason: PatchRejectReason::OldStringNotUnique
        }
    );
}

#[test]
fn parent_directory_escape_is_rejected() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("src.txt"), "content\n").expect("write file");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let mut snapshot = engine.snapshot("src.txt").expect("snapshot");
    snapshot.path = "../src.txt".to_owned();

    let preview = engine
        .dry_run_apply(&PatchRequest {
            path: "../src.txt".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "content".to_owned(),
                new: "changed".to_owned(),
            },
        })
        .expect("dry run");

    assert_eq!(
        preview.status,
        WorkspacePatchStatus::Rejected {
            reason: PatchRejectReason::OutsideWorkspace
        }
    );
}

#[test]
fn hidden_agent_config_path_is_rejected() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("src.txt"), "content\n").expect("write file");
    std::fs::create_dir(dir.path().join(".claude")).expect("mkdir claude");
    std::fs::write(dir.path().join(".claude/settings.local.json"), "{}").expect("write settings");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let mut snapshot = engine.snapshot("src.txt").expect("snapshot");
    snapshot.path = ".claude/settings.local.json".to_owned();

    let preview = engine
        .dry_run_apply(&PatchRequest {
            path: ".claude/settings.local.json".to_owned(),
            expected_snapshot: snapshot,
            operation: PatchOperation::ReplaceOne {
                old: "{}".to_owned(),
                new: "{\"x\":true}".to_owned(),
            },
        })
        .expect("dry run");

    assert_eq!(
        preview.status,
        WorkspacePatchStatus::Rejected {
            reason: PatchRejectReason::HiddenPath
        }
    );
}

#[test]
fn write_file_creates_new_nested_file_inside_workspace() {
    let dir = tempdir().expect("tempdir");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");

    let preview = engine
        .write_file(&WriteFileRequest {
            path: "src/index.html".to_owned(),
            content: "<h1>Whale</h1>\n".to_owned(),
            create_parent_dirs: true,
        })
        .expect("write file");

    assert_eq!(preview.status, WorkspacePatchStatus::Applied);
    assert_eq!(preview.touched_files, vec!["src/index.html"]);
    assert!(preview.diff.contains("--- /dev/null"));
    assert!(preview.diff.contains("+++ b/src/index.html"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("src/index.html")).expect("read file"),
        "<h1>Whale</h1>\n"
    );
}

#[test]
fn write_file_rejects_hidden_paths() {
    let dir = tempdir().expect("tempdir");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");

    let preview = engine
        .write_file(&WriteFileRequest {
            path: ".claude/settings.local.json".to_owned(),
            content: "{}".to_owned(),
            create_parent_dirs: true,
        })
        .expect("write file");

    assert_eq!(
        preview.status,
        WorkspacePatchStatus::Rejected {
            reason: PatchRejectReason::HiddenPath
        }
    );
    assert!(!dir.path().join(".claude/settings.local.json").exists());
}

#[test]
fn write_file_accepts_absolute_path_inside_workspace() {
    let dir = tempdir().expect("tempdir");
    let engine = WorkspacePatchEngine::new(dir.path()).expect("engine");
    let absolute_path = dir.path().join("index.html");

    let preview = engine
        .write_file(&WriteFileRequest {
            path: absolute_path.display().to_string(),
            content: "<!doctype html>\n".to_owned(),
            create_parent_dirs: true,
        })
        .expect("write file");

    assert_eq!(preview.status, WorkspacePatchStatus::Applied);
    assert_eq!(preview.touched_files, vec!["index.html"]);
    assert_eq!(
        std::fs::read_to_string(absolute_path).expect("read index"),
        "<!doctype html>\n"
    );
}
