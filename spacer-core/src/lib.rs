use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SPACER_DIR: &str = ".spacer";
const CONFIG_FILE: &str = "config.json";

/// A Space is a named collection of related Projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
}

/// A Project lives inside a Space and points to a code directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub space: String,
    pub path: PathBuf,
}

/// A Change tracks a unit of work (e.g. a git branch) within a Project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub name: String,
    pub project: String,
    pub space: String,
}

/// The on-disk configuration stored in `<root>/.spacer/config.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub spaces: Vec<Space>,
    pub projects: Vec<Project>,
    pub changes: Vec<Change>,
}

/// Returns the default Spacer root: `$SPACER_ROOT` env var or the current directory.
pub fn default_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("SPACER_ROOT") {
        return Ok(PathBuf::from(root));
    }
    std::env::current_dir().context("failed to determine current directory")
}

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

/// A handle to an opened Spacer workspace, providing all domain operations.
pub struct Workspace {
    pub root: PathBuf,
    config: Config,
}

impl Workspace {
    /// Opens a workspace at `root`, loading any existing config from disk.
    pub fn open(root: &Path) -> Result<Self> {
        let config = load_config(root)?;
        Ok(Self { root: root.to_path_buf(), config })
    }

    /// Initializes a fresh workspace at `root` and writes an empty config to disk.
    pub fn init(root: &Path) -> Result<Self> {
        let config = Config::default();
        save_config(root, &config)?;
        Ok(Self { root: root.to_path_buf(), config })
    }

    /// Returns `true` if the `.spacer` directory already exists at `root`.
    pub fn is_initialized(root: &Path) -> bool {
        spacer_dir(root).exists()
    }

    /// Persists the current in-memory state to disk.
    pub fn save(&self) -> Result<()> {
        save_config(&self.root, &self.config)
    }

    /// Returns a reference to the underlying config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    // -------------------------------------------------------------------------
    // Spaces
    // -------------------------------------------------------------------------

    /// Creates a new space. `path` defaults to `<root>/<name>` if not provided.
    pub fn create_space(&mut self, name: impl Into<String>, path: Option<PathBuf>) -> Result<Space> {
        let name = name.into();
        if self.config.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' already exists", name);
        }
        let path = path.unwrap_or_else(|| self.root.join(&name));
        let space = Space { name, path };
        self.config.spaces.push(space.clone());
        Ok(space)
    }

    pub fn spaces(&self) -> &[Space] {
        &self.config.spaces
    }

    /// Deletes a space by name. Returns an error if no such space exists.
    pub fn delete_space(&mut self, name: &str) -> Result<()> {
        let before = self.config.spaces.len();
        self.config.spaces.retain(|s| s.name != name);
        if self.config.spaces.len() == before {
            bail!("space '{}' not found", name);
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Projects
    // -------------------------------------------------------------------------

    /// Creates a new project inside `space`. `path` defaults to `<space_path>/<name>`.
    pub fn create_project(
        &mut self,
        name: impl Into<String>,
        space: &str,
        path: Option<PathBuf>,
    ) -> Result<Project> {
        let name = name.into();
        let space_path = self
            .config
            .spaces
            .iter()
            .find(|s| s.name == space)
            .map(|s| s.path.clone())
            .ok_or_else(|| anyhow::anyhow!("space '{}' not found", space))?;
        if self.config.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' already exists in space '{}'", name, space);
        }
        let path = path.unwrap_or_else(|| space_path.join(&name));
        let project = Project { name, space: space.to_string(), path };
        self.config.projects.push(project.clone());
        Ok(project)
    }

    /// Lists projects, optionally filtered by space name.
    pub fn projects(&self, space: Option<&str>) -> Vec<&Project> {
        self.config
            .projects
            .iter()
            .filter(|p| space.map_or(true, |s| p.space == s))
            .collect()
    }

    /// Deletes a project by name and space. Returns an error if not found.
    pub fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        let before = self.config.projects.len();
        self.config.projects.retain(|p| !(p.name == name && p.space == space));
        if self.config.projects.len() == before {
            bail!("project '{}' in space '{}' not found", name, space);
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Changes
    // -------------------------------------------------------------------------

    /// Starts (creates) a new change within a project.
    pub fn start_change(
        &mut self,
        name: impl Into<String>,
        project: &str,
        space: &str,
    ) -> Result<Change> {
        let name = name.into();
        if !self.config.projects.iter().any(|p| p.name == project && p.space == space) {
            bail!("project '{}' in space '{}' not found", project, space);
        }
        if self.config.changes.iter().any(|c| c.name == name && c.project == project && c.space == space) {
            bail!("change '{}' already exists in project '{}'", name, project);
        }
        let change = Change { name, project: project.to_string(), space: space.to_string() };
        self.config.changes.push(change.clone());
        Ok(change)
    }

    /// Lists changes, optionally filtered by project and/or space.
    pub fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change> {
        self.config
            .changes
            .iter()
            .filter(|c| {
                project.map_or(true, |p| c.project == p)
                    && space.map_or(true, |s| c.space == s)
            })
            .collect()
    }

    /// Finishes (removes) a change. Returns an error if not found.
    pub fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()> {
        let before = self.config.changes.len();
        self.config.changes.retain(|c| !(c.name == name && c.project == project && c.space == space));
        if self.config.changes.len() == before {
            bail!("change '{}' not found", name);
        }
        Ok(())
    }
}
