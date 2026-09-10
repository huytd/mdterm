use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file '{}': {}", path, e))
}

#[tauri::command]
fn write_file(path: String, contents: String) -> Result<(), String> {
    if let Some(parent) = PathBuf::from(&path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directories: {}", e))?;
        }
    }
    fs::write(&path, contents).map_err(|e| format!("Failed to write file '{}': {}", path, e))
}

#[tauri::command]
fn read_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = PathBuf::from(&path);
    if !dir.exists() {
        return Err(format!("Path '{}' does not exist", path));
    }
    let mut entries = Vec::new();
    let read_entries = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", path, e))?;

    for entry in read_entries {
        if let Ok(entry) = entry {
            let p = entry.path();
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') && name != ".md" {
                continue;
            }
            if name == "target" || name == "node_modules" || name == "dist" {
                continue;
            }

            entries.push(FileEntry {
                name,
                path: p.to_string_lossy().to_string(),
                is_dir,
                size,
            });
        }
    }

    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Ok(entries)
}

#[tauri::command]
fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())
}

#[tauri::command]
fn get_current_dir() -> Result<String, String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Could not get current directory: {}", e))
}

#[tauri::command]
fn create_file(path: String) -> Result<(), String> {
    if let Some(parent) = PathBuf::from(&path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if !PathBuf::from(&path).exists() {
        fs::write(&path, "").map_err(|e| format!("Failed to create file: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn delete_file(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.is_dir() {
        fs::remove_dir_all(&p).map_err(|e| format!("Failed to remove directory: {}", e))
    } else {
        fs::remove_file(&p).map_err(|e| format!("Failed to remove file: {}", e))
    }
}

#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename(&old_path, &new_path).map_err(|e| format!("Failed to rename: {}", e))
}

#[tauri::command]
fn export_document(path: String, content: String) -> Result<(), String> {
    write_file(path, content)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            read_dir,
            get_home_dir,
            get_current_dir,
            create_file,
            delete_file,
            rename_file,
            export_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

