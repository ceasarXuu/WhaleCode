use clap::Parser;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_mcp_server::run_main;
use codex_utils_cli::CliConfigOverrides;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version)]
struct McpServerArgs {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    /// Original Whale CLI binary used when MCP tools must re-enter the agent CLI.
    #[arg(long = "codex-bin", value_name = "PATH", hide = true)]
    codex_bin: Option<PathBuf>,

    /// Original Linux sandbox helper path forwarded by the Whale CLI.
    #[arg(long = "linux-sandbox-bin", value_name = "PATH", hide = true)]
    linux_sandbox_bin: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let args = McpServerArgs::parse();
        run_main(
            mcp_server_runtime_paths(arg0_paths, &args),
            args.config_overrides,
        )
        .await?;
        Ok(())
    })
}

fn mcp_server_runtime_paths(
    mut arg0_paths: Arg0DispatchPaths,
    args: &McpServerArgs,
) -> Arg0DispatchPaths {
    if let Some(codex_bin) = args.codex_bin.clone() {
        arg0_paths.codex_self_exe = Some(codex_bin);
    }
    if let Some(linux_sandbox_bin) = args.linux_sandbox_bin.clone() {
        arg0_paths.codex_linux_sandbox_exe = Some(linux_sandbox_bin);
    }
    arg0_paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_accepts_forwarded_runtime_paths() {
        let args = McpServerArgs::try_parse_from([
            "whale-mcp-server",
            "--codex-bin",
            "/tmp/whale",
            "--linux-sandbox-bin",
            "/tmp/codex-linux-sandbox",
        ])
        .expect("parse");
        let paths = mcp_server_runtime_paths(Arg0DispatchPaths::default(), &args);

        assert_eq!(paths.codex_self_exe, Some(PathBuf::from("/tmp/whale")));
        assert_eq!(
            paths.codex_linux_sandbox_exe,
            Some(PathBuf::from("/tmp/codex-linux-sandbox"))
        );
    }
}
