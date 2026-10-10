use crate::{Result, WorkspaceSnapshot, Worktree};
use std::path::{Path, PathBuf};

/// Persists domain data. Implementations do not enforce lifecycle policy.
pub trait Registry: Send + Sync {
    fn load(&self) -> Result<WorkspaceSnapshot>;
    fn save(&self, snapshot: &WorkspaceSnapshot) -> Result<()>;
    fn is_initialized(&self) -> Result<bool>;
}

/// Directory effects needed by workspace use cases. Deliberately has no recursive delete.
pub trait Filesystem: Send + Sync {
    fn normalize(&self, path: &Path) -> Result<PathBuf>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
}

/// Git mechanisms, independent of registration and active context.
pub trait Git: Send + Sync {
    fn initialize(&self, repository: &Path) -> Result<()>;
    fn add_worktree(&self, repository: &Path, branch: &str, path: &Path, base: &str) -> Result<()>;
    fn worktrees(&self, repository: &Path) -> Result<Vec<Worktree>>;
    fn remove_worktree(&self, repository: &Path, path: &Path) -> Result<()>;
}
