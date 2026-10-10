use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tabled::settings::Style;
use tabled::Table;

use crate::cli::mapper;

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
    /// Register an existing primary Git repository without changing files
    Register {
        name: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        space: Option<String>,
    },
    /// Unregister a project, preserving files, branches, and worktrees
    Unregister {
        name: String,
        #[arg(long)]
        space: Option<String>,
    },
    /// List projects (optionally filtered by space)
    List {
        /// Filter by space name (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
    /// Remove a project registration without tracked changes, preserving files
    Delete {
        /// Name of the project
        name: String,
        /// Space the project belongs to (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
    /// Set the active project (used when --project is omitted)
    Use {
        /// Name of the project to activate
        name: String,
        /// Space the project belongs to (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
    /// Show the currently active project
    Current,
    /// Print the path of a project (use with shell integration to cd into it)
    Go {
        /// Name of the project (defaults to active project)
        name: Option<String>,
        /// Space the project belongs to (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
}

fn resolve_space(flag: Option<String>, ws: &spacer_core::Workspace) -> Result<String> {
    flag.or_else(|| ws.active_space()).ok_or_else(|| {
        anyhow::anyhow!(
            "no space specified — use --space or set one with `spacer space use <name>`"
        )
    })
}

fn resolve_project(
    flag: Option<String>,
    space: &str,
    ws: &spacer_core::Workspace,
) -> Result<String> {
    flag.or_else(|| ws.active_project_in(space)).ok_or_else(|| {
        anyhow::anyhow!(
            "no project specified — pass a name or set one with `spacer project use <name>`"
        )
    })
}

pub fn run(args: ProjectArgs, ws: &mut spacer_core::Workspace) -> Result<()> {
    match args.command {
        ProjectCommands::Create { name, space, path } => {
            let space = resolve_space(space, ws)?;
            ws.create_git_project(&name, &space, path)?;
            println!("Created project '{}' in space '{}'", name, space);
        }
        ProjectCommands::Register { name, path, space } => {
            let space = resolve_space(space, ws)?;
            ws.register_project(&name, &space, path)?;
            println!("Registered project '{space}/{name}'");
        }
        ProjectCommands::List { space } => {
            let space = space.or_else(|| ws.active_space());
            let active_space = ws.active_space();
            let active = ws.active_project();
            let projects = ws.projects(space.as_deref());
            if projects.is_empty() {
                println!("No projects found.");
            } else {
                let rows =
                    mapper::project_rows(&projects, active_space.as_deref().zip(active.as_deref()));
                println!("{}", Table::new(rows).with(Style::sharp()));
            }
        }
        ProjectCommands::Delete { name, space } | ProjectCommands::Unregister { name, space } => {
            let space = resolve_space(space, ws)?;
            ws.delete_project(&name, &space)?;
            println!(
                "Unregistered project '{space}/{name}'; files, branches, and worktrees preserved"
            );
        }
        ProjectCommands::Use { name, space } => {
            let space = resolve_space(space, ws)?;
            ws.set_active_project(&name, &space)?;
            println!("Now using project '{}'", name);
        }
        ProjectCommands::Current => match ws.active_project() {
            Some(name) => println!("{}", name),
            None => println!("No active project. Set one with `spacer project use <name>`."),
        },
        ProjectCommands::Go { name, space } => {
            let space = resolve_space(space, ws)?;
            let name = resolve_project(name, &space, ws)?;
            print!("{}", ws.project_path(&name, &space)?.display());
        }
    }

    Ok(())
}
