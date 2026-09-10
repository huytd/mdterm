use wasm_bindgen::prelude::*;
use crate::state::FileEntry;
use serde_json::json;

#[wasm_bindgen(inline_js = r#"
export async function tauriInvoke(cmd, args) {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        return await window.__TAURI__.core.invoke(cmd, args);
    }
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
        return await window.__TAURI_INTERNALS__.invoke(cmd, args);
    }
    throw new Error("Tauri IPC is not available in browser mode");
}

export function isTauriEnvironment() {
    return !!((window.__TAURI__ && window.__TAURI__.core) || window.__TAURI_INTERNALS__);
}

export function triggerDownload(filename, text, mimeType) {
    const blob = new Blob([text], { type: mimeType || 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

export function openPrintDialog() {
    window.print();
}

export function execEditorCommand(command, value) {
    document.execCommand(command, false, value || null);
}

export function windowFind(query, caseSensitive, backward) {
    if (window.find) {
        return window.find(query, caseSensitive, backward, true);
    }
    return false;
}

export function initTerminalSession(containerId) {
    const container = document.getElementById(containerId);
    if (!container) return;

    if (window._mdtermTerminal) {
        try {
            window._mdtermTerminal.dispose();
        } catch (e) {}
        window._mdtermTerminal = null;
    }
    container.innerHTML = '';

    if (typeof Terminal === 'undefined') {
        const errDiv = document.createElement('div');
        errDiv.style.color = '#ef4444';
        errDiv.style.padding = '20px';
        errDiv.innerText = 'Terminal library (xterm.js) is not loaded.';
        container.appendChild(errDiv);
        return;
    }

    const term = new Terminal({
        cursorBlink: true,
        cursorStyle: 'bar',
        fontSize: 13,
        lineHeight: 1.25,
        fontFamily: 'JetBrains Mono, Menlo, Monaco, Consolas, "Courier New", monospace',
        theme: {
            background: '#0f141c',
            foreground: '#e2e8f0',
            cursor: '#38bdf8',
            cursorAccent: '#0f141c',
            selectionBackground: 'rgba(56, 189, 248, 0.35)',
            black: '#1e293b',
            red: '#f87171',
            green: '#4ade80',
            yellow: '#fbbf24',
            blue: '#60a5fa',
            magenta: '#c084fc',
            cyan: '#38bdf8',
            white: '#f1f5f9',
            brightBlack: '#64748b',
            brightRed: '#ef4444',
            brightGreen: '#22c55e',
            brightYellow: '#eab308',
            brightBlue: '#3b82f6',
            brightMagenta: '#a855f7',
            brightCyan: '#06b6d4',
            brightWhite: '#ffffff'
        },
        convertEol: true,
        allowTransparency: false
    });

    let fitAddon = null;
    if (typeof FitAddon !== 'undefined' && FitAddon.FitAddon) {
        fitAddon = new FitAddon.FitAddon();
        term.loadAddon(fitAddon);
    }

    term.open(container);
    setTimeout(() => {
        if (fitAddon) {
            try { fitAddon.fit(); } catch (e) {}
        }
    }, 60);

    const hasTauri = !!(window.__TAURI__ && window.__TAURI__.core);

    if (hasTauri) {
        const invoke = window.__TAURI__.core.invoke;
        const event = window.__TAURI__.event;

        const cols = term.cols || 80;
        const rows = term.rows || 24;
        invoke('pty_spawn', { cols, rows }).catch(err => {
            term.write('\r\n\x1b[31mFailed to spawn shell: ' + err + '\x1b[0m\r\n');
        });

        if (event && typeof event.listen === 'function') {
            event.listen('pty-output', (e) => {
                term.write(e.payload);
            });
        }

        term.onData(data => {
            invoke('pty_write', { data }).catch(() => {});
        });

        const handleResize = () => {
            if (fitAddon) {
                try {
                    fitAddon.fit();
                    invoke('pty_resize', { cols: term.cols, rows: term.rows }).catch(() => {});
                } catch (e) {}
            }
        };

        const ro = new ResizeObserver(() => {
            handleResize();
        });
        ro.observe(container);
        window.addEventListener('resize', handleResize);
    } else {
        term.write('\x1b[1;36m=== mdterm Terminal Emulator ===\x1b[0m\r\n');
        term.write('\x1b[90mRunning in browser preview. In Tauri desktop app, a native shell runs here.\x1b[0m\r\n\r\n$ ');
        let buf = '';
        term.onData(data => {
            if (data === '\r') {
                term.write('\r\n');
                if (buf.trim() === 'clear') {
                    term.clear();
                } else if (buf.trim().length > 0) {
                    term.write('Command in demo mode: ' + buf + '\r\n');
                }
                buf = '';
                term.write('$ ');
            } else if (data === '\u007F') {
                if (buf.length > 0) {
                    buf = buf.slice(0, -1);
                    term.write('\b \b');
                }
            } else {
                buf += data;
                term.write(data);
            }
        });

        const ro = new ResizeObserver(() => {
            if (fitAddon) {
                try { fitAddon.fit(); } catch (e) {}
            }
        });
        ro.observe(container);
    }

    window._mdtermTerminal = term;
    window._mdtermFitAddon = fitAddon;
}

export function clearTerminalSession() {
    if (window._mdtermTerminal) {
        window._mdtermTerminal.clear();
    }
}

export function fitTerminalSession() {
    if (window._mdtermFitAddon) {
        try {
            window._mdtermFitAddon.fit();
        } catch (e) {}
    }
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn tauriInvoke(
        cmd: &str,
        args: wasm_bindgen::JsValue,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_name = isTauriEnvironment)]
    pub fn is_tauri_env() -> bool;

    pub fn triggerDownload(filename: &str, text: &str, mime_type: &str);

    pub fn openPrintDialog();

    #[wasm_bindgen(js_name = execEditorCommand)]
    pub fn exec_editor_cmd(command: &str, value: Option<&str>);

    #[wasm_bindgen(js_name = windowFind)]
    pub fn window_find(query: &str, case_sensitive: bool, backward: bool) -> bool;

    #[wasm_bindgen(js_name = initTerminalSession)]
    pub fn init_terminal_session(container_id: &str);

    #[wasm_bindgen(js_name = clearTerminalSession)]
    pub fn clear_terminal_session();

    #[wasm_bindgen(js_name = fitTerminalSession)]
    pub fn fit_terminal_session();
}

pub async fn read_file(path: &str) -> Result<String, String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({ "path": path }))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        let res = tauriInvoke("read_file", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        serde_wasm_bindgen::from_value(res)
            .map_err(|e| format!("Failed to parse response: {:?}", e))
    } else {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(saved)) = storage.get_item(&format!("mdterm_file_{}", path)) {
                    return Ok(saved);
                }
            }
        }
        Err(format!("File '{}' not found in storage", path))
    }
}

pub async fn write_file(path: &str, contents: &str) -> Result<(), String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({ "path": path, "contents": contents }))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        tauriInvoke("write_file", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        Ok(())
    } else {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(&format!("mdterm_file_{}", path), contents);
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn read_dir(path: &str) -> Result<Vec<FileEntry>, String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({ "path": path }))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        let res = tauriInvoke("read_dir", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        serde_wasm_bindgen::from_value(res)
            .map_err(|e| format!("Failed to parse response: {:?}", e))
    } else {
        Ok(vec![
            FileEntry {
                name: "Welcome.md".to_string(),
                path: "Welcome.md".to_string(),
                is_dir: false,
                size: 2400,
            },
            FileEntry {
                name: "Syntax-Guide.md".to_string(),
                path: "Syntax-Guide.md".to_string(),
                is_dir: false,
                size: 3100,
            },
        ])
    }
}

#[allow(dead_code)]
pub async fn get_current_dir() -> Result<String, String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({}))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        let res = tauriInvoke("get_current_dir", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        serde_wasm_bindgen::from_value(res)
            .map_err(|e| format!("Failed to parse response: {:?}", e))
    } else {
        Ok("Documents".to_string())
    }
}

#[allow(dead_code)]
pub async fn create_file(path: &str) -> Result<(), String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({ "path": path }))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        tauriInvoke("create_file", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        Ok(())
    } else {
        write_file(path, "").await
    }
}

#[allow(dead_code)]
pub async fn delete_file(path: &str) -> Result<(), String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({ "path": path }))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        tauriInvoke("delete_file", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        Ok(())
    } else {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.remove_item(&format!("mdterm_file_{}", path));
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn rename_file(old_path: &str, new_path: &str) -> Result<(), String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({ "old_path": old_path, "new_path": new_path }))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        tauriInvoke("rename_file", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        Ok(())
    } else {
        if let Ok(content) = read_file(old_path).await {
            let _ = write_file(new_path, &content).await;
            let _ = delete_file(old_path).await;
        }
        Ok(())
    }
}

#[wasm_bindgen(inline_js = r#"
export function getSelectionCoordinates() {
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
        const rect = sel.getRangeAt(0).getBoundingClientRect();
        return [rect.left, rect.bottom];
    }
    return [150, 150];
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = getSelectionCoordinates)]
    pub fn get_selection_coordinates() -> js_sys::Array;
}

pub fn get_cursor_pos() -> (f64, f64) {
    let arr = get_selection_coordinates();
    let x = arr.get(0).as_f64().unwrap_or(150.0);
    let y = arr.get(1).as_f64().unwrap_or(150.0);
    (x, y)
}
