//! GitHub adapter. Tokens stay in native memory and the OS credential store.
use crate::{catalog_directory, JsonCatalog};
use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spacer_core::{connections::*, Error, Result};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::{Zeroize, Zeroizing};

/// Public OAuth App registration; deployments can override this through the environment.
const GITHUB_CLIENT_ID: &str = "Ov23liV39bucQcfRgIOw";
pub const GITHUB_OAUTH_SCOPES: &str = "repo offline_access";

/// Resolve the public registration from an explicit value, environment, or bundled default.
pub fn github_client_id(explicit: Option<&str>) -> Result<String> {
    let id = explicit
        .map(str::to_owned)
        .or_else(|| std::env::var("SPACER_GITHUB_CLIENT_ID").ok())
        .unwrap_or_else(|| GITHUB_CLIENT_ID.into());
    validate_client_id(&id)?;
    Ok(id)
}
fn validate_client_id(id: &str) -> Result<()> {
    if id.trim().is_empty() {
        return Err(Error::Authentication("GitHub OAuth App client ID is not configured; set SPACER_GITHUB_CLIENT_ID or use connector login --client-id".into()));
    }
    if id.starts_with("Iv") {
        return Err(Error::Authentication("this client ID belongs to a GitHub App; configure an OAuth App client ID with Device flow enabled instead".into()));
    }
    Ok(())
}
/// Native storage contract; test stores are injected, never selected by environment.
pub trait Secrets: Send + Sync {
    fn get(&self, id: &str) -> Result<Zeroizing<String>>;
    fn put(&self, id: &str, value: &str) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}
pub struct NativeSecrets;
impl NativeSecrets {
    fn entry(id: &str) -> Result<keyring::Entry> {
        uuid::Uuid::parse_str(id)
            .map_err(|_| Error::InvalidOperation("invalid connector reference".into()))?;
        keyring::Entry::new("io.github.programmablemike.spacer.github", id)
            .map_err(|_| secure_error())
    }
}
fn secure_error() -> Error {
    Error::Authentication("secure credential storage is unavailable, locked, or denied; unlock your system credential manager and retry (no plaintext fallback)".into())
}
impl Secrets for NativeSecrets {
    fn get(&self, id: &str) -> Result<Zeroizing<String>> {
        Self::entry(id)?.get_password().map(Zeroizing::new).map_err(|_| Error::Authentication("credentials are missing or locked; unlock your credential manager or reconnect".into()))
    }
    fn put(&self, id: &str, value: &str) -> Result<()> {
        Self::entry(id)?
            .set_password(value)
            .map_err(|_| secure_error())
    }
    fn delete(&self, id: &str) -> Result<()> {
        match Self::entry(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(secure_error()),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
    client_id: Option<String>,
}
impl Drop for Tokens {
    fn drop(&mut self) {
        self.access_token.zeroize();
        if let Some(t) = &mut self.refresh_token {
            t.zeroize();
        }
    }
}
struct Pending {
    device_code: Zeroizing<String>,
    client_id: String,
    expires_at: u64,
    interval: u64,
    next_poll: u64,
}
pub struct Github {
    client: Client,
    secrets: Arc<dyn Secrets>,
    pending: Mutex<HashMap<String, Pending>>,
    api_base: String,
    oauth_base: String,
    credential_locks: std::path::PathBuf,
}
impl Github {
    pub fn new(secrets: Arc<dyn Secrets>) -> Result<Self> {
        let client = Client::builder()
            .user_agent("spacer/0.0.1")
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| Error::Remote("cannot initialize HTTPS client".into()))?;
        Ok(Self {
            client,
            secrets,
            pending: Mutex::new(HashMap::new()),
            api_base: "https://api.github.com".into(),
            oauth_base: "https://github.com".into(),
            credential_locks: catalog_directory()?.join("credentials"),
        })
    }
    fn api(&self, route: &str, token: &str) -> Result<Value> {
        response(
            self.client
                .get(format!("{}{route}", self.api_base))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .bearer_auth(token),
        )
    }
    fn account(&self, token: &str) -> Result<RemoteAccount> {
        let value = self.api("/user", token)?;
        Ok(RemoteAccount {
            id: numeric_id(&value)?,
            login: string(&value, "login")?.into(),
        })
    }
    fn store(&self, id: &str, tokens: &Tokens) -> Result<()> {
        let payload = Zeroizing::new(serde_json::to_string(tokens).map_err(|_| secure_error())?);
        self.secrets.put(id, &payload)
    }
    fn tokens(&self, connector: &Connector) -> Result<Tokens> {
        if connector.host != "github.com" {
            return Err(Error::InvalidOperation(
                "only github.com is supported".into(),
            ));
        }
        let tokens = match connector.source {
            CredentialSource::GithubCli => Tokens {
                access_token: gh_token(&connector.login)?.to_string(),
                refresh_token: None,
                expires_at: None,
                client_id: None,
            },
            CredentialSource::Keychain => {
                // Serialize refresh across CLI/desktop processes: refresh tokens rotate.
                let directory = &self.credential_locks;
                std::fs::create_dir_all(directory).map_err(|_| secure_error())?;
                uuid::Uuid::parse_str(&connector.id).map_err(|_| secure_error())?;
                let lock = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(directory.join(format!("{}.lock", connector.id)))
                    .map_err(|_| secure_error())?;
                lock.try_lock().map_err(|_| {
                    Error::Authentication(
                        "credentials are in use; retry after the other operation completes".into(),
                    )
                })?;
                let _guard = crate::registry::RegistryLock(lock);
                let encoded = self.secrets.get(&connector.id)?;
                let mut tokens: Tokens = serde_json::from_str(&encoded).map_err(|_| {
                    Error::Authentication("stored credentials are invalid; reconnect".into())
                })?;
                if tokens.expires_at.is_some_and(|t| t <= now() + 60) {
                    let refresh = tokens.refresh_token.as_deref().ok_or_else(|| {
                        Error::Authentication("credentials expired; reconnect".into())
                    })?;
                    let client_id = tokens.client_id.as_deref().ok_or_else(|| {
                        Error::Authentication("credentials expired; reconnect".into())
                    })?;
                    let mut value = response(
                        self.client
                            .post(format!("{}/login/oauth/access_token", self.oauth_base))
                            .header("Accept", "application/json")
                            .form(&[
                                ("client_id", client_id),
                                ("grant_type", "refresh_token"),
                                ("refresh_token", refresh),
                            ]),
                    )?;
                    tokens = parse_tokens(&mut value, client_id)?;
                    self.store(&connector.id, &tokens)?;
                }
                tokens
            }
        };
        let account = self.account(&tokens.access_token)?;
        if account.id != connector.account_id {
            return Err(Error::Authentication(
                "credential account differs from the selected connector; reconnect explicitly"
                    .into(),
            ));
        }
        Ok(tokens)
    }
}
impl Hosting for Github {
    fn existing_accounts(&self) -> Result<Vec<RemoteAccount>> {
        let output = gh()
            .args([
                "auth",
                "status",
                "--hostname",
                "github.com",
                "--json",
                "hosts",
                "--jq",
                "[.hosts[\"github.com\"][] | {login: .login}]",
            ])
            .output()
            .map_err(|_| {
                Error::Authentication(
                    "GitHub CLI is unavailable; sign in with GitHub instead".into(),
                )
            })?;
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| Error::Authentication("no existing GitHub CLI accounts found".into()))?;
        value
            .as_array()
            .ok_or_else(|| Error::Authentication("no existing GitHub CLI accounts found".into()))?
            .iter()
            .filter_map(|v| v.get("login").and_then(Value::as_str))
            .map(|login| {
                Ok(RemoteAccount {
                    id: String::new(),
                    login: login.into(),
                })
            })
            .collect()
    }
    fn existing_account(&self, login: &str) -> Result<RemoteAccount> {
        if login.is_empty()
            || login
                .chars()
                .any(|c| !(c.is_ascii_alphanumeric() || c == '-'))
        {
            return Err(Error::InvalidOperation("invalid GitHub username".into()));
        }
        let token = gh_token(login)?;
        let account = self.account(&token)?;
        if !account.login.eq_ignore_ascii_case(login) {
            return Err(Error::Authentication(
                "existing credential belongs to a different account".into(),
            ));
        }
        Ok(account)
    }
    fn begin_login(&self, id: &str, client_id: &str) -> Result<LoginChallenge> {
        validate_client_id(client_id)?;
        let mut value = response(
            self.client
                .post(format!("{}/login/device/code", self.oauth_base))
                .header("Accept", "application/json")
                .form(&[("client_id", client_id), ("scope", GITHUB_OAUTH_SCOPES)]),
        )?;
        let device_code = Zeroizing::new(value.get("device_code").and_then(Value::as_str).ok_or_else(|| Error::Authentication("device authorization is unavailable; enable device flow for this OAuth App".into()))?.to_string());
        let interval = value
            .get("interval")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .max(1);
        let expires_at = now()
            + value
                .get("expires_in")
                .and_then(Value::as_u64)
                .unwrap_or(900)
                .min(900);
        let challenge = LoginChallenge {
            id: id.into(),
            user_code: string(&value, "user_code")?.into(),
            verification_uri: "https://github.com/login/device".into(),
            interval,
            expires_at,
        };
        self.pending.lock().map_err(|_| secure_error())?.insert(
            id.into(),
            Pending {
                device_code,
                client_id: client_id.into(),
                expires_at,
                interval,
                next_poll: now() + interval,
            },
        );
        wipe_value(&mut value);
        Ok(challenge)
    }
    fn poll_login(&self, id: &str) -> Result<Option<RemoteAccount>> {
        let mut pending = self.pending.lock().map_err(|_| secure_error())?;
        let p = pending.get_mut(id).ok_or_else(|| {
            Error::Authentication("authorization cancelled or expired; start again".into())
        })?;
        if now() >= p.expires_at {
            pending.remove(id);
            return Err(Error::Authentication(
                "authorization expired; start again".into(),
            ));
        }
        if now() < p.next_poll {
            return Ok(None);
        }
        p.next_poll = now() + p.interval;
        let mut value = response(
            self.client
                .post(format!("{}/login/oauth/access_token", self.oauth_base))
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", p.client_id.as_str()),
                    ("device_code", p.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ]),
        )?;
        match value.get("error").and_then(Value::as_str) {
            Some("authorization_pending") => return Ok(None),
            Some("slow_down") => {
                p.interval += 5;
                p.next_poll = now() + p.interval;
                return Ok(None);
            }
            Some(_) => {
                pending.remove(id);
                return Err(Error::Authentication(
                    "authorization denied or expired; start again".into(),
                ));
            }
            None => (),
        }
        let tokens = parse_tokens(&mut value, &p.client_id)?;
        let account = self.account(&tokens.access_token)?;
        self.store(id, &tokens)?;
        pending.remove(id);
        Ok(Some(account))
    }
    fn cancel_login(&self, id: &str) -> Result<()> {
        self.pending.lock().map_err(|_| secure_error())?.remove(id);
        Ok(())
    }
    fn delete_credentials(&self, id: &str) -> Result<()> {
        self.secrets.delete(id)
    }
    fn repositories(&self, connector: &Connector) -> Result<Vec<RemoteRepository>> {
        let tokens = self.tokens(connector)?;
        let mut repositories = Vec::new();
        let mut page = 1;
        loop {
            let value = self.api(
                &format!("/user/repos?per_page=100&page={page}&sort=full_name"),
                &tokens.access_token,
            )?;
            let rows = value
                .as_array()
                .ok_or_else(|| Error::Remote("invalid repository response".into()))?;
            for row in rows {
                repositories.push(parse_repository(row)?);
            }
            if rows.len() < 100 {
                break;
            }
            page += 1;
        }
        repositories.sort_by(|a, b| a.full_name.cmp(&b.full_name));
        repositories.dedup_by(|a, b| a.id == b.id);
        Ok(repositories)
    }
    fn repository(&self, connector: &Connector, id: &str) -> Result<RemoteRepository> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidOperation(
                "invalid GitHub repository ID".into(),
            ));
        }
        let tokens = self.tokens(connector)?;
        parse_repository(&self.api(&format!("/repositories/{id}"), &tokens.access_token)?)
    }
    fn clone_repository(
        &self,
        connector: &Connector,
        repository: &RemoteRepository,
        path: &Path,
    ) -> Result<()> {
        parse_remote(&repository.full_name, &repository.clone_url)?;
        // The helper reads the selected connection through native storage. Neither
        // the token nor a credential-bearing URL enters argv or Git configuration.
        let executable = std::env::current_exe()
            .map_err(|_| Error::Git("cannot locate native credential helper".into()))?;
        let exe = executable
            .to_str()
            .ok_or_else(|| Error::Git("credential helper path must be UTF-8".into()))?;
        let helper = format!(
            "!{} --spacer-credential {} {}",
            shell_quote(exe),
            shell_quote(&connector.id),
            shell_quote(&format!("{}.git", repository.full_name))
        );
        let mut command = crate::git::command();
        command.args([
            "-c",
            "credential.helper=",
            "-c",
            &format!("credential.helper={helper}"),
            "-c",
            "credential.useHttpPath=true",
            "-c",
            "core.hooksPath=",
            "-c",
            "http.followRedirects=false",
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
            "clone",
            "--",
            &repository.clone_url,
        ]);
        command.arg(crate::git::git_path(path));
        command
            .env("GIT_TERMINAL_PROMPT", "0")
            .env_remove("GIT_ASKPASS")
            .env_remove("SSH_ASKPASS");
        let output = command
            .output()
            .map_err(|_| Error::Git("could not start Git clone".into()))?;
        if !output.status.success() {
            return Err(Error::Git(format!("clone failed; check connector access and inspect '{}' before retrying; files are preserved", path.display())));
        }
        Ok(())
    }
}
fn gh() -> Command {
    let mut command = Command::new("gh");
    for key in [
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "GH_ENTERPRISE_TOKEN",
        "GITHUB_ENTERPRISE_TOKEN",
    ] {
        command.env_remove(key);
    }
    command
}
fn gh_token(login: &str) -> Result<Zeroizing<String>> {
    let mut output = gh()
        .args(["auth", "token", "--hostname", "github.com", "--user", login])
        .output()
        .map_err(|_| {
            Error::Authentication("GitHub CLI unavailable; reconnect with GitHub".into())
        })?;
    if !output.status.success() {
        output.stdout.zeroize();
        return Err(Error::Authentication(
            "selected GitHub CLI credentials unavailable; reconnect".into(),
        ));
    }
    let bytes = Zeroizing::new(std::mem::take(&mut output.stdout));
    let token = Zeroizing::new(
        std::str::from_utf8(&bytes)
            .map_err(|_| secure_error())?
            .trim()
            .to_string(),
    );
    Ok(token)
}
fn response(request: RequestBuilder) -> Result<Value> {
    let response = request.send().map_err(|_| {
        Error::Remote("GitHub request failed or timed out; check connectivity and retry".into())
    })?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        if status == 400 {
            let value = response.json::<Value>().unwrap_or(Value::Null);
            return Err(match value.get("error").and_then(Value::as_str) {
                Some("device_flow_disabled") => Error::Authentication("device flow is disabled for this OAuth App; its owner must enable Device flow in the app settings before signing in".into()),
                Some("incorrect_client_credentials") => Error::Authentication("GitHub OAuth App client ID is invalid; check SPACER_GITHUB_CLIENT_ID".into()),
                _ => Error::Authentication("GitHub rejected authorization; reconnect or check the App configuration".into()),
            });
        }
        return Err(match status {
            401 => Error::Authentication("GitHub credentials are invalid or expired; reconnect".into()),
            403 => Error::Remote("GitHub denied access; check OAuth App organization approval, repository permissions, SSO, or rate limits".into()),
            404 => Error::Remote("GitHub repository is unavailable to this connector".into()),
            429 => Error::Remote("GitHub rate limit reached; retry later".into()),
            _ => Error::Remote(format!("GitHub request failed (HTTP {}); retry later", status)),
        });
    }
    response
        .json()
        .map_err(|_| Error::Remote("invalid GitHub response".into()))
}
fn parse_tokens(value: &mut Value, client_id: &str) -> Result<Tokens> {
    let result = (|| {
        if !value
            .get("scope")
            .and_then(Value::as_str)
            .is_some_and(|scopes| scopes.split([',', ' ']).any(|s| s == "repo"))
        {
            return Err(Error::Authentication("GitHub did not grant the required repo scope; reconnect with an OAuth App and approve repository access".into()));
        }
        Ok(Tokens {
            access_token: string(value, "access_token")?.into(),
            refresh_token: value
                .get("refresh_token")
                .and_then(Value::as_str)
                .map(Into::into),
            expires_at: value
                .get("expires_in")
                .and_then(Value::as_u64)
                .map(|s| now() + s),
            client_id: Some(client_id.into()),
        })
    })();
    wipe_value(value);
    result
}
fn wipe_value(value: &mut Value) {
    match value {
        Value::String(s) => s.zeroize(),
        Value::Object(o) => {
            for v in o.values_mut() {
                wipe_value(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                wipe_value(v);
            }
        }
        _ => (),
    }
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Remote("GitHub response missing required metadata".into()))
}
fn numeric_id(value: &Value) -> Result<String> {
    value
        .get("id")
        .and_then(Value::as_u64)
        .map(|id| id.to_string())
        .ok_or_else(|| Error::Remote("GitHub response missing stable ID".into()))
}
fn parse_repository(value: &Value) -> Result<RemoteRepository> {
    let full_name = string(value, "full_name")?;
    let clone_url = string(value, "clone_url")?;
    parse_remote(full_name, clone_url)?;
    Ok(RemoteRepository {
        id: numeric_id(value)?,
        full_name: full_name.into(),
        clone_url: clone_url.into(),
        private: value
            .get("private")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}
fn parse_remote(full_name: &str, url: &str) -> Result<()> {
    if full_name.split('/').count() != 2
        || full_name.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || !s
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        })
        || url != format!("https://github.com/{full_name}.git")
    {
        return Err(Error::InvalidOperation(
            "invalid GitHub clone URL; refusing credentials to another destination".into(),
        ));
    }
    Ok(())
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
/// Hidden entry point shared by both executables; writes only Git's credential protocol.
pub fn credential_helper(args: &[String]) -> Result<()> {
    if args.len() != 3 || args[2] != "get" {
        return Ok(());
    }
    let mut input = String::new();
    std::io::stdin()
        .take(8192)
        .read_to_string(&mut input)
        .map_err(|_| secure_error())?;
    let fields: HashMap<_, _> = input
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    if fields.get("protocol") != Some(&"https")
        || fields.get("host") != Some(&"github.com")
        || fields.get("path") != Some(&args[1].as_str())
    {
        return Ok(());
    }
    let catalog = JsonCatalog::new(catalog_directory()?);
    let connector = catalog
        .load()?
        .connectors
        .into_iter()
        .find(|c| c.id == args[0])
        .ok_or_else(|| Error::Authentication("connector not found".into()))?;
    let github = Github::new(Arc::new(NativeSecrets))?;
    let token = github.tokens(&connector)?;
    write!(
        std::io::stdout(),
        "username=x-access-token\npassword={}\n\n",
        token.access_token
    )
    .map_err(|_| secure_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    #[derive(Default)]
    struct MemorySecrets(Mutex<HashMap<String, String>>);
    impl Secrets for MemorySecrets {
        fn get(&self, id: &str) -> Result<Zeroizing<String>> {
            self.0
                .lock()
                .unwrap()
                .get(id)
                .cloned()
                .map(Zeroizing::new)
                .ok_or_else(secure_error)
        }
        fn put(&self, id: &str, value: &str) -> Result<()> {
            self.0.lock().unwrap().insert(id.into(), value.into());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<()> {
            self.0.lock().unwrap().remove(id);
            Ok(())
        }
    }
    fn server(responses: Vec<(u16, Value)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, response) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut chunk = [0; 4096];
                loop {
                    let size = stream.read(&mut chunk).unwrap();
                    if size == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..size]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..end]);
                        let length = head
                            .lines()
                            .find_map(|l| {
                                l.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|s| s.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let body = serde_json::to_string(&response).unwrap();
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (address, handle)
    }
    fn adapter(address: String, store: Arc<dyn Secrets>, directory: &Path) -> Github {
        Github {
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            secrets: store,
            pending: Mutex::new(HashMap::new()),
            api_base: address.clone(),
            oauth_base: address,
            credential_locks: directory.into(),
        }
    }
    #[test]
    #[ignore = "requires a user credential-manager session; writes and removes an isolated test item"]
    fn native_secure_store_roundtrip() {
        let id = uuid::Uuid::new_v4().to_string();
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = NativeSecrets.delete(&self.0);
            }
        }
        let cleanup = Cleanup(id.clone());
        NativeSecrets
            .put(&id, "spacer-credential-store-probe")
            .unwrap();
        assert_eq!(
            NativeSecrets.get(&id).unwrap().as_str(),
            "spacer-credential-store-probe"
        );
        NativeSecrets.delete(&id).unwrap();
        assert!(NativeSecrets.get(&id).is_err());
        drop(cleanup);
    }
    #[test]
    fn device_login_stores_only_completed_credentials_and_honors_slow_down() {
        let (address, server) = server(vec![
            (
                200,
                serde_json::json!({"device_code":"test-device-secret","user_code":"ABCD-EFGH","interval":1,"expires_in":900}),
            ),
            (200, serde_json::json!({"error":"slow_down"})),
            (
                200,
                serde_json::json!({"access_token":"test-access","scope":"repo","refresh_token":"test-refresh","expires_in":28800}),
            ),
            (200, serde_json::json!({"id":42,"login":"example"})),
        ]);
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(MemorySecrets::default());
        let github = adapter(address, store.clone(), directory.path());
        let id = uuid::Uuid::new_v4().to_string();
        let challenge = github.begin_login(&id, "public-client").unwrap();
        assert_eq!(challenge.user_code, "ABCD-EFGH");
        assert!(store.0.lock().unwrap().is_empty());
        assert!(github.poll_login(&id).unwrap().is_none());
        github
            .pending
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .next_poll = 0;
        assert!(github.poll_login(&id).unwrap().is_none());
        assert_eq!(github.pending.lock().unwrap().get(&id).unwrap().interval, 6);
        github
            .pending
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .next_poll = 0;
        assert_eq!(github.poll_login(&id).unwrap().unwrap().id, "42");
        let encoded = store.get(&id).unwrap();
        assert!(encoded.contains("test-refresh"));
        assert!(!encoded.contains("test-device-secret"));
        assert!(github.pending.lock().unwrap().is_empty());
        let requests = server.join().unwrap();
        assert!(requests[0].contains("client_id=public-client"));
        assert!(requests[0].contains("scope=repo+offline_access"));
        assert!(!requests
            .iter()
            .any(|request| request.contains("client_secret")));
        assert!(requests[2].contains("device_code=test-device-secret"));
    }
    #[test]
    fn cancelled_expired_and_denied_authorizations_never_store_credentials() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(MemorySecrets::default());
        let github = adapter("http://127.0.0.1:1".into(), store.clone(), directory.path());
        for expired in [false, true] {
            let id = uuid::Uuid::new_v4().to_string();
            github.pending.lock().unwrap().insert(
                id.clone(),
                Pending {
                    device_code: Zeroizing::new("secret".into()),
                    client_id: "public".into(),
                    interval: 5,
                    next_poll: 0,
                    expires_at: if expired { 0 } else { u64::MAX },
                },
            );
            if !expired {
                github.cancel_login(&id).unwrap();
            }
            assert!(github.poll_login(&id).is_err());
        }
        assert!(store.0.lock().unwrap().is_empty());
    }
    #[test]
    fn refresh_rotates_credentials_then_verifies_the_selected_account() {
        let (address, server) = server(vec![
            (
                200,
                serde_json::json!({"access_token":"new-access","scope":"repo","refresh_token":"new-refresh","expires_in":28800}),
            ),
            (200, serde_json::json!({"id":99,"login":"other"})),
        ]);
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(MemorySecrets::default());
        let github = adapter(address, store.clone(), directory.path());
        let id = uuid::Uuid::new_v4().to_string();
        github
            .store(
                &id,
                &Tokens {
                    access_token: "old-access".into(),
                    refresh_token: Some("old-refresh".into()),
                    expires_at: Some(0),
                    client_id: Some("public-client".into()),
                },
            )
            .unwrap();
        let connector = Connector {
            id: id.clone(),
            label: "Work".into(),
            host: "github.com".into(),
            account_id: "42".into(),
            login: "expected".into(),
            source: CredentialSource::Keychain,
        };
        assert!(matches!(
            github.tokens(&connector),
            Err(Error::Authentication(_))
        ));
        let encoded = store.get(&id).unwrap();
        assert!(encoded.contains("new-refresh"));
        assert!(!encoded.contains("old-refresh"));
        let requests = server.join().unwrap();
        assert!(requests[0].contains("refresh_token=old-refresh"));
        assert!(!requests[0].contains("client_secret"));
    }
    #[test]
    fn secure_storage_failure_does_not_publish_a_completed_connection() {
        struct Denied;
        impl Secrets for Denied {
            fn get(&self, _: &str) -> Result<Zeroizing<String>> {
                Err(secure_error())
            }
            fn put(&self, _: &str, _: &str) -> Result<()> {
                Err(secure_error())
            }
            fn delete(&self, _: &str) -> Result<()> {
                Ok(())
            }
        }
        let (address, server) = server(vec![
            (
                200,
                serde_json::json!({"access_token":"test-private-access","scope":"repo","refresh_token":"test-private-refresh","expires_in":28800}),
            ),
            (200, serde_json::json!({"id":42,"login":"example"})),
        ]);
        let directory = tempfile::tempdir().unwrap();
        let github = Arc::new(adapter(address, Arc::new(Denied), directory.path()));
        let mut connections = Connections::new(
            JsonCatalog::new(directory.path().join("catalog")),
            github.clone(),
        );
        // Inject a pending provider challenge without initiating live authorization.
        let id = uuid::Uuid::new_v4().to_string();
        github.pending.lock().unwrap().insert(
            id.clone(),
            Pending {
                device_code: Zeroizing::new("test-device".into()),
                client_id: "public-client".into(),
                expires_at: u64::MAX,
                interval: 1,
                next_poll: 0,
            },
        );
        let error = github.poll_login(&id).unwrap_err().to_string();
        assert!(error.contains("no plaintext fallback"));
        assert!(!error.contains("test-private"));
        github.cancel_login(&id).unwrap();
        assert!(connections.snapshot().unwrap().connectors.is_empty());
        assert!(!directory.path().join("catalog/accounts.json").exists());
        server.join().unwrap();
        // No provider ticket remains reusable after cancellation.
        assert!(connections.poll_login(&id).is_err());
    }
    #[test]
    fn oauth_repository_discovery_paginates_without_installations() {
        let repository = |id: u64| serde_json::json!({"id":id,"full_name":format!("organization/repo{id}"),"clone_url":format!("https://github.com/organization/repo{id}.git"),"private":true});
        let first_page: Vec<_> = (1..=100).map(repository).collect();
        let (address, server) = server(vec![
            (200, serde_json::json!({"id":42,"login":"example"})),
            (200, serde_json::json!(first_page)),
            (200, serde_json::json!([repository(100), repository(101)])),
        ]);
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(MemorySecrets::default());
        let github = adapter(address, store, directory.path());
        let id = uuid::Uuid::new_v4().to_string();
        github
            .store(
                &id,
                &Tokens {
                    access_token: "gho_test".into(),
                    refresh_token: None,
                    expires_at: None,
                    client_id: None,
                },
            )
            .unwrap();
        let connector = Connector {
            id,
            label: "Work".into(),
            host: "github.com".into(),
            account_id: "42".into(),
            login: "example".into(),
            source: CredentialSource::Keychain,
        };
        let repos = github.repositories(&connector).unwrap();
        assert_eq!(repos.len(), 101);
        assert!(repos
            .iter()
            .any(|r| r.full_name == "organization/repo101" && r.private));
        let requests = server.join().unwrap();
        assert!(requests[1].starts_with("GET /user/repos?per_page=100&page=1&"));
        assert!(requests[2].starts_with("GET /user/repos?per_page=100&page=2&"));
        assert!(!requests.iter().any(|r| r.contains("installations")));
    }
    #[test]
    fn oauth_tokens_require_repo_scope_and_can_be_nonexpiring() {
        let mut value = serde_json::json!({"access_token":"test-token","scope":"repo,read:org","token_type":"bearer"});
        let tokens = parse_tokens(&mut value, "oauth-client").unwrap();
        assert!(tokens.refresh_token.is_none());
        assert!(tokens.expires_at.is_none());
        assert_eq!(tokens.access_token, "test-token");
        assert_eq!(value["access_token"], "");
        let mut restricted = serde_json::json!({"access_token":"test-token","scope":"read:user"});
        assert!(parse_tokens(&mut restricted, "oauth-client").is_err());
        assert_eq!(restricted["access_token"], "");
    }
    #[test]
    fn github_app_client_ids_are_rejected_before_network_access() {
        let directory = tempfile::tempdir().unwrap();
        let github = adapter(
            "http://127.0.0.1:1".into(),
            Arc::new(MemorySecrets::default()),
            directory.path(),
        );
        assert!(github
            .begin_login("fixture-id", "Iv23fixture")
            .unwrap_err()
            .to_string()
            .contains("OAuth App"));
        assert!(validate_client_id("")
            .unwrap_err()
            .to_string()
            .contains("not configured"));
    }
    #[test]
    fn errors_do_not_echo_remote_secrets_and_clone_urls_are_constrained() {
        let (address, server) = server(vec![(
            403,
            serde_json::json!({"message":"test-sensitive-token"}),
        )]);
        let directory = tempfile::tempdir().unwrap();
        let github = adapter(
            address,
            Arc::new(MemorySecrets::default()),
            directory.path(),
        );
        assert!(!github
            .account("test-sensitive-token")
            .unwrap_err()
            .to_string()
            .contains("test-sensitive-token"));
        server.join().unwrap();
        assert!(parse_remote("owner/repo", "https://github.com/owner/repo.git").is_ok());
        for url in [
            "https://evil.invalid/owner/repo.git",
            "https://token@github.com/owner/repo.git",
            "http://github.com/owner/repo.git",
        ] {
            assert!(parse_remote("owner/repo", url).is_err());
        }
        assert!(parse_remote("owner/../repo", "https://github.com/owner/../repo.git").is_err());
    }
}
