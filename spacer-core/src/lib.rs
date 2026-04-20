use anyhow::{Context, Result};
use std::path::PathBuf;

mod adapters;
mod change;
mod project;
mod space;
mod workspace;
pub mod store;

pub use adapters::config::Config;
pub use adapters::{ConfigAdapter, GitAdapter, MemoryAdapter};
pub use change::Change;
pub use project::Project;
pub use space::Space;
pub use store::{Backend, ChangeStore, ProjectStore, SpaceStore};
pub use workspace::Workspace;

/// Returns the default Spacer root: `$SPACER_ROOT` env var or the current directory.
pub fn default_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("SPACER_ROOT") {
        return Ok(PathBuf::from(root));
    }
    std::env::current_dir().context("failed to determine current directory")
}
