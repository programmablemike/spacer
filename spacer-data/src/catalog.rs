use crate::connection_dto::*;
use spacer_core::{
    connections::{Catalog, CatalogSnapshot},
    Error, Result,
};
use std::{fs::OpenOptions, io::Write, path::PathBuf, sync::Mutex};

pub struct JsonCatalog {
    directory: PathBuf,
    baseline: Mutex<Option<Option<Vec<u8>>>>,
}
impl JsonCatalog {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            baseline: Mutex::new(None),
        }
    }
    fn bytes(&self) -> Result<Option<Vec<u8>>> {
        match std::fs::read(self.directory.join("accounts.json")) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(Error::Persistence(
                "cannot read user catalog; check permissions".into(),
            )),
        }
    }
}
impl Catalog for JsonCatalog {
    fn new_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn load(&self) -> Result<CatalogSnapshot> {
        let mut baseline = self
            .baseline
            .lock()
            .map_err(|_| Error::Persistence("catalog lock interrupted".into()))?;
        *baseline = None;
        let bytes = self.bytes()?;
        let state = if let Some(bytes) = &bytes {
            let stored: StoredCatalog = serde_json::from_slice(bytes).map_err(|_| {
                Error::Persistence("invalid user catalog; inspect or restore accounts.json".into())
            })?;
            if stored.schema_version != 1 {
                return Err(Error::Persistence(
                    "unsupported user catalog version; upgrade Spacer".into(),
                ));
            }
            CatalogSnapshot {
                identities: stored.identities.into_iter().map(Into::into).collect(),
                connectors: stored
                    .connectors
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_>>()?,
            }
        } else {
            CatalogSnapshot::default()
        };
        spacer_core::connections::validate_catalog(&state)?;
        let mut ids = std::collections::HashSet::new();
        let mut labels = std::collections::HashSet::new();
        for i in &state.identities {
            if uuid::Uuid::parse_str(&i.id).is_err()
                || !ids.insert(&i.id)
                || !labels.insert((&i.label, "identity"))
            {
                return Err(Error::Persistence(
                    "duplicate or invalid identity in user catalog".into(),
                ));
            }
        }
        for c in &state.connectors {
            if !ids.insert(&c.id) || !labels.insert((&c.label, "connector")) {
                return Err(Error::Persistence(
                    "duplicate connector in user catalog".into(),
                ));
            }
        }
        *baseline = Some(bytes);
        Ok(state)
    }
    fn save(&self, state: &CatalogSnapshot) -> Result<()> {
        let mut baseline = self
            .baseline
            .lock()
            .map_err(|_| Error::Persistence("catalog lock interrupted".into()))?;
        let expected = baseline
            .as_ref()
            .ok_or_else(|| Error::Persistence("load catalog before saving".into()))?;
        std::fs::create_dir_all(&self.directory)
            .map_err(|_| Error::Persistence("cannot create user configuration directory".into()))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join("accounts.lock"))
            .map_err(|_| Error::Persistence("cannot open catalog lock".into()))?;
        lock.try_lock()
            .map_err(|_| Error::Persistence("user catalog is busy; retry".into()))?;
        let _guard = crate::registry::RegistryLock(lock);
        if &self.bytes()? != expected {
            return Err(Error::Persistence(
                "user catalog changed; reopen and retry".into(),
            ));
        }
        let stored = StoredCatalog {
            schema_version: 1,
            identities: state
                .identities
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            connectors: state
                .connectors
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&stored)
            .map_err(|_| Error::Persistence("cannot serialize user catalog".into()))?;
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(|_| Error::Persistence("cannot create temporary user catalog".into()))?;
        file.write_all(&bytes)
            .and_then(|_| file.as_file().sync_all())
            .map_err(|_| Error::Persistence("cannot write user catalog".into()))?;
        file.persist(self.directory.join("accounts.json"))
            .map_err(|_| Error::Persistence("cannot replace user catalog".into()))?;
        *baseline = Some(Some(bytes));
        #[cfg(unix)]
        std::fs::File::open(&self.directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| {
                Error::Persistence(
                    "catalog written but directory sync failed; reopen to inspect".into(),
                )
            })?;
        Ok(())
    }
}
/// User-level metadata location. The override supports isolated sessions and tests.
pub fn catalog_directory() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SPACER_CONFIG_DIR") {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(Error::InvalidOperation(
                "SPACER_CONFIG_DIR must be absolute".into(),
            ));
        }
        return Ok(path);
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    let base = base.filter(|p| p.is_absolute()).ok_or_else(|| {
        Error::InvalidOperation(
            "set an absolute SPACER_CONFIG_DIR to use connectors and identities".into(),
        )
    })?;
    Ok(base.join("spacer"))
}
