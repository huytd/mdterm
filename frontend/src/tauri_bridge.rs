use wasm_bindgen::prelude::*;
use crate::state::FileEntry;
use serde::{Deserialize, Serialize};
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

export function showToast(message) {
    let toast = document.getElementById('mdterm-toast-container');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'mdterm-toast-container';
        toast.className = 'toast-container';
        document.body.appendChild(toast);
    }
    toast.innerHTML = '<span class="toast-icon">✓</span> <span class="toast-text">' + message + '</span>';
    toast.classList.add('toast-show');
    clearTimeout(window._mdtermToastTimer);
    window._mdtermToastTimer = setTimeout(() => {
        toast.classList.remove('toast-show');
    }, 3500);
}

export function encodeB64(str) {
    if (!str) return '';
    try {
        const bytes = new TextEncoder().encode(str);
        let binary = '';
        for (let i = 0; i < bytes.length; i++) {
            binary += String.fromCharCode(bytes[i]);
        }
        return btoa(binary);
    } catch (e) {
        return btoa(unescape(encodeURIComponent(str)));
    }
}

export async function sendRemoteSave(path, content) {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        const pathB64 = encodeB64(path);
        const contentB64 = encodeB64(content);
        const packet = '__MDTERM_SAVE__:' + pathB64 + ':' + contentB64 + '\n';
        const sId = window._mdtermActiveSessionId || '1';
        await window.__TAURI__.core.invoke('pty_write', { sessionId: sId, data: packet });
        return true;
    }
    return false;
}

export async function closeRemoteSession() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        const sId = window._mdtermActiveSessionId || '1';
        await window.__TAURI__.core.invoke('pty_write', { sessionId: sId, data: '__MDTERM_CLOSE__\n' }).catch(() => {});
        return true;
    }
    return false;
}

export async function windowShow() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_show');
        } catch (e) {
            return await window.__TAURI__.core.invoke('plugin:window|show').catch(() => {});
        }
    }
}

export async function windowMinimize() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_minimize');
        } catch (e) {
            return await window.__TAURI__.core.invoke('plugin:window|minimize').catch(() => {});
        }
    }
}

export async function windowToggleMaximize() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_toggle_maximize');
        } catch (e) {
            const isMax = await window.__TAURI__.core.invoke('plugin:window|is_maximized').catch(() => false);
            if (isMax) {
                await window.__TAURI__.core.invoke('plugin:window|unmaximize').catch(() => {});
                return false;
            } else {
                await window.__TAURI__.core.invoke('plugin:window|maximize').catch(() => {});
                return true;
            }
        }
    }
    return false;
}

export async function windowIsMaximized() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_is_maximized');
        } catch (e) {
            return await window.__TAURI__.core.invoke('plugin:window|is_maximized').catch(() => false);
        }
    }
    return false;
}

export async function windowClose() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_close');
        } catch (e) {
            return await window.__TAURI__.core.invoke('plugin:window|close').catch(() => {});
        }
    }
}

export async function windowStartDragging() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_start_dragging');
        } catch (e) {
            return await window.__TAURI__.core.invoke('plugin:window|start_dragging').catch(() => {});
        }
    }
}

export async function windowStartResize(direction) {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
            return await window.__TAURI__.core.invoke('window_start_resize', { direction });
        } catch (e) {
            return await window.__TAURI__.core.invoke('plugin:window|start_resize_dragging', { value: direction }).catch(() => {});
        }
    }
}

function loadMermaid(callback) {
    if (typeof mermaid !== 'undefined') {
        callback();
        return;
    }
    if (!window._mdtermMermaidCallbacks) {
        window._mdtermMermaidCallbacks = [];
        const script = document.createElement('script');
        script.src = 'mermaid.min.js';
        script.onload = () => {
            const cbs = window._mdtermMermaidCallbacks || [];
            window._mdtermMermaidCallbacks = null;
            cbs.forEach(cb => {
                try { cb(); } catch (e) { console.error(e); }
            });
        };
        script.onerror = (e) => {
            window._mdtermMermaidCallbacks = null;
            console.error('Failed to load mermaid.min.js', e);
        };
        document.head.appendChild(script);
    }
    window._mdtermMermaidCallbacks.push(callback);
}

export function renderMermaidDiagrams() {
    const targets = document.querySelectorAll('.mermaid-preview-target');
    if (!targets || targets.length === 0) return;

    if (typeof mermaid === 'undefined') {
        loadMermaid(() => renderMermaidDiagrams());
        return;
    }

    const themeName = (window._mdtermCurrentTheme || 'dark').toLowerCase().replace(/^theme-/, '');
    const mermaidTheme = themeName === 'light' ? 'default' : (themeName === 'nord' ? 'nord' : 'dark');

    try {
        mermaid.initialize({
            startOnLoad: false,
            theme: mermaidTheme,
            securityLevel: 'loose',
            fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif'
        });
    } catch (e) {}

    targets.forEach((target, index) => {
        const wrapper = target.closest('.mermaid-block-wrapper');
        const codeEl = wrapper ? wrapper.querySelector('code') : null;
        const codeText = codeEl ? codeEl.innerText.trim() : (target.getAttribute('data-raw-code') || '');

        if (!codeText) return;

        const id = 'mermaid-svg-' + Date.now() + '-' + index;
        try {
            mermaid.render(id, codeText)
                .then(result => {
                    target.innerHTML = result.svg;
                })
                .catch(err => {
                    const msg = (err && (err.message || err.str)) || String(err);
                    target.innerHTML = '<div class="mermaid-error"><div class="mermaid-error-title">⚠️ Mermaid Syntax Error</div><pre class="mermaid-error-msg">' + msg + '</pre></div>';
                });
        } catch (syncErr) {
            const msg = (syncErr && (syncErr.message || syncErr.str)) || String(syncErr);
            target.innerHTML = '<div class="mermaid-error"><div class="mermaid-error-title">⚠️ Mermaid Syntax Error</div><pre class="mermaid-error-msg">' + msg + '</pre></div>';
        }
    });
}

window._toggleMermaidView = function(btn, mode) {
    const wrapper = btn.closest('.mermaid-block-wrapper');
    if (!wrapper) return;

    const preview = wrapper.querySelector('.mermaid-preview-container');
    const codePre = wrapper.querySelector('.mermaid-code-pre');
    const tabs = wrapper.querySelectorAll('.mermaid-tab-btn');

    tabs.forEach(t => t.classList.toggle('active', t.getAttribute('data-tab') === mode));

    if (mode === 'code') {
        if (preview) preview.style.display = 'none';
        if (codePre) {
            codePre.style.display = 'block';
            const codeEl = codePre.querySelector('code');
            if (codeEl) codeEl.focus();
        }
    } else {
        if (codePre) codePre.style.display = 'none';
        if (preview) {
            preview.style.display = 'flex';
            const target = preview.querySelector('.mermaid-preview-target');
            const codeEl = wrapper.querySelector('code');
            if (target && codeEl) {
                loadMermaid(() => {
                    const codeText = codeEl.innerText.trim();
                    const id = 'mermaid-svg-' + Date.now() + '-' + Math.floor(Math.random() * 100000);
                    const themeName = (window._mdtermCurrentTheme || 'dark').toLowerCase().replace(/^theme-/, '');
                    const mermaidTheme = themeName === 'light' ? 'default' : (themeName === 'nord' ? 'nord' : 'dark');
                    try {
                        mermaid.initialize({ startOnLoad: false, theme: mermaidTheme, securityLevel: 'loose' });
                    } catch (e) {}

                    try {
                        mermaid.render(id, codeText)
                            .then(res => { target.innerHTML = res.svg; })
                            .catch(err => {
                                const msg = (err && (err.message || err.str)) || String(err);
                                target.innerHTML = '<div class="mermaid-error"><div class="mermaid-error-title">⚠️ Mermaid Syntax Error</div><pre class="mermaid-error-msg">' + msg + '</pre></div>';
                            });
                    } catch (syncErr) {
                        const msg = (syncErr && (syncErr.message || syncErr.str)) || String(syncErr);
                        target.innerHTML = '<div class="mermaid-error"><div class="mermaid-error-title">⚠️ Mermaid Syntax Error</div><pre class="mermaid-error-msg">' + msg + '</pre></div>';
                    }
                });
            }
        }
    }
};

window.renderMermaidDiagrams = renderMermaidDiagrams;
window.showToast = showToast;
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

    #[wasm_bindgen(js_name = renderMermaidDiagrams)]
    pub fn render_mermaid_diagrams();

    #[wasm_bindgen(js_name = showToast)]
    pub fn show_toast(message: &str);

    #[wasm_bindgen(js_name = sendRemoteSave, catch)]
    async fn sendRemoteSave(
        path: &str,
        content: &str,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_name = closeRemoteSession, catch)]
    async fn closeRemoteSession() -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_name = windowShow)]
    pub async fn window_show_js();

    #[wasm_bindgen(js_name = windowMinimize)]
    pub async fn window_minimize_js();

    #[wasm_bindgen(js_name = windowToggleMaximize)]
    pub async fn window_toggle_maximize_js() -> wasm_bindgen::JsValue;

    #[wasm_bindgen(js_name = windowIsMaximized)]
    pub async fn window_is_maximized_js() -> wasm_bindgen::JsValue;

    #[wasm_bindgen(js_name = windowClose)]
    pub async fn window_close_js();

    #[wasm_bindgen(js_name = windowStartDragging)]
    pub async fn window_start_dragging_js();

    #[wasm_bindgen(js_name = windowStartResize)]
    pub async fn window_start_resize_js(direction: &str);
}

#[wasm_bindgen(module = "/js/terminal-core.js")]
extern "C" {
    #[wasm_bindgen(js_name = initTerminalSession)]
    async fn init_terminal_session_js(container_id: &str, session_id: &str) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(js_name = setTerminalTheme)]
    pub fn set_terminal_theme(theme_name: &str);

    #[wasm_bindgen(js_name = applyTerminalConfig)]
    pub fn apply_terminal_config_js(config: wasm_bindgen::JsValue);

    #[wasm_bindgen(js_name = clearTerminalSession)]
    pub fn clear_terminal_session_js(session_id: Option<&str>);

    #[wasm_bindgen(js_name = fitTerminalSession)]
    pub fn fit_terminal_session_js(session_id: Option<&str>);

    #[wasm_bindgen(js_name = focusTerminalSession)]
    pub fn focus_terminal_session_js(session_id: Option<&str>);

    #[wasm_bindgen(js_name = closeTerminalSession)]
    pub fn close_terminal_session_js(session_id: &str);
}

pub fn window_show() {
    leptos::task::spawn_local(async {
        window_show_js().await;
    });
}

#[allow(dead_code)]
pub fn window_minimize() {
    leptos::task::spawn_local(async {
        window_minimize_js().await;
    });
}

pub async fn window_toggle_maximize() -> bool {
    let val = window_toggle_maximize_js().await;
    val.as_bool().unwrap_or(false)
}

pub async fn window_is_maximized() -> bool {
    let val = window_is_maximized_js().await;
    val.as_bool().unwrap_or(false)
}

#[allow(dead_code)]
pub fn window_close() {
    leptos::task::spawn_local(async {
        window_close_js().await;
    });
}

pub fn window_start_dragging() {
    leptos::task::spawn_local(async {
        window_start_dragging_js().await;
    });
}

pub fn window_start_resize(direction: &str) {
    let dir = direction.to_string();
    leptos::task::spawn_local(async move {
        window_start_resize_js(&dir).await;
    });
}

pub async fn send_remote_save(path: &str, content: &str) -> Result<(), String> {
    sendRemoteSave(path, content)
        .await
        .map(|_| ())
        .map_err(|e| format!("Remote save error: {:?}", e))
}

pub async fn close_remote_session() {
    let _ = closeRemoteSession().await;
}

pub async fn set_window_theme(theme: &str) -> Result<(), String> {
    if !is_tauri_env() {
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&json!({ "theme": theme }))
        .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
    tauriInvoke("set_window_theme", args)
        .await
        .map(|_| ())
        .map_err(|e| format!("Invoke error: {:?}", e))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeValue {
    Name(String),
    Custom(serde_json::Value),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TerminalConfig {
    #[serde(default)]
    pub theme: Option<ThemeValue>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size: Option<f64>,
    #[serde(default)]
    pub character_height: Option<f64>,
    #[serde(default)]
    pub renderer: Option<String>,
    /// Passed through untouched to the JS tmux client.
    #[serde(default)]
    pub tmux: Option<serde_json::Value>,
}

pub fn apply_terminal_config(config: &TerminalConfig) {
    if let Ok(val) = serde_wasm_bindgen::to_value(config) {
        apply_terminal_config_js(val);
    }
}

pub async fn init_terminal_session(container_id: &str, session_id: &str) {
    let _ = init_terminal_session_js(container_id, session_id).await;
}

pub fn close_terminal_session(session_id: &str) {
    close_terminal_session_js(session_id);
}

#[allow(dead_code)]
pub fn clear_terminal_session(session_id: Option<&str>) {
    clear_terminal_session_js(session_id);
}

pub fn fit_terminal_session(session_id: Option<&str>) {
    fit_terminal_session_js(session_id);
}

pub fn focus_terminal_session(session_id: Option<&str>) {
    focus_terminal_session_js(session_id);
}

pub async fn get_terminal_config() -> Result<TerminalConfig, String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({}))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        let res = tauriInvoke("get_terminal_config", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        serde_wasm_bindgen::from_value(res)
            .map_err(|e| format!("Failed to parse response: {:?}", e))
    } else {
        Ok(TerminalConfig::default())
    }
}

#[allow(dead_code)]
pub async fn get_config_path() -> Result<String, String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({}))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        let res = tauriInvoke("get_config_path", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        serde_wasm_bindgen::from_value(res)
            .map_err(|e| format!("Failed to parse response: {:?}", e))
    } else {
        Ok("~/.config/mdterm/config.yml".to_string())
    }
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
                if path == "index.html" || path.ends_with(".html") || path.ends_with(".htm") {
                    return Ok(crate::components::samples::SAMPLE_HTML.to_string());
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
                name: "index.html".to_string(),
                path: "index.html".to_string(),
                is_dir: false,
                size: 1800,
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

pub async fn get_cli_file() -> Result<Option<String>, String> {
    if is_tauri_env() {
        let args = serde_wasm_bindgen::to_value(&json!({}))
            .map_err(|e| format!("Failed to serialize args: {:?}", e))?;
        let res = tauriInvoke("get_cli_file", args)
            .await
            .map_err(|e| format!("Invoke error: {:?}", e))?;
        serde_wasm_bindgen::from_value(res)
            .map_err(|e| format!("Failed to parse response: {:?}", e))
    } else {
        Ok(None)
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

#[wasm_bindgen(module = "/js/tmux-client.js")]
extern "C" {
    #[wasm_bindgen(js_name = tmuxAction)]
    pub fn tmux_action(session_id: &str, action: &str);

    #[wasm_bindgen(js_name = tmuxDetect)]
    async fn tmux_detect_js() -> wasm_bindgen::JsValue;

    #[wasm_bindgen(js_name = tmuxAttach, catch)]
    async fn tmux_attach_js(session: Option<String>, create: bool) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

// tmux-client.js imports these two modules relatively; wasm-bindgen only copies
// JS files that are referenced here, so reference each once to bundle it.
#[wasm_bindgen(module = "/js/tmux-ui.js")]
extern "C" {
    #[wasm_bindgen(js_name = setPrefixBadge)]
    fn _unused_tmux_ui();
}

#[wasm_bindgen(module = "/js/tmux-keys.js")]
extern "C" {
    #[wasm_bindgen(js_name = keyEventToTmux)]
    fn _unused_tmux_keys();
}

/// Session ids of tabs that show a tmux window (`tmux-<conn>-<window>`).
pub fn is_tmux_session(session_id: &str) -> bool {
    session_id.starts_with("tmux-")
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
pub struct TmuxSessionInfo {
    pub name: String,
    #[serde(default)]
    pub windows: u32,
    #[serde(default)]
    pub attached: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
pub struct TmuxDetectInfo {
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub sessions: Vec<TmuxSessionInfo>,
    #[serde(default)]
    pub integration: String,
    #[serde(default)]
    pub session: String,
}

pub async fn tmux_detect() -> Option<TmuxDetectInfo> {
    let val = tmux_detect_js().await;
    if val.is_null() || val.is_undefined() {
        return None;
    }
    serde_wasm_bindgen::from_value(val).ok()
}

pub async fn tmux_attach(session: Option<String>, create: bool) -> Result<(), String> {
    tmux_attach_js(session, create)
        .await
        .map(|_| ())
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{:?}", e)))
}
