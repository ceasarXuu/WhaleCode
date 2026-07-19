use super::*;
use crate::ModelsManagerConfig;
use pretty_assertions::assert_eq;

#[test]
fn reasoning_summaries_override_true_enables_support() {
    let model = model_info_from_slug("unknown-model");
    let config = ModelsManagerConfig {
        model_supports_reasoning_summaries: Some(true),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);
    let mut expected = model;
    expected.supports_reasoning_summaries = true;

    assert_eq!(updated, expected);
}

#[test]
fn reasoning_summaries_override_false_does_not_disable_support() {
    let mut model = model_info_from_slug("unknown-model");
    model.supports_reasoning_summaries = true;
    let config = ModelsManagerConfig {
        model_supports_reasoning_summaries: Some(false),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}

#[test]
fn reasoning_summaries_override_false_is_noop_when_model_is_false() {
    let model = model_info_from_slug("unknown-model");
    let config = ModelsManagerConfig {
        model_supports_reasoning_summaries: Some(false),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}

#[test]
fn model_context_window_override_clamps_to_max_context_window() {
    let mut model = model_info_from_slug("unknown-model");
    model.context_window = Some(273_000);
    model.max_context_window = Some(400_000);
    let config = ModelsManagerConfig {
        model_context_window: Some(500_000),
        ..Default::default()
    };

    let updated = with_config_overrides(model.clone(), &config);
    let mut expected = model;
    expected.context_window = Some(400_000);

    assert_eq!(updated, expected);
}

#[test]
fn model_context_window_uses_model_value_without_override() {
    let mut model = model_info_from_slug("unknown-model");
    model.context_window = Some(273_000);
    model.max_context_window = Some(400_000);
    let config = ModelsManagerConfig::default();

    let updated = with_config_overrides(model.clone(), &config);

    assert_eq!(updated, model);
}

#[test]
fn deepseek_v4_uses_whalecode_standard_base_instructions() {
    for slug in [
        "deepseek-v4-flash",
        "deepseek-v4-pro",
        "deepseek/deepseek-v4-pro",
    ] {
        let model = model_info_from_slug(slug);
        let updated = with_config_overrides(model, &ModelsManagerConfig::default());

        assert_eq!(
            updated.base_instructions,
            BASE_INSTRUCTIONS_WHALECODE_STANDARD
        );
        assert_eq!(updated.model_messages, None);
    }
}

#[test]
fn explicit_base_instructions_override_wins_for_deepseek_v4() {
    let model = model_info_from_slug("deepseek-v4-pro");
    let config = ModelsManagerConfig {
        base_instructions: Some("custom base instructions".to_string()),
        ..Default::default()
    };

    let updated = with_config_overrides(model, &config);

    assert_eq!(updated.base_instructions, "custom base instructions");
    assert_eq!(updated.model_messages, None);
}

#[test]
fn non_deepseek_model_keeps_its_model_base_instructions() {
    let model = model_info_from_slug("other-model");

    let updated = with_config_overrides(model.clone(), &ModelsManagerConfig::default());

    assert_eq!(updated, model);
}
