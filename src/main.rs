use anyhow::Result;
use clap::Parser;

mod cli;
mod composition;
mod tui;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Shell integration needs no workspace and must work outside an initialized root.
    if let Some(Commands::Shell(args)) = cli.command {
        return cli::commands::shell::run(args);
    }
    let mut workspace = composition::open_workspace()?;
    match cli.command {
        Some(Commands::Workspace(args)) => cli::commands::workspace::run(args, &mut workspace),
        Some(Commands::Space(args)) => cli::commands::space::run(args, &mut workspace),
        Some(Commands::Project(args)) => cli::commands::project::run(args, &mut workspace),
        Some(Commands::Change(args)) => cli::commands::change::run(args, &mut workspace),
        Some(Commands::Shell(_)) => unreachable!(),
        None => tui::run(workspace.snapshot()),
    }
}
