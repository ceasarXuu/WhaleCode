/// The current Whale CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Monotonic Whale build number embedded at compile time.
pub const WHALE_BUILD_NUMBER: &str = env!("WHALE_BUILD_NUMBER");

pub(crate) fn whale_version_display() -> String {
    format!("v{CODEX_CLI_VERSION} build {WHALE_BUILD_NUMBER}")
}
