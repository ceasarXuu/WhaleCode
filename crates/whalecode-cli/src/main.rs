use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use whalecode_core::{
    default_live_max_turns, default_session_path, run_bootstrap_agent,
    run_live_agent_with_observer, AgentError, LiveAgentOptions,
};
use whalecode_model::{
    deepseek_api_key_source, response_from_stream_events, store_deepseek_api_key, ChatMessage,
    DeepSeekApiKeySource, DeepSeekChatRequest, DeepSeekClient, DeepSeekConfig, ModelError,
    ModelStreamEvent, SecretStoreError, DEEPSEEK_DEFAULT_MODEL,
};
use whalecode_protocol::ModelUsage;

mod line_input;
mod session_view;

use line_input::{LineInput, LineReader};

#[derive(Debug, Parser)]
#[command(name = "whale")]
#[command(about = "DeepSeek-first coding agent CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    Logs {
        #[arg(long)]
        session: Option<PathBuf>,
    },
    Run {
        #[arg(required = true, num_args = 1..)]
        task: Vec<String>,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        session: Option<PathBuf>,
        #[arg(long, help = "Accepted for compatibility; run is live by default")]
        live: bool,
        #[arg(long, conflicts_with = "live")]
        bootstrap: bool,
        #[arg(long)]
        allow_write: bool,
        #[arg(long)]
        allow_command: bool,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, default_value_t = default_live_max_turns())]
        max_turns: usize,
    },
    ModelSmoke {
        #[arg(required = true, num_args = 1..)]
        prompt: Vec<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run_cli().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run_cli() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Status) => print_status()?,
        Some(Command::Logs { session }) => session_view::print_session_log(session)?,
        Some(Command::Run {
            task,
            cwd,
            session,
            live: _,
            bootstrap,
            allow_write,
            allow_command,
            model,
            max_turns,
        }) => {
            run_once(
                RunInvocation {
                    task,
                    cwd,
                    session,
                    mode: if bootstrap {
                        RunMode::Bootstrap
                    } else {
                        RunMode::Live
                    },
                    allow_write,
                    allow_command,
                    model,
                    max_turns,
                },
                true,
            )
            .await?
        }
        Some(Command::ModelSmoke {
            prompt,
            model,
            base_url,
        }) => model_smoke(prompt, model, base_url).await?,
        None => run_interactive().await?,
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("task cannot be empty")]
    EmptyTask,
    #[error("failed to resolve current directory: {0}")]
    CurrentDir(std::io::Error),
    #[error("failed to read input: {0}")]
    ReadInput(std::io::Error),
    #[error("failed to write output: {0}")]
    WriteOutput(std::io::Error),
    #[error("agent error: {0}")]
    Agent(#[from] AgentError),
    #[error("model error: {0}")]
    Model(#[from] ModelError),
    #[error("secret store error: {0}")]
    Secret(#[from] SecretStoreError),
    #[error("session view error: {0}")]
    SessionView(#[from] session_view::SessionViewError),
}

fn print_status() -> Result<(), CliError> {
    let session_path = default_session_path()?;
    let workspace = std::env::current_dir().map_err(CliError::CurrentDir)?;
    println!("WhaleCode V1 generic agent CLI substrate");
    println!("command: whale");
    println!("workspace: {}", workspace.display());
    println!(
        "deepseek_api_key: {}",
        api_key_source_label(deepseek_api_key_source())
    );
    println!("runtime: live_deepseek_tool_loop");
    println!("session_store: jsonl");
    println!("session_logs: whale logs");
    println!("model: {DEEPSEEK_DEFAULT_MODEL}");
    println!("deepseek_adapter: request_builder_sse_parser_tool_calls");
    println!("live_model_smoke: whale model-smoke --model {DEEPSEEK_DEFAULT_MODEL} \"hello\"");
    println!("live_run: whale run --allow-write --allow-command \"fix the bug\"");
    println!("bootstrap_debug: whale run --bootstrap \"inspect this repo\"");
    println!("primitive_host: scaffolded");
    println!("next_session_path: {}", session_path.display());
    Ok(())
}

struct RunInvocation {
    task: Vec<String>,
    cwd: Option<PathBuf>,
    session: Option<PathBuf>,
    mode: RunMode,
    allow_write: bool,
    allow_command: bool,
    model: Option<String>,
    max_turns: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Live,
    Bootstrap,
}

async fn run_once(invocation: RunInvocation, print_session_footer: bool) -> Result<(), CliError> {
    let task = normalize_task(invocation.task)?;
    let cwd = match invocation.cwd {
        Some(path) => path,
        None => std::env::current_dir().map_err(CliError::CurrentDir)?,
    };
    let summary = if invocation.mode == RunMode::Live {
        let session_path = match invocation.session {
            Some(path) => path,
            None => default_session_path()?,
        };
        let options = LiveAgentOptions {
            task,
            cwd,
            session_path,
            model: invocation
                .model
                .unwrap_or_else(|| DEEPSEEK_DEFAULT_MODEL.to_owned()),
            allow_write: invocation.allow_write,
            allow_command: invocation.allow_command,
            max_turns: invocation.max_turns,
        };
        let mut streamed_text = false;
        let mut stream_error = None;
        let summary = {
            let mut observer = |event: &ModelStreamEvent| {
                if let ModelStreamEvent::TextDelta(content) = event {
                    streamed_text = true;
                    if stream_error.is_none() {
                        if let Err(error) = write_stream_delta(content) {
                            stream_error = Some(error);
                        }
                    }
                }
            };
            run_live_agent_with_observer(options, Some(&mut observer)).await?
        };
        if let Some(error) = stream_error {
            return Err(CliError::WriteOutput(error));
        }
        if streamed_text {
            println!();
        } else {
            println!("{}", summary.final_message);
        }
        summary
    } else {
        let summary = run_bootstrap_agent(task, cwd, invocation.session)?;
        println!("{}", summary.final_message);
        summary
    };
    print_token_usage(&summary.usage);
    if print_session_footer {
        println!();
        println!("session: {}", summary.session_path.display());
        println!("events: {}", summary.events_written);
    }
    Ok(())
}

fn print_token_usage(usage: &ModelUsage) {
    println!("input tokens: {}", usage.input_tokens);
    println!("output tokens: {}", usage.output_tokens);
}

fn write_stream_delta(content: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "{content}")?;
    stdout.flush()
}

async fn run_interactive() -> Result<(), CliError> {
    let mut stdout = io::stdout();
    let mut settings = InteractiveSettings::default();
    let session_path = default_session_path()?;
    let mut input = LineReader::new("whale> ");
    writeln!(
        stdout,
        "Whale live agent. Type a task and press Enter, /apikey to store a DeepSeek key, /permissions to inspect gates, or /exit to quit."
    )
    .map_err(CliError::WriteOutput)?;
    writeln!(stdout, "session: {}", session_path.display()).map_err(CliError::WriteOutput)?;
    loop {
        let line = match input.read_line().map_err(CliError::ReadInput)? {
            LineInput::Submit(line) => line,
            LineInput::Exit => break,
        };
        let task = line.trim();
        if task.is_empty() {
            continue;
        }
        if matches!(task, "/exit" | "exit" | "quit") {
            break;
        }
        if task == "/apikey" {
            save_api_key_interactively(&mut stdout)?;
            continue;
        }
        if handle_interactive_command(task, &mut settings, &mut stdout)? {
            continue;
        }
        if let Err(error) = run_once(
            RunInvocation {
                task: vec![task.to_owned()],
                cwd: None,
                session: Some(session_path.clone()),
                mode: RunMode::Live,
                allow_write: settings.allow_write,
                allow_command: settings.allow_command,
                model: Some(settings.model.clone()),
                max_turns: settings.max_turns,
            },
            false,
        )
        .await
        {
            writeln!(stdout, "error: {error}").map_err(CliError::WriteOutput)?;
            if is_missing_api_key_error(&error) {
                writeln!(stdout, "Run /apikey to store your DeepSeek API key.")
                    .map_err(CliError::WriteOutput)?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct InteractiveSettings {
    allow_write: bool,
    allow_command: bool,
    model: String,
    max_turns: usize,
}

impl Default for InteractiveSettings {
    fn default() -> Self {
        Self {
            allow_write: true,
            allow_command: false,
            model: DEEPSEEK_DEFAULT_MODEL.to_owned(),
            max_turns: default_live_max_turns(),
        }
    }
}

fn handle_interactive_command(
    task: &str,
    settings: &mut InteractiveSettings,
    stdout: &mut io::Stdout,
) -> Result<bool, CliError> {
    match task {
        "/permissions" => {
            print_interactive_permissions(settings, stdout)?;
            Ok(true)
        }
        "/write on" => {
            settings.allow_write = true;
            writeln!(stdout, "edit_file: enabled").map_err(CliError::WriteOutput)?;
            Ok(true)
        }
        "/write off" => {
            settings.allow_write = false;
            writeln!(stdout, "edit_file: disabled").map_err(CliError::WriteOutput)?;
            Ok(true)
        }
        "/command on" => {
            settings.allow_command = true;
            writeln!(stdout, "run_command: enabled").map_err(CliError::WriteOutput)?;
            Ok(true)
        }
        "/command off" => {
            settings.allow_command = false;
            writeln!(stdout, "run_command: disabled").map_err(CliError::WriteOutput)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn print_interactive_permissions(
    settings: &InteractiveSettings,
    stdout: &mut io::Stdout,
) -> Result<(), CliError> {
    writeln!(stdout, "mode: live").map_err(CliError::WriteOutput)?;
    writeln!(stdout, "model: {}", settings.model).map_err(CliError::WriteOutput)?;
    writeln!(stdout, "edit_file: {}", enabled_label(settings.allow_write))
        .map_err(CliError::WriteOutput)?;
    writeln!(
        stdout,
        "run_command: {}",
        enabled_label(settings.allow_command)
    )
    .map_err(CliError::WriteOutput)?;
    writeln!(stdout, "max_turns: {}", settings.max_turns).map_err(CliError::WriteOutput)?;
    Ok(())
}

fn enabled_label(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn is_missing_api_key_error(error: &CliError) -> bool {
    matches!(
        error,
        CliError::Model(ModelError::MissingApiKey)
            | CliError::Agent(AgentError::Model(ModelError::MissingApiKey))
    )
}

fn save_api_key_interactively(stdout: &mut io::Stdout) -> Result<(), CliError> {
    let key = prompt_api_key(stdout)?;
    let path = store_deepseek_api_key(&key)?;
    writeln!(
        stdout,
        "DeepSeek API key saved to user secret store: {}",
        path.display()
    )
    .map_err(CliError::WriteOutput)?;
    Ok(())
}

fn prompt_api_key(stdout: &mut io::Stdout) -> Result<String, CliError> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        rpassword::prompt_password("DeepSeek API key: ").map_err(CliError::ReadInput)
    } else {
        writeln!(stdout, "DeepSeek API key:").map_err(CliError::WriteOutput)?;
        stdout.flush().map_err(CliError::WriteOutput)?;
        let mut key = String::new();
        io::stdin()
            .read_line(&mut key)
            .map_err(CliError::ReadInput)?;
        Ok(key)
    }
}

fn api_key_source_label(source: DeepSeekApiKeySource) -> &'static str {
    match source {
        DeepSeekApiKeySource::Environment => "environment",
        DeepSeekApiKeySource::UserSecret => "user_secret",
        DeepSeekApiKeySource::Missing => "missing",
    }
}

async fn model_smoke(
    prompt: Vec<String>,
    model: Option<String>,
    base_url: Option<String>,
) -> Result<(), CliError> {
    let prompt = normalize_task(prompt)?;
    let mut config = DeepSeekConfig::from_env();
    if let Some(model) = model {
        config.model = model;
    }
    if let Some(base_url) = base_url {
        config.base_url = base_url;
    }

    let request = DeepSeekChatRequest::streaming(&config, vec![ChatMessage::user(prompt)]);
    let mut streamed_text = false;
    let mut stream_error = None;
    let events = {
        let mut observer = |event: &ModelStreamEvent| {
            if let ModelStreamEvent::TextDelta(content) = event {
                streamed_text = true;
                if stream_error.is_none() {
                    if let Err(error) = write_stream_delta(content) {
                        stream_error = Some(error);
                    }
                }
            }
        };
        DeepSeekClient::new(config)
            .stream_chat_with_observer(&request, &mut observer)
            .await?
    };
    if let Some(error) = stream_error {
        return Err(CliError::WriteOutput(error));
    }
    let response = response_from_stream_events(events.clone());

    if streamed_text {
        println!();
    } else {
        println!("{}", response.final_text);
    }
    let usage = usage_from_events(&events);
    print_token_usage(&usage);
    println!();
    println!("model_events: {}", events.len());
    Ok(())
}

fn usage_from_events(events: &[ModelStreamEvent]) -> ModelUsage {
    let mut usage = ModelUsage::default();
    for event in events {
        if let ModelStreamEvent::Usage(chunk_usage) = event {
            usage.input_tokens += chunk_usage.input_tokens;
            usage.output_tokens += chunk_usage.output_tokens;
            usage.cached_input_tokens += chunk_usage.cached_input_tokens;
        }
    }
    usage
}

fn normalize_task(task: Vec<String>) -> Result<String, CliError> {
    let task = task.join(" ");
    if task.trim().is_empty() {
        Err(CliError::EmptyTask)
    } else {
        Ok(task)
    }
}
