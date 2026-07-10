use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::sandboxing::ToolError;
use codex_protocol::error::CodexErr;
use codex_protocol::error::SandboxErr;
use codex_protocol::exec_output::ExecOutcome;
use codex_protocol::exec_output::ExecToolCallOutput;
use codex_protocol::exec_output::StreamOutput;
use codex_protocol::parse_command::ParsedCommand;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecCommandBeginEvent;
use codex_protocol::protocol::ExecCommandEndEvent;
use codex_protocol::protocol::ExecCommandSource;
use codex_protocol::protocol::ExecCommandStatus;
use codex_protocol::protocol::FileChange;
use codex_protocol::protocol::PatchApplyBeginEvent;
use codex_protocol::protocol::PatchApplyEndEvent;
use codex_protocol::protocol::PatchApplyStatus;
use codex_protocol::protocol::TurnDiffEvent;
use codex_shell_command::parse_command::parse_command;
use codex_utils_absolute_path::AbsolutePathBuf;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::format_exec_output_str;

#[derive(Clone, Copy)]
pub(crate) struct ToolEventCtx<'a> {
    pub session: &'a Session,
    pub turn: &'a TurnContext,
    pub call_id: &'a str,
    pub turn_diff_tracker: Option<&'a SharedTurnDiffTracker>,
}

impl<'a> ToolEventCtx<'a> {
    pub fn new(
        session: &'a Session,
        turn: &'a TurnContext,
        call_id: &'a str,
        turn_diff_tracker: Option<&'a SharedTurnDiffTracker>,
    ) -> Self {
        Self {
            session,
            turn,
            call_id,
            turn_diff_tracker,
        }
    }
}

pub(crate) enum ToolEventStage {
    Begin,
    Success(ExecToolCallOutput),
    Failure(ToolEventFailure),
}

pub(crate) enum ToolEventFailure {
    Output(ExecToolCallOutput),
    Message(String),
    Rejected {
        output: ExecToolCallOutput,
        user_declined: bool,
    },
}

pub(crate) async fn emit_exec_command_begin(
    ctx: ToolEventCtx<'_>,
    command: &[String],
    cwd: &AbsolutePathBuf,
    parsed_cmd: &[ParsedCommand],
    source: ExecCommandSource,
    interaction_input: Option<String>,
    process_id: Option<&str>,
) {
    ctx.session
        .send_event(
            ctx.turn,
            EventMsg::ExecCommandBegin(ExecCommandBeginEvent {
                call_id: ctx.call_id.to_string(),
                process_id: process_id.map(str::to_owned),
                turn_id: ctx.turn.sub_id.clone(),
                command: command.to_vec(),
                cwd: cwd.clone(),
                parsed_cmd: parsed_cmd.to_vec(),
                source,
                interaction_input,
            }),
        )
        .await;
}
// Concrete, allocation-free emitter: avoid trait objects and boxed futures.
pub(crate) enum ToolEmitter {
    Shell {
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        source: ExecCommandSource,
        parsed_cmd: Vec<ParsedCommand>,
        freeform: bool,
    },
    ApplyPatch {
        changes: HashMap<PathBuf, FileChange>,
        auto_approved: bool,
    },
    UnifiedExec {
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        source: ExecCommandSource,
        parsed_cmd: Vec<ParsedCommand>,
        process_id: Option<String>,
    },
}

impl ToolEmitter {
    pub fn shell(
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        source: ExecCommandSource,
        freeform: bool,
    ) -> Self {
        let parsed_cmd = parse_command(&command);
        Self::Shell {
            command,
            cwd,
            source,
            parsed_cmd,
            freeform,
        }
    }

    pub fn apply_patch(changes: HashMap<PathBuf, FileChange>, auto_approved: bool) -> Self {
        Self::ApplyPatch {
            changes,
            auto_approved,
        }
    }

    pub fn unified_exec(
        command: &[String],
        cwd: AbsolutePathBuf,
        source: ExecCommandSource,
        process_id: Option<String>,
    ) -> Self {
        let parsed_cmd = parse_command(command);
        Self::UnifiedExec {
            command: command.to_vec(),
            cwd,
            source,
            parsed_cmd,
            process_id,
        }
    }

    pub async fn emit(&self, ctx: ToolEventCtx<'_>, stage: ToolEventStage) {
        match (self, stage) {
            (
                Self::Shell {
                    command,
                    cwd,
                    source,
                    parsed_cmd,
                    ..
                },
                stage,
            ) => {
                emit_exec_stage(
                    ctx,
                    ExecCommandInput::new(
                        command, cwd, parsed_cmd, *source, /*interaction_input*/ None,
                        /*process_id*/ None,
                    ),
                    stage,
                )
                .await;
            }

            (
                Self::ApplyPatch {
                    changes,
                    auto_approved,
                },
                ToolEventStage::Begin,
            ) => {
                if let Some(tracker) = ctx.turn_diff_tracker {
                    let mut guard = tracker.lock().await;
                    guard.on_patch_begin(changes);
                }
                ctx.session
                    .send_event(
                        ctx.turn,
                        EventMsg::PatchApplyBegin(PatchApplyBeginEvent {
                            call_id: ctx.call_id.to_string(),
                            turn_id: ctx.turn.sub_id.clone(),
                            auto_approved: *auto_approved,
                            changes: changes.clone(),
                        }),
                    )
                    .await;
            }
            (Self::ApplyPatch { changes, .. }, ToolEventStage::Success(output)) => {
                emit_patch_end(
                    ctx,
                    changes.clone(),
                    output.stdout.text.clone(),
                    output.stderr.text.clone(),
                    output.exit_code == 0,
                    if output.exit_code == 0 {
                        PatchApplyStatus::Completed
                    } else {
                        PatchApplyStatus::Failed
                    },
                )
                .await;
            }
            (
                Self::ApplyPatch { changes, .. },
                ToolEventStage::Failure(ToolEventFailure::Output(output)),
            ) => {
                emit_patch_end(
                    ctx,
                    changes.clone(),
                    output.stdout.text.clone(),
                    output.stderr.text.clone(),
                    output.exit_code == 0,
                    if output.exit_code == 0 {
                        PatchApplyStatus::Completed
                    } else {
                        PatchApplyStatus::Failed
                    },
                )
                .await;
            }
            (
                Self::ApplyPatch { changes, .. },
                ToolEventStage::Failure(ToolEventFailure::Message(message)),
            ) => {
                emit_patch_end(
                    ctx,
                    changes.clone(),
                    String::new(),
                    message.clone(),
                    /*success*/ false,
                    PatchApplyStatus::Failed,
                )
                .await;
            }
            (
                Self::ApplyPatch { changes, .. },
                ToolEventStage::Failure(ToolEventFailure::Rejected {
                    output,
                    user_declined,
                }),
            ) => {
                emit_patch_end(
                    ctx,
                    changes.clone(),
                    output.stdout.text.clone(),
                    output.stderr.text.clone(),
                    /*success*/ false,
                    if user_declined {
                        PatchApplyStatus::Declined
                    } else {
                        PatchApplyStatus::Failed
                    },
                )
                .await;
            }
            (
                Self::UnifiedExec {
                    command,
                    cwd,
                    source,
                    parsed_cmd,
                    process_id,
                },
                stage,
            ) => {
                emit_exec_stage(
                    ctx,
                    ExecCommandInput::new(
                        command,
                        cwd,
                        parsed_cmd,
                        *source,
                        /*interaction_input*/ None,
                        process_id.as_deref(),
                    ),
                    stage,
                )
                .await;
            }
        }
    }

    pub async fn begin(&self, ctx: ToolEventCtx<'_>) {
        self.emit(ctx, ToolEventStage::Begin).await;
    }

    fn format_exec_output_for_model(
        &self,
        output: &ExecToolCallOutput,
        ctx: ToolEventCtx<'_>,
        artifact_ref: Option<&str>,
    ) -> String {
        match self {
            Self::Shell { freeform: true, .. } => {
                super::format_exec_output_for_model_freeform_with_ref(
                    output,
                    ctx.turn.truncation_policy,
                    artifact_ref,
                )
            }
            _ => super::format_exec_output_for_model_structured_with_ref(
                output,
                ctx.turn.truncation_policy,
                artifact_ref,
            ),
        }
    }

    fn format_exec_error_for_model(
        &self,
        outcome: ExecOutcome,
        termination_signal: Option<i32>,
        message: String,
        ctx: ToolEventCtx<'_>,
    ) -> (String, ExecToolCallOutput) {
        let output = ExecToolCallOutput {
            exit_code: -1,
            outcome,
            termination_signal,
            pipeline_stage_exit_codes: None,
            stdout: StreamOutput::new(String::new()),
            stderr: StreamOutput::new(message.clone()),
            aggregated_output: StreamOutput::new(message),
            duration: Duration::ZERO,
        };
        let content = self.format_exec_output_for_model(&output, ctx, None);
        (content, output)
    }

    pub async fn finish(
        &self,
        ctx: ToolEventCtx<'_>,
        out: Result<ExecToolCallOutput, ToolError>,
        artifact_ref: Option<&str>,
    ) -> Result<String, FunctionCallError> {
        let (event, result) = match out {
            Ok(output) => {
                let content = self.format_exec_output_for_model(&output, ctx, artifact_ref);
                let success = output.is_success();
                let event = if success {
                    ToolEventStage::Success(output)
                } else {
                    ToolEventStage::Failure(ToolEventFailure::Output(output))
                };
                let result = if success {
                    Ok(content)
                } else {
                    Err(FunctionCallError::RespondToModel(content))
                };
                (event, result)
            }
            Err(ToolError::Codex(CodexErr::Sandbox(SandboxErr::Timeout { output })))
            | Err(ToolError::Codex(CodexErr::Sandbox(SandboxErr::Denied { output, .. }))) => {
                let response = self.format_exec_output_for_model(&output, ctx, artifact_ref);
                let event = ToolEventStage::Failure(ToolEventFailure::Output(*output));
                let result = Err(FunctionCallError::RespondToModel(response));
                (event, result)
            }
            Err(ToolError::Codex(err)) => {
                let message = format!("execution error: {err:?}");
                if matches!(self, Self::ApplyPatch { .. }) {
                    let event = ToolEventStage::Failure(ToolEventFailure::Message(message.clone()));
                    (event, Err(FunctionCallError::RespondToModel(message)))
                } else {
                    let (outcome, termination_signal) = match &err {
                        CodexErr::Spawn => (ExecOutcome::SpawnFailed, None),
                        CodexErr::Interrupted => (ExecOutcome::Cancelled, None),
                        CodexErr::Sandbox(SandboxErr::Signal(signal)) => {
                            (ExecOutcome::Signaled, Some(*signal))
                        }
                        _ => (ExecOutcome::ExecutionError, None),
                    };
                    let (content, output) =
                        self.format_exec_error_for_model(outcome, termination_signal, message, ctx);
                    let event = ToolEventStage::Failure(ToolEventFailure::Output(output));
                    let result = Err(FunctionCallError::RespondToModel(content));
                    (event, result)
                }
            }
            Err(ToolError::Rejected(message)) => {
                if matches!(self, Self::ApplyPatch { .. }) {
                    let event = ToolEventStage::Failure(ToolEventFailure::Message(message.clone()));
                    (event, Err(FunctionCallError::RespondToModel(message)))
                } else {
                    let (content, output) =
                        self.format_exec_error_for_model(ExecOutcome::Rejected, None, message, ctx);
                    let event = ToolEventStage::Failure(ToolEventFailure::Rejected {
                        output,
                        user_declined: false,
                    });
                    (event, Err(FunctionCallError::RespondToModel(content)))
                }
            }
            Err(ToolError::UserDeclined) => {
                let message = match self {
                    Self::Shell { .. } | Self::UnifiedExec { .. } => {
                        "exec command rejected by user".to_string()
                    }
                    Self::ApplyPatch { .. } => "patch rejected by user".to_string(),
                };
                let (content, output) =
                    self.format_exec_error_for_model(ExecOutcome::Rejected, None, message, ctx);
                let content = match self {
                    Self::Shell { .. } | Self::UnifiedExec { .. } => content,
                    Self::ApplyPatch { .. } => output.stderr.text.clone(),
                };
                let event = ToolEventStage::Failure(ToolEventFailure::Rejected {
                    output,
                    user_declined: true,
                });
                (event, Err(FunctionCallError::RespondToModel(content)))
            }
        };
        self.emit(ctx, event).await;
        result
    }
}

struct ExecCommandInput<'a> {
    command: &'a [String],
    cwd: &'a AbsolutePathBuf,
    parsed_cmd: &'a [ParsedCommand],
    source: ExecCommandSource,
    interaction_input: Option<&'a str>,
    process_id: Option<&'a str>,
}

impl<'a> ExecCommandInput<'a> {
    fn new(
        command: &'a [String],
        cwd: &'a AbsolutePathBuf,
        parsed_cmd: &'a [ParsedCommand],
        source: ExecCommandSource,
        interaction_input: Option<&'a str>,
        process_id: Option<&'a str>,
    ) -> Self {
        Self {
            command,
            cwd,
            parsed_cmd,
            source,
            interaction_input,
            process_id,
        }
    }
}

struct ExecCommandResult {
    stdout: String,
    stderr: String,
    aggregated_output: String,
    shell_exit_code: Option<i32>,
    outcome: ExecOutcome,
    termination_signal: Option<i32>,
    pipeline_stage_exit_codes: Option<Vec<i32>>,
    duration: Duration,
    formatted_output: String,
    status: ExecCommandStatus,
}

async fn emit_exec_stage(
    ctx: ToolEventCtx<'_>,
    exec_input: ExecCommandInput<'_>,
    stage: ToolEventStage,
) {
    match stage {
        ToolEventStage::Begin => {
            emit_exec_command_begin(
                ctx,
                exec_input.command,
                exec_input.cwd,
                exec_input.parsed_cmd,
                exec_input.source,
                exec_input.interaction_input.map(str::to_owned),
                exec_input.process_id,
            )
            .await;
        }
        ToolEventStage::Success(output)
        | ToolEventStage::Failure(ToolEventFailure::Output(output)) => {
            let exec_result = ExecCommandResult {
                stdout: output.stdout.text.clone(),
                stderr: output.stderr.text.clone(),
                aggregated_output: output.aggregated_output.text.clone(),
                shell_exit_code: output.shell_exit_code(),
                outcome: output.outcome,
                termination_signal: output.termination_signal,
                pipeline_stage_exit_codes: output.pipeline_stage_exit_codes.clone(),
                duration: output.duration,
                formatted_output: format_exec_output_str(&output, ctx.turn.truncation_policy),
                status: if output.is_success() {
                    ExecCommandStatus::Completed
                } else {
                    ExecCommandStatus::Failed
                },
            };
            emit_exec_end(ctx, exec_input, exec_result).await;
        }
        ToolEventStage::Failure(ToolEventFailure::Rejected {
            output,
            user_declined,
        }) => {
            let exec_result = ExecCommandResult {
                stdout: output.stdout.text.clone(),
                stderr: output.stderr.text.clone(),
                aggregated_output: output.aggregated_output.text.clone(),
                shell_exit_code: None,
                outcome: output.outcome,
                termination_signal: output.termination_signal,
                pipeline_stage_exit_codes: output.pipeline_stage_exit_codes.clone(),
                duration: output.duration,
                formatted_output: format_exec_output_str(&output, ctx.turn.truncation_policy),
                status: if user_declined {
                    ExecCommandStatus::Declined
                } else {
                    ExecCommandStatus::Failed
                },
            };
            emit_exec_end(ctx, exec_input, exec_result).await;
        }
        ToolEventStage::Failure(ToolEventFailure::Message(message)) => {
            let output = ExecToolCallOutput {
                exit_code: -1,
                outcome: ExecOutcome::ExecutionError,
                termination_signal: None,
                pipeline_stage_exit_codes: None,
                stdout: StreamOutput::new(String::new()),
                stderr: StreamOutput::new(message.clone()),
                aggregated_output: StreamOutput::new(message),
                duration: Duration::ZERO,
            };
            let exec_result = ExecCommandResult {
                stdout: output.stdout.text.clone(),
                stderr: output.stderr.text.clone(),
                aggregated_output: output.aggregated_output.text.clone(),
                shell_exit_code: None,
                outcome: output.outcome,
                termination_signal: None,
                pipeline_stage_exit_codes: None,
                duration: output.duration,
                formatted_output: format_exec_output_str(&output, ctx.turn.truncation_policy),
                status: ExecCommandStatus::Failed,
            };
            emit_exec_end(ctx, exec_input, exec_result).await;
        }
    }
}

async fn emit_exec_end(
    ctx: ToolEventCtx<'_>,
    exec_input: ExecCommandInput<'_>,
    exec_result: ExecCommandResult,
) {
    if matches!(exec_result.outcome, ExecOutcome::Signaled)
        != exec_result.termination_signal.is_some()
    {
        tracing::warn!(
            target: "codex_core::tool_feedback",
            event_name = "tool.exec_outcome_incomplete",
            call_id = ctx.call_id,
            process_id = exec_input.process_id,
            execution_outcome = exec_result.outcome.as_str(),
            termination_signal = ?exec_result.termination_signal,
        );
    }
    tracing::info!(
        target: "codex_core::tool_feedback",
        event_name = "tool.exec_outcome_recorded",
        call_id = ctx.call_id,
        process_id = exec_input.process_id,
        execution_outcome = exec_result.outcome.as_str(),
        shell_exit_code = ?exec_result.shell_exit_code,
        termination_signal = ?exec_result.termination_signal,
        pipeline_stage_exit_codes = ?exec_result.pipeline_stage_exit_codes,
    );

    ctx.session
        .send_event(
            ctx.turn,
            EventMsg::ExecCommandEnd(ExecCommandEndEvent {
                call_id: ctx.call_id.to_string(),
                process_id: exec_input.process_id.map(str::to_owned),
                turn_id: ctx.turn.sub_id.clone(),
                command: exec_input.command.to_vec(),
                cwd: exec_input.cwd.clone(),
                parsed_cmd: exec_input.parsed_cmd.to_vec(),
                source: exec_input.source,
                interaction_input: exec_input.interaction_input.map(str::to_owned),
                stdout: exec_result.stdout,
                stderr: exec_result.stderr,
                aggregated_output: exec_result.aggregated_output,
                shell_exit_code: exec_result.shell_exit_code,
                outcome: exec_result.outcome,
                termination_signal: exec_result.termination_signal,
                pipeline_stage_exit_codes: exec_result.pipeline_stage_exit_codes,
                duration: exec_result.duration,
                formatted_output: exec_result.formatted_output,
                status: exec_result.status,
            }),
        )
        .await;
}

async fn emit_patch_end(
    ctx: ToolEventCtx<'_>,
    changes: HashMap<PathBuf, FileChange>,
    stdout: String,
    stderr: String,
    success: bool,
    status: PatchApplyStatus,
) {
    ctx.session
        .send_event(
            ctx.turn,
            EventMsg::PatchApplyEnd(PatchApplyEndEvent {
                call_id: ctx.call_id.to_string(),
                turn_id: ctx.turn.sub_id.clone(),
                stdout,
                stderr,
                success,
                changes,
                status,
            }),
        )
        .await;

    if let Some(tracker) = ctx.turn_diff_tracker {
        let unified_diff = {
            let mut guard = tracker.lock().await;
            guard.get_unified_diff()
        };
        if let Ok(Some(unified_diff)) = unified_diff {
            ctx.session
                .send_event(ctx.turn, EventMsg::TurnDiff(TurnDiffEvent { unified_diff }))
                .await;
        }
    }
}
