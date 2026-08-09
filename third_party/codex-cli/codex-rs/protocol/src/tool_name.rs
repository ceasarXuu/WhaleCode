use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Identifies a callable tool, preserving the namespace split when the model
/// provides one.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ToolName {
    pub name: String,
    pub namespace: Option<String>,
}

impl ToolName {
    pub fn new(namespace: Option<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace,
        }
    }

    pub fn plain(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
        }
    }

    pub fn namespaced(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: Some(namespace.into()),
        }
    }

    pub fn display(&self) -> String {
        match &self.namespace {
            Some(namespace) => format!("{namespace}{}", self.name),
            None => self.name.clone(),
        }
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.namespace {
            Some(namespace) => write!(f, "{namespace}{}", self.name),
            None => f.write_str(&self.name),
        }
    }
}

impl From<String> for ToolName {
    fn from(name: String) -> Self {
        Self::plain(name)
    }
}

impl From<&str> for ToolName {
    fn from(name: &str) -> Self {
        Self::plain(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn serde_round_trips_plain_and_namespaced_identity_without_flattening() {
        let identities = [
            ToolName::plain("alpha_beta_gamma"),
            ToolName::namespaced("alpha", "beta_gamma"),
            ToolName::namespaced("alpha_beta", "gamma"),
            ToolName::namespaced("namespace:/with:_separators", "tool:/with:_separators"),
        ];

        for identity in identities {
            let encoded = serde_json::to_string(&identity).expect("ToolName should serialize");
            let decoded =
                serde_json::from_str::<ToolName>(&encoded).expect("ToolName should deserialize");
            assert_eq!(decoded, identity);
        }
    }
}
