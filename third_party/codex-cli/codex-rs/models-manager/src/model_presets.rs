use codex_protocol::openai_models::ModelPreset;

pub const WHALE_DEFAULT_MODEL: &str = "deepseek-v4-flash";
const WHALE_MODEL_PREFIX: &str = "deepseek-";

/// Legacy notice keys kept for config compatibility with older migration prompts.
///
/// Hardcoded model presets were removed; model listings are now derived from the active catalog.
pub const HIDE_GPT5_1_MIGRATION_PROMPT_CONFIG: &str = "hide_gpt5_1_migration_prompt";
pub const HIDE_GPT_5_1_CODEX_MAX_MIGRATION_PROMPT_CONFIG: &str =
    "hide_gpt-5.1-codex-max_migration_prompt";

pub(crate) fn retain_whale_models_for_listing(presets: &mut Vec<ModelPreset>) {
    presets.retain(|preset| preset.model.starts_with(WHALE_MODEL_PREFIX));
}

pub(crate) fn mark_whale_default_model(presets: &mut [ModelPreset]) {
    if let Some(default_index) = presets
        .iter()
        .position(|preset| preset.model == WHALE_DEFAULT_MODEL && preset.show_in_picker)
    {
        for (index, preset) in presets.iter_mut().enumerate() {
            preset.is_default = index == default_index;
        }
    }
}
