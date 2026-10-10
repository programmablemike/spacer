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
    assert!(!path.join(".git").exists());
    ok(root, &["change", "start", "feature", "--project", "web"]);
    assert!(ok(root, &["change", "list"]).contains("feature"));
    assert!(!path.join("feature").exists());
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
    assert!(ok(root, &["shell", "sh"]).contains("spacer()"));
    let result = cli(root, &[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("configuration"));
    assert!(result.stdout.is_empty());
}

#[test]
fn environment_context_is_resolved_at_boundary_without_changing_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
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
    ok(root, &["change", "start", "feat/login", "--project", "web"]);
    let path = root.join(".spacer/config.json");
    let before = std::fs::read(&path).unwrap();
    for args in [
        &["space", "delete", "alpha"][..],
        &["project", "delete", "web"][..],
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
