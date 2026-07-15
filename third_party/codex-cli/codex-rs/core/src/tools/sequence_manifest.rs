use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNestedAction;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::router::ToolCall;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifestEntry {
    pub(crate) call_id: String,
    pub(crate) parent_call_id: Option<String>,
    pub(crate) tool_name: String,
    pub(crate) is_apply_patch: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolSequenceManifest {
    pub(crate) entries: Vec<ToolSequenceManifestEntry>,
    pub(crate) request_patch_count: usize,
}

impl ToolSequenceManifest {
    pub(crate) fn from_calls(calls: &[ToolCall]) -> Result<Self, String> {
        let mut entries = Vec::new();
        for call in calls {
            entries.push(top_level_entry(call));
            if !is_taskspace_control(call) {
                continue;
            }
            let arguments = taskspace_control_arguments(call)?;
            let args = parse_taskspace_control_args(arguments).map_err(|error| {
                format!(
                    "taskspace_control call `{}` is invalid: {error}",
                    call.call_id
                )
            })?;
            entries.extend(
                args.nested_actions()
                    .into_iter()
                    .enumerate()
                    .map(|(index, action)| nested_entry(call, index, &action)),
            );
        }
        let request_patch_count = entries.iter().filter(|entry| entry.is_apply_patch).count();
        Ok(Self {
            entries,
            request_patch_count,
        })
    }
}

fn top_level_entry(call: &ToolCall) -> ToolSequenceManifestEntry {
    ToolSequenceManifestEntry {
        call_id: call.call_id.clone(),
        parent_call_id: None,
        tool_name: call.tool_name.display(),
        is_apply_patch: call.tool_name.namespace.is_none() && call.tool_name.name == "apply_patch",
    }
}

fn nested_entry(
    parent: &ToolCall,
    index: usize,
    action: &TaskSpaceNestedAction,
) -> ToolSequenceManifestEntry {
    let tool_name = action.namespace().map_or_else(
        || action.tool_name().to_string(),
        |namespace| format!("{namespace}.{}", action.tool_name()),
    );
    ToolSequenceManifestEntry {
        call_id: format!("{}:nested:{index}", parent.call_id),
        parent_call_id: Some(parent.call_id.clone()),
        is_apply_patch: action.namespace().is_none() && action.tool_name() == "apply_patch",
        tool_name,
    }
}

fn is_taskspace_control(call: &ToolCall) -> bool {
    call.tool_name.namespace.is_none() && call.tool_name.name == "taskspace_control"
}

fn taskspace_control_arguments(call: &ToolCall) -> Result<&str, String> {
    let ToolPayload::Function { arguments } = &call.payload else {
        return Err(format!(
            "taskspace_control call `{}` requires function arguments",
            call.call_id
        ));
    };
    Ok(arguments)
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
    fn counts_top_level_and_declared_nested_patch_identities() {
        let bootstrap = call(
            "taskspace_control",
            "bootstrap",
            r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Solve"},"current_work_node":{"node_id":"edit","goal":"Edit"},"work_nodes":[],"finish":{"node_id":"finish","goal":"Summarize"},"edges":[{"from":"root","to":"edit"},{"from":"edit","to":"finish"}],"continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch"},"actions":[{"tool_name":"exec_command","arguments":{"cmd":"test"}}]}}"#,
        );
        let manifest =
            ToolSequenceManifest::from_calls(&[bootstrap, call("apply_patch", "top-patch", "{}")])
                .expect("manifest");

        assert_eq!(manifest.request_patch_count, 2);
        assert_eq!(manifest.entries.len(), 4);
        assert_eq!(
            manifest.entries[1].parent_call_id.as_deref(),
            Some("bootstrap")
        );
        assert_eq!(manifest.entries[1].tool_name, "apply_patch");
        assert_eq!(manifest.entries[2].tool_name, "exec_command");
    }

    #[test]
    fn rejects_an_unparseable_taskspace_manifest() {
        let error = ToolSequenceManifest::from_calls(&[call(
            "taskspace_control",
            "bad-control",
            r#"{"action":"initialize_map"}"#,
        )])
        .expect_err("invalid control args");
        assert!(error.contains("bad-control"));
        assert!(error.contains("invalid taskspace_control arguments"));
    }
}
