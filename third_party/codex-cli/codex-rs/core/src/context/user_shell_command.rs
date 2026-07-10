use std::time::Duration;

use codex_protocol::exec_output::ExecOutcome;

use super::ContextualUserFragment;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UserShellCommand {
    pub(crate) command: String,
    pub(crate) outcome: ExecOutcome,
    pub(crate) shell_exit_code: Option<i32>,
    pub(crate) termination_signal: Option<i32>,
    pub(crate) pipeline_stage_exit_codes: Option<Vec<i32>>,
    pub(crate) duration_seconds: f64,
    pub(crate) output: String,
}

impl UserShellCommand {
    pub(crate) fn new(
        command: impl Into<String>,
        outcome: ExecOutcome,
        shell_exit_code: Option<i32>,
        termination_signal: Option<i32>,
        pipeline_stage_exit_codes: Option<Vec<i32>>,
        duration: Duration,
        output: impl Into<String>,
    ) -> Self {
        Self {
            command: command.into(),
            outcome,
            shell_exit_code,
            termination_signal,
            pipeline_stage_exit_codes,
            duration_seconds: duration.as_secs_f64(),
            output: output.into(),
        }
    }
}

impl ContextualUserFragment for UserShellCommand {
    const ROLE: &'static str = "user";
    const START_MARKER: &'static str = "<user_shell_command>";
    const END_MARKER: &'static str = "</user_shell_command>";

    fn body(&self) -> String {
        let shell_exit_code = self
            .shell_exit_code
            .map_or_else(|| "unavailable".to_string(), |code| code.to_string());
        let pipeline_stage_exit_codes = self.pipeline_stage_exit_codes.as_ref().map_or_else(
            || "unavailable".to_string(),
            |codes| {
                codes
                    .iter()
                    .map(i32::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            },
        );
        let termination_signal = self
            .termination_signal
            .map_or_else(|| "unavailable".to_string(), |signal| signal.to_string());
        format!(
            "\n<command>\n{}\n</command>\n<result>\nExecution outcome: {}\nShell exit code: {}\nPipeline stage exit codes: {}\nTermination signal: {}\nDuration: {:.4} seconds\nOutput:\n{}\n</result>\n",
            self.command,
            self.outcome.as_str(),
            shell_exit_code,
            pipeline_stage_exit_codes,
            termination_signal,
            self.duration_seconds,
            self.output,
        )
    }
}
