use crate::tools::router::ToolCall;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifestEntry {
    pub(crate) call_id: String,
    pub(crate) tool_name: String,
    pub(crate) is_apply_patch: bool,
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
            })
            .collect::<Vec<_>>();
        let request_patch_count = entries.iter().filter(|entry| entry.is_apply_patch).count();
        Self {
            entries,
            request_patch_count,
        }
    }
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
            taskspace_action: None,
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
        assert_eq!(manifest.entries[1].tool_name, "apply_patch");
    }
}
