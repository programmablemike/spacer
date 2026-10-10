use anyhow::{Context, Result};
use spacer_core::{ports::Filesystem, ContextOverrides, Workspace};
use spacer_data::{GitCli, JsonRegistry, LocalFilesystem};
use std::path::PathBuf;

pub fn open_workspace() -> Result<Workspace> {
    let root = match std::env::var("SPACER_ROOT") {
        Ok(root) if !root.is_empty() => PathBuf::from(root),
        Ok(_) => anyhow::bail!("SPACER_ROOT must not be empty"),
        Err(_) => std::env::current_dir().context("failed to determine current directory")?,
    };
    let root = LocalFilesystem.normalize(&root)?;
    let context = ContextOverrides {
        space: std::env::var("SPACER_SPACE").ok(),
        project: std::env::var("SPACER_PROJECT").ok(),
    };
    let registry = JsonRegistry::new(&root);
    Ok(Workspace::open(
        root,
        registry,
        LocalFilesystem,
        GitCli,
        context,
    )?)
}
