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

pub fn run(args: WorkspaceArgs) -> Result<()> {
    match args.command {
        WorkspaceCommands::Init => {
            let root = spacer_core::default_root()?;

            if spacer_core::Workspace::is_initialized(&root) {
                println!("Spacer is already initialized at {}", root.display());
                return Ok(());
            }

            spacer_core::Workspace::init(&root)?;
            println!("Initialized Spacer at {}", root.display());
            Ok(())
        }
    }
}
