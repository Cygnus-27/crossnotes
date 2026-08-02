//! User-adjustable settings, persisted alongside the identity and trust store.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::discovery::DEFAULT_TRANSFER_PORT;
use crate::error::Result;
use crate::store;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Where received files land. Every incoming write is confined to this
    /// directory — see [`crate::paths::resolve_within`].
    pub receive_dir: PathBuf,
    pub transfer_port: u16,
    /// Name shown to other devices. Cosmetic; carries no authority.
    pub device_name: String,
}

impl Settings {
    pub fn defaults(device_name: String) -> Self {
        Settings {
            receive_dir: default_receive_dir(),
            transfer_port: DEFAULT_TRANSFER_PORT,
            device_name,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    settings: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(dir: &Path, device_name: String) -> Result<Self> {
        let path = dir.join(SETTINGS_FILE);
        let settings = store::read_json::<Settings>(&path)?
            .unwrap_or_else(|| Settings::defaults(device_name));

        // The configured folder may have been deleted or moved since last run.
        // Recreating it is friendlier than failing every incoming transfer.
        let _ = std::fs::create_dir_all(&settings.receive_dir);

        Ok(SettingsStore {
            path,
            settings: Mutex::new(settings),
        })
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn receive_dir(&self) -> PathBuf {
        self.settings.lock().unwrap().receive_dir.clone()
    }

    pub fn transfer_port(&self) -> u16 {
        self.settings.lock().unwrap().transfer_port
    }

    pub fn device_name(&self) -> String {
        self.settings.lock().unwrap().device_name.clone()
    }

    pub fn set_receive_dir(&self, dir: PathBuf) -> Result<()> {
        std::fs::create_dir_all(&dir)?;
        self.settings.lock().unwrap().receive_dir = dir;
        self.persist()
    }

    pub fn set_device_name(&self, name: String) -> Result<()> {
        let trimmed: String = name.trim().chars().take(64).collect();
        if !trimmed.is_empty() {
            self.settings.lock().unwrap().device_name = trimmed;
        }
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        let settings = self.settings.lock().unwrap().clone();
        store::write_json(&self.path, &settings)
    }
}

/// Default landing spot: a `fluqsr` folder inside the user's Downloads, or the
/// home directory if Downloads cannot be located.
fn default_receive_dir() -> PathBuf {
    let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
    let downloads = home.join("Downloads");

    if downloads.is_dir() {
        downloads.join("fluqsr")
    } else {
        home.join("fluqsr")
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fluqsr-settings-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_are_used_on_first_run() {
        let dir = temp_dir("defaults");
        let settings = SettingsStore::load(&dir, "Laptop".into()).unwrap();

        assert_eq!(settings.device_name(), "Laptop");
        assert_eq!(settings.transfer_port(), DEFAULT_TRANSFER_PORT);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn changes_survive_a_restart() {
        let dir = temp_dir("persist");
        let receive = temp_dir("persist-receive");

        let settings = SettingsStore::load(&dir, "Laptop".into()).unwrap();
        settings.set_receive_dir(receive.clone()).unwrap();
        settings.set_device_name("Athar's Laptop".into()).unwrap();

        let reloaded = SettingsStore::load(&dir, "ignored".into()).unwrap();
        assert_eq!(reloaded.device_name(), "Athar's Laptop");
        assert_eq!(reloaded.receive_dir(), receive);

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&receive).ok();
    }

    #[test]
    fn setting_the_receive_dir_creates_it() {
        let dir = temp_dir("create");
        let target = dir.join("nested").join("inbox");

        SettingsStore::load(&dir, "Laptop".into())
            .unwrap()
            .set_receive_dir(target.clone())
            .unwrap();

        assert!(target.is_dir());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_blank_device_name_is_ignored() {
        let dir = temp_dir("blank-name");
        let settings = SettingsStore::load(&dir, "Laptop".into()).unwrap();

        settings.set_device_name("   ".into()).unwrap();
        assert_eq!(settings.device_name(), "Laptop");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn device_names_are_bounded() {
        let dir = temp_dir("long-name");
        let settings = SettingsStore::load(&dir, "Laptop".into()).unwrap();

        settings.set_device_name("a".repeat(500)).unwrap();
        assert!(settings.device_name().chars().count() <= 64);

        std::fs::remove_dir_all(&dir).ok();
    }
}
