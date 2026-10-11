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
    /// Manage commit names and emails
    Identity(commands::accounts::IdentityArgs),
    /// Manage authenticated GitHub connections
    Connector(commands::accounts::ConnectorArgs),
    /// Review and provision projects from GitHub
    Setup(commands::accounts::SetupArgs),
    /// Manage the Spacer workspace
    Workspace(commands::workspace::WorkspaceArgs),
    /// Manage spaces
    Space(commands::space::SpaceArgs),
    /// Manage projects
    Project(commands::project::ProjectArgs),
    /// Manage changes
    Change(commands::change::ChangeArgs),
    /// Print a validated filesystem path
    Path(commands::path::PathArgs),
    /// Resolve space/project/change navigation
    Go(commands::go::GoArgs),
    /// Generate shell completions
    Completions(commands::completions::CompletionsArgs),
    /// Enter a managed shell or print shell integration
    Shell(commands::shell::ShellArgs),
}
