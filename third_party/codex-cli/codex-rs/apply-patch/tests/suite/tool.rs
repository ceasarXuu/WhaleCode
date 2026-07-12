use assert_cmd::Command;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

fn run_apply_patch_in_dir(dir: &Path, patch: &str) -> anyhow::Result<assert_cmd::assert::Assert> {
    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("apply_patch")?);
    cmd.current_dir(dir);
    Ok(cmd.arg(patch).assert())
}

fn apply_patch_command(dir: &Path) -> anyhow::Result<Command> {
    let mut cmd = Command::new(codex_utils_cargo_bin::cargo_bin("apply_patch")?);
    cmd.current_dir(dir);
    Ok(cmd)
}

fn resolved_under(root: &Path, path: &str) -> anyhow::Result<PathBuf> {
    Ok(root.canonicalize()?.join(path))
}

#[test]
fn test_apply_patch_cli_applies_multiple_operations() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let add_path = tmp.path().join("nested/new.txt");
    let modify_path = tmp.path().join("modify.txt");
    let delete_path = tmp.path().join("delete.txt");

    fs::write(&modify_path, "line1\nline2\n")?;
    fs::write(&delete_path, "obsolete\n")?;

    let patch = "*** Begin Patch\n*** Add File: nested/new.txt\n+created\n*** Delete File: delete.txt\n*** Update File: modify.txt\n@@\n-line2\n+changed\n*** End Patch";

    run_apply_patch_in_dir(tmp.path(), patch)?.success().stdout(
        "Success. Updated the following files:\nA nested/new.txt\nM modify.txt\nD delete.txt\n",
    );

    assert_eq!(fs::read_to_string(add_path)?, "created\n");
    assert_eq!(fs::read_to_string(&modify_path)?, "line1\nchanged\n");
    assert!(!delete_path.exists());

    Ok(())
}

#[test]
fn test_apply_patch_cli_applies_multiple_chunks() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let target_path = tmp.path().join("multi.txt");
    fs::write(&target_path, "line1\nline2\nline3\nline4\n")?;

    let patch = "*** Begin Patch\n*** Update File: multi.txt\n@@\n-line2\n+changed2\n@@\n-line4\n+changed4\n*** End Patch";

    run_apply_patch_in_dir(tmp.path(), patch)?
        .success()
        .stdout("Success. Updated the following files:\nM multi.txt\n");

    assert_eq!(
        fs::read_to_string(&target_path)?,
        "line1\nchanged2\nline3\nchanged4\n"
    );

    Ok(())
}

#[test]
fn test_apply_patch_cli_moves_file_to_new_directory() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let original_path = tmp.path().join("old/name.txt");
    let new_path = tmp.path().join("renamed/dir/name.txt");
    fs::create_dir_all(original_path.parent().expect("parent should exist"))?;
    fs::write(&original_path, "old content\n")?;

    let patch = "*** Begin Patch\n*** Update File: old/name.txt\n*** Move to: renamed/dir/name.txt\n@@\n-old content\n+new content\n*** End Patch";

    run_apply_patch_in_dir(tmp.path(), patch)?
        .success()
        .stdout("Success. Updated the following files:\nM renamed/dir/name.txt\n");

    assert!(!original_path.exists());
    assert_eq!(fs::read_to_string(&new_path)?, "new content\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_empty_patch() -> anyhow::Result<()> {
    let tmp = tempdir()?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** End Patch")
        .assert()
        .failure()
        .stderr("No files were modified.\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_reports_missing_context() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let target_path = tmp.path().join("modify.txt");
    let expected_target_path = resolved_under(tmp.path(), "modify.txt")?;
    fs::write(&target_path, "line1\nline2\n")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Update File: modify.txt\n@@\n-missing\n+changed\n*** End Patch")
        .assert()
        .failure()
        .stderr(format!(
            "Failed to find expected lines in {}:\nmissing\n",
            expected_target_path.display()
        ));
    assert_eq!(fs::read_to_string(&target_path)?, "line1\nline2\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_missing_file_delete() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let missing_path = resolved_under(tmp.path(), "missing.txt")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Delete File: missing.txt\n*** End Patch")
        .assert()
        .failure()
        .stderr(format!(
            "Failed to delete file {}\n",
            missing_path.display()
        ));

    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_empty_update_hunk() -> anyhow::Result<()> {
    let tmp = tempdir()?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Update File: foo.txt\n*** End Patch")
        .assert()
        .failure()
        .stderr("Invalid patch hunk on line 2: Update file hunk for path 'foo.txt' is empty\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_requires_existing_file_for_update() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let missing_path = resolved_under(tmp.path(), "missing.txt")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Update File: missing.txt\n@@\n-old\n+new\n*** End Patch")
        .assert()
        .failure()
        .stderr(format!(
            "Failed to read file to update {}: No such file or directory (os error 2)\n",
            missing_path.display()
        ));

    Ok(())
}

#[test]
fn test_apply_patch_cli_move_overwrites_existing_destination() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let original_path = tmp.path().join("old/name.txt");
    let destination = tmp.path().join("renamed/dir/name.txt");
    fs::create_dir_all(original_path.parent().expect("parent should exist"))?;
    fs::create_dir_all(destination.parent().expect("parent should exist"))?;
    fs::write(&original_path, "from\n")?;
    fs::write(&destination, "existing\n")?;

    run_apply_patch_in_dir(
        tmp.path(),
        "*** Begin Patch\n*** Update File: old/name.txt\n*** Move to: renamed/dir/name.txt\n@@\n-from\n+new\n*** End Patch",
    )?
    .success()
    .stdout("Success. Updated the following files:\nM renamed/dir/name.txt\n");

    assert!(!original_path.exists());
    assert_eq!(fs::read_to_string(&destination)?, "new\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_add_overwrites_existing_file() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let path = tmp.path().join("duplicate.txt");
    fs::write(&path, "old content\n")?;

    run_apply_patch_in_dir(
        tmp.path(),
        "*** Begin Patch\n*** Add File: duplicate.txt\n+new content\n*** End Patch",
    )?
    .success()
    .stdout("Success. Updated the following files:\nA duplicate.txt\n");

    assert_eq!(fs::read_to_string(&path)?, "new content\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_delete_directory_fails() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let dir = tmp.path().join("dir");
    let expected_dir = resolved_under(tmp.path(), "dir")?;
    fs::create_dir(&dir)?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Delete File: dir\n*** End Patch")
        .assert()
        .failure()
        .stderr(format!(
            "Failed to delete file {}\n",
            expected_dir.display()
        ));

    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_invalid_hunk_header() -> anyhow::Result<()> {
    let tmp = tempdir()?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Frobnicate File: foo\n*** End Patch")
        .assert()
        .failure()
        .stderr("Invalid patch hunk on line 2: '*** Frobnicate File: foo' is not a valid hunk header. Valid hunk headers: '*** Add File: {path}', '*** Delete File: {path}', '*** Update File: {path}'\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_updates_file_appends_trailing_newline() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let target_path = tmp.path().join("no_newline.txt");
    fs::write(&target_path, "no newline at end")?;

    run_apply_patch_in_dir(
        tmp.path(),
        "*** Begin Patch\n*** Update File: no_newline.txt\n@@\n-no newline at end\n+first line\n+second line\n*** End Patch",
    )?
    .success()
    .stdout("Success. Updated the following files:\nM no_newline.txt\n");

    let contents = fs::read_to_string(&target_path)?;
    assert!(contents.ends_with('\n'));
    assert_eq!(contents, "first line\nsecond line\n");

    Ok(())
}

#[test]
fn test_apply_patch_cli_validation_failure_has_zero_side_effects() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let new_file = tmp.path().join("created.txt");
    let missing_file = resolved_under(tmp.path(), "missing.txt")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Add File: created.txt\n+hello\n*** Update File: missing.txt\n@@\n-old\n+new\n*** End Patch")
        .assert()
        .failure()
        .stdout("")
        .stderr(format!(
            "Failed to read file to update {}: No such file or directory (os error 2)\n",
            missing_file.display()
        ));

    assert!(!new_file.exists());

    Ok(())
}

#[test]
fn test_apply_patch_cli_late_delete_validation_preserves_earlier_update() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let existing_file = tmp.path().join("existing.txt");
    let missing_file = resolved_under(tmp.path(), "missing.txt")?;
    fs::write(&existing_file, "before\n")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Update File: existing.txt\n@@\n-before\n+after\n*** Delete File: missing.txt\n*** End Patch")
        .assert()
        .failure()
        .stdout("")
        .stderr(format!("Failed to delete file {}\n", missing_file.display()));

    assert_eq!(fs::read_to_string(existing_file)?, "before\n");
    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_conflicting_paths_before_writing() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let path = resolved_under(tmp.path(), "same.txt")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Add File: same.txt\n+first\n*** Add File: same.txt\n+second\n*** End Patch")
        .assert()
        .failure()
        .stdout("")
        .stderr(format!(
            "Patch contains conflicting operations for {}\n",
            path.display()
        ));

    assert!(!path.exists());
    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_move_to_same_path_without_writing() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let path = resolved_under(tmp.path(), "same.txt")?;
    fs::write(&path, "before\n")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Update File: same.txt\n*** Move to: same.txt\n@@\n-before\n+after\n*** End Patch")
        .assert()
        .failure()
        .stdout("")
        .stderr(format!(
            "Patch move source and destination are identical: {}\n",
            path.display()
        ));

    assert_eq!(fs::read_to_string(path)?, "before\n");
    Ok(())
}

#[test]
fn test_apply_patch_cli_rejects_non_directory_parent_before_writing() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let parent = resolved_under(tmp.path(), "parent")?;
    fs::write(&parent, "not a directory\n")?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Add File: parent/child.txt\n+child\n*** End Patch")
        .assert()
        .failure()
        .stdout("")
        .stderr(format!(
            "Patch parent path is not a directory: {}\n",
            parent.display()
        ));

    assert_eq!(fs::read_to_string(parent)?, "not a directory\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_apply_patch_cli_rejects_symlink_before_writing() -> anyhow::Result<()> {
    use std::os::unix::fs::symlink;

    let tmp = tempdir()?;
    let target = resolved_under(tmp.path(), "target.txt")?;
    let link = resolved_under(tmp.path(), "link.txt")?;
    fs::write(&target, "before\n")?;
    symlink(&target, &link)?;

    apply_patch_command(tmp.path())?
        .arg("*** Begin Patch\n*** Update File: link.txt\n@@\n-before\n+after\n*** End Patch")
        .assert()
        .failure()
        .stdout("")
        .stderr(format!(
            "Patch path is a symbolic link and cannot be rolled back safely: {}\n",
            link.display()
        ));

    assert_eq!(fs::read_to_string(target)?, "before\n");
    assert!(fs::symlink_metadata(link)?.file_type().is_symlink());
    Ok(())
}
