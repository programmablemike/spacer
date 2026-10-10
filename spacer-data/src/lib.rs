//! Outbound adapters implementing ports owned by spacer-core.
pub mod filesystem;
pub mod git;
pub mod registry;

pub use filesystem::LocalFilesystem;
pub use git::GitCli;
pub use registry::JsonRegistry;
