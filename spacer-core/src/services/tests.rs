use super::super::ports::{Filesystem, Git, Registry};
use super::*;
use std::sync::{Arc, Mutex};

fn mem_ws() -> Workspace {
    Workspace::open(
        PathBuf::from("/test"),
        MemoryRegistry::default(),
        FakeFilesystem::default(),
        FakeGit::default(),
        ContextOverrides::default(),
    )
    .unwrap()
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
    let names: Vec<_> = ws
        .projects(Some("alpha"))
        .iter()
        .map(|p| p.name.as_str())
        .collect();
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

#[derive(Clone, Default)]
struct MemoryRegistry {
    state: Arc<Mutex<WorkspaceSnapshot>>,
    fail: Arc<Mutex<bool>>,
}
impl Registry for MemoryRegistry {
    fn load(&self) -> Result<WorkspaceSnapshot> {
        Ok(self.state.lock().unwrap().clone())
    }
    fn save(&self, state: &WorkspaceSnapshot) -> Result<()> {
        if *self.fail.lock().unwrap() {
            return Err(Error::Persistence("injected save failure".into()));
        }
        *self.state.lock().unwrap() = state.clone();
        Ok(())
    }
    fn is_initialized(&self) -> Result<bool> {
        Ok(false)
    }
}
#[derive(Clone, Default)]
struct FakeFilesystem {
    paths: Arc<Mutex<Vec<PathBuf>>>,
    fail: bool,
}
impl Filesystem for FakeFilesystem {
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        if self.fail {
            return Err(Error::Filesystem("injected directory failure".into()));
        }
        self.paths.lock().unwrap().push(path.into());
        Ok(())
    }
}
#[derive(Clone, Default)]
struct FakeGit {
    calls: Arc<Mutex<Vec<String>>>,
    trees: Vec<Worktree>,
    fail: bool,
}
impl Git for FakeGit {
    fn initialize(&self, _: &Path) -> Result<()> {
        self.calls.lock().unwrap().push("init".into());
        if self.fail {
            Err(Error::Git("injected Git failure".into()))
        } else {
            Ok(())
        }
    }
    fn add_worktree(&self, _: &Path, _: &str, _: &Path, _: &str) -> Result<()> {
        self.calls.lock().unwrap().push("add".into());
        Ok(())
    }
    fn worktrees(&self, _: &Path) -> Result<Vec<Worktree>> {
        Ok(self.trees.clone())
    }
    fn remove_worktree(&self, _: &Path, path: &Path) -> Result<()> {
        self.calls.lock().unwrap().push(path.display().to_string());
        Ok(())
    }
}

#[test]
fn validation_happens_before_directory_or_git_effects() {
    let fs = FakeFilesystem::default();
    let git = FakeGit::default();
    let mut ws = Workspace::open(
        "/test".into(),
        MemoryRegistry::default(),
        fs.clone(),
        git.clone(),
        ContextOverrides::default(),
    )
    .unwrap();
    assert!(matches!(
        ws.create_git_project("web", "missing", None),
        Err(Error::NotFound(_))
    ));
    assert!(fs.paths.lock().unwrap().is_empty());
    assert!(git.calls.lock().unwrap().is_empty());
    ws.create_space("alpha", None).unwrap();
    ws.create_git_project("web", "alpha", None).unwrap();
    let paths = fs.paths.lock().unwrap().len();
    let calls = git.calls.lock().unwrap().len();
    assert!(matches!(
        ws.create_git_project("web", "alpha", None),
        Err(Error::AlreadyExists(_))
    ));
    assert_eq!(fs.paths.lock().unwrap().len(), paths);
    assert_eq!(git.calls.lock().unwrap().len(), calls);
}
#[test]
fn failed_save_does_not_publish_new_state() {
    let registry = MemoryRegistry::default();
    let mut ws = Workspace::open(
        "/test".into(),
        registry.clone(),
        FakeFilesystem::default(),
        FakeGit::default(),
        ContextOverrides::default(),
    )
    .unwrap();
    ws.create_space("alpha", None).unwrap();
    let before = ws.snapshot();
    *registry.fail.lock().unwrap() = true;
    assert!(matches!(
        ws.create_project("web", "alpha", None),
        Err(Error::Persistence(_))
    ));
    assert_eq!(ws.snapshot(), before);
    assert_eq!(*registry.state.lock().unwrap(), before);
}
#[test]
fn failed_directory_creation_is_not_persisted() {
    let registry = MemoryRegistry::default();
    let mut ws = Workspace::open(
        "/test".into(),
        registry.clone(),
        FakeFilesystem {
            fail: true,
            ..Default::default()
        },
        FakeGit::default(),
        ContextOverrides::default(),
    )
    .unwrap();
    assert!(ws.create_space("alpha", None).is_err());
    assert!(ws.spaces().is_empty());
    assert!(registry.state.lock().unwrap().spaces.is_empty());
}
#[test]
fn git_failure_does_not_register_project() {
    let mut ws = Workspace::open(
        "/test".into(),
        MemoryRegistry::default(),
        FakeFilesystem::default(),
        FakeGit {
            fail: true,
            ..Default::default()
        },
        ContextOverrides::default(),
    )
    .unwrap();
    ws.create_space("alpha", None).unwrap();
    assert!(matches!(
        ws.create_git_project("web", "alpha", None),
        Err(Error::Git(_))
    ));
    assert!(ws.projects(None).is_empty());
}
#[test]
fn tracking_and_unregistering_never_call_git() {
    let git = FakeGit::default();
    let mut ws = Workspace::open(
        "/test".into(),
        MemoryRegistry::default(),
        FakeFilesystem::default(),
        git.clone(),
        ContextOverrides::default(),
    )
    .unwrap();
    ws.create_space("alpha", None).unwrap();
    ws.create_project("web", "alpha", None).unwrap();
    ws.start_change("feature", "web", "alpha").unwrap();
    ws.finish_change("feature", "web", "alpha").unwrap();
    ws.delete_project("web", "alpha").unwrap();
    ws.delete_space("alpha").unwrap();
    assert!(git.calls.lock().unwrap().is_empty());
}
#[test]
fn session_context_is_effective_but_never_persisted() {
    let registry = MemoryRegistry::default();
    let mut ws = Workspace::open(
        "/test".into(),
        registry.clone(),
        FakeFilesystem::default(),
        FakeGit::default(),
        ContextOverrides {
            space: Some("session".into()),
            project: Some("override".into()),
        },
    )
    .unwrap();
    ws.create_space("saved", None).unwrap();
    ws.set_active_space("saved").unwrap();
    assert_eq!(ws.snapshot().active_space.as_deref(), Some("session"));
    assert_eq!(ws.active_project().as_deref(), Some("override"));
    assert_eq!(
        registry.state.lock().unwrap().active_space.as_deref(),
        Some("saved")
    );
    assert!(registry.state.lock().unwrap().active_project.is_none());
}
#[test]
fn close_uses_observed_path_and_rejects_primary() {
    let git = FakeGit {
        trees: vec![
            Worktree {
                path: "/primary".into(),
                branch: Some("main".into()),
                is_primary: true,
            },
            Worktree {
                path: "/elsewhere/custom".into(),
                branch: Some("feature".into()),
                is_primary: false,
            },
        ],
        ..Default::default()
    };
    let mut ws = Workspace::open(
        "/test".into(),
        MemoryRegistry::default(),
        FakeFilesystem::default(),
        git.clone(),
        ContextOverrides::default(),
    )
    .unwrap();
    ws.create_space("alpha", None).unwrap();
    ws.create_project("web", "alpha", None).unwrap();
    assert!(matches!(
        ws.close_worktree("main", "web", "alpha"),
        Err(Error::InvalidOperation(_))
    ));
    ws.close_worktree("feature", "web", "alpha").unwrap();
    assert_eq!(*git.calls.lock().unwrap(), vec!["/elsewhere/custom"]);
}
