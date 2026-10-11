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
    fn is_empty_or_missing(&self, path: &Path) -> Result<bool>;
    fn is_dir(&self, path: &Path) -> Result<bool>;
    fn normalize(&self, path: &Path) -> Result<PathBuf>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
}

/// Git mechanisms, independent of registration and active context.
pub trait Git: Send + Sync {
    fn commit_identity(&self, _repository: &Path) -> Result<(Option<String>, Option<String>)> {
        Err(crate::Error::Git(
            "authorship inspection is unavailable".into(),
        ))
    }
    fn apply_identity(&self, _repository: &Path, _name: &str, _email: &str) -> Result<()> {
        Err(crate::Error::Git(
            "authorship configuration is unavailable".into(),
        ))
    }
    fn validate_repository(&self, repository: &Path) -> Result<()>;
    fn is_clean(&self, worktree: &Path) -> Result<bool>;
    fn initialize(&self, repository: &Path) -> Result<()>;
    fn add_worktree(&self, repository: &Path, branch: &str, path: &Path, base: &str) -> Result<()>;
    fn worktrees(&self, repository: &Path) -> Result<Vec<Worktree>>;
    fn remove_worktree(&self, repository: &Path, path: &Path) -> Result<()>;
}
