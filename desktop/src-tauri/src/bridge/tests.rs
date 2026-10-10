use super::*;
use std::path::Path;
use std::process::Command;

fn git(path: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap()
}
fn create_project(session: &Session, root: &str) {
    session
        .mutate(
            root,
            Operation::CreateSpace {
                name: "team".into(),
                path: None,
            },
        )
        .unwrap();
    session
        .mutate(
            root,
            Operation::CreateProject {
                name: "app".into(),
                space: "team".into(),
                path: None,
            },
        )
        .unwrap();
}
fn change(name: &str, finish: bool) -> Operation {
    if finish {
        Operation::FinishChange {
            name: name.into(),
            project: "app".into(),
            space: "team".into(),
        }
    } else {
        Operation::StartChange {
            name: name.into(),
            project: "app".into(),
            space: "team".into(),
        }
    }
}

#[test]
fn desktop_lifecycle_preserves_dirty_checkout_branches_and_unregistered_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = Session::default();
    let root = temp.path().to_str().unwrap();
    assert!(session.open(root, false).is_err());
    assert!(!temp.path().join(".spacer").exists());
    let root = session.open(root, true).unwrap().root;
    let root_text = root.to_str().unwrap();
    create_project(&session, root_text);
    let repository = root.join("team/app");
    assert_eq!(
        session
            .mutate(root_text, change("feat/desktop", false))
            .unwrap_err()
            .kind,
        "git"
    );
    assert!(
        git(&repository, &["commit", "--allow-empty", "-m", "Initial"])
            .status
            .success()
    );
    let state = session
        .mutate(root_text, change("feat/desktop", false))
        .unwrap();
    let checkout = state.projects[0]
        .worktrees
        .iter()
        .find(|tree| !tree.is_primary)
        .unwrap();
    let target = Target::Worktree {
        project: "app".into(),
        space: "team".into(),
        path: checkout.path.clone(),
    };
    assert_eq!(session.resolve(root_text, target).unwrap(), checkout.path);
    let file = checkout.path.join("preserve.txt");
    std::fs::write(&file, "unsaved work").unwrap();
    let error = session
        .mutate(root_text, change("feat/desktop", true))
        .unwrap_err();
    assert_eq!(error.kind, "invalidOperation");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "unsaved work");
    std::fs::remove_file(file).unwrap();
    session
        .mutate(root_text, change("feat/desktop", true))
        .unwrap();
    assert!(!checkout.path.exists());
    assert!(git(
        &repository,
        &["show-ref", "--verify", "refs/heads/feat/desktop"]
    )
    .status
    .success());
    session
        .mutate(
            root_text,
            Operation::UnregisterProject {
                project: "app".into(),
                space: "team".into(),
            },
        )
        .unwrap();
    assert!(repository.join(".git").is_dir());
    assert!(session.read(root_text).unwrap().projects.is_empty());
}

#[test]
fn mutations_reload_cli_changes_and_reject_stale_workspace_requests() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let mut session = Session::default();
    let first_root = session
        .open(first.path().to_str().unwrap(), true)
        .unwrap()
        .root;
    let first_text = first_root.to_str().unwrap();
    let mut external = composition::open(&first_root, false).unwrap();
    external.create_space("from-cli", None).unwrap();
    let state = session
        .mutate(
            first_text,
            Operation::CreateSpace {
                name: "from-desktop".into(),
                path: None,
            },
        )
        .unwrap();
    assert_eq!(state.spaces.len(), 2);
    let second_root = session
        .open(second.path().to_str().unwrap(), true)
        .unwrap()
        .root;
    assert!(session
        .mutate(
            first_text,
            Operation::CreateSpace {
                name: "wrong-workspace".into(),
                path: None
            }
        )
        .is_err());
    assert!(session
        .read(second_root.to_str().unwrap())
        .unwrap()
        .spaces
        .is_empty());
    let invalid = first.path().join("missing");
    assert!(session.open(invalid.to_str().unwrap(), false).is_err());
    assert!(session.read(second_root.to_str().unwrap()).is_ok());
}

#[test]
fn failed_repository_scan_is_visible_and_does_not_block_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = Session::default();
    let root = session
        .open(temp.path().to_str().unwrap(), true)
        .unwrap()
        .root;
    let root_text = root.to_str().unwrap();
    create_project(&session, root_text);
    // Move rather than delete the fixture repository to model an external change.
    let moved = root.join("moved-app");
    std::fs::rename(root.join("team/app"), &moved).unwrap();
    let state = session.read(root_text).unwrap();
    assert!(state.projects[0].error.is_some());
    assert!(state.projects[0].worktrees.is_empty());
    session
        .mutate(
            root_text,
            Operation::CreateSpace {
                name: "another".into(),
                path: None,
            },
        )
        .unwrap();
    session
        .mutate(
            root_text,
            Operation::UnregisterProject {
                project: "app".into(),
                space: "team".into(),
            },
        )
        .unwrap();
    assert!(moved.join(".git").is_dir());
}

#[test]
fn typed_requests_and_path_resolution_reject_ambiguous_targets() {
    assert!(serde_json::from_str::<Operation>(
        r#"{"kind":"createSpace","name":"team","force":true}"#
    )
    .is_err());
    let temp = tempfile::tempdir().unwrap();
    let mut session = Session::default();
    let root = session
        .open(temp.path().to_str().unwrap(), true)
        .unwrap()
        .root;
    let root_text = root.to_str().unwrap();
    assert!(session
        .mutate(
            root_text,
            Operation::CreateSpace {
                name: "relative".into(),
                path: Some("elsewhere".into())
            }
        )
        .is_err());
    create_project(&session, root_text);
    assert!(session
        .resolve(
            root_text,
            Target::Worktree {
                project: "app".into(),
                space: "team".into(),
                path: root.clone()
            }
        )
        .is_err());
    let config = root.join(".spacer/config.json");
    std::fs::write(&config, "broken").unwrap();
    assert!(session.open(root_text, true).is_err());
    assert_eq!(std::fs::read_to_string(config).unwrap(), "broken");
}
