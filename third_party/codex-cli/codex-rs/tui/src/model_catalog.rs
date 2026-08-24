use codex_app_server_protocol::ProviderModelAvailability;
use codex_protocol::ProviderRoute;
use codex_protocol::openai_models::ModelPreset;
use std::convert::Infallible;

#[derive(Debug, Clone)]
pub(crate) struct ProviderModelGroup {
    pub(crate) route: ProviderRoute,
    pub(crate) display_name: String,
    pub(crate) availability: ProviderModelAvailability,
    pub(crate) models: Vec<ModelPreset>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModelCatalog {
    models: Vec<ModelPreset>,
    provider_groups: Vec<ProviderModelGroup>,
}

impl ModelCatalog {
    pub(crate) fn new(models: Vec<ModelPreset>) -> Self {
        Self {
            models,
            provider_groups: Vec::new(),
        }
    }

    pub(crate) fn with_provider_groups(
        models: Vec<ModelPreset>,
        provider_groups: Vec<ProviderModelGroup>,
    ) -> Self {
        Self {
            models,
            provider_groups,
        }
    }

    pub(crate) fn try_list_models(&self) -> Result<Vec<ModelPreset>, Infallible> {
        Ok(self.models.clone())
    }

    pub(crate) fn provider_groups(&self) -> &[ProviderModelGroup] {
        &self.provider_groups
    }

    pub(crate) fn provider_availability(
        &self,
        route: &ProviderRoute,
    ) -> Option<&ProviderModelAvailability> {
        self.provider_groups
            .iter()
            .find(|group| &group.route == route)
            .map(|group| &group.availability)
    }

    pub(crate) fn default_model_for_route(&self, route: &ProviderRoute) -> Option<&ModelPreset> {
        let models = &self
            .provider_groups
            .iter()
            .find(|group| &group.route == route)?
            .models;
        models
            .iter()
            .find(|model| model.is_default)
            .or_else(|| models.first())
    }
}
