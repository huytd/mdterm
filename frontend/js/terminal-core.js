/**
 * terminal-core.js
 * Core terminal module for mdterm (Tauri 2 + xterm.js 6.x)
 */

import {
  isTmuxSessionId, mountTmuxWindow, focusTmuxWindow, fitTmuxWindow, unmountTmuxWindow,
  tmuxApplyConfig, tmuxSetTheme
} from './tmux-client.js';

export const TERMINAL_THEMES = {
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

export function getTerminalTheme(name) {
  if (!name) return TERMINAL_THEMES.dark;
  if (typeof name === 'object' && name !== null) {
    return Object.assign({}, TERMINAL_THEMES.dark, name);
  }
  const clean = String(name).toLowerCase().replace(/^theme-/, '');
  return TERMINAL_THEMES[clean] || TERMINAL_THEMES.dark;
}

export function normalizeFontFamily(str) {
  if (!str || typeof str !== 'string') return str;
  const parts = str.split(',')
    .map(part => part.trim())
    .filter(part => part.length > 0)
    .map(part => {
      let clean = part;
      if ((clean.startsWith('"') && clean.endsWith('"')) || (clean.startsWith("'") && clean.endsWith("'"))) {
        clean = clean.slice(1, -1).trim();
      }
      const lower = clean.toLowerCase();
      if (['monospace', 'sans-serif', 'serif', 'system-ui', 'ui-monospace', 'cursive', 'fantasy'].includes(lower)) {
        return lower;
      }
      return `"${clean}"`;
    });
  const hasGeneric = parts.some(p => ['monospace', 'sans-serif', 'serif', 'system-ui', 'ui-monospace'].includes(p.toLowerCase()));
  if (!hasGeneric) {
    parts.push('Menlo', 'Monaco', 'Consolas', '"Courier New"', 'monospace');
  }
  return parts.join(', ');
}

export function computeLineHeight(rawHeight, fontSize = 13) {
  let lineHeight = 1.0;
  if (rawHeight !== undefined && rawHeight !== null) {
    let ch = Number(rawHeight);
    if (!isNaN(ch) && ch > 0) {
      if (ch > 5.0) {
        const fs = Number(fontSize) || 13;
        ch = ch / fs;
      }
      lineHeight = Math.max(1.0, ch);
    }
  }
  return lineHeight;
}

export function buildTerminalOptions(cfg = {}) {
  let fontSize = 13;
  if (cfg.font_size || cfg.fontSize) {
    const parsed = Number(cfg.font_size || cfg.fontSize);
    if (!isNaN(parsed) && parsed > 0) fontSize = parsed;
  }

  const defaultFontFamily = '"JetBrains Mono", Menlo, Monaco, Consolas, "Courier New", monospace';
  let fontFamily = defaultFontFamily;
  const rawFont = cfg.font_family || cfg.fontFamily;
  if (rawFont) {
    fontFamily = normalizeFontFamily(rawFont);
  }

  const rawHeight = cfg.character_height || cfg.characterHeight || cfg.line_height || cfg.lineHeight;
  const lineHeight = computeLineHeight(rawHeight, fontSize);

  let initialTheme;
  if (cfg.theme) {
    if (typeof cfg.theme === 'object' && cfg.theme !== null) {
      initialTheme = Object.assign({}, TERMINAL_THEMES.dark, cfg.theme);
    } else {
      initialTheme = getTerminalTheme(cfg.theme);
    }
  } else {
    initialTheme = getTerminalTheme((typeof window !== 'undefined' && window._mdtermCurrentTheme) || 'dark');
  }

  return {
    allowProposedApi: true,
    cursorBlink: true,
    cursorStyle: 'block',
    fontSize,
    lineHeight,
    fontFamily,
    theme: initialTheme,
    convertEol: false,
    customGlyphs: true,
    allowTransparency: false
  };
}

function getTerminalCtor(opts) {
  if (opts && opts.Terminal) return opts.Terminal;
  if (typeof Terminal !== 'undefined') {
    return Terminal.Terminal || Terminal;
  }
  if (typeof window !== 'undefined' && window.Terminal) {
    return window.Terminal.Terminal || window.Terminal;
  }
  return null;
}

function getFitAddonCtor(opts) {
  if (opts && opts.FitAddon) return opts.FitAddon;
  if (typeof FitAddon !== 'undefined') {
    return FitAddon.FitAddon || FitAddon;
  }
  if (typeof window !== 'undefined' && window.FitAddon) {
    return window.FitAddon.FitAddon || window.FitAddon;
  }
  return null;
}

function getWebglAddonCtor(opts) {
  if (opts && opts.WebglAddon) return opts.WebglAddon;
  if (typeof WebglAddon !== 'undefined') {
    return WebglAddon.WebglAddon || WebglAddon;
  }
  if (typeof window !== 'undefined' && window.WebglAddon) {
    return window.WebglAddon.WebglAddon || window.WebglAddon;
  }
  return null;
}

function getUnicode11AddonCtor(opts) {
  if (opts && opts.Unicode11Addon) return opts.Unicode11Addon;
  if (typeof Unicode11Addon !== 'undefined') {
    return Unicode11Addon.Unicode11Addon || Unicode11Addon;
  }
  if (typeof window !== 'undefined' && window.Unicode11Addon) {
    return window.Unicode11Addon.Unicode11Addon || window.Unicode11Addon;
  }
  return null;
}

function getTauriInvoke() {
  if (typeof window !== 'undefined') {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      return window.__TAURI__.core.invoke;
    }
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
      return window.__TAURI_INTERNALS__.invoke;
    }
  }
  return null;
}

async function tauriInvoke(cmd, args) {
  const inv = getTauriInvoke();
  if (inv) {
    return await inv(cmd, args);
  }
  throw new Error('Tauri IPC is not available in browser mode');
}

function windowShow() {
  if (typeof window !== 'undefined') {
    if (typeof window.windowShow === 'function') {
      window.windowShow();
      return;
    }
    const inv = getTauriInvoke();
    if (inv) {
      inv('window_show').catch(() => {
        inv('plugin:window|show').catch(() => {});
      });
    }
  }
}

function decodeB64(str) {
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
}

function showToast(message) {
  if (typeof window !== 'undefined' && typeof window.showToast === 'function') {
    window.showToast(message);
    return;
  }
  if (typeof document === 'undefined') return;
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

/**
 * Pure terminal creation function.
 * Usable in test harnesses without Tauri.
 */
export async function createTerminal(container, cfg = {}, opts = {}) {
  const TerminalCtor = getTerminalCtor(opts);
  if (!TerminalCtor) {
    throw new Error('Terminal library (xterm.js) is not loaded.');
  }

  const termOptions = buildTerminalOptions(cfg);

  // F5: before term.open(), await document.fonts.load(fontSize + 'px ' + fontFamily) (try/catch, 1s timeout) and document.fonts.ready
  if (typeof document !== 'undefined' && document.fonts && typeof document.fonts.load === 'function') {
    try {
      const fontSpec = `${termOptions.fontSize}px ${termOptions.fontFamily}`;
      const fontLoadPromise = document.fonts.load(fontSpec);
      const timeoutPromise = new Promise(resolve => setTimeout(resolve, 1000));
      await Promise.race([fontLoadPromise, timeoutPromise]);
    } catch (e) {
      // ignore font load timeout/error
    }
    try {
      if (document.fonts.ready) {
        const readyTimeout = new Promise(resolve => setTimeout(resolve, 1000));
        await Promise.race([document.fonts.ready, readyTimeout]);
      }
    } catch (e) {
      // ignore ready error
    }
  }

  const term = new TerminalCtor(termOptions);

  // Load Unicode11
  const Unicode11AddonCtor = getUnicode11AddonCtor(opts);
  if (Unicode11AddonCtor) {
    try {
      const unicode11 = new Unicode11AddonCtor();
      term.loadAddon(unicode11);
      if (term.unicode) {
        term.unicode.activeVersion = '11';
      }
    } catch (e) {
      console.warn('Failed to load Unicode11Addon:', e);
    }
  }

  // Load FitAddon
  let fitAddon = null;
  const FitAddonCtor = getFitAddonCtor(opts);
  if (FitAddonCtor) {
    try {
      fitAddon = new FitAddonCtor();
      term.loadAddon(fitAddon);
    } catch (e) {
      console.warn('Failed to load FitAddon:', e);
    }
  }

  // Open terminal in container if container is provided
  if (container) {
    term.open(container);
  }

  // Renderer: cfg.renderer 'dom' (default) | 'webgl'. The DOM renderer is the
  // default because WebGL glyph output was observed to be unreliable under
  // WebKit (missing box-drawing glyphs / blank frames). webgl falls back to DOM
  // if the addon throws or the context is lost.
  let currentRenderer = 'dom';
  let webglAddonInstance = null;
  const targetRenderer = (cfg && cfg.renderer) ? String(cfg.renderer).toLowerCase() : 'dom';
  const WebglAddonCtor = getWebglAddonCtor(opts);

  if (targetRenderer === 'webgl' && WebglAddonCtor && container) {
    try {
      const webgl = new WebglAddonCtor();
      if (typeof webgl.onContextLoss === 'function') {
        webgl.onContextLoss(() => {
          try {
            webgl.dispose();
          } catch (e) {}
          webglAddonInstance = null;
          currentRenderer = 'dom';
          console.warn('WebGL context lost, fell back to DOM renderer');
        });
      }
      term.loadAddon(webgl);
      webglAddonInstance = webgl;
      currentRenderer = 'webgl';
    } catch (e) {
      console.warn('Failed to load WebglAddon, falling back to DOM renderer:', e);
      if (webglAddonInstance) {
        try { webglAddonInstance.dispose(); } catch (err) {}
        webglAddonInstance = null;
      }
      currentRenderer = 'dom';
    }
  }

  if (fitAddon && container && container.clientWidth > 0 && container.clientHeight > 0) {
    try { fitAddon.fit(); } catch (e) {}
  }

  let lastCols = term.cols || 0;
  let lastRows = term.rows || 0;
  let resizeTimer = null;
  let rafId = null;
  let disposed = false;

  // F4: Debounced resize logic
  function performResize() {
    if (disposed) return;
    if (!container) return;

    // skip when the container is hidden (offsetParent === null or clientWidth/clientHeight 0)
    if (container.offsetParent === null || container.clientWidth === 0 || container.clientHeight === 0) {
      return;
    }

    if (fitAddon) {
      try {
        fitAddon.fit();
      } catch (e) {}
    }

    const cols = term.cols;
    const rows = term.rows;

    if (cols > 0 && rows > 0 && (cols !== lastCols || rows !== lastRows)) {
      lastCols = cols;
      lastRows = rows;

      let cellW = 0;
      let cellH = 0;
      try {
        const cell = term._core?._renderService?.dimensions?.css?.cell;
        if (cell) {
          cellW = Number(cell.width) || 0;
          cellH = Number(cell.height) || 0;
        }
      } catch (e) {}

      const pixelWidth = cellW > 0 ? Math.round(cellW * cols) : 0;
      const pixelHeight = cellH > 0 ? Math.round(cellH * rows) : 0;

      if (typeof opts?.onResize === 'function') {
        opts.onResize(cols, rows, pixelWidth, pixelHeight);
      }
    }
  }

  function scheduleResize(immediate = false) {
    if (disposed) return;
    if (resizeTimer) {
      clearTimeout(resizeTimer);
      resizeTimer = null;
    }
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    if (immediate) {
      performResize();
      return;
    }

    resizeTimer = setTimeout(() => {
      resizeTimer = null;
      rafId = requestAnimationFrame(() => {
        rafId = null;
        performResize();
      });
    }, 50);
  }

  // F5: document.fonts 'loadingdone' listener that forces a re-measure (reassign options.fontFamily) and calls scheduleResize()
  let fontLoadingDoneHandler = null;
  if (typeof document !== 'undefined' && document.fonts && typeof document.fonts.addEventListener === 'function') {
    fontLoadingDoneHandler = () => {
      if (disposed) return;
      try {
        const curFont = term.options.fontFamily;
        term.options.fontFamily = '';
        term.options.fontFamily = curFont;
        if (typeof term.refresh === 'function') {
          term.refresh(0, (term.rows || 24) - 1);
        }
      } catch (e) {}
      scheduleResize();
    };
    document.fonts.addEventListener('loadingdone', fontLoadingDoneHandler);
  }

  let ro = null;
  if (typeof ResizeObserver !== 'undefined' && container && opts?.observe !== false) {
    ro = new ResizeObserver(() => {
      scheduleResize();
    });
    ro.observe(container);
  }

  function dispose() {
    if (disposed) return;
    disposed = true;
    if (ro) {
      ro.disconnect();
      ro = null;
    }
    if (resizeTimer) {
      clearTimeout(resizeTimer);
      resizeTimer = null;
    }
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    if (fontLoadingDoneHandler && typeof document !== 'undefined' && document.fonts) {
      document.fonts.removeEventListener('loadingdone', fontLoadingDoneHandler);
      fontLoadingDoneHandler = null;
    }
    if (webglAddonInstance) {
      try { webglAddonInstance.dispose(); } catch (e) {}
      webglAddonInstance = null;
    }
    if (fitAddon) {
      try { fitAddon.dispose(); } catch (e) {}
      fitAddon = null;
    }
    try {
      term.dispose();
    } catch (e) {}
  }

  return {
    term,
    fitAddon,
    ro,
    get renderer() { return currentRenderer; },
    dispose,
    scheduleResize
  };
}

/**
 * Wires clipboard, paste, app shortcuts, OSC handlers (editor open/save, OSC 52,
 * OSC 7 cwd) and file links into an xterm instance. Shared by plain PTY tabs
 * and tmux panes; `ctx.getCwd` / `ctx.sendInput` abstract the backend.
 */
export function installTerminalIntegration(term, container, ctx) {
  const { sId, sessionObj, hasTauri, tauriInvokeFn, getCwd, sendInput, handleKey } = ctx;
  const writeClipboardText = async (text) => {
    if (hasTauri) {
      await tauriInvokeFn('plugin:clipboard-manager|write_text', { text });
      return;
    }
    if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
      await navigator.clipboard.writeText(text);
    }
  };

  const readClipboardText = async () => {
    if (hasTauri) {
      return await tauriInvokeFn('plugin:clipboard-manager|read_text');
    }
    if (navigator.clipboard && typeof navigator.clipboard.readText === 'function') {
      return await navigator.clipboard.readText();
    }
    return '';
  };

  const originalPaste = term.paste.bind(term);
  let lastPasteTime = 0;
  let lastPasteText = '';
  term.paste = (data) => {
    if (!data) return;
    const now = Date.now();
    if (data === lastPasteText && (now - lastPasteTime) < 250) {
      return;
    }
    lastPasteTime = now;
    lastPasteText = data;
    originalPaste(data);
  };

  const handleDomPaste = (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (typeof e.stopImmediatePropagation === 'function') {
      e.stopImmediatePropagation();
    }
    const text = (e.clipboardData && typeof e.clipboardData.getData === 'function')
      ? e.clipboardData.getData('text/plain')
      : '';
    if (text) {
      term.paste(text);
    } else {
      readClipboardText().then((clipText) => {
        if (clipText) term.paste(clipText);
      }).catch(() => {});
    }
  };
  if (term.textarea) {
    term.textarea.addEventListener('paste', handleDomPaste, true);
  }
  container.addEventListener('paste', handleDomPaste, true);

  const isMac = /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent || '');
  term.attachCustomKeyEventHandler((event) => {
    if (event.type !== 'keydown') return true;

    if (typeof handleKey === 'function' && handleKey(event)) {
      event.preventDefault();
      event.stopPropagation();
      return false;
    }

    const key = event.key.toLowerCase();
    const isSuper = !!(event.metaKey || (typeof event.getModifierState === 'function' && (
      event.getModifierState('Meta') ||
      event.getModifierState('Super') ||
      event.getModifierState('OS')
    )));
    const isCtrl = event.ctrlKey;
    const isAlt = event.altKey;
    const isShift = event.shiftKey;

    // 1. Theme cycling: (Super or Ctrl) + Alt + T
    if ((isSuper || isCtrl) && isAlt && key === 't') {
      return false;
    }

    // 2. Tab creation: Super + T OR Ctrl + Shift + T
    if ((isSuper && !isAlt && key === 't') || (isCtrl && isShift && !isAlt && key === 't')) {
      return false;
    }

    // 3. Tab / Editor closing: Super + W OR Ctrl + Shift + W
    if ((isSuper && !isAlt && key === 'w') || (isCtrl && isShift && !isAlt && key === 'w')) {
      return false;
    }

    // 4. Tab cycling: Ctrl + Tab / Super + Tab / Ctrl + PageUp / Ctrl + PageDown
    if ((isSuper || isCtrl) && (key === 'tab' || key === 'pageup' || key === 'pagedown')) {
      return false;
    }

    // 5. Tab switching by number: Super + 1..9 OR Ctrl + Shift + 1..9
    if ((isSuper || (isCtrl && isShift)) && !isAlt && key >= '1' && key <= '9') {
      return false;
    }

    // 6. Editor mode switching: Alt + 1..3 OR Super + M OR Ctrl + Shift + M
    if ((isAlt && !isSuper && !isCtrl && (key === '1' || key === '2' || key === '3')) ||
      (isSuper && !isAlt && key === 'm') ||
      (isCtrl && isShift && !isAlt && key === 'm')) {
      return false;
    }

    // 7. Find & Replace: Super + F OR Ctrl + Shift + F (Ctrl + F alone goes to shell)
    if ((isSuper && !isAlt && key === 'f') || (isCtrl && isShift && !isAlt && key === 'f')) {
      return false;
    }

    // 8. Document shortcuts: Super + (S/O/N) OR Ctrl + Shift + (S/O/N)
    if ((isSuper || (isCtrl && isShift)) && !isAlt && (key === 's' || key === 'o' || key === 'n')) {
      return false;
    }

    // 9. Window close: Super + Q OR Ctrl + Shift + Q
    if ((isSuper || (isCtrl && isShift)) && !isAlt && key === 'q') {
      return false;
    }

    const copyShortcut = (isMac && event.metaKey && key === 'c') ||
      (!isMac && event.ctrlKey && event.shiftKey && key === 'c') ||
      (event.ctrlKey && key === 'insert');
    const pasteShortcut = (isMac && event.metaKey && key === 'v') ||
      (!isMac && event.ctrlKey && event.shiftKey && key === 'v') ||
      (event.shiftKey && key === 'insert');

    if (copyShortcut && term.hasSelection()) {
      event.preventDefault();
      event.stopPropagation();
      writeClipboardText(term.getSelection()).catch((error) => {
        console.error('Failed to copy terminal selection:', error);
      });
      return false;
    }

    if (pasteShortcut) {
      event.preventDefault();
      event.stopPropagation();
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

  const focusThisTerm = () => {
    try {
      window._mdtermActiveSessionId = sId;
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

  focusThisTerm();
  setTimeout(focusThisTerm, 60);
  setTimeout(focusThisTerm, 150);

  const handlePaneClick = () => {
    const sel = window.getSelection ? window.getSelection().toString() : '';
    if (!sel || sel.length === 0) {
      focusThisTerm();
    }
  };
  container.addEventListener('click', handlePaneClick);

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
          detail: { name, path, content, side, is_remote: isRemote, session_id: sId }
        }));
        return true;
      } else if (action === 'saved') {
        const name = decodeB64(parts[1]) || 'document.md';
        showToast("✓ Saved '" + name + "' on remote server");
        window.dispatchEvent(new CustomEvent('mdterm-file-saved', {
          detail: { name, session_id: sId }
        }));
        return true;
      } else if (action === 'closed') {
        const name = decodeB64(parts[1]) || 'document.md';
        window.dispatchEvent(new CustomEvent('mdterm-remote-closed', {
          detail: { name, session_id: sId }
        }));
        return true;
      } else if (action.startsWith('open-file')) {
        const path = parts.slice(1).join(';');
        const name = path.split('/').pop() || path;
        showToast("Opened '" + name + "' in editor (" + side + ")");
        window.dispatchEvent(new CustomEvent('mdterm-open-file', {
          detail: { name, path, content: '', side, is_remote: false, session_id: sId }
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

    term.parser.registerOscHandler(52, (data) => {
      try {
        const separator = data.indexOf(';');
        if (separator < 0) return false;
        const encodedText = data.slice(separator + 1);
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

    term.parser.registerOscHandler(7, (data) => {
      try {
        if (data.startsWith('file://')) {
          const url = new URL(data);
          sessionObj.cwd = decodeURI(url.pathname);
        }
      } catch (e) {}
      return true;
    });
  }

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
          const startX = match.index + leadingOffset + 1;
          const endX = startX + rawPath.length - 1;

          links.push({
            range: {
              start: { x: startX, y: bufferLineNumber },
              end: { x: endX, y: bufferLineNumber }
            },
            text: rawPath,
            activate: async (event, clickedText) => {
              let cleanPath = clickedText.trim().replace(/^['"`]+|['"`]+$/g, '');
              if (cleanPath.includes(':')) {
                const colonIdx = cleanPath.indexOf(':');
                if (colonIdx > 0) cleanPath = cleanPath.substring(0, colonIdx);
              }

              const filename = cleanPath.split('/').pop() || cleanPath;
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
                let resolvedPath = cleanPath;
                try {
                  if (!cleanPath.startsWith('/') && !cleanPath.startsWith('~')) {
                    let cwd = sessionObj.cwd || '';
                    if (!cwd) {
                      cwd = await getCwd().catch(() => '');
                    }
                    if (cwd) {
                      resolvedPath = cwd.replace(/\/+$/, '') + '/' + cleanPath;
                    }
                  } else if (cleanPath.startsWith('~/')) {
                    const home = await tauriInvokeFn('get_home_dir').catch(() => '');
                    if (home) {
                      resolvedPath = home + cleanPath.slice(1);
                    }
                  }

                  const content = await tauriInvokeFn('read_file', { path: resolvedPath });
                  showToast("Opened '" + filename + "' (" + side + ")");
                  window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                    detail: { name: filename, path: resolvedPath, content, side, session_id: sId }
                  }));
                  return;
                } catch (e) {}

                showToast("Opening '" + filename + "' (" + side + ")...");
                const flag = isSuper ? '--left ' : '';
                Promise.resolve(sendInput('mdterm ' + flag + '"' + cleanPath + '"\n')).catch(() => {});
              } else {
                showToast("Opened '" + filename + "' (" + side + ")");
                const isHtml = /\.(html|htm|xhtml)$/i.test(filename);
                const sampleContent = isHtml
                  ? `<!DOCTYPE html>\n<html lang="en">\n<head>\n  <meta charset="utf-8" />\n  <title>${filename}</title>\n</head>\n<body>\n  <h1>${filename}</h1>\n  <p>Opened via terminal click.</p>\n</body>\n</html>`
                  : '# ' + filename + '\n\nOpened via terminal click.';
                window.dispatchEvent(new CustomEvent('mdterm-open-file', {
                  detail: { name: filename, path: cleanPath, content: sampleContent, side, session_id: sId }
                }));
              }
            }
          });

          fileLinkRegex.lastIndex = match.index + leadingOffset + rawPath.length;
        }

        callback(links);
      }
    });
  }

}

export async function initTerminalSession(containerId, sessionId) {
  const sId = sessionId || '1';
  if (isTmuxSessionId(sId)) {
    const tmuxContainer = document.getElementById(containerId);
    if (tmuxContainer) mountTmuxWindow(tmuxContainer, sId);
    return;
  }
  if (typeof window !== 'undefined') {
    window._mdtermSessions = window._mdtermSessions || {};
    window._mdtermActiveSessionId = sId;
  }

  const container = document.getElementById(containerId);
  if (!container) return;

  if (window._mdtermSessions[sId] && window._mdtermSessions[sId].container === container && window._mdtermSessions[sId].term) {
    const existing = window._mdtermSessions[sId];
    window._mdtermTerminal = existing.term;
    window._mdtermFitAddon = existing.fitAddon;
    if (typeof existing.scheduleResize === 'function') {
      existing.scheduleResize();
    } else if (existing.fitAddon) {
      try { existing.fitAddon.fit(); } catch (e) {}
    }
    return;
  }

  if (window._mdtermSessions[sId]) {
    try {
      const prev = window._mdtermSessions[sId];
      if (typeof prev.unlistenExit === 'function') prev.unlistenExit();
      if (prev.ro) prev.ro.disconnect();
      if (typeof prev.dispose === 'function') {
        prev.dispose();
      } else if (prev.term) {
        prev.term.dispose();
      }
    } catch (e) {}
    delete window._mdtermSessions[sId];
  }
  container.innerHTML = '';

  const tauriInvokeFn = getTauriInvoke();
  const hasTauri = typeof tauriInvokeFn === 'function';

  if (hasTauri && !window._mdtermTerminalConfig) {
    try {
      if (window._mdtermTerminalConfigPromise) {
        const loadedCfg = await window._mdtermTerminalConfigPromise;
        if (loadedCfg) {
          window._mdtermTerminalConfig = loadedCfg;
        }
      } else {
        const loadedCfg = await tauriInvokeFn('get_terminal_config');
        if (loadedCfg) {
          window._mdtermTerminalConfig = loadedCfg;
        }
      }
    } catch (e) {
      console.error('Failed to get terminal config on startup:', e);
    }
  }

  const cfg = window._mdtermTerminalConfig || {};

  const defaultFontFamily = '"JetBrains Mono", Menlo, Monaco, Consolas, "Courier New", monospace';
  const rawFont = cfg.font_family || cfg.fontFamily;
  const fontFamily = rawFont ? normalizeFontFamily(rawFont) : defaultFontFamily;
  document.documentElement.style.setProperty('--font-mono', fontFamily);

  const sessionObj = {
    id: sId,
    container,
    cwd: '',
    ro: null
  };

  const opts = {
    onResize(cols, rows, pixelWidth, pixelHeight) {
      if (hasTauri) {
        // Resizes sent before pty_spawn resolves would hit "No active PTY
        // session" and leave the PTY at a stale size, so chain them on spawn.
        (sessionObj.ptyReady || Promise.resolve()).then(() => tauriInvokeFn('pty_resize', {
          sessionId: sId,
          cols,
          rows,
          pixelWidth,
          pixelHeight
        })).catch(() => {});
      }
    }
  };

  let termResult;
  try {
    termResult = await createTerminal(container, cfg, opts);
  } catch (err) {
    const errDiv = document.createElement('div');
    errDiv.style.color = '#ef4444';
    errDiv.style.padding = '20px';
    errDiv.innerText = err.message || 'Failed to initialize terminal.';
    container.appendChild(errDiv);
    return;
  }

  const { term, fitAddon, dispose, scheduleResize } = termResult;
  sessionObj.term = term;
  sessionObj.fitAddon = fitAddon;
  sessionObj.dispose = dispose;
  sessionObj.scheduleResize = scheduleResize;

  window._mdtermTerminal = term;
  window._mdtermFitAddon = fitAddon;
  window._mdtermSessions[sId] = sessionObj;

  windowShow();

  installTerminalIntegration(term, container, {
    sId,
    sessionObj,
    hasTauri,
    tauriInvokeFn,
    getCwd: () => tauriInvokeFn('pty_get_cwd', { sessionId: sId }),
    sendInput: (data) => tauriInvokeFn('pty_write', { sessionId: sId, data })
  });

  if (hasTauri) {
    const initialCols = term.cols && term.cols > 2 ? term.cols : 80;
    const initialRows = term.rows && term.rows > 1 ? term.rows : 24;

    // Backend Channel contract:
    // invoke('pty_spawn', { sessionId, cols, rows, onOutput }) where onOutput = new window.__TAURI__.core.Channel();
    // its onmessage receives raw bytes (ArrayBuffer; also handle number[]/Uint8Array) -> term.write(new Uint8Array(...)).
    // There are NO more 'pty-output-*' events; the 'pty-exit-{id}' event still exists.
    let onOutput = null;
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.Channel === 'function') {
      onOutput = new window.__TAURI__.core.Channel();
      const utf8Decoder = new TextDecoder('utf-8');
      onOutput.onmessage = (msg) => {
        if (!msg) return;
        let uint8 = null;
        if (msg instanceof ArrayBuffer) {
          uint8 = new Uint8Array(msg);
        } else if (msg instanceof Uint8Array) {
          uint8 = msg;
        } else if (Array.isArray(msg)) {
          uint8 = new Uint8Array(msg);
        } else if (typeof msg === 'string') {
          term.write(msg);
          return;
        } else if (msg.buffer instanceof ArrayBuffer) {
          uint8 = new Uint8Array(msg.buffer, msg.byteOffset, msg.byteLength);
        } else {
          uint8 = new Uint8Array(msg);
        }
        const text = utf8Decoder.decode(uint8, { stream: true });
        term.write(text);
      };
    }

    sessionObj.ptyReady = tauriInvokeFn('pty_spawn', {
      sessionId: sId,
      cols: initialCols,
      rows: initialRows,
      onOutput
    }).catch(err => {
      term.write('\r\n\x1b[31mFailed to spawn shell: ' + err + '\x1b[0m\r\n');
    });
    // The container may have been re-fit between the initial size and spawn.
    sessionObj.ptyReady.then(() => {
      if (term.cols !== initialCols || term.rows !== initialRows) {
        tauriInvokeFn('pty_resize', { sessionId: sId, cols: term.cols, rows: term.rows }).catch(() => {});
      }
    });

    if (window._mdtermTerminalConfig) {
      applyTerminalConfig(window._mdtermTerminalConfig);
    }

    const event = window.__TAURI__ && window.__TAURI__.event;
    if (event && typeof event.listen === 'function') {
      event.listen('pty-exit-' + sId, () => {
        window.dispatchEvent(new CustomEvent('mdterm-pty-exit', { detail: { session_id: sId } }));
      }).then(unlisten => {
        sessionObj.unlistenExit = unlisten;
      }).catch(() => {});
    }

    term.onData(data => {
      tauriInvokeFn('pty_write', { sessionId: sId, data }).catch(() => {});
    });

    term.onBinary(data => {
      const bytes = new Uint8Array(data.length);
      for (let i = 0; i < data.length; i++) {
        bytes[i] = data.charCodeAt(i) & 0xff;
      }
      tauriInvokeFn('pty_write_bytes', { sessionId: sId, data: Array.from(bytes) }).catch(() => {});
    });

    sessionObj.ro = termResult.ro;
  } else {
    term.write('\x1b[1;36m=== mdterm Terminal [' + sId + '] ===\x1b[0m\r\n$ ');
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
            detail: { name: fname, path: fname, content: sampleContent, session_id: sId }
          }));
          term.write('\x1b[32m✓ Opened \'' + fname + '\' in mdterm editor\x1b[0m\r\n');
        } else if (buf.trim() === 'exit') {
          window.dispatchEvent(new CustomEvent('mdterm-pty-exit', { detail: { session_id: sId } }));
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
      scheduleResize();
    });
    ro.observe(container);
    sessionObj.ro = ro;
  }
}

export function setTerminalTheme(themeName) {
  if (typeof window !== 'undefined') {
    window._mdtermCurrentTheme = themeName;
  }
  tmuxSetTheme(themeName);
  let theme;
  if (typeof themeName === 'object' && themeName !== null) {
    theme = Object.assign({}, TERMINAL_THEMES.dark, themeName);
  } else {
    theme = getTerminalTheme(themeName);
  }
  if (typeof window !== 'undefined' && window._mdtermSessions) {
    Object.values(window._mdtermSessions).forEach(sess => {
      if (sess && sess.term) {
        sess.term.options.theme = theme;
        try {
          if (typeof sess.term.refresh === 'function') {
            sess.term.refresh(0, (sess.term.rows || 24) - 1);
          }
        } catch (e) {}
      }
    });
  }
  if (typeof window !== 'undefined' && window._mdtermTerminal) {
    window._mdtermTerminal.options.theme = theme;
  }
  if (typeof document !== 'undefined') {
    document.querySelectorAll('.terminal-container').forEach(c => {
      if (c && theme && theme.background) {
        c.style.backgroundColor = theme.background;
      }
    });
  }
  try {
    if (typeof window !== 'undefined' && typeof window.renderMermaidDiagrams === 'function') {
      window.renderMermaidDiagrams();
    }
  } catch (e) {}
}

export function applyTerminalConfig(cfg) {
  if (!cfg) return;
  if (typeof window !== 'undefined') {
    window._mdtermTerminalConfig = cfg;
  }
  if (cfg.theme) {
    setTerminalTheme(cfg.theme);
  }
  tmuxApplyConfig();
  const rawFont = cfg.font_family || cfg.fontFamily;
  const normalizedFont = rawFont ? normalizeFontFamily(rawFont) : null;
  if (normalizedFont && typeof document !== 'undefined') {
    document.documentElement.style.setProperty('--font-mono', normalizedFont);
  }
  if (typeof window !== 'undefined' && window._mdtermSessions) {
    Object.values(window._mdtermSessions).forEach(sess => {
      if (!sess || !sess.term) return;
      let fontChanged = false;
      if (normalizedFont && sess.term.options.fontFamily !== normalizedFont) {
        sess.term.options.fontFamily = normalizedFont;
        fontChanged = true;
      }
      if (cfg.font_size || cfg.fontSize) {
        const parsed = Number(cfg.font_size || cfg.fontSize);
        if (!isNaN(parsed) && parsed > 0 && sess.term.options.fontSize !== parsed) {
          sess.term.options.fontSize = parsed;
          fontChanged = true;
        }
      }
      const rawHeight = cfg.character_height || cfg.characterHeight || cfg.line_height || cfg.lineHeight;
      if (rawHeight !== undefined && rawHeight !== null) {
        const currentFontSize = sess.term.options.fontSize || 13;
        const lh = computeLineHeight(rawHeight, currentFontSize);
        if (sess.term.options.lineHeight !== lh) {
          sess.term.options.lineHeight = lh;
          fontChanged = true;
        }
      }
      if (fontChanged && typeof sess.term.refresh === 'function') {
        try {
          sess.term.refresh(0, (sess.term.rows || 24) - 1);
        } catch (e) {}
      }
      if (typeof sess.scheduleResize === 'function') {
        sess.scheduleResize();
      } else if (sess.fitAddon) {
        try { sess.fitAddon.fit(); } catch (e) {}
      }
    });
  }
}

export function clearTerminalSession(sessionId) {
  const sId = sessionId || (typeof window !== 'undefined' && window._mdtermActiveSessionId);
  const sess = (sId && typeof window !== 'undefined' && window._mdtermSessions && window._mdtermSessions[sId])
    ? window._mdtermSessions[sId]
    : null;
  const term = sess ? sess.term : (typeof window !== 'undefined' && window._mdtermTerminal);
  if (term) {
    term.clear();
  }
}

export function fitTerminalSession(sessionId) {
  const sId = sessionId || (typeof window !== 'undefined' && window._mdtermActiveSessionId);
  if (isTmuxSessionId(sId)) {
    fitTmuxWindow(sId);
    return;
  }
  const sess = (sId && typeof window !== 'undefined' && window._mdtermSessions && window._mdtermSessions[sId])
    ? window._mdtermSessions[sId]
    : null;
  if (sess && typeof sess.scheduleResize === 'function') {
    sess.scheduleResize();
  } else if (typeof window !== 'undefined' && window._mdtermActiveSessionId && window._mdtermSessions && window._mdtermSessions[window._mdtermActiveSessionId]) {
    const active = window._mdtermSessions[window._mdtermActiveSessionId];
    if (active && typeof active.scheduleResize === 'function') {
      active.scheduleResize();
    }
  }
}

export function focusTerminalSession(sessionId) {
  const sId = sessionId || (typeof window !== 'undefined' && window._mdtermActiveSessionId);
  if (sId && typeof window !== 'undefined') {
    window._mdtermActiveSessionId = sId;
  }
  if (isTmuxSessionId(sId)) {
    focusTmuxWindow(sId);
    return;
  }
  const sess = (sId && typeof window !== 'undefined' && window._mdtermSessions && window._mdtermSessions[sId])
    ? window._mdtermSessions[sId]
    : null;
  const term = sess ? sess.term : (typeof window !== 'undefined' && window._mdtermTerminal);
  if (term) {
    try {
      if (typeof window !== 'undefined' && typeof window.focus === 'function') {
        window.focus();
      }
      term.focus();
      const container = sess ? sess.container : document.querySelector('.terminal-container');
      if (container) {
        const textarea = container.querySelector('.xterm-helper-textarea');
        if (textarea && document.activeElement !== textarea) {
          textarea.focus({ preventScroll: true });
        }
      }
    } catch (e) {}
  }
}

export function closeTerminalSession(sessionId) {
  const sId = sessionId || (typeof window !== 'undefined' && window._mdtermActiveSessionId);
  if (!sId) return;
  if (isTmuxSessionId(sId)) {
    unmountTmuxWindow(sId);
    return;
  }
  if (typeof window !== 'undefined' && window._mdtermSessions && window._mdtermSessions[sId]) {
    const sess = window._mdtermSessions[sId];
    try {
      if (typeof sess.unlistenExit === 'function') sess.unlistenExit();
      if (sess.ro) sess.ro.disconnect();
      if (typeof sess.dispose === 'function') {
        sess.dispose();
      } else if (sess.term) {
        sess.term.dispose();
      }
    } catch (e) {}
    delete window._mdtermSessions[sId];
  }
  const tauriInvokeFn = getTauriInvoke();
  if (tauriInvokeFn) {
    tauriInvokeFn('pty_close', { sessionId: sId }).catch(() => {});
  }
}

// 8. Expose window.__mdtermDebug = { dump(sessionId) }
if (typeof window !== 'undefined') {
  window.__mdtermDebug = {
    dump(sessionId) {
      const sId = sessionId || window._mdtermActiveSessionId || '1';
      const sess = (window._mdtermSessions && window._mdtermSessions[sId]) ? window._mdtermSessions[sId] : null;
      const term = sess ? sess.term : window._mdtermTerminal;
      if (!term || !term.buffer || !term.buffer.active) {
        return null;
      }
      const buf = term.buffer.active;
      const lines = [];
      for (let i = 0; i < buf.length; i++) {
        const line = buf.getLine(i);
        lines.push(line ? line.translateToString(true) : '');
      }
      return {
        cols: term.cols,
        rows: term.rows,
        cursor: {
          x: buf.cursorX,
          y: buf.cursorY
        },
        alt: buf.type === 'alternate',
        lines
      };
    }
  };
}
