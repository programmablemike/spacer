use crate::ports::{Filesystem, Git, Registry};
use crate::{Change, ContextOverrides, Error, Project, Result, Space, WorkspaceSnapshot, Worktree};
use std::path::{Path, PathBuf};

mod validation;

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
        let root = filesystem.normalize(&root)?;
        let mut state = registry.load()?;
        validation::validate_state(&state)?;
        let mut project_paths = std::collections::HashSet::new();
        for project in &state.projects {
            if !project_paths.insert(filesystem.normalize(&project.path)?) {
                return Err(Error::InvalidOperation("duplicate canonical project paths in registry; back up and reconcile registrations before continuing".into()));
            }
        }
        // Legacy stale defaults are cleared in memory; opening never writes storage.
        validation::clear_stale_context(&mut state);
        let mut context = context;
        context.space = context.space.filter(|s| !s.is_empty());
        context.project = context.project.filter(|s| !s.is_empty());
        let effective_space = context.space.as_ref().or(state.active_space.as_ref());
        if let Some(space) = &context.space {
            if !state.spaces.iter().any(|s| &s.name == space) {
                return Err(Error::InvalidOperation(format!(
                    "session space '{space}' is not registered; correct or unset SPACER_SPACE"
                )));
            }
        }
        if let Some(project) = &context.project {
            let space = effective_space.ok_or_else(|| {
                Error::InvalidOperation(
                    "session project requires a registered space; set SPACER_SPACE".into(),
                )
            })?;
            if !state
                .projects
                .iter()
                .any(|p| &p.space == space && &p.name == project)
            {
                return Err(Error::InvalidOperation(format!("session project '{project}' is not registered in space '{space}'; correct or unset SPACER_PROJECT")));
            }
            // Bind the override to its space, even if that space came from defaults.
            context.space = Some(space.clone());
        }
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
            .or(self.state.active_space.as_ref())
            .filter(|name| self.state.spaces.iter().any(|s| &s.name == *name))
            .cloned()
    }
    pub fn active_project(&self) -> Option<String> {
        let space = self.active_space()?;
        let project = self.context.project.as_ref().or_else(|| {
            (self.state.active_space.as_deref() == Some(space.as_str()))
                .then_some(self.state.active_project.as_ref())
                .flatten()
        })?;
        self.state
            .projects
            .iter()
            .any(|p| p.space == space && &p.name == project)
            .then(|| project.clone())
    }
    /// A default project is usable only within its selected space.
    pub fn active_project_in(&self, space: &str) -> Option<String> {
        (self.active_space().as_deref() == Some(space))
            .then(|| self.active_project())
            .flatten()
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
        validation::entity_name(name)?;
        if self.state.spaces.iter().any(|s| s.name == name) {
            return Err(Error::AlreadyExists(format!(
                "space '{name}' already exists"
            )));
        }
        let space = Space {
            name: name.into(),
            path: self.creation_path(&self.root, name, path)?,
        };
        self.filesystem.create_dir_all(&space.path)?;
        let mut next = self.state.clone();
        next.spaces.push(space.clone());
        self.persist(next)?;
        Ok(space)
    }
    fn creation_path(
        &self,
        parent: &Path,
        name: &str,
        explicit: Option<PathBuf>,
    ) -> Result<PathBuf> {
        let parent = self.filesystem.normalize(parent)?;
        let implicit = explicit.is_none();
        let path = self
            .filesystem
            .normalize(&explicit.unwrap_or_else(|| parent.join(name)))?;
        if implicit && (!path.starts_with(&parent) || path == parent) {
            return Err(Error::InvalidOperation(
                "implicit path escapes its parent; use an explicit custom path".into(),
            ));
        }
        let metadata = self.filesystem.normalize(&self.root.join(".spacer"))?;
        if path.starts_with(&metadata) || metadata.starts_with(&path) {
            return Err(Error::InvalidOperation(
                "space/project path overlaps workspace metadata".into(),
            ));
        }
        Ok(path)
    }
    /// Unregister an empty space, preserving files.
    pub fn delete_space(&mut self, name: &str) -> Result<()> {
        self.space(name)?;
        if self.state.projects.iter().any(|p| p.space == name) {
            return Err(Error::InvalidOperation(format!(
                "space '{name}' contains registered projects; unregister them first"
            )));
        }
        let mut next = self.state.clone();
        if next.active_space.as_deref() == Some(name) {
            next.active_space = None;
            next.active_project = None;
        }
        next.spaces.retain(|s| s.name != name);
        self.persist(next)
    }
    pub fn set_active_space(&mut self, name: &str) -> Result<()> {
        self.space(name)?;
        let mut next = self.state.clone();
        if next.active_space.as_deref() != Some(name) {
            next.active_project = None;
        }
        next.active_space = Some(name.into());
        self.persist(next)
    }
    pub fn set_active_project(&mut self, name: &str, space: &str) -> Result<()> {
        self.project(name, space)?;
        let mut next = self.state.clone();
        next.active_space = Some(space.into());
        next.active_project = Some(name.into());
        self.persist(next)
    }
    pub fn create_project(
        &mut self,
        name: &str,
        space: &str,
        path: Option<PathBuf>,
    ) -> Result<Project> {
        self.create_project_with(name, space, path, false, false)
    }
    /// Explicit Git use case; ordinary create_project remains directory/metadata-only.
    pub fn create_git_project(
        &mut self,
        name: &str,
        space: &str,
        path: Option<PathBuf>,
    ) -> Result<Project> {
        self.create_project_with(name, space, path, true, false)
    }
    pub fn register_project(&mut self, name: &str, space: &str, path: PathBuf) -> Result<Project> {
        self.create_project_with(name, space, Some(path), false, true)
    }
    fn create_project_with(
        &mut self,
        name: &str,
        space: &str,
        path: Option<PathBuf>,
        initialize_git: bool,
        register: bool,
    ) -> Result<Project> {
        validation::entity_name(name)?;
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
            path: self.creation_path(&parent.path, name, path)?,
        };
        for existing in &self.state.projects {
            if self.filesystem.normalize(&existing.path)? == project.path {
                return Err(Error::AlreadyExists(format!(
                    "path '{}' is already registered as '{}/{}'",
                    project.path.display(),
                    existing.space,
                    existing.name
                )));
            }
        }
        if register {
            self.git.validate_repository(&project.path)?;
        } else {
            if initialize_git && !self.filesystem.is_empty_or_missing(&project.path)? {
                return Err(Error::InvalidOperation(
                    "project directory is not empty; register an existing repository instead"
                        .into(),
                ));
            }
            self.filesystem.create_dir_all(&project.path)?;
        }
        if initialize_git {
            self.git.initialize(&project.path)?;
        }
        let mut next = self.state.clone();
        next.projects.push(project.clone());
        self.persist(next).map_err(|e| Error::Persistence(format!("{e}; directory '{}' may already exist; inspect it and retry registration instead of deleting it", project.path.display())))?;
        Ok(project)
    }
    /// Unregister only, regardless of which Git implementation is injected.
    pub fn delete_project(&mut self, name: &str, space: &str) -> Result<()> {
        self.project(name, space)?;
        if self
            .state
            .changes
            .iter()
            .any(|c| c.space == space && c.project == name)
        {
            return Err(Error::InvalidOperation(format!(
                "project '{name}' has tracked changes; finish tracking them before unregistering"
            )));
        }
        let mut next = self.state.clone();
        if next.active_space.as_deref() == Some(space)
            && next.active_project.as_deref() == Some(name)
        {
            next.active_project = None;
        }
        next.projects
            .retain(|p| !(p.name == name && p.space == space));
        self.persist(next)
    }
    /// Metadata tracking; does not create a checkout.
    pub fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change> {
        validation::change_name(name)?;
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
        let repository = &self.project(project, space)?.path;
        self.git.validate_repository(repository)?;
        self.git.worktrees(repository)
    }
    /// Explicit worktree operation; branch-backed checkouts are observed through Git.
    pub fn start_worktree(
        &self,
        name: &str,
        project: &str,
        space: &str,
        base: &str,
    ) -> Result<Worktree> {
        validation::change_name(name)?;
        let repository = &self.project(project, space)?.path;
        self.git.validate_repository(repository)?;
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
        let metadata = self.filesystem.normalize(&self.root.join(".spacer"))?;
        let parent = metadata.join("worktrees").join(space).join(project);
        let path = self.filesystem.normalize(&parent.join(name))?;
        if !path.starts_with(&parent) || path.starts_with(repository) {
            return Err(Error::InvalidOperation(
                "worktree path escapes its managed directory or overlaps project sources".into(),
            ));
        }
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
        self.git.validate_repository(repository)?;
        if !self.git.is_clean(&worktree.path)? {
            return Err(Error::InvalidOperation("worktree contains modified, untracked, or ignored files; preserve them before finishing".into()));
        }
        self.git.remove_worktree(repository, &worktree.path)
    }
}

#[cfg(test)]
mod tests;
