use serde_json::{json, Value};
use whalecode_model::CollectedToolCall;
use whalecode_patch::{
    PatchOperation, PatchRequest, WorkspacePatchEngine, WorkspacePatchStatus, WriteFileRequest,
};
use whalecode_permission::{
    ApprovalPolicy, PermissionContext, PermissionDecision, PermissionEngine, PermissionOperation,
    PermissionRequest,
};
use whalecode_protocol::{
    AgentId, AgentRole, ArtifactId, PatchApplyStatus, PatchEvent, PermissionEvent, SessionEvent,
    ToolCallId, ToolEvent, ToolStatus, WorkflowPhase,
};
use whalecode_tools::{ToolRequest, ToolResultEnvelope, ToolRuntime};

use crate::command_tool::{run_command, RunCommandArgs};
use crate::live_tool_args::{
    argument_path, argument_query, parse_edit_file, parse_list_files, parse_read_file,
    parse_search_text, parse_write_file, tool_input_summary,
};
use crate::tool_log::tool_log_preview;
use crate::{permission_event_kind, recorder::EventRecorder, AgentError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolExecutionResult {
    pub(crate) message: String,
    pub(crate) summary: String,
}

pub(crate) async fn execute_model_tool(
    tools: &ToolRuntime,
    patch_engine: &WorkspacePatchEngine,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder<'_>,
    call: &CollectedToolCall,
    allow_write: bool,
    allow_command: bool,
) -> Result<ToolExecutionResult, AgentError> {
    let call_id = ToolCallId::from(call.id.clone());
    recorder.append(SessionEvent::Tool(ToolEvent::CallStarted {
        call_id: call_id.clone(),
        tool_name: call.name.clone(),
        input_summary: tool_input_summary(&call.name, &call.arguments),
    }))?;

    let result = match call.name.as_str() {
        "list_files" => execute_read_tool(
            tools,
            permission,
            recorder,
            &call.name,
            PermissionOperation::SearchWorkspace {
                query: "*".to_owned(),
            },
            &call.arguments,
            parse_list_files,
        ),
        "read_file" => execute_read_tool(
            tools,
            permission,
            recorder,
            &call.name,
            PermissionOperation::ReadFile {
                path: argument_path(&call.arguments),
            },
            &call.arguments,
            parse_read_file,
        ),
        "search_text" => execute_read_tool(
            tools,
            permission,
            recorder,
            &call.name,
            PermissionOperation::SearchWorkspace {
                query: argument_query(&call.arguments),
            },
            &call.arguments,
            parse_search_text,
        ),
        "edit_file" => execute_edit_tool(
            patch_engine,
            permission,
            recorder,
            &call.name,
            &call.arguments,
            allow_write,
        ),
        "write_file" => execute_write_file_tool(
            patch_engine,
            permission,
            recorder,
            &call.name,
            &call.arguments,
            allow_write,
        ),
        "run_command" => {
            execute_command_tool(
                tools,
                permission,
                recorder,
                &call.name,
                &call.arguments,
                allow_command,
            )
            .await
        }
        _ => Ok(tool_error(
            format!("unknown tool: {}", call.name),
            ToolStatus::Failed,
        )),
    }?;

    let output_artifact = ArtifactId::from(format!("tool-output-{}", recorder.next_sequence()));
    let (content_preview, truncated) = tool_log_preview(&result.message);
    recorder.append(SessionEvent::Tool(ToolEvent::OutputRecorded {
        call_id: call_id.clone(),
        artifact_id: output_artifact.clone(),
        summary: result.summary.clone(),
        content_preview,
        truncated,
    }))?;
    recorder.append(SessionEvent::Tool(ToolEvent::CallFinished {
        call_id,
        status: result_status(&result.message),
        output_artifact: Some(output_artifact),
    }))?;
    Ok(result)
}

async fn execute_command_tool(
    tools: &ToolRuntime,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder<'_>,
    tool_name: &str,
    arguments: &str,
    allow_command: bool,
) -> Result<ToolExecutionResult, AgentError> {
    let args = match serde_json::from_str::<RunCommandArgs>(arguments) {
        Ok(args) => args,
        Err(error) => {
            return Ok(tool_error(
                format!("invalid run_command arguments: {error}"),
                ToolStatus::Failed,
            ));
        }
    };
    let mut command_vector = vec![args.command.clone()];
    command_vector.extend(args.args.clone().unwrap_or_default());
    let decision = decide(
        permission,
        recorder,
        tool_name,
        PermissionOperation::Shell {
            command: command_vector,
        },
        WorkflowPhase::Verify,
        allow_command,
    )?;
    if !matches!(decision, PermissionDecision::Allow) {
        return Ok(tool_error(
            "run_command requires whale run --allow-command".to_owned(),
            ToolStatus::Rejected,
        ));
    }

    let output = run_command(tools.workspace_root(), args).await;
    Ok(ToolExecutionResult {
        message: output.message,
        summary: output.summary,
    })
}

fn execute_read_tool<F>(
    tools: &ToolRuntime,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder<'_>,
    tool_name: &str,
    operation: PermissionOperation,
    arguments: &str,
    build_request: F,
) -> Result<ToolExecutionResult, AgentError>
where
    F: FnOnce(&str) -> Result<ToolRequest, String>,
{
    let decision = decide(
        permission,
        recorder,
        tool_name,
        operation,
        WorkflowPhase::Analyze,
        false,
    )?;
    if !matches!(decision, PermissionDecision::Allow) {
        return Ok(tool_error(
            "read tool denied by permission policy".to_owned(),
            ToolStatus::Rejected,
        ));
    }
    let request = match build_request(arguments) {
        Ok(request) => request,
        Err(message) => return Ok(tool_error(message, ToolStatus::Failed)),
    };
    match tools.execute(request) {
        Ok(output) => Ok(tool_success(
            tool_output_json(&output),
            preview(&output.content),
        )),
        Err(error) => Ok(tool_error(error.to_string(), ToolStatus::Failed)),
    }
}

fn execute_edit_tool(
    patch_engine: &WorkspacePatchEngine,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder<'_>,
    tool_name: &str,
    arguments: &str,
    allow_write: bool,
) -> Result<ToolExecutionResult, AgentError> {
    let args = match parse_edit_file(arguments) {
        Ok(args) => args,
        Err(error) => return Ok(tool_error(error, ToolStatus::Failed)),
    };
    let decision = decide(
        permission,
        recorder,
        tool_name,
        PermissionOperation::WriteFile {
            path: args.path.clone(),
        },
        WorkflowPhase::Implement,
        allow_write,
    )?;
    if !matches!(decision, PermissionDecision::Allow) {
        return Ok(tool_error(
            "edit_file requires whale run --allow-write".to_owned(),
            ToolStatus::Rejected,
        ));
    }

    let snapshot = match patch_engine.snapshot(&args.path) {
        Ok(snapshot) => snapshot,
        Err(error) => return Ok(tool_error(error.to_string(), ToolStatus::Failed)),
    };
    let request = PatchRequest {
        path: args.path,
        expected_snapshot: snapshot,
        operation: PatchOperation::ReplaceOne {
            old: args.old_string,
            new: args.new_string,
        },
    };
    let preview = match patch_engine.apply(&request) {
        Ok(preview) => preview,
        Err(error) => return Ok(tool_error(error.to_string(), ToolStatus::Failed)),
    };
    let applied = matches!(preview.status, WorkspacePatchStatus::Applied);
    record_patch_result(recorder, &preview)?;
    Ok(patch_tool_result(preview, applied))
}

fn execute_write_file_tool(
    patch_engine: &WorkspacePatchEngine,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder<'_>,
    tool_name: &str,
    arguments: &str,
    allow_write: bool,
) -> Result<ToolExecutionResult, AgentError> {
    let args = match parse_write_file(arguments) {
        Ok(args) => args,
        Err(error) => return Ok(tool_error(error, ToolStatus::Failed)),
    };
    let decision = decide(
        permission,
        recorder,
        tool_name,
        PermissionOperation::WriteFile {
            path: args.path.clone(),
        },
        WorkflowPhase::Implement,
        allow_write,
    )?;
    if !matches!(decision, PermissionDecision::Allow) {
        return Ok(tool_error(
            "write_file requires whale run --allow-write".to_owned(),
            ToolStatus::Rejected,
        ));
    }

    let preview = match patch_engine.write_file(&WriteFileRequest {
        path: args.path,
        content: args.content,
        create_parent_dirs: args.create_parent_dirs.unwrap_or(true),
    }) {
        Ok(preview) => preview,
        Err(error) => return Ok(tool_error(error.to_string(), ToolStatus::Failed)),
    };
    record_patch_result(recorder, &preview)?;

    let applied = matches!(preview.status, WorkspacePatchStatus::Applied);
    Ok(patch_tool_result(preview, applied))
}

fn decide(
    permission: &PermissionEngine,
    recorder: &mut EventRecorder<'_>,
    tool_name: &str,
    operation: PermissionOperation,
    phase: WorkflowPhase,
    allow_write: bool,
) -> Result<PermissionDecision, AgentError> {
    let decision = permission.decide(
        &PermissionRequest {
            subject: tool_name.to_owned(),
            operation,
        },
        &PermissionContext {
            agent_id: AgentId::from("supervisor"),
            role: AgentRole::Supervisor,
            phase,
            work_unit_id: None,
            approval_policy: if allow_write {
                ApprovalPolicy::PreApproved
            } else {
                ApprovalPolicy::Never
            },
        },
    );
    recorder.append(SessionEvent::Permission(PermissionEvent::Decision {
        subject: tool_name.to_owned(),
        decision: permission_event_kind(&decision),
    }))?;
    Ok(decision)
}

fn record_patch_result(
    recorder: &mut EventRecorder<'_>,
    preview: &whalecode_patch::PatchPreview,
) -> Result<(), AgentError> {
    let artifact_id = ArtifactId::from(format!("patch-{}", recorder.next_sequence()));
    recorder.append(SessionEvent::Patch(PatchEvent::ArtifactCreated {
        artifact_id: artifact_id.clone(),
        touched_files: preview.touched_files.clone(),
    }))?;
    let apply_status = match preview.status {
        WorkspacePatchStatus::Applied => PatchApplyStatus::Applied,
        WorkspacePatchStatus::Rejected { .. } => PatchApplyStatus::Rejected,
    };
    recorder.append(SessionEvent::Patch(PatchEvent::ApplyResult {
        artifact_id,
        status: apply_status,
    }))?;
    Ok(())
}

fn patch_tool_result(preview: whalecode_patch::PatchPreview, applied: bool) -> ToolExecutionResult {
    tool_success(
        serde_json::to_string(&json!({
            "ok": applied,
            "tool_status": if applied { "succeeded" } else { "rejected" },
            "status": preview.status,
            "touched_files": preview.touched_files,
            "diff": preview.diff,
        }))
        .expect("patch result is serializable"),
        if applied {
            "patch applied".to_owned()
        } else {
            "patch rejected".to_owned()
        },
    )
}

fn tool_output_json(output: &ToolResultEnvelope) -> String {
    serde_json::to_string(&json!({
        "ok": true,
        "content": output.content,
        "truncated": output.truncated,
        "original_len": output.original_len,
        "metadata": output.metadata,
    }))
    .expect("tool result is serializable")
}

fn tool_success(message: String, summary: String) -> ToolExecutionResult {
    ToolExecutionResult { message, summary }
}

fn tool_error(message: String, status: ToolStatus) -> ToolExecutionResult {
    ToolExecutionResult {
        message: serde_json::to_string(&json!({
            "ok": false,
            "status": status,
            "error": message,
        }))
        .expect("tool error is serializable"),
        summary: "tool failed".to_owned(),
    }
}

fn result_status(message: &str) -> ToolStatus {
    let Ok(value) = serde_json::from_str::<Value>(message) else {
        return ToolStatus::Failed;
    };
    if value.get("ok").and_then(Value::as_bool) == Some(true) {
        ToolStatus::Succeeded
    } else if value.get("tool_status").and_then(Value::as_str) == Some("rejected")
        || value.get("status").and_then(Value::as_str) == Some("rejected")
    {
        ToolStatus::Rejected
    } else {
        ToolStatus::Failed
    }
}

fn preview(content: &str) -> String {
    let mut lines = content.lines().take(3).collect::<Vec<_>>().join(" | ");
    if lines.len() > 160 {
        lines.truncate(160);
        lines.push_str("...");
    }
    if lines.is_empty() {
        "(empty result)".to_owned()
    } else {
        lines
    }
}
