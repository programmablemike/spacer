use crate::ports::{Filesystem, Git, Registry};
use crate::{Change, ContextOverrides, Error, Project, Result, Space, WorkspaceSnapshot, Worktree};
use std::path::{Component, Path, PathBuf};

/// Inbound application API shared by presentation adapters.
/// Outbound dependencies are injected; the core performs no direct I/O.
pub struct Workspace {
    root: PathBuf,
    state: WorkspaceSnapshot,
    context: ContextOverrides,
    registry: Box<dyn Registry>,
    filesystem: Box<dyn Filesystem>,
    git: Box<dyn Git>,
}

impl Workspace {
    pub fn open(
        root: PathBuf,
        registry: impl Registry + 'static,
        filesystem: impl Filesystem + 'static,
        git: impl Git + 'static,
        context: ContextOverrides,
    ) -> Result<Self> {
        let state = registry.load()?;
        Ok(Self {
            root,
            state,
            context,
            registry: Box::new(registry),
            filesystem: Box::new(filesystem),
            git: Box::new(git),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns true when initialized now, false when already initialized.
    pub fn initialize(&mut self) -> Result<bool> {
        if self.registry.is_initialized()? {
            return Ok(false);
        }
        self.registry.save(&self.state)?;
        Ok(true)
    }

    /// Includes effective session context; persisting never writes overrides into defaults.
    pub fn snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            active_space: self.active_space(),
            active_project: self.active_project(),
            ..self.state.clone()
        }
    }

    fn persist(&mut self, next: WorkspaceSnapshot) -> Result<()> {
        self.registry.save(&next)?;
        self.state = next;
        Ok(())
    }

    pub fn spaces(&self) -> &[Space] {
        &self.state.spaces
    }
    pub fn projects(&self, space: Option<&str>) -> Vec<&Project> {
        self.state
            .projects
            .iter()
            .filter(|p| space.is_none_or(|s| p.space == s))
            .collect()
    }
    pub fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change> {
        self.state
            .changes
            .iter()
            .filter(|c| {
                project.is_none_or(|p| c.project == p) && space.is_none_or(|s| c.space == s)
            })
            .collect()
    }
    pub fn active_space(&self) -> Option<String> {
        self.context
            .space
            .as_ref()
            .filter(|s| !s.is_empty())
            .or(self.state.active_space.as_ref())
            .cloned()
    }
    pub fn active_project(&self) -> Option<String> {
        self.context
            .project
            .as_ref()
            .filter(|s| !s.is_empty())
            .or(self.state.active_project.as_ref())
            .cloned()
    }
    fn space(&self, name: &str) -> Result<&Space> {
        self.state
            .spaces
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| Error::NotFound(format!("space '{name}' not found")))
    }
    fn project(&self, name: &str, space: &str) -> Result<&Project> {
        self.state
            .projects
            .iter()
            .find(|p| p.name == name && p.space == space)
            .ok_or_else(|| {
                Error::NotFound(format!("project '{name}' in space '{space}' not found"))
            })
    }
    pub fn create_space(&mut self, name: &str, path: Option<PathBuf>) -> Result<Space> {
        if self.state.spaces.iter().any(|s| s.name == name) {
            return Err(Error::AlreadyExists(format!(
                "space '{name}' already exists"
            )));
        }
        let space = Space {
            name: name.into(),
            path: path.unwrap_or_else(|| self.root.join(name)),
        };
        self.filesystem.create_dir_all(&space.path)?;
        let mut next = self.state.clone();
        next.spaces.push(space.clone());
        self.persist(next)?;
        Ok(space)
    }
    /// Unregister only; neither files nor child records are removed.
    pub fn delete_space(&mut self, name: &str) -> Result<()> {
        self.space(name)?;
        let mut next = self.state.clone();
        next.spaces.retain(|s| s.name != name);
        self.persist(next)
    }
    pub fn set_active_space(&mut self, name: &str) -> Result<()> {
        self.space(name)?;
        let mut next = self.state.clone();
        next.active_space = Some(name.into());
        self.persist(next)
    }
    pub fn set_active_project(&mut self, name: &str, space: &str) -> Result<()> {
        self.project(name, space)?;
        let mut next = self.state.clone();
        next.active_project = Some(name.into());
        self.persist(next)
    }
    pub fn create_project(
        &mut self,
        name: &str,
        space: &str,
        path: Option<PathBuf>,
    ) -> Result<Project> {
        self.create_project_with(name, space, path, false)
    }
    /// Explicit Git use case; ordinary create_project remains directory/metadata-only.
    pub fn create_git_project(
        &mut self,
        name: &str,
        space: &str,
        path: Option<PathBuf>,
    ) -> Result<Project> {
        self.create_project_with(name, space, path, true)
    }
    fn create_project_with(
        &mut self,
        name: &str,
        space: &str,
        path: Option<PathBuf>,
        initialize_git: bool,
    ) -> Result<Project> {
        let parent = self.space(space)?;
        if self
            .state
            .projects
            .iter()
            .any(|p| p.name == name && p.space == space)
        {
            return Err(Error::AlreadyExists(format!(
                "project '{name}' already exists in space '{space}'"
            )));
        }
        let project = Project {
            name: name.into(),
            space: space.into(),
            path: path.unwrap_or_else(|| parent.path.join(name)),
        };
        self.filesystem.create_dir_all(&project.path)?;
        if initialize_git {
            self.git.initialize(&project.path)?;
        }
        let mut next = self.state.clone();
        next.projects.push(project.clone());
        self.persist(next)?;
        Ok(project)
    }
    /// Unregister only, regardless of which Git implementation is injected.
    pub fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        self.project(name, space)?;
        let mut next = self.state.clone();
        next.projects
            .retain(|p| !(p.name == name && p.space == space));
        self.persist(next)
    }
    /// Metadata tracking; does not create a checkout.
    pub fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change> {
        self.project(project, space)?;
        if self
            .state
            .changes
            .iter()
            .any(|c| c.name == name && c.project == project && c.space == space)
        {
            return Err(Error::AlreadyExists(format!(
                "change '{name}' already exists in project '{project}'"
            )));
        }
        let change = Change {
            name: name.into(),
            project: project.into(),
            space: space.into(),
        };
        let mut next = self.state.clone();
        next.changes.push(change.clone());
        self.persist(next)?;
        Ok(change)
    }
    pub fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()> {
        if !self
            .state
            .changes
            .iter()
            .any(|c| c.name == name && c.project == project && c.space == space)
        {
            return Err(Error::NotFound(format!("change '{name}' not found")));
        }
        let mut next = self.state.clone();
        next.changes
            .retain(|c| !(c.name == name && c.project == project && c.space == space));
        self.persist(next)
    }
    /// Observe actual Git state for a registered project. Does not create registry entries.
    pub fn worktrees(&self, project: &str, space: &str) -> Result<Vec<Worktree>> {
        self.git.worktrees(&self.project(project, space)?.path)
    }
    /// Explicit worktree operation; branch-backed checkouts are observed through Git.
    pub fn start_worktree(
        &self,
        name: &str,
        project: &str,
        space: &str,
        base: &str,
    ) -> Result<Worktree> {
        if name.is_empty()
            || Path::new(name)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(Error::InvalidOperation(
                "worktree name must be a relative branch path without '.' or '..' components"
                    .into(),
            ));
        }
        let repository = &self.project(project, space)?.path;
        if self
            .git
            .worktrees(repository)?
            .iter()
            .any(|w| w.branch.as_deref() == Some(name))
        {
            return Err(Error::AlreadyExists(format!(
                "worktree for branch '{name}' already exists"
            )));
        }
        let path = repository.join(name);
        self.git.add_worktree(repository, name, &path, base)?;
        Ok(Worktree {
            path,
            branch: Some(name.into()),
            is_primary: false,
        })
    }
    /// Close a linked checkout at its observed path. The branch remains in Git.
    pub fn close_worktree(&self, name: &str, project: &str, space: &str) -> Result<()> {
        let repository = &self.project(project, space)?.path;
        let worktree = self
            .git
            .worktrees(repository)?
            .into_iter()
            .find(|w| w.branch.as_deref() == Some(name))
            .ok_or_else(|| Error::NotFound(format!("worktree for branch '{name}' not found")))?;
        if worktree.is_primary {
            return Err(Error::InvalidOperation(
                "cannot close the primary working tree".into(),
            ));
        }
        self.git.remove_worktree(repository, &worktree.path)
    }
}

#[cfg(test)]
mod tests;
