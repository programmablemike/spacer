use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tabled::{Table};
use tabled::settings::Style;

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
    /// Delete a space
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

pub fn run(args: SpaceArgs) -> Result<()> {
    let root = spacer_core::default_root()?;
    let mut ws = spacer_core::Workspace::open(&root)?;

    match args.command {
        SpaceCommands::Create { name, path } => {
            ws.create_space(&name, path)?;
            ws.save()?;
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
        SpaceCommands::Delete { name } => {
            ws.delete_space(&name)?;
            ws.save()?;
            println!("Deleted space '{}'", name);
        }
        SpaceCommands::Use { name } => {
            ws.set_active_space(&name)?;
            ws.save()?;
            println!("Now using space '{}'", name);
        }
        SpaceCommands::Current => {
            match ws.active_space() {
                Some(name) => println!("{}", name),
                None => println!("No active space. Set one with `spacer space use <name>`."),
            }
        }
        SpaceCommands::Go { name } => {
            let name = name
                .or_else(|| ws.active_space())
                .ok_or_else(|| anyhow::anyhow!(
                    "no space specified — pass a name or set one with `spacer space use <name>`"
                ))?;
            let space = ws.spaces()
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| anyhow::anyhow!("space '{}' not found", name))?;
            print!("{}", space.path.display());
        }
    }

    Ok(())
}
