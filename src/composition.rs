use anyhow::{Context, Result};
use spacer_core::{ports::Filesystem, ContextOverrides, Workspace};
use spacer_data::{GitCli, JsonRegistry, LocalFilesystem};
use std::path::PathBuf;

pub fn open_workspace(initialize: bool) -> Result<Workspace> {
    let cwd = std::env::current_dir().context("failed to determine current directory")?;
    let explicit = std::env::var_os("SPACER_ROOT");
    let root = if let Some(root) = explicit {
        anyhow::ensure!(!root.is_empty(), "SPACER_ROOT must not be empty");
        LocalFilesystem.normalize(&PathBuf::from(root))?
    } else if initialize {
        LocalFilesystem.normalize(&cwd)?
    } else {
        let mut found = None;
        for ancestor in cwd.ancestors() {
            if ancestor.join(".spacer").try_exists()? {
                found = Some(ancestor.to_path_buf());
                break;
            }
        }
        LocalFilesystem.normalize(
            &found.context("no workspace found; run `spacer workspace init` or set SPACER_ROOT")?,
        )?
    };
    if !initialize {
        anyhow::ensure!(
            root.join(".spacer/config.json").try_exists()?,
            "workspace '{}' is not initialized; run `spacer workspace init`",
            root.display()
        );
    }
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

pub fn connections() -> Result<spacer_core::connections::Connections> {
    Ok(spacer_core::connections::Connections::new(
        spacer_data::JsonCatalog::new(spacer_data::catalog_directory()?),
        std::sync::Arc::new(spacer_data::Github::new(std::sync::Arc::new(
            spacer_data::NativeSecrets,
        ))?),
    ))
}
pub fn open_root(root: PathBuf) -> Result<Workspace> {
    let registry = JsonRegistry::new(&root);
    Ok(Workspace::open(
        root,
        registry,
        LocalFilesystem,
        GitCli,
        ContextOverrides::default(),
    )?)
}
