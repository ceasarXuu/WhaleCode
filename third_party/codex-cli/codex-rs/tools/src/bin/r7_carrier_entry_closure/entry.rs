use super::Entry;
use super::SourceInventory;
use super::profiles::Profile;
use super::profiles::production_profiles;
use codex_tools::DiscoverablePluginInfo;
use codex_tools::DiscoverableTool;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolHandlerKind;
use codex_tools::ToolName;
use codex_tools::ToolRegistryPlanDeferredTool;
use codex_tools::ToolRegistryPlanMcpTool;
use codex_tools::ToolRegistryPlanParams;
use codex_tools::ToolSpec;
use codex_tools::WaitAgentTimeoutOptions;
use codex_tools::build_tool_registry_plan;
use codex_tools::create_tools_json_for_responses_api;
use std::collections::BTreeMap;

pub fn build_entries(inventory: &SourceInventory) -> Result<Vec<Entry>, String> {
    let mcp = rmcp::model::Tool {
        name: "lookup".to_string().into(),
        title: None,
        description: Some("closure fixture".to_string().into()),
        input_schema: std::sync::Arc::new(rmcp::model::object(
            serde_json::json!({"type":"object","properties":{}}),
        )),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: None,
        meta: None,
    };
    let mcp_tools = [ToolRegistryPlanMcpTool {
        name: ToolName::new(Some("fixture".into()), "lookup"),
        tool: &mcp,
    }];
    let deferred = [ToolRegistryPlanDeferredTool {
        name: ToolName::new(Some("deferred".into()), "lookup"),
        server_name: "deferred",
        connector_name: None,
        connector_description: None,
    }];
    let dynamic = super::dynamic_fixtures();
    let discoverable = discoverable_fixtures();
    let namespaces = BTreeMap::from([(
        "fixture".to_string(),
        codex_tools::ToolNamespace {
            name: "fixture".into(),
            description: None,
        },
    )])
    .into_iter()
    .collect();
    let mut entries = Vec::new();
    for profile in production_profiles() {
        let profile_start = entries.len();
        let plan = build_tool_registry_plan(
            &profile.config,
            ToolRegistryPlanParams {
                mcp_tools: Some(&mcp_tools),
                deferred_mcp_tools: Some(&deferred),
                tool_namespaces: Some(&namespaces),
                discoverable_tools: Some(&discoverable),
                dynamic_tools: &dynamic,
                default_agent_type_description: "closure fixture",
                wait_agent_timeouts: WaitAgentTimeoutOptions {
                    default_timeout_ms: 30_000,
                    min_timeout_ms: 1_000,
                    max_timeout_ms: 300_000,
                },
            },
        );
        let handlers = plan
            .handlers
            .iter()
            .map(|item| (item.name.display(), item.kind))
            .collect::<BTreeMap<_, _>>();
        let response_specs = plan
            .specs
            .iter()
            .map(|configured| configured.spec.clone())
            .collect::<Vec<_>>();
        for configured in &plan.specs {
            add_spec_entries(
                &mut entries,
                profile.id,
                profile.nested,
                &configured.spec,
                configured.supports_parallel_tool_calls,
                &handlers,
                inventory,
            )?;
        }
        add_handler_only_entries(&mut entries, &profile, plan.handlers, inventory);
        let response_tools = create_tools_json_for_responses_api(&response_specs)
            .map_err(|error| error.to_string())?;
        let mapped_names = super::provider::deepseek_function_names(response_tools)?;
        let response_entries = entries[profile_start..].to_vec();
        add_deepseek_entries(&mut entries, response_entries, mapped_names, inventory)?;
    }
    Ok(entries)
}

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

fn discoverable_fixtures() -> [DiscoverableTool; 1] {
    [DiscoverableTool::Plugin(Box::new(DiscoverablePluginInfo {
        id: "fixture-plugin".into(),
        name: "Fixture Plugin".into(),
        description: Some("closure fixture".into()),
        has_skills: true,
        mcp_server_names: vec!["fixture".into()],
        app_connector_ids: Vec::new(),
    }))]
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
    for role in base_pipeline_roles(origin, spec, kind) {
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

fn add_handler_only_entries(
    entries: &mut Vec<Entry>,
    profile: &Profile,
    handlers: Vec<codex_tools::ToolHandlerSpec>,
    inventory: &SourceInventory,
) {
    for handler in handlers {
        if entries.iter().any(|entry| {
            entry.profile_id == profile.id && entry.tool_name == handler.name.display()
        }) {
            continue;
        }
        entries.push(make_entry(
            profile.id,
            "responses",
            &handler.name.display(),
            handler.name.namespace.clone(),
            "Function",
            payload_for_handler(handler.kind),
            Some(handler.kind),
            source_for_handler(handler.kind),
            if profile.nested {
                "code_mode"
            } else {
                "direct"
            },
            "function",
            "carrier",
            "deferred_registry",
            false,
            false,
            inventory,
        ));
    }
}

fn add_deepseek_entries(
    entries: &mut Vec<Entry>,
    response_entries: Vec<Entry>,
    mapped_names: std::collections::BTreeSet<String>,
    inventory: &SourceInventory,
) -> Result<(), String> {
    let response_names = response_entries
        .iter()
        .map(|entry| entry.tool_name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let missing = mapped_names
        .iter()
        .filter(|name| !response_names.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "DeepSeek mapper produced tools absent from closure: {missing:?}"
        ));
    }
    for mut entry in response_entries {
        entry.wire_api = "deepseek_chat".into();
        if !entry.model_visible || !mapped_names.contains(&entry.tool_name) {
            entry.model_visible = false;
            entry.disposition = "non_carrier".into();
            entry.reason_code = "provider_wire_unsupported".into();
        } else if entry.tool_spec == "WebSearch"
            || (entry.tool_spec == "Freeform" && entry.tool_name == "apply_patch")
        {
            entry.disposition = "projected_carrier".into();
            entry.reason_code = "deepseek_projected_function".into();
        }
        entry.pipeline.insert(
            "provider_mapper".into(),
            inventory.bindings["deepseek_mapper"].clone(),
        );
        entries.push(entry);
    }
    Ok(())
}

fn add_spec_entries(
    output: &mut Vec<Entry>,
    profile: &str,
    nested: bool,
    spec: &ToolSpec,
    parallel: bool,
    handlers: &BTreeMap<String, ToolHandlerKind>,
    inventory: &SourceInventory,
) -> Result<(), String> {
    let origin = if nested { "code_mode" } else { "direct" };
    match spec {
        ToolSpec::Namespace(namespace) => {
            output.push(make_entry(
                profile,
                "responses",
                &namespace.name,
                None,
                "Namespace",
                "NotApplicable",
                None,
                "mcp",
                origin,
                "namespace",
                "container",
                "namespace_container",
                true,
                parallel,
                inventory,
            ));
            for tool in &namespace.tools {
                let ResponsesApiNamespaceTool::Function(tool) = tool;
                let name = ToolName::new(Some(namespace.name.clone()), tool.name.clone());
                let kind = handlers.get(&name.display()).copied();
                output.push(make_entry(
                    profile,
                    "responses",
                    &name.display(),
                    name.namespace,
                    "Function",
                    kind.map(payload_for_handler).unwrap_or("Function"),
                    kind,
                    kind.map(source_for_handler).unwrap_or("dynamic"),
                    origin,
                    "namespace",
                    "carrier",
                    "decorated_namespace_function",
                    true,
                    parallel,
                    inventory,
                ));
            }
        }
        _ => {
            let name = spec.name().to_string();
            let kind = handlers.get(&name).copied();
            let (variant, payload, route, disposition, reason, visible) = spec_shape(spec, kind);
            output.push(make_entry(
                profile,
                "responses",
                &name,
                None,
                variant,
                payload,
                kind,
                kind.map(source_for_handler).unwrap_or("builtin"),
                origin,
                route,
                disposition,
                reason,
                visible,
                parallel,
                inventory,
            ));
        }
    }
    Ok(())
}

fn base_pipeline_roles(
    origin: &str,
    spec: &str,
    kind: Option<ToolHandlerKind>,
) -> Vec<&'static str> {
    let mut roles = vec![
        "registry_plan",
        "tools_config",
        "model_profile",
        "core_registration",
        "invocation_parser",
        "alias_router",
        "approval",
        "executor",
        "output_mapper",
        "responses_mapper",
    ];
    if origin == "code_mode" {
        roles.push("code_mode_decorator");
        roles.push("nested_tools_config");
    }
    if kind == Some(ToolHandlerKind::DynamicTool) {
        roles.push("dynamic_registry");
    }
    if kind == Some(ToolHandlerKind::Mcp) || spec == "Namespace" {
        roles.push("mcp_registry");
    }
    roles
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
