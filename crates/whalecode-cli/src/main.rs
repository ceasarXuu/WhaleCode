use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "whalecode")]
#[command(about = "DeepSeek-first coding agent CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Status) {
        Command::Status => {
            println!("WhaleCode V1 generic agent CLI substrate");
            println!("runtime: scaffolded");
            println!("primitive_host: scaffolded");
        }
    }
}
