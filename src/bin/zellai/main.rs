mod doctor;
mod init;
mod run;
mod status_writer;
#[cfg(not(target_arch = "wasm32"))]
mod teams_cmd;
#[cfg(not(target_arch = "wasm32"))]
mod workspace_cmd;

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

    /// Create a new workspace
    New {
        /// Workspace name
        name: String,

        /// Workspace template (single-agent, team, review, research)
        #[arg(long, default_value = "single-agent")]
        template: String,

        /// Working directory (default: current directory)
        #[arg(long)]
        dir: Option<String>,

        /// Overwrite existing workspace
        #[arg(long)]
        force: bool,
    },

    /// List saved workspaces
    List,

    /// Delete a saved workspace
    Kill {
        /// Workspace name to delete
        name: String,
    },

    /// Attach to (restore) a saved workspace
    Attach {
        /// Workspace name to restore
        name: String,
    },

    /// Launch a multi-agent team layout
    Teams {
        /// Layout override (orchestrator-top, orchestrator-left, equal-grid)
        #[arg(long)]
        layout: Option<String>,

        /// Working directory (default: current directory)
        #[arg(long)]
        dir: Option<String>,
    },

    /// Check environment and diagnose issues
    Doctor,
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
        #[cfg(not(target_arch = "wasm32"))]
        Commands::New {
            name,
            template,
            dir,
            force,
        } => {
            if let Err(msg) = workspace_cmd::cmd_new(&name, &template, dir.as_deref(), force) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Commands::List => {
            if let Err(msg) = workspace_cmd::cmd_list() {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Commands::Kill { name } => {
            if let Err(msg) = workspace_cmd::cmd_kill(&name) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Commands::Attach { name } => {
            if let Err(msg) = workspace_cmd::cmd_attach(&name) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Commands::Teams { layout, dir } => {
            if let Err(msg) = teams_cmd::cmd_teams(layout.as_deref(), dir.as_deref()) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
        #[cfg(target_arch = "wasm32")]
        Commands::New { .. }
        | Commands::List
        | Commands::Kill { .. }
        | Commands::Attach { .. }
        | Commands::Teams { .. } => {
            eprintln!("workspace commands are not available in WASM builds");
            std::process::exit(1);
        }
        Commands::Doctor => {
            if let Err(msg) = doctor::run() {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
    }
}
