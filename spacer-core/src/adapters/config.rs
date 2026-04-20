use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{Change, Project, Space};
use crate::store::{Backend, ChangeStore, ProjectStore, SpaceStore};

const SPACER_DIR: &str = ".spacer";
const CONFIG_FILE: &str = "config.json";

/// The on-disk configuration stored in `<root>/.spacer/config.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub spaces: Vec<Space>,
    pub projects: Vec<Project>,
    pub changes: Vec<Change>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_space: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_project: Option<String>,
}

/// Backend adapter that persists workspace state as JSON under `<root>/.spacer/config.json`
/// and creates space/project directories on the filesystem.
pub struct ConfigAdapter {
    pub root: PathBuf,
    config: Config,
}

impl ConfigAdapter {
    /// Opens an existing workspace, loading config from disk (or returning an empty
    /// config if none exists yet).
    pub fn open(root: &Path) -> Result<Self> {
        let config = load_config(root)?;
        Ok(Self { root: root.to_path_buf(), config })
    }

    /// Initializes a fresh workspace, writing an empty config to disk immediately.
    pub fn init(root: &Path) -> Result<Self> {
        let config = Config::default();
        save_config(root, &config)?;
        Ok(Self { root: root.to_path_buf(), config })
    }

    /// Returns `true` if the `.spacer` directory already exists at `root`.
    pub fn is_initialized(root: &Path) -> bool {
        root.join(SPACER_DIR).exists()
    }

    /// Returns a reference to the underlying config.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Store trait implementations
// ---------------------------------------------------------------------------

impl SpaceStore for ConfigAdapter {
    fn create_space(&mut self, name: &str, path: Option<PathBuf>) -> Result<Space> {
        if self.config.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' already exists", name);
        }
        let path = path.unwrap_or_else(|| self.root.join(name));
        let space = Space { name: name.to_string(), path };
        space.create_dir()?;
        self.config.spaces.push(space.clone());
        Ok(space)
    }

    fn spaces(&self) -> &[Space] {
        &self.config.spaces
    }

    fn delete_space(&mut self, name: &str) -> Result<()> {
        let before = self.config.spaces.len();
        self.config.spaces.retain(|s| s.name != name);
        if self.config.spaces.len() == before {
            bail!("space '{}' not found", name);
        }
        Ok(())
    }

    fn active_space(&self) -> Option<String> {
        if let Ok(s) = std::env::var("SPACER_SPACE") {
            if !s.is_empty() {
                return Some(s);
            }
        }
        self.config.active_space.clone()
    }

    fn set_active_space(&mut self, name: &str) -> Result<()> {
        if !self.config.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' not found", name);
        }
        self.config.active_space = Some(name.to_string());
        Ok(())
    }
}

impl ProjectStore for ConfigAdapter {
    fn create_project(&mut self, name: &str, space: &str, path: Option<PathBuf>) -> Result<Project> {
        let space_path = self.config.spaces
            .iter()
            .find(|s| s.name == space)
            .map(|s| s.path.clone())
            .ok_or_else(|| anyhow::anyhow!("space '{}' not found", space))?;
        if self.config.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' already exists in space '{}'", name, space);
        }
        let path = path.unwrap_or_else(|| space_path.join(name));
        let project = Project { name: name.to_string(), space: space.to_string(), path };
        project.create_dir()?;
        self.config.projects.push(project.clone());
        Ok(project)
    }

    fn projects(&self, space: Option<&str>) -> Vec<&Project> {
        self.config.projects
            .iter()
            .filter(|p| space.map_or(true, |s| p.space == s))
            .collect()
    }

    fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        let before = self.config.projects.len();
        self.config.projects.retain(|p| !(p.name == name && p.space == space));
        if self.config.projects.len() == before {
            bail!("project '{}' in space '{}' not found", name, space);
        }
        Ok(())
    }

    fn active_project(&self) -> Option<String> {
        if let Ok(p) = std::env::var("SPACER_PROJECT") {
            if !p.is_empty() {
                return Some(p);
            }
        }
        self.config.active_project.clone()
    }

    fn set_active_project(&mut self, name: &str, space: &str) -> Result<()> {
        if !self.config.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' in space '{}' not found", name, space);
        }
        self.config.active_project = Some(name.to_string());
        Ok(())
    }
}

impl ChangeStore for ConfigAdapter {
    fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change> {
        if !self.config.projects.iter().any(|p| p.name == project && p.space == space) {
            bail!("project '{}' in space '{}' not found", project, space);
        }
        if self.config.changes.iter().any(|c| c.name == name && c.project == project && c.space == space) {
            bail!("change '{}' already exists in project '{}'", name, project);
        }
        let change = Change { name: name.to_string(), project: project.to_string(), space: space.to_string() };
        self.config.changes.push(change.clone());
        Ok(change)
    }

    fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change> {
        self.config.changes
            .iter()
            .filter(|c| {
                project.map_or(true, |p| c.project == p)
                    && space.map_or(true, |s| c.space == s)
            })
            .collect()
    }

    fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()> {
        let before = self.config.changes.len();
        self.config.changes.retain(|c| !(c.name == name && c.project == project && c.space == space));
        if self.config.changes.len() == before {
            bail!("change '{}' not found", name);
        }
        Ok(())
    }
}

impl Backend for ConfigAdapter {
    fn save(&self) -> Result<()> {
        save_config(&self.root, &self.config)
    }
}

// ---------------------------------------------------------------------------
// Persistence helpers (private)
// ---------------------------------------------------------------------------

fn spacer_dir(root: &Path) -> PathBuf {
    root.join(SPACER_DIR)
}

fn config_path(root: &Path) -> PathBuf {
    spacer_dir(root).join(CONFIG_FILE)
}

fn load_config(root: &Path) -> Result<Config> {
    let path = config_path(root);
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data).context("failed to parse config")
}

fn save_config(root: &Path, config: &Config) -> Result<()> {
    let dir = spacer_dir(root);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;
    let path = config_path(root);
    let data = serde_json::to_string_pretty(config).context("failed to serialize config")?;
    std::fs::write(&path, data)
        .with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{ProjectStore, SpaceStore};

    fn adapter(root: &Path) -> ConfigAdapter {
        ConfigAdapter::open(root).unwrap()
    }

    #[test]
    fn creates_space_directory_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = adapter(tmp.path());
        a.create_space("myspace", None).unwrap();
        assert!(tmp.path().join("myspace").is_dir());
    }

    #[test]
    fn creates_project_directory_under_space() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = adapter(tmp.path());
        a.create_space("alpha", None).unwrap();
        a.create_project("web", "alpha", None).unwrap();
        assert!(tmp.path().join("alpha").join("web").is_dir());
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let mut a = adapter(tmp.path());
            a.create_space("alpha", None).unwrap();
            a.create_project("web", "alpha", None).unwrap();
            a.save().unwrap();
        }
        // Re-open and verify state was persisted
        let a = adapter(tmp.path());
        assert_eq!(a.spaces().len(), 1);
        assert_eq!(a.spaces()[0].name, "alpha");
        assert_eq!(a.projects(Some("alpha")).len(), 1);
        assert_eq!(a.projects(Some("alpha"))[0].name, "web");
    }

    #[test]
    fn is_initialized_reflects_spacer_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!ConfigAdapter::is_initialized(tmp.path()));
        ConfigAdapter::init(tmp.path()).unwrap();
        assert!(ConfigAdapter::is_initialized(tmp.path()));
    }

    #[test]
    fn open_with_no_config_returns_empty_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let a = adapter(tmp.path());
        assert!(a.spaces().is_empty());
        assert!(a.projects(None).is_empty());
    }
}
