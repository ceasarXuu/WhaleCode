mod handlers;
mod manifest;
mod providers;
mod safety;

pub(crate) use handlers::WebFetchHandler;
pub(crate) use handlers::WebSearchHandler;
pub(crate) use manifest::resolve_web_tool_manifest_availability;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum WebToolError {
    #[error("{tool} is disabled")]
    Disabled { tool: &'static str },
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("unsafe URL rejected: {0}")]
    UnsafeUrl(String),
    #[error("{provider} is missing API key environment variable {env_var}")]
    MissingApiKey {
        provider: &'static str,
        env_var: String,
    },
    #[error("{provider} returned HTTP {status}: {message}")]
    Http {
        provider: &'static str,
        status: u16,
        message: String,
    },
    #[error("{provider} request failed: {source}")]
    Network {
        provider: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{provider} response could not be parsed: {message}")]
    Parse {
        provider: &'static str,
        message: String,
    },
    #[error("secret store error: {message}")]
    SecretStore { message: String },
}

impl WebToolError {
    fn is_fallback_candidate(&self) -> bool {
        matches!(
            self,
            Self::MissingApiKey { .. }
                | Self::Network { .. }
                | Self::Http {
                    status: 408 | 409 | 425 | 429 | 500..=599,
                    ..
                }
        )
    }
}
