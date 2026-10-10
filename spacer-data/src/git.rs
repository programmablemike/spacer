use spacer_core::{ports::Git, Error, Result, Worktree};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git process adapter. Has no registry, environment context, or deletion policy.
#[derive(Default)]
pub struct GitCli;
impl Git for GitCli {
    fn validate_repository(&self, repository: &Path) -> Result<()> {
        let bare = run(
            repository,
            &["rev-parse".as_ref(), "--is-bare-repository".as_ref()],
        )?;
        if bare != b"false\n" {
            return Err(Error::InvalidOperation(
                "bare repositories are not supported".into(),
            ));
        }
        let trees = self.worktrees(repository)?;
        let primary = trees
            .first()
            .ok_or_else(|| Error::Git("repository has no primary checkout".into()))?;
        let actual = std::fs::canonicalize(repository).map_err(|e| Error::Git(e.to_string()))?;
        let expected =
            std::fs::canonicalize(&primary.path).map_err(|e| Error::Git(e.to_string()))?;
        if actual != expected {
            return Err(Error::InvalidOperation(
                "register the primary repository root, not a subdirectory or linked worktree"
                    .into(),
            ));
        }
        Ok(())
    }
    fn is_clean(&self, worktree: &Path) -> Result<bool> {
        Ok(run(
            worktree,
            &[
                "status".as_ref(),
                "--porcelain=v1".as_ref(),
                "--untracked-files=all".as_ref(),
                "--ignored".as_ref(),
            ],
        )?
        .is_empty())
    }

    fn initialize(&self, repository: &Path) -> Result<()> {
        run(repository, &["init".as_ref()]).map(|_| ())
    }
    fn add_worktree(&self, repository: &Path, branch: &str, path: &Path, base: &str) -> Result<()> {
        run(
            repository,
            &[
                "check-ref-format".as_ref(),
                "--branch".as_ref(),
                branch.as_ref(),
            ],
        )?;
        run(
            repository,
            &["rev-parse".as_ref(), "--verify".as_ref(), "HEAD".as_ref()],
        )
        .map_err(|_| {
            Error::Git(
                "project has no initial commit; create a commit with Git before starting a change"
                    .into(),
            )
        })?;
        if path.try_exists().map_err(|e| Error::Git(e.to_string()))? {
            return Err(Error::AlreadyExists(format!(
                "worktree target '{}' already exists; inspect it before retrying",
                path.display()
            )));
        }
        let path = std::path::absolute(path)
            .map_err(|e| Error::Git(format!("invalid worktree path: {e}")))?;
        run(
            repository,
            &[
                "worktree".as_ref(),
                "add".as_ref(),
                "-b".as_ref(),
                branch.as_ref(),
                "--".as_ref(),
                git_path(&path).as_os_str(),
                base.as_ref(),
            ],
        )
        .map(|_| ())
    }
    fn worktrees(&self, repository: &Path) -> Result<Vec<Worktree>> {
        parse_worktrees(&run(
            repository,
            &[
                "worktree".as_ref(),
                "list".as_ref(),
                "--porcelain".as_ref(),
                "-z".as_ref(),
            ],
        )?)
    }
    fn remove_worktree(&self, repository: &Path, path: &Path) -> Result<()> {
        run(
            repository,
            &[
                "worktree".as_ref(),
                "remove".as_ref(),
                "--".as_ref(),
                git_path(path).as_os_str(),
            ],
        )
        .map(|_| ())
    }
}
// Git for Windows does not accept Rust's canonical verbatim path prefix.
fn git_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut parts = path.components();
        if let Some(Component::Prefix(prefix)) = parts.next() {
            match prefix.kind() {
                Prefix::VerbatimDisk(drive) => {
                    let mut result = PathBuf::from(format!("{}:", char::from(drive)));
                    result.push(parts.as_path());
                    return result;
                }
                Prefix::VerbatimUNC(server, share) => {
                    let mut result = PathBuf::from(r"\\");
                    result.push(server);
                    result.push(share);
                    result.push(parts.as_path());
                    return result;
                }
                _ => {}
            }
        }
    }
    path.to_path_buf()
}
fn run(cwd: &Path, args: &[&OsStr]) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    // Hook and shell environments must not redirect a registered repository's
    // index, objects, worktree, or command-scoped configuration elsewhere.
    for name in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
        "GIT_OBJECT_DIRECTORY",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_IMPLICIT_WORK_TREE",
        "GIT_GRAFT_FILE",
        "GIT_INDEX_FILE",
        "GIT_NO_REPLACE_OBJECTS",
        "GIT_REPLACE_REF_BASE",
        "GIT_PREFIX",
        "GIT_SHALLOW_FILE",
        "GIT_COMMON_DIR",
        "GIT_NAMESPACE",
    ] {
        command.env_remove(name);
    }
    let output = command
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git in '{}': {e}", cwd.display())))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(Error::Git(format!(
            "git in '{}': {}",
            cwd.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
fn parse_worktrees(output: &[u8]) -> Result<Vec<Worktree>> {
    let mut entries: Vec<Worktree> = Vec::new();
    for field in output.split(|b| *b == 0) {
        if let Some(path) = field.strip_prefix(b"worktree ") {
            entries.push(Worktree {
                path: path_from_bytes(path)?,
                branch: None,
                is_primary: entries.is_empty(),
            });
        } else if let Some(branch) = field.strip_prefix(b"branch refs/heads/") {
            let entry = entries.last_mut().ok_or_else(|| {
                Error::Git("invalid git worktree output: branch without path".into())
            })?;
            entry.branch = Some(
                std::str::from_utf8(branch)
                    .map_err(|e| Error::Git(format!("invalid branch encoding: {e}")))?
                    .into(),
            );
        }
    }
    Ok(entries)
}
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(OsString::from_vec(bytes.to_vec()).into())
    }
    #[cfg(not(unix))]
    {
        Ok(OsString::from(
            std::str::from_utf8(bytes)
                .map_err(|e| Error::Git(format!("invalid worktree path encoding: {e}")))?,
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn converts_canonical_paths_for_git() {
        assert_eq!(
            git_path(Path::new(r"\\?\C:\work\feature")),
            PathBuf::from(r"C:\work\feature")
        );
        assert_eq!(
            git_path(Path::new(r"\\?\UNC\server\share\feature")),
            PathBuf::from(r"\\server\share\feature")
        );
    }
    #[test]
    fn parses_actual_paths_and_detached_worktrees() {
        let trees = parse_worktrees(b"worktree /main\0HEAD abc\0branch refs/heads/main\0\0worktree /elsewhere/a path\0HEAD abc\0branch refs/heads/feature\0\0worktree /detached\0detached\0\0").unwrap();
        assert_eq!(trees.len(), 3);
        assert!(trees[0].is_primary);
        assert!(!trees[1].is_primary);
        assert_eq!(trees[1].path, PathBuf::from("/elsewhere/a path"));
        assert_eq!(trees[1].branch.as_deref(), Some("feature"));
        assert!(trees[2].branch.is_none());
    }
}
