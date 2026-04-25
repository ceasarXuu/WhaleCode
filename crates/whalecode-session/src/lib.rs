use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use thiserror::Error;
use whalecode_protocol::{EventEnvelope, PrimitiveEvent, PrimitiveId, SessionEvent};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session event sequence must be monotonic")]
    NonMonotonicSequence,
    #[error("failed to open session file {path}: {source}")]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write session file {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read session file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("malformed jsonl at line {line}: {source}")]
    MalformedJson {
        line: usize,
        source: serde_json::Error,
    },
    #[error("json serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReplaySnapshot {
    pub event_count: usize,
    pub transcript: Vec<TranscriptEntry>,
    pub model_event_count: usize,
    pub tool_event_count: usize,
    pub tool_output_count: usize,
    pub permission_event_count: usize,
    pub patch_event_count: usize,
    pub phase_event_count: usize,
    pub turn_event_count: usize,
    pub registered_primitives: BTreeSet<PrimitiveId>,
    pub enabled_primitives: BTreeSet<PrimitiveId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptEntry {
    pub role: TranscriptRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptRole {
    User,
    Assistant,
}

pub struct JsonlSessionStore {
    path: PathBuf,
    writer: File,
    last_sequence: Option<u64>,
}

impl JsonlSessionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let path = path.as_ref().to_path_buf();
        let last_sequence = last_sequence_in_file(&path)?;
        let writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| SessionError::Open {
                path: path.clone(),
                source,
            })?;
        Ok(Self {
            path,
            writer,
            last_sequence,
        })
    }

    pub fn append(&mut self, event: &EventEnvelope<SessionEvent>) -> Result<(), SessionError> {
        ensure_monotonic(self.last_sequence, event.sequence)?;
        serde_json::to_writer(&mut self.writer, event)?;
        self.writer
            .write_all(b"\n")
            .map_err(|source| SessionError::Write {
                path: self.path.clone(),
                source,
            })?;
        self.writer.flush().map_err(|source| SessionError::Write {
            path: self.path.clone(),
            source,
        })?;
        self.last_sequence = Some(event.sequence);
        Ok(())
    }
}

pub fn read_jsonl(
    path: impl AsRef<Path>,
) -> Result<Vec<EventEnvelope<SessionEvent>>, SessionError> {
    let path = path.as_ref().to_path_buf();
    let file = File::open(&path).map_err(|source| SessionError::Open {
        path: path.clone(),
        source,
    })?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|source| SessionError::Read {
            path: path.clone(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let event = serde_json::from_str(&line).map_err(|source| SessionError::MalformedJson {
            line: index + 1,
            source,
        })?;
        events.push(event);
    }
    Ok(events)
}

pub fn replay_jsonl(path: impl AsRef<Path>) -> Result<ReplaySnapshot, SessionError> {
    let events = read_jsonl(path)?;
    replay_events(&events)
}

pub fn replay_events(
    events: &[EventEnvelope<SessionEvent>],
) -> Result<ReplaySnapshot, SessionError> {
    let mut previous = None;
    let mut snapshot = ReplaySnapshot::default();
    for event in events {
        ensure_monotonic(previous, event.sequence)?;
        previous = Some(event.sequence);
        snapshot.apply(&event.payload);
        snapshot.event_count += 1;
    }
    Ok(snapshot)
}

impl ReplaySnapshot {
    fn apply(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::Transcript(transcript) => {
                self.transcript.push(match transcript {
                    whalecode_protocol::TranscriptEvent::UserMessage { content } => {
                        TranscriptEntry {
                            role: TranscriptRole::User,
                            content: content.clone(),
                        }
                    }
                    whalecode_protocol::TranscriptEvent::AssistantMessage { content } => {
                        TranscriptEntry {
                            role: TranscriptRole::Assistant,
                            content: content.clone(),
                        }
                    }
                });
            }
            SessionEvent::Tool(tool) => {
                self.tool_event_count += 1;
                if matches!(tool, whalecode_protocol::ToolEvent::OutputRecorded { .. }) {
                    self.tool_output_count += 1;
                }
            }
            SessionEvent::Model(_) => {
                self.model_event_count += 1;
            }
            SessionEvent::Permission(_) => {
                self.permission_event_count += 1;
            }
            SessionEvent::Patch(_) => {
                self.patch_event_count += 1;
            }
            SessionEvent::Phase(_) => {
                self.phase_event_count += 1;
            }
            SessionEvent::Turn(_) => {
                self.turn_event_count += 1;
            }
            SessionEvent::Primitive(primitive) => match primitive {
                PrimitiveEvent::Registered { primitive_id } => {
                    self.registered_primitives.insert(primitive_id.clone());
                }
                PrimitiveEvent::Enabled { primitive_id } => {
                    self.enabled_primitives.insert(primitive_id.clone());
                }
                PrimitiveEvent::Disabled { primitive_id } => {
                    self.enabled_primitives.remove(primitive_id);
                }
                PrimitiveEvent::EvalRecorded { .. } => {}
            },
            _ => {}
        }
    }
}

fn last_sequence_in_file(path: &Path) -> Result<Option<u64>, SessionError> {
    if !path.exists() {
        return Ok(None);
    }
    let events = read_jsonl(path)?;
    let mut previous = None;
    for event in events {
        ensure_monotonic(previous, event.sequence)?;
        previous = Some(event.sequence);
    }
    Ok(previous)
}

fn ensure_monotonic(previous: Option<u64>, next: u64) -> Result<(), SessionError> {
    if previous.is_some_and(|seq| next <= seq) {
        return Err(SessionError::NonMonotonicSequence);
    }
    Ok(())
}
