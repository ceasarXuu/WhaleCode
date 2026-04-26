use std::io::{self, IsTerminal, Write};

use whalecode_core::{default_live_max_turns, default_session_path};
use whalecode_model::{store_deepseek_api_key, ModelError, DEEPSEEK_DEFAULT_MODEL};

use crate::{
    line_input::{LineInput, LineReader},
    run_once_with_ctrl_c,
    run_status::{print_startup_status, RunDisplayConfig},
    CliError, RunInvocation, RunMode,
};

pub(crate) async fn run_interactive() -> Result<(), CliError> {
    let mut stdout = io::stdout();
    let mut settings = InteractiveSettings::default();
    let session_path = default_session_path()?;
    let mut input = LineReader::new("whale> ");
    writeln!(
        stdout,
        "Whale live agent. Type a task and press Enter, /apikey to store a DeepSeek key, /permissions to inspect gates, or /exit to quit. Ctrl+C interrupts the current turn or exits at the prompt."
    )
    .map_err(CliError::WriteOutput)?;
    let workspace = std::env::current_dir().map_err(CliError::CurrentDir)?;
    print_startup_status(&RunDisplayConfig {
        workspace: &workspace,
        model: &settings.model,
        allow_write: settings.allow_write,
        allow_command: settings.allow_command,
        max_turns: settings.max_turns,
        session_path: Some(&session_path),
    })
    .map_err(CliError::WriteOutput)?;
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
        if let Err(error) = run_once_with_ctrl_c(
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
            writeln!(stdout, "write_file: enabled").map_err(CliError::WriteOutput)?;
            writeln!(stdout, "edit_file: enabled").map_err(CliError::WriteOutput)?;
            Ok(true)
        }
        "/write off" => {
            settings.allow_write = false;
            writeln!(stdout, "write_file: disabled").map_err(CliError::WriteOutput)?;
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
    writeln!(
        stdout,
        "write_file: {}",
        enabled_label(settings.allow_write)
    )
    .map_err(CliError::WriteOutput)?;
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
            | CliError::Agent(whalecode_core::AgentError::Model(ModelError::MissingApiKey))
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
