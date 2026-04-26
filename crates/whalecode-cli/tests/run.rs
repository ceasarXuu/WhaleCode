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
            "--bootstrap",
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
            "--bootstrap",
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
fn whale_logs_replays_session_event_trace() {
    let repo = tempdir().expect("repo");
    std::fs::write(repo.path().join("README.md"), "# Fixture\n").expect("write readme");
    let session_path = repo.path().join("session.jsonl");

    let run_output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "run",
            "--bootstrap",
            "inspect fixture",
            "--cwd",
            repo.path().to_str().expect("repo path"),
            "--session",
            session_path.to_str().expect("session path"),
        ])
        .output()
        .expect("run whale");
    assert!(run_output.status.success());

    let logs_output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "logs",
            "--session",
            session_path.to_str().expect("session path"),
        ])
        .output()
        .expect("run whale logs");

    assert!(logs_output.status.success());
    let stdout = String::from_utf8(logs_output.stdout).expect("stdout utf8");
    assert!(stdout.contains("session:"));
    assert!(stdout.contains("turn started index=1"));
    assert!(stdout.contains("tool output"));
    assert!(stdout.contains("assistant"));
}

#[test]
fn whale_status_reports_live_runtime() {
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
    assert!(stdout.contains("runtime: live_deepseek_tool_loop"));
    assert!(stdout.contains("live_model_smoke: whale model-smoke"));
    assert!(stdout.contains("session_logs: whale logs"));
    assert!(stdout.contains("bootstrap_debug: whale run --bootstrap"));
}

#[test]
fn whale_model_smoke_requires_explicit_deepseek_api_key() {
    let whale_home = tempdir().expect("whale home");
    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args(["model-smoke", "hello"])
        .env_remove("DEEPSEEK_API_KEY")
        .env("WHALE_SECRET_HOME", whale_home.path())
        .output()
        .expect("run whale model-smoke");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("DEEPSEEK_API_KEY"));
}

#[test]
fn whale_model_smoke_treats_empty_deepseek_api_key_as_missing() {
    let whale_home = tempdir().expect("whale home");
    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args(["model-smoke", "hello"])
        .env("DEEPSEEK_API_KEY", "")
        .env("WHALE_SECRET_HOME", whale_home.path())
        .output()
        .expect("run whale model-smoke");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("DEEPSEEK_API_KEY"));
}

#[test]
fn whale_run_requires_explicit_deepseek_api_key_for_any_natural_input() {
    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    let session_path = repo.path().join("session.jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "run",
            "hi",
            "--cwd",
            repo.path().to_str().expect("repo path"),
            "--session",
            session_path.to_str().expect("session path"),
        ])
        .env_remove("DEEPSEEK_API_KEY")
        .env("WHALE_SECRET_HOME", whale_home.path())
        .output()
        .expect("run whale");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stdout.is_empty());
    assert!(stderr.contains("DeepSeek API key is required"));
    assert!(session_path.exists());
}

#[test]
fn whale_run_actionable_task_requires_explicit_deepseek_api_key_by_default() {
    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
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
        .env_remove("DEEPSEEK_API_KEY")
        .env("WHALE_SECRET_HOME", whale_home.path())
        .output()
        .expect("run whale");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("DEEPSEEK_API_KEY"));
    assert!(session_path.exists());
}

#[test]
fn whale_interactive_apikey_stores_key_outside_repo() {
    use std::{io::Write, process::Stdio};

    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_whale"))
        .current_dir(repo.path())
        .env("WHALE_SECRET_HOME", whale_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn whale");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"/apikey\nstored-test-key\n/exit\n")
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait whale");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("DeepSeek API key saved to user secret store"));

    let secret_path = whale_home.path().join("secrets/deepseek_api_key");
    assert_eq!(
        std::fs::read_to_string(&secret_path).expect("read secret"),
        "stored-test-key"
    );
    assert!(!repo.path().join(".whale/secrets/deepseek_api_key").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&secret_path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn whale_interactive_defaults_to_live_agent_instead_of_bootstrap() {
    use std::{io::Write, process::Stdio};

    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_whale"))
        .current_dir(repo.path())
        .env_remove("DEEPSEEK_API_KEY")
        .env("WHALE_SECRET_HOME", whale_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn whale");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"inspect this repository\n/exit\n")
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait whale");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Whale live agent"));
    assert!(stdout.contains("DeepSeek API key is required"));
    assert!(stdout.contains("Run /apikey"));
    assert!(!stdout.contains("Bootstrap agent accepted the task"));
}

#[test]
fn whale_interactive_sends_greeting_to_live_agent() {
    use std::{io::Write, process::Stdio};

    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_whale"))
        .current_dir(repo.path())
        .env_remove("DEEPSEEK_API_KEY")
        .env("WHALE_HOME", whale_home.path())
        .env("WHALE_SECRET_HOME", whale_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn whale");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"hi\n/exit\n").expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait whale");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("DeepSeek API key is required"));
    assert!(whale_home.path().join("sessions").exists());
}

#[test]
fn whale_interactive_reports_permission_gates() {
    use std::{io::Write, process::Stdio};

    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_whale"))
        .current_dir(repo.path())
        .env("WHALE_SECRET_HOME", whale_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn whale");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"/permissions\n/command on\n/write off\n/permissions\n/exit\n")
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait whale");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("mode: live"));
    assert!(stdout.contains("edit_file: enabled"));
    assert!(stdout.contains("run_command: disabled"));
    assert!(stdout.contains("run_command: enabled"));
    assert!(stdout.contains("edit_file: disabled"));
}
