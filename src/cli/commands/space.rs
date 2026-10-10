use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tabled::settings::Style;
use tabled::Table;

use crate::cli::mapper;

#[derive(Args)]
pub struct SpaceArgs {
    #[command(subcommand)]
    pub command: SpaceCommands,
}

#[derive(Subcommand)]
pub enum SpaceCommands {
    /// Create a new space
    Create {
        /// Name of the space
        name: String,
        /// Directory path for the space (defaults to <root>/<name>)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// List all spaces
    List,
    /// Unregister an empty space, preserving its directory
    Unregister { name: String },
    /// Deprecated alias for unregister; preserves files
    Delete {
        /// Name of the space to delete
        name: String,
    },
    /// Set the active space (used when --space is omitted)
    Use {
        /// Name of the space to activate
        name: String,
    },
    /// Show the currently active space
    Current,
    /// Print the path of a space (use with a shell function to cd into it)
    Go {
        /// Name of the space (defaults to active space)
        name: Option<String>,
    },
}

pub fn run(args: SpaceArgs, ws: &mut spacer_core::Workspace) -> Result<()> {
    match args.command {
        SpaceCommands::Create { name, path } => {
            ws.create_space(&name, path)?;
            println!("Created space '{}'", name);
        }
        SpaceCommands::List => {
            let active = ws.active_space();
            let spaces = ws.spaces();
            if spaces.is_empty() {
                println!("No spaces found. Create one with `spacer space create <name>`.");
            } else {
                let rows = mapper::space_rows(spaces, active.as_deref());
                println!("{}", Table::new(rows).with(Style::sharp()));
            }
        }
        SpaceCommands::Delete { name } | SpaceCommands::Unregister { name } => {
            ws.delete_space(&name)?;
            println!("Unregistered space '{name}'; files preserved");
        }
        SpaceCommands::Use { name } => {
            ws.set_active_space(&name)?;
            println!("Now using space '{}'", name);
        }
        SpaceCommands::Current => match ws.active_space() {
            Some(name) => println!("{}", name),
            None => println!("No active space. Set one with `spacer space use <name>`."),
        },
        SpaceCommands::Go { name } => {
            let name = name.or_else(|| ws.active_space()).ok_or_else(|| {
                anyhow::anyhow!(
                    "no space specified — pass a name or set one with `spacer space use <name>`"
                )
            })?;
            let space = ws
                .spaces()
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| anyhow::anyhow!("space '{}' not found", name))?;
            print!("{}", space.path.display());
        }
    }

    Ok(())
}
