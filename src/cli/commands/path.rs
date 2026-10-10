use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct PathArgs {
    #[command(subcommand)]
    pub command: PathCommands,
}

#[derive(Subcommand)]
pub enum PathCommands {
    /// Print the filesystem path of a space
    Space {
        /// Name of the space (defaults to active space)
        name: Option<String>,
    },
    /// Print the filesystem path of a project
    Project {
        /// Name of the project (defaults to active project)
        name: Option<String>,
        /// Space the project belongs to (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
    /// Print the filesystem path of a change (git worktree)
    Change {
        /// Name of the change
        name: String,
        /// Project the change belongs to (defaults to active project)
        #[arg(long)]
        project: Option<String>,
        /// Space the project belongs to (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
}

pub fn run(args: PathArgs, ws: &spacer_core::Workspace) -> Result<()> {
    let path = match args.command {
        PathCommands::Space { name } => {
            let name = name
                .or_else(|| ws.active_space())
                .ok_or_else(|| anyhow::anyhow!("specify a space or select one"))?;
            ws.space_path(&name)?
        }
        PathCommands::Project { name, space } => {
            let space = space
                .or_else(|| ws.active_space())
                .ok_or_else(|| anyhow::anyhow!("specify a space or select one"))?;
            let name = name
                .or_else(|| ws.active_project_in(&space))
                .ok_or_else(|| anyhow::anyhow!("specify a project or select one in this space"))?;
            ws.project_path(&name, &space)?
        }
        PathCommands::Change {
            name,
            project,
            space,
        } => {
            let space = space
                .or_else(|| ws.active_space())
                .ok_or_else(|| anyhow::anyhow!("specify a space or select one"))?;
            let project = project
                .or_else(|| ws.active_project_in(&space))
                .ok_or_else(|| anyhow::anyhow!("specify a project or select one in this space"))?;
            ws.change_path(&name, &project, &space)?
        }
    };
    print!("{}", path.display());
    Ok(())
}
