use clap::Args;
use clap::Parser;
use codex_app_server::AppServerRuntimeOptions;
use codex_app_server::AppServerTransport;
use codex_app_server::AppServerWebsocketAuthArgs;
use codex_app_server::app_server_control_socket_path;
use codex_app_server::run_main_with_transport_options;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_core::config::find_codex_home;
use codex_core::config_loader::LoaderOverrides;
use codex_protocol::protocol::SessionSource;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_cli::CliConfigOverrides;
use std::path::PathBuf;

#[cfg(debug_assertions)]
use codex_app_server::PluginStartupTasks;

// Debug-only test hook: lets integration tests point the server at a temporary
// managed config file without writing to /etc.
#[cfg(debug_assertions)]
const MANAGED_CONFIG_PATH_ENV_VAR: &str = "CODEX_APP_SERVER_MANAGED_CONFIG_PATH";
#[cfg(debug_assertions)]
const DISABLE_MANAGED_CONFIG_ENV_VAR: &str = "CODEX_APP_SERVER_DISABLE_MANAGED_CONFIG";

#[derive(Debug, Parser)]
struct AppServerArgs {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    /// Omit to run the app server; specify a subcommand for tooling.
    #[command(subcommand)]
    subcommand: Option<AppServerSubcommand>,

    /// Transport endpoint URL. Supported values: `stdio://` (default),
    /// `unix://`, `unix://PATH`, `ws://IP:PORT`, `off`.
    #[arg(
        long = "listen",
        value_name = "URL",
        default_value = AppServerTransport::DEFAULT_LISTEN_URL
    )]
    listen: AppServerTransport,

    /// Controls whether analytics are enabled by default.
    #[arg(long = "analytics-default-enabled")]
    analytics_default_enabled: bool,

    /// Session source used to derive product restrictions and metadata.
    #[arg(
        long = "session-source",
        value_name = "SOURCE",
        default_value = "vscode",
        value_parser = SessionSource::from_startup_arg
    )]
    session_source: SessionSource,

    #[command(flatten)]
    auth: AppServerWebsocketAuthArgs,

    /// Original Whale CLI binary used when app-server must re-enter the agent CLI.
    #[arg(long = "codex-bin", value_name = "PATH", hide = true)]
    codex_bin: Option<PathBuf>,

    /// Original Linux sandbox helper path forwarded by the Whale CLI.
    #[arg(long = "linux-sandbox-bin", value_name = "PATH", hide = true)]
    linux_sandbox_bin: Option<PathBuf>,

    /// Hidden debug-only test hook used by integration tests that spawn the
    /// production app-server binary.
    #[cfg(debug_assertions)]
    #[arg(long = "disable-plugin-startup-tasks-for-tests", hide = true)]
    disable_plugin_startup_tasks_for_tests: bool,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::enum_variant_names)]
enum AppServerSubcommand {
    /// Proxy stdio bytes to the running app-server control socket.
    Proxy(AppServerProxyCommand),

    /// [experimental] Generate TypeScript bindings for the app server protocol.
    GenerateTs(GenerateTsCommand),

    /// [experimental] Generate JSON Schema for the app server protocol.
    GenerateJsonSchema(GenerateJsonSchemaCommand),

    /// [internal] Generate internal JSON Schema artifacts for Whale tooling.
    #[clap(hide = true)]
    GenerateInternalJsonSchema(GenerateInternalJsonSchemaCommand),
}

#[derive(Debug, Args)]
struct AppServerProxyCommand {
    /// Path to the app-server Unix domain socket to connect to.
    #[arg(long = "sock", value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    socket_path: Option<AbsolutePathBuf>,
}

#[derive(Debug, Args)]
struct GenerateTsCommand {
    /// Output directory where .ts files will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Optional path to the Prettier executable to format generated files
    #[arg(short = 'p', long = "prettier", value_name = "PRETTIER_BIN")]
    prettier: Option<PathBuf>,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    experimental: bool,
}

#[derive(Debug, Args)]
struct GenerateJsonSchemaCommand {
    /// Output directory where the schema bundle will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    experimental: bool,
}

#[derive(Debug, Args)]
struct GenerateInternalJsonSchemaCommand {
    /// Output directory where internal JSON Schema artifacts will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,
}

fn parse_socket_path(raw: &str) -> Result<AbsolutePathBuf, String> {
    AbsolutePathBuf::relative_to_current_dir(raw)
        .map_err(|err| format!("failed to resolve socket path `{raw}`: {err}"))
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let args = AppServerArgs::parse();
        let arg0_paths = app_server_runtime_paths(arg0_paths, &args);
        match args.subcommand {
            None => {
                let loader_overrides = if disable_managed_config_from_debug_env() {
                    LoaderOverrides::without_managed_config_for_tests()
                } else {
                    managed_config_path_from_debug_env()
                        .map(LoaderOverrides::with_managed_config_path_for_tests)
                        .unwrap_or_default()
                };
                let runtime_options = app_server_runtime_options(&args);
                let transport = args.listen;
                let session_source = args.session_source;
                let auth = args.auth.try_into_settings()?;

                run_main_with_transport_options(
                    arg0_paths,
                    args.config_overrides,
                    loader_overrides,
                    args.analytics_default_enabled,
                    transport,
                    session_source,
                    auth,
                    runtime_options,
                )
                .await?;
            }
            Some(AppServerSubcommand::Proxy(proxy_cli)) => {
                let socket_path = match proxy_cli.socket_path {
                    Some(socket_path) => socket_path,
                    None => {
                        let codex_home = find_codex_home()?;
                        app_server_control_socket_path(&codex_home)?
                    }
                };
                codex_stdio_to_uds::run(socket_path.as_path()).await?;
            }
            Some(AppServerSubcommand::GenerateTs(gen_cli)) => {
                let options = codex_app_server_protocol::GenerateTsOptions {
                    experimental_api: gen_cli.experimental,
                    ..Default::default()
                };
                codex_app_server_protocol::generate_ts_with_options(
                    &gen_cli.out_dir,
                    gen_cli.prettier.as_deref(),
                    options,
                )?;
            }
            Some(AppServerSubcommand::GenerateJsonSchema(gen_cli)) => {
                codex_app_server_protocol::generate_json_with_experimental(
                    &gen_cli.out_dir,
                    gen_cli.experimental,
                )?;
            }
            Some(AppServerSubcommand::GenerateInternalJsonSchema(gen_cli)) => {
                codex_app_server_protocol::generate_internal_json_schema(&gen_cli.out_dir)?;
            }
        }
        Ok(())
    })
}

fn app_server_runtime_options(args: &AppServerArgs) -> AppServerRuntimeOptions {
    let runtime_options = AppServerRuntimeOptions::default();
    #[cfg(not(debug_assertions))]
    let _ = args;
    #[cfg(debug_assertions)]
    if args.disable_plugin_startup_tasks_for_tests {
        let mut runtime_options = runtime_options;
        runtime_options.plugin_startup_tasks = PluginStartupTasks::Skip;
        return runtime_options;
    }

    runtime_options
}

fn app_server_runtime_paths(
    mut arg0_paths: Arg0DispatchPaths,
    args: &AppServerArgs,
) -> Arg0DispatchPaths {
    if let Some(codex_bin) = args.codex_bin.clone() {
        arg0_paths.codex_self_exe = Some(codex_bin);
    }
    if let Some(linux_sandbox_bin) = args.linux_sandbox_bin.clone() {
        arg0_paths.codex_linux_sandbox_exe = Some(linux_sandbox_bin);
    }
    arg0_paths
}

fn disable_managed_config_from_debug_env() -> bool {
    #[cfg(debug_assertions)]
    {
        if let Ok(value) = std::env::var(DISABLE_MANAGED_CONFIG_ENV_VAR) {
            return matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES");
        }
    }

    false
}

fn managed_config_path_from_debug_env() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        if let Ok(value) = std::env::var(MANAGED_CONFIG_PATH_ENV_VAR) {
            return if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            };
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_server_analytics_default_disabled_without_flag() {
        let args = AppServerArgs::try_parse_from(["whale-app-server"]).expect("parse");
        assert!(!args.analytics_default_enabled);
        assert_eq!(args.listen, AppServerTransport::Stdio);
    }

    #[test]
    fn app_server_analytics_default_enabled_with_flag() {
        let args =
            AppServerArgs::try_parse_from(["whale-app-server", "--analytics-default-enabled"])
                .expect("parse");
        assert!(args.analytics_default_enabled);
    }

    #[test]
    fn app_server_listen_websocket_url_parses() {
        let args =
            AppServerArgs::try_parse_from(["whale-app-server", "--listen", "ws://127.0.0.1:4500"])
                .expect("parse");
        assert_eq!(
            args.listen,
            AppServerTransport::WebSocket {
                bind_address: "127.0.0.1:4500".parse().expect("valid socket address"),
            }
        );
    }

    #[test]
    fn app_server_listen_invalid_url_fails_to_parse() {
        let parse_result =
            AppServerArgs::try_parse_from(["whale-app-server", "--listen", "http://foo"]);
        assert!(parse_result.is_err());
    }

    #[test]
    fn app_server_proxy_sock_path_parses() {
        let args =
            AppServerArgs::try_parse_from(["whale-app-server", "proxy", "--sock", "whale.sock"])
                .expect("parse");
        let Some(AppServerSubcommand::Proxy(proxy)) = args.subcommand else {
            panic!("expected proxy subcommand");
        };
        assert_eq!(
            proxy.socket_path,
            Some(
                AbsolutePathBuf::relative_to_current_dir("whale.sock")
                    .expect("relative path should resolve")
            )
        );
    }

    #[test]
    fn app_server_capability_token_flags_parse() {
        let args = AppServerArgs::try_parse_from([
            "whale-app-server",
            "--ws-auth",
            "capability-token",
            "--ws-token-file",
            "/tmp/whale-token",
        ])
        .expect("parse");
        assert_eq!(
            args.auth.ws_auth,
            Some(codex_app_server::WebsocketAuthCliMode::CapabilityToken)
        );
        assert_eq!(
            args.auth.ws_token_file,
            Some(PathBuf::from("/tmp/whale-token"))
        );
    }

    #[test]
    fn app_server_accepts_forwarded_runtime_paths() {
        let args = AppServerArgs::try_parse_from([
            "whale-app-server",
            "--codex-bin",
            "/tmp/whale",
            "--linux-sandbox-bin",
            "/tmp/codex-linux-sandbox",
        ])
        .expect("parse");
        let paths = app_server_runtime_paths(Arg0DispatchPaths::default(), &args);

        assert_eq!(paths.codex_self_exe, Some(PathBuf::from("/tmp/whale")));
        assert_eq!(
            paths.codex_linux_sandbox_exe,
            Some(PathBuf::from("/tmp/codex-linux-sandbox"))
        );
    }

    #[test]
    fn app_server_rejects_removed_insecure_non_loopback_flag() {
        let parse_result = AppServerArgs::try_parse_from([
            "whale-app-server",
            "--allow-unauthenticated-non-loopback-ws",
        ]);
        assert!(parse_result.is_err());
    }
}
