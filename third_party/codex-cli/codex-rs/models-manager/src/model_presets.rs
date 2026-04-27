use codex_protocol::openai_models::ModelPreset;

const WHALE_MODEL_PREFIX: &str = "deepseek-";

/// Legacy notice keys kept for config compatibility with older migration prompts.
///
/// Hardcoded model presets were removed; model listings are now derived from the active catalog.
pub const HIDE_GPT5_1_MIGRATION_PROMPT_CONFIG: &str = "hide_gpt5_1_migration_prompt";
pub const HIDE_GPT_5_1_CODEX_MAX_MIGRATION_PROMPT_CONFIG: &str =
    "hide_gpt-5.1-codex-max_migration_prompt";

/// Keep Whale's public model listing focused on DeepSeek-backed choices.
pub(crate) fn retain_whale_models_for_listing(presets: &mut Vec<ModelPreset>) {
    presets.retain(|preset| is_whale_model(&preset.model));
}

pub(crate) fn is_whale_model(model: &str) -> bool {
    model.starts_with(WHALE_MODEL_PREFIX)
}
