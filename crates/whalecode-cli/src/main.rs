use std::{
    io::{self, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use whalecode_core::{default_session_path, run_bootstrap_agent, AgentError};

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
    },
}

fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Status) => print_status()?,
        Some(Command::Run { task, cwd, session }) => run_once(task, cwd, session)?,
        None => run_interactive()?,
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
}

fn print_status() -> Result<(), CliError> {
    let session_path = default_session_path()?;
    println!("WhaleCode V1 generic agent CLI substrate");
    println!("command: whale");
    println!("runtime: bootstrap_agent_loop");
    println!("session_store: jsonl");
    println!("model: bootstrap-local");
    println!("deepseek_adapter: request_builder_and_sse_parser");
    println!("primitive_host: scaffolded");
    println!("next_session_path: {}", session_path.display());
    Ok(())
}

fn run_once(
    task: Vec<String>,
    cwd: Option<PathBuf>,
    session: Option<PathBuf>,
) -> Result<(), CliError> {
    let task = normalize_task(task)?;
    let cwd = match cwd {
        Some(path) => path,
        None => std::env::current_dir().map_err(CliError::CurrentDir)?,
    };
    let summary = run_bootstrap_agent(task, cwd, session)?;
    println!("{}", summary.final_message);
    println!();
    println!("session: {}", summary.session_path.display());
    println!("events: {}", summary.events_written);
    Ok(())
}

fn run_interactive() -> Result<(), CliError> {
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
        run_once(vec![task.to_owned()], None, None)?;
    }
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
