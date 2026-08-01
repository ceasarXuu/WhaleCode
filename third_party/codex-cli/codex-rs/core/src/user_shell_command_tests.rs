use super::*;
use crate::context::ContextualUserFragment;
use crate::context::UserShellCommand;
use crate::session::tests::make_session_and_context;
use codex_protocol::exec_output::ExecOutcome;
use codex_protocol::exec_output::StreamOutput;
use codex_protocol::models::ContentItem;
use pretty_assertions::assert_eq;
use std::time::Duration;

#[test]
fn detects_user_shell_command_text_variants() {
    assert!(UserShellCommand::matches_text(
        "<user_shell_command>\necho hi\n</user_shell_command>"
    ));
    assert!(!UserShellCommand::matches_text("echo hi"));
}

#[tokio::test]
async fn formats_basic_record() {
    let exec_output = ExecToolCallOutput {
        exit_code: 0,
        outcome: ExecOutcome::Exited,
        termination_signal: None,
        pipeline_stage_exit_codes: None,
        stdout: StreamOutput::new("hi".to_string()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new("hi".to_string()),
        duration: Duration::from_secs(1),
    };
    let (_, turn_context) = make_session_and_context().await;
    let item = user_shell_command_record_item("echo hi", &exec_output, &turn_context);
    let ResponseItem::Message { content, .. } = item else {
        panic!("expected message");
    };
    let [ContentItem::InputText { text }] = content.as_slice() else {
        panic!("expected input text");
    };
    assert_eq!(
        text,
        "<user_shell_command>\n<command>\necho hi\n</command>\n<result>\nExecution outcome: exited\nShell exit code: 0\nPipeline stage exit codes: unavailable\nTermination signal: unavailable\nDuration: 1.0000 seconds\nOutput:\nhi\n</result>\n</user_shell_command>"
    );
}

#[tokio::test]
async fn uses_aggregated_output_over_streams() {
    let exec_output = ExecToolCallOutput {
        exit_code: 42,
        outcome: ExecOutcome::Exited,
        termination_signal: None,
        pipeline_stage_exit_codes: None,
        stdout: StreamOutput::new("stdout-only".to_string()),
        stderr: StreamOutput::new("stderr-only".to_string()),
        aggregated_output: StreamOutput::new("combined output wins".to_string()),
        duration: Duration::from_millis(120),
    };
    let (_, turn_context) = make_session_and_context().await;
    let record = format_user_shell_command_record("false", &exec_output, &turn_context);
    assert_eq!(
        record,
        "<user_shell_command>\n<command>\nfalse\n</command>\n<result>\nExecution outcome: exited\nShell exit code: 42\nPipeline stage exit codes: unavailable\nTermination signal: unavailable\nDuration: 0.1200 seconds\nOutput:\ncombined output wins\n</result>\n</user_shell_command>"
    );
}

#[tokio::test]
async fn does_not_publish_synthetic_exit_code_for_timeout() {
    let exec_output = ExecToolCallOutput {
        exit_code: 124,
        outcome: ExecOutcome::TimedOut,
        termination_signal: None,
        pipeline_stage_exit_codes: None,
        stdout: StreamOutput::new(String::new()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new("partial output".to_string()),
        duration: Duration::from_secs(2),
    };
    let (_, turn_context) = make_session_and_context().await;
    let record = format_user_shell_command_record("sleep 10", &exec_output, &turn_context);

    assert!(record.contains("Execution outcome: timed_out"));
    assert!(record.contains("Shell exit code: unavailable"));
    assert!(!record.contains("Shell exit code: 124"));
}
