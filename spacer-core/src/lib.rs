//! Framework-independent domain model, application services, and outbound ports.
pub mod error;
pub mod model;
pub mod ports;
pub mod services;

pub use error::{Error, Result};
pub use model::{Change, ContextOverrides, Project, Space, WorkspaceSnapshot, Worktree};
pub use services::Workspace;
