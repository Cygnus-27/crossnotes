//! This device's own identity: a stable ID, a human-readable name, and the
//! long-lived keypair everything else authenticates with.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::crypto::{DeviceKey, Fingerprint};
use crate::error::Result;
use crate::store;

const IDENTITY_FILE: &str = "identity.json";

/// What gets written to disk. The private key lives here in PEM form, so the
/// file is created with owner-only permissions where the platform supports it.
#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    device_id: String,
    device_name: String,
    key_pem: String,
}

/// The public half of an identity — safe to broadcast on the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub fingerprint: Fingerprint,
}

pub struct Identity {
    pub device_id: String,
    pub device_name: String,
    pub key: DeviceKey,
}

impl Identity {
    /// Load the identity from `dir`, generating and persisting a new one on
    /// first run.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        if let Some(stored) = store::read_json::<StoredIdentity>(&dir.join(IDENTITY_FILE))? {
            if let Ok(key) = DeviceKey::from_pem(&stored.key_pem) {
                return Ok(Identity {
                    device_id: stored.device_id,
                    device_name: stored.device_name,
                    key,
                });
            }
            // A corrupt key is not recoverable — regenerating gives the user a
            // working app again, at the cost of needing to re-pair. Better
            // than refusing to start.
        }

        let identity = Identity {
            device_id: uuid::Uuid::new_v4().to_string(),
            device_name: default_device_name(),
            key: DeviceKey::generate()?,
        };
        identity.persist(dir)?;
        Ok(identity)
    }

    pub fn persist(&self, dir: &Path) -> Result<()> {
        let stored = StoredIdentity {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            key_pem: self.key.to_pem(),
        };
        let path = dir.join(IDENTITY_FILE);
        store::write_json(&path, &stored)?;
        store::restrict_to_owner(&path);
        Ok(())
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.key.fingerprint()
    }

    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            platform: current_platform().to_string(),
            fingerprint: self.fingerprint(),
        }
    }
}

pub fn current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else {
        "unknown"
    }
}

/// A name the user will recognise in someone else's peer list. Falls back
/// through the usual environment variables before giving up on something
/// generic — this is cosmetic, so it must never fail.
fn default_device_name() -> String {
    for key in ["COMPUTERNAME", "HOSTNAME", "HOST"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    #[cfg(unix)]
    if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    for key in ["USERNAME", "USER"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return format!("{trimmed}'s {}", current_platform());
            }
        }
    }

    format!("fluqsr {}", current_platform())
}

/// Where identity, trust store, and settings live.
pub fn data_dir(app_data: PathBuf) -> Result<PathBuf> {
    std::fs::create_dir_all(&app_data)?;
    Ok(app_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fluqsr-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn identity_is_stable_across_restarts() {
        let dir = temp_dir("identity");

        let first = Identity::load_or_create(&dir).unwrap();
        let second = Identity::load_or_create(&dir).unwrap();

        assert_eq!(first.device_id, second.device_id);
        assert_eq!(
            first.fingerprint(),
            second.fingerprint(),
            "reloading must not rotate the key, or every peer would need re-pairing"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn separate_devices_get_separate_identities() {
        let a = temp_dir("identity-a");
        let b = temp_dir("identity-b");

        let first = Identity::load_or_create(&a).unwrap();
        let second = Identity::load_or_create(&b).unwrap();

        assert_ne!(first.device_id, second.device_id);
        assert_ne!(first.fingerprint(), second.fingerprint());

        std::fs::remove_dir_all(&a).ok();
        std::fs::remove_dir_all(&b).ok();
    }

    #[test]
    fn a_corrupt_key_regenerates_rather_than_failing_to_start() {
        let dir = temp_dir("identity-corrupt");
        Identity::load_or_create(&dir).unwrap();

        std::fs::write(
            dir.join(IDENTITY_FILE),
            r#"{"device_id":"x","device_name":"y","key_pem":"not a key"}"#,
        )
        .unwrap();

        let recovered = Identity::load_or_create(&dir).unwrap();
        assert!(!recovered.device_id.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn device_name_is_never_empty() {
        assert!(!default_device_name().trim().is_empty());
    }
}
