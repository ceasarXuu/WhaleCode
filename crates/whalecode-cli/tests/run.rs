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
fn whale_status_reports_bootstrap_runtime() {
    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .arg("status")
        .output()
        .expect("run whale status");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("command: whale"));
    assert!(stdout.contains("runtime: bootstrap_agent_loop"));
}
