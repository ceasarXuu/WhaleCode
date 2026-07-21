use codex_features::Feature;
use codex_features::Features;
use codex_models_manager::model_info::model_info_from_slug;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::SessionSource;
use codex_tools::ToolsConfig;
use codex_tools::ToolsConfigParams;
use codex_tools::UnifiedExecShellMode;

pub struct Profile {
    pub id: &'static str,
    pub config: ToolsConfig,
    pub nested: bool,
}

pub fn production_profiles() -> Vec<Profile> {
    let mut profiles = vec![
        fallback_profile("deepseek_v4_pro_default", "deepseek-v4-pro"),
        fallback_profile("deepseek_v4_flash_default", "deepseek-v4-flash"),
    ];
    profiles.extend([
        remote_profile(
            "function_unified_exec",
            ConfigShellToolType::UnifiedExec,
            Some(ApplyPatchToolType::Function),
            &[
                Feature::RequestPermissionsTool,
                Feature::Goals,
                Feature::SpawnCsv,
            ],
            false,
        ),
        remote_profile(
            "freeform_code",
            ConfigShellToolType::Default,
            Some(ApplyPatchToolType::Freeform),
            &[Feature::ApplyPatchFreeform, Feature::CodeMode],
            false,
        ),
        remote_profile("local_shell", ConfigShellToolType::Local, None, &[], false),
        remote_profile(
            "code_nested",
            ConfigShellToolType::Default,
            Some(ApplyPatchToolType::Freeform),
            &[Feature::ApplyPatchFreeform, Feature::CodeMode],
            true,
        ),
        remote_profile(
            "multi_agent_v2",
            ConfigShellToolType::UnifiedExec,
            Some(ApplyPatchToolType::Function),
            &[Feature::MultiAgentV2],
            false,
        ),
    ]);
    profiles
}

fn fallback_profile(id: &'static str, slug: &str) -> Profile {
    build_profile(
        id,
        model_info_from_slug(slug),
        Features::with_defaults(),
        false,
    )
}

fn remote_profile(
    id: &'static str,
    shell_type: ConfigShellToolType,
    apply_patch_tool_type: Option<ApplyPatchToolType>,
    enabled: &[Feature],
    nested: bool,
) -> Profile {
    let mut model = model_info_from_slug(id);
    model.shell_type = shell_type;
    model.apply_patch_tool_type = apply_patch_tool_type;
    model.supports_parallel_tool_calls = true;
    model.supports_search_tool = true;
    model.experimental_supported_tools = vec!["list_dir".into(), "test_sync_tool".into()];
    model.used_fallback_model_metadata = false;
    let mut features = Features::with_defaults();
    for feature in enabled {
        features.enable(*feature);
    }
    if shell_type == ConfigShellToolType::Local {
        features.disable(Feature::UnifiedExec);
    }
    build_profile(id, model, features, nested)
}

fn build_profile(id: &'static str, model: ModelInfo, features: Features, nested: bool) -> Profile {
    let config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &model,
        available_models: &[],
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Live),
        session_source: SessionSource::Cli,
        sandbox_policy: &SandboxPolicy::DangerFullAccess,
        windows_sandbox_level: WindowsSandboxLevel::Disabled,
    })
    .with_unified_exec_shell_mode(UnifiedExecShellMode::Direct)
    .with_web_search_config(None)
    .with_allow_login_shell(true)
    .with_has_environment(true)
    .with_spawn_agent_usage_hint(true)
    .with_spawn_agent_usage_hint_text(None)
    .with_hide_spawn_agent_metadata(false)
    .with_goal_tools_allowed(true)
    .with_max_concurrent_threads_per_session(None)
    .with_agent_type_description("R7 closure profile".into());
    Profile {
        id,
        config: if nested {
            config.for_code_mode_nested_tools()
        } else {
            config
        },
        nested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_use_production_config_derivation() {
        let profiles = production_profiles();
        assert_eq!(profiles.len(), 7);
        let local = profiles
            .iter()
            .find(|profile| profile.id == "local_shell")
            .expect("local profile");
        assert_eq!(local.config.shell_type, ConfigShellToolType::Local);
        let nested = profiles
            .iter()
            .find(|profile| profile.id == "code_nested")
            .expect("nested profile");
        assert!(nested.nested);
        assert!(!nested.config.code_mode_enabled);
        let function = profiles
            .iter()
            .find(|profile| profile.id == "function_unified_exec")
            .expect("function profile");
        assert_eq!(
            function.config.apply_patch_tool_type,
            Some(ApplyPatchToolType::Function)
        );
        assert!(function.config.request_permissions_tool_enabled);
    }
}
