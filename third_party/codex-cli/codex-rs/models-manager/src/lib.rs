pub mod cache;
pub mod collaboration_mode_presets;
pub(crate) mod config;
pub mod manager;
pub mod model_info;
pub mod model_presets;
pub mod test_support;

pub use codex_protocol::auth::AuthMode;
pub use config::ModelsManagerConfig;

/// Load the bundled model catalog shipped with `codex-models-manager`.
pub fn bundled_models_response()
-> std::result::Result<codex_protocol::openai_models::ModelsResponse, serde_json::Error> {
    let mut response: codex_protocol::openai_models::ModelsResponse =
        serde_json::from_str(include_str!("../models.json"))?;
    for model in response
        .models
        .iter_mut()
        .filter(|model| model.slug.starts_with("deepseek-"))
    {
        model_info::use_whale_base_instructions_if_empty(model);
    }
    Ok(response)
}

/// Convert the client version string to a whole version string (e.g. "1.2.3-alpha.4" -> "1.2.3").
pub fn client_version_to_whole() -> String {
    codex_login::default_client::OPENAI_CODEX_COMPATIBILITY_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::client_version_to_whole;

    #[test]
    fn models_client_uses_stable_codex_compatibility_version() {
        assert_eq!(client_version_to_whole(), "0.149.1");
    }
}
