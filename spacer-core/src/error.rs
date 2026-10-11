use std::fmt;

/// Stable categories for presentation adapters; details contain actionable context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotFound(String),
    AlreadyExists(String),
    InvalidOperation(String),
    Persistence(String),
    Filesystem(String),
    Git(String),
    Authentication(String),
    Remote(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(s)
            | Self::AlreadyExists(s)
            | Self::InvalidOperation(s)
            | Self::Persistence(s)
            | Self::Filesystem(s)
            | Self::Git(s)
            | Self::Authentication(s)
            | Self::Remote(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
