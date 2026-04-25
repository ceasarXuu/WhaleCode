use tempfile::tempdir;
use whalecode_patch::{
    PatchOperation, PatchRejectReason, PatchRequest, WorkspacePatchEngine, WorkspacePatchStatus,
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
