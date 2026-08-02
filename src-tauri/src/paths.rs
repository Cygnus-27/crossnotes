//! Turning attacker-controlled strings into safe filesystem paths.
//!
//! Every path in an incoming transfer is chosen by the sender, so this module
//! treats all of it as hostile. The rule enforced here is absolute: a transfer
//! may only ever create files *underneath* the user's chosen receive
//! directory, no matter what the sender puts on the wire.
//!
//! Three separate defences, because any one of them can have a gap:
//!   1. Reject structurally dangerous paths outright (`safe_relative_path`).
//!   2. Sanitize each surviving component (reserved names, control bytes, ...).
//!   3. Re-check the joined result is still inside the root (`resolve_within`).

use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Names Windows refuses to treat as ordinary files, in any directory and with
/// any extension (`CON.txt` is still `CON`). Writing one can hang on a device
/// handle rather than creating a file.
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Cap on a single path component, comfortably under the 255-byte limit that
/// most filesystems enforce.
const MAX_COMPONENT_LEN: usize = 200;

/// Cap on the whole relative path, leaving room for the receive directory
/// prefix before anything approaches Windows' MAX_PATH.
const MAX_RELATIVE_LEN: usize = 1024;

/// Validate and normalize a sender-supplied relative path.
///
/// Returns a path guaranteed to be relative, traversal-free, and composed only
/// of sanitized components. Rejects rather than repairs anything structurally
/// dangerous — silently "fixing" `../../etc/passwd` into `etc/passwd` would
/// write a file the user never agreed to receive.
pub fn safe_relative_path(raw: &str) -> Result<PathBuf> {
    let reject = |reason: &str| Err(Error::UnsafePath(format!("{raw:?}: {reason}")));

    if raw.trim().is_empty() {
        return reject("empty path");
    }
    if raw.len() > MAX_RELATIVE_LEN {
        return reject("path is too long");
    }
    if raw.contains('\0') {
        return reject("contains a null byte");
    }

    // Normalize separators first so a Windows-style path from a Windows sender
    // is understood as a path on Unix too, rather than becoming one giant
    // filename containing backslashes.
    let normalized = raw.replace('\\', "/");

    // Absolute in the POSIX sense, or a UNC path like //server/share.
    if normalized.starts_with('/') {
        return reject("absolute path");
    }

    // Windows drive letters, both `C:/x` and the drive-relative `C:x`.
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return reject("contains a drive letter");
    }

    let mut safe = PathBuf::new();
    for segment in normalized.split('/') {
        // Collapse the empty segments that doubled slashes and a trailing
        // slash produce.
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return reject("contains a parent-directory traversal");
        }
        safe.push(sanitize_component(segment)?);
    }

    if safe.as_os_str().is_empty() {
        return reject("resolves to an empty path");
    }

    // Belt and braces: whatever the string analysis concluded, the OS's own
    // parser must also agree there is nothing exotic in here.
    for component in safe.components() {
        match component {
            Component::Normal(_) => {}
            _ => return reject("contains a non-ordinary path component"),
        }
    }

    Ok(safe)
}

/// Make one path segment safe to write to disk on any supported platform.
pub fn sanitize_component(segment: &str) -> Result<String> {
    if segment.is_empty() {
        return Err(Error::UnsafePath("empty path component".into()));
    }

    let mut cleaned: String = segment
        .chars()
        .map(|ch| match ch {
            // Separators can't survive inside a component, and `:` would open
            // an NTFS alternate data stream (`report.txt:hidden.exe`) —
            // a genuine way to smuggle a payload past a filename check.
            '/' | '\\' | ':' => '_',
            // Illegal on Windows, and awkward-to-hostile everywhere else.
            '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();

    // Windows silently strips trailing dots and spaces, so `evil.txt.` becomes
    // `evil.txt` *after* any checks we do. Strip them ourselves so what we
    // validate is what actually lands. Leading dots are fine and meaningful
    // (dotfiles), so only the trailing end is trimmed.
    cleaned = cleaned.trim_end_matches([' ', '.']).to_string();

    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return Err(Error::UnsafePath(format!(
            "component {segment:?} sanitizes to nothing usable"
        )));
    }

    // Reserved device names apply to the stem, ignoring any extension.
    let stem = cleaned.split('.').next().unwrap_or(&cleaned);
    if WINDOWS_RESERVED
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
    {
        cleaned = format!("_{cleaned}");
    }

    if cleaned.len() > MAX_COMPONENT_LEN {
        cleaned = truncate_preserving_extension(&cleaned, MAX_COMPONENT_LEN);
    }

    Ok(cleaned)
}

/// Shorten an over-long name without losing the extension, which is what the
/// user actually needs to open the file afterwards.
fn truncate_preserving_extension(name: &str, max_len: usize) -> String {
    let path = Path::new(name);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| ext.len() <= 16);

    match extension {
        Some(ext) => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
            let room = max_len.saturating_sub(ext.len() + 1);
            format!("{}.{}", floor_char_boundary(stem, room), ext)
        }
        None => floor_char_boundary(name, max_len).to_string(),
    }
}

/// Truncate to at most `max_len` bytes without splitting a UTF-8 character.
fn floor_char_boundary(value: &str, max_len: usize) -> &str {
    if value.len() <= max_len {
        return value;
    }
    let mut end = max_len;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

/// Join a validated relative path onto the receive root and confirm the result
/// really is inside it.
///
/// The final containment check is not redundant. `safe_relative_path` reasons
/// about the string; this reasons about the filesystem, and catches the case
/// where a path component is a symlink pointing out of the root.
pub fn resolve_within(root: &Path, relative: &Path) -> Result<PathBuf> {
    let candidate = root.join(relative);

    // Compare against the canonical root so that a symlinked or 8.3-shortened
    // receive directory doesn't produce a spurious mismatch.
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    // The file itself does not exist yet, so canonicalize the deepest parent
    // that does and verify that lands inside the root.
    let mut existing = candidate.as_path();
    let resolved_parent = loop {
        match existing.parent() {
            Some(parent) => {
                if let Ok(canonical) = parent.canonicalize() {
                    break canonical;
                }
                existing = parent;
            }
            None => return Err(Error::UnsafePath(format!("{candidate:?} has no parent"))),
        }
    };

    if !resolved_parent.starts_with(&canonical_root) {
        return Err(Error::UnsafePath(format!(
            "{relative:?} would write outside the receive folder"
        )));
    }

    Ok(candidate)
}

/// Find a free name so an incoming file never silently destroys an existing
/// one. `report.pdf` becomes `report (1).pdf`, then `report (2).pdf`.
///
/// Note the deliberate absence of an overwrite path: a sender should not be
/// able to replace a file on the receiver's disk just by naming it.
pub fn unique_path(desired: &Path) -> PathBuf {
    if !desired.exists() {
        return desired.to_path_buf();
    }

    let parent = desired.parent().unwrap_or(Path::new(""));
    let stem = desired
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = desired.extension().and_then(|e| e.to_str());

    for counter in 1..10_000 {
        let candidate = match extension {
            Some(ext) => parent.join(format!("{stem} ({counter}).{ext}")),
            None => parent.join(format!("{stem} ({counter})")),
        };
        if !candidate.exists() {
            return candidate;
        }
    }

    // Pathological directory; fall back to something collision-proof.
    parent.join(format!("{stem}-{}", uuid::Uuid::new_v4()))
}

/// The in-progress name for a file being received. Kept distinct so a partial
/// transfer is never mistaken for a complete file, and so resume can find it.
pub fn partial_path(final_path: &Path) -> PathBuf {
    let mut name = final_path.as_os_str().to_os_string();
    name.push(".fluqsr-part");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_paths() {
        assert_eq!(safe_relative_path("notes.txt").unwrap(), Path::new("notes.txt"));
        assert_eq!(
            safe_relative_path("photos/2026/trip.jpg").unwrap(),
            Path::new("photos").join("2026").join("trip.jpg")
        );
    }

    #[test]
    fn treats_windows_separators_as_separators() {
        assert_eq!(
            safe_relative_path("photos\\trip.jpg").unwrap(),
            Path::new("photos").join("trip.jpg"),
            "a backslash must split components, not become part of a filename"
        );
    }

    #[test]
    fn rejects_traversal() {
        for evil in [
            "../secrets",
            "a/../../b",
            "..\\..\\Windows\\System32\\drivers\\etc\\hosts",
            "photos/../../../.ssh/authorized_keys",
            "..",
        ] {
            assert!(
                safe_relative_path(evil).is_err(),
                "should have rejected traversal: {evil}"
            );
        }
    }

    #[test]
    fn rejects_absolute_and_drive_paths() {
        for evil in [
            "/etc/passwd",
            "/",
            "C:/Windows/System32/calc.exe",
            "c:relative.txt",
            "\\\\server\\share\\file.txt",
            "//server/share/file.txt",
        ] {
            assert!(
                safe_relative_path(evil).is_err(),
                "should have rejected absolute path: {evil}"
            );
        }
    }

    #[test]
    fn rejects_null_bytes() {
        assert!(safe_relative_path("good.txt\0.exe").is_err());
    }

    #[test]
    fn rejects_empty_and_dot_only_paths() {
        for evil in ["", "   ", ".", "./", "./././"] {
            assert!(
                safe_relative_path(evil).is_err(),
                "should have rejected: {evil:?}"
            );
        }
    }

    #[test]
    fn collapses_redundant_separators() {
        assert_eq!(
            safe_relative_path("a//b/./c.txt").unwrap(),
            Path::new("a").join("b").join("c.txt")
        );
    }

    #[test]
    fn neutralizes_ntfs_alternate_data_streams() {
        // `report.txt:payload.exe` writes a hidden stream on NTFS that does not
        // show up in a directory listing but is still executable.
        let safe = safe_relative_path("report.txt:payload.exe").unwrap();
        assert_eq!(safe, Path::new("report.txt_payload.exe"));
        assert!(!safe.to_string_lossy().contains(':'));
    }

    #[test]
    fn escapes_windows_reserved_device_names() {
        for reserved in ["CON", "con.txt", "NUL", "aux.tar.gz", "COM1", "lpt9.dat"] {
            let safe = sanitize_component(reserved).unwrap();
            assert!(
                safe.starts_with('_'),
                "{reserved} should have been escaped, got {safe}"
            );
        }
    }

    #[test]
    fn leaves_names_that_merely_resemble_reserved_ones() {
        for ordinary in ["console.log", "contract.pdf", "nullable.rs", "communicate"] {
            assert_eq!(sanitize_component(ordinary).unwrap(), ordinary);
        }
    }

    #[test]
    fn strips_trailing_dots_and_spaces() {
        // Windows would strip these itself, after our checks had run.
        assert_eq!(sanitize_component("evil.txt. . ").unwrap(), "evil.txt");
        assert_eq!(sanitize_component("trailing ").unwrap(), "trailing");
    }

    #[test]
    fn keeps_leading_dots_for_dotfiles() {
        assert_eq!(sanitize_component(".gitignore").unwrap(), ".gitignore");
        assert_eq!(sanitize_component(".env.local").unwrap(), ".env.local");
    }

    #[test]
    fn strips_control_characters() {
        let safe = sanitize_component("na\u{7}me\u{1b}[0m.txt").unwrap();
        assert!(!safe.chars().any(|c| c.is_control()));
    }

    #[test]
    fn truncates_long_names_but_keeps_the_extension() {
        let long = format!("{}.pdf", "a".repeat(500));
        let safe = sanitize_component(&long).unwrap();
        assert!(safe.len() <= MAX_COMPONENT_LEN);
        assert!(safe.ends_with(".pdf"), "extension must survive: {safe}");
    }

    #[test]
    fn truncation_does_not_split_multibyte_characters() {
        let long = "\u{1f600}".repeat(200);
        let safe = sanitize_component(&long).unwrap();
        assert!(safe.len() <= MAX_COMPONENT_LEN);
        // Would have panicked during truncation if a boundary were split.
        assert!(!safe.is_empty());
    }

    #[test]
    fn rejects_absurdly_long_paths() {
        let long = format!("{}/x.txt", "a/".repeat(2000));
        assert!(safe_relative_path(&long).is_err());
    }

    #[test]
    fn unicode_filenames_pass_through_intact() {
        assert_eq!(
            safe_relative_path("документы/отчёт.pdf").unwrap(),
            Path::new("документы").join("отчёт.pdf")
        );
        assert_eq!(sanitize_component("日本語.txt").unwrap(), "日本語.txt");
    }

    #[test]
    fn resolve_within_accepts_paths_inside_the_root() {
        let root = std::env::temp_dir().join(format!("fluqsr-within-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        let relative = safe_relative_path("sub/file.txt").unwrap();
        let resolved = resolve_within(&root, &relative).unwrap();
        assert!(resolved.ends_with(Path::new("sub").join("file.txt")));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unique_path_never_overwrites() {
        let root = std::env::temp_dir().join(format!("fluqsr-unique-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        let target = root.join("report.pdf");
        assert_eq!(unique_path(&target), target, "free name should be used as-is");

        std::fs::write(&target, b"original").unwrap();
        let second = unique_path(&target);
        assert_eq!(second, root.join("report (1).pdf"));

        std::fs::write(&second, b"second").unwrap();
        assert_eq!(unique_path(&target), root.join("report (2).pdf"));

        // The original must be untouched.
        assert_eq!(std::fs::read(&target).unwrap(), b"original");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn partial_files_are_distinguishable_from_finished_ones() {
        let partial = partial_path(Path::new("/tmp/movie.mkv"));
        assert!(partial.to_string_lossy().ends_with(".fluqsr-part"));
        assert_ne!(partial, PathBuf::from("/tmp/movie.mkv"));
    }
}
