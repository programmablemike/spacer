use anyhow::Result;
use clap::{Args, Subcommand};
use tabled::settings::Style;
use tabled::Table;

use crate::cli::mapper;

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
        /// Show legacy metadata tracking entries instead of Git worktrees
        #[arg(long)]
        tracked: bool,
        #[arg(long)]
        project: Option<String>,
        /// Defaults to active space
        #[arg(long)]
        space: Option<String>,
    },
    /// Finish (remove) a change
    Finish {
        /// Remove only a legacy metadata tracking entry, preserving Git worktrees
        #[arg(long)]
        tracked: bool,
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
    flag.or_else(|| ws.active_space()).ok_or_else(|| {
        anyhow::anyhow!(
            "no space specified — use --space or set one with `spacer space use <name>`"
        )
    })
}

pub fn run(args: ChangeArgs, ws: &mut spacer_core::Workspace) -> Result<()> {
    match args.command {
        ChangeCommands::Start {
            name,
            project,
            space,
        } => {
            let space = resolve_space(space, ws)?;
            let checkout = ws.start_worktree(&name, &project, &space, "HEAD")?;
            println!("Started change '{}' at {}", name, checkout.path.display());
        }
        ChangeCommands::List {
            project,
            space,
            tracked,
        } => {
            let space = space.or_else(|| ws.active_space());
            if tracked {
                let changes = ws.changes(project.as_deref(), space.as_deref());
                println!(
                    "{}",
                    Table::new(mapper::change_rows(&changes)).with(Style::sharp())
                );
            } else {
                let mut rows = Vec::new();
                for p in ws
                    .projects(space.as_deref())
                    .into_iter()
                    .filter(|p| project.as_ref().is_none_or(|name| name == &p.name))
                {
                    for tree in ws
                        .worktrees(&p.name, &p.space)?
                        .into_iter()
                        .filter(|w| !w.is_primary)
                    {
                        rows.push(mapper::WorktreeRow {
                            space: p.space.clone(),
                            project: p.name.clone(),
                            branch: tree
                                .branch
                                .unwrap_or_else(|| "(detached; use Git directly)".into()),
                            path: tree.path.display().to_string(),
                        });
                    }
                }
                if rows.is_empty() {
                    println!("No changes found.");
                } else {
                    println!("{}", Table::new(rows).with(Style::sharp()));
                }
            }
        }
        ChangeCommands::Finish {
            tracked,
            name,
            project,
            space,
        } => {
            let space = resolve_space(space, ws)?;
            if tracked {
                ws.finish_change(&name, &project, &space)?;
                println!("Removed legacy tracking '{name}'; Git files preserved");
            } else {
                ws.close_worktree(&name, &project, &space)?;
                println!("Finished change '{name}'; branch retained");
            }
        }
    }

    Ok(())
}
