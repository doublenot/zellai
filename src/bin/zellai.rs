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
    Init,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => {
            println!("zellai init: not yet implemented");
            std::process::exit(1);
        }
    }
}
