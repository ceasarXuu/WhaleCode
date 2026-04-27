use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version)]
struct ExecServerArgs {
    /// Transport endpoint URL. Supported values: `ws://IP:PORT`.
    #[arg(
        long = "listen",
        value_name = "URL",
        default_value = codex_exec_server::DEFAULT_LISTEN_URL
    )]
    listen: String,

    /// Original Whale CLI binary used when exec-server must re-enter the agent CLI.
    #[arg(long = "codex-bin", value_name = "PATH")]
    codex_bin: PathBuf,

    /// Original Linux sandbox helper path forwarded by the Whale CLI.
    #[arg(long = "linux-sandbox-bin", value_name = "PATH")]
    linux_sandbox_bin: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = ExecServerArgs::parse();
    let runtime_paths = codex_exec_server::ExecServerRuntimePaths::new(
        absolute_path(args.codex_bin)?,
        args.linux_sandbox_bin.map(absolute_path).transpose()?,
    )?;
    codex_exec_server::run_main(&args.listen, runtime_paths)
        .await
        .map_err(anyhow::Error::from_boxed)
}

fn absolute_path(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_server_accepts_forwarded_runtime_paths() {
        let args = ExecServerArgs::try_parse_from([
            "whale-exec-server",
            "--listen",
            "ws://127.0.0.1:5000",
            "--codex-bin",
            "/tmp/whale",
            "--linux-sandbox-bin",
            "/tmp/codex-linux-sandbox",
        ])
        .expect("parse");

        assert_eq!(args.listen, "ws://127.0.0.1:5000");
        assert_eq!(args.codex_bin, PathBuf::from("/tmp/whale"));
        assert_eq!(
            args.linux_sandbox_bin,
            Some(PathBuf::from("/tmp/codex-linux-sandbox"))
        );
    }
}
