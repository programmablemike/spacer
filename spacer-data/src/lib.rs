//! Outbound adapters implementing ports owned by spacer-core.
pub mod filesystem;
pub mod git;
pub mod registry;

pub use filesystem::LocalFilesystem;
pub use git::GitCli;
pub use registry::JsonRegistry;

mod catalog;
mod connection_dto;
mod github;
pub use catalog::{catalog_directory, JsonCatalog};
pub use github::{credential_helper, Github, NativeSecrets, GITHUB_CLIENT_ID};
