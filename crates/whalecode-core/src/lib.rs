use std::{
    env,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use whalecode_model::{BootstrapModelRuntime, ModelRequest, ModelStreamEvent};
use whalecode_permission::{
    ApprovalPolicy, PermissionContext, PermissionDecision, PermissionEngine, PermissionOperation,
    PermissionRequest,
};
use whalecode_protocol::{
    AgentId, AgentRole, ArtifactId, ModelEvent, ModelStreamDelta, PermissionDecisionKind,
    PermissionEvent, PhaseEvent, SessionEvent, SessionFinishStatus, SessionLifecycleEvent,
    ToolCallId, ToolEvent, ToolStatus, TranscriptEvent, TurnEvent, TurnFinishStatus, TurnId,
    WorkflowPhase,
};
use whalecode_session::SessionError;
use whalecode_tools::{ToolError, ToolRequest, ToolResultEnvelope, ToolRuntime};

mod command_tool;
mod interaction_router;
mod live;
mod live_tool_defs;
mod live_tools;
mod recorder;
mod tool_log;

pub use interaction_router::*;
pub use live::*;

use recorder::{ensure_parent_dir, EventRecorder};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Busy,
    Blocked,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRuntime {
    pub id: AgentId,
    pub role: AgentRole,
    pub state: AgentState,
}

impl AgentRuntime {
    pub fn new(id: AgentId, role: AgentRole) -> Self {
        Self {
            id,
            role,
            state: AgentState::Idle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunOptions {
    pub task: String,
    pub cwd: PathBuf,
    pub session_path: PathBuf,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunSummary {
    pub final_message: String,
    pub session_path: PathBuf,
    pub events_written: u64,
    pub tool_summaries: Vec<String>,
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("task cannot be empty")]
    EmptyTask,
    #[error("failed to create session directory {path}: {source}")]
    CreateSessionDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to resolve current directory: {0}")]
    CurrentDir(std::io::Error),
    #[error("session error: {0}")]
    Session(#[from] SessionError),
    #[error("permission rejected tool execution: {tool}")]
    PermissionRejected { tool: String },
    #[error("tool error: {0}")]
    Tool(#[from] ToolError),
    #[error("model error: {0}")]
    Model(#[from] whalecode_model::ModelError),
    #[error("patch error: {0}")]
    Patch(#[from] whalecode_patch::PatchError),
    #[error("live agent exceeded max turns ({max_turns})")]
    MaxTurns { max_turns: usize },
}

#[derive(Debug, Clone)]
pub struct AgentLoop {
    agent: AgentRuntime,
    model: BootstrapModelRuntime,
    permission: PermissionEngine,
}

impl AgentLoop {
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        Self {
            agent: AgentRuntime::new(AgentId::from("supervisor"), AgentRole::Supervisor),
            model: BootstrapModelRuntime::new(model),
            permission: PermissionEngine,
        }
    }

    pub fn run(&self, options: AgentRunOptions) -> Result<AgentRunSummary, AgentError> {
        if options.task.trim().is_empty() {
            return Err(AgentError::EmptyTask);
        }
        ensure_parent_dir(&options.session_path)?;

        let tools = ToolRuntime::new(&options.cwd)?;
        let mut recorder = EventRecorder::open(&options.session_path)?;
        recorder.append(SessionEvent::Session(SessionLifecycleEvent::Started {
            cwd: options.cwd.display().to_string(),
        }))?;
        recorder.append(SessionEvent::Transcript(TranscriptEvent::UserMessage {
            content: options.task.clone(),
        }))?;
        recorder.append(SessionEvent::Phase(PhaseEvent::Transition {
            from: WorkflowPhase::Idle,
            to: WorkflowPhase::Analyze,
        }))?;
        recorder.set_turn(TurnId::from("turn-1"));
        recorder.append(SessionEvent::Turn(TurnEvent::Started { index: 1 }))?;
        recorder.append(SessionEvent::Model(ModelEvent::RequestStarted {
            model: options.model.clone(),
        }))?;

        let mut tool_summaries = Vec::new();
        let list_result = self.run_read_tool(
            &tools,
            &mut recorder,
            "list_files",
            PermissionOperation::SearchWorkspace {
                query: "*".to_owned(),
            },
            ToolRequest::ListFiles { max_entries: 120 },
        )?;
        tool_summaries.push(format!(
            "Listed workspace files: {}",
            preview_lines(&list_result, 8)
        ));

        for candidate in ["AGENTS.md", "README.md"] {
            if options.cwd.join(candidate).is_file() {
                let read_result = self.run_read_tool(
                    &tools,
                    &mut recorder,
                    "read_file",
                    PermissionOperation::ReadFile {
                        path: candidate.to_owned(),
                    },
                    ToolRequest::ReadFile {
                        path: candidate.to_owned(),
                        max_bytes: Some(4096),
                    },
                )?;
                tool_summaries.push(format!(
                    "Read {candidate}: {} bytes{}",
                    read_result.original_len,
                    if read_result.truncated {
                        " before truncation"
                    } else {
                        ""
                    }
                ));
            }
        }

        let response = self.model.complete(ModelRequest {
            model: options.model.clone(),
            task: options.task.clone(),
            tool_summaries: tool_summaries.clone(),
        });
        for event in response.events {
            match event {
                ModelStreamEvent::TextDelta(content) => {
                    recorder.append(SessionEvent::Model(ModelEvent::StreamDelta {
                        delta: ModelStreamDelta::Text { content },
                    }))?;
                }
                ModelStreamEvent::ReasoningDelta(content) => {
                    recorder.append(SessionEvent::Model(ModelEvent::StreamDelta {
                        delta: ModelStreamDelta::Reasoning { content },
                    }))?;
                }
                ModelStreamEvent::ToolCallDelta {
                    index: _,
                    id: _,
                    name,
                    arguments_delta,
                } => recorder.append(SessionEvent::Model(ModelEvent::StreamDelta {
                    delta: ModelStreamDelta::ToolCall {
                        name,
                        arguments_delta,
                    },
                }))?,
                ModelStreamEvent::Finished => {
                    recorder.append(SessionEvent::Model(ModelEvent::RequestFinished {
                        usage: None,
                    }))?;
                }
            }
        }

        recorder.append(SessionEvent::Transcript(
            TranscriptEvent::AssistantMessage {
                content: response.final_text.clone(),
            },
        ))?;
        recorder.append(SessionEvent::Phase(PhaseEvent::Transition {
            from: WorkflowPhase::Analyze,
            to: WorkflowPhase::Done,
        }))?;
        recorder.append(SessionEvent::Turn(TurnEvent::Finished {
            index: 1,
            status: TurnFinishStatus::Completed,
        }))?;
        recorder.clear_turn();
        recorder.append(SessionEvent::Session(SessionLifecycleEvent::Finished {
            status: SessionFinishStatus::Succeeded,
        }))?;

        Ok(AgentRunSummary {
            final_message: response.final_text,
            session_path: options.session_path,
            events_written: recorder.events_written(),
            tool_summaries,
        })
    }

    fn run_read_tool(
        &self,
        tools: &ToolRuntime,
        recorder: &mut EventRecorder,
        tool_name: &str,
        operation: PermissionOperation,
        request: ToolRequest,
    ) -> Result<ToolResultEnvelope, AgentError> {
        let permission = self.permission.decide(
            &PermissionRequest {
                subject: tool_name.to_owned(),
                operation,
            },
            &PermissionContext {
                agent_id: self.agent.id.clone(),
                role: self.agent.role.clone(),
                phase: WorkflowPhase::Analyze,
                work_unit_id: None,
                approval_policy: ApprovalPolicy::Never,
            },
        );
        recorder.append(SessionEvent::Permission(PermissionEvent::Decision {
            subject: tool_name.to_owned(),
            decision: permission_event_kind(&permission),
        }))?;

        let call_id = ToolCallId::from(format!("tool-{}", recorder.next_sequence()));
        recorder.append(SessionEvent::Tool(ToolEvent::CallStarted {
            call_id: call_id.clone(),
            tool_name: tool_name.to_owned(),
        }))?;

        let result = match permission {
            PermissionDecision::Allow => tools.execute(request)?,
            PermissionDecision::Ask { .. } | PermissionDecision::Deny { .. } => {
                recorder.append(SessionEvent::Tool(ToolEvent::CallFinished {
                    call_id,
                    status: ToolStatus::Rejected,
                    output_artifact: None,
                }))?;
                return Err(AgentError::PermissionRejected {
                    tool: tool_name.to_owned(),
                });
            }
        };

        let output_artifact = ArtifactId::from(format!("tool-output-{}", recorder.next_sequence()));
        recorder.append(SessionEvent::Tool(ToolEvent::OutputRecorded {
            call_id: call_id.clone(),
            artifact_id: output_artifact.clone(),
            summary: format!("{} bytes", result.original_len),
            content_preview: preview_lines(&result, 20),
            truncated: result.truncated,
        }))?;
        recorder.append(SessionEvent::Tool(ToolEvent::CallFinished {
            call_id,
            status: ToolStatus::Succeeded,
            output_artifact: Some(output_artifact),
        }))?;
        Ok(result)
    }
}

pub fn default_session_path() -> Result<PathBuf, AgentError> {
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000);
    Ok(default_sessions_dir().join(format!("session-{stamp}-{}.jsonl", std::process::id())))
}

pub fn default_sessions_dir() -> PathBuf {
    let base = env::var_os("WHALE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".whale")))
        .unwrap_or_else(|| PathBuf::from(".whale"));
    base.join("sessions")
}

pub fn run_bootstrap_agent(
    task: impl Into<String>,
    cwd: impl AsRef<Path>,
    session_path: Option<PathBuf>,
) -> Result<AgentRunSummary, AgentError> {
    let session_path = match session_path {
        Some(path) => path,
        None => default_session_path()?,
    };
    AgentLoop::new("bootstrap-local").run(AgentRunOptions {
        task: task.into(),
        cwd: cwd.as_ref().to_path_buf(),
        session_path,
        model: "bootstrap-local".to_owned(),
    })
}

pub(crate) fn permission_event_kind(decision: &PermissionDecision) -> PermissionDecisionKind {
    match decision {
        PermissionDecision::Allow => PermissionDecisionKind::Allowed,
        PermissionDecision::Ask { .. } => PermissionDecisionKind::Asked,
        PermissionDecision::Deny { .. } => PermissionDecisionKind::Denied,
    }
}

fn preview_lines(result: &ToolResultEnvelope, max_lines: usize) -> String {
    let lines = result
        .content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join(", ");
    if result.truncated {
        format!("{lines}, ...")
    } else {
        lines
    }
}
