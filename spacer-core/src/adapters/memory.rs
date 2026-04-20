use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::{Change, Project, Space};
use crate::store::{Backend, ChangeStore, ProjectStore, SpaceStore};

/// An in-memory backend with no filesystem I/O and a no-op `save`.
///
/// Intended for unit tests. Unlike `ConfigAdapter`, it does not read
/// env vars for `active_space`/`active_project` so test state is fully
/// controlled by the caller.
#[derive(Default)]
pub struct MemoryAdapter {
    spaces: Vec<Space>,
    projects: Vec<Project>,
    changes: Vec<Change>,
    active_space: Option<String>,
    active_project: Option<String>,
}

impl SpaceStore for MemoryAdapter {
    fn create_space(&mut self, name: &str, path: Option<PathBuf>) -> Result<Space> {
        if self.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' already exists", name);
        }
        let path = path.unwrap_or_else(|| PathBuf::from(name));
        let space = Space { name: name.to_string(), path };
        self.spaces.push(space.clone());
        Ok(space)
    }

    fn spaces(&self) -> &[Space] {
        &self.spaces
    }

    fn delete_space(&mut self, name: &str) -> Result<()> {
        let before = self.spaces.len();
        self.spaces.retain(|s| s.name != name);
        if self.spaces.len() == before {
            bail!("space '{}' not found", name);
        }
        Ok(())
    }

    fn active_space(&self) -> Option<String> {
        self.active_space.clone()
    }

    fn set_active_space(&mut self, name: &str) -> Result<()> {
        if !self.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' not found", name);
        }
        self.active_space = Some(name.to_string());
        Ok(())
    }
}

impl ProjectStore for MemoryAdapter {
    fn create_project(&mut self, name: &str, space: &str, path: Option<PathBuf>) -> Result<Project> {
        let space_path = self.spaces
            .iter()
            .find(|s| s.name == space)
            .map(|s| s.path.clone())
            .ok_or_else(|| anyhow::anyhow!("space '{}' not found", space))?;
        if self.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' already exists in space '{}'", name, space);
        }
        let path = path.unwrap_or_else(|| space_path.join(name));
        let project = Project { name: name.to_string(), space: space.to_string(), path };
        self.projects.push(project.clone());
        Ok(project)
    }

    fn projects(&self, space: Option<&str>) -> Vec<&Project> {
        self.projects
            .iter()
            .filter(|p| space.map_or(true, |s| p.space == s))
            .collect()
    }

    fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        let before = self.projects.len();
        self.projects.retain(|p| !(p.name == name && p.space == space));
        if self.projects.len() == before {
            bail!("project '{}' in space '{}' not found", name, space);
        }
        Ok(())
    }

    fn active_project(&self) -> Option<String> {
        self.active_project.clone()
    }

    fn set_active_project(&mut self, name: &str, space: &str) -> Result<()> {
        if !self.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' in space '{}' not found", name, space);
        }
        self.active_project = Some(name.to_string());
        Ok(())
    }
}

impl ChangeStore for MemoryAdapter {
    fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change> {
        if !self.projects.iter().any(|p| p.name == project && p.space == space) {
            bail!("project '{}' in space '{}' not found", project, space);
        }
        if self.changes.iter().any(|c| c.name == name && c.project == project && c.space == space) {
            bail!("change '{}' already exists in project '{}'", name, project);
        }
        let change = Change {
            name: name.to_string(),
            project: project.to_string(),
            space: space.to_string(),
        };
        self.changes.push(change.clone());
        Ok(change)
    }

    fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change> {
        self.changes
            .iter()
            .filter(|c| {
                project.map_or(true, |p| c.project == p)
                    && space.map_or(true, |s| c.space == s)
            })
            .collect()
    }

    fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()> {
        let before = self.changes.len();
        self.changes.retain(|c| !(c.name == name && c.project == project && c.space == space));
        if self.changes.len() == before {
            bail!("change '{}' not found", name);
        }
        Ok(())
    }
}

impl Backend for MemoryAdapter {
    fn save(&self) -> Result<()> {
        Ok(()) // no-op: nothing to persist
    }
}
