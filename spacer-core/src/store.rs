use anyhow::Result;
use std::path::PathBuf;

use crate::{Change, Project, Space};

/// Read/write operations for spaces.
pub trait SpaceStore {
    fn create_space(&mut self, name: &str, path: Option<PathBuf>) -> Result<Space>;
    fn spaces(&self) -> &[Space];
    fn delete_space(&mut self, name: &str) -> Result<()>;
    fn active_space(&self) -> Option<String>;
    fn set_active_space(&mut self, name: &str) -> Result<()>;
}

/// Read/write operations for projects.
pub trait ProjectStore {
    fn create_project(&mut self, name: &str, space: &str, path: Option<PathBuf>) -> Result<Project>;
    fn projects(&self, space: Option<&str>) -> Vec<&Project>;
    fn delete_project(&mut self, name: &str, space: &str) -> Result<()>;
    fn active_project(&self) -> Option<String>;
    fn set_active_project(&mut self, name: &str, space: &str) -> Result<()>;
}

/// Read/write operations for changes.
pub trait ChangeStore {
    fn start_change(&mut self, name: &str, project: &str, space: &str) -> Result<Change>;
    fn changes(&self, project: Option<&str>, space: Option<&str>) -> Vec<&Change>;
    fn finish_change(&mut self, name: &str, project: &str, space: &str) -> Result<()>;
}

/// Combined backend: a type that satisfies all three stores and can persist state.
pub trait Backend: SpaceStore + ProjectStore + ChangeStore {
    fn save(&self) -> Result<()>;
}
