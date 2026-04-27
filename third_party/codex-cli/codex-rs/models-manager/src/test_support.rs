//! Test-only helpers exposed for dependent crate tests.
//!
//! Production code should not depend on this module.

use crate::ModelsManagerConfig;
use crate::bundled_models_response;
use crate::manager::construct_model_info_from_candidates;
use crate::manager::mark_whale_default_model;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelPreset;

/// Get model identifier without consulting remote state or cache.
pub fn get_model_offline_for_tests(model: Option<&str>) -> String {
    if let Some(model) = model {
        return model.to_string();
    }
    let presets = model_presets_offline_for_tests();
    presets
        .iter()
        .find(|preset| preset.is_default)
        .or_else(|| presets.first())
        .map(|preset| preset.model.clone())
        .unwrap_or_default()
}

/// Build picker-ready model presets without consulting remote state or cache.
pub fn model_presets_offline_for_tests() -> Vec<ModelPreset> {
    let mut response = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    response.models.sort_by(|a, b| a.priority.cmp(&b.priority));
    let mut presets: Vec<ModelPreset> = response.models.into_iter().map(Into::into).collect();
    ModelPreset::mark_default_by_picker_visibility(&mut presets);
    mark_whale_default_model(&mut presets);
    presets
}

/// Build `ModelInfo` without consulting remote state or cache.
pub fn construct_model_info_offline_for_tests(
    model: &str,
    config: &ModelsManagerConfig,
) -> ModelInfo {
    let candidates: &[ModelInfo] = if let Some(model_catalog) = config.model_catalog.as_ref() {
        &model_catalog.models
    } else {
        &[]
    };
    construct_model_info_from_candidates(model, candidates, config)
}
