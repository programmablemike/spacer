use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

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
            let spaces = ws.spaces();
            let active = ws.active_space();
            if spaces.is_empty() {
                println!("No spaces found. Create one with `spacer space create <name>`.");
            } else {
                for space in spaces {
                    let marker = if active.as_deref() == Some(space.name.as_str()) { '*' } else { ' ' };
                    println!("{} {:20} {}", marker, space.name, space.path.display());
                }
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
    }

    Ok(())
}
