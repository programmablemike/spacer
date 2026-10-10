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
    fn is_empty_or_missing(&self, _: &Path) -> Result<bool> {
        Ok(true)
    }
    fn is_dir(&self, _: &Path) -> Result<bool> {
        Ok(true)
    }
    fn normalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(path.into())
    }
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
    fn validate_repository(&self, _: &Path) -> Result<()> {
        Ok(())
    }
    fn is_clean(&self, _: &Path) -> Result<bool> {
        Ok(true)
    }
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
    let mut initial = mem_ws();
    initial.create_space("session", None).unwrap();
    initial.create_project("override", "session", None).unwrap();
    *registry.state.lock().unwrap() = initial.snapshot();
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

fn reopen(state: WorkspaceSnapshot, context: ContextOverrides) -> Result<Workspace> {
    Workspace::open(
        "/test".into(),
        MemoryRegistry {
            state: Arc::new(Mutex::new(state)),
            ..Default::default()
        },
        FakeFilesystem::default(),
        FakeGit::default(),
        context,
    )
}

#[test]
fn parent_removal_refuses_tracked_children_without_mutation() {
    let mut ws = mem_ws();
    ws.create_space("alpha", None).unwrap();
    ws.create_project("web", "alpha", None).unwrap();
    ws.start_change("feat/login", "web", "alpha").unwrap();
    let before = ws.snapshot();
    assert!(matches!(
        ws.delete_space("alpha"),
        Err(Error::InvalidOperation(_))
    ));
    assert!(matches!(
        ws.delete_project("web", "alpha"),
        Err(Error::InvalidOperation(_))
    ));
    assert_eq!(before, ws.snapshot());
}

#[test]
fn project_identity_includes_space_and_switching_clears_selection() {
    let mut ws = mem_ws();
    for space in ["alpha", "beta"] {
        ws.create_space(space, None).unwrap();
        ws.create_project("web", space, None).unwrap();
    }
    ws.set_active_project("web", "alpha").unwrap();
    assert_eq!(ws.active_space().as_deref(), Some("alpha"));
    assert_eq!(ws.active_project_in("alpha").as_deref(), Some("web"));
    assert_eq!(ws.active_project_in("beta"), None);
    ws.delete_project("web", "beta").unwrap();
    assert_eq!(ws.active_project().as_deref(), Some("web"));
    ws.set_active_space("alpha").unwrap();
    assert_eq!(ws.active_project().as_deref(), Some("web"));
    ws.set_active_space("beta").unwrap();
    assert_eq!(ws.active_project(), None);
    ws.set_active_project("web", "alpha").unwrap();
    ws.delete_project("web", "alpha").unwrap();
    assert_eq!(ws.active_project(), None);
    ws.delete_space("alpha").unwrap();
    assert_eq!(ws.active_space(), None);
}

#[test]
fn session_space_does_not_retarget_saved_project() {
    let mut ws = mem_ws();
    for space in ["alpha", "beta"] {
        ws.create_space(space, None).unwrap();
        ws.create_project("web", space, None).unwrap();
    }
    ws.set_active_project("web", "alpha").unwrap();
    let snapshot = ws.snapshot();
    let overridden = reopen(
        snapshot.clone(),
        ContextOverrides {
            space: Some("beta".into()),
            project: None,
        },
    )
    .unwrap();
    assert_eq!(overridden.active_project(), None);
    let mut overridden = reopen(
        snapshot,
        ContextOverrides {
            space: None,
            project: Some("web".into()),
        },
    )
    .unwrap();
    overridden.set_active_space("beta").unwrap();
    assert_eq!(overridden.active_space().as_deref(), Some("alpha"));
    overridden.delete_project("web", "alpha").unwrap();
    assert_eq!(overridden.active_project(), None);
}

#[test]
fn stale_defaults_are_cleared_without_writing_on_open() {
    let mut ws = mem_ws();
    ws.create_space("alpha", None).unwrap();
    let mut state = ws.snapshot();
    state.active_space = Some("alpha".into());
    state.active_project = Some("missing".into());
    let registry = MemoryRegistry {
        state: Arc::new(Mutex::new(state.clone())),
        ..Default::default()
    };
    let mut reopened = Workspace::open(
        "/test".into(),
        registry.clone(),
        FakeFilesystem::default(),
        FakeGit::default(),
        ContextOverrides::default(),
    )
    .unwrap();
    assert_eq!(reopened.active_project(), None);
    assert_eq!(*registry.state.lock().unwrap(), state);
    reopened.create_space("beta", None).unwrap();
    assert_eq!(registry.state.lock().unwrap().active_project, None);
    state.active_space = Some("missing".into());
    let reopened = reopen(state, ContextOverrides::default()).unwrap();
    assert_eq!(reopened.active_space(), None);
    assert_eq!(reopened.active_project(), None);
}

#[test]
fn invalid_session_context_fails_instead_of_falling_back() {
    for context in [
        ContextOverrides {
            space: Some("missing".into()),
            project: None,
        },
        ContextOverrides {
            space: None,
            project: Some("missing".into()),
        },
    ] {
        assert!(matches!(
            reopen(WorkspaceSnapshot::default(), context),
            Err(Error::InvalidOperation(_))
        ));
    }
}

#[test]
fn invalid_loaded_relationships_and_duplicates_are_rejected() {
    let mut ws = mem_ws();
    ws.create_space("alpha", None).unwrap();
    ws.create_project("web", "alpha", None).unwrap();
    ws.start_change("feature", "web", "alpha").unwrap();
    for kind in 0..5 {
        let mut state = ws.snapshot();
        match kind {
            0 => state.spaces.clear(),
            1 => state.projects.clear(),
            2 => state.spaces.push(state.spaces[0].clone()),
            3 => state.projects.push(state.projects[0].clone()),
            _ => state.changes.push(state.changes[0].clone()),
        }
        assert!(matches!(
            reopen(state, ContextOverrides::default()),
            Err(Error::InvalidOperation(_))
        ));
    }
}

#[test]
fn invalid_names_are_rejected_before_side_effects() {
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
    ws.create_space("alpha", None).unwrap();
    ws.create_project("web", "alpha", None).unwrap();
    let before = ws.snapshot();
    let paths = fs.paths.lock().unwrap().len();
    for name in [
        "",
        " ",
        ".",
        "..",
        ".spacer",
        "../escape",
        "/absolute",
        "a/b",
        "a\\b",
        "NUL.txt",
        "com1",
        "a:",
        "trailing.",
        "trailing ",
        "line\nbreak",
    ] {
        assert!(ws.create_space(name, None).is_err(), "{name}");
        assert!(
            ws.create_git_project(name, "alpha", None).is_err(),
            "{name}"
        );
    }
    for name in [
        "",
        "../escape",
        "/absolute",
        "a//b",
        "a/./b",
        "a/../b",
        "a\\b",
        "HEAD",
        "a.lock",
        "a@{b",
        "a b",
        "-option",
        "a?b",
    ] {
        assert!(ws.start_change(name, "web", "alpha").is_err(), "{name}");
        assert!(
            ws.start_worktree(name, "web", "alpha", "HEAD").is_err(),
            "{name}"
        );
    }
    assert_eq!(ws.snapshot(), before);
    assert_eq!(fs.paths.lock().unwrap().len(), paths);
    assert!(git.calls.lock().unwrap().is_empty());
    ws.create_space("team space", None).unwrap();
    ws.start_change("feat/login", "web", "alpha").unwrap();
}

#[test]
fn failed_context_save_keeps_previous_pair() {
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
    ws.create_project("web", "alpha", None).unwrap();
    ws.set_active_project("web", "alpha").unwrap();
    ws.create_space("beta", None).unwrap();
    let before = ws.snapshot();
    *registry.fail.lock().unwrap() = true;
    assert!(ws.set_active_space("beta").is_err());
    assert!(ws.delete_project("web", "alpha").is_err());
    assert_eq!(ws.snapshot(), before);
    assert_eq!(*registry.state.lock().unwrap(), before);
}
