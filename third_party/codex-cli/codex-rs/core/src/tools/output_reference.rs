use codex_utils_string::take_bytes_at_char_boundary;
use serde_json::Value as JsonValue;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::path::PathBuf;

pub(crate) const OUTPUT_INLINE_THRESHOLD_BYTES: usize = 8 * 1024;
pub(crate) const OUTPUT_REFERENCE_THRESHOLD_BYTES: usize = 50 * 1024;
pub(crate) const OUTPUT_REFERENCE_SLICE_BYTES: usize = 2 * 1024;
pub(crate) const OUTPUT_SLICE_MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum OutputReferencePolicy {
    Inline,
    Summarized,
    Referenced,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct OutputReferenceV1 {
    pub(crate) policy: OutputReferencePolicy,
    pub(crate) sha256: String,
    pub(crate) artifact_ref: Option<String>,
    pub(crate) bytes: usize,
    pub(crate) lines: usize,
    pub(crate) head: String,
    pub(crate) tail: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum OutputSliceMode {
    Head,
    Tail,
    LineRange { start_line: usize, end_line: usize },
    Grep { pattern: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct OutputSliceRequest {
    pub(crate) mode: OutputSliceMode,
    pub(crate) max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutputSliceResult {
    pub(crate) mode: &'static str,
    pub(crate) range: JsonValue,
    pub(crate) truncated: bool,
    pub(crate) continuation: Option<JsonValue>,
    pub(crate) content: String,
}

impl OutputReferenceV1 {
    pub(crate) fn from_raw_output(raw_output: &[u8], policy: OutputReferencePolicy) -> Self {
        let text = String::from_utf8_lossy(raw_output);
        let sha256 = Sha256::digest(raw_output);
        Self {
            policy,
            sha256: format!("{sha256:x}"),
            artifact_ref: None,
            bytes: raw_output.len(),
            lines: text.lines().count(),
            head: redact_sensitive_text(take_bytes_at_char_boundary(
                &text,
                OUTPUT_REFERENCE_SLICE_BYTES,
            )),
            tail: redact_sensitive_text(&take_tail_bytes_at_char_boundary(
                &text,
                OUTPUT_REFERENCE_SLICE_BYTES,
            )),
        }
    }

    pub(crate) fn to_model_visible_text(&self) -> String {
        let output_ref = self.artifact_ref.as_deref().unwrap_or("unavailable");
        format!(
            "OutputReferenceV1:\noutput_ref: {}\nartifact_ref: {}\nsha256: {}\nbytes: {}\nlines: {}\npolicy: {}\nraw_output_elided: true\nsummary: Raw output is stored as an audit artifact and only bounded redacted slices are model-visible.\nsensitive_data_scan: redacted_model_visible\ninline_head_bytes: {}\ninline_tail_bytes: {}\nsuggested_slices:\n- mode=head max_bytes=4096\n- mode=tail max_bytes=4096\n- mode=line_range start_line=1 end_line=40 max_bytes=4096\n- mode=grep pattern=<literal> max_bytes=4096\n\n[head]\n{}\n\n[tail]\n{}",
            output_ref,
            output_ref,
            self.sha256,
            self.bytes,
            self.lines,
            self.policy.as_str(),
            self.head.len(),
            self.tail.len(),
            self.head,
            self.tail,
        )
    }
}

impl OutputReferencePolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline_output",
            Self::Summarized => "summarized_medium_output",
            Self::Referenced => "referenced_large_output",
        }
    }
}

impl OutputReferenceV1 {
    pub(crate) fn with_artifact_ref(mut self, artifact_ref: Option<String>) -> Self {
        self.artifact_ref = artifact_ref;
        self
    }
}

pub(crate) fn policy_for_raw_output(raw_output: &[u8]) -> OutputReferencePolicy {
    if raw_output.len() <= OUTPUT_INLINE_THRESHOLD_BYTES {
        OutputReferencePolicy::Inline
    } else if raw_output.len() <= OUTPUT_REFERENCE_THRESHOLD_BYTES {
        OutputReferencePolicy::Summarized
    } else {
        OutputReferencePolicy::Referenced
    }
}

pub(crate) fn reference_text_for_raw_output(
    raw_output: &[u8],
    artifact_ref: Option<&str>,
) -> Option<String> {
    let policy = policy_for_raw_output(raw_output);
    if policy == OutputReferencePolicy::Inline {
        return None;
    }
    Some(
        OutputReferenceV1::from_raw_output(raw_output, policy)
            .with_artifact_ref(artifact_ref.map(str::to_string))
            .to_model_visible_text(),
    )
}

pub(crate) async fn write_output_artifact_for_rollout(
    rollout_path: Option<&Path>,
    raw_output: &[u8],
) -> std::io::Result<Option<String>> {
    if policy_for_raw_output(raw_output) == OutputReferencePolicy::Inline {
        return Ok(None);
    }
    let Some(rollout_path) = rollout_path else {
        return Ok(None);
    };

    let output_ref =
        OutputReferenceV1::from_raw_output(raw_output, policy_for_raw_output(raw_output));
    let artifact_dir = output_artifact_dir(rollout_path);
    tokio::fs::create_dir_all(&artifact_dir).await?;
    let artifact_path = artifact_dir.join(format!("{}.stdout", output_ref.sha256));
    tokio::fs::write(&artifact_path, raw_output).await?;
    Ok(Some(format!("output-ref://sha256/{}", output_ref.sha256)))
}

fn output_artifact_dir(rollout_path: &Path) -> PathBuf {
    session_store_root(rollout_path)
        .join("session-store")
        .join("output-refs")
        .join("sha256")
}

fn session_store_root(rollout_path: &Path) -> &Path {
    rollout_path
        .ancestors()
        .find_map(
            |ancestor| match ancestor.file_name().and_then(|name| name.to_str()) {
                Some("sessions" | "archived_sessions") => ancestor.parent(),
                _ => None,
            },
        )
        .or_else(|| rollout_path.parent())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
pub(crate) async fn read_output_artifact_slice(
    rollout_path: Option<&Path>,
    output_ref: &str,
    request: OutputSliceRequest,
) -> std::io::Result<String> {
    let Some(rollout_path) = rollout_path else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "output artifact rollout path is unavailable",
        ));
    };
    let sha = parse_output_ref_sha(output_ref)?;
    let artifact_path = output_artifact_dir(rollout_path).join(format!("{sha}.stdout"));
    let raw_output = tokio::fs::read(&artifact_path).await?;
    read_output_bytes_slice(output_ref, &raw_output, request)
}

pub(crate) async fn read_output_artifact_slice_result(
    rollout_path: Option<&Path>,
    output_ref: &str,
    request: OutputSliceRequest,
) -> std::io::Result<OutputSliceResult> {
    let Some(rollout_path) = rollout_path else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "output artifact rollout path is unavailable",
        ));
    };
    let sha = parse_output_ref_sha(output_ref)?;
    let artifact_path = output_artifact_dir(rollout_path).join(format!("{sha}.stdout"));
    let raw_output = tokio::fs::read(&artifact_path).await?;
    read_output_bytes_slice_result(output_ref, &raw_output, request)
}

#[cfg(test)]
pub(crate) fn read_output_bytes_slice(
    output_ref: &str,
    raw_output: &[u8],
    request: OutputSliceRequest,
) -> std::io::Result<String> {
    let sha = parse_output_ref_sha(output_ref)?;
    let actual_sha = format!("{:x}", Sha256::digest(raw_output));
    if actual_sha != sha {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "output artifact sha256 mismatch for {output_ref}: expected {sha}, got {actual_sha}"
            ),
        ));
    }
    let text = String::from_utf8_lossy(raw_output);
    let max_bytes = request.max_bytes.clamp(1, OUTPUT_SLICE_MAX_BYTES);
    let slice = match request.mode {
        OutputSliceMode::Head => take_bytes_at_char_boundary(&text, max_bytes).to_string(),
        OutputSliceMode::Tail => take_tail_bytes_at_char_boundary(&text, max_bytes),
        OutputSliceMode::LineRange {
            start_line,
            end_line,
        } => bounded_line_range(&text, start_line, end_line, max_bytes),
        OutputSliceMode::Grep { pattern } => bounded_grep(&text, &pattern, max_bytes),
    };
    let slice = redact_sensitive_text(&slice);
    Ok(format!(
        "OutputSliceV1:\nartifact_ref: {output_ref}\nsha256: {sha}\nbytes: {}\nsensitive_data_scan: redacted_model_visible\n\n{}",
        slice.len(),
        slice
    ))
}

pub(crate) fn read_output_bytes_slice_result(
    output_ref: &str,
    raw_output: &[u8],
    request: OutputSliceRequest,
) -> std::io::Result<OutputSliceResult> {
    let sha = parse_output_ref_sha(output_ref)?;
    let actual_sha = format!("{:x}", Sha256::digest(raw_output));
    if actual_sha != sha {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "output artifact sha256 mismatch for {output_ref}: expected {sha}, got {actual_sha}"
            ),
        ));
    }
    let text = String::from_utf8_lossy(raw_output);
    let max_bytes = request.max_bytes.clamp(1, OUTPUT_SLICE_MAX_BYTES);
    let total_lines = text.lines().count();
    let (mode, content, range, truncated, continuation) = match request.mode {
        OutputSliceMode::Head => {
            let content = take_bytes_at_char_boundary(&text, max_bytes).to_string();
            let returned_bytes = content.len();
            (
                "head",
                content,
                serde_json::json!({
                    "start_byte": 0,
                    "returned_bytes": returned_bytes,
                    "total_bytes": raw_output.len(),
                }),
                returned_bytes < text.len(),
                None,
            )
        }
        OutputSliceMode::Tail => {
            let content = take_tail_bytes_at_char_boundary(&text, max_bytes);
            let returned_bytes = content.len();
            (
                "tail",
                content,
                serde_json::json!({
                    "start_byte": text.len().saturating_sub(returned_bytes),
                    "returned_bytes": returned_bytes,
                    "total_bytes": raw_output.len(),
                }),
                returned_bytes < text.len(),
                None,
            )
        }
        OutputSliceMode::LineRange {
            start_line,
            end_line,
        } => {
            if start_line == 0 || end_line < start_line {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "line_range requires 1 <= start_line <= end_line",
                ));
            }
            let selected = text
                .lines()
                .enumerate()
                .filter_map(|(index, line)| {
                    let line_no = index + 1;
                    (line_no >= start_line && line_no <= end_line).then_some(line)
                })
                .collect::<Vec<_>>()
                .join("\n");
            let content = take_bytes_at_char_boundary(&selected, max_bytes).to_string();
            let truncated = content.len() < selected.len();
            let returned_lines = content.lines().count();
            let continuation = truncated.then(|| {
                serde_json::json!({
                    "action": "read_output_ref",
                    "output_ref": output_ref,
                    "mode": "line_range",
                    "start_line": start_line.saturating_add(returned_lines.max(1)),
                    "end_line": end_line,
                    "max_bytes": max_bytes,
                })
            });
            (
                "line_range",
                content,
                serde_json::json!({
                    "requested_start_line": start_line,
                    "requested_end_line": end_line,
                    "returned_line_count": returned_lines,
                    "total_lines": total_lines,
                }),
                truncated,
                continuation,
            )
        }
        OutputSliceMode::Grep { pattern } => {
            let matching = text
                .lines()
                .enumerate()
                .filter_map(|(index, line)| {
                    line.contains(&pattern)
                        .then(|| format!("{}:{line}\n", index + 1))
                })
                .collect::<String>();
            let content = take_bytes_at_char_boundary(&matching, max_bytes).to_string();
            let truncated = content.len() < matching.len();
            let returned_bytes = content.len();
            (
                "grep",
                content,
                serde_json::json!({
                    "pattern": pattern,
                    "returned_bytes": returned_bytes,
                    "total_matching_bytes": matching.len(),
                }),
                truncated,
                None,
            )
        }
    };
    Ok(OutputSliceResult {
        mode,
        range,
        truncated,
        continuation,
        content: redact_sensitive_text(&content),
    })
}

fn parse_output_ref_sha(output_ref: &str) -> std::io::Result<&str> {
    let Some(sha) = output_ref.strip_prefix("output-ref://sha256/") else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output_ref must use output-ref://sha256/<sha256>",
        ));
    };
    if sha.len() != 64 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output_ref sha256 must be 64 hex characters",
        ));
    }
    Ok(sha)
}

#[cfg(test)]
fn bounded_line_range(text: &str, start_line: usize, end_line: usize, max_bytes: usize) -> String {
    if start_line == 0 || end_line < start_line {
        return String::new();
    }
    let selected = text
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_no = idx + 1;
            (line_no >= start_line && line_no <= end_line).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    take_bytes_at_char_boundary(&selected, max_bytes).to_string()
}

#[cfg(test)]
fn bounded_grep(text: &str, pattern: &str, max_bytes: usize) -> String {
    let mut output = String::new();
    for (idx, line) in text.lines().enumerate() {
        if !line.contains(pattern) {
            continue;
        }
        let next = format!("{}:{line}\n", idx + 1);
        if output.len().saturating_add(next.len()) > max_bytes {
            break;
        }
        output.push_str(&next);
    }
    output
}

fn take_tail_bytes_at_char_boundary(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let start = text.len().saturating_sub(max_bytes);
    let start = (start..=text.len())
        .find(|idx| text.is_char_boundary(*idx))
        .unwrap_or(text.len());
    text[start..].to_string()
}

fn redact_sensitive_text(text: &str) -> String {
    text.lines()
        .map(redact_sensitive_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_sensitive_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let lowercase = trimmed.to_ascii_lowercase();
    if lowercase.starts_with("authorization: bearer ") {
        let prefix_len = line.len() - trimmed.len() + "authorization: bearer ".len();
        return format!("{}[REDACTED]", &line[..prefix_len]);
    }
    for key in [
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "secret",
        "password",
        "credential",
    ] {
        if let Some(redacted) = redact_assignment_value(line, key) {
            return redacted;
        }
    }
    line.to_string()
}

fn redact_assignment_value(line: &str, key: &str) -> Option<String> {
    let lowercase = line.to_ascii_lowercase();
    let key_start = lowercase.find(key)?;
    let after_key = &line[key_start + key.len()..];
    let delimiter_offset = after_key.find(['=', ':'])?;
    let value_start = key_start + key.len() + delimiter_offset + 1;
    if line[value_start..].trim().is_empty() {
        return None;
    }
    let value_indent = line[value_start..]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let redaction_start = value_start + value_indent;
    Some(format!("{}[REDACTED]", &line[..redaction_start]))
}

#[cfg(test)]
#[path = "output_reference_tests.rs"]
mod tests;
