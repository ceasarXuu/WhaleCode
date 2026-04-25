use std::path::PathBuf;

use whalecode_model::{
    collect_model_output, ChatMessage, ChatMessageRole, CollectedModelOutput, DeepSeekChatRequest,
    DeepSeekClient, DeepSeekConfig, DeepSeekFunctionCall, DeepSeekToolCall, ModelStreamEvent,
};
use whalecode_patch::WorkspacePatchEngine;
use whalecode_permission::PermissionEngine;
use whalecode_protocol::{
    ModelEvent, ModelStreamDelta, PhaseEvent, SessionEvent, SessionFinishStatus,
    SessionLifecycleEvent, TranscriptEvent, TurnEvent, TurnFinishStatus, TurnId, WorkflowPhase,
};
use whalecode_tools::ToolRuntime;

use crate::{
    live_tool_defs::live_tool_defs, live_tools::execute_model_tool, recorder::ensure_parent_dir,
    recorder::EventRecorder, AgentError, AgentRunSummary,
};

const DEFAULT_MAX_TURNS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveAgentOptions {
    pub task: String,
    pub cwd: PathBuf,
    pub session_path: PathBuf,
    pub model: String,
    pub allow_write: bool,
    pub allow_command: bool,
    pub max_turns: usize,
}

pub async fn run_live_agent(options: LiveAgentOptions) -> Result<AgentRunSummary, AgentError> {
    if options.task.trim().is_empty() {
        return Err(AgentError::EmptyTask);
    }
    ensure_parent_dir(&options.session_path)?;

    let tools = ToolRuntime::new(&options.cwd)?;
    let patch_engine = WorkspacePatchEngine::new(&options.cwd)?;
    let permission = PermissionEngine;
    let mut recorder = EventRecorder::open(&options.session_path)?;
    let mut config = DeepSeekConfig::from_env();
    config.model = options.model.clone();
    let client = DeepSeekClient::new(config.clone());

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

    let mut messages = vec![
        system_message(options.allow_write, options.allow_command),
        ChatMessage::user(&options.task),
    ];
    let mut tool_summaries = Vec::new();
    let max_turns = options.max_turns.max(1);

    for turn_index in 1..=max_turns {
        recorder.set_turn(TurnId::from(format!("turn-{turn_index}")));
        recorder.append(SessionEvent::Turn(TurnEvent::Started {
            index: turn_index as u64,
        }))?;
        recorder.append(SessionEvent::Model(ModelEvent::RequestStarted {
            model: config.model.clone(),
        }))?;
        let request =
            DeepSeekChatRequest::streaming(&config, messages.clone()).with_tools(live_tool_defs());
        let events = match client.stream_chat(&request).await {
            Ok(events) => events,
            Err(error) => {
                recorder.append(SessionEvent::Model(ModelEvent::RequestFailed {
                    message: error.to_string(),
                }))?;
                recorder.append(SessionEvent::Turn(TurnEvent::Finished {
                    index: turn_index as u64,
                    status: TurnFinishStatus::Failed,
                }))?;
                recorder.clear_turn();
                recorder.append(SessionEvent::Session(SessionLifecycleEvent::Finished {
                    status: SessionFinishStatus::Failed,
                }))?;
                return Err(error.into());
            }
        };
        record_model_events(&mut recorder, &events)?;
        recorder.append(SessionEvent::Model(ModelEvent::RequestFinished {
            usage: None,
        }))?;

        let output = collect_model_output(&events);
        if output.tool_calls.is_empty() {
            let final_message = non_empty_final_text(&output.text);
            recorder.append(SessionEvent::Transcript(
                TranscriptEvent::AssistantMessage {
                    content: final_message.clone(),
                },
            ))?;
            recorder.append(SessionEvent::Phase(PhaseEvent::Transition {
                from: WorkflowPhase::Analyze,
                to: WorkflowPhase::Done,
            }))?;
            recorder.append(SessionEvent::Turn(TurnEvent::Finished {
                index: turn_index as u64,
                status: TurnFinishStatus::Completed,
            }))?;
            recorder.clear_turn();
            recorder.append(SessionEvent::Session(SessionLifecycleEvent::Finished {
                status: SessionFinishStatus::Succeeded,
            }))?;
            return Ok(AgentRunSummary {
                final_message,
                session_path: options.session_path,
                events_written: recorder.events_written(),
                tool_summaries,
            });
        }

        let calls = output.tool_calls.clone();
        messages.push(assistant_message(&output));
        for call in calls {
            let tool_result = match execute_model_tool(
                &tools,
                &patch_engine,
                &permission,
                &mut recorder,
                &call,
                options.allow_write,
                options.allow_command,
            )
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    recorder.append(SessionEvent::Turn(TurnEvent::Finished {
                        index: turn_index as u64,
                        status: TurnFinishStatus::Failed,
                    }))?;
                    recorder.clear_turn();
                    recorder.append(SessionEvent::Session(SessionLifecycleEvent::Finished {
                        status: SessionFinishStatus::Failed,
                    }))?;
                    return Err(error);
                }
            };
            tool_summaries.push(format!("{}: {}", call.name, tool_result.summary));
            messages.push(tool_message(call.id, tool_result.message));
        }
        recorder.append(SessionEvent::Turn(TurnEvent::Finished {
            index: turn_index as u64,
            status: TurnFinishStatus::Continued,
        }))?;
        recorder.clear_turn();
    }

    recorder.append(SessionEvent::Session(SessionLifecycleEvent::Finished {
        status: SessionFinishStatus::Failed,
    }))?;
    Err(AgentError::MaxTurns { max_turns })
}

pub fn default_live_max_turns() -> usize {
    DEFAULT_MAX_TURNS
}

fn record_model_events(
    recorder: &mut EventRecorder,
    events: &[ModelStreamEvent],
) -> Result<(), AgentError> {
    for event in events {
        match event {
            ModelStreamEvent::TextDelta(content) => {
                recorder.append(SessionEvent::Model(ModelEvent::StreamDelta {
                    delta: ModelStreamDelta::Text {
                        content: content.clone(),
                    },
                }))?;
            }
            ModelStreamEvent::ReasoningDelta(content) => {
                recorder.append(SessionEvent::Model(ModelEvent::StreamDelta {
                    delta: ModelStreamDelta::Reasoning {
                        content: content.clone(),
                    },
                }))?;
            }
            ModelStreamEvent::ToolCallDelta {
                name,
                arguments_delta,
                ..
            } => {
                recorder.append(SessionEvent::Model(ModelEvent::StreamDelta {
                    delta: ModelStreamDelta::ToolCall {
                        name: name.clone(),
                        arguments_delta: arguments_delta.clone(),
                    },
                }))?;
            }
            ModelStreamEvent::Finished => {}
        }
    }
    Ok(())
}

fn assistant_message(output: &CollectedModelOutput) -> ChatMessage {
    ChatMessage {
        role: ChatMessageRole::Assistant,
        content: output.text.clone(),
        reasoning_content: non_empty(output.reasoning.clone()),
        tool_call_id: None,
        tool_calls: Some(
            output
                .tool_calls
                .iter()
                .map(|call| DeepSeekToolCall {
                    id: call.id.clone(),
                    kind: "function".to_owned(),
                    function: DeepSeekFunctionCall {
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                    },
                })
                .collect(),
        ),
    }
}

fn tool_message(tool_call_id: String, content: String) -> ChatMessage {
    ChatMessage {
        role: ChatMessageRole::Tool,
        content,
        reasoning_content: None,
        tool_call_id: Some(tool_call_id),
        tool_calls: None,
    }
}

fn system_message(allow_write: bool, allow_command: bool) -> ChatMessage {
    let write_policy = if allow_write {
        "You may call edit_file. It applies one exact replacement at a time through a stale-read-safe patch engine."
    } else {
        "Do not call edit_file unless the user reruns with --allow-write."
    };
    let command_policy = if allow_command {
        "You may call run_command for bounded verification commands. Pass command and args separately; do not assume a shell."
    } else {
        "Do not call run_command unless the user reruns with --allow-command."
    };
    ChatMessage {
        role: ChatMessageRole::System,
        content: format!(
            "You are WhaleCode, a terminal coding agent. Inspect the repository before changing it. Use list_files, read_file, and search_text to gather evidence. {write_policy} {command_policy} Prefer minimal, testable fixes. Return a concise final summary after tool work is complete."
        ),
        reasoning_content: None,
        tool_call_id: None,
        tool_calls: None,
    }
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn non_empty_final_text(value: &str) -> String {
    if value.trim().is_empty() {
        "Live agent finished without a final message.".to_owned()
    } else {
        value.to_owned()
    }
}
