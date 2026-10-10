use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceCommands,
}

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    /// Initialize Spacer in the current directory (or $SPACER_ROOT)
    Init,
}

pub fn run(args: WorkspaceArgs, ws: &mut spacer_core::Workspace) -> Result<()> {
    match args.command {
        WorkspaceCommands::Init => {
            if ws.initialize()? {
                println!("Initialized Spacer at {}", ws.root().display());
            } else {
                println!("Spacer is already initialized at {}", ws.root().display());
            }
            Ok(())
        }
    }
}
