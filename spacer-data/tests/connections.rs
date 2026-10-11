use spacer_core::{connections::*, ports::Registry, ContextOverrides, Error, Result, Workspace};
use spacer_data::{GitCli, JsonCatalog, JsonRegistry, LocalFilesystem};
use std::{
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct FakeHosting {
    removed: Mutex<Vec<String>>,
    cloned: Mutex<Vec<String>>,
}
fn repository(id: &str) -> RemoteRepository {
    RemoteRepository {
        id: id.into(),
        full_name: format!("example/repo{id}"),
        clone_url: format!("https://github.com/example/repo{id}.git"),
        private: true,
    }
}
impl Hosting for FakeHosting {
    fn existing_accounts(&self) -> Result<Vec<RemoteAccount>> {
        Ok(vec![RemoteAccount {
            id: "42".into(),
            login: "example".into(),
        }])
    }
    fn existing_account(&self, _login: &str) -> Result<RemoteAccount> {
        Ok(RemoteAccount {
            id: "42".into(),
            login: "example".into(),
        })
    }
    fn begin_login(&self, id: &str, _client: &str) -> Result<LoginChallenge> {
        Ok(LoginChallenge {
            id: id.into(),
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://github.com/login/device".into(),
            interval: 5,
            expires_at: 900,
        })
    }
    fn poll_login(&self, _id: &str) -> Result<Option<RemoteAccount>> {
        Ok(Some(RemoteAccount {
            id: "42".into(),
            login: "example".into(),
        }))
    }
    fn cancel_login(&self, _id: &str) -> Result<()> {
        Ok(())
    }
    fn delete_credentials(&self, id: &str) -> Result<()> {
        self.removed.lock().unwrap().push(id.into());
        Ok(())
    }
    fn repositories(&self, _c: &Connector) -> Result<Vec<RemoteRepository>> {
        Ok(vec![repository("1"), repository("2")])
    }
    fn repository(&self, _c: &Connector, id: &str) -> Result<RemoteRepository> {
        Ok(repository(id))
    }
    fn clone_repository(
        &self,
        _c: &Connector,
        remote: &RemoteRepository,
        path: &Path,
    ) -> Result<()> {
        std::fs::create_dir_all(path).unwrap();
        git(path, &["init", "--initial-branch=main"]);
        std::fs::write(path.join("README.md"), "fixture").unwrap();
        self.cloned.lock().unwrap().push(remote.id.clone());
        Ok(())
    }
}
fn git(path: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim_end().into()
}
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
fn service(directory: &Path, hosting: Arc<FakeHosting>) -> Connections {
    Connections::new(JsonCatalog::new(directory.into()), hosting)
}
#[test]
fn identities_and_connectors_are_independent_and_metadata_contains_no_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let hosting = Arc::new(FakeHosting::default());
    let service = service(directory.path(), hosting.clone());
    let identity = service
        .add_identity("Personal", "Example", "example@users.noreply.github.com")
        .unwrap();
    let c = service.link_existing("Work", "example").unwrap();
    assert_ne!(identity.id, c.id);
    assert!(service
        .add_identity("Personal", "Other", "other@example.invalid")
        .is_err());
    for email in [
        "no-at-sign",
        "bad\n@example.invalid",
        "space @example.invalid",
    ] {
        assert!(service.add_identity("Bad", "Example", email).is_err());
    }
    service.disconnect(&c.id).unwrap();
    assert!(hosting.removed.lock().unwrap().is_empty()); // External credentials untouched.
    assert_eq!(service.snapshot().unwrap().identities, vec![identity]);
    let bytes = std::fs::read_to_string(directory.path().join("accounts.json")).unwrap();
    assert!(!bytes.contains("access_token"));
    assert!(!bytes.contains("password"));
}
#[test]
fn optimistic_catalog_writers_cannot_overwrite_each_other() {
    let directory = tempfile::tempdir().unwrap();
    let first = JsonCatalog::new(directory.path().into());
    let second = JsonCatalog::new(directory.path().into());
    let mut a = first.load().unwrap();
    let b = second.load().unwrap();
    a.identities.push(Identity {
        id: first.new_id(),
        label: "A".into(),
        name: "Example".into(),
        email: "example@example.invalid".into(),
    });
    first.save(&a).unwrap();
    assert!(matches!(second.save(&b), Err(Error::Persistence(_))));
    assert_eq!(second.load().unwrap(), a);
}
#[test]
fn provisioning_records_selected_account_and_authorship_and_preserves_legacy_operations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("workspace");
    std::fs::create_dir(&root).unwrap();
    let hosting = Arc::new(FakeHosting::default());
    let service = service(&directory.path().join("user"), hosting);
    let c = service.link_existing("Work", "example").unwrap();
    let identity = service
        .add_identity("Work", "Example", "example@example.invalid")
        .unwrap();
    let mut ws = workspace(&root);
    ws.initialize().unwrap();
    ws.create_space("work", None).unwrap();
    ws.set_setup_defaults(SetupDefault {
        space: None,
        connector: Some(c.id.clone()),
        identity: Some(identity.id.clone()),
    })
    .unwrap();
    let plan = service
        .plan_import(&ws, "work", "repo", None, None, "1")
        .unwrap();
    assert_eq!(plan.identity, Some(identity.clone()));
    service.import(&mut ws, &plan).unwrap();
    assert_eq!(
        git(&plan.path, &["config", "--local", "user.name"]),
        identity.name
    );
    assert_eq!(
        git(&plan.path, &["config", "--local", "user.email"]),
        identity.email
    );
    let binding = &ws.snapshot().bindings[0];
    assert_eq!(binding.connector.as_deref(), Some(c.id.as_str()));
    assert_eq!(binding.remote.as_ref().unwrap().id, "1");
    let stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(".spacer/config.json")).unwrap()).unwrap();
    assert_eq!(stored["schema_version"], 2);
    service.disconnect(&c.id).unwrap();
    service.remove_identity(&identity.id).unwrap();
    let mut reopened = workspace(&root);
    assert_eq!(reopened.projects(None).len(), 1); // Local access remains possible.
    reopened.delete_project("repo", "work").unwrap();
    assert!(reopened.snapshot().bindings.is_empty());
    assert!(plan.path.join("README.md").exists());
}
#[test]
fn explicit_keep_overrides_defaults_and_plans_reject_changed_selection() {
    let directory = tempfile::tempdir().unwrap();
    let hosting = Arc::new(FakeHosting::default());
    let service = service(&directory.path().join("user"), hosting.clone());
    let c = service.link_existing("Work", "example").unwrap();
    let identity = service
        .add_identity("Work", "Example", "example@example.invalid")
        .unwrap();
    let mut ws = workspace(directory.path());
    ws.initialize().unwrap();
    ws.create_space("work", None).unwrap();
    ws.set_setup_defaults(SetupDefault {
        space: Some("work".into()),
        connector: Some(c.id.clone()),
        identity: Some(identity.id.clone()),
    })
    .unwrap();
    let plan = service
        .plan_import(&ws, "work", "repo", None, Some(""), "1")
        .unwrap();
    assert!(plan.identity.is_none());
    service.disconnect(&c.id).unwrap();
    assert!(service.import(&mut ws, &plan).is_err());
    assert!(hosting.cloned.lock().unwrap().is_empty());
}
#[test]
fn registration_failure_preserves_checkout_and_avoids_duplicate_clone() {
    let directory = tempfile::tempdir().unwrap();
    let hosting = Arc::new(FakeHosting::default());
    let service = service(&directory.path().join("user"), hosting.clone());
    let c = service.link_existing("Work", "example").unwrap();
    let mut ws = workspace(directory.path());
    ws.initialize().unwrap();
    ws.create_space("work", None).unwrap();
    let plan = service
        .plan_import(&ws, "work", "repo", Some(&c.id), Some(""), "1")
        .unwrap();
    let registry = JsonRegistry::new(directory.path());
    let mut newer = registry.load().unwrap();
    newer.active_space = Some("work".into());
    registry.save(&newer).unwrap();
    let error = service.import(&mut ws, &plan).unwrap_err().to_string();
    assert!(error.contains("checkout retained"));
    assert!(plan.path.join("README.md").exists());
    assert!(ws.projects(None).is_empty());
    assert!(service.import(&mut ws, &plan).is_err());
    assert_eq!(hosting.cloned.lock().unwrap().len(), 1);
    let mut reopened = workspace(directory.path());
    reopened
        .register_project("repo", "work", plan.path)
        .unwrap();
}
#[test]
fn local_identity_is_shared_by_worktrees_and_external_changes_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(
        &directory.path().join("user"),
        Arc::new(FakeHosting::default()),
    );
    let identity = service
        .add_identity("Work", "Example", "example@example.invalid")
        .unwrap();
    let mut ws = workspace(directory.path());
    ws.initialize().unwrap();
    ws.create_space("work", None).unwrap();
    let project = ws.create_git_project("repo", "work", None).unwrap();
    service
        .apply_identity(&mut ws, "work", "repo", &identity.id)
        .unwrap();
    git(
        &project.path,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "Fixture",
        ],
    );
    let tree = ws
        .start_worktree("feature", "repo", "work", "HEAD")
        .unwrap();
    assert_eq!(git(&tree.path, &["config", "user.email"]), identity.email);
    git(
        &project.path,
        &[
            "config",
            "--local",
            "user.email",
            "external@example.invalid",
        ],
    );
    assert!(service
        .apply_identity(&mut ws, "work", "repo", &identity.id)
        .unwrap_err()
        .to_string()
        .contains("outside Spacer"));
    assert_eq!(
        git(&project.path, &["config", "user.email"]),
        "external@example.invalid"
    );
}

#[cfg(unix)]
#[test]
fn import_preview_rejects_overlap_through_legacy_symlink_paths() {
    let directory = tempfile::tempdir().unwrap();
    let mut ws = workspace(directory.path());
    ws.initialize().unwrap();
    ws.create_space("work", None).unwrap();
    let project = ws.create_git_project("repo", "work", None).unwrap();
    ws.create_space("nested", Some(project.path.clone()))
        .unwrap();
    let alias = directory.path().join("alias");
    std::os::unix::fs::symlink(&project.path, &alias).unwrap();
    let registry = JsonRegistry::new(directory.path());
    let mut legacy = registry.load().unwrap();
    legacy.projects[0].path = alias;
    registry.save(&legacy).unwrap();
    let reopened = workspace(directory.path());
    assert!(matches!(
        reopened.preview_import_destination("new-repo", "nested"),
        Err(Error::AlreadyExists(_))
    ));
    assert!(!project.path.join("new-repo").exists());
}

#[test]
fn removal_by_label_resolves_managed_credentials_to_stable_ids() {
    let directory = tempfile::tempdir().unwrap();
    let hosting = Arc::new(FakeHosting::default());
    let mut service = service(directory.path(), hosting.clone());
    service
        .add_identity("Personal", "Example", "example@example.invalid")
        .unwrap();
    service.remove_identity("Personal").unwrap();
    let challenge = service.begin_login("Managed", "fixture-client").unwrap();
    let connector = service.poll_login(&challenge.id).unwrap().unwrap();
    service.disconnect("Managed").unwrap();
    assert_eq!(*hosting.removed.lock().unwrap(), vec![connector.id]);
    assert_eq!(service.snapshot().unwrap(), CatalogSnapshot::default());
}
