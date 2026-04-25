mod doctor;
mod init;
mod log;
mod run;
mod status_writer;
#[cfg(not(target_arch = "wasm32"))]
mod teams_cmd;
#[cfg(not(target_arch = "wasm32"))]
mod workspace_cmd;

use clap::{CommandFactory, Parser, Subcommand};

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

    /// Run a command as a named agent wrapper
    ///
    /// Explicitly sets the agent kind and runs the trailing command with status
    /// tracking. This is the subcommand equivalent of using a `zellai-<agent>`
    /// symlink.
    ///
    /// Example: zellai wrap --agent codex -- codex --model o3
    ///
    /// To create named wrappers, symlink the zellai binary:
    ///   ln -s $(which zellai) ~/.local/bin/zellai-codex
    ///   ln -s $(which zellai) ~/.local/bin/zellai-gemini
    ///   ln -s $(which zellai) ~/.local/bin/zellai-aider
    ///
    /// Or use shell aliases:
    ///   alias zellai-codex='zellai wrap --agent codex -- codex'
    Wrap {
        /// Agent name (required)
        #[arg(long)]
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

    /// View per-pane execution logs
    Log {
        /// Pane name to show logs for
        pane: String,

        /// Workspace name (default: from ZELLAI_WORKSPACE env or "default")
        #[arg(long)]
        workspace: Option<String>,

        /// Number of lines to show (default: all)
        #[arg(long, short = 'n')]
        lines: Option<usize>,

        /// Follow log output (not yet implemented)
        #[arg(long, short = 'f')]
        follow: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for (bash, zsh, fish)
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

fn main() {
    // Check argv[0] for named wrapper invocation (e.g., zellai-codex, zellai-gemini).
    // If the binary is invoked via a symlink like `zellai-codex`, we skip clap parsing
    // and go straight to run_with_agent with the appropriate agent name.
    if let Some(argv0) = std::env::args().next()
        && let Some(agent) = run::extract_agent_from_argv0(&argv0)
    {
        let command_args: Vec<String> = std::env::args().skip(1).collect();

        // If no command args provided, default to running the agent name as the command
        // (e.g., `zellai-codex` with no args runs `codex`)
        let command = if command_args.is_empty() {
            vec![agent.clone()]
        } else {
            command_args
        };

        if let Err(msg) = run::run_with_agent(&agent, command) {
            eprintln!("{msg}");
            std::process::exit(1);
        }
        return;
    }

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
        Commands::Wrap { agent, command } => {
            if let Err(msg) = run::run_with_agent(&agent, command) {
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
        Commands::Log {
            pane,
            workspace,
            lines,
            follow,
        } => {
            if let Err(msg) = log::run(&pane, workspace.as_deref(), follow, lines) {
                eprintln!("{msg}");
                std::process::exit(1);
            }
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "zellai", &mut std::io::stdout());
        }
    }
}
