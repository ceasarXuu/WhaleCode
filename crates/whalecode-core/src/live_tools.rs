use serde::Deserialize;
use serde_json::{json, Value};
use whalecode_model::CollectedToolCall;
use whalecode_patch::{PatchOperation, PatchRequest, WorkspacePatchEngine, WorkspacePatchStatus};
use whalecode_permission::{
    ApprovalPolicy, PermissionContext, PermissionDecision, PermissionEngine, PermissionOperation,
    PermissionRequest,
};
use whalecode_protocol::{
    AgentId, AgentRole, ArtifactId, PatchApplyStatus, PatchEvent, PermissionEvent, SessionEvent,
    ToolCallId, ToolEvent, ToolStatus, WorkflowPhase,
};
use whalecode_tools::{ToolRequest, ToolResultEnvelope, ToolRuntime};

use crate::{permission_event_kind, recorder::EventRecorder, AgentError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolExecutionResult {
    pub(crate) message: String,
    pub(crate) summary: String,
}

#[derive(Debug, Deserialize)]
struct ListFilesArgs {
    max_entries: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ReadFileArgs {
    path: String,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchTextArgs {
    query: String,
    max_matches: Option<usize>,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct EditFileArgs {
    path: String,
    #[serde(alias = "old")]
    old_string: String,
    #[serde(alias = "new")]
    new_string: String,
}

pub(crate) fn execute_model_tool(
    tools: &ToolRuntime,
    patch_engine: &WorkspacePatchEngine,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder,
    call: &CollectedToolCall,
    allow_write: bool,
) -> Result<ToolExecutionResult, AgentError> {
    let call_id = ToolCallId::from(call.id.clone());
    recorder.append(SessionEvent::Tool(ToolEvent::CallStarted {
        call_id: call_id.clone(),
        tool_name: call.name.clone(),
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
        _ => Ok(tool_error(
            format!("unknown tool: {}", call.name),
            ToolStatus::Failed,
        )),
    }?;

    recorder.append(SessionEvent::Tool(ToolEvent::CallFinished {
        call_id,
        status: result_status(&result.message),
        output_artifact: None,
    }))?;
    Ok(result)
}

fn execute_read_tool<F>(
    tools: &ToolRuntime,
    permission: &PermissionEngine,
    recorder: &mut EventRecorder,
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
    recorder: &mut EventRecorder,
    tool_name: &str,
    arguments: &str,
    allow_write: bool,
) -> Result<ToolExecutionResult, AgentError> {
    let args = match serde_json::from_str::<EditFileArgs>(arguments) {
        Ok(args) => args,
        Err(error) => {
            return Ok(tool_error(
                format!("invalid edit_file arguments: {error}"),
                ToolStatus::Failed,
            ));
        }
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
            "edit_file requires whale run --live --allow-write".to_owned(),
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

    let applied = matches!(preview.status, WorkspacePatchStatus::Applied);
    Ok(tool_success(
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
    ))
}

fn decide(
    permission: &PermissionEngine,
    recorder: &mut EventRecorder,
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

fn parse_list_files(arguments: &str) -> Result<ToolRequest, String> {
    let args = parse_args::<ListFilesArgs>(arguments)?;
    Ok(ToolRequest::ListFiles {
        max_entries: args.max_entries.unwrap_or(120).clamp(1, 500),
    })
}

fn parse_read_file(arguments: &str) -> Result<ToolRequest, String> {
    let args = parse_args::<ReadFileArgs>(arguments)?;
    Ok(ToolRequest::ReadFile {
        path: args.path,
        max_bytes: Some(args.max_bytes.unwrap_or(32 * 1024).clamp(1024, 128 * 1024)),
    })
}

fn parse_search_text(arguments: &str) -> Result<ToolRequest, String> {
    let args = parse_args::<SearchTextArgs>(arguments)?;
    Ok(ToolRequest::SearchText {
        query: args.query,
        max_matches: args.max_matches.unwrap_or(50).clamp(1, 200),
        max_bytes: Some(args.max_bytes.unwrap_or(32 * 1024).clamp(1024, 128 * 1024)),
    })
}

fn parse_args<T: for<'de> Deserialize<'de>>(arguments: &str) -> Result<T, String> {
    serde_json::from_str(arguments).map_err(|error| format!("invalid tool arguments: {error}"))
}

fn argument_path(arguments: &str) -> String {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| value.get("path").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

fn argument_query(arguments: &str) -> String {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| {
            value
                .get("query")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

pub(crate) fn live_tool_defs() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List gitignore-aware workspace files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "max_entries": {"type": "integer", "minimum": 1, "maximum": 500}
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read a UTF-8 workspace file by relative path.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "max_bytes": {"type": "integer", "minimum": 1024, "maximum": 131072}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_text",
                "description": "Search for literal text in gitignore-aware workspace files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "max_matches": {"type": "integer", "minimum": 1, "maximum": 200},
                        "max_bytes": {"type": "integer", "minimum": 1024, "maximum": 131072}
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Patch one UTF-8 workspace file by replacing one exact old string with a new string. Requires --allow-write.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "old_string": {"type": "string"},
                        "new_string": {"type": "string"}
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        }),
    ]
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
