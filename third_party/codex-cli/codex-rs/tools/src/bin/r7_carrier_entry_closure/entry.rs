use super::Entry;
use super::SourceInventory;
use codex_tools::ToolHandlerKind;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

pub fn spec_shape(
    spec: &ToolSpec,
    kind: Option<ToolHandlerKind>,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
) {
    match spec {
        ToolSpec::Function(tool) if tool.name == "taskspace_control" => (
            "Function",
            "Function",
            "function",
            "non_carrier",
            "state_tool_excluded",
            true,
        ),
        ToolSpec::Function(tool) if tool.name == "exec" => (
            "Function",
            "Function",
            "code_exec",
            "projected_carrier",
            "code_mode_outer",
            true,
        ),
        ToolSpec::Function(_) => (
            "Function",
            kind.map(payload_for_handler).unwrap_or("Function"),
            "function",
            "carrier",
            "shared_function_handler",
            true,
        ),
        ToolSpec::ToolSearch { .. } => (
            "ToolSearch",
            "ToolSearch",
            "tool_search",
            "non_carrier",
            "provider_native",
            true,
        ),
        ToolSpec::LocalShell {} => (
            "LocalShell",
            "LocalShell",
            "local_shell",
            "non_carrier",
            "provider_native",
            true,
        ),
        ToolSpec::ImageGeneration { .. } => (
            "ImageGeneration",
            "NotApplicable",
            "image_generation",
            "non_carrier",
            "provider_native",
            true,
        ),
        ToolSpec::WebSearch { .. } => (
            "WebSearch",
            "NotApplicable",
            "web_search",
            "non_carrier",
            "provider_native",
            true,
        ),
        ToolSpec::Freeform(_) => (
            "Freeform",
            "Custom",
            "apply_patch",
            "projected_carrier",
            "freeform_projection",
            true,
        ),
        ToolSpec::Namespace(_) => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_entry(
    profile: &str,
    wire: &str,
    name: &str,
    namespace: Option<String>,
    spec: &str,
    payload: &str,
    kind: Option<ToolHandlerKind>,
    source: &str,
    origin: &str,
    route: &str,
    disposition: &str,
    reason: &str,
    visible: bool,
    parallel: bool,
    inventory: &SourceInventory,
) -> Entry {
    let mut pipeline = BTreeMap::new();
    for role in [
        "registry_plan",
        "core_registration",
        "invocation_parser",
        "alias_router",
        "approval",
        "executor",
        "output_mapper",
        "responses_mapper",
    ] {
        pipeline.insert(role.into(), inventory.bindings[role].clone());
    }
    let handler_binding = kind
        .map(|value| inventory.bindings[&format!("handler::{value:?}")].clone())
        .unwrap_or_else(|| inventory.bindings["core_registration"].clone());
    pipeline.insert("handler".into(), handler_binding);
    Entry {
        profile_id: profile.into(),
        wire_api: wire.into(),
        tool_name: name.into(),
        namespace,
        tool_spec: spec.into(),
        tool_payload: payload.into(),
        handler_kind: kind.map(|value| format!("{value:?}")),
        registration_source: source.into(),
        invocation_origin: origin.into(),
        route: route.into(),
        disposition: disposition.into(),
        reason_code: reason.into(),
        model_visible: visible,
        supports_parallel: parallel,
        pipeline,
    }
}

pub fn payload_for_handler(kind: ToolHandlerKind) -> &'static str {
    match kind {
        ToolHandlerKind::Mcp => "Mcp",
        ToolHandlerKind::ApplyPatch => "Custom",
        ToolHandlerKind::ToolSearch => "ToolSearch",
        _ => "Function",
    }
}

pub fn source_for_handler(kind: ToolHandlerKind) -> &'static str {
    match kind {
        ToolHandlerKind::Mcp => "mcp",
        ToolHandlerKind::DynamicTool => "dynamic",
        _ => "builtin",
    }
}
