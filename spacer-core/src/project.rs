use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A Project lives inside a Space and points to a code directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub space: String,
    pub path: PathBuf,
}

impl Project {
    /// Returns `true` if the project's directory exists on disk.
    pub fn exists(&self) -> bool {
        self.path.is_dir()
    }

    /// Creates the project's directory on disk (idempotent).
    pub fn create_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.path)
            .with_context(|| format!("failed to create project directory '{}'", self.path.display()))
    }
}
