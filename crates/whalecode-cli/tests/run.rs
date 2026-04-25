use std::process::Command;

use tempfile::tempdir;

#[test]
fn whale_run_executes_bootstrap_agent_and_writes_session() {
    let repo = tempdir().expect("repo");
    std::fs::write(repo.path().join("README.md"), "# Fixture\n").expect("write readme");
    let session_path = repo.path().join("session.jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "run",
            "inspect fixture",
            "--cwd",
            repo.path().to_str().expect("repo path"),
            "--session",
            session_path.to_str().expect("session path"),
        ])
        .output()
        .expect("run whale");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Bootstrap agent accepted the task"));
    assert!(stdout.contains("session:"));
    assert!(session_path.exists());
}

#[test]
fn whale_run_defaults_workspace_to_process_current_directory() {
    let repo = tempdir().expect("repo");
    std::fs::write(repo.path().join("README.md"), "# Workspace Root\n").expect("write readme");
    let session_path = repo.path().join("session.jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .current_dir(repo.path())
        .args([
            "run",
            "inspect default workspace",
            "--session",
            session_path.to_str().expect("session path"),
        ])
        .output()
        .expect("run whale");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Read README.md"));
    assert!(session_path.exists());
}

#[test]
fn whale_status_reports_bootstrap_runtime() {
    let repo = tempdir().expect("repo");

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .current_dir(repo.path())
        .arg("status")
        .output()
        .expect("run whale status");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("command: whale"));
    let canonical_workspace = repo.path().canonicalize().expect("canonical workspace");
    assert!(stdout.contains(&format!("workspace: {}", canonical_workspace.display())));
    assert!(stdout.contains("runtime: bootstrap_agent_loop"));
    assert!(stdout.contains("live_model_smoke: whale model-smoke"));
}

#[test]
fn whale_model_smoke_requires_explicit_deepseek_api_key() {
    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args(["model-smoke", "hello"])
        .env_remove("DEEPSEEK_API_KEY")
        .output()
        .expect("run whale model-smoke");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("DEEPSEEK_API_KEY"));
}

#[test]
fn whale_model_smoke_treats_empty_deepseek_api_key_as_missing() {
    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args(["model-smoke", "hello"])
        .env("DEEPSEEK_API_KEY", "")
        .output()
        .expect("run whale model-smoke");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("DEEPSEEK_API_KEY"));
}

#[test]
fn whale_run_live_requires_explicit_deepseek_api_key() {
    let repo = tempdir().expect("repo");
    std::fs::write(repo.path().join("README.md"), "# Fixture\n").expect("write readme");
    let session_path = repo.path().join("session.jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "run",
            "--live",
            "inspect fixture",
            "--cwd",
            repo.path().to_str().expect("repo path"),
            "--session",
            session_path.to_str().expect("session path"),
        ])
        .env_remove("DEEPSEEK_API_KEY")
        .output()
        .expect("run whale");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("DEEPSEEK_API_KEY"));
    assert!(session_path.exists());
}
