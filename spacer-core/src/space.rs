use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A Space is a named collection of related Projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
}

impl Space {
    /// Returns `true` if the space's directory exists on disk.
    pub fn exists(&self) -> bool {
        self.path.is_dir()
    }

    /// Creates the space's directory on disk (idempotent).
    pub fn create_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.path)
            .with_context(|| format!("failed to create space directory '{}'", self.path.display()))
    }
}
