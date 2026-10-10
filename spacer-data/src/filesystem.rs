use spacer_core::{ports::Filesystem, Error, Result};
use std::path::Path;

#[derive(Default)]
pub struct LocalFilesystem;
impl Filesystem for LocalFilesystem {
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path).map_err(|e| {
            Error::Filesystem(format!(
                "failed to create directory '{}': {e}",
                path.display()
            ))
        })
    }
}
