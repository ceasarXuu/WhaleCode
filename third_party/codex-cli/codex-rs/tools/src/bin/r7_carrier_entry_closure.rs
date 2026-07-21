use codex_protocol::config_types::WebSearchMode;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::WebSearchToolType;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolHandlerKind;
use codex_tools::ToolName;
use codex_tools::ToolRegistryPlanDeferredTool;
use codex_tools::ToolRegistryPlanMcpTool;
use codex_tools::ToolRegistryPlanParams;
use codex_tools::ToolSpec;
use codex_tools::ToolsConfig;
use codex_tools::UnifiedExecShellMode;
use codex_tools::WaitAgentTimeoutOptions;
use codex_tools::WebSearchToolManifest;
use codex_tools::build_tool_registry_plan;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
#[path = "r7_carrier_entry_closure/entry.rs"]
mod entry;
#[path = "r7_carrier_entry_closure/sources.rs"]
mod sources;
use entry::make_entry;
use entry::payload_for_handler;
use entry::source_for_handler;
use entry::spec_shape;
use sources::SourceBinding;
use sources::TOOL_HANDLER_VARIANTS;
use sources::TOOL_PAYLOAD_VARIANTS;
use sources::TOOL_SPEC_VARIANTS;
use sources::assert_variants;
use sources::canonical_hash;
use sources::index_sources;
use sources::pipeline_bindings;

#[derive(Serialize, Clone)]
struct Entry {
    profile_id: String,
    wire_api: String,
    tool_name: String,
    namespace: Option<String>,
    tool_spec: String,
    tool_payload: String,
    handler_kind: Option<String>,
    registration_source: String,
    invocation_origin: String,
    route: String,
    disposition: String,
    reason_code: String,
    model_visible: bool,
    supports_parallel: bool,
    pipeline: BTreeMap<String, SourceBinding>,
}

#[derive(Serialize)]
struct Closure {
    schema_version: u8,
    artifact_role: &'static str,
    generator_version: u8,
    generated: bool,
    source_inventory: SourceInventory,
    generation_digest: String,
    entries: Vec<Entry>,
}

#[derive(Serialize)]
struct SourceInventory {
    roots: Vec<String>,
    tool_spec_variants: Vec<String>,
    tool_payload_variants: Vec<String>,
    tool_handler_variants: Vec<String>,
    bindings: BTreeMap<String, SourceBinding>,
    scanned_sources: BTreeMap<String, String>,
    inventory_sha256: String,
}

struct Profile {
    id: &'static str,
    config: ToolsConfig,
    nested: bool,
}

fn main() -> Result<(), String> {
    let (repo_root, output, check) = parse_args()?;
    let index = index_sources(&repo_root)?;
    assert_variants(&index, "ToolSpec", TOOL_SPEC_VARIANTS)?;
    assert_variants(&index, "ToolPayload", TOOL_PAYLOAD_VARIANTS)?;
    assert_variants(&index, "ToolHandlerKind", TOOL_HANDLER_VARIANTS)?;
    let bindings = pipeline_bindings(&repo_root, &index)?;
    let scanned_sources = index.binding_source_hashes(&bindings);
    let inventory_sha256 = canonical_hash(&(&bindings, &scanned_sources))?;
    let source_inventory = SourceInventory {
        roots: vec![
            "third_party/codex-cli/codex-rs/tools/src".into(),
            "third_party/codex-cli/codex-rs/core/src".into(),
            "third_party/codex-cli/codex-rs/codex-api/src".into(),
        ],
        tool_spec_variants: TOOL_SPEC_VARIANTS.iter().map(ToString::to_string).collect(),
        tool_payload_variants: TOOL_PAYLOAD_VARIANTS
            .iter()
            .map(ToString::to_string)
            .collect(),
        tool_handler_variants: TOOL_HANDLER_VARIANTS
            .iter()
            .map(ToString::to_string)
            .collect(),
        bindings,
        scanned_sources,
        inventory_sha256,
    };
    let mut entries = build_entries(&source_inventory)?;
    entries.sort_by_key(entry_key);
    entries.dedup_by(|left, right| entry_key(left) == entry_key(right));
    assert_matrix_coverage(&entries)?;
    let generation_digest = canonical_hash(&(source_inventory.inventory_sha256.clone(), &entries))?;
    let closure = Closure {
        schema_version: 2,
        artifact_role: "entry_closure",
        generator_version: 1,
        generated: true,
        source_inventory,
        generation_digest,
        entries,
    };
    let bytes = serde_json::to_vec_pretty(&closure).map_err(|error| error.to_string())?;
    let mut rendered = bytes;
    rendered.push(b'\n');
    if check {
        let existing = fs::read(&output).map_err(|error| error.to_string())?;
        if existing != rendered {
            return Err(format!(
                "generated closure differs from {}",
                output.display()
            ));
        }
    } else {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&output, rendered).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf, bool), String> {
    let mut args = env::args().skip(1);
    let mut root = None;
    let mut output = None;
    let mut check = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo-root" => root = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            "--check" => check = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let root = root
        .ok_or("--repo-root is required")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let output = output.ok_or("--output is required")?;
    let output = if output.is_absolute() {
        output
    } else {
        root.join(output)
    };
    Ok((root, output, check))
}

fn base_config(
    shell_type: ConfigShellToolType,
    patch: Option<ApplyPatchToolType>,
    code: bool,
) -> ToolsConfig {
    ToolsConfig {
        available_models: Vec::new(),
        shell_type,
        shell_command_backend: codex_tools::ShellCommandBackendConfig::Classic,
        unified_exec_shell_mode: UnifiedExecShellMode::Direct,
        has_environment: true,
        allow_login_shell: true,
        apply_patch_tool_type: patch,
        web_search_mode: Some(WebSearchMode::Live),
        web_search_config: None,
        web_search_tool_manifest: WebSearchToolManifest::Generic,
        web_search_tool_type: WebSearchToolType::Text,
        image_gen_tool: true,
        search_tool: true,
        tool_suggest: false,
        exec_permission_approvals_enabled: true,
        request_permissions_tool_enabled: true,
        code_mode_enabled: code,
        code_mode_only_enabled: false,
        can_request_original_image_detail: true,
        collab_tools: true,
        goal_tools: true,
        multi_agent_v2: false,
        hide_spawn_agent_metadata: false,
        spawn_agent_usage_hint: true,
        spawn_agent_usage_hint_text: None,
        max_concurrent_threads_per_session: None,
        default_mode_request_user_input: true,
        experimental_supported_tools: vec!["list_dir".into(), "test_sync_tool".into()],
        agent_jobs_tools: true,
        agent_jobs_worker_tools: true,
        agent_type_description: String::new(),
    }
}

fn build_entries(inventory: &SourceInventory) -> Result<Vec<Entry>, String> {
    let profiles = [
        Profile {
            id: "function",
            config: base_config(
                ConfigShellToolType::UnifiedExec,
                Some(ApplyPatchToolType::Function),
                false,
            ),
            nested: false,
        },
        Profile {
            id: "freeform_code",
            config: base_config(
                ConfigShellToolType::Default,
                Some(ApplyPatchToolType::Freeform),
                true,
            ),
            nested: false,
        },
        Profile {
            id: "local_shell",
            config: base_config(ConfigShellToolType::Local, None, false),
            nested: false,
        },
        Profile {
            id: "code_nested",
            config: base_config(
                ConfigShellToolType::Default,
                Some(ApplyPatchToolType::Freeform),
                false,
            ),
            nested: true,
        },
    ];
    let mcp = rmcp::model::Tool {
        name: "lookup".to_string().into(),
        title: None,
        description: Some("closure fixture".to_string().into()),
        input_schema: std::sync::Arc::new(rmcp::model::object(
            json!({"type":"object","properties":{}}),
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
    let dynamic = [
        DynamicToolSpec {
            namespace: None,
            name: "dynamic_plain".into(),
            description: "fixture".into(),
            input_schema: json!({"type":"object"}),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("dynamic".into()),
            name: "lookup".into(),
            description: "fixture".into(),
            input_schema: json!({"type":"object"}),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("deferred_dynamic".into()),
            name: "lookup".into(),
            description: "fixture".into(),
            input_schema: json!({"type":"object"}),
            defer_loading: true,
        },
    ];
    let namespaces = BTreeMap::from([(
        "fixture".to_string(),
        codex_tools::ToolNamespace {
            name: "fixture".into(),
            description: None,
        },
    )]);
    let namespaces = namespaces.into_iter().collect();
    let mut entries = Vec::new();
    for profile in profiles {
        let plan = build_tool_registry_plan(
            &profile.config,
            ToolRegistryPlanParams {
                mcp_tools: Some(&mcp_tools),
                deferred_mcp_tools: Some(&deferred),
                tool_namespaces: Some(&namespaces),
                discoverable_tools: None,
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
        for configured in plan.specs {
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
        for handler in plan.handlers {
            if !entries.iter().any(|entry| {
                entry.profile_id == profile.id && entry.tool_name == handler.name.display()
            }) {
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
    }
    let response_entries = entries.clone();
    for mut entry in response_entries {
        entry.wire_api = "deepseek_chat".into();
        if entry.namespace.is_some()
            || matches!(
                entry.tool_spec.as_str(),
                "Namespace" | "ToolSearch" | "LocalShell" | "ImageGeneration"
            )
        {
            entry.model_visible = false;
            entry.disposition = "non_carrier".into();
            entry.reason_code = "provider_wire_unsupported".into();
        }
        entry.pipeline.insert(
            "provider_mapper".into(),
            inventory.bindings["deepseek_mapper"].clone(),
        );
        entries.push(entry);
    }
    Ok(entries)
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

fn entry_key(entry: &Entry) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        entry.profile_id, entry.wire_api, entry.invocation_origin, entry.tool_spec, entry.tool_name
    )
}

fn assert_matrix_coverage(entries: &[Entry]) -> Result<(), String> {
    for variant in TOOL_SPEC_VARIANTS {
        if !entries.iter().any(|entry| entry.tool_spec == *variant) {
            return Err(format!("ToolSpec matrix gap: {variant}"));
        }
    }
    for variant in TOOL_PAYLOAD_VARIANTS {
        if !entries.iter().any(|entry| entry.tool_payload == *variant) {
            return Err(format!("ToolPayload matrix gap: {variant}"));
        }
    }
    for wire in ["responses", "deepseek_chat"] {
        if !entries.iter().any(|entry| entry.wire_api == wire) {
            return Err(format!("wire matrix gap: {wire}"));
        }
    }
    for disposition in ["carrier", "projected_carrier", "non_carrier", "container"] {
        if !entries.iter().any(|entry| entry.disposition == disposition) {
            return Err(format!("disposition matrix gap: {disposition}"));
        }
    }
    Ok(())
}
