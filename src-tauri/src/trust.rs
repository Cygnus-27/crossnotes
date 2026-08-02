//! The set of peers this device has paired with, and the keys pinned to them.
//!
//! Pairing writes a `(device_id -> fingerprint)` entry here. Every subsequent
//! connection re-checks the peer's key against that pin, so a device is
//! authenticated exactly once by a human and by cryptography thereafter.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::crypto::Fingerprint;
use crate::error::{Error, Result};
use crate::store;

const TRUST_FILE: &str = "trusted-peers.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPeer {
    pub device_id: String,
    pub device_name: String,
    pub fingerprint: Fingerprint,
    pub paired_at: u64,
    /// Skip the accept prompt for this peer. Opt-in, per device, and only
    /// meaningful because the peer's key is already pinned.
    #[serde(default)]
    pub auto_accept: bool,
}

/// Outcome of checking a peer's presented key against what we have pinned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustVerdict {
    /// Key matches the pin. Proceed.
    Trusted(Box<TrustedPeer>),
    /// We have never seen this device. Needs pairing confirmation.
    Unknown,
    /// We know this device ID but the key changed. Either the peer reinstalled
    /// the app, or someone is impersonating it. Never resolve this silently.
    Mismatch { expected: Fingerprint },
}

#[derive(Default, Serialize, Deserialize)]
struct StoredTrust {
    #[serde(default)]
    peers: Vec<TrustedPeer>,
}

pub struct TrustStore {
    path: PathBuf,
    peers: Mutex<HashMap<String, TrustedPeer>>,
}

impl TrustStore {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(TRUST_FILE);
        let stored: StoredTrust = store::read_json(&path)?.unwrap_or_default();
        let peers = stored
            .peers
            .into_iter()
            .map(|peer| (peer.device_id.clone(), peer))
            .collect();

        Ok(TrustStore {
            path,
            peers: Mutex::new(peers),
        })
    }

    /// Decide whether a presented key is acceptable for a given device ID.
    ///
    /// Note that a mismatch is reported rather than corrected. Silently
    /// re-pinning on change would make the whole scheme trust-on-every-use
    /// instead of trust-on-first-use, and defeat the point.
    pub fn verify(&self, device_id: &str, presented: &Fingerprint) -> TrustVerdict {
        let peers = self.peers.lock().unwrap();
        match peers.get(device_id) {
            None => TrustVerdict::Unknown,
            Some(peer) if peer.fingerprint == *presented => {
                TrustVerdict::Trusted(Box::new(peer.clone()))
            }
            Some(peer) => TrustVerdict::Mismatch {
                expected: peer.fingerprint,
            },
        }
    }

    pub fn get(&self, device_id: &str) -> Option<TrustedPeer> {
        self.peers.lock().unwrap().get(device_id).cloned()
    }

    pub fn is_trusted(&self, device_id: &str, presented: &Fingerprint) -> bool {
        matches!(self.verify(device_id, presented), TrustVerdict::Trusted(_))
    }

    pub fn auto_accepts(&self, device_id: &str, presented: &Fingerprint) -> bool {
        match self.verify(device_id, presented) {
            TrustVerdict::Trusted(peer) => peer.auto_accept,
            _ => false,
        }
    }

    pub fn list(&self) -> Vec<TrustedPeer> {
        let mut peers: Vec<_> = self.peers.lock().unwrap().values().cloned().collect();
        peers.sort_by_key(|peer| peer.device_name.to_lowercase());
        peers
    }

    /// Pin a peer. Called only after the user has confirmed the pairing code
    /// on both devices — never automatically.
    pub fn pair(
        &self,
        device_id: &str,
        device_name: &str,
        fingerprint: Fingerprint,
    ) -> Result<TrustedPeer> {
        let peer = TrustedPeer {
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            fingerprint,
            paired_at: unix_now(),
            auto_accept: false,
        };

        self.peers
            .lock()
            .unwrap()
            .insert(device_id.to_string(), peer.clone());
        self.persist()?;
        Ok(peer)
    }

    pub fn forget(&self, device_id: &str) -> Result<()> {
        self.peers.lock().unwrap().remove(device_id);
        self.persist()
    }

    pub fn set_auto_accept(&self, device_id: &str, auto_accept: bool) -> Result<()> {
        {
            let mut peers = self.peers.lock().unwrap();
            let peer = peers
                .get_mut(device_id)
                .ok_or_else(|| Error::UnknownPeer(device_id.to_string()))?;
            peer.auto_accept = auto_accept;
        }
        self.persist()
    }

    /// Keep the stored display name current when a peer renames itself. The
    /// name is cosmetic and carries no authority, so following it is safe —
    /// but the fingerprint it is stored against is never touched here.
    pub fn refresh_name(&self, device_id: &str, device_name: &str) {
        let changed = {
            let mut peers = self.peers.lock().unwrap();
            match peers.get_mut(device_id) {
                Some(peer) if peer.device_name != device_name => {
                    peer.device_name = device_name.to_string();
                    true
                }
                _ => false,
            }
        };
        if changed {
            let _ = self.persist();
        }
    }

    fn persist(&self) -> Result<()> {
        let stored = StoredTrust {
            peers: self.peers.lock().unwrap().values().cloned().collect(),
        };
        store::write_json(&self.path, &stored)
    }
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::DeviceKey;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fluqsr-trust-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fingerprint() -> Fingerprint {
        DeviceKey::generate().unwrap().fingerprint()
    }

    #[test]
    fn unknown_peers_are_not_trusted() {
        let dir = temp_dir("unknown");
        let store = TrustStore::load(&dir).unwrap();
        assert_eq!(store.verify("nobody", &fingerprint()), TrustVerdict::Unknown);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn paired_peers_are_trusted() {
        let dir = temp_dir("paired");
        let store = TrustStore::load(&dir).unwrap();
        let fp = fingerprint();

        store.pair("laptop", "Athar's Laptop", fp).unwrap();

        assert!(matches!(
            store.verify("laptop", &fp),
            TrustVerdict::Trusted(_)
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_substituted_key_is_reported_as_a_mismatch() {
        // The impersonation case: right device ID, wrong key. This must never
        // silently succeed, and must not be reported as merely "unknown"
        // either, since that would let an attacker trigger a fresh pairing
        // prompt and hope the user clicks through it.
        let dir = temp_dir("mismatch");
        let store = TrustStore::load(&dir).unwrap();
        let genuine = fingerprint();
        let attacker = fingerprint();

        store.pair("laptop", "Athar's Laptop", genuine).unwrap();

        match store.verify("laptop", &attacker) {
            TrustVerdict::Mismatch { expected } => assert_eq!(expected, genuine),
            other => panic!("expected a mismatch, got {other:?}"),
        }
        assert!(!store.is_trusted("laptop", &attacker));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pins_survive_a_restart() {
        let dir = temp_dir("persist");
        let fp = fingerprint();

        TrustStore::load(&dir)
            .unwrap()
            .pair("phone", "Pixel", fp)
            .unwrap();

        let reloaded = TrustStore::load(&dir).unwrap();
        assert!(reloaded.is_trusted("phone", &fp));
        assert_eq!(reloaded.list().len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn forgetting_a_peer_revokes_trust() {
        let dir = temp_dir("forget");
        let store = TrustStore::load(&dir).unwrap();
        let fp = fingerprint();

        store.pair("phone", "Pixel", fp).unwrap();
        store.forget("phone").unwrap();

        assert_eq!(store.verify("phone", &fp), TrustVerdict::Unknown);
        assert!(TrustStore::load(&dir).unwrap().list().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_accept_is_off_until_explicitly_enabled() {
        let dir = temp_dir("auto-accept");
        let store = TrustStore::load(&dir).unwrap();
        let fp = fingerprint();

        store.pair("desk", "Desktop", fp).unwrap();
        assert!(
            !store.auto_accepts("desk", &fp),
            "pairing alone must not grant silent acceptance"
        );

        store.set_auto_accept("desk", true).unwrap();
        assert!(store.auto_accepts("desk", &fp));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_accept_does_not_apply_to_a_substituted_key() {
        let dir = temp_dir("auto-accept-mismatch");
        let store = TrustStore::load(&dir).unwrap();
        let genuine = fingerprint();

        store.pair("desk", "Desktop", genuine).unwrap();
        store.set_auto_accept("desk", true).unwrap();

        assert!(
            !store.auto_accepts("desk", &fingerprint()),
            "auto-accept must be gated on the pinned key, not the device ID"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn renaming_a_peer_keeps_its_pin() {
        let dir = temp_dir("rename");
        let store = TrustStore::load(&dir).unwrap();
        let fp = fingerprint();

        store.pair("phone", "Pixel", fp).unwrap();
        store.refresh_name("phone", "Pixel 9");

        assert!(store.is_trusted("phone", &fp));
        assert_eq!(store.get("phone").unwrap().device_name, "Pixel 9");

        std::fs::remove_dir_all(&dir).ok();
    }
}
