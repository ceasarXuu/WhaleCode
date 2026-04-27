use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = codex_cloud_tasks::Cli::parse();
    codex_cloud_tasks::run_main(cli, None).await
}
