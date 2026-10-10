use serde::{Deserialize, Serialize};
use spacer_core::{ports::Registry, Change, Error, Project, Result, Space, WorkspaceSnapshot};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// JSON persistence only. Domain policy and directory lifecycle belong to the service.
pub struct JsonRegistry {
    root: PathBuf,
    baseline: Mutex<Option<Option<Vec<u8>>>>,
}
impl JsonRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            baseline: Mutex::new(None),
        }
    }
    fn path(&self) -> PathBuf {
        self.root.join(".spacer/config.json")
    }
}
impl Registry for JsonRegistry {
    fn load(&self) -> Result<WorkspaceSnapshot> {
        let path = self.path();
        if self
            .root
            .join(".spacer/state.json")
            .try_exists()
            .map_err(|e| persistence(&path, e))?
        {
            return Err(persistence(&path, "legacy state.json requires manual reconciliation; back up both files and see docs/how-to/recover-workspace.md before archiving state.json"));
        }
        let mut baseline = self
            .baseline
            .lock()
            .map_err(|_| persistence(&path, "registry lock poisoned"))?;
        *baseline = None;
        let data = read_config(&path)?;
        let state = if let Some(bytes) = &data {
            let config: StoredConfig =
                serde_json::from_slice(bytes).map_err(|e| persistence(&path, e))?;
            if config.schema_version > 1 {
                return Err(persistence(
                    &path,
                    "unsupported schema version; upgrade Spacer without modifying this file",
                ));
            }
            if config.spaces.iter().any(|s| !s.path.is_absolute())
                || config.projects.iter().any(|p| !p.path.is_absolute())
            {
                return Err(persistence(&path, "relative legacy paths are ambiguous; back up configuration and replace them with verified absolute paths before retrying"));
            }
            config.into()
        } else {
            WorkspaceSnapshot::default()
        };
        *baseline = Some(data);
        Ok(state)
    }
    fn save(&self, snapshot: &WorkspaceSnapshot) -> Result<()> {
        let path = self.path();
        let mut baseline = self
            .baseline
            .lock()
            .map_err(|_| persistence(&path, "registry lock poisoned"))?;
        let expected = baseline
            .as_ref()
            .ok_or_else(|| persistence(&path, "load successfully before saving"))?;
        let directory = self.root.join(".spacer");
        std::fs::create_dir_all(&directory).map_err(|e| persistence(&path, e))?;
        // Keep this inode stable: removing the lock file would permit competing locks.
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("registry.lock"))
            .map_err(|e| persistence(&path, e))?;
        lock.try_lock().map_err(|e| {
            persistence(
                &path,
                format!("another writer is active; retry the command: {e}"),
            )
        })?;
        if &read_config(&path)? != expected {
            return Err(persistence(
                &path,
                "registry changed since loading; reopen and retry (no metadata was overwritten)",
            ));
        }
        let data = serde_json::to_vec_pretty(&StoredConfig::from(snapshot.clone()))
            .map_err(|e| persistence(&path, e))?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(&directory).map_err(|e| persistence(&path, e))?;
        temporary
            .write_all(&data)
            .map_err(|e| persistence(&path, e))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|e| persistence(&path, e))?;
        temporary
            .persist(&path)
            .map_err(|e| persistence(&path, e))?;
        *baseline = Some(Some(data));
        // Flush the directory entry on Unix. On failure the replacement may already
        // be visible; callers must reopen instead of assuming a rollback.
        #[cfg(unix)]
        std::fs::File::open(&directory).and_then(|f| f.sync_all())
            .map_err(|e| persistence(&path, format!("replacement written but directory sync failed; reopen to inspect state: {e}")))?;
        Ok(())
    }
    fn is_initialized(&self) -> Result<bool> {
        self.path()
            .try_exists()
            .map_err(|e| persistence(&self.path(), e))
    }
}
fn read_config(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(data) => Ok(Some(data)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(persistence(path, e)),
    }
}
fn persistence(path: &Path, error: impl std::fmt::Display) -> Error {
    Error::Persistence(format!("configuration '{}': {error}", path.display()))
}

// Serialization is an adapter detail. Preserve the existing JSON schema.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConfig {
    #[serde(default)]
    schema_version: u32,
    spaces: Vec<StoredSpace>,
    projects: Vec<StoredProject>,
    changes: Vec<StoredChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_space: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_project: Option<String>,
}
impl From<StoredConfig> for WorkspaceSnapshot {
    fn from(c: StoredConfig) -> Self {
        Self {
            spaces: c.spaces.into_iter().map(Into::into).collect(),
            projects: c.projects.into_iter().map(Into::into).collect(),
            changes: c.changes.into_iter().map(Into::into).collect(),
            active_space: c.active_space,
            active_project: c.active_project,
        }
    }
}
impl From<WorkspaceSnapshot> for StoredConfig {
    fn from(c: WorkspaceSnapshot) -> Self {
        Self {
            schema_version: 1,
            spaces: c.spaces.into_iter().map(Into::into).collect(),
            projects: c.projects.into_iter().map(Into::into).collect(),
            changes: c.changes.into_iter().map(Into::into).collect(),
            active_space: c.active_space,
            active_project: c.active_project,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSpace {
    name: String,
    path: PathBuf,
}
impl From<StoredSpace> for Space {
    fn from(value: StoredSpace) -> Self {
        Self {
            name: value.name,
            path: value.path,
        }
    }
}
impl From<Space> for StoredSpace {
    fn from(value: Space) -> Self {
        Self {
            name: value.name,
            path: value.path,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredProject {
    name: String,
    space: String,
    path: PathBuf,
}
impl From<StoredProject> for Project {
    fn from(value: StoredProject) -> Self {
        Self {
            name: value.name,
            space: value.space,
            path: value.path,
        }
    }
}
impl From<Project> for StoredProject {
    fn from(value: Project) -> Self {
        Self {
            name: value.name,
            space: value.space,
            path: value.path,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredChange {
    name: String,
    project: String,
    space: String,
}
impl From<StoredChange> for Change {
    fn from(value: StoredChange) -> Self {
        Self {
            name: value.name,
            project: value.project,
            space: value.space,
        }
    }
}
impl From<Change> for StoredChange {
    fn from(value: Change) -> Self {
        Self {
            name: value.name,
            project: value.project,
            space: value.space,
        }
    }
}
