use tempfile::tempdir;
use whalecode_core::{run_bootstrap_agent, AgentError};
use whalecode_session::{replay_jsonl, TranscriptRole};

#[test]
fn bootstrap_agent_persists_replayable_session() {
    let repo = tempdir().expect("repo");
    std::fs::write(repo.path().join("README.md"), "# Fixture\n").expect("write readme");
    std::fs::write(repo.path().join("AGENTS.md"), "# Instructions\n").expect("write agents");
    let session_path = repo.path().join(".test-sessions/session.jsonl");

    let summary = run_bootstrap_agent(
        "summarize this fixture",
        repo.path(),
        Some(session_path.clone()),
    )
    .expect("run bootstrap agent");

    assert!(summary.final_message.contains("summarize this fixture"));
    assert!(summary.final_message.contains("Repository observation"));
    assert!(summary.session_path.exists());
    assert!(summary.events_written >= 10);

    let replay = replay_jsonl(&session_path).expect("replay");
    assert_eq!(replay.transcript[0].role, TranscriptRole::User);
    assert_eq!(replay.transcript[0].content, "summarize this fixture");
    assert_eq!(replay.transcript[1].role, TranscriptRole::Assistant);
    assert!(replay.tool_event_count >= 4);
}

#[test]
fn bootstrap_agent_rejects_empty_tasks() {
    let repo = tempdir().expect("repo");
    let session_path = repo.path().join("session.jsonl");

    let err = run_bootstrap_agent("   ", repo.path(), Some(session_path)).expect_err("empty task");

    assert!(matches!(err, AgentError::EmptyTask));
}
