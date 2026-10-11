use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_spacer"))
        .current_dir(root)
        .env("SPACER_CONFIG_DIR", root.join("user"))
        .env("SPACER_ROOT", root.join("workspace"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn offline_identity_management_works_before_workspace_setup_and_apply_requires_review() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let add = cli(
        root,
        &[
            "identity",
            "add",
            "Personal",
            "--name",
            "Example",
            "--email",
            "example@users.noreply.github.com",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let list = cli(root, &["identity", "list"]);
    assert!(String::from_utf8_lossy(&list.stdout).contains("Personal"));
    let workspace = root.join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    for args in [
        &["workspace", "init"][..],
        &["space", "create", "personal"][..],
        &["project", "create", "example", "--space", "personal"][..],
    ] {
        let out = cli(root, args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let apply = [
        "identity",
        "apply",
        "Personal",
        "--space",
        "personal",
        "--project",
        "example",
    ];
    let preview = cli(root, &apply);
    assert!(!preview.status.success());
    assert!(String::from_utf8_lossy(&preview.stdout).contains("Proposed: Example"));
    let mut confirmed = apply.to_vec();
    confirmed.push("--yes");
    assert!(cli(root, &confirmed).status.success());
    let out = Command::new("git")
        .current_dir(workspace.join("personal/example"))
        .args(["config", "--local", "user.email"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "example@users.noreply.github.com"
    );
    assert!(cli(root, &["identity", "remove", "Personal"])
        .status
        .success());
    assert!(cli(root, &["project", "list"]).status.success());
}
#[test]
fn credential_helper_refuses_wrong_host_without_accessing_a_store() {
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_spacer"))
        .env("SPACER_CONFIG_DIR", directory.path())
        .args(["--spacer-credential", "unused-id", "owner/repo.git", "get"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"protocol=https\nhost=evil.invalid\npath=owner/repo.git\n\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn connector_login_rejects_a_github_app_registration_before_authorization() {
    let directory = tempfile::tempdir().unwrap();
    let output = cli(
        directory.path(),
        &[
            "connector",
            "login",
            "Personal",
            "--client-id",
            "Iv23fixture",
            "--no-browser",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("OAuth App client ID"));
    assert!(!directory.path().join("user/accounts.json").exists());
}
