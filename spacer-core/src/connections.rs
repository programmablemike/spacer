//! User-level connectors and commit identities. Credentials never enter snapshots.
use crate::{Error, Result, Workspace};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub id: String,
    pub label: String,
    pub name: String,
    pub email: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialSource {
    Keychain,
    GithubCli,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connector {
    pub id: String,
    pub label: String,
    pub host: String,
    pub account_id: String,
    pub login: String,
    pub source: CredentialSource,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogSnapshot {
    pub identities: Vec<Identity>,
    pub connectors: Vec<Connector>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteAccount {
    pub id: String,
    pub login: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRepository {
    pub id: String,
    pub full_name: String,
    pub clone_url: String,
    pub private: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectBinding {
    pub space: String,
    pub project: String,
    pub connector: Option<String>,
    pub identity: Option<String>,
    pub remote: Option<RemoteRepository>,
    pub applied_name: Option<String>,
    pub applied_email: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SetupDefault {
    pub space: Option<String>,
    pub connector: Option<String>,
    pub identity: Option<String>,
}
#[derive(Clone, Debug)]
pub struct LoginChallenge {
    pub id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_at: u64,
}
#[derive(Clone, Debug)]
pub struct ImportPlan {
    pub root: PathBuf,
    pub space: String,
    pub project: String,
    pub path: PathBuf,
    pub connector: Connector,
    pub identity: Option<Identity>,
    pub repository: RemoteRepository,
}
/// Atomic optimistic persistence, shared by the CLI and desktop composition roots.
pub trait Catalog: Send + Sync {
    fn load(&self) -> Result<CatalogSnapshot>;
    fn save(&self, snapshot: &CatalogSnapshot) -> Result<()>;
    fn new_id(&self) -> String;
}
/// Native provider access. Implementations own all tokens and credential retrieval.
pub trait Hosting: Send + Sync {
    fn existing_accounts(&self) -> Result<Vec<RemoteAccount>>;
    fn existing_account(&self, login: &str) -> Result<RemoteAccount>;
    fn begin_login(&self, id: &str, client_id: &str) -> Result<LoginChallenge>;
    fn poll_login(&self, id: &str) -> Result<Option<RemoteAccount>>;
    fn cancel_login(&self, id: &str) -> Result<()>;
    fn delete_credentials(&self, id: &str) -> Result<()>;
    fn repositories(&self, connector: &Connector) -> Result<Vec<RemoteRepository>>;
    fn repository(&self, connector: &Connector, id: &str) -> Result<RemoteRepository>;
    fn clone_repository(
        &self,
        connector: &Connector,
        repository: &RemoteRepository,
        path: &Path,
    ) -> Result<()>;
}

pub struct Connections {
    catalog: Box<dyn Catalog>,
    hosting: Arc<dyn Hosting>,
    pending: std::collections::HashMap<String, String>,
}
impl Connections {
    pub fn new(catalog: impl Catalog + 'static, hosting: Arc<dyn Hosting>) -> Self {
        Self {
            catalog: Box::new(catalog),
            hosting,
            pending: Default::default(),
        }
    }
    pub fn snapshot(&self) -> Result<CatalogSnapshot> {
        self.catalog.load()
    }
    pub fn add_identity(&self, label: &str, name: &str, email: &str) -> Result<Identity> {
        text(label, "identity label")?;
        text(name, "commit name")?;
        text(email, "commit email")?;
        if !email.contains('@')
            || email.chars().any(char::is_whitespace)
            || name.contains(['<', '>'])
            || email.contains(['<', '>'])
        {
            return Err(Error::InvalidOperation("provide a commit name and email without angle brackets; email must contain @ and no whitespace".into()));
        }
        let mut state = self.catalog.load()?;
        if state.identities.iter().any(|i| i.label == label) {
            return Err(Error::AlreadyExists("identity label already exists".into()));
        }
        let identity = Identity {
            id: self.catalog.new_id(),
            label: label.into(),
            name: name.into(),
            email: email.into(),
        };
        state.identities.push(identity.clone());
        self.catalog.save(&state)?;
        Ok(identity)
    }
    pub fn remove_identity(&self, id: &str) -> Result<()> {
        let mut state = self.catalog.load()?;
        let identity = state
            .identities
            .iter()
            .find(|i| i.id == id || i.label == id)
            .ok_or_else(|| Error::NotFound("identity not found".into()))?;
        let identity_id = identity.id.clone();
        state.identities.retain(|i| i.id != identity_id);
        self.catalog.save(&state)
    }
    pub fn existing_accounts(&self) -> Result<Vec<RemoteAccount>> {
        self.hosting.existing_accounts()
    }
    fn check_label(&self, label: &str) -> Result<()> {
        text(label, "connector label")?;
        if self
            .catalog
            .load()?
            .connectors
            .iter()
            .any(|c| c.label == label)
        {
            return Err(Error::AlreadyExists(
                "connector label already exists".into(),
            ));
        }
        Ok(())
    }
    fn save_connector(
        &self,
        id: String,
        label: &str,
        account: RemoteAccount,
        source: CredentialSource,
    ) -> Result<Connector> {
        text(label, "connector label")?;
        let mut state = self.catalog.load()?;
        if state.connectors.iter().any(|c| c.label == label) {
            return Err(Error::AlreadyExists(
                "connector label already exists".into(),
            ));
        }
        let connector = Connector {
            id,
            label: label.into(),
            host: "github.com".into(),
            account_id: account.id,
            login: account.login,
            source,
        };
        state.connectors.push(connector.clone());
        self.catalog.save(&state)?;
        Ok(connector)
    }
    pub fn link_existing(&self, label: &str, login: &str) -> Result<Connector> {
        self.check_label(label)?;
        let account = self.hosting.existing_account(login)?;
        self.save_connector(
            self.catalog.new_id(),
            label,
            account,
            CredentialSource::GithubCli,
        )
    }
    pub fn begin_login(&mut self, label: &str, client_id: &str) -> Result<LoginChallenge> {
        self.check_label(label)?;
        text(client_id, "GitHub App client ID")?;
        let id = self.catalog.new_id();
        let challenge = self.hosting.begin_login(&id, client_id)?;
        self.pending.insert(id, label.into());
        Ok(challenge)
    }
    pub fn poll_login(&mut self, id: &str) -> Result<Option<Connector>> {
        let label = self
            .pending
            .get(id)
            .ok_or_else(|| {
                Error::NotFound("authorization expired or cancelled; start again".into())
            })?
            .clone();
        let account = match self.hosting.poll_login(id) {
            Ok(Some(account)) => account,
            Ok(None) => return Ok(None),
            Err(error) => {
                self.pending.remove(id);
                let _ = self.hosting.cancel_login(id);
                return Err(error);
            }
        };
        self.pending.remove(id);
        match self.save_connector(id.into(), &label, account, CredentialSource::Keychain) {
            Ok(c) => Ok(Some(c)),
            Err(e) => {
                if self.hosting.delete_credentials(id).is_err() {
                    return Err(Error::Persistence(format!("{e}; credential cleanup failed; remove the Spacer credential-manager entry for connector {id}")));
                }
                Err(e)
            }
        }
    }
    pub fn cancel_login(&mut self, id: &str) -> Result<()> {
        self.pending.remove(id);
        self.hosting.cancel_login(id)
    }
    pub fn disconnect(&self, id: &str) -> Result<()> {
        let mut state = self.catalog.load()?;
        let connector = state
            .connectors
            .iter()
            .find(|c| c.id == id || c.label == id)
            .ok_or_else(|| Error::NotFound("connector not found".into()))?
            .clone();
        // If secure deletion fails, keep a visible entry so it can be retried.
        if connector.source == CredentialSource::Keychain {
            self.hosting.delete_credentials(&connector.id)?;
        }
        state.connectors.retain(|c| c.id != connector.id);
        self.catalog.save(&state)
    }
    pub fn connector(&self, id: &str) -> Result<Connector> {
        self.catalog
            .load()?
            .connectors
            .into_iter()
            .find(|c| c.id == id || c.label == id)
            .ok_or_else(|| {
                Error::NotFound("connector is missing; select or reconnect an account".into())
            })
    }
    pub fn identity(&self, id: &str) -> Result<Identity> {
        self.catalog
            .load()?
            .identities
            .into_iter()
            .find(|i| i.id == id || i.label == id)
            .ok_or_else(|| {
                Error::NotFound(
                    "identity is missing; select an identity or retain existing Git configuration"
                        .into(),
                )
            })
    }
    pub fn repositories(&self, id: &str) -> Result<Vec<RemoteRepository>> {
        self.hosting.repositories(&self.connector(id)?)
    }
    pub fn plan_import(
        &self,
        workspace: &Workspace,
        space: &str,
        name: &str,
        connector: Option<&str>,
        identity: Option<&str>,
        repository: &str,
    ) -> Result<ImportPlan> {
        let defaults = workspace.setup_defaults(space)?;
        let connector = connector
            .or(defaults.connector.as_deref())
            .ok_or_else(|| Error::InvalidOperation("select a connector".into()))?;
        let identity = match identity {
            Some("") => None, // Explicitly retain ordinary Git configuration.
            value => value
                .or(defaults.identity.as_deref())
                .map(|id| self.identity(id))
                .transpose()?,
        };
        let connector = self.connector(connector)?;
        let path = workspace.import_destination(name, space)?;
        let repository = self.hosting.repository(&connector, repository)?;
        Ok(ImportPlan {
            root: workspace.root().into(),
            space: space.into(),
            project: name.into(),
            path,
            connector,
            identity,
            repository,
        })
    }
    pub fn import(&self, workspace: &mut Workspace, plan: &ImportPlan) -> Result<()> {
        if workspace.root() != plan.root
            || workspace.import_destination(&plan.project, &plan.space)? != plan.path
        {
            return Err(Error::InvalidOperation(
                "workspace or destination changed; review a new setup plan".into(),
            ));
        }
        // Reject stale account or authorship selections rather than switching them.
        if self.connector(&plan.connector.id)? != plan.connector
            || plan
                .identity
                .as_ref()
                .is_some_and(|i| self.identity(&i.id).as_ref() != Ok(i))
        {
            return Err(Error::InvalidOperation(
                "connector or identity changed; review a new setup plan".into(),
            ));
        }
        let remote = self
            .hosting
            .repository(&plan.connector, &plan.repository.id)?;
        if remote != plan.repository {
            return Err(Error::InvalidOperation(
                "repository changed; review a new setup plan".into(),
            ));
        }
        self.hosting
            .clone_repository(&plan.connector, &remote, &plan.path)?;
        workspace.register_import(plan).map_err(|e| Error::Persistence(format!("{e}; checkout retained at '{}'; use project register and apply the identity explicitly to recover", plan.path.display())))
    }
    pub fn apply_identity(
        &self,
        workspace: &mut Workspace,
        space: &str,
        project: &str,
        id: &str,
    ) -> Result<()> {
        workspace.apply_project_identity(space, project, &self.identity(id)?)
    }
}
fn text(value: &str, kind: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(Error::InvalidOperation(format!(
            "{kind} must be nonempty, at most 256 bytes, and contain no control characters"
        )));
    }
    Ok(())
}

/// Validate persisted user metadata before any provider or Git operation.
pub fn validate_catalog(state: &CatalogSnapshot) -> Result<()> {
    let mut ids = std::collections::HashSet::new();
    let mut identity_labels = std::collections::HashSet::new();
    let mut connector_labels = std::collections::HashSet::new();
    for i in &state.identities {
        for (v, kind) in [
            (&i.id, "identity ID"),
            (&i.label, "identity label"),
            (&i.name, "commit name"),
            (&i.email, "commit email"),
        ] {
            text(v, kind)?;
        }
        if !i.email.contains('@')
            || i.email.chars().any(char::is_whitespace)
            || i.name.contains(['<', '>'])
            || i.email.contains(['<', '>'])
            || !ids.insert(&i.id)
            || !identity_labels.insert(&i.label)
        {
            return Err(Error::InvalidOperation("invalid or duplicate identity in user catalog; repair accounts.json before continuing".into()));
        }
    }
    for c in &state.connectors {
        for (v, kind) in [
            (&c.id, "connector ID"),
            (&c.label, "connector label"),
            (&c.account_id, "account ID"),
            (&c.login, "account login"),
        ] {
            text(v, kind)?;
        }
        if c.host != "github.com" || !ids.insert(&c.id) || !connector_labels.insert(&c.label) {
            return Err(Error::InvalidOperation("invalid or duplicate connector in user catalog; repair accounts.json before continuing".into()));
        }
    }
    Ok(())
}
