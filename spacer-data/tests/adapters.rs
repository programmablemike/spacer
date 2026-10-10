use spacer_core::{ports::Registry, ContextOverrides, Error, Workspace};
use spacer_data::{GitCli, JsonRegistry, LocalFilesystem};
use std::path::Path;
use std::process::Command;

fn workspace(root: &Path) -> Workspace {
    Workspace::open(
        root.into(),
        JsonRegistry::new(root),
        LocalFilesystem,
        GitCli,
        ContextOverrides::default(),
    )
    .unwrap()
}
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn json_schema_roundtrip_and_ordinary_operations_preserve_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let mut ws = workspace(root);
    assert!(ws.initialize().unwrap());
    assert!(!ws.initialize().unwrap());
    ws.create_space("alpha", None).unwrap();
    let p = ws.create_project("web", "alpha", None).unwrap();
    assert!(p.path.is_dir());
    assert!(!p.path.join(".git").exists());
    ws.set_active_space("alpha").unwrap();
    ws.set_active_project("web", "alpha").unwrap();
    ws.start_change("feature", "web", "alpha").unwrap();
    assert!(!p.path.join("feature").exists());
    let mut reopened = workspace(root);
    assert_eq!(reopened.snapshot(), ws.snapshot());
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".spacer/config.json")).unwrap())
            .unwrap();
    assert_eq!(json["active_space"], "alpha");
    assert_eq!(json["projects"][0]["space"], "alpha");
    let file = p.path.join("keep.txt");
    std::fs::write(&file, "keep").unwrap();
    reopened.finish_change("feature", "web", "alpha").unwrap();
    reopened.delete_project("web", "alpha").unwrap();
    reopened.delete_space("alpha").unwrap();
    assert_eq!(std::fs::read_to_string(file).unwrap(), "keep");
    assert!(workspace(root).spaces().is_empty());
}

#[test]
fn loads_existing_config_with_missing_optional_context() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".spacer")).unwrap();
    std::fs::write(
        tmp.path().join(".spacer/config.json"),
        r#"{"spaces":[{"name":"legacy","path":"relative"}],"projects":[],"changes":[]}"#,
    )
    .unwrap();
    let ws = workspace(tmp.path());
    assert_eq!(ws.spaces()[0].name, "legacy");
    assert_eq!(ws.spaces()[0].path, Path::new("relative"));
    assert!(ws.active_space().is_none());
}

#[test]
fn corrupt_config_is_reported_without_overwriting_it() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".spacer")).unwrap();
    let path = tmp.path().join(".spacer/config.json");
    std::fs::write(&path, "broken").unwrap();
    assert!(matches!(
        JsonRegistry::new(tmp.path()).load(),
        Err(Error::Persistence(_))
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "broken");
}

#[test]
fn explicit_git_lifecycle_preserves_branch_and_dirty_worktree() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    ws.create_space("alpha", None).unwrap();
    let p = ws.create_git_project("web", "alpha", None).unwrap();
    git(&p.path, &["commit", "--allow-empty", "-m", "initial"]);
    let primary = ws
        .worktrees("web", "alpha")
        .unwrap()
        .into_iter()
        .find(|w| w.is_primary)
        .unwrap();
    assert!(matches!(
        ws.close_worktree(primary.branch.as_deref().unwrap(), "web", "alpha"),
        Err(Error::InvalidOperation(_))
    ));
    let tree = ws
        .start_worktree("feat/login", "web", "alpha", "HEAD")
        .unwrap();
    assert!(tree.path.join(".git").is_file());
    let dirty = tree.path.join("untracked.txt");
    std::fs::write(&dirty, "keep").unwrap();
    assert!(matches!(
        ws.close_worktree("feat/login", "web", "alpha"),
        Err(Error::Git(_))
    ));
    assert!(dirty.exists());
    std::fs::remove_file(dirty).unwrap();
    let reopened = workspace(tmp.path());
    assert!(reopened
        .worktrees("web", "alpha")
        .unwrap()
        .iter()
        .any(|w| w.branch.as_deref() == Some("feat/login")));
    reopened
        .close_worktree("feat/login", "web", "alpha")
        .unwrap();
    assert!(!tree.path.exists());
    git(&p.path, &["show-ref", "--verify", "refs/heads/feat/login"]);
    assert!(reopened.changes(None, None).is_empty());
}

#[test]
fn discovers_and_closes_external_worktree_at_actual_path() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    ws.create_space("alpha", None).unwrap();
    let p = ws.create_git_project("web", "alpha", None).unwrap();
    git(&p.path, &["commit", "--allow-empty", "-m", "initial"]);
    let external = tmp.path().join("custom checkout");
    git(
        &p.path,
        &[
            "worktree",
            "add",
            "-b",
            "external",
            external.to_str().unwrap(),
        ],
    );
    let observed = ws.worktrees("web", "alpha").unwrap();
    let found = observed
        .iter()
        .find(|w| w.branch.as_deref() == Some("external"))
        .unwrap();
    assert_eq!(
        found.path.canonicalize().unwrap(),
        external.canonicalize().unwrap()
    );
    ws.close_worktree("external", "web", "alpha").unwrap();
    assert!(!external.exists());
}

#[test]
fn git_failure_leaves_metadata_intact() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    ws.create_space("alpha", None).unwrap();
    ws.create_git_project("web", "alpha", None).unwrap();
    let before = ws.snapshot();
    assert!(ws
        .start_worktree("feature", "web", "alpha", "HEAD")
        .is_err());
    assert_eq!(workspace(tmp.path()).snapshot(), before);
}

#[test]
fn stale_writer_cannot_overwrite_newer_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let mut first = workspace(tmp.path());
    let mut second = workspace(tmp.path());
    first.create_space("first", None).unwrap();
    assert!(second
        .create_space("second", None)
        .unwrap_err()
        .to_string()
        .contains("changed since loading"));
    let mut reopened = workspace(tmp.path());
    assert_eq!(reopened.spaces().len(), 1);
    reopened.create_space("second", None).unwrap();
    assert_eq!(workspace(tmp.path()).spaces().len(), 2);
}

#[test]
fn concurrent_registry_writers_preserve_one_update_and_report_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = ["first", "second"]
        .into_iter()
        .map(|name| {
            let root = tmp.path().to_owned();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut ws = workspace(&root);
                barrier.wait();
                ws.create_space(name, None)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(workspace(tmp.path()).spaces().len(), 1);
}

#[test]
fn unsupported_schema_and_abandoned_temporary_file_preserve_configuration() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    ws.create_space("keep", None).unwrap();
    let path = tmp.path().join(".spacer/config.json");
    let before = std::fs::read(&path).unwrap();
    std::fs::write(tmp.path().join(".spacer/.tmp-interrupted"), "partial").unwrap();
    assert_eq!(workspace(tmp.path()).spaces().len(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let future = r#"{"schema_version":999,"spaces":[],"projects":[],"changes":[]}"#;
    std::fs::write(&path, future).unwrap();
    let registry = JsonRegistry::new(tmp.path());
    assert!(registry.load().is_err());
    assert!(registry.save(&Default::default()).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), future);
}
