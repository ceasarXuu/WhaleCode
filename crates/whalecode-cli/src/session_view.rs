use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use thiserror::Error;
use whalecode_core::default_sessions_dir;
use whalecode_protocol::{
    ModelEvent, ModelStreamDelta, PatchApplyStatus, PatchEvent, PermissionDecisionKind,
    PermissionEvent, PrimitiveEvent, ReplayEvent, SessionEvent, SessionFinishStatus,
    SessionLifecycleEvent, ToolEvent, ToolStatus, TranscriptEvent, TurnEvent, TurnFinishStatus,
};
use whalecode_session::{read_jsonl, SessionError};

#[derive(Debug, Error)]
pub(crate) enum SessionViewError {
    #[error("no session logs found in {dir}")]
    NoSessions { dir: PathBuf },
    #[error("failed to list session directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("session error: {0}")]
    Session(#[from] SessionError),
    #[error("failed to write output: {0}")]
    Write(#[from] std::io::Error),
}

pub(crate) fn print_session_log(session: Option<PathBuf>) -> Result<(), SessionViewError> {
    let path = match session {
        Some(path) => path,
        None => latest_session_path()?,
    };
    let events = read_jsonl(&path)?;
    let mut stdout = io::stdout();
    writeln!(stdout, "session: {}", path.display())?;
    writeln!(stdout, "events: {}", events.len())?;
    for event in events {
        let turn = event
            .turn_id
            .as_ref()
            .map(|turn| format!(" turn={}", turn.0))
            .unwrap_or_default();
        writeln!(
            stdout,
            "{:>4} {}{} {}",
            event.sequence,
            event.occurred_at.to_rfc3339(),
            turn,
            describe_event(&event.payload)
        )?;
    }
    Ok(())
}

fn latest_session_path() -> Result<PathBuf, SessionViewError> {
    let dir = default_sessions_dir();
    let entries = fs::read_dir(&dir).map_err(|source| SessionViewError::ReadDir {
        path: dir.clone(),
        source,
    })?;
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.pop().ok_or(SessionViewError::NoSessions { dir })
}

fn describe_event(event: &SessionEvent) -> String {
    match event {
        SessionEvent::Session(event) => describe_session(event),
        SessionEvent::Transcript(event) => describe_transcript(event),
        SessionEvent::Model(event) => describe_model(event),
        SessionEvent::Tool(event) => describe_tool(event),
        SessionEvent::Permission(event) => describe_permission(event),
        SessionEvent::Patch(event) => describe_patch(event),
        SessionEvent::Primitive(event) => describe_primitive(event),
        SessionEvent::Replay(event) => describe_replay(event),
        SessionEvent::Phase(event) => format!("phase {:?}", event),
        SessionEvent::Turn(event) => describe_turn(event),
    }
}

fn describe_session(event: &SessionLifecycleEvent) -> String {
    match event {
        SessionLifecycleEvent::Started { cwd } => format!("session started cwd={cwd}"),
        SessionLifecycleEvent::Finished { status } => {
            format!("session finished status={}", session_status(status))
        }
    }
}

fn describe_transcript(event: &TranscriptEvent) -> String {
    match event {
        TranscriptEvent::UserMessage { content } => format!("user {}", preview(content)),
        TranscriptEvent::AssistantMessage { content } => format!("assistant {}", preview(content)),
    }
}

fn describe_model(event: &ModelEvent) -> String {
    match event {
        ModelEvent::RequestStarted { model } => format!("model request started model={model}"),
        ModelEvent::StreamDelta { delta } => match delta {
            ModelStreamDelta::Text { content } => format!("model text {}", preview(content)),
            ModelStreamDelta::Reasoning { content } => {
                format!("model reasoning {}", preview(content))
            }
            ModelStreamDelta::ToolCall {
                name,
                arguments_delta,
            } => format!(
                "model tool_call name={name} args_delta={}",
                preview(arguments_delta)
            ),
        },
        ModelEvent::RequestFinished { usage } => match usage {
            Some(usage) => format!(
                "model request finished input_tokens={} output_tokens={} cached_input_tokens={}",
                usage.input_tokens, usage.output_tokens, usage.cached_input_tokens
            ),
            None => "model request finished usage=false".to_owned(),
        },
        ModelEvent::RequestFailed { message } => {
            format!("model request failed {}", preview(message))
        }
    }
}

fn describe_tool(event: &ToolEvent) -> String {
    match event {
        ToolEvent::CallStarted { call_id, tool_name } => {
            format!("tool started id={} name={tool_name}", call_id.0)
        }
        ToolEvent::CallFinished {
            call_id,
            status,
            output_artifact,
        } => format!(
            "tool finished id={} status={} artifact={}",
            call_id.0,
            tool_status(status),
            output_artifact
                .as_ref()
                .map(|artifact| artifact.0.as_str())
                .unwrap_or("none")
        ),
        ToolEvent::OutputRecorded {
            call_id,
            artifact_id,
            summary,
            content_preview,
            truncated,
        } => format!(
            "tool output id={} artifact={} summary={} truncated={} preview={}",
            call_id.0,
            artifact_id.0,
            preview(summary),
            truncated,
            preview(content_preview)
        ),
    }
}

fn describe_permission(event: &PermissionEvent) -> String {
    match event {
        PermissionEvent::Decision { subject, decision } => {
            format!(
                "permission subject={subject} decision={}",
                permission(decision)
            )
        }
    }
}

fn describe_patch(event: &PatchEvent) -> String {
    match event {
        PatchEvent::ArtifactCreated {
            artifact_id,
            touched_files,
        } => format!(
            "patch artifact={} files={}",
            artifact_id.0,
            touched_files.join(",")
        ),
        PatchEvent::ApplyResult {
            artifact_id,
            status,
        } => format!(
            "patch applied artifact={} status={}",
            artifact_id.0,
            patch_status(status)
        ),
    }
}

fn describe_primitive(event: &PrimitiveEvent) -> String {
    format!("primitive {:?}", event)
}

fn describe_replay(event: &ReplayEvent) -> String {
    format!("replay {:?}", event)
}

fn describe_turn(event: &TurnEvent) -> String {
    match event {
        TurnEvent::Started { index } => format!("turn started index={index}"),
        TurnEvent::Finished { index, status } => {
            format!("turn finished index={index} status={}", turn_status(status))
        }
    }
}

fn session_status(status: &SessionFinishStatus) -> &'static str {
    match status {
        SessionFinishStatus::Succeeded => "succeeded",
        SessionFinishStatus::Failed => "failed",
        SessionFinishStatus::Cancelled => "cancelled",
    }
}

fn tool_status(status: &ToolStatus) -> &'static str {
    match status {
        ToolStatus::Succeeded => "succeeded",
        ToolStatus::Failed => "failed",
        ToolStatus::Rejected => "rejected",
    }
}

fn permission(decision: &PermissionDecisionKind) -> &'static str {
    match decision {
        PermissionDecisionKind::Allowed => "allowed",
        PermissionDecisionKind::Asked => "asked",
        PermissionDecisionKind::Denied => "denied",
    }
}

fn patch_status(status: &PatchApplyStatus) -> &'static str {
    match status {
        PatchApplyStatus::Applied => "applied",
        PatchApplyStatus::Conflict => "conflict",
        PatchApplyStatus::Rejected => "rejected",
    }
}

fn turn_status(status: &TurnFinishStatus) -> &'static str {
    match status {
        TurnFinishStatus::Completed => "completed",
        TurnFinishStatus::Continued => "continued",
        TurnFinishStatus::Failed => "failed",
    }
}

fn preview(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= 180 {
        return compact;
    }
    let mut boundary = 180;
    while !compact.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...", &compact[..boundary])
}
