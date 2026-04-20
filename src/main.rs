use anyhow::Result;
use clap::Parser;

mod cli;
mod tui;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Workspace(args)) => cli::commands::workspace::run(args),
        Some(Commands::Space(args)) => cli::commands::space::run(args),
        Some(Commands::Project(args)) => cli::commands::project::run(args),
        Some(Commands::Change(args)) => cli::commands::change::run(args),
        Some(Commands::Shell(args)) => cli::commands::shell::run(args),
        None => tui::run(),
    }
}
