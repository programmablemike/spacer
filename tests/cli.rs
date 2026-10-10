use std::path::Path;
use std::process::{Command, Output};

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spacer"))
        .args(args)
        .env("SPACER_ROOT", root)
        .env_remove("SPACER_SPACE")
        .env_remove("SPACER_PROJECT")
        .output()
        .unwrap()
}
fn ok(root: &Path, args: &[&str]) -> String {
    let result = cli(root, args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

#[test]
fn cli_roundtrip_uses_shared_service_and_persists_each_operation() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    assert!(ok(root, &["workspace", "init"]).starts_with("Initialized"));
    assert!(ok(root, &["workspace", "init"]).contains("already initialized"));
    ok(root, &["space", "create", "alpha"]);
    ok(root, &["space", "use", "alpha"]);
    ok(root, &["project", "create", "web"]);
    ok(root, &["project", "use", "web"]);
    let path = root.join("alpha/web");
    assert_eq!(
        ok(root, &["project", "go"]),
        path.canonicalize().unwrap().display().to_string()
    );
    assert!(path.join(".git").exists());
    initial_commit(&path);
    ok(root, &["change", "start", "feature", "--project", "web"]);
    assert!(ok(root, &["change", "list"]).contains("feature"));
    assert!(root.join(".spacer/worktrees/alpha/web/feature").exists());
    ok(root, &["change", "finish", "feature", "--project", "web"]);
    ok(root, &["project", "delete", "web"]);
    assert!(path.is_dir());
    assert!(!cli(root, &["project", "go", "web"]).status.success());
}

#[test]
fn shell_does_not_load_workspace_and_corrupt_state_errors_before_tui_setup() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir(root.join(".spacer")).unwrap();
    std::fs::write(root.join(".spacer/config.json"), "broken").unwrap();
    assert!(ok(root, &["shell", "integration", "sh"]).contains("spacer()"));
    let result = cli(root, &[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("configuration"));
    assert!(result.stdout.is_empty());
}

#[test]
fn environment_context_is_resolved_at_boundary_without_changing_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ok(root, &["workspace", "init"]);
    ok(root, &["space", "create", "saved"]);
    ok(root, &["space", "use", "saved"]);
    ok(root, &["space", "create", "session"]);
    let output = Command::new(env!("CARGO_BIN_EXE_spacer"))
        .args(["project", "create", "web"])
        .env("SPACER_ROOT", root)
        .env("SPACER_SPACE", "session")
        .env_remove("SPACER_PROJECT")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(root.join("session/web").is_dir());
    assert_eq!(ok(root, &["space", "current"]).trim(), "saved");
}

#[test]
fn scoped_context_and_parent_guards_survive_reopening() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ok(root, &["workspace", "init"]);
    for space in ["alpha", "beta"] {
        ok(root, &["space", "create", space]);
        ok(root, &["project", "create", "web", "--space", space]);
    }
    ok(root, &["project", "use", "web", "--space", "alpha"]);
    assert_eq!(ok(root, &["space", "current"]).trim(), "alpha");
    assert!(!cli(root, &["project", "go", "--space", "beta"])
        .status
        .success());
    assert!(!ok(root, &["project", "list", "--space", "beta"]).contains('*'));
    initial_commit(&root.join("alpha/web"));
    ok(root, &["change", "start", "feat/login", "--project", "web"]);
    let path = root.join(".spacer/config.json");
    let before = std::fs::read(&path).unwrap();
    for args in [
        &["space", "delete", "alpha"][..],
        &["space", "create", "../escape"][..],
    ] {
        let result = cli(root, args);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    ok(root, &["space", "use", "beta"]);
    assert!(ok(root, &["project", "current"]).contains("No active project"));
    ok(root, &["project", "use", "web"]);
    ok(root, &["project", "delete", "web"]);
    assert!(ok(root, &["project", "current"]).contains("No active project"));
    assert!(root.join("beta/web").is_dir());
}

#[test]
fn malformed_relationships_do_not_rewrite_configuration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir(root.join(".spacer")).unwrap();
    let path = root.join(".spacer/config.json");
    let data = r#"{"spaces":[],"projects":[{"name":"web","space":"missing","path":"/keep"}],"changes":[]}"#;
    let data = if cfg!(windows) {
        data.replace("/keep", "C:/keep")
    } else {
        data.to_owned()
    };
    std::fs::write(&path, &data).unwrap();
    let result = cli(root, &["space", "create", "new"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("missing space"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), data);
    assert!(!root.join("new").exists());
}

fn initial_commit(path: &Path) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "--allow-empty",
            "-m",
            "initial",
        ])
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn unregister_preserves_real_worktrees_and_registration_rediscovers_them() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ok(root, &["workspace", "init"]);
    ok(root, &["space", "create", "alpha"]);
    ok(root, &["space", "use", "alpha"]);
    ok(root, &["project", "create", "web"]);
    let path = root.join("alpha/web");
    assert!(
        !cli(root, &["change", "start", "feature", "--project", "web"])
            .status
            .success()
    );
    initial_commit(&path);
    ok(root, &["change", "start", "feature", "--project", "web"]);
    let tree = root.join(".spacer/worktrees/alpha/web/feature");
    std::fs::write(tree.join("keep"), "keep").unwrap();
    assert!(
        !cli(root, &["change", "finish", "feature", "--project", "web"])
            .status
            .success()
    );
    assert!(tree.join("keep").exists());
    ok(root, &["project", "unregister", "web"]);
    assert!(tree.is_dir());
    ok(
        root,
        &[
            "project",
            "register",
            "web",
            "--path",
            path.to_str().unwrap(),
        ],
    );
    assert!(ok(root, &["change", "list"]).contains("feature"));
}

fn from_directory(cwd: &Path, root: Option<&Path>, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacer"));
    cmd.current_dir(cwd)
        .args(args)
        .env_remove("SPACER_ROOT")
        .env_remove("SPACER_SPACE")
        .env_remove("SPACER_PROJECT");
    if let Some(root) = root {
        cmd.env("SPACER_ROOT", root);
    }
    cmd.output().unwrap()
}

#[test]
fn discovery_prefers_nearest_workspace_and_explicit_roots_never_fall_back() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    assert!(!from_directory(root, None, &["space", "list"])
        .status
        .success());
    ok(root, &["workspace", "init"]);
    ok(root, &["space", "create", "alpha"]);
    ok(root, &["project", "create", "web", "--space", "alpha"]);
    ok(root, &["project", "use", "web", "--space", "alpha"]);
    let project = root.join("alpha/web");
    initial_commit(&project);
    ok(root, &["change", "start", "feat/login", "--project", "web"]);
    let tree = root.join(".spacer/worktrees/alpha/web/feat/login");
    for cwd in [
        root.to_owned(),
        root.join("alpha"),
        project.clone(),
        tree.clone(),
    ] {
        let output = from_directory(&cwd, None, &["project", "current"]);
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "web");
    }
    assert_eq!(
        ok(root, &["path", "change", "feat/login"]),
        tree.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(
        ok(root, &["go", "alpha/web/feat/login"]),
        tree.canonicalize().unwrap().display().to_string()
    );
    assert!(
        !from_directory(&project, Some(&root.join("missing")), &["project", "list"])
            .status
            .success()
    );
    assert!(from_directory(&project, None, &["workspace", "init"])
        .status
        .success());
    let output = from_directory(&project, None, &["project", "current"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("No active project"));
    std::fs::write(project.join(".spacer/config.json"), "broken").unwrap();
    assert!(!from_directory(&project, None, &["project", "list"])
        .status
        .success());
    assert!(from_directory(&project, Some(root), &["project", "list"])
        .status
        .success());
}

#[test]
fn navigation_uses_external_git_paths_and_rejects_missing_checkouts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ok(root, &["workspace", "init"]);
    ok(root, &["space", "create", "alpha"]);
    ok(root, &["project", "create", "web", "--space", "alpha"]);
    ok(root, &["project", "use", "web", "--space", "alpha"]);
    let project = root.join("alpha/web");
    initial_commit(&project);
    let external = root.join("custom checkout");
    assert!(Command::new("git")
        .current_dir(&project)
        .args([
            "worktree",
            "add",
            "-b",
            "external",
            external.to_str().unwrap()
        ])
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(
        ok(root, &["path", "change", "external"]),
        external.canonicalize().unwrap().display().to_string()
    );
    std::fs::rename(&external, root.join("moved checkout")).unwrap();
    let output = cli(root, &["path", "change", "external"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing"));
}

#[cfg(unix)]
#[test]
fn shell_wrappers_preserve_cwd_and_failure_status() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ok(root, &["workspace", "init"]);
    ok(root, &["space", "create", "space with spaces"]);
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_spacer"));
    let path = std::env::join_paths(
        std::iter::once(binary.parent().unwrap().to_owned())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    for shell in ["bash", "zsh", "sh"] {
        if Command::new(shell)
            .arg("-c")
            .arg("exit 0")
            .status()
            .is_err()
        {
            continue;
        }
        let output = Command::new(shell)
            .arg("-c")
            .arg(
                r#"
            export PATH="$SPACER_TEST_PATH"
            eval "$(command spacer shell integration sh)" || exit 10
            spacer go 'space with spaces' || exit 11
            before=$PWD
            spacer go missing && exit 12
            [ "$before" = "$PWD" ] || exit 13
            spacer path space missing && exit 14
            [ "$before" = "$PWD" ] || exit 15
            command spacer completions bash >/dev/null || exit 16
        "#,
            )
            .env("PATH", &path)
            .env("SPACER_TEST_PATH", &path)
            .env("SPACER_ROOT", root)
            .env_remove("SPACER_SPACE")
            .env_remove("SPACER_PROJECT")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{shell}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn inherited_git_environment_cannot_redirect_project_operations() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ok(root, &["workspace", "init"]);
    ok(root, &["space", "create", "alpha"]);
    ok(root, &["project", "create", "other", "--space", "alpha"]);
    let other = root.join("alpha/other");
    initial_commit(&other);
    let config = std::fs::read(other.join(".git/config")).unwrap();
    let head = std::fs::read(other.join(".git/HEAD")).unwrap();
    let poisoned = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_spacer"))
            .args(args)
            .env("SPACER_ROOT", root)
            .env_remove("SPACER_SPACE")
            .env_remove("SPACER_PROJECT")
            .env("GIT_DIR", other.join(".git"))
            .env("GIT_WORK_TREE", &other)
            .env("GIT_COMMON_DIR", other.join(".git"))
            .env("GIT_INDEX_FILE", other.join(".git/index"))
            .env("GIT_OBJECT_DIRECTORY", other.join(".git/objects"))
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.worktree")
            .env("GIT_CONFIG_VALUE_0", &other)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    poisoned(&["project", "create", "web", "--space", "alpha"]);
    let project = root.join("alpha/web");
    assert!(project.join(".git").is_dir());
    initial_commit(&project);
    poisoned(&[
        "change",
        "start",
        "feature",
        "--space",
        "alpha",
        "--project",
        "web",
    ]);
    assert!(Command::new("git")
        .current_dir(&project)
        .args(["show-ref", "--verify", "refs/heads/feature"])
        .output()
        .unwrap()
        .status
        .success());
    assert!(!Command::new("git")
        .current_dir(&other)
        .args(["show-ref", "--verify", "refs/heads/feature"])
        .output()
        .unwrap()
        .status
        .success());
    poisoned(&[
        "change",
        "finish",
        "feature",
        "--space",
        "alpha",
        "--project",
        "web",
    ]);
    assert_eq!(std::fs::read(other.join(".git/config")).unwrap(), config);
    assert_eq!(std::fs::read(other.join(".git/HEAD")).unwrap(), head);
}
