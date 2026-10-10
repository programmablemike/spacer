use anyhow::Result;
use clap::{CommandFactory, Parser};

mod cli;
mod composition;
mod tui;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Commands::Completions(args)) = cli.command {
        return cli::commands::completions::run(args, &mut Cli::command());
    }
    // Shell integration needs no workspace and must work outside an initialized root.
    if let Some(Commands::Shell(args)) = cli.command {
        return cli::commands::shell::run(args);
    }
    let mut workspace =
        composition::open_workspace(matches!(&cli.command, Some(Commands::Workspace(_))))?;
    match cli.command {
        Some(Commands::Workspace(args)) => cli::commands::workspace::run(args, &mut workspace),
        Some(Commands::Space(args)) => cli::commands::space::run(args, &mut workspace),
        Some(Commands::Project(args)) => cli::commands::project::run(args, &mut workspace),
        Some(Commands::Path(args)) => cli::commands::path::run(args, &workspace),
        Some(Commands::Go(args)) => cli::commands::go::run(args, &workspace),
        Some(Commands::Change(args)) => cli::commands::change::run(args, &mut workspace),
        Some(Commands::Shell(_) | Commands::Completions(_)) => unreachable!(),
        None => tui::run(workspace.view()?),
    }
}
