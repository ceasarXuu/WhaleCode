use serde_json::Deserializer;

use super::plan::TaskspaceExecPlan;

const SOURCE_PREFIX: &str = "taskspace.plan(";
const MAX_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TaskspaceExecDecodeError {
    pub(crate) reason_code: &'static str,
    pub(crate) message: String,
}

pub(crate) fn decode_taskspace_exec_source(
    source: &str,
) -> Result<TaskspaceExecPlan, TaskspaceExecDecodeError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(error(
            "source_too_large",
            format!(
                "TaskSpace Exec source is {} bytes; maximum is {MAX_SOURCE_BYTES}",
                source.len()
            ),
        ));
    }
    let source = source.trim();
    if source.is_empty() {
        return Err(error("source_empty", "TaskSpace Exec source is empty"));
    }
    if source.starts_with("```") || source.ends_with("```") {
        return Err(error(
            "source_markdown_fence",
            "TaskSpace Exec source must not use Markdown fences",
        ));
    }
    let Some(body) = source.strip_prefix(SOURCE_PREFIX) else {
        return Err(error(
            "source_wrapper_invalid",
            "TaskSpace Exec source must start with `taskspace.plan(`",
        ));
    };
    let body = body.strip_suffix(';').unwrap_or(body).trim_end();
    let Some(json) = body.strip_suffix(')') else {
        return Err(error(
            "source_wrapper_invalid",
            "TaskSpace Exec source must end with `)` or `);`",
        ));
    };

    let mut deserializer = Deserializer::from_str(json.trim());
    let plan = serde_path_to_error::deserialize(&mut deserializer).map_err(|decode_error| {
        let path = decode_error.path().to_string();
        let detail = decode_error.into_inner();
        error(
            "source_plan_invalid",
            format!("TaskSpace Exec plan is invalid at `{path}`: {detail}"),
        )
    })?;
    deserializer.end().map_err(|detail| {
        error(
            "source_trailing_content",
            format!("TaskSpace Exec plan has trailing content: {detail}"),
        )
    })?;
    Ok(plan)
}

fn error(reason_code: &'static str, message: impl Into<String>) -> TaskspaceExecDecodeError {
    TaskspaceExecDecodeError {
        reason_code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::tools::taskspace_exec::TASKSPACE_EXEC_PLAN_VERSION;

    fn valid_source() -> String {
        format!(
            r#"taskspace.plan({{
                "version":"{TASKSPACE_EXEC_PLAN_VERSION}",
                "capability_id":"sha256:catalog",
                "calls":[
                    {{"item_id":"map-1","tool":"taskspace_control","input":{{"action":"read_map"}}}},
                    {{"item_id":"work-1","tool":"read_file","node_id":"inspect","input":{{"path":"README.md"}}}},
                    {{"item_id":"patch-1","tool":"apply_patch","node_id":"fix","input":"*** Begin Patch"}}
                ],
                "hosted_records":[{{
                    "response_id":"resp-1",
                    "provider_item_type":"web_search_call",
                    "provider_item_id":"ws-1",
                    "node_id":"research"
                }}]
            }});"#
        )
    }

    #[test]
    fn strict_declarative_source_decodes_to_one_complete_plan() {
        let plan = decode_taskspace_exec_source(&valid_source()).expect("valid plan");

        assert_eq!(plan.version, TASKSPACE_EXEC_PLAN_VERSION);
        assert_eq!(plan.calls.len(), 3);
        assert_eq!(plan.calls[1].node_id.as_deref(), Some("inspect"));
        assert_eq!(plan.calls[1].input, json!({"path": "README.md"}));
        assert_eq!(plan.hosted_records.len(), 1);
    }

    #[test]
    fn dynamic_javascript_cannot_be_used_as_a_plan() {
        for source in [
            "const calls = []; taskspace.plan({calls});",
            "if (ready) taskspace.plan({});",
            "await tools.read_file({path: 'README.md'});",
            "taskspace.plan(buildPlan());",
        ] {
            assert!(decode_taskspace_exec_source(source).is_err(), "{source}");
        }
    }

    #[test]
    fn unknown_fields_and_trailing_statements_fail_closed() {
        let unknown = valid_source().replace(
            "\"capability_id\":\"sha256:catalog\",",
            "\"capability_id\":\"sha256:catalog\",\"surprise\":true,",
        );
        assert_eq!(
            decode_taskspace_exec_source(&unknown)
                .expect_err("unknown field")
                .reason_code,
            "source_plan_invalid"
        );

        let trailing = valid_source().replace(");", "); cleanup();");
        assert_eq!(
            decode_taskspace_exec_source(&trailing)
                .expect_err("trailing statement")
                .reason_code,
            "source_trailing_content"
        );
    }

    #[test]
    fn malformed_wrapper_markdown_and_empty_source_have_stable_reasons() {
        let cases = [
            ("", "source_empty"),
            ("```json\n{}\n```", "source_markdown_fence"),
            ("{}", "source_wrapper_invalid"),
            ("taskspace.plan({}", "source_wrapper_invalid"),
        ];
        for (source, expected) in cases {
            assert_eq!(
                decode_taskspace_exec_source(source)
                    .expect_err("invalid source")
                    .reason_code,
                expected
            );
        }
    }

    #[test]
    fn oversized_source_is_rejected_before_parsing() {
        let source = "x".repeat(MAX_SOURCE_BYTES + 1);
        assert_eq!(
            decode_taskspace_exec_source(&source)
                .expect_err("oversized source")
                .reason_code,
            "source_too_large"
        );
    }
}
