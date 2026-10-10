use serde::{Deserialize, Serialize};
use spacer_core::{ports::Registry, Change, Error, Project, Result, Space, WorkspaceSnapshot};
use std::path::{Path, PathBuf};

/// JSON persistence only. Domain policy and directory lifecycle belong to the service.
pub struct JsonRegistry {
    root: PathBuf,
}
impl JsonRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    fn path(&self) -> PathBuf {
        self.root.join(".spacer/config.json")
    }
}
impl Registry for JsonRegistry {
    fn load(&self) -> Result<WorkspaceSnapshot> {
        let path = self.path();
        let data = match std::fs::read_to_string(&path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(WorkspaceSnapshot::default())
            }
            Err(e) => return Err(persistence(&path, e)),
        };
        let config: StoredConfig =
            serde_json::from_str(&data).map_err(|e| persistence(&path, e))?;
        Ok(config.into())
    }
    fn save(&self, snapshot: &WorkspaceSnapshot) -> Result<()> {
        let path = self.path();
        let data = serde_json::to_string_pretty(&StoredConfig::from(snapshot.clone()))
            .map_err(|e| persistence(&path, e))?;
        std::fs::create_dir_all(self.root.join(".spacer")).map_err(|e| persistence(&path, e))?;
        std::fs::write(&path, data).map_err(|e| persistence(&path, e))
    }
    fn is_initialized(&self) -> Result<bool> {
        self.root
            .join(".spacer")
            .try_exists()
            .map_err(|e| persistence(&self.path(), e))
    }
}
fn persistence(path: &Path, error: impl std::fmt::Display) -> Error {
    Error::Persistence(format!("configuration '{}': {error}", path.display()))
}

// Serialization is an adapter detail. Preserve the existing JSON schema.
#[derive(Serialize, Deserialize)]
struct StoredConfig {
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
            spaces: c.spaces.into_iter().map(Into::into).collect(),
            projects: c.projects.into_iter().map(Into::into).collect(),
            changes: c.changes.into_iter().map(Into::into).collect(),
            active_space: c.active_space,
            active_project: c.active_project,
        }
    }
}

#[derive(Serialize, Deserialize)]
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
