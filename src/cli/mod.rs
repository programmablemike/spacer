pub mod commands;
pub mod mapper;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "spacer", about = "Manage multiple code projects", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage the Spacer workspace
    Workspace(commands::workspace::WorkspaceArgs),
    /// Manage spaces
    Space(commands::space::SpaceArgs),
    /// Manage projects
    Project(commands::project::ProjectArgs),
    /// Manage changes
    Change(commands::change::ChangeArgs),
    /// Print shell integration code
    Shell(commands::shell::ShellArgs),
}
