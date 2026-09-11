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
        await window.__TAURI__.core.invoke('pty_write', { data: packet });
        return true;
    }
    return false;
}

export async function closeRemoteSession() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        await window.__TAURI__.core.invoke('pty_write', { data: '__MDTERM_CLOSE__\n' }).catch(() => {});
        return true;
    }
    return false;
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


const TERMINAL_THEMES = {
    dark: {
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
    light: {
        background: '#f8fafc',
        foreground: '#0f172a',
        cursor: '#0284c7',
        cursorAccent: '#f8fafc',
        selectionBackground: 'rgba(2, 132, 199, 0.25)',
        black: '#334155',
        red: '#dc2626',
        green: '#16a34a',
        yellow: '#ca8a04',
        blue: '#2563eb',
        magenta: '#9333ea',
        cyan: '#0891b2',
        white: '#cbd5e1',
        brightBlack: '#475569',
        brightRed: '#b91c1c',
        brightGreen: '#15803d',
        brightYellow: '#a16207',
        brightBlue: '#1d4ed8',
        brightMagenta: '#7e22ce',
        brightCyan: '#0e7490',
        brightWhite: '#020617'
    },
    nord: {
        background: '#242933',
        foreground: '#eceff4',
        cursor: '#88c0d0',
        cursorAccent: '#242933',
        selectionBackground: 'rgba(136, 192, 208, 0.3)',
        black: '#2e3440',
        red: '#bf616a',
        green: '#a3be8c',
        yellow: '#ebcb8b',
        blue: '#81a1c1',
        magenta: '#b48ead',
        cyan: '#88c0d0',
        white: '#e5e9f0',
        brightBlack: '#4c566a',
        brightRed: '#bf616a',
        brightGreen: '#a3be8c',
        brightYellow: '#ebcb8b',
        brightBlue: '#81a1c1',
        brightMagenta: '#b48ead',
        brightCyan: '#8fbcbb',
        brightWhite: '#eceff4'
    },
    monokai: {
        background: '#1e1f1c',
        foreground: '#f8f8f2',
        cursor: '#f8f8f0',
        cursorAccent: '#1e1f1c',
        selectionBackground: 'rgba(73, 72, 62, 0.7)',
        black: '#272822',
        red: '#f92672',
        green: '#a6e22e',
        yellow: '#e6db74',
        blue: '#66d9ef',
        magenta: '#ae81ff',
        cyan: '#a1efe4',
        white: '#f8f8f2',
        brightBlack: '#75715e',
        brightRed: '#f92672',
        brightGreen: '#a6e22e',
        brightYellow: '#e6db74',
        brightBlue: '#66d9ef',
        brightMagenta: '#ae81ff',
        brightCyan: '#a1efe4',
        brightWhite: '#f9f8f5'
    }
};

function getTerminalTheme(name) {
    if (!name) return TERMINAL_THEMES.dark;
    const clean = name.toLowerCase().replace(/^theme-/, '');
    return TERMINAL_THEMES[clean] || TERMINAL_THEMES.dark;
}

export function renderMermaidDiagrams() {
    if (typeof mermaid === 'undefined') return;

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

    const targets = document.querySelectorAll('.mermaid-preview-target');
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
            if (target && codeEl && typeof mermaid !== 'undefined') {
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
            }
        }
    }
};

export function setTerminalTheme(themeName) {
    window._mdtermCurrentTheme = themeName;
    let theme;
    if (typeof themeName === 'object' && themeName !== null) {
        theme = Object.assign({}, TERMINAL_THEMES.dark, themeName);
    } else {
        theme = getTerminalTheme(themeName);
    }
    if (window._mdtermTerminal) {
        window._mdtermTerminal.options.theme = theme;
        try {
            if (typeof window._mdtermTerminal.refresh === 'function') {
                window._mdtermTerminal.refresh(0, (window._mdtermTerminal.rows || 24) - 1);
            }
        } catch (e) {}
    }
    const container = document.getElementById('mdterm-xterm-container');
    if (container && theme && theme.background) {
        container.style.backgroundColor = theme.background;
    }
    try {
        renderMermaidDiagrams();
    } catch (e) {}
}

export function applyTerminalConfig(cfg) {
    if (!cfg) return;
    window._mdtermTerminalConfig = cfg;
    if (cfg.theme) {
        setTerminalTheme(cfg.theme);
    }
    if (window._mdtermTerminal) {
        if (cfg.font_family || cfg.fontFamily) {
            window._mdtermTerminal.options.fontFamily = cfg.font_family || cfg.fontFamily;
        }
        if (cfg.font_size || cfg.fontSize) {
            const parsed = Number(cfg.font_size || cfg.fontSize);
            if (!isNaN(parsed) && parsed > 0) {
                window._mdtermTerminal.options.fontSize = parsed;
            }
        }
        const rawHeight = cfg.character_height || cfg.characterHeight || cfg.line_height || cfg.lineHeight;
        if (rawHeight !== undefined && rawHeight !== null) {
            let ch = Number(rawHeight);
            if (!isNaN(ch) && ch > 0) {
                if (ch > 5.0) {
                    const currentFontSize = window._mdtermTerminal.options.fontSize || 13;
                    ch = ch / currentFontSize;
                }
                window._mdtermTerminal.options.lineHeight = Math.max(1.0, ch);
            }
        }
        if (window._mdtermFitAddon) {
            try {
                window._mdtermFitAddon.fit();
                if (window.__TAURI__) {
                    const invoke = (window.__TAURI__.core && window.__TAURI__.core.invoke) || (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke);
                    if (invoke) {
                        invoke('pty_resize', { cols: window._mdtermTerminal.cols, rows: window._mdtermTerminal.rows }).catch(() => {});
                    }
                }
            } catch (e) {}
        }
    }
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

    const cfg = window._mdtermTerminalConfig || {};
    let fontSize = 13;
    if (cfg.font_size || cfg.fontSize) {
        const parsed = Number(cfg.font_size || cfg.fontSize);
        if (!isNaN(parsed) && parsed > 0) fontSize = parsed;
    }
    let fontFamily = 'JetBrains Mono, Menlo, Monaco, Consolas, "Courier New", monospace';
    if (cfg.font_family || cfg.fontFamily) {
        fontFamily = cfg.font_family || cfg.fontFamily;
    }
    let lineHeight = 1.25;
    const rawHeight = cfg.character_height || cfg.characterHeight || cfg.line_height || cfg.lineHeight;
    if (rawHeight !== undefined && rawHeight !== null) {
        let ch = Number(rawHeight);
        if (!isNaN(ch) && ch > 0) {
            if (ch > 5.0) {
                ch = ch / fontSize;
            }
            lineHeight = Math.max(1.0, ch);
        }
    }
    let initialTheme;
    if (cfg.theme) {
        if (typeof cfg.theme === 'object' && cfg.theme !== null) {
            initialTheme = Object.assign({}, TERMINAL_THEMES.dark, cfg.theme);
        } else {
            initialTheme = getTerminalTheme(cfg.theme);
            window._mdtermCurrentTheme = cfg.theme;
        }
    } else {
        initialTheme = getTerminalTheme(window._mdtermCurrentTheme || 'dark');
    }

    const term = new Terminal({
        cursorBlink: true,
        cursorStyle: 'block',
        fontSize: fontSize,
        lineHeight: lineHeight,
        fontFamily: fontFamily,
        theme: initialTheme,
        convertEol: true,
        allowTransparency: false
    });

    let fitAddon = null;
    if (typeof FitAddon !== 'undefined' && FitAddon.FitAddon) {
        fitAddon = new FitAddon.FitAddon();
        term.loadAddon(fitAddon);
    }

    term.open(container);

    const tauriInvoke = window.__TAURI__ && window.__TAURI__.core
        ? window.__TAURI__.core.invoke
        : null;
    const hasTauri = typeof tauriInvoke === 'function';

    const writeClipboardText = async (text) => {
        if (hasTauri) {
            await tauriInvoke('plugin:clipboard-manager|write_text', { text });
            return;
        }
        if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
            await navigator.clipboard.writeText(text);
        }
    };

    const readClipboardText = async () => {
        if (hasTauri) {
            return await tauriInvoke('plugin:clipboard-manager|read_text');
        }
        if (navigator.clipboard && typeof navigator.clipboard.readText === 'function') {
            return await navigator.clipboard.readText();
        }
        return '';
    };

    // Use native terminal shortcuts so copy/paste works independently of WebView permissions.
    const isMac = /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent || '');
    term.attachCustomKeyEventHandler((event) => {
        if (event.type !== 'keydown') return true;

        const key = event.key.toLowerCase();
        const copyShortcut = (isMac && event.metaKey && key === 'c') ||
            (!isMac && event.ctrlKey && event.shiftKey && key === 'c') ||
            (event.ctrlKey && key === 'insert');
        const pasteShortcut = (isMac && event.metaKey && key === 'v') ||
            (!isMac && event.ctrlKey && event.shiftKey && key === 'v') ||
            (event.shiftKey && key === 'insert');

        if (copyShortcut && term.hasSelection()) {
            writeClipboardText(term.getSelection()).catch((error) => {
                console.error('Failed to copy terminal selection:', error);
            });
            return false;
        }

        if (pasteShortcut) {
            readClipboardText()
                .then((text) => {
                    if (text) term.paste(text);
                })
                .catch((error) => {
                    console.error('Failed to paste into terminal:', error);
                });
            return false;
        }

        return true;
    });

    const focusTerm = () => {
        try {
            const active = document.activeElement;
            const isEditorActive = active && (
                active.closest('.editor-pane') ||
                active.closest('.modal-content') ||
                active.tagName === 'INPUT' ||
                (active.tagName === 'TEXTAREA' && !active.classList.contains('xterm-helper-textarea')) ||
                active.getAttribute('contenteditable') === 'true'
            );
            if (isEditorActive) {
                return;
            }
            if (typeof window !== 'undefined' && typeof window.focus === 'function') {
                window.focus();
            }
            term.focus();
            const textarea = container.querySelector('.xterm-helper-textarea');
            if (textarea && document.activeElement !== textarea) {
                textarea.focus({ preventScroll: true });
            }
        } catch (e) {}
    };

    // Immediate & staggered focus to ensure terminal is active upon app start
    focusTerm();
    if (typeof requestAnimationFrame === 'function') {
        requestAnimationFrame(focusTerm);
    }
    setTimeout(() => {
        if (fitAddon) {
            try { fitAddon.fit(); } catch (e) {}
        }
        focusTerm();
    }, 60);
    setTimeout(focusTerm, 150);
    setTimeout(focusTerm, 300);
    setTimeout(focusTerm, 600);

    // Clicking anywhere in the container or terminal pane focuses the terminal
    const handlePaneClick = () => {
        const sel = window.getSelection ? window.getSelection().toString() : '';
        if (!sel || sel.length === 0) {
            focusTerm();
        }
    };
    container.addEventListener('click', handlePaneClick);
    const pane = container.closest('.terminal-pane') || container.parentElement;
    if (pane && pane !== container) {
        pane.addEventListener('click', handlePaneClick);
    }

    // Auto-focus terminal on window focus if no editor/modal is active
    const handleWindowFocus = () => {
        focusTerm();
    };
    window.addEventListener('focus', handleWindowFocus);

    // Decode UTF-8 string from Base64
    const decodeB64 = (str) => {
        if (!str) return '';
        try {
            const binary = atob(str.trim());
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
                bytes[i] = binary.charCodeAt(i);
            }
            return new TextDecoder().decode(bytes);
        } catch (e) {
            return atob(str.trim());
        }
    };

    // Show animated toast notification
    const showToast = (message) => {
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
    };

    // Handler for OSC escape sequences: \x1b]5337;open;NAME_B64;PATH_B64;CONTENT_B64[;SIDE]\x07
    const handleOpenOsc = (data) => {
        try {
            const parts = data.split(';');
            const action = parts[0];
            let side = 'right';
            if (action.endsWith('-left')) {
                side = 'left';
            } else if (action.endsWith('-right')) {
                side = 'right';
            } else if (parts[4] && parts[4].toLowerCase() === 'left') {
                side = 'left';
            }

            if (action.startsWith('open') || action.startsWith('open-content')) {
                const name = decodeB64(parts[1]) || 'document.md';
                const path = decodeB64(parts[2]);
                const content = decodeB64(parts[3]);
                const isRemote = action.includes('remote') || action.includes('wait');

                showToast("Opened '" + name + "' in editor (" + (isRemote ? "remote, " : "") + side + ")");
                window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                    detail: { name, path, content, side, is_remote: isRemote }
                }));
                return true;
            } else if (action === 'saved') {
                const name = decodeB64(parts[1]) || 'document.md';
                showToast("✓ Saved '" + name + "' on remote server");
                window.dispatchEvent(new CustomEvent('mdterm-file-saved', {
                    detail: { name }
                }));
                return true;
            } else if (action === 'closed') {
                const name = decodeB64(parts[1]) || 'document.md';
                window.dispatchEvent(new CustomEvent('mdterm-remote-closed', {
                    detail: { name }
                }));
                return true;
            } else if (action.startsWith('open-file')) {
                const path = parts.slice(1).join(';');
                const name = path.split('/').pop() || path;
                showToast("Opened '" + name + "' in editor (" + side + ")");
                window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                    detail: { name, path, content: '', side, is_remote: false }
                }));
                return true;
            }
        } catch (e) {
            console.error('Error handling terminal OSC sequence:', e);
        }
        return false;
    };

    if (term.parser && typeof term.parser.registerOscHandler === 'function') {
        term.parser.registerOscHandler(5337, handleOpenOsc);
        term.parser.registerOscHandler(7777, handleOpenOsc);
        term.parser.registerOscHandler(1337, handleOpenOsc);

        // OSC 52: applications such as tmux use this sequence to copy to the host clipboard.
        term.parser.registerOscHandler(52, (data) => {
            try {
                const separator = data.indexOf(';');
                if (separator < 0) return false;

                const encodedText = data.slice(separator + 1);
                // Clipboard reads over OSC 52 are deliberately ignored; paste remains user-initiated.
                if (!encodedText || encodedText === '?') return true;

                const text = decodeB64(encodedText);
                writeClipboardText(text).catch((error) => {
                    console.error('Failed to handle OSC 52 clipboard write:', error);
                });
                return true;
            } catch (error) {
                console.error('Invalid OSC 52 clipboard sequence:', error);
                return false;
            }
        });
    }

    // Clickable file opening handler
    const handleTerminalFileClick = async (clickedPath, event) => {
        if (!clickedPath) return;

        let cleanPath = clickedPath.trim().replace(/^['"`]+|['"`]+$/g, '');
        if (cleanPath.includes(':')) {
            const colonIdx = cleanPath.indexOf(':');
            if (colonIdx > 0) cleanPath = cleanPath.substring(0, colonIdx);
        }

        const filename = cleanPath.split('/').pop() || cleanPath;

        // Detect if user held Super (Meta / Windows / Command) during click
        const isSuper = !!(event && (
            event.metaKey ||
            (typeof event.getModifierState === 'function' && (
                event.getModifierState('Meta') ||
                event.getModifierState('Super') ||
                event.getModifierState('OS')
            ))
        ));
        const side = isSuper ? 'left' : 'right';

        if (hasTauri) {
            const invoke = window.__TAURI__.core.invoke;
            let resolvedPath = cleanPath;

            try {
                if (!cleanPath.startsWith('/') && !cleanPath.startsWith('~')) {
                    let cwd = window._mdtermCwd || '';
                    if (!cwd) {
                        cwd = await invoke('pty_get_cwd').catch(() => '');
                    }
                    if (cwd) {
                        resolvedPath = cwd.replace(/\/+$/, '') + '/' + cleanPath;
                    }
                } else if (cleanPath.startsWith('~/')) {
                    const home = await invoke('get_home_dir').catch(() => '');
                    if (home) {
                        resolvedPath = home + cleanPath.slice(1);
                    }
                }

                // Try reading file locally
                const content = await invoke('read_file', { path: resolvedPath });
                showToast("Opened '" + filename + "' (" + side + ")");
                window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                    detail: { name: filename, path: resolvedPath, content, side }
                }));
                return;
            } catch (e) {
                // Not found locally or error reading local file.
                // This happens when the user is on a remote SSH server or inside a remote tmux session!
            }

            // Fallback for remote SSH / tmux: invoke mdterm in terminal
            showToast("Opening '" + filename + "' (" + side + ")...");
            const flag = isSuper ? '--left ' : '';
            invoke('pty_write', { data: 'mdterm ' + flag + '"' + cleanPath + '"\n' }).catch(() => {});
        } else {
            // Browser preview mode
            showToast("Opened '" + filename + "' (" + side + ")");
            const isHtml = /\.(html|htm|xhtml)$/i.test(filename);
            const sampleContent = isHtml
                ? `<!DOCTYPE html>\n<html lang="en">\n<head>\n  <meta charset="utf-8" />\n  <title>${filename}</title>\n</head>\n<body>\n  <h1>${filename}</h1>\n  <p>Opened via terminal click.</p>\n</body>\n</html>`
                : '# ' + filename + '\n\nOpened via terminal click.';
            window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                detail: { name: filename, path: cleanPath, content: sampleContent, side }
            }));
        }
    };

    // OSC 7 for shell current working directory reporting
    if (term.parser && typeof term.parser.registerOscHandler === 'function') {
        term.parser.registerOscHandler(7, (data) => {
            try {
                if (data.startsWith('file://')) {
                    const url = new URL(data);
                    window._mdtermCwd = decodeURI(url.pathname);
                }
            } catch (e) {}
            return true;
        });
    }

    // Register Link Provider in xterm.js for files (like from `ls` output)
    const fileLinkRegex = /(?:^|[\s"'\(\)\[\]<>{},;:`])((?:(?:\.|\.\.|\~)?\/)?(?:[\w.-]+\/)*[\w.-]+\.(?:md|markdown|mdown|mkd|txt|rst|org|html|htm|xhtml|toml|json|yaml|yml|sh|rs|js|ts|css|py|c|cpp|h|go))(?:[\s"'\(\)\[\]<>{},;:`]|$)/gi;

    if (typeof term.registerLinkProvider === 'function') {
        term.registerLinkProvider({
            provideLinks(bufferLineNumber, callback) {
                const line = term.buffer.active.getLine(bufferLineNumber - 1);
                if (!line) {
                    callback(undefined);
                    return;
                }

                const lineText = line.translateToString(true);
                const links = [];
                let match;
                fileLinkRegex.lastIndex = 0;

                while ((match = fileLinkRegex.exec(lineText)) !== null) {
                    const fullMatch = match[0];
                    const rawPath = match[1];
                    const leadingOffset = fullMatch.indexOf(rawPath);
                    const startX = match.index + leadingOffset + 1; // 1-based x
                    const endX = startX + rawPath.length - 1;

                    links.push({
                        range: {
                            start: { x: startX, y: bufferLineNumber },
                            end: { x: endX, y: bufferLineNumber }
                        },
                        text: rawPath,
                        activate: async (event, clickedText) => {
                            await handleTerminalFileClick(clickedText, event);
                        }
                    });

                    fileLinkRegex.lastIndex = match.index + leadingOffset + rawPath.length;
                }

                callback(links);
            }
        });
    }

    if (hasTauri) {
        const invoke = window.__TAURI__.core.invoke;
        const event = window.__TAURI__.event;

        const cols = term.cols || 80;
        const rows = term.rows || 24;
        invoke('pty_spawn', { cols, rows }).catch(err => {
            term.write('\r\n\x1b[31mFailed to spawn shell: ' + err + '\x1b[0m\r\n');
        });

        invoke('get_terminal_config').then(loadedCfg => {
            if (loadedCfg) {
                applyTerminalConfig(loadedCfg);
            }
        }).catch(err => {
            console.error('Failed to get terminal config:', err);
        });

        if (event && typeof event.listen === 'function') {
            event.listen('pty-output', (e) => {
                term.write(e.payload);
            });
            event.listen('terminal-config-changed', (e) => {
                if (e.payload) {
                    applyTerminalConfig(e.payload);
                    window.dispatchEvent(new CustomEvent('mdterm-config-changed', { detail: e.payload }));
                }
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
                } else if (buf.trim().startsWith('mdterm ')) {
                    const fname = buf.trim().slice(7).trim();
                    const isHtml = /\.(html|htm|xhtml)$/i.test(fname);
                    const sampleContent = isHtml
                        ? `<!DOCTYPE html>\n<html lang="en">\n<head>\n  <meta charset="utf-8" />\n  <title>${fname}</title>\n</head>\n<body>\n  <h1>${fname}</h1>\n  <p>Opened via terminal in mdterm editor.</p>\n</body>\n</html>`
                        : '# ' + fname + '\n\nOpened via terminal in mdterm editor.';
                    window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                        detail: { name: fname, path: fname, content: sampleContent }
                    }));
                    term.write('\x1b[32m✓ Opened \'' + fname + '\' in mdterm editor\x1b[0m\r\n');
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
    if (window._mdtermTerminalConfig) {
        applyTerminalConfig(window._mdtermTerminalConfig);
    } else if (window._mdtermCurrentTheme) {
        setTerminalTheme(window._mdtermCurrentTheme);
    }
}

export function clearTerminalSession() {
    if (window._mdtermTerminal) {
        window._mdtermTerminal.clear();
    }
}

export function fitTerminalSession() {
    const doFit = () => {
        if (window._mdtermFitAddon) {
            try {
                window._mdtermFitAddon.fit();
                if (window._mdtermTerminal && window.__TAURI__) {
                    const invoke = (window.__TAURI__.core && window.__TAURI__.core.invoke) || (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke);
                    if (invoke) {
                        invoke('pty_resize', { cols: window._mdtermTerminal.cols, rows: window._mdtermTerminal.rows }).catch(() => {});
                    }
                }
            } catch (e) {}
        }
    };
    doFit();
    setTimeout(doFit, 50);
    setTimeout(doFit, 150);
}

export function focusTerminalSession() {
    if (window._mdtermTerminal) {
        try {
            if (typeof window !== 'undefined' && typeof window.focus === 'function') {
                window.focus();
            }
            window._mdtermTerminal.focus();
            const textarea = document.querySelector('.terminal-container .xterm-helper-textarea');
            if (textarea && document.activeElement !== textarea) {
                textarea.focus({ preventScroll: true });
            }
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

    #[wasm_bindgen(js_name = setTerminalTheme)]
    pub fn set_terminal_theme(theme_name: &str);

    #[wasm_bindgen(js_name = applyTerminalConfig)]
    pub fn apply_terminal_config_js(config: wasm_bindgen::JsValue);

    #[wasm_bindgen(js_name = renderMermaidDiagrams)]
    pub fn render_mermaid_diagrams();

    #[wasm_bindgen(js_name = clearTerminalSession)]
    pub fn clear_terminal_session();

    #[wasm_bindgen(js_name = fitTerminalSession)]
    pub fn fit_terminal_session();

    #[wasm_bindgen(js_name = focusTerminalSession)]
    pub fn focus_terminal_session();

    #[wasm_bindgen(js_name = showToast)]
    pub fn show_toast(message: &str);

    #[wasm_bindgen(js_name = sendRemoteSave, catch)]
    async fn sendRemoteSave(
        path: &str,
        content: &str,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_name = closeRemoteSession, catch)]
    async fn closeRemoteSession() -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

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
}

pub fn apply_terminal_config(config: &TerminalConfig) {
    if let Ok(val) = serde_wasm_bindgen::to_value(config) {
        apply_terminal_config_js(val);
    }
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
