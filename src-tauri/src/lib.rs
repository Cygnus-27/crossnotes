// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::Serialize;
use std::{fs, path::Path, process::Command};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttachedFile {
    file_name: String,
    relative_path: String,
    is_image: bool,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn open_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let status = Command::new("explorer")
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch explorer: {err}"))?;

    #[cfg(target_os = "macos")]
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch Finder: {err}"))?;

    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open")
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch file manager: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("file manager exited with status: {status}"))
    }
}

#[tauri::command]
fn attach_file_to_vault(source_path: String, vault_path: String) -> Result<AttachedFile, String> {
    let source = Path::new(&source_path);
    if !source.is_file() {
        return Err(format!("source is not a file: {source_path}"));
    }

    let vault = Path::new(&vault_path);
    if !vault.is_dir() {
        return Err(format!("vault is not a directory: {vault_path}"));
    }

    let attachments_dir = vault.join("Attachments");
    fs::create_dir_all(&attachments_dir)
        .map_err(|err| format!("failed to create Attachments folder: {err}"))?;

    let original_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "source file has no valid UTF-8 filename".to_string())?;
    let safe_name = sanitize_file_name(original_name);
    let destination_name = unique_file_name(&attachments_dir, &safe_name);
    let destination = attachments_dir.join(&destination_name);

    fs::copy(source, &destination).map_err(|err| format!("failed to copy attachment: {err}"))?;

    Ok(AttachedFile {
        file_name: destination_name.clone(),
        relative_path: format!("Attachments/{destination_name}"),
        is_image: is_image_file(&destination_name),
    })
}

fn sanitize_file_name(file_name: &str) -> String {
    let sanitized: String = file_name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            ch if ch.is_control() => '-',
            ch => ch,
        })
        .collect();

    let trimmed = sanitized.trim_matches([' ', '.']);
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_file_name(directory: &Path, file_name: &str) -> String {
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("attachment");
    let extension = path.extension().and_then(|extension| extension.to_str());

    let mut candidate = file_name.to_string();
    let mut counter = 1;

    while directory.join(&candidate).exists() {
        candidate = match extension {
            Some(extension) => format!("{stem}-{counter}.{extension}"),
            None => format!("{stem}-{counter}"),
        };
        counter += 1;
    }

    candidate
}

fn is_image_file(file_name: &str) -> bool {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());

    matches!(
        extension.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "avif")
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            open_in_file_manager,
            attach_file_to_vault
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
