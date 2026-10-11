use anyhow::Result;
use clap::{CommandFactory, Parser};

mod cli;
mod composition;
mod tui;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("--spacer-credential") {
        return spacer_data::credential_helper(&arguments[2..]).map_err(Into::into);
    }
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Identity(args)) => return cli::commands::accounts::identities(args),
        Some(Commands::Connector(args)) => return cli::commands::accounts::connectors(args),
        Some(Commands::Setup(args)) => return cli::commands::accounts::setup(args),
        _ => (),
    }
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
        Some(
            Commands::Shell(_)
            | Commands::Completions(_)
            | Commands::Identity(_)
            | Commands::Connector(_)
            | Commands::Setup(_),
        ) => unreachable!(),
        None => tui::run(workspace.view()?),
    }
}
