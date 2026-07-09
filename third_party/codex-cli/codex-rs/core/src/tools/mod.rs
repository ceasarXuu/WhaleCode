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

    formatted_truncate_text(&content, truncation_policy)
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

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::exec_output::StreamOutput;
    use std::time::Duration;

    #[test]
    fn exec_output_formatter_truncates_without_semantic_rewrite() {
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

        assert!(
            formatted
                .contains("{'statistics': {}}: 'averageDepartmentBudget' is a required property")
        );
        assert!(!formatted.contains("TaskSpaceToolSemanticSummaryV1"));
        assert!(!formatted.contains("schema_property_rename_hints="));
    }

    #[test]
    fn exec_output_formatter_does_not_infer_schema_rename_hints() {
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

        assert!(formatted.starts_with("{'name': 'Madrid'"));
        assert!(!formatted.contains("TaskSpaceToolSemanticSummaryV1"));
        assert!(!formatted.contains("schema_property_rename_hints="));
    }

    #[test]
    fn exec_output_formatter_does_not_reinterpret_schema_type_mismatch() {
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

        assert!(formatted.starts_with("jsonschema.exceptions.ValidationError"));
        assert!(!formatted.contains("TaskSpaceToolSemanticSummaryV1"));
        assert!(!formatted.contains("schema_type_mismatches:"));
    }

    #[test]
    fn exec_output_formatter_does_not_treat_data_lists_as_schema_paths() {
        let raw_output = "\
{'projects': ['Madrid', 'Ferrari']}: 'id' is a required property
{'name': 'RedBull'}: {'name': 'RedBull'} is not of type 'string'
{'name': 'McLaren'}: {'name': 'McLaren'} is not of type 'string'";
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

        assert!(!formatted.contains("missing_required_properties:"));
        assert!(
            !formatted.contains("RedBull expected string"),
            "{formatted}"
        );
        assert!(
            !formatted.contains("McLaren expected string"),
            "{formatted}"
        );
    }
}
