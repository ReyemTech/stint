use crate::Result;
use keyring::Entry;

pub const DEFAULT_SERVICE_PREFIX: &str = "tech.reyem.stint";

#[derive(Clone)]
pub struct Secrets {
    prefix: String,
}

impl Default for Secrets {
    fn default() -> Self {
        // STINT_SECRET_PREFIX lets test harnesses redirect keychain writes
        // to a synthetic prefix so they never touch a developer's real
        // tech.reyem.stint.* entries (and the ACLs that go with them).
        // Mirrors how STINT_DB redirects the database path.
        let prefix = std::env::var("STINT_SECRET_PREFIX")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_SERVICE_PREFIX.to_string());
        Self { prefix }
    }
}

impl Secrets {
    pub fn with_service_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    fn service_for(&self, key: &str) -> String {
        format!("{}.{}", self.prefix, key)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let entry = Entry::new(&self.service_for(key), "stint")?;
        match entry.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let entry = Entry::new(&self.service_for(key), "stint")?;
        entry.set_password(value)?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let entry = Entry::new(&self.service_for(key), "stint")?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
