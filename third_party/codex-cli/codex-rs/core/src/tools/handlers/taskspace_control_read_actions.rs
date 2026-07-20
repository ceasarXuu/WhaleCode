use crate::action_map::projection_identity_from_context;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::taskspace_control_output::action_failure_error;
use crate::tools::handlers::taskspace_control_output::format_read_result;
use crate::tools::output_reference::OutputSliceMode;
use crate::tools::output_reference::OutputSliceRequest;
use crate::tools::output_reference::read_output_artifact_slice_result;

use super::ControlExecution;

#[allow(clippy::too_many_arguments)]
pub(super) async fn read_output_ref(
    session: &Session,
    turn: &TurnContext,
    output_ref: String,
    mode: String,
    start_line: Option<usize>,
    end_line: Option<usize>,
    pattern: Option<String>,
    max_bytes: usize,
) -> Result<ControlExecution, FunctionCallError> {
    let canonical_revision = canonical_revision(session).await;
    let request = OutputSliceRequest {
        mode: parse_output_slice_mode(&mode, start_line, end_line, pattern, canonical_revision)?,
        max_bytes,
    };
    let rollout_path = session.current_rollout_path().await.map_err(|error| {
        action_failure_error(
            "read_output_ref",
            None,
            canonical_revision,
            "resource",
            "TASKSPACE_RESOURCE_FAILURE",
            "resource_failed",
            error.to_string(),
        )
    })?;
    let slice = read_output_artifact_slice_result(rollout_path.as_deref(), &output_ref, request)
        .await
        .map_err(|error| output_read_error(error, canonical_revision))?;
    session
        .record_action_map_output_ref_trace_event(
            turn,
            "output_ref.slice_read",
            None,
            output_ref.clone(),
            vec![
                "output_ref".into(),
                "slice_read".into(),
                format!("mode:{mode}"),
            ],
        )
        .await;
    Ok((
        format_read_result(
            "read_output_ref",
            canonical_revision,
            serde_json::json!({
                "kind": "output_range",
                "output_ref": output_ref,
                "mode": slice.mode,
                "range": slice.range,
                "truncated": slice.truncated,
                "continuation": slice.continuation,
                "content": slice.content,
            }),
        ),
        true,
        None,
    ))
}

pub(super) async fn read_map(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
) -> Result<ControlExecution, FunctionCallError> {
    tracing::info!(
        target: "codex_core::taskspace",
        call_id,
        "taskspace.map_read_requested"
    );
    let projection = match session.read_action_map_projection(turn, call_id).await {
        Ok(projection) => projection,
        Err(error) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                call_id,
                error,
                "taskspace.map_read_rejected"
            );
            return Err(action_failure_error(
                "read_map",
                None,
                canonical_revision(session).await,
                "protocol",
                "TASKSPACE_PROTOCOL_FAILURE",
                "protocol_failed",
                error,
            ));
        }
    };
    let canonical_revision = canonical_revision(session).await;
    let identity = projection_identity_from_context(&projection).ok_or_else(|| {
        map_projection_failure(
            canonical_revision,
            "TaskSpace current Map projection identity is invalid.",
        )
    })?;
    let map_id = identity.map_id.ok_or_else(|| {
        map_projection_failure(
            canonical_revision,
            "TaskSpace current Map projection map_id is missing.",
        )
    })?;
    let revision = identity.revision.ok_or_else(|| {
        map_projection_failure(
            canonical_revision,
            "TaskSpace current Map projection revision is missing.",
        )
    })?;
    let canonical_sha256 = identity.canonical_sha256.ok_or_else(|| {
        map_projection_failure(
            canonical_revision,
            "TaskSpace current Map projection canonical_sha256 is missing.",
        )
    })?;
    Ok((
        format_read_result(
            "read_map",
            Some(revision),
            serde_json::json!({
                "kind": "map_projection",
                "map_id": map_id,
                "revision": revision,
                "canonical_sha256": canonical_sha256,
                "content": projection,
            }),
        ),
        true,
        None,
    ))
}

async fn canonical_revision(session: &Session) -> Option<u64> {
    session
        .action_map_control_state(None)
        .await
        .map(|state| state.revision)
}

fn parse_output_slice_mode(
    mode: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    pattern: Option<String>,
    canonical_revision: Option<u64>,
) -> Result<OutputSliceMode, FunctionCallError> {
    match mode {
        "head" => Ok(OutputSliceMode::Head),
        "tail" => Ok(OutputSliceMode::Tail),
        "line_range" => match (start_line, end_line) {
            (Some(start_line), Some(end_line)) => Ok(OutputSliceMode::LineRange {
                start_line,
                end_line,
            }),
            _ => Err(read_argument_failure(
                canonical_revision,
                "TASKSPACE_RANGE_INVALID",
                "read_output_ref line_range requires start_line and end_line",
            )),
        },
        "grep" => pattern.map_or_else(
            || {
                Err(read_argument_failure(
                    canonical_revision,
                    "TASKSPACE_INVALID_ARGUMENT",
                    "read_output_ref grep requires pattern",
                ))
            },
            |pattern| Ok(OutputSliceMode::Grep { pattern }),
        ),
        _ => Err(read_argument_failure(
            canonical_revision,
            "TASKSPACE_INVALID_ARGUMENT",
            "read_output_ref mode must be head, tail, line_range, or grep",
        )),
    }
}

fn output_read_error(error: std::io::Error, canonical_revision: Option<u64>) -> FunctionCallError {
    let message = error.to_string();
    match error.kind() {
        std::io::ErrorKind::InvalidInput if message.starts_with("output_ref ") => {
            read_argument_failure(canonical_revision, "TASKSPACE_INVALID_ARGUMENT", message)
        }
        std::io::ErrorKind::InvalidInput => {
            read_argument_failure(canonical_revision, "TASKSPACE_RANGE_INVALID", message)
        }
        std::io::ErrorKind::NotFound => action_failure_error(
            "read_output_ref",
            None,
            canonical_revision,
            "resource",
            "TASKSPACE_OUTPUT_REF_NOT_FOUND",
            "resource_failed",
            message,
        ),
        _ => action_failure_error(
            "read_output_ref",
            None,
            canonical_revision,
            "resource",
            "TASKSPACE_RESOURCE_FAILURE",
            "resource_failed",
            message,
        ),
    }
}

fn read_argument_failure(
    canonical_revision: Option<u64>,
    code: &'static str,
    message: impl Into<String>,
) -> FunctionCallError {
    action_failure_error(
        "read_output_ref",
        None,
        canonical_revision,
        "argument",
        code,
        "argument_failed",
        message.into(),
    )
}

fn map_projection_failure(
    canonical_revision: Option<u64>,
    message: &'static str,
) -> FunctionCallError {
    action_failure_error(
        "read_map",
        None,
        canonical_revision,
        "protocol",
        "TASKSPACE_PROTOCOL_FAILURE",
        "protocol_failed",
        message.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure_json(error: FunctionCallError) -> serde_json::Value {
        let FunctionCallError::RespondToModel(payload) = error else {
            panic!("expected model-visible failure");
        };
        serde_json::from_str(&payload).expect("typed JSON failure")
    }

    #[test]
    fn malformed_output_ref_is_an_argument_failure() {
        let value = failure_json(output_read_error(
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "output_ref must use output-ref://sha256/<sha256>",
            ),
            Some(4),
        ));

        assert_eq!(value["action"], "read_output_ref");
        assert_eq!(value["status"], "argument_failed");
        assert_eq!(value["error"]["code"], "TASKSPACE_INVALID_ARGUMENT");
        assert_eq!(value["canonical_revision"], 4);
    }

    #[test]
    fn invalid_line_range_is_distinct_from_a_missing_artifact() {
        let range = failure_json(output_read_error(
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "line_range requires 1 <= start_line <= end_line",
            ),
            Some(4),
        ));
        let missing = failure_json(output_read_error(
            std::io::Error::new(std::io::ErrorKind::NotFound, "artifact does not exist"),
            Some(4),
        ));

        assert_eq!(range["error"]["code"], "TASKSPACE_RANGE_INVALID");
        assert_eq!(missing["error"]["code"], "TASKSPACE_OUTPUT_REF_NOT_FOUND");
    }
}
