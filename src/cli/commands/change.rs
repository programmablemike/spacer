use anyhow::Result;
use clap::{Args, Subcommand};
use tabled::{Table, Tabled};
use tabled::settings::Style;

#[derive(Tabled)]
struct ChangeRow {
    #[tabled(rename = "SPACE")]
    space: String,
    #[tabled(rename = "PROJECT")]
    project: String,
    #[tabled(rename = "NAME")]
    name: String,
}

#[derive(Args)]
pub struct ChangeArgs {
    #[command(subcommand)]
    pub command: ChangeCommands,
}

#[derive(Subcommand)]
pub enum ChangeCommands {
    /// Start a new change in a project
    Start {
        /// Name of the change (e.g. a branch name)
        name: String,
        /// Project the change belongs to
        #[arg(long)]
        project: String,
        /// Space the project lives in (defaults to active space)
        #[arg(long)]
        space: Option<String>,
    },
    /// List changes (optionally filtered by project/space)
    List {
        #[arg(long)]
        project: Option<String>,
        /// Defaults to active space
        #[arg(long)]
        space: Option<String>,
    },
    /// Finish (remove) a change
    Finish {
        /// Name of the change
        name: String,
        /// Project the change belongs to
        #[arg(long)]
        project: String,
        /// Space the project lives in (defaults to active space)
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

pub fn run(args: ChangeArgs) -> Result<()> {
    let root = spacer_core::default_root()?;
    let mut ws = spacer_core::Workspace::open(&root)?;

    match args.command {
        ChangeCommands::Start { name, project, space } => {
            let space = resolve_space(space, &ws)?;
            ws.start_change(&name, &project, &space)?;
            ws.save()?;
            println!("Started change '{}' in project '{}/{}'", name, space, project);
        }
        ChangeCommands::List { project, space } => {
            let space = space.or_else(|| ws.active_space());
            let changes = ws.changes(project.as_deref(), space.as_deref());
            if changes.is_empty() {
                println!("No changes found.");
            } else {
                let rows: Vec<ChangeRow> = changes.iter().map(|c| ChangeRow {
                    space: c.space.clone(),
                    project: c.project.clone(),
                    name: c.name.clone(),
                }).collect();
                println!("{}", Table::new(rows).with(Style::sharp()));
            }
        }
        ChangeCommands::Finish { name, project, space } => {
            let space = resolve_space(space, &ws)?;
            ws.finish_change(&name, &project, &space)?;
            ws.save()?;
            println!("Finished change '{}'", name);
        }
    }

    Ok(())
}
