//! Desktop presentation contract. Lifecycle decisions stay in spacer-core.
use serde::{Deserialize, Serialize};
use spacer_core::{Error, Workspace};
use std::path::PathBuf;

use crate::composition;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    pub kind: &'static str,
    pub message: String,
}
impl From<Error> for BridgeError {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::NotFound(_) => "notFound",
            Error::AlreadyExists(_) => "alreadyExists",
            Error::InvalidOperation(_) => "invalidOperation",
            Error::Persistence(_) => "persistence",
            Error::Filesystem(_) => "filesystem",
            Error::Git(_) => "git",
            Error::Authentication(_) => "authentication",
            Error::Remote(_) => "remote",
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}
impl BridgeError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: "internal",
            message: message.into(),
        }
    }
}
pub type Result<T> = std::result::Result<T, BridgeError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub root: PathBuf,
    pub spaces: Vec<Space>,
    pub projects: Vec<Project>,
    pub tracked_changes: Vec<TrackedChange>,
    pub active_space: Option<String>,
    pub active_project: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Space {
    pub name: String,
    pub path: PathBuf,
}
#[derive(Debug, Serialize)]
pub struct Project {
    pub name: String,
    pub space: String,
    pub path: PathBuf,
    pub worktrees: Vec<Checkout>,
    pub error: Option<BridgeError>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkout {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_primary: bool,
}
#[derive(Debug, Serialize)]
pub struct TrackedChange {
    pub name: String,
    pub project: String,
    pub space: String,
}
impl Snapshot {
    fn from_workspace(workspace: &Workspace) -> Self {
        let registry = workspace.snapshot();
        let projects = registry
            .projects
            .into_iter()
            .map(|project| {
                let (worktrees, error) = match workspace.worktrees(&project.name, &project.space) {
                    Ok(trees) => (
                        trees
                            .into_iter()
                            .map(|tree| Checkout {
                                path: tree.path,
                                branch: tree.branch,
                                is_primary: tree.is_primary,
                            })
                            .collect(),
                        None,
                    ),
                    Err(error) => (Vec::new(), Some(error.into())),
                };
                Project {
                    name: project.name,
                    space: project.space,
                    path: project.path,
                    worktrees,
                    error,
                }
            })
            .collect();
        Self {
            root: workspace.root().to_path_buf(),
            spaces: registry
                .spaces
                .into_iter()
                .map(|s| Space {
                    name: s.name,
                    path: s.path,
                })
                .collect(),
            projects,
            tracked_changes: registry
                .changes
                .into_iter()
                .map(|c| TrackedChange {
                    name: c.name,
                    project: c.project,
                    space: c.space,
                })
                .collect(),
            active_space: registry.active_space,
            active_project: registry.active_project,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Operation {
    CreateSpace {
        name: String,
        path: Option<String>,
    },
    CreateProject {
        name: String,
        space: String,
        path: Option<String>,
    },
    RegisterProject {
        name: String,
        space: String,
        path: String,
    },
    UnregisterSpace {
        space: String,
    },
    UnregisterProject {
        project: String,
        space: String,
    },
    UseSpace {
        space: String,
    },
    UseProject {
        project: String,
        space: String,
    },
    StartChange {
        name: String,
        project: String,
        space: String,
    },
    FinishChange {
        name: String,
        project: String,
        space: String,
    },
    RemoveTrackedChange {
        name: String,
        project: String,
        space: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Target {
    Workspace,
    Space {
        space: String,
    },
    Project {
        project: String,
        space: String,
    },
    Worktree {
        project: String,
        space: String,
        path: PathBuf,
    },
}

#[derive(Default)]
pub struct Session {
    root: Option<PathBuf>,
}
impl Session {
    pub fn open(&mut self, root: &str, initialize: bool) -> Result<Snapshot> {
        let workspace = composition::open(&composition::absolute_path(root)?, initialize)?;
        let snapshot = Snapshot::from_workspace(&workspace);
        self.root = Some(snapshot.root.clone());
        Ok(snapshot)
    }

    pub(crate) fn workspace(&self, expected_root: &str) -> Result<Workspace> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| Error::InvalidOperation("open a workspace first".into()))?;
        if root != &PathBuf::from(expected_root) {
            return Err(Error::InvalidOperation(
                "workspace changed; refresh before continuing".into(),
            )
            .into());
        }
        // Each request reloads the registry so CLI changes are never overwritten
        // using a long-lived cached Workspace. Registry conflict detection still
        // protects writes made while the operation is running.
        Ok(composition::open(root, false)?)
    }

    pub fn read(&self, root: &str) -> Result<Snapshot> {
        Ok(Snapshot::from_workspace(&self.workspace(root)?))
    }

    pub fn mutate(&self, root: &str, operation: Operation) -> Result<Snapshot> {
        let mut workspace = self.workspace(root)?;
        match operation {
            Operation::CreateSpace { name, path } => {
                workspace.create_space(
                    &name,
                    path.as_deref()
                        .map(composition::absolute_path)
                        .transpose()?,
                )?;
            }
            Operation::CreateProject { name, space, path } => {
                workspace.create_git_project(
                    &name,
                    &space,
                    path.as_deref()
                        .map(composition::absolute_path)
                        .transpose()?,
                )?;
            }
            Operation::RegisterProject { name, space, path } => {
                workspace.register_project(&name, &space, composition::absolute_path(&path)?)?;
            }
            Operation::UnregisterSpace { space } => workspace.delete_space(&space)?,
            Operation::UnregisterProject { project, space } => {
                workspace.delete_project(&project, &space)?
            }
            Operation::UseSpace { space } => workspace.set_active_space(&space)?,
            Operation::UseProject { project, space } => {
                workspace.set_active_project(&project, &space)?
            }
            Operation::StartChange {
                name,
                project,
                space,
            } => {
                workspace.start_worktree(&name, &project, &space, "HEAD")?;
            }
            Operation::FinishChange {
                name,
                project,
                space,
            } => workspace.close_worktree(&name, &project, &space)?,
            Operation::RemoveTrackedChange {
                name,
                project,
                space,
            } => workspace.finish_change(&name, &project, &space)?,
        }
        // Per-project read errors remain visible, without reporting a successful
        // mutation as failed merely because an unrelated repository is missing.
        Ok(Snapshot::from_workspace(&workspace))
    }

    pub fn resolve(&self, root: &str, target: Target) -> Result<PathBuf> {
        let workspace = self.workspace(root)?;
        let path = match target {
            Target::Workspace => workspace.root().to_path_buf(),
            Target::Space { space } => workspace.space_path(&space)?,
            Target::Project { project, space } => workspace.project_path(&project, &space)?,
            Target::Worktree {
                project,
                space,
                path,
            } => {
                workspace
                    .worktrees(&project, &space)?
                    .into_iter()
                    .find(|tree| tree.path == path)
                    .ok_or_else(|| {
                        Error::NotFound(
                            "checkout is no longer registered with Git; refresh the workspace"
                                .into(),
                        )
                    })?
                    .path
            }
        };
        Ok(composition::existing_directory(path)?)
    }
}

#[cfg(test)]
mod tests;
