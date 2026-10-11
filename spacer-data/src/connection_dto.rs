//! Serialization boundary for user catalog and workspace bindings; no secrets.
use serde::{Deserialize, Serialize};
use spacer_core::connections::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredCatalog {
    pub schema_version: u32,
    pub identities: Vec<StoredIdentity>,
    pub connectors: Vec<StoredConnector>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredIdentity {
    pub id: String,
    pub label: String,
    pub name: String,
    pub email: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredConnector {
    pub id: String,
    pub label: String,
    pub host: String,
    pub account_id: String,
    pub login: String,
    pub source: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredRemote {
    pub id: String,
    pub full_name: String,
    pub clone_url: String,
    pub private: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredBinding {
    pub space: String,
    pub project: String,
    pub connector: Option<String>,
    pub identity: Option<String>,
    pub remote: Option<StoredRemote>,
    pub applied_name: Option<String>,
    pub applied_email: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredDefault {
    pub space: Option<String>,
    pub connector: Option<String>,
    pub identity: Option<String>,
}
macro_rules! mapping {
    ($stored:ident, $domain:ident, $($field:ident),+) => {
        impl From<$stored> for $domain { fn from(s: $stored) -> Self { Self { $($field: s.$field),+ } } }
        impl From<$domain> for $stored { fn from(s: $domain) -> Self { Self { $($field: s.$field),+ } } }
    }
}
mapping!(StoredIdentity, Identity, id, label, name, email);
mapping!(
    StoredRemote,
    RemoteRepository,
    id,
    full_name,
    clone_url,
    private
);
mapping!(StoredDefault, SetupDefault, space, connector, identity);
impl From<StoredBinding> for ProjectBinding {
    fn from(s: StoredBinding) -> Self {
        Self {
            space: s.space,
            project: s.project,
            connector: s.connector,
            identity: s.identity,
            remote: s.remote.map(Into::into),
            applied_name: s.applied_name,
            applied_email: s.applied_email,
        }
    }
}
impl From<ProjectBinding> for StoredBinding {
    fn from(s: ProjectBinding) -> Self {
        Self {
            space: s.space,
            project: s.project,
            connector: s.connector,
            identity: s.identity,
            remote: s.remote.map(Into::into),
            applied_name: s.applied_name,
            applied_email: s.applied_email,
        }
    }
}
impl From<Connector> for StoredConnector {
    fn from(c: Connector) -> Self {
        Self {
            id: c.id,
            label: c.label,
            host: c.host,
            account_id: c.account_id,
            login: c.login,
            source: match c.source {
                CredentialSource::Keychain => "keychain",
                CredentialSource::GithubCli => "github-cli",
            }
            .into(),
        }
    }
}
impl TryFrom<StoredConnector> for Connector {
    type Error = spacer_core::Error;
    fn try_from(c: StoredConnector) -> spacer_core::Result<Self> {
        if c.host != "github.com" || uuid::Uuid::parse_str(&c.id).is_err() {
            return Err(spacer_core::Error::Persistence(
                "unsupported connector host or invalid ID".into(),
            ));
        }
        let source = match c.source.as_str() {
            "keychain" => CredentialSource::Keychain,
            "github-cli" => CredentialSource::GithubCli,
            _ => {
                return Err(spacer_core::Error::Persistence(
                    "unsupported credential source".into(),
                ))
            }
        };
        Ok(Self {
            id: c.id,
            label: c.label,
            host: c.host,
            account_id: c.account_id,
            login: c.login,
            source,
        })
    }
}
