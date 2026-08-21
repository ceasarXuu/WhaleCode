use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct AppCommand {
    /// Workspace path to open in the Desktop app.
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: PathBuf,
}

pub async fn run_app(cmd: AppCommand) -> anyhow::Result<()> {
    let _ = cmd;
    anyhow::bail!(
        "Whale Desktop is not distributed yet; this command will not install Whale Desktop."
    )
}
