mod init;
mod run;
mod status_writer;

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

    /// Run a command with zellai status tracking
    Run {
        /// Agent name (default: detect from command, or "unknown")
        #[arg(long, default_value = "unknown")]
        agent: String,

        /// The command and arguments to run
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
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
        Commands::Run { agent, command } => {
            if let Err(msg) = run::run(agent, command) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
    }
}
