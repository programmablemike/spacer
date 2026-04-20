use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::adapters::ConfigAdapter;
use crate::store::Backend;
use crate::{Change, Project, Space};

/// A high-level facade over any [`Backend`].
///
/// The CLI and TUI always use the default `Workspace<ConfigAdapter>` obtained
/// via [`Workspace::open`]. Alternative backends (e.g. an in-memory store for
/// tests) can be injected via [`Workspace::new`].
pub struct Workspace<B: Backend> {
    pub root: PathBuf,
    backend: B,
}

// ---------------------------------------------------------------------------
// Constructors for the default (filesystem) backend
// ---------------------------------------------------------------------------

impl Workspace<ConfigAdapter> {
    /// Returns a reference to the underlying [`Config`].
    ///
    /// Only available on the default `ConfigAdapter` backend. Code that needs
    /// to work with any backend should use the store trait methods instead.
    pub fn config(&self) -> &crate::adapters::config::Config {
        self.backend.config()
    }

    /// Opens a workspace at `root`, loading any existing config from disk.
    pub fn open(root: &Path) -> Result<Self> {
        let backend = ConfigAdapter::open(root)?;
        Ok(Self { root: root.to_path_buf(), backend })
    }

    /// Initializes a fresh workspace at `root` and writes an empty config to disk.
    pub fn init(root: &Path) -> Result<Self> {
        let backend = ConfigAdapter::init(root)?;
        Ok(Self { root: root.to_path_buf(), backend })
    }

    /// Returns `true` if the `.spacer` directory already exists at `root`.
    pub fn is_initialized(root: &Path) -> bool {
        ConfigAdapter::is_initialized(root)
    }
}

// ---------------------------------------------------------------------------
// Generic constructor (any backend)
// ---------------------------------------------------------------------------

impl<B: Backend> Workspace<B> {
    /// Creates a workspace wrapping the given backend.
    pub fn new(root: PathBuf, backend: B) -> Self {
        Self { root, backend }
    }

    /// Returns a reference to the underlying backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    // -------------------------------------------------------------------------
    // Persistence
    // -------------------------------------------------------------------------

    pub fn save(&self) -> Result<()> {
        self.backend.save()
    }

    // -------------------------------------------------------------------------
    // Spaces
    // -------------------------------------------------------------------------

    pub fn create_space(&mut self, name: &str, path: Option<PathBuf>) -> Result<Space> {
        self.backend.create_space(name, path)
    }

    pub fn spaces(&self) -> &[Space] {
        self.backend.spaces()
    }

    pub fn delete_space(&mut self, name: &str) -> Result<()> {
        self.backend.delete_space(name)
    }

    pub fn active_space(&self) -> Option<String> {
        self.backend.active_space()
    }

    pub fn set_active_space(&mut self, name: &str) -> Result<()> {
        self.backend.set_active_space(name)
    }

    // -------------------------------------------------------------------------
    // Projects
    // -------------------------------------------------------------------------

    pub fn create_project(&mut self, name: &str, space: &str, path: Option<PathBuf>) -> Result<Project> {
        self.backend.create_project(name, space, path)
    }

    pub fn projects(&self, space: Option<&str>) -> Vec<&Project> {
        self.backend.projects(space)
    }

    pub fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        self.backend.delete_project(name, space)
    }

    pub fn active_project(&self) -> Option<String> {
        self.backend.active_project()
    }

    pub fn set_active_project(&mut self, name: &str, space: &str) -> Result<()> {
        self.backend.set_active_project(name, space)
    }

    // -------------------------------------------------------------------------
    // Changes
    // -------------------------------------------------------------------------

    pub fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change> {
        self.backend.start_change(name, project, space)
    }

    pub fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change> {
        self.backend.changes(project, space)
    }

    pub fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()> {
        self.backend.finish_change(name, project, space)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryAdapter;

    fn mem_ws() -> Workspace<MemoryAdapter> {
        Workspace::new(PathBuf::from("/test"), MemoryAdapter::default())
    }

    // -------------------------------------------------------------------------
    // Spaces
    // -------------------------------------------------------------------------

    #[test]
    fn create_and_list_spaces() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_space("beta", None).unwrap();
        let names: Vec<_> = ws.spaces().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn create_space_duplicate_is_error() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        assert!(ws.create_space("alpha", None).is_err());
    }

    #[test]
    fn delete_space() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.delete_space("alpha").unwrap();
        assert!(ws.spaces().is_empty());
    }

    #[test]
    fn delete_missing_space_is_error() {
        let mut ws = mem_ws();
        assert!(ws.delete_space("nope").is_err());
    }

    #[test]
    fn active_space_roundtrip() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        assert!(ws.active_space().is_none());
        ws.set_active_space("alpha").unwrap();
        assert_eq!(ws.active_space().as_deref(), Some("alpha"));
    }

    #[test]
    fn set_active_space_unknown_is_error() {
        let mut ws = mem_ws();
        assert!(ws.set_active_space("ghost").is_err());
    }

    // -------------------------------------------------------------------------
    // Projects
    // -------------------------------------------------------------------------

    #[test]
    fn create_and_list_projects() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.create_project("api", "alpha", None).unwrap();
        let names: Vec<_> = ws.projects(Some("alpha")).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["web", "api"]);
    }

    #[test]
    fn create_project_in_unknown_space_is_error() {
        let mut ws = mem_ws();
        assert!(ws.create_project("web", "ghost", None).is_err());
    }

    #[test]
    fn create_project_duplicate_is_error() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        assert!(ws.create_project("web", "alpha", None).is_err());
    }

    #[test]
    fn project_list_filtered_by_space() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_space("beta", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.create_project("web", "beta", None).unwrap();
        assert_eq!(ws.projects(Some("alpha")).len(), 1);
        assert_eq!(ws.projects(Some("beta")).len(), 1);
        assert_eq!(ws.projects(None).len(), 2);
    }

    #[test]
    fn delete_project() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.delete_project("web", "alpha").unwrap();
        assert!(ws.projects(Some("alpha")).is_empty());
    }

    #[test]
    fn active_project_roundtrip() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        assert!(ws.active_project().is_none());
        ws.set_active_project("web", "alpha").unwrap();
        assert_eq!(ws.active_project().as_deref(), Some("web"));
    }

    // -------------------------------------------------------------------------
    // Changes
    // -------------------------------------------------------------------------

    #[test]
    fn start_and_list_changes() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.start_change("feat/login", "web", "alpha").unwrap();
        ws.start_change("fix/typo", "web", "alpha").unwrap();
        assert_eq!(ws.changes(Some("web"), Some("alpha")).len(), 2);
    }

    #[test]
    fn start_change_in_unknown_project_is_error() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        assert!(ws.start_change("feat/x", "ghost", "alpha").is_err());
    }

    #[test]
    fn start_change_duplicate_is_error() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.start_change("feat/login", "web", "alpha").unwrap();
        assert!(ws.start_change("feat/login", "web", "alpha").is_err());
    }

    #[test]
    fn finish_change() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.start_change("feat/login", "web", "alpha").unwrap();
        ws.finish_change("feat/login", "web", "alpha").unwrap();
        assert!(ws.changes(None, None).is_empty());
    }

    #[test]
    fn finish_missing_change_is_error() {
        let mut ws = mem_ws();
        assert!(ws.finish_change("ghost", "web", "alpha").is_err());
    }

    #[test]
    fn changes_filtered_by_project_and_space() {
        let mut ws = mem_ws();
        ws.create_space("alpha", None).unwrap();
        ws.create_project("web", "alpha", None).unwrap();
        ws.create_project("api", "alpha", None).unwrap();
        ws.start_change("feat/a", "web", "alpha").unwrap();
        ws.start_change("feat/b", "api", "alpha").unwrap();
        assert_eq!(ws.changes(Some("web"), None).len(), 1);
        assert_eq!(ws.changes(Some("api"), None).len(), 1);
        assert_eq!(ws.changes(None, Some("alpha")).len(), 2);
    }
}
