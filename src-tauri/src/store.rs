//! Small JSON persistence helpers shared by identity, trust, and settings.

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Result;

/// Read a JSON file, returning `None` when it does not exist yet or cannot be
/// parsed. Callers treat unreadable state as absent and rebuild it — every
/// user of this module can regenerate its file, and refusing to start because
/// of one corrupt byte would be worse than starting fresh.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    Ok(serde_json::from_slice(&bytes).ok())
}

/// Write JSON atomically: serialize to a sibling temp file, then rename over
/// the target. A crash mid-write leaves the previous version intact rather
/// than a truncated file — which for the trust store would mean silently
/// losing pinned peer keys.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let bytes = serde_json::to_vec_pretty(value)?;
    let temp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));

    fs::write(&temp, &bytes)?;
    if let Err(err) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(err.into());
    }
    Ok(())
}

/// Tighten permissions on a file holding secret material. Best-effort: on
/// platforms without Unix modes this is a no-op, and a failure here should not
/// stop the app from working.
pub fn restrict_to_owner(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Sample {
        value: u32,
    }

    fn temp_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fluqsr-store-{tag}-{}.json", uuid::Uuid::new_v4()))
    }

    #[test]
    fn missing_file_reads_as_none() {
        let path = temp_path("missing");
        assert!(read_json::<Sample>(&path).unwrap().is_none());
    }

    #[test]
    fn round_trips_a_value() {
        let path = temp_path("round-trip");
        write_json(&path, &Sample { value: 42 }).unwrap();
        assert_eq!(read_json::<Sample>(&path).unwrap(), Some(Sample { value: 42 }));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn corrupt_file_reads_as_none_instead_of_erroring() {
        let path = temp_path("corrupt");
        fs::write(&path, b"{ not json").unwrap();
        assert!(read_json::<Sample>(&path).unwrap().is_none());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn writing_leaves_no_temp_files_behind() {
        let path = temp_path("no-temps");
        write_json(&path, &Sample { value: 1 }).unwrap();
        write_json(&path, &Sample { value: 2 }).unwrap();

        let dir = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let leftovers = fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                name.starts_with(&stem) && name.contains("tmp-")
            })
            .count();

        assert_eq!(leftovers, 0);
        fs::remove_file(&path).ok();
    }
}
