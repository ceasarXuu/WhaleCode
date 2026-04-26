use std::{
    collections::{BTreeSet, HashMap},
    io::{self, Write},
    path::Path,
    time::Instant,
};

use whalecode_protocol::{
    ArtifactId, ModelEvent, ModelUsage, PatchApplyStatus, PatchEvent, PermissionDecisionKind,
    PermissionEvent, SessionEvent, ToolCallId, ToolEvent, ToolStatus,
};

pub(crate) struct RunDisplayConfig<'a> {
    pub(crate) workspace: &'a Path,
    pub(crate) model: &'a str,
    pub(crate) allow_write: bool,
    pub(crate) allow_command: bool,
    pub(crate) max_turns: usize,
    pub(crate) session_path: Option<&'a Path>,
}

#[derive(Debug)]
pub(crate) struct RunStatus {
    started_at: Instant,
    tool_calls: usize,
    active_tools: HashMap<ToolCallId, ToolLabel>,
    pending_artifacts: HashMap<ArtifactId, Vec<String>>,
    changed_files: BTreeSet<String>,
    write_error: Option<io::Error>,
}

#[derive(Debug, Clone)]
struct ToolLabel {
    name: String,
    input: Option<String>,
}

impl RunStatus {
    pub(crate) fn new() -> Self {
        Self {
            started_at: Instant::now(),
            tool_calls: 0,
            active_tools: HashMap::new(),
            pending_artifacts: HashMap::new(),
            changed_files: BTreeSet::new(),
            write_error: None,
        }
    }

    pub(crate) fn observe(&mut self, event: &SessionEvent) {
        if self.write_error.is_some() {
            return;
        }
        let result = match event {
            SessionEvent::Tool(event) => self.observe_tool(event),
            SessionEvent::Permission(event) => self.observe_permission(event),
            SessionEvent::Patch(event) => self.observe_patch(event),
            SessionEvent::Model(ModelEvent::RequestFailed { message }) => {
                self.write_line(format!("model: failed {}", preview(message, 120)))
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.write_error = Some(error);
        }
    }

    pub(crate) fn print_summary(&mut self, usage: &ModelUsage) -> io::Result<()> {
        if let Some(error) = self.write_error.take() {
            return Err(error);
        }
        let elapsed = self.started_at.elapsed();
        self.write_line(format!("input tokens: {}", usage.input_tokens))?;
        self.write_line(format!("output tokens: {}", usage.output_tokens))?;
        self.write_line(format!(
            "cached input tokens: {}",
            usage.cached_input_tokens
        ))?;
        self.write_line(format!("tool calls: {}", self.tool_calls))?;
        self.write_line(format!(
            "files changed: {}",
            format_changed_files(&self.changed_files)
        ))?;
        self.write_line(format!("duration: {:.1}s", elapsed.as_secs_f64()))
    }

    pub(crate) fn take_error(&mut self) -> Option<io::Error> {
        self.write_error.take()
    }

    fn observe_tool(&mut self, event: &ToolEvent) -> io::Result<()> {
        match event {
            ToolEvent::CallStarted {
                call_id,
                tool_name,
                input_summary,
            } => {
                self.tool_calls += 1;
                self.active_tools.insert(
                    call_id.clone(),
                    ToolLabel {
                        name: tool_name.clone(),
                        input: input_summary.clone(),
                    },
                );
                self.write_line(format!(
                    "tool: {}{}",
                    tool_name,
                    format_optional_input(input_summary.as_deref())
                ))
            }
            ToolEvent::OutputRecorded {
                call_id,
                summary,
                truncated,
                ..
            } => {
                let label = self.tool_label(call_id);
                self.write_line(format!(
                    "tool: {} -> {}{}",
                    label,
                    preview(summary, 120),
                    if *truncated { " (truncated)" } else { "" }
                ))
            }
            ToolEvent::CallFinished {
                call_id, status, ..
            } => {
                if matches!(status, ToolStatus::Succeeded) {
                    return Ok(());
                }
                let label = self.tool_label(call_id);
                self.write_line(format!("tool: {} {}", label, tool_status(status)))
            }
        }
    }

    fn observe_permission(&mut self, event: &PermissionEvent) -> io::Result<()> {
        match event {
            PermissionEvent::Decision { subject, decision } => {
                if should_print_permission(subject, decision) {
                    self.write_line(format!("permission: {} {}", subject, permission(decision)))?;
                }
            }
        }
        Ok(())
    }

    fn observe_patch(&mut self, event: &PatchEvent) -> io::Result<()> {
        match event {
            PatchEvent::ArtifactCreated {
                artifact_id,
                touched_files,
            } => {
                self.pending_artifacts
                    .insert(artifact_id.clone(), touched_files.clone());
                Ok(())
            }
            PatchEvent::ApplyResult {
                artifact_id,
                status,
            } => match status {
                PatchApplyStatus::Applied => {
                    let files = self
                        .pending_artifacts
                        .remove(artifact_id)
                        .unwrap_or_default();
                    for file in &files {
                        self.changed_files.insert(file.clone());
                    }
                    self.write_line(format!("modified: {}", format_files(&files)))
                }
                PatchApplyStatus::Conflict | PatchApplyStatus::Rejected => {
                    self.write_line(format!("patch: {}", patch_status(status)))
                }
            },
        }
    }

    fn tool_label(&self, call_id: &ToolCallId) -> String {
        self.active_tools
            .get(call_id)
            .map(|tool| {
                format!(
                    "{}{}",
                    tool.name,
                    format_optional_input(tool.input.as_deref())
                )
            })
            .unwrap_or_else(|| call_id.0.clone())
    }

    fn write_line(&mut self, line: String) -> io::Result<()> {
        let mut stdout = io::stdout();
        writeln!(stdout, "{line}")?;
        stdout.flush()
    }
}

pub(crate) fn print_startup_status(config: &RunDisplayConfig<'_>) -> io::Result<()> {
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "workspace: {} | model: {} | write_file: {} | edit_file: {} | run_command: {} | max_turns: {}",
        config.workspace.display(),
        config.model,
        enabled_label(config.allow_write),
        enabled_label(config.allow_write),
        enabled_label(config.allow_command),
        config.max_turns
    )?;
    if let Some(session_path) = config.session_path {
        writeln!(stdout, "session: {}", session_path.display())?;
    }
    stdout.flush()
}

fn enabled_label(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn should_print_permission(subject: &str, decision: &PermissionDecisionKind) -> bool {
    !matches!(decision, PermissionDecisionKind::Allowed)
        || matches!(subject, "edit_file" | "write_file" | "run_command")
}

fn format_optional_input(input: Option<&str>) -> String {
    input
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(" {}", preview(value, 120)))
        .unwrap_or_default()
}

fn format_changed_files(files: &BTreeSet<String>) -> String {
    if files.is_empty() {
        "0".to_owned()
    } else {
        format!("{} ({})", files.len(), format_files(files))
    }
}

fn format_files<'a>(files: impl IntoIterator<Item = &'a String>) -> String {
    let files = files.into_iter().map(String::as_str).collect::<Vec<_>>();
    if files.is_empty() {
        return "0".to_owned();
    }
    let shown = files.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    if files.len() > 3 {
        format!("{shown}, +{} more", files.len() - 3)
    } else {
        shown
    }
}

fn preview(value: &str, max_len: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= max_len {
        return compact;
    }
    let mut boundary = max_len;
    while !compact.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...", &compact[..boundary])
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

#[cfg(test)]
mod tests {
    use whalecode_protocol::{ArtifactId, PatchEvent, SessionEvent};

    use super::RunStatus;

    #[test]
    fn run_status_tracks_changed_files_only_after_applied_patch() {
        let mut status = RunStatus::new();
        let artifact = ArtifactId::from("patch-1");

        status.observe(&SessionEvent::Patch(PatchEvent::ArtifactCreated {
            artifact_id: artifact.clone(),
            touched_files: vec!["src/lib.rs".to_owned()],
        }));

        assert!(status.changed_files.is_empty());

        status.observe(&SessionEvent::Patch(PatchEvent::ApplyResult {
            artifact_id: artifact,
            status: whalecode_protocol::PatchApplyStatus::Applied,
        }));

        assert!(status.changed_files.contains("src/lib.rs"));
    }
}
