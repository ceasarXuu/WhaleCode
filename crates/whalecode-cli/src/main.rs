use std::{
    io::{self, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use whalecode_core::{
    default_live_max_turns, default_session_path, run_bootstrap_agent, run_live_agent, AgentError,
    LiveAgentOptions,
};
use whalecode_model::{
    response_from_stream_events, ChatMessage, DeepSeekChatRequest, DeepSeekClient, DeepSeekConfig,
    ModelError, DEEPSEEK_DEFAULT_MODEL,
};

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
    Run {
        #[arg(required = true, num_args = 1..)]
        task: Vec<String>,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        session: Option<PathBuf>,
        #[arg(long)]
        live: bool,
        #[arg(long)]
        allow_write: bool,
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
        Some(Command::Run {
            task,
            cwd,
            session,
            live,
            allow_write,
            model,
            max_turns,
        }) => run_once(task, cwd, session, live, allow_write, model, max_turns).await?,
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
}

fn print_status() -> Result<(), CliError> {
    let session_path = default_session_path()?;
    println!("WhaleCode V1 generic agent CLI substrate");
    println!("command: whale");
    println!("runtime: bootstrap_agent_loop + live_deepseek_tool_loop");
    println!("session_store: jsonl");
    println!("model: bootstrap-local or {DEEPSEEK_DEFAULT_MODEL}");
    println!("deepseek_adapter: request_builder_sse_parser_tool_calls");
    println!("live_model_smoke: whale model-smoke --model {DEEPSEEK_DEFAULT_MODEL} \"hello\"");
    println!("live_run: whale run --live --allow-write \"fix the bug\"");
    println!("primitive_host: scaffolded");
    println!("next_session_path: {}", session_path.display());
    Ok(())
}

async fn run_once(
    task: Vec<String>,
    cwd: Option<PathBuf>,
    session: Option<PathBuf>,
    live: bool,
    allow_write: bool,
    model: Option<String>,
    max_turns: usize,
) -> Result<(), CliError> {
    let task = normalize_task(task)?;
    let cwd = match cwd {
        Some(path) => path,
        None => std::env::current_dir().map_err(CliError::CurrentDir)?,
    };
    let summary = if live {
        let session_path = match session {
            Some(path) => path,
            None => default_session_path()?,
        };
        run_live_agent(LiveAgentOptions {
            task,
            cwd,
            session_path,
            model: model.unwrap_or_else(|| DEEPSEEK_DEFAULT_MODEL.to_owned()),
            allow_write,
            max_turns,
        })
        .await?
    } else {
        run_bootstrap_agent(task, cwd, session)?
    };
    println!("{}", summary.final_message);
    println!();
    println!("session: {}", summary.session_path.display());
    println!("events: {}", summary.events_written);
    Ok(())
}

async fn run_interactive() -> Result<(), CliError> {
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "Whale bootstrap agent. Type a task and press Enter, or /exit to quit."
    )
    .map_err(CliError::WriteOutput)?;
    loop {
        write!(stdout, "whale> ").map_err(CliError::WriteOutput)?;
        stdout.flush().map_err(CliError::WriteOutput)?;
        let mut line = String::new();
        let bytes = io::stdin()
            .read_line(&mut line)
            .map_err(CliError::ReadInput)?;
        if bytes == 0 {
            break;
        }
        let task = line.trim();
        if task.is_empty() {
            continue;
        }
        if matches!(task, "/exit" | "exit" | "quit") {
            break;
        }
        run_once(
            vec![task.to_owned()],
            None,
            None,
            false,
            false,
            None,
            default_live_max_turns(),
        )
        .await?;
    }
    Ok(())
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
    let events = DeepSeekClient::new(config).stream_chat(&request).await?;
    let response = response_from_stream_events(events.clone());

    println!("{}", response.final_text);
    println!();
    println!("model_events: {}", events.len());
    Ok(())
}

fn normalize_task(task: Vec<String>) -> Result<String, CliError> {
    let task = task.join(" ");
    if task.trim().is_empty() {
        Err(CliError::EmptyTask)
    } else {
        Ok(task)
    }
}
