//! Native setup presentation. Requests and replies contain metadata only.
use crate::bridge::{BridgeError, Result};
use serde::{Deserialize, Serialize};
use spacer_core::{connections::*, Workspace};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub type SharedConnections = Arc<Mutex<SetupSession>>;
#[derive(Default)]
pub struct SetupSession {
    service: Option<Connections>,
    plans: HashMap<String, ImportPlan>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Request {
    Catalog,
    AddIdentity {
        label: String,
        name: String,
        email: String,
    },
    RemoveIdentity {
        identity: String,
    },
    Existing,
    Link {
        label: String,
        login: String,
    },
    BeginLogin {
        label: String,
    },
    PollLogin {
        id: String,
    },
    CancelLogin {
        id: String,
    },
    Disconnect {
        connector: String,
    },
    Repositories {
        connector: String,
    },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogView {
    pub identities: Vec<IdentityView>,
    pub connectors: Vec<ConnectorView>,
}
#[derive(Serialize)]
pub struct IdentityView {
    pub id: String,
    pub label: String,
    pub name: String,
    pub email: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorView {
    pub id: String,
    pub label: String,
    pub login: String,
    pub account_id: String,
    pub source: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryView {
    pub id: String,
    pub full_name: String,
    pub private: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeView {
    pub id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub id: String,
    pub repository: String,
    pub project: String,
    pub path: String,
    pub connector: ConnectorView,
    pub identity: Option<IdentityView>,
}
#[derive(Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Reply {
    Catalog(CatalogView),
    Existing(Vec<String>),
    Login(ChallengeView),
    Connected(Option<ConnectorView>),
    Repositories(Vec<RepositoryView>),
    Done,
}
impl From<Identity> for IdentityView {
    fn from(i: Identity) -> Self {
        Self {
            id: i.id,
            label: i.label,
            name: i.name,
            email: i.email,
        }
    }
}
impl From<Connector> for ConnectorView {
    fn from(c: Connector) -> Self {
        Self {
            id: c.id,
            label: c.label,
            login: c.login,
            account_id: c.account_id,
            source: match c.source {
                CredentialSource::Keychain => "keychain",
                CredentialSource::GithubCli => "github-cli",
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositorySelection {
    pub id: String,
    pub project: String,
}
impl SetupSession {
    fn service(&mut self) -> Result<&mut Connections> {
        if self.service.is_none() {
            self.service = Some(crate::composition::connections()?);
        }
        Ok(self.service.as_mut().expect("service initialized"))
    }
    pub fn request(&mut self, request: Request) -> Result<Reply> {
        let service = self.service()?;
        Ok(match request {
            Request::Catalog => {
                let c = service.snapshot()?;
                Reply::Catalog(CatalogView {
                    identities: c.identities.into_iter().map(Into::into).collect(),
                    connectors: c.connectors.into_iter().map(Into::into).collect(),
                })
            }
            Request::AddIdentity { label, name, email } => {
                service.add_identity(&label, &name, &email)?;
                Reply::Done
            }
            Request::RemoveIdentity { identity } => {
                service.remove_identity(&identity)?;
                Reply::Done
            }
            Request::Existing => Reply::Existing(
                service
                    .existing_accounts()?
                    .into_iter()
                    .map(|a| a.login)
                    .collect(),
            ),
            Request::Link { label, login } => {
                Reply::Connected(Some(service.link_existing(&label, &login)?.into()))
            }
            Request::BeginLogin { label } => {
                let client = std::env::var("SPACER_GITHUB_CLIENT_ID")
                    .unwrap_or_else(|_| spacer_data::GITHUB_CLIENT_ID.into());
                let challenge = service.begin_login(&label, &client)?;
                Reply::Login(ChallengeView {
                    id: challenge.id,
                    user_code: challenge.user_code,
                    verification_uri: challenge.verification_uri,
                    interval: challenge.interval,
                })
            }
            Request::PollLogin { id } => Reply::Connected(service.poll_login(&id)?.map(Into::into)),
            Request::CancelLogin { id } => {
                service.cancel_login(&id)?;
                Reply::Done
            }
            Request::Disconnect { connector } => {
                service.disconnect(&connector)?;
                Reply::Done
            }
            Request::Repositories { connector } => Reply::Repositories(
                service
                    .repositories(&connector)?
                    .into_iter()
                    .map(|r| RepositoryView {
                        id: r.id,
                        full_name: r.full_name,
                        private: r.private,
                    })
                    .collect(),
            ),
        })
    }
    pub fn plan(
        &mut self,
        workspace: &Workspace,
        space: &str,
        connector: &str,
        identity: Option<&str>,
        repositories: &[RepositorySelection],
    ) -> Result<Vec<PlanView>> {
        if repositories.is_empty() {
            return Err(BridgeError::internal("select at least one repository"));
        }
        let service = self.service()?;
        let available = service.repositories(connector)?;
        let mut plans = Vec::new();
        let mut paths = std::collections::HashSet::new();
        for selection in repositories {
            let remote = available
                .iter()
                .find(|r| r.id == selection.id)
                .ok_or_else(|| {
                    BridgeError::internal("repository unavailable to selected connector")
                })?;
            let name = &selection.project;
            let plan = service.plan_import(
                workspace,
                space,
                name,
                Some(connector),
                identity,
                &remote.id,
            )?;
            if !paths.insert(plan.path.clone()) {
                return Err(BridgeError::internal(
                    "selected repositories share a project name; use separate spaces",
                ));
            }
            plans.push(plan);
        }
        self.plans.clear();
        let mut views = Vec::new();
        for plan in plans {
            let id = uuid::Uuid::new_v4().to_string();
            views.push(PlanView {
                id: id.clone(),
                repository: plan.repository.full_name.clone(),
                project: plan.project.clone(),
                path: plan.path.display().to_string(),
                connector: plan.connector.clone().into(),
                identity: plan.identity.clone().map(Into::into),
            });
            self.plans.insert(id, plan);
        }
        Ok(views)
    }
    pub fn execute(&mut self, workspace: &mut Workspace, id: &str) -> Result<()> {
        let plan = self.plans.remove(id).ok_or_else(|| {
            BridgeError::internal("setup plan expired or already executed; review a new plan")
        })?;
        self.service()?.import(workspace, &plan)?;
        Ok(())
    }
    pub fn apply(
        &mut self,
        workspace: &mut Workspace,
        space: &str,
        project: &str,
        identity: &str,
    ) -> Result<()> {
        self.service()?
            .apply_identity(workspace, space, project, identity)?;
        Ok(())
    }
}
