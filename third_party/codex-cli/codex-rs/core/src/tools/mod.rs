pub(crate) mod code_mode;
pub(crate) mod context;
pub(crate) mod events;
pub(crate) mod handlers;
pub(crate) mod hook_names;
pub(crate) mod network_approval;
pub(crate) mod orchestrator;
pub(crate) mod output_reference;
pub(crate) mod parallel;
pub(crate) mod registry;
pub(crate) mod router;
pub(crate) mod runtimes;
pub(crate) mod sandboxing;
pub(crate) mod spec;
pub(crate) mod tool_dispatch_trace;
pub(crate) mod tool_search_entry;

use codex_protocol::exec_output::ExecToolCallOutput;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::formatted_truncate_text;
use codex_utils_output_truncation::truncate_text;
pub use router::ToolRouter;
use serde::Serialize;

// Telemetry preview limits: keep log events smaller than model budgets.
pub(crate) const TELEMETRY_PREVIEW_MAX_BYTES: usize = 2 * 1024; // 2 KiB
pub(crate) const TELEMETRY_PREVIEW_MAX_LINES: usize = 64; // lines
pub(crate) const TELEMETRY_PREVIEW_TRUNCATION_NOTICE: &str =
    "[... telemetry preview truncated ...]";

/// Format the combined exec output for sending back to the model.
/// Includes exit code and duration metadata; truncates large bodies safely.
pub fn format_exec_output_for_model_structured_with_ref(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
    artifact_ref: Option<&str>,
) -> String {
    let ExecToolCallOutput {
        exit_code,
        duration,
        ..
    } = exec_output;

    #[derive(Serialize)]
    struct ExecMetadata {
        exit_code: i32,
        duration_seconds: f32,
    }

    #[derive(Serialize)]
    struct ExecOutput<'a> {
        output: &'a str,
        metadata: ExecMetadata,
    }

    // round to 1 decimal place
    let duration_seconds = ((duration.as_secs_f32()) * 10.0).round() / 10.0;

    let formatted_output =
        format_exec_output_str_with_ref(exec_output, truncation_policy, artifact_ref);

    let payload = ExecOutput {
        output: &formatted_output,
        metadata: ExecMetadata {
            exit_code: *exit_code,
            duration_seconds,
        },
    };

    #[expect(clippy::expect_used)]
    serde_json::to_string(&payload).expect("serialize ExecOutput")
}

pub fn format_exec_output_for_model_freeform_with_ref(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
    artifact_ref: Option<&str>,
) -> String {
    // round to 1 decimal place
    let duration_seconds = ((exec_output.duration.as_secs_f32()) * 10.0).round() / 10.0;

    let content = build_content_with_timeout(exec_output);

    let total_lines = content.lines().count();

    let formatted_output =
        output_reference::reference_text_for_raw_output(content.as_bytes(), artifact_ref)
            .unwrap_or_else(|| truncate_text(&content, truncation_policy));
    let formatted_output = prepend_taskspace_semantic_summary(
        formatted_output,
        taskspace_tool_semantic_summary(&content),
    );

    let mut sections = Vec::new();

    sections.push(format!("Exit code: {}", exec_output.exit_code));
    sections.push(format!("Wall time: {duration_seconds} seconds"));
    if total_lines != formatted_output.lines().count() {
        sections.push(format!("Total output lines: {total_lines}"));
    }

    sections.push("Output:".to_string());
    sections.push(formatted_output);

    sections.join("\n")
}

pub fn format_exec_output_str(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
) -> String {
    format_exec_output_str_with_ref(exec_output, truncation_policy, None)
}

pub fn format_exec_output_str_with_ref(
    exec_output: &ExecToolCallOutput,
    truncation_policy: TruncationPolicy,
    artifact_ref: Option<&str>,
) -> String {
    let content = build_content_with_timeout(exec_output);
    if let Some(reference_text) =
        output_reference::reference_text_for_raw_output(content.as_bytes(), artifact_ref)
    {
        return reference_text;
    }

    // Truncate for model consumption before serialization, but keep compact
    // machine-readable failure semantics extracted from the complete output.
    let formatted_output = formatted_truncate_text(&content, truncation_policy);
    prepend_taskspace_semantic_summary(formatted_output, taskspace_tool_semantic_summary(&content))
}

/// Extracts exec output content and prepends a timeout message if the command timed out.
fn build_content_with_timeout(exec_output: &ExecToolCallOutput) -> String {
    if exec_output.timed_out {
        format!(
            "command timed out after {} milliseconds\n{}",
            exec_output.duration.as_millis(),
            exec_output.aggregated_output.text
        )
    } else {
        exec_output.aggregated_output.text.clone()
    }
}

pub(crate) fn prepend_taskspace_semantic_summary(
    preview: String,
    semantic_summary: Option<String>,
) -> String {
    let Some(summary) = semantic_summary else {
        return preview;
    };
    let summary = summary.trim();
    if summary.is_empty() || preview.contains(summary) {
        return preview;
    }
    format!("{summary}\n{preview}")
}

pub(crate) fn append_taskspace_tool_tail_sentinels(preview: String, full_text: &str) -> String {
    let Some(summary) = taskspace_read_file_summary_from_text(full_text) else {
        return preview;
    };
    if preview.contains(&summary) {
        return preview;
    }

    let mut output = preview;
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("TaskSpaceToolTailSentinelV1:\n");
    output.push_str(&summary);
    output
}

pub(crate) fn taskspace_tool_semantic_summary(text: &str) -> Option<String> {
    let properties = taskspace_required_properties_from_text(text);
    let type_mismatches = taskspace_schema_type_mismatches_from_text(text);
    if properties.is_empty() && type_mismatches.is_empty() {
        return None;
    }
    let mut lines = vec!["TaskSpaceToolSemanticSummaryV1:".to_string()];
    if !properties.is_empty() {
        lines.push(format!(
            "missing_required_properties: {}",
            properties.join(", ")
        ));
        let rename_hints = taskspace_property_rename_hints_from_text(text, &properties);
        if !rename_hints.is_empty() {
            lines.push(format!(
                "schema_property_rename_hints={}",
                rename_hints.join(", ")
            ));
        }
    }
    if !type_mismatches.is_empty() {
        lines.push(format!(
            "schema_type_mismatches: {}",
            type_mismatches.join(", ")
        ));
    }
    Some(lines.join("\n"))
}

fn taskspace_read_file_summary_from_text(text: &str) -> Option<String> {
    text.lines()
        .rev()
        .filter_map(|line| {
            let start = line.find("TaskSpaceReadFileSummaryV1:")?;
            let summary = line[start..].trim();
            taskspace_read_file_summary_has_parseable_eof(summary).then(|| summary.to_string())
        })
        .next()
}

fn taskspace_read_file_summary_has_parseable_eof(summary: &str) -> bool {
    summary
        .split_whitespace()
        .any(|part| matches!(part, "eof_reached=true" | "eof_reached=false"))
}

fn taskspace_required_properties_from_text(text: &str) -> Vec<String> {
    let mut properties = Vec::new();
    for line in text.lines() {
        if let Some((_, rest)) = line.split_once("missing_required_properties:") {
            let required_part = rest.split('|').next().unwrap_or(rest);
            for property in required_part
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                push_unique_taskspace_required_property(&mut properties, property.to_string());
            }
        }

        let lower = line.to_ascii_lowercase();
        if !lower.contains("is a required property") {
            continue;
        }
        let Some(marker_start) = lower.find("is a required property") else {
            continue;
        };
        let before = &line[..marker_start];
        if let Some(property) = taskspace_quoted_suffix_value(before) {
            push_unique_taskspace_required_property(&mut properties, property);
        }
    }
    properties
}

fn push_unique_taskspace_required_property(properties: &mut Vec<String>, property: String) {
    let property = property.trim().to_string();
    if property.is_empty() {
        return;
    }
    if !properties.iter().any(|existing| existing == &property) {
        properties.push(property);
    }
}

fn taskspace_quoted_suffix_value(text: &str) -> Option<String> {
    taskspace_quoted_suffix_value_with(text, '\'')
        .or_else(|| taskspace_quoted_suffix_value_with(text, '"'))
}

fn taskspace_quoted_suffix_value_with(text: &str, quote: char) -> Option<String> {
    let end = text.rfind(quote)?;
    let before_end = &text[..end];
    let start = before_end.rfind(quote)?;
    let value = before_end[start + quote.len_utf8()..].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn taskspace_schema_type_mismatches_from_text(text: &str) -> Vec<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut mismatches = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let lower = line.to_ascii_lowercase();
        let Some(marker_start) = lower.find(" is not of type ") else {
            continue;
        };
        let expected = taskspace_quoted_suffix_value(&line[marker_start..])
            .unwrap_or_else(|| "unknown".to_string());
        let property = lines
            .iter()
            .skip(index + 1)
            .take(8)
            .find_map(|candidate| taskspace_last_bracket_path_segment(candidate));
        let Some(property) = property else {
            continue;
        };
        let mismatch = format!("{property} expected {expected}");
        if !mismatches.iter().any(|existing| existing == &mismatch) {
            mismatches.push(mismatch);
        }
    }
    mismatches
}

fn taskspace_last_bracket_path_segment(line: &str) -> Option<String> {
    let mut rest = line;
    let mut last = None;
    while let Some(start) = rest.find("['") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("']") else {
            break;
        };
        let value = after_start[..end].trim();
        if !value.is_empty() && value != "properties" && value != "items" {
            last = Some(value.to_string());
        }
        rest = &after_start[end + 2..];
    }
    last
}

fn taskspace_property_rename_hints_from_text(
    text: &str,
    missing_required_properties: &[String],
) -> Vec<String> {
    let mut hints = Vec::new();
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("is a required property") {
            continue;
        }
        let Some(marker_start) = lower.find("is a required property") else {
            continue;
        };
        let before = &line[..marker_start];
        let Some(required) = taskspace_quoted_suffix_value(before) else {
            continue;
        };
        if !missing_required_properties
            .iter()
            .any(|property| property == &required)
        {
            continue;
        }
        for existing_key in taskspace_quoted_object_keys(before) {
            if taskspace_property_key_suggests_rename(&existing_key, &required) {
                let hint = format!("{existing_key}->{required}");
                if !hints.iter().any(|existing| existing == &hint) {
                    hints.push(hint);
                }
            }
        }
    }
    hints
}

fn taskspace_quoted_object_keys(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for quote in ['\'', '"'] {
        let mut rest = text;
        while let Some(start) = rest.find(quote) {
            let after_start = &rest[start + quote.len_utf8()..];
            let Some(end) = after_start.find(quote) else {
                break;
            };
            let value = &after_start[..end];
            let after_end = &after_start[end + quote.len_utf8()..];
            if after_end.trim_start().starts_with(':')
                && !keys.iter().any(|existing| existing == value)
            {
                keys.push(value.to_string());
            }
            rest = after_end;
        }
    }
    keys
}

fn taskspace_property_key_suggests_rename(existing_key: &str, required_property: &str) -> bool {
    let existing = taskspace_normalize_property_name(existing_key);
    let required = taskspace_normalize_property_name(required_property);
    if existing.is_empty() || required.is_empty() {
        return false;
    }
    if existing == required {
        return true;
    }
    let required_singular = required.strip_suffix('s').unwrap_or(&required);
    required_singular.len() >= 4 && existing.contains(required_singular)
}

fn taskspace_normalize_property_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::exec_output::StreamOutput;
    use std::time::Duration;

    #[test]
    fn exec_output_formatter_preserves_schema_summary_before_truncation() {
        let raw_output = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}",
            "{'statistics': {}}: 'averageDepartmentBudget' is a required property",
            "{'statistics': {}}: 'totalEmployees' is a required property",
            "{'statistics': {}}: 'skillDistribution' is a required property",
            "{'statistics': {}}: 'departmentSizes' is a required property",
            "x".repeat(4_096),
            "{'statistics': {}}: 'projectStatusDistribution' is a required property",
            "{'statistics': {}}: 'averageYearsOfService' is a required property",
        );
        let exec_output = ExecToolCallOutput {
            exit_code: 1,
            stdout: StreamOutput::new(String::new()),
            stderr: StreamOutput::new(String::new()),
            aggregated_output: StreamOutput::new(raw_output),
            duration: Duration::from_millis(100),
            timed_out: false,
        };

        let formatted =
            format_exec_output_str_with_ref(&exec_output, TruncationPolicy::Bytes(512), None);

        assert!(formatted.starts_with("TaskSpaceToolSemanticSummaryV1"));
        assert!(formatted.contains(
            "missing_required_properties: averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService"
        ));
    }

    #[test]
    fn exec_output_formatter_summarizes_schema_rename_hints() {
        let raw_output = "\
{'name': 'Madrid', 'member_ids': ['D001-E001']}: 'members' is a required property
{'total_employees': 12, 'project_status_distribution': {'In Progress': 3}}: 'totalEmployees' is a required property
{'total_employees': 12, 'project_status_distribution': {'In Progress': 3}}: 'projectStatusDistribution' is a required property";
        let exec_output = ExecToolCallOutput {
            exit_code: 1,
            stdout: StreamOutput::new(String::new()),
            stderr: StreamOutput::new(String::new()),
            aggregated_output: StreamOutput::new(raw_output.to_string()),
            duration: Duration::from_millis(100),
            timed_out: false,
        };

        let formatted =
            format_exec_output_str_with_ref(&exec_output, TruncationPolicy::Bytes(512), None);

        assert!(formatted.starts_with("TaskSpaceToolSemanticSummaryV1"));
        assert!(formatted.contains(
            "missing_required_properties: members, totalEmployees, projectStatusDistribution"
        ));
        assert!(formatted.contains(
            "schema_property_rename_hints=member_ids->members, total_employees->totalEmployees, project_status_distribution->projectStatusDistribution"
        ));
    }

    #[test]
    fn exec_output_formatter_summarizes_schema_type_mismatch() {
        let raw_output = "\
jsonschema.exceptions.ValidationError: [{'skill': 'Python', 'count': 4}] is not of type 'object'

Failed validating 'type' in schema['properties']['statistics']['properties']['skillDistribution']:
    {'type': 'object', 'additionalProperties': {'type': 'integer'}}

On instance['statistics']['skillDistribution']:
    [{'skill': 'Python', 'count': 4}]";
        let exec_output = ExecToolCallOutput {
            exit_code: 1,
            stdout: StreamOutput::new(String::new()),
            stderr: StreamOutput::new(String::new()),
            aggregated_output: StreamOutput::new(raw_output.to_string()),
            duration: Duration::from_millis(100),
            timed_out: false,
        };

        let formatted =
            format_exec_output_str_with_ref(&exec_output, TruncationPolicy::Bytes(512), None);

        assert!(formatted.starts_with("TaskSpaceToolSemanticSummaryV1"));
        assert!(formatted.contains("schema_type_mismatches: skillDistribution expected object"));
        assert!(!formatted.contains("missing_required_properties:"));
    }
}
