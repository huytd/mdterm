use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, State};

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child_pid: Option<u32>,
}

#[derive(Default)]
pub struct PtyState {
    session: Mutex<Option<PtySession>>,
}

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

#[tauri::command]
fn pty_spawn(app: AppHandle, state: State<PtyState>, cols: u16, rows: u16) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let existing_path = std::env::var("PATH").unwrap_or_default();
    let home_bin = dirs::home_dir()
        .map(|h| h.join(".local/bin").to_string_lossy().to_string())
        .unwrap_or_default();
    let repo_bin = std::env::current_dir()
        .map(|d| d.join("bin").to_string_lossy().to_string())
        .unwrap_or_default();
    cmd.env("PATH", format!("{}:{}:{}", repo_bin, home_bin, existing_path));

    if let Ok(current_dir) = std::env::current_dir() {
        cmd.cwd(current_dir);
    }

    let child = pair.slave.spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell '{}': {}", shell, e))?;
    let child_pid = child.process_id();

    // Drop slave in parent so EOF is triggered when shell exits
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()
        .map_err(|e| format!("Failed to clone reader: {}", e))?;
    let writer = pair.master.take_writer()
        .map_err(|e| format!("Failed to take writer: {}", e))?;

    // Store in state
    {
        let mut sess = state.session.lock().map_err(|_| "Failed to lock PTY state".to_string())?;
        *sess = Some(PtySession {
            master: pair.master,
            writer,
            child_pid,
        });
    }

    // Spawn reader thread
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = app.emit("pty-output", "\r\n[Process completed]\r\n");
                    break;
                }
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = app.emit("pty-output", text);
                }
                Err(e) => {
                    log::warn!("PTY read error: {}", e);
                    break;
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn pty_get_cwd(state: State<PtyState>) -> Result<String, String> {
    let sess = state.session.lock().map_err(|_| "Lock error".to_string())?;
    if let Some(session) = sess.as_ref() {
        if let Some(pid) = session.child_pid {
            // Check tmux pane_current_path if tmux is running
            if let Ok(out) = std::process::Command::new("tmux")
                .args(["display-message", "-p", "#{pane_current_path}"])
                .output()
            {
                if out.status.success() {
                    let tmux_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !tmux_path.is_empty() && std::path::Path::new(&tmux_path).is_dir() {
                        return Ok(tmux_path);
                    }
                }
            }

            // On Linux, read /proc/<pid>/cwd
            #[cfg(target_os = "linux")]
            {
                let proc_path = format!("/proc/{}/cwd", pid);
                if let Ok(link) = std::fs::read_link(&proc_path) {
                    return Ok(link.to_string_lossy().to_string());
                }
            }
        }
    }

    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Could not get cwd: {}", e))
}

#[tauri::command]
fn pty_write(state: State<PtyState>, data: String) -> Result<(), String> {
    let mut sess = state.session.lock().map_err(|_| "Lock error".to_string())?;
    if let Some(session) = sess.as_mut() {
        session.writer.write_all(data.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        session.writer.flush()
            .map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    } else {
        Err("No active PTY session".to_string())
    }
}

#[tauri::command]
fn pty_resize(state: State<PtyState>, cols: u16, rows: u16) -> Result<(), String> {
    let sess = state.session.lock().map_err(|_| "Lock error".to_string())?;
    if let Some(session) = sess.as_ref() {
        session.master.resize(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| format!("Resize error: {}", e))?;
        Ok(())
    } else {
        Err("No active PTY session".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(PtyState::default())
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            read_dir,
            get_home_dir,
            get_current_dir,
            create_file,
            delete_file,
            rename_file,
            export_document,
            pty_spawn,
            pty_get_cwd,
            pty_write,
            pty_resize
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

