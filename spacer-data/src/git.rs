use spacer_core::{ports::Git, Error, Result, Worktree};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git process adapter. Has no registry, environment context, or deletion policy.
#[derive(Default)]
pub struct GitCli;
impl Git for GitCli {
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
                path.as_os_str(),
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
                path.as_os_str(),
            ],
        )
        .map(|_| ())
    }
}
fn run(cwd: &Path, args: &[&OsStr]) -> Result<Vec<u8>> {
    let output = Command::new("git")
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
