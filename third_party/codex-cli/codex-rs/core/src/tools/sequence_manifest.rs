use crate::tools::router::ToolCall;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifestEntry {
    pub(crate) call_id: String,
    pub(crate) tool_name: String,
    pub(crate) is_apply_patch: bool,
    pub(crate) is_taskspace_control: bool,
    pub(crate) taskspace_control_action: Option<String>,
    pub(crate) taskspace_binding: Option<String>,
    pub(crate) requires_taskspace_binding: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifest {
    pub(crate) entries: Vec<ToolSequenceManifestEntry>,
    pub(crate) request_patch_count: usize,
}

impl ToolSequenceManifest {
    pub(crate) fn from_calls(calls: &[ToolCall]) -> Self {
        let entries = calls
            .iter()
            .map(|call| ToolSequenceManifestEntry {
                call_id: call.call_id.clone(),
                tool_name: call.tool_name.display(),
                is_apply_patch: call.tool_name.namespace.is_none()
                    && call.tool_name.name == "apply_patch",
                is_taskspace_control: is_taskspace_control(call),
                taskspace_control_action: taskspace_control_action(call),
                taskspace_binding: call.taskspace_binding.clone(),
                requires_taskspace_binding: requires_taskspace_binding(call),
            })
            .collect::<Vec<_>>();
        let request_patch_count = entries.iter().filter(|entry| entry.is_apply_patch).count();
        Self {
            entries,
            request_patch_count,
        }
    }
}

pub(crate) fn is_boundary_action(action: Option<&str>) -> bool {
    matches!(
        action,
        Some("initialize_map" | "bind_node" | "complete_then_continue")
    )
}

fn is_taskspace_control(call: &ToolCall) -> bool {
    call.tool_name.namespace.is_none() && call.tool_name.name == "taskspace_control"
}

fn taskspace_control_action(call: &ToolCall) -> Option<String> {
    if !is_taskspace_control(call) {
        return None;
    }
    let crate::tools::context::ToolPayload::Function { arguments } = &call.payload else {
        return None;
    };
    serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|value| value.get("action")?.as_str().map(str::to_owned))
}

fn requires_taskspace_binding(call: &ToolCall) -> bool {
    if is_taskspace_control(call) {
        return false;
    }
    matches!(
        call.payload,
        crate::tools::context::ToolPayload::Function { .. }
            | crate::tools::context::ToolPayload::ToolSearch { .. }
            | crate::tools::context::ToolPayload::Mcp { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::context::ToolPayload;
    use codex_tools::ToolName;

    fn call(name: &str, call_id: &str) -> ToolCall {
        ToolCall {
            tool_name: ToolName::plain(name),
            call_id: call_id.into(),
            payload: ToolPayload::Function {
                arguments: "{}".into(),
            },
            taskspace_binding: None,
        }
    }

    #[test]
    fn counts_top_level_patch_calls_without_interpreting_control_semantics() {
        let manifest = ToolSequenceManifest::from_calls(&[
            call("taskspace_control", "control"),
            call("apply_patch", "patch"),
        ]);
        assert_eq!(manifest.request_patch_count, 1);
        assert_eq!(manifest.entries.len(), 2);
        assert!(manifest.entries[0].is_taskspace_control);
        assert_eq!(manifest.entries[1].tool_name, "apply_patch");
    }
}
