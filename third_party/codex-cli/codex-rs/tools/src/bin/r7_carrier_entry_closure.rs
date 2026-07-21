use codex_protocol::config_types::WebSearchMode;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::WebSearchToolType;
use codex_tools::ToolsConfig;
use codex_tools::UnifiedExecShellMode;
use codex_tools::WebSearchToolManifest;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
#[path = "r7_carrier_entry_closure/entry.rs"]
mod entry;
#[path = "r7_carrier_entry_closure/sources.rs"]
mod sources;
use entry::build_entries;
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

fn main() -> Result<(), String> {
    let (repo_root, output, check) = parse_args()?;
    let index = index_sources(&repo_root)?;
    assert_variants(&index, "ToolSpec", TOOL_SPEC_VARIANTS)?;
    assert_variants(&index, "ToolPayload", TOOL_PAYLOAD_VARIANTS)?;
    assert_variants(&index, "ToolHandlerKind", TOOL_HANDLER_VARIANTS)?;
    let bindings = pipeline_bindings(&repo_root, &index)?;
    let scanned_sources = index.all_source_hashes();
    let inventory_sha256 = canonical_hash(&(&bindings, &scanned_sources))?;
    let source_inventory = SourceInventory {
        roots: vec![
            "third_party/codex-cli/codex-rs/tools/src".into(),
            "third_party/codex-cli/codex-rs/core/src".into(),
            "third_party/codex-cli/codex-rs/codex-api/src".into(),
        ],
        tool_spec_variants: index.enum_variants("ToolSpec")?,
        tool_payload_variants: index.enum_variants("ToolPayload")?,
        tool_handler_variants: index.enum_variants("ToolHandlerKind")?,
        bindings,
        scanned_sources,
        inventory_sha256,
    };
    let mut entries = build_entries(&source_inventory)?;
    entries.sort_by_key(entry_key);
    assert_unique_entry_keys(&entries)?;
    assert_matrix_coverage(&entries)?;
    assert_handler_gate(&source_inventory, &entries)?;
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

fn dynamic_fixtures() -> [DynamicToolSpec; 3] {
    [
        DynamicToolSpec {
            namespace: None,
            name: "dynamic_plain".into(),
            description: "fixture".into(),
            input_schema: serde_json::json!({"type":"object"}),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("dynamic".into()),
            name: "lookup".into(),
            description: "fixture".into(),
            input_schema: serde_json::json!({"type":"object"}),
            defer_loading: false,
        },
        DynamicToolSpec {
            namespace: Some("deferred_dynamic".into()),
            name: "lookup".into(),
            description: "fixture".into(),
            input_schema: serde_json::json!({"type":"object"}),
            defer_loading: true,
        },
    ]
}

fn entry_key(entry: &Entry) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        entry.profile_id, entry.wire_api, entry.invocation_origin, entry.tool_spec, entry.tool_name
    )
}

fn assert_unique_entry_keys(entries: &[Entry]) -> Result<(), String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        *counts.entry(entry_key(entry)).or_default() += 1;
    }
    let duplicates = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(key, count)| format!("{key} x{count}"))
        .collect::<Vec<_>>();
    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(format!("duplicate entry keys: {}", duplicates.join(", ")))
    }
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

fn assert_handler_gate(inventory: &SourceInventory, entries: &[Entry]) -> Result<(), String> {
    let inventory_handlers = inventory
        .tool_handler_variants
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let entry_handlers = entries
        .iter()
        .filter_map(|entry| entry.handler_kind.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if inventory_handlers == entry_handlers {
        Ok(())
    } else {
        Err(format!(
            "handler closure mismatch: inventory_only={:?} entry_only={:?}",
            inventory_handlers
                .difference(&entry_handlers)
                .collect::<Vec<_>>(),
            entry_handlers
                .difference(&inventory_handlers)
                .collect::<Vec<_>>()
        ))
    }
}
