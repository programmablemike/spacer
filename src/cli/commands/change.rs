use anyhow::Result;
use clap::{Args, Subcommand};

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
        /// Space the project lives in
        #[arg(long)]
        space: String,
    },
    /// List changes (optionally filtered by project/space)
    List {
        #[arg(long)]
        project: Option<String>,
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
        /// Space the project lives in
        #[arg(long)]
        space: String,
    },
}

pub fn run(args: ChangeArgs) -> Result<()> {
    let root = spacer_core::default_root()?;
    let mut ws = spacer_core::Workspace::open(&root)?;

    match args.command {
        ChangeCommands::Start { name, project, space } => {
            ws.start_change(&name, &project, &space)?;
            ws.save()?;
            println!("Started change '{}' in project '{}/{}'", name, space, project);
        }
        ChangeCommands::List { project, space } => {
            let changes = ws.changes(project.as_deref(), space.as_deref());
            if changes.is_empty() {
                println!("No changes found.");
            } else {
                for c in changes {
                    println!("{:20} {:20} {}", c.space, c.project, c.name);
                }
            }
        }
        ChangeCommands::Finish { name, project, space } => {
            ws.finish_change(&name, &project, &space)?;
            ws.save()?;
            println!("Finished change '{}'", name);
        }
    }

    Ok(())
}
