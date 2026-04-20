use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A Change tracks a unit of work (e.g. a git branch) within a Project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub name: String,
    pub project: String,
    pub space: String,
}

impl Change {
    /// Returns the derived worktree path: `<project_path>/<name>`.
    ///
    /// Requires looking up the project path from the workspace; this helper
    /// accepts it as a parameter so `Change` stays a plain data type.
    pub fn worktree_path(&self, project_path: &Path) -> PathBuf {
        project_path.join(&self.name)
    }
}
