use spacer_core::{ports::Filesystem, Error, Result};
use std::path::{Component, Path, PathBuf};

#[derive(Default)]
pub struct LocalFilesystem;
impl Filesystem for LocalFilesystem {
    fn normalize(&self, path: &Path) -> Result<PathBuf> {
        if path.as_os_str().is_empty() {
            return Err(Error::InvalidOperation("path must not be empty".into()));
        }
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()
                .map_err(|e| Error::Filesystem(e.to_string()))?
                .join(path)
        };
        let mut result = PathBuf::new();
        for part in absolute.components() {
            match part {
                Component::CurDir => {}
                Component::ParentDir => {
                    result.pop();
                }
                component => {
                    result.push(component.as_os_str());
                    match std::fs::symlink_metadata(&result) {
                        Ok(_) => {
                            result = std::fs::canonicalize(&result).map_err(|e| {
                                Error::Filesystem(format!(
                                    "cannot resolve '{}': {e}",
                                    result.display()
                                ))
                            })?
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => {
                            return Err(Error::Filesystem(format!(
                                "cannot inspect '{}': {e}",
                                result.display()
                            )))
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path).map_err(|e| {
            Error::Filesystem(format!(
                "failed to create directory '{}': {e}",
                path.display()
            ))
        })
    }
}
