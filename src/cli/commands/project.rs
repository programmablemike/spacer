use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tabled::{Table, Tabled};
use tabled::settings::Style;

#[derive(Tabled)]
struct ProjectRow {
    #[tabled(rename = " ")]
    active: char,
    #[tabled(rename = "SPACE")]
    space: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PATH")]
    path: String,
}

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
    flag.or_else(|| ws.active_space())
        .ok_or_else(|| anyhow::anyhow!(
            "no space specified — use --space or set one with `spacer space use <name>`"
        ))
}

fn resolve_project(flag: Option<String>, ws: &spacer_core::Workspace) -> Result<String> {
    flag.or_else(|| ws.active_project())
        .ok_or_else(|| anyhow::anyhow!(
            "no project specified — pass a name or set one with `spacer project use <name>`"
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
            let active = ws.active_project();
            let projects = ws.projects(space.as_deref());
            if projects.is_empty() {
                println!("No projects found.");
            } else {
                let rows: Vec<ProjectRow> = projects.iter().map(|p| ProjectRow {
                    active: if active.as_deref() == Some(p.name.as_str()) { '*' } else { ' ' },
                    space: p.space.clone(),
                    name: p.name.clone(),
                    path: p.path.display().to_string(),
                }).collect();
                println!("{}", Table::new(rows).with(Style::sharp()));
            }
        }
        ProjectCommands::Delete { name, space } => {
            let space = resolve_space(space, &ws)?;
            ws.delete_project(&name, &space)?;
            ws.save()?;
            println!("Deleted project '{}' from space '{}'", name, space);
        }
        ProjectCommands::Use { name, space } => {
            let space = resolve_space(space, &ws)?;
            ws.set_active_project(&name, &space)?;
            ws.save()?;
            println!("Now using project '{}'", name);
        }
        ProjectCommands::Current => {
            match ws.active_project() {
                Some(name) => println!("{}", name),
                None => println!("No active project. Set one with `spacer project use <name>`."),
            }
        }
        ProjectCommands::Go { name, space } => {
            let name = resolve_project(name, &ws)?;
            let space = resolve_space(space, &ws)?;
            let project = ws.projects(Some(&space))
                .into_iter()
                .find(|p| p.name == name)
                .ok_or_else(|| anyhow::anyhow!("project '{}' in space '{}' not found", name, space))?;
            print!("{}", project.path.display());
        }
    }

    Ok(())
}
