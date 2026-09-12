use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, Manager, State};

pub mod config;

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
fn set_window_theme(window: tauri::WebviewWindow, theme: String) -> Result<(), String> {
    let native_theme = match theme.as_str() {
        "theme-light" => tauri::Theme::Light,
        _ => tauri::Theme::Dark,
    };

    window
        .set_theme(Some(native_theme))
        .map_err(|e| format!("Failed to set window theme: {}", e))
}

#[tauri::command]
fn window_show(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::WebviewWindow) -> Result<bool, String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
fn window_is_maximized(window: tauri::WebviewWindow) -> Result<bool, String> {
    window.is_maximized().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_close(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_start_dragging(window: tauri::WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_start_resize(window: tauri::Window, direction: String) -> Result<(), String> {
    let dir = match direction.as_str() {
        "East" => tauri_runtime::ResizeDirection::East,
        "North" => tauri_runtime::ResizeDirection::North,
        "NorthEast" => tauri_runtime::ResizeDirection::NorthEast,
        "NorthWest" => tauri_runtime::ResizeDirection::NorthWest,
        "South" => tauri_runtime::ResizeDirection::South,
        "SouthEast" => tauri_runtime::ResizeDirection::SouthEast,
        "SouthWest" => tauri_runtime::ResizeDirection::SouthWest,
        "West" => tauri_runtime::ResizeDirection::West,
        _ => return Err(format!("Invalid direction: {}", direction)),
    };
    window.start_resize_dragging(dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn window_set_size(window: tauri::WebviewWindow, width: u32, height: u32) -> Result<(), String> {
    window.set_size(tauri::PhysicalSize::new(width, height)).map_err(|e| e.to_string())
}

#[tauri::command]
fn window_get_size(window: tauri::WebviewWindow) -> Result<(u32, u32), String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    Ok((size.width, size.height))
}




#[tauri::command]
fn get_cli_file() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    for arg in args.into_iter().skip(1) {
        if !arg.starts_with('-') {
            let path = PathBuf::from(&arg);
            if path.exists() {
                if let Ok(abs) = fs::canonicalize(&path) {
                    return Some(abs.to_string_lossy().to_string());
                }
                return Some(arg);
            }
        }
    }
    None
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
    cmd.env("MDTERM", "1");
    cmd.env("TERM_PROGRAM", "mdterm");
    if std::env::var("LANG").is_err() {
        cmd.env("LANG", "en_US.UTF-8");
    }

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
                    break;
                }
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = app.emit("pty-output", text);
                }
                Err(_) => {
                    break;
                }
            }
        }
        app.exit(0);
    });

    Ok(())
}

#[tauri::command]
fn pty_get_cwd(state: State<PtyState>) -> Result<String, String> {
    let sess = state.session.lock().map_err(|_| "Lock error".to_string())?;
    if let Some(session) = sess.as_ref() {
        if let Some(_pid) = session.child_pid {
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
        let bytes = data.as_bytes();
        for chunk in bytes.chunks(4096) {
            session.writer.write_all(chunk)
                .map_err(|e| format!("Write error: {}", e))?;
        }
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

#[tauri::command]
fn get_terminal_config() -> Result<config::TerminalConfig, String> {
    Ok(config::load_terminal_config())
}

#[tauri::command]
fn get_config_path() -> Result<String, String> {
    config::get_config_path()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine config path".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                // Defensive fallback: ensure window is shown even if frontend init encounters an unexpected issue
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    let _ = w.show();
                    let _ = w.set_focus();
                });
            }

            // Ensure config exists and watch for modifications
            let _ = config::ensure_default_config_exists();
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let config_path = match config::get_config_path() {
                    Some(p) => p,
                    None => return,
                };
                let mut last_modified = fs::metadata(&config_path).and_then(|m| m.modified()).ok();

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    let current_modified = fs::metadata(&config_path).and_then(|m| m.modified()).ok();
                    if current_modified != last_modified && current_modified.is_some() {
                        last_modified = current_modified;
                        let cfg = config::load_terminal_config();
                        let _ = app_handle.emit("terminal-config-changed", &cfg);
                    }
                }
            });

            Ok(())
        })
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
            set_window_theme,
            pty_spawn,
            pty_get_cwd,
            pty_write,
            pty_resize,
            get_cli_file,
            get_terminal_config,
            get_config_path,
            window_show,
            window_minimize,
            window_toggle_maximize,
            window_is_maximized,
            window_close,
            window_start_dragging,
            window_start_resize,
            window_set_size,
            window_get_size
        ])

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
