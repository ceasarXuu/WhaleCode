use anyhow::Result;
use anyhow::bail;

pub(crate) async fn run(_http_client_factory: codex_http_client::HttpClientFactory) -> Result<()> {
    bail!(
        "Whale standalone updates are disabled until a Whale-owned installer channel is published"
    )
}

#[cfg(unix)]
pub(crate) fn reexec_managed_updater(managed_whale_bin: &std::path::Path) -> Result<()> {
    use anyhow::Context;
    use std::os::unix::process::CommandExt;

    let err = std::process::Command::new(managed_whale_bin)
        .args(["app-server", "daemon", "pid-update-loop"])
        .exec();
    Err(err).with_context(|| {
        format!(
            "failed to replace updater with managed Whale binary {}",
            managed_whale_bin.display()
        )
    })
}
