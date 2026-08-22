use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Non-secret identity for selecting a model provider and its authentication path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProviderRoute {
    pub model_provider_id: String,
    pub access_method: ProviderAccessMethod,
}

impl ProviderRoute {
    pub fn new(model_provider_id: impl Into<String>, access_method: ProviderAccessMethod) -> Self {
        Self {
            model_provider_id: model_provider_id.into(),
            access_method,
        }
    }
}

/// Authentication path used for a provider route. This enum never carries credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ProviderAccessMethod {
    Chatgpt,
    ApiKey,
}

#[cfg(test)]
mod tests {
    use super::ProviderAccessMethod;
    use super::ProviderRoute;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn serializes_without_auth_material() {
        let route = ProviderRoute::new("openai", ProviderAccessMethod::Chatgpt);

        assert_eq!(
            serde_json::to_value(route).expect("route should serialize"),
            json!({
                "modelProviderId": "openai",
                "accessMethod": "chatgpt"
            })
        );
    }

    #[test]
    fn distinguishes_openai_auth_paths() {
        let subscription = ProviderRoute::new("openai", ProviderAccessMethod::Chatgpt);
        let api = ProviderRoute::new("openai", ProviderAccessMethod::ApiKey);

        assert_ne!(subscription, api);
    }
}
