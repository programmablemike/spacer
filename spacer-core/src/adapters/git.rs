use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::store::{Backend, ChangeStore, ProjectStore, SpaceStore};
use crate::{Change, Project, Space};

const SPACER_DIR: &str = ".spacer";
const STATE_FILE: &str = "state.json";

// ---------------------------------------------------------------------------
// Active-context state (the only thing persisted by this adapter)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
struct ActiveState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_space: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_project: Option<String>,
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Backend adapter that manages:
/// - **Spaces** as plain filesystem directories
/// - **Projects** as git repositories (`git init`)
/// - **Changes** as git worktrees (`git worktree add / remove`)
///
/// State is derived from the filesystem on `open()`. Only the active
/// space/project context is persisted in `.spacer/state.json`.
///
/// ## Worktree layout
///
/// Changes are created as named worktrees inside the project directory:
///
/// ```text
/// workspace/
///   my-org/                   ← space
///     my-repo/                ← project (bare-ish repo root)
///       main/                 ← change worktree (git worktree add main -b main)
///       feature-login/        ← change worktree
/// ```
///
/// The project must have at least one commit before `start_change` can be called.
pub struct GitAdapter {
    root: PathBuf,
    spaces: Vec<Space>,
    projects: Vec<Project>,
    changes: Vec<Change>,
    active: ActiveState,
}

impl GitAdapter {
    /// Opens a workspace at `root`, scanning the filesystem and git state.
    pub fn open(root: &Path) -> Result<Self> {
        let active = load_state(root)?;
        let spaces = scan_spaces(root);
        let projects = scan_projects(&spaces);
        let changes = scan_changes(&projects);
        Ok(Self { root: root.to_path_buf(), spaces, projects, changes, active })
    }

    /// Initializes a new workspace at `root`, writing an empty state file.
    pub fn init(root: &Path) -> Result<Self> {
        let active = ActiveState::default();
        save_state(root, &active)?;
        Ok(Self {
            root: root.to_path_buf(),
            spaces: Vec::new(),
            projects: Vec::new(),
            changes: Vec::new(),
            active,
        })
    }

    /// Returns `true` if the `.spacer` directory already exists at `root`.
    pub fn is_initialized(root: &Path) -> bool {
        root.join(SPACER_DIR).exists()
    }
}

// ---------------------------------------------------------------------------
// Store trait implementations
// ---------------------------------------------------------------------------

impl SpaceStore for GitAdapter {
    fn create_space(&mut self, name: &str, path: Option<PathBuf>) -> Result<Space> {
        if self.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' already exists", name);
        }
        let path = path.unwrap_or_else(|| self.root.join(name));
        let space = Space { name: name.to_string(), path };
        space.create_dir()?;
        self.spaces.push(space.clone());
        Ok(space)
    }

    fn spaces(&self) -> &[Space] {
        &self.spaces
    }

    /// Removes the space directory. Fails if the space still contains projects.
    fn delete_space(&mut self, name: &str) -> Result<()> {
        let space = self.spaces.iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("space '{}' not found", name))?;
        if self.projects.iter().any(|p| p.space == name) {
            bail!("space '{}' still has projects — delete them first", name);
        }
        let path = space.path.clone();
        std::fs::remove_dir(&path).with_context(|| {
            format!("failed to remove '{}' — directory may not be empty", path.display())
        })?;
        self.spaces.retain(|s| s.name != name);
        Ok(())
    }

    fn active_space(&self) -> Option<String> {
        if let Ok(s) = std::env::var("SPACER_SPACE") {
            if !s.is_empty() {
                return Some(s);
            }
        }
        self.active.active_space.clone()
    }

    fn set_active_space(&mut self, name: &str) -> Result<()> {
        if !self.spaces.iter().any(|s| s.name == name) {
            bail!("space '{}' not found", name);
        }
        self.active.active_space = Some(name.to_string());
        Ok(())
    }
}

impl ProjectStore for GitAdapter {
    /// Creates a project directory and runs `git init` inside it.
    fn create_project(&mut self, name: &str, space: &str, path: Option<PathBuf>) -> Result<Project> {
        let space_path = self.spaces.iter()
            .find(|s| s.name == space)
            .map(|s| s.path.clone())
            .ok_or_else(|| anyhow::anyhow!("space '{}' not found", space))?;
        if self.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' already exists in space '{}'", name, space);
        }
        let path = path.unwrap_or_else(|| space_path.join(name));
        std::fs::create_dir_all(&path)
            .with_context(|| format!("failed to create '{}'", path.display()))?;
        run_git(&["init"], &path)
            .with_context(|| format!("failed to initialise git repo at '{}'", path.display()))?;
        let project = Project { name: name.to_string(), space: space.to_string(), path };
        self.projects.push(project.clone());
        Ok(project)
    }

    fn projects(&self, space: Option<&str>) -> Vec<&Project> {
        self.projects.iter()
            .filter(|p| space.map_or(true, |s| p.space == s))
            .collect()
    }

    /// Removes the project directory. Fails if the project has active changes.
    fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        let project = self.projects.iter()
            .find(|p| p.name == name && p.space == space)
            .ok_or_else(|| anyhow::anyhow!("project '{}' in space '{}' not found", name, space))?;
        if self.changes.iter().any(|c| c.project == name && c.space == space) {
            bail!("project '{}' has active changes — finish them first", name);
        }
        let path = project.path.clone();
        std::fs::remove_dir_all(&path)
            .with_context(|| format!("failed to remove '{}'", path.display()))?;
        self.projects.retain(|p| !(p.name == name && p.space == space));
        Ok(())
    }

    fn active_project(&self) -> Option<String> {
        if let Ok(p) = std::env::var("SPACER_PROJECT") {
            if !p.is_empty() {
                return Some(p);
            }
        }
        self.active.active_project.clone()
    }

    fn set_active_project(&mut self, name: &str, space: &str) -> Result<()> {
        if !self.projects.iter().any(|p| p.name == name && p.space == space) {
            bail!("project '{}' in space '{}' not found", name, space);
        }
        self.active.active_project = Some(name.to_string());
        Ok(())
    }
}

impl ChangeStore for GitAdapter {
    /// Creates a new worktree at `<project_path>/<name>` on a new branch `<name>`.
    ///
    /// Requires at least one commit in the repository.
    fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change> {
        let project_path = self.projects.iter()
            .find(|p| p.name == project && p.space == space)
            .map(|p| p.path.clone())
            .ok_or_else(|| anyhow::anyhow!("project '{}' in space '{}' not found", project, space))?;
        if self.changes.iter().any(|c| c.name == name && c.project == project && c.space == space) {
            bail!("change '{}' already exists in project '{}'", name, project);
        }
        run_git(&["worktree", "add", name, "-b", name], &project_path)
            .with_context(|| format!(
                "failed to add worktree '{}' — does the project have an initial commit?",
                name
            ))?;
        let change = Change {
            name: name.to_string(),
            project: project.to_string(),
            space: space.to_string(),
        };
        self.changes.push(change.clone());
        Ok(change)
    }

    fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change> {
        self.changes.iter()
            .filter(|c| {
                project.map_or(true, |p| c.project == p)
                    && space.map_or(true, |s| c.space == s)
            })
            .collect()
    }

    /// Removes the worktree directory and deregisters it from git.
    fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()> {
        let project_path = self.projects.iter()
            .find(|p| p.name == project && p.space == space)
            .map(|p| p.path.clone())
            .ok_or_else(|| anyhow::anyhow!("project '{}' in space '{}' not found", project, space))?;
        if !self.changes.iter().any(|c| c.name == name && c.project == project && c.space == space) {
            bail!("change '{}' not found in project '{}/{}'", name, space, project);
        }
        run_git(&["worktree", "remove", name], &project_path)
            .with_context(|| format!("failed to remove worktree '{}'", name))?;
        self.changes.retain(|c| !(c.name == name && c.project == project && c.space == space));
        Ok(())
    }
}

impl Backend for GitAdapter {
    /// Persists only the active space/project context; all other state is
    /// derived from the filesystem and git on the next `open()`.
    fn save(&self) -> Result<()> {
        save_state(&self.root, &self.active)
    }
}

// ---------------------------------------------------------------------------
// Filesystem scanning
// ---------------------------------------------------------------------------

fn scan_spaces(root: &Path) -> Vec<Space> {
    let mut spaces = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else { return spaces };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else { continue };
        if name.starts_with('.') {
            continue; // skip .spacer and other hidden dirs
        }
        spaces.push(Space { name, path });
    }
    spaces.sort_by(|a, b| a.name.cmp(&b.name));
    spaces
}

fn scan_projects(spaces: &[Space]) -> Vec<Project> {
    let mut projects = Vec::new();
    for space in spaces {
        let Ok(entries) = std::fs::read_dir(&space.path) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || !path.join(".git").exists() {
                continue;
            }
            let Ok(name) = entry.file_name().into_string() else { continue };
            projects.push(Project { name, space: space.name.clone(), path });
        }
    }
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

fn scan_changes(projects: &[Project]) -> Vec<Change> {
    let mut changes = Vec::new();
    for project in projects {
        let Ok(worktrees) = list_worktrees(&project.path) else { continue };
        // The first entry is always the main working tree — skip it.
        for wt in worktrees.into_iter().skip(1) {
            if let Some(branch) = wt.branch {
                changes.push(Change {
                    name: branch,
                    project: project.name.clone(),
                    space: project.space.clone(),
                });
            }
        }
    }
    changes
}

// ---------------------------------------------------------------------------
// Git helpers
// ---------------------------------------------------------------------------

struct WorktreeInfo {
    #[allow(dead_code)]
    path: PathBuf,
    branch: Option<String>,
}

fn list_worktrees(project_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = run_git(&["worktree", "list", "--porcelain"], project_path)?;
    Ok(parse_worktree_output(&output))
}

fn parse_worktree_output(output: &str) -> Vec<WorktreeInfo> {
    let mut entries: Vec<WorktreeInfo> = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    let flush = |path: Option<PathBuf>, branch: Option<String>, out: &mut Vec<WorktreeInfo>| {
        if let Some(p) = path {
            out.push(WorktreeInfo { path: p, branch });
        }
    };

    for line in output.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            flush(current_path.take(), current_branch.take(), &mut entries);
            current_path = Some(PathBuf::from(p));
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(b.to_string());
        }
        // detached HEAD entries have no branch — current_branch stays None
    }
    flush(current_path, current_branch, &mut entries);

    entries
}

fn run_git(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .context("failed to run git — is it installed and on PATH?")?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {}: {}", args.join(" "), stderr.trim())
    }
}

// ---------------------------------------------------------------------------
// State persistence
// ---------------------------------------------------------------------------

fn state_path(root: &Path) -> PathBuf {
    root.join(SPACER_DIR).join(STATE_FILE)
}

fn load_state(root: &Path) -> Result<ActiveState> {
    let path = state_path(root);
    if !path.exists() {
        return Ok(ActiveState::default());
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data).context("failed to parse state file")
}

fn save_state(root: &Path, state: &ActiveState) -> Result<()> {
    let dir = root.join(SPACER_DIR);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;
    let path = state_path(root);
    let data = serde_json::to_string_pretty(state).context("failed to serialize state")?;
    std::fs::write(&path, data)
        .with_context(|| format!("failed to write {}", path.display()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: run a git command in a directory (panics on failure).
    fn git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("git must be installed for these tests");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn make_committed_repo(path: &Path) {
        git(&["init"], path);
        git(&["config", "user.email", "test@example.com"], path);
        git(&["config", "user.name", "Test"], path);
        git(&["commit", "--allow-empty", "-m", "init"], path);
    }

    #[test]
    fn creates_space_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = GitAdapter::open(tmp.path()).unwrap();
        a.create_space("alpha", None).unwrap();
        assert!(tmp.path().join("alpha").is_dir());
        assert_eq!(a.spaces().len(), 1);
    }

    #[test]
    fn create_space_duplicate_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = GitAdapter::open(tmp.path()).unwrap();
        a.create_space("alpha", None).unwrap();
        assert!(a.create_space("alpha", None).is_err());
    }

    #[test]
    fn creates_project_with_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = GitAdapter::open(tmp.path()).unwrap();
        a.create_space("alpha", None).unwrap();
        a.create_project("web", "alpha", None).unwrap();
        assert!(tmp.path().join("alpha").join("web").join(".git").exists());
        assert_eq!(a.projects(Some("alpha")).len(), 1);
    }

    #[test]
    fn scan_discovers_existing_repos_on_open() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a space and a git repo manually
        let space_path = tmp.path().join("alpha");
        let project_path = space_path.join("web");
        std::fs::create_dir_all(&project_path).unwrap();
        git(&["init"], &project_path);

        let a = GitAdapter::open(tmp.path()).unwrap();
        assert_eq!(a.spaces().len(), 1);
        assert_eq!(a.projects(Some("alpha")).len(), 1);
    }

    #[test]
    fn start_and_finish_change_via_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = GitAdapter::open(tmp.path()).unwrap();
        a.create_space("alpha", None).unwrap();
        a.create_project("web", "alpha", None).unwrap();

        // Need at least one commit before worktrees work
        let project_path = tmp.path().join("alpha").join("web");
        make_committed_repo(&project_path);

        a.start_change("feat-login", "web", "alpha").unwrap();
        // Worktree directory should exist inside the project
        assert!(project_path.join("feat-login").is_dir());
        assert_eq!(a.changes(Some("web"), Some("alpha")).len(), 1);

        a.finish_change("feat-login", "web", "alpha").unwrap();
        assert!(!project_path.join("feat-login").exists());
        assert!(a.changes(Some("web"), Some("alpha")).is_empty());
    }

    #[test]
    fn scan_discovers_existing_worktrees_on_open() {
        let tmp = tempfile::tempdir().unwrap();
        let space_path = tmp.path().join("alpha");
        let project_path = space_path.join("web");
        std::fs::create_dir_all(&project_path).unwrap();
        make_committed_repo(&project_path);
        git(&["worktree", "add", "feat-login", "-b", "feat-login"], &project_path);

        let a = GitAdapter::open(tmp.path()).unwrap();
        assert_eq!(a.changes(Some("web"), Some("alpha")).len(), 1);
        assert_eq!(a.changes(Some("web"), Some("alpha"))[0].name, "feat-login");
    }

    #[test]
    fn active_space_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = GitAdapter::open(tmp.path()).unwrap();
        a.create_space("alpha", None).unwrap();
        a.set_active_space("alpha").unwrap();
        assert_eq!(a.active_space().as_deref(), Some("alpha"));
        a.save().unwrap();

        // Re-open and verify persistence
        let a2 = GitAdapter::open(tmp.path()).unwrap();
        assert_eq!(a2.active_space().as_deref(), Some("alpha"));
    }

    #[test]
    fn parse_worktree_output_skips_detached_head() {
        let output = "\
worktree /main\n\
HEAD abc123\n\
branch refs/heads/main\n\
\n\
worktree /detached\n\
HEAD def456\n\
detached\n\
\n\
worktree /feature\n\
HEAD 789abc\n\
branch refs/heads/feature\n\
";
        let entries = parse_worktree_output(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert!(entries[1].branch.is_none()); // detached
        assert_eq!(entries[2].branch.as_deref(), Some("feature"));
    }
}
