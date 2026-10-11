use std::path::{Path, PathBuf};

use spacer_core::{ports::Filesystem, ContextOverrides, Error, Result, Workspace};
use spacer_data::{GitCli, JsonRegistry, LocalFilesystem};

pub fn absolute_path(value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(Error::InvalidOperation(
            "choose an absolute directory path".into(),
        ));
    }
    Ok(path)
}

pub fn open(root: &Path, initialize: bool) -> Result<Workspace> {
    if !root.is_absolute() || !LocalFilesystem.is_dir(root)? {
        return Err(Error::InvalidOperation(
            "choose an existing workspace directory".into(),
        ));
    }
    let root = LocalFilesystem.normalize(root)?;
    let registry = JsonRegistry::new(&root);
    let mut workspace = Workspace::open(
        root,
        registry,
        LocalFilesystem,
        GitCli,
        ContextOverrides::default(),
    )?;
    if initialize {
        workspace.initialize()?;
    } else if !workspace
        .root()
        .join(".spacer/config.json")
        .try_exists()
        .map_err(|error| Error::Filesystem(error.to_string()))?
    {
        return Err(Error::InvalidOperation(
            "this folder is not a workspace; choose Create workspace to initialize it".into(),
        ));
    }
    Ok(workspace)
}

pub fn existing_directory(path: PathBuf) -> Result<PathBuf> {
    if !LocalFilesystem.is_dir(&path)? {
        return Err(Error::NotFound(
            "directory is missing; refresh and inspect the workspace".into(),
        ));
    }
    Ok(path)
}

pub fn connections() -> Result<spacer_core::connections::Connections> {
    Ok(spacer_core::connections::Connections::new(
        spacer_data::JsonCatalog::new(spacer_data::catalog_directory()?),
        std::sync::Arc::new(spacer_data::Github::new(std::sync::Arc::new(
            spacer_data::NativeSecrets,
        ))?),
    ))
}
