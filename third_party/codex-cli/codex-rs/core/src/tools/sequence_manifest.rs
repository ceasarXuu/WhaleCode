use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceContinuation;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::router::ToolCall;
use serde::Deserialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifestEntry {
    pub(crate) call_id: String,
    pub(crate) tool_name: String,
    pub(crate) is_apply_patch: bool,
    pub(crate) is_taskspace_control: bool,
    pub(crate) apply_patch_arguments_valid: bool,
    pub(crate) continuation_requirement: Option<TaskSpaceContinuation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifest {
    pub(crate) entries: Vec<ToolSequenceManifestEntry>,
    pub(crate) request_patch_count: usize,
}

impl ToolSequenceManifest {
    pub(crate) fn from_calls(calls: &[ToolCall]) -> Self {
        let entries = calls.iter().map(top_level_entry).collect::<Vec<_>>();
        let request_patch_count = entries.iter().filter(|entry| entry.is_apply_patch).count();
        Self {
            entries,
            request_patch_count,
        }
    }
}

fn top_level_entry(call: &ToolCall) -> ToolSequenceManifestEntry {
    let is_apply_patch = is_plain_tool(call, "apply_patch");
    let is_taskspace_control = is_plain_tool(call, "taskspace_control");
    let continuation_requirement = is_taskspace_control
        .then(|| taskspace_control_arguments(call))
        .flatten()
        .and_then(|arguments| parse_taskspace_control_args(arguments).ok())
        .and_then(|arguments| arguments.continuation_requirement());
    ToolSequenceManifestEntry {
        call_id: call.call_id.clone(),
        tool_name: call.tool_name.display(),
        is_apply_patch,
        is_taskspace_control,
        apply_patch_arguments_valid: !is_apply_patch || apply_patch_arguments_valid(call),
        continuation_requirement,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyPatchArguments {
    input: String,
}

fn apply_patch_arguments_valid(call: &ToolCall) -> bool {
    match &call.payload {
        ToolPayload::Function { arguments } => {
            serde_json::from_str::<ApplyPatchArguments>(arguments)
                .is_ok_and(|parsed| !parsed.input.is_empty())
        }
        ToolPayload::Custom { input } => !input.is_empty(),
        _ => false,
    }
}

fn is_plain_tool(call: &ToolCall, name: &str) -> bool {
    call.tool_name.namespace.is_none() && call.tool_name.name == name
}

fn taskspace_control_arguments(call: &ToolCall) -> Option<&str> {
    let ToolPayload::Function { arguments } = &call.payload else {
        return None;
    };
    Some(arguments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tools::ToolName;

    fn call(name: &str, call_id: &str, arguments: &str) -> ToolCall {
        ToolCall {
            tool_name: ToolName::plain(name),
            call_id: call_id.into(),
            payload: ToolPayload::Function {
                arguments: arguments.into(),
            },
        }
    }

    #[test]
    fn records_top_level_patch_and_control_continuation() {
        let bootstrap = call(
            "taskspace_control",
            "bootstrap",
            r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Solve"},"initial_work_node":{"node_id":"edit","goal":"Edit"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[{"from":"root","to":"edit"},{"from":"edit","to":"finish"}],"continuation":"next_apply_patch"}"#,
        );
        let manifest = ToolSequenceManifest::from_calls(&[
            bootstrap,
            call("apply_patch", "top-patch", r#"{"input":"patch"}"#),
        ]);

        assert_eq!(manifest.request_patch_count, 1);
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(
            manifest.entries[0].continuation_requirement,
            Some(TaskSpaceContinuation::NextApplyPatch)
        );
        assert_eq!(manifest.entries[1].tool_name, "apply_patch");
        assert!(manifest.entries[1].apply_patch_arguments_valid);
    }

    #[test]
    fn leaves_unparseable_taskspace_arguments_to_the_tool_handler() {
        let manifest = ToolSequenceManifest::from_calls(&[call(
            "taskspace_control",
            "bad-control",
            r#"{"action":"initialize_map"}"#,
        )]);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.request_patch_count, 0);
        assert_eq!(manifest.entries[0].continuation_requirement, None);
    }

    #[test]
    fn marks_malformed_direct_patch_arguments_without_rewriting_them() {
        let manifest = ToolSequenceManifest::from_calls(&[call(
            "apply_patch",
            "bad-patch",
            r#"{"input":"patch"}}"#,
        )]);
        assert_eq!(manifest.request_patch_count, 1);
        assert!(!manifest.entries[0].apply_patch_arguments_valid);
    }
}
