use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommands,
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project inside a space
    Create {
        /// Name of the project
        name: String,
        /// Space to place the project in (defaults to active space)
        #[arg(long)]
        space: Option<String>,
        /// Directory path (defaults to <space_path>/<name>)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// List projects (optionally filtered by space)
    List {
        /// Filter by space name (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
    /// Delete a project
    Delete {
        /// Name of the project
        name: String,
        /// Space the project belongs to (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
}

fn resolve_space(flag: Option<String>, ws: &spacer_core::Workspace) -> Result<String> {
    flag.or_else(|| ws.active_space())
        .ok_or_else(|| anyhow::anyhow!(
            "no space specified — use --space or set one with `spacer space use <name>`"
        ))
}

pub fn run(args: ProjectArgs) -> Result<()> {
    let root = spacer_core::default_root()?;
    let mut ws = spacer_core::Workspace::open(&root)?;

    match args.command {
        ProjectCommands::Create { name, space, path } => {
            let space = resolve_space(space, &ws)?;
            ws.create_project(&name, &space, path)?;
            ws.save()?;
            println!("Created project '{}' in space '{}'", name, space);
        }
        ProjectCommands::List { space } => {
            let space = space.or_else(|| ws.active_space());
            let projects = ws.projects(space.as_deref());
            if projects.is_empty() {
                println!("No projects found.");
            } else {
                for p in projects {
                    println!("{:20} {:20} {}", p.space, p.name, p.path.display());
                }
            }
        }
        ProjectCommands::Delete { name, space } => {
            let space = resolve_space(space, &ws)?;
            ws.delete_project(&name, &space)?;
            ws.save()?;
            println!("Deleted project '{}' from space '{}'", name, space);
        }
    }

    Ok(())
}
