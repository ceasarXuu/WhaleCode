use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifestEntry {
    pub(crate) call_id: String,
    pub(crate) tool_name: String,
    pub(crate) is_apply_patch: bool,
    pub(crate) is_taskspace_control: bool,
    pub(crate) payload_kind: &'static str,
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
                tool_name: call.provider_tool_name_display(),
                is_apply_patch: call.provider_tool_name.namespace.is_none()
                    && call.provider_tool_name.name == "apply_patch",
                is_taskspace_control: is_taskspace_control(call),
                payload_kind: payload_kind(&call.payload),
            })
            .collect::<Vec<_>>();
        let request_patch_count = entries.iter().filter(|entry| entry.is_apply_patch).count();
        Self {
            entries,
            request_patch_count,
        }
    }
}

pub(crate) fn is_taskspace_control(call: &ToolCall) -> bool {
    call.provider_tool_name.namespace.is_none()
        && call.provider_tool_name.name == "taskspace_control"
}

fn payload_kind(payload: &ToolPayload) -> &'static str {
    match payload {
        ToolPayload::Function { .. } => "function",
        ToolPayload::ToolSearch { .. } => "tool_search",
        ToolPayload::Custom { .. } => "custom",
        ToolPayload::LocalShell { .. } => "local_shell",
        ToolPayload::Mcp { .. } => "mcp",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tools::ToolName;

    fn call(name: &str, call_id: &str) -> ToolCall {
        ToolCall {
            provider_tool_name: ToolName::plain(name),
            dispatch_tool_name: ToolName::plain(name),
            call_id: call_id.into(),
            payload: ToolPayload::Function {
                arguments: "{}".into(),
            },
        }
    }

    #[test]
    fn describes_native_calls_without_taskspace_tool_decoration() {
        let manifest = ToolSequenceManifest::from_calls(&[
            call("taskspace_control", "control"),
            call("apply_patch", "patch"),
        ]);
        assert_eq!(manifest.request_patch_count, 1);
        assert!(manifest.entries[0].is_taskspace_control);
        assert_eq!(manifest.entries[1].tool_name, "apply_patch");
    }
}
