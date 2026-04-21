mod init;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "zellai",
    version,
    about = "AI agent workspace manager for Zellij"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize zellai hooks for the current project
    Init {
        /// Overwrite existing hook files even if not managed by zellai
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { force } => {
            if let Err(msg) = init::run(force) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
    }
}
