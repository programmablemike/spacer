use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub space: String,
    pub path: PathBuf,
}

/// A metadata tracking entry, not proof of an existing Git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub name: String,
    pub project: String,
    pub space: String,
}
impl Change {
    pub fn worktree_path(&self, project_path: &Path) -> PathBuf {
        project_path.join(&self.name)
    }
}

/// Owned domain data usable by any presentation without exposing a storage format.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub bindings: Vec<crate::connections::ProjectBinding>,
    pub setup_defaults: Vec<crate::connections::SetupDefault>,
    pub spaces: Vec<Space>,
    pub projects: Vec<Project>,
    pub changes: Vec<Change>,
    pub active_space: Option<String>,
    pub active_project: Option<String>,
}

/// Session overrides supplied by the composition root, never read from the environment here.
#[derive(Debug, Clone, Default)]
pub struct ContextOverrides {
    pub space: Option<String>,
    pub project: Option<String>,
}

/// Observed Git state. Location is authoritative even when it differs from the branch name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectWorktree {
    pub space: String,
    pub project: String,
    pub worktree: Worktree,
}
#[derive(Debug, Clone)]
pub struct WorkspaceView {
    pub registry: WorkspaceSnapshot,
    pub worktrees: Vec<ProjectWorktree>,
}
