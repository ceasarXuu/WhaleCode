use codex_extension_api::ToolBatchPreflightFailure;
use codex_extension_api::ToolBatchPreflightInput;
use codex_extension_api::ToolPayload;

use crate::runtime::TaskSpaceRuntimeHandle;
use crate::tool::TASKSPACE_CONTROL_TOOL;

pub(crate) async fn validate(
    input: &ToolBatchPreflightInput<'_>,
) -> Result<(), ToolBatchPreflightFailure> {
    let Some(runtime) = input.thread_store.get::<TaskSpaceRuntimeHandle>() else {
        return Ok(());
    };
    if !runtime.is_enabled() {
        return Ok(());
    }

    let control_indices = input
        .calls
        .iter()
        .enumerate()
        .filter_map(|(index, call)| {
            (call.tool_name.is_default_namespace() && call.tool_name.name == TASKSPACE_CONTROL_TOOL)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    let control_index = match control_indices.as_slice() {
        [] => return Ok(()),
        [index] => *index,
        _ => {
            return Err(ToolBatchPreflightFailure::new(
                "taskspace_control_multiple",
                "a TaskSpace response may contain only one taskspace_control call",
            ));
        }
    };
    if control_index != 0 {
        return Err(ToolBatchPreflightFailure::new(
            "taskspace_control_must_be_first",
            "taskspace_control must be the first tool call in its response",
        ));
    }

    let ToolPayload::Function { arguments } = &input.calls[control_index].payload else {
        return Err(ToolBatchPreflightFailure::new(
            "taskspace_control_payload_invalid",
            "taskspace_control requires function arguments",
        ));
    };
    let value: serde_json::Value = serde_json::from_str(arguments).map_err(|error| {
        ToolBatchPreflightFailure::new(
            "taskspace_control_arguments_invalid",
            format!("invalid TaskSpace control arguments: {error}"),
        )
    })?;
    match value.get("action").and_then(serde_json::Value::as_str) {
        Some("read_map") if input.calls.len() != 1 => Err(ToolBatchPreflightFailure::new(
            "taskspace_read_map_has_siblings",
            "taskspace_control action=read_map must not include sibling tool calls",
        )),
        Some("read_map") => Ok(()),
        Some("initialize_and_execute") => {
            if runtime.is_active() {
                return Err(ToolBatchPreflightFailure::new(
                    "taskspace_already_initialized",
                    "TaskSpace initialize_and_execute requires an unbound thread",
                ));
            }
            let map_id = format!("map-{}", runtime.thread_id());
            let (commit, actions) = crate::initialize_manifest::prepare(
                &map_id,
                &input.calls[0].call_id,
                arguments,
                &input.calls[1..],
            )
            .map_err(|error| {
                ToolBatchPreflightFailure::new(
                    "taskspace_initialize_rejected",
                    format!("invalid TaskSpace initialization: {error}"),
                )
            })?;
            runtime
                .commit_initialization(
                    &input.calls[0].call_id,
                    input.turn_store.level_id(),
                    commit,
                    actions,
                )
                .await
                .map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_state_commit_failed",
                        format!("failed to commit TaskSpace initialization: {error}"),
                    )
                })?;
            Ok(())
        }
        Some("execute") => {
            let record = runtime.record().await.ok_or_else(|| {
                ToolBatchPreflightFailure::new(
                    "taskspace_map_missing",
                    "TaskSpace execute requires an active canonical Map",
                )
            })?;
            let (transaction, actions) = crate::execute_manifest::prepare(
                &record.map.map_id,
                &input.calls[0].call_id,
                arguments,
                &input.calls[1..],
            )
            .map_err(|error| {
                ToolBatchPreflightFailure::new(
                    "taskspace_execute_manifest_invalid",
                    format!("invalid TaskSpace execute manifest: {error}"),
                )
            })?;
            let commit =
                crate::transactions::execute(&record.map, transaction).map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_execute_rejected",
                        format!("TaskSpace execute rejected: {error:?}"),
                    )
                })?;
            runtime
                .commit_control(
                    &input.calls[0].call_id,
                    input.turn_store.level_id(),
                    "execute",
                    "execute_prepare",
                    commit,
                    actions,
                )
                .await
                .map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_state_commit_failed",
                        format!("failed to commit TaskSpace execute: {error}"),
                    )
                })?;
            Ok(())
        }
        Some("finish_map") if input.calls.len() != 1 => Err(ToolBatchPreflightFailure::new(
            "taskspace_finish_map_has_siblings",
            "taskspace_control action=finish_map must not include sibling tool calls",
        )),
        Some("finish_map") => {
            let record = runtime.record().await.ok_or_else(|| {
                ToolBatchPreflightFailure::new(
                    "taskspace_map_missing",
                    "TaskSpace finish_map requires an active canonical Map",
                )
            })?;
            let transaction =
                crate::lifecycle_manifest::prepare_finish(&input.calls[0].call_id, arguments)
                    .map_err(|error| {
                        ToolBatchPreflightFailure::new(
                            "taskspace_finish_manifest_invalid",
                            format!("invalid TaskSpace finish manifest: {error}"),
                        )
                    })?;
            let commit =
                crate::transactions::finish_map(&record.map, transaction).map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_finish_rejected",
                        format!("TaskSpace finish rejected: {error:?}"),
                    )
                })?;
            runtime
                .commit_control(
                    &input.calls[0].call_id,
                    input.turn_store.level_id(),
                    "finish_map",
                    "finish_map",
                    commit,
                    Vec::new(),
                )
                .await
                .map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_state_commit_failed",
                        format!("failed to commit TaskSpace finish: {error}"),
                    )
                })?;
            Ok(())
        }
        Some("reopen_map") => {
            let record = runtime.record().await.ok_or_else(|| {
                ToolBatchPreflightFailure::new(
                    "taskspace_map_missing",
                    "TaskSpace reopen_map requires an active canonical Map",
                )
            })?;
            let (transaction, actions) = crate::lifecycle_manifest::prepare_reopen(
                &record.map.map_id,
                &input.calls[0].call_id,
                arguments,
                &input.calls[1..],
            )
            .map_err(|error| {
                ToolBatchPreflightFailure::new(
                    "taskspace_reopen_manifest_invalid",
                    format!("invalid TaskSpace reopen manifest: {error}"),
                )
            })?;
            let commit =
                crate::transactions::reopen_map(&record.map, transaction).map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_reopen_rejected",
                        format!("TaskSpace reopen rejected: {error:?}"),
                    )
                })?;
            runtime
                .commit_control(
                    &input.calls[0].call_id,
                    input.turn_store.level_id(),
                    "reopen_map",
                    "reopen_map_prepare",
                    commit,
                    actions,
                )
                .await
                .map_err(|error| {
                    ToolBatchPreflightFailure::new(
                        "taskspace_state_commit_failed",
                        format!("failed to commit TaskSpace reopen: {error}"),
                    )
                })?;
            Ok(())
        }
        _ => Err(ToolBatchPreflightFailure::new(
            "taskspace_action_not_available",
            "TaskSpace control action is not available in this runtime",
        )),
    }
}
