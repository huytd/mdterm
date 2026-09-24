/**
 * tmux-client.js
 * Native UI for tmux control mode: every tmux pane is its own xterm.js, every
 * tmux window is an mdterm tab (session id `tmux-<conn>-<window>`).
 *
 * The Rust backend (src-tauri/src/tmux) forwards one ordered channel per
 * connection: `[1][pane u32 LE][bytes]` for pane output and `[2][json]` for
 * notifications and command replies. Replies are handled synchronously inside
 * the channel callback so that a pane's seed (from capture-pane) and the
 * output that follows it are applied in exactly the order tmux produced them.
 */

import { createTerminal, installTerminalIntegration, buildTerminalOptions, getTerminalTheme } from './terminal-core.js';
import { keyEventToTmux, parseListKeys, createKeyRouter } from './tmux-keys.js';
import { showPrompt, showConfirm, showOverlayText, setPrefixBadge, showPaneBadge, showPaneModeOverlay } from './tmux-ui.js';

const SESSION_PREFIX = 'tmux-';
const DEFAULT_SCROLLBACK = 2000;
const SEP = '\t';

const PANE_FIELDS = [
  'pane_id', 'window_id', 'pane_active', 'cursor_x', 'cursor_y', 'alternate_on',
  'cursor_flag', 'keypad_cursor_flag', 'keypad_flag', 'mouse_any_flag',
  'mouse_button_flag', 'mouse_standard_flag', 'mouse_sgr_flag', 'mouse_utf8_flag',
  'insert_flag', 'wrap_flag', 'origin_flag', 'scroll_region_upper',
  'scroll_region_lower', 'pane_width', 'pane_height', 'pane_current_path', 'pane_title',
  'pane_current_command'
];
const WINDOW_FIELDS = [
  'window_id', 'window_index', 'window_active', 'window_layout',
  'window_visible_layout', 'window_flags', 'window_name'
];

function formatOf(fields) {
  return "'" + fields.map(f => '#{' + f + '}').join(SEP) + "'";
}

/** Splits a format line; the last field keeps any separators it contains. */
function parseRow(line, fields) {
  const parts = line.split(SEP);
  const row = {};
  fields.forEach((f, i) => {
    row[f] = i === fields.length - 1 ? parts.slice(i).join(SEP) : (parts[i] || '');
  });
  return row;
}

const idNum = (s) => Number(String(s).replace(/^[%@$]/, ''));

/** Quotes a value as one tmux command argument (double quotes; `\`, `"`, `$` escaped). */
export function tmuxQuote(value) {
  return '"' + String(value).replace(/[\\"$]/g, (c) => '\\' + c) + '"';
}

// ---------------------------------------------------------------------------
// Layout strings: "8205,80x24,0,0{40x24,0,0,0,39x24,41,0,1}"
// ---------------------------------------------------------------------------

export function parseLayout(str) {
  if (!str) return null;
  let s = String(str);
  if (/^[0-9a-f]{4},/i.test(s)) s = s.slice(5);
  let i = 0;
  const num = () => {
    const m = /^\d+/.exec(s.slice(i));
    if (!m) throw new Error('layout: number expected at ' + i);
    i += m[0].length;
    return Number(m[0]);
  };
  const eat = (c) => {
    if (s[i] !== c) throw new Error('layout: expected ' + c + ' at ' + i);
    i++;
  };
  const node = () => {
    const w = num(); eat('x'); const h = num(); eat(','); const x = num(); eat(','); const y = num();
    const n = { x, y, w, h };
    if (s[i] === ',') {
      i++;
      n.pane = num();
    } else if (s[i] === '{' || s[i] === '[') {
      const close = s[i] === '{' ? '}' : ']';
      n.dir = s[i] === '{' ? 'horizontal' : 'vertical';
      i++;
      n.children = [];
      for (;;) {
        n.children.push(node());
        if (s[i] === ',') { i++; continue; }
        eat(close);
        break;
      }
    }
    return n;
  };
  try {
    const root = node();
    return i === s.length ? root : null;
  } catch (e) {
    return null;
  }
}

export function layoutPanes(node, out = []) {
  if (!node) return out;
  if (node.pane !== undefined) out.push(node);
  (node.children || []).forEach(c => layoutPanes(c, out));
  return out;
}

function firstPane(node) {
  return layoutPanes(node)[0];
}

// ---------------------------------------------------------------------------
// Pane seeding: turn capture-pane output + pane flags into terminal bytes.
// ---------------------------------------------------------------------------

export function buildSeed(meta, captured, normalScreen) {
  const h = Number(meta.pane_height) || 0;
  const alt = meta.alternate_on === '1';
  const lines = captured.slice();
  const visible = lines.slice(Math.max(0, lines.length - h));
  const history = lines.slice(0, Math.max(0, lines.length - h));
  const row = (l) => '\x1b[0m' + l;
  let out = '';
  if (!alt) {
    out += history.concat(visible).map(row).join('\r\n');
  } else {
    const normal = normalScreen.slice(0, h);
    out += history.concat(normal).map(row).join('\r\n');
    out += '\x1b[?1049h\x1b[H\x1b[2J';
    out += visible.map(row).join('\r\n');
  }
  out += '\x1b[0m';
  const upper = Number(meta.scroll_region_upper) || 0;
  const lower = Number(meta.scroll_region_lower);
  if (upper !== 0 || (h > 0 && !isNaN(lower) && lower !== h - 1)) {
    out += `\x1b[${upper + 1};${lower + 1}r`;
  }
  if (meta.origin_flag === '1') out += '\x1b[?6h';
  out += `\x1b[${(Number(meta.cursor_y) || 0) + 1};${(Number(meta.cursor_x) || 0) + 1}H`;
  if (meta.cursor_flag === '0') out += '\x1b[?25l';
  if (meta.keypad_cursor_flag === '1') out += '\x1b[?1h';
  if (meta.keypad_flag === '1') out += '\x1b=';
  if (meta.insert_flag === '1') out += '\x1b[4h';
  if (meta.wrap_flag === '0') out += '\x1b[?7l';
  if (meta.mouse_standard_flag === '1') out += '\x1b[?1000h';
  if (meta.mouse_button_flag === '1') out += '\x1b[?1002h';
  if (meta.mouse_any_flag === '1') out += '\x1b[?1003h';
  if (meta.mouse_utf8_flag === '1') out += '\x1b[?1005h';
  if (meta.mouse_sgr_flag === '1') out += '\x1b[?1006h';
  return out;
}

// ---------------------------------------------------------------------------

function invoke(cmd, args) {
  return window.__TAURI__.core.invoke(cmd, args);
}

function dispatch(name, detail) {
  window.dispatchEvent(new CustomEvent(name, { detail }));
}

function toast(msg) {
  if (typeof window.showToast === 'function') window.showToast(msg);
}

export function isTmuxSessionId(sId) {
  return typeof sId === 'string' && sId.startsWith(SESSION_PREFIX);
}

export function tmuxSessionId(connId, windowId) {
  return `${SESSION_PREFIX}${connId}-${windowId}`;
}

function parseSessionId(sId) {
  const m = /^tmux-(\d+)-(\d+)$/.exec(sId || '');
  return m ? { connId: Number(m[1]), windowId: Number(m[2]) } : null;
}

const connections = new Map();

/**
 * An xterm opened inside a hidden tab measures a zero-size character cell and
 * renders nothing; re-assigning the font forces a new measurement.
 */
function revalidateTerm(term) {
  if (!term) return;
  const valid = term._core?._charSizeService?.hasValidSize;
  if (valid === false) {
    const font = term.options.fontFamily;
    term.options.fontFamily = '';
    term.options.fontFamily = font;
  }
  try { term.refresh(0, term.rows - 1); } catch (e) {}
}

const utf8 = new TextEncoder();

function currentConfig() {
  return window._mdtermTerminalConfig || {};
}

class PaneView {
  constructor(conn, id) {
    this.conn = conn;
    this.id = id;
    this.el = document.createElement('div');
    this.el.className = 'tmux-pane';
    this.el.dataset.pane = String(id);
    this.term = null;
    this.dispose = null;
    this.decoder = new TextDecoder('utf-8');
    this.live = false;
    this.seeding = false;
    this.creating = null;
    this.cols = 0;
    this.rows = 0;
    this.cwd = '';
    this.command = '';
    this.sessionObj = { id: '', container: this.el, cwd: '' };
    this.tmuxMode = null;
    this.modeOverlay = null;
    this.scrollbackBadge = null;
    this.scrollbackListener = null;
    this.gripEl = null;

    this.el.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      if (e.altKey && e.shiftKey) {
        const win = this.conn.windows.get(this.windowId);
        if (!win) return;
        const z = (win.flags || '').includes('Z');
        const l = layoutPanes(win.visible || win.layout);
        if (l.length <= 1 || z) return;
        e.preventDefault();
        e.stopPropagation();
        this.conn.startPaneDrag(e, this, win);
      }
    }, true);

    this.el.addEventListener('mousedown', (e) => {
      if (e.button === 0 && e.altKey && e.shiftKey) {
        e.preventDefault();
        e.stopPropagation();
      }
    }, true);

    this.el.addEventListener('mousedown', () => conn.selectPane(this.id));
  }

  async ensureTerm() {
    if (this.term) return;
    if (this.creating) return this.creating;
    this.creating = (async () => {
      const cfg = currentConfig();
      const res = await createTerminal(this.el, cfg, { observe: false });
      if (this.disposed) {
        res.dispose();
        return;
      }
      this.term = res.term;
      this.dispose = res.dispose;
      if (res.fitAddon) {
        try { res.fitAddon.dispose(); } catch (e) {}
      }
      if (this.cols > 0 && this.rows > 0) this.term.resize(this.cols, this.rows);
      const sId = this.conn.sessionIdForPane(this.id);
      this.sessionObj.id = sId;
      installTerminalIntegration(this.term, this.el, {
        sId,
        sessionObj: this.sessionObj,
        hasTauri: true,
        tauriInvokeFn: invoke,
        getCwd: () => this.conn.paneCwd(this.id),
        sendInput: (data) => this.conn.sendKeys(this.id, utf8.encode(data)),
        handleKey: (ev) => this.conn.handleKey(this.id, ev)
      });
      this.term.onData((data) => this.conn.sendKeys(this.id, utf8.encode(data)));
      this.term.onBinary((data) => {
        const bytes = new Uint8Array(data.length);
        for (let i = 0; i < data.length; i++) bytes[i] = data.charCodeAt(i) & 0xff;
        this.conn.sendKeys(this.id, bytes);
      });
      this.term.textarea && this.term.textarea.addEventListener('focus', () => this.conn.selectPane(this.id));
      this.conn.seedPane(this);
    })();
    return this.creating;
  }

  setSize(cols, rows) {
    if (cols === this.cols && rows === this.rows) return;
    this.cols = cols;
    this.rows = rows;
    if (this.term && cols > 0 && rows > 0) {
      try { this.term.resize(cols, rows); } catch (e) {}
      // xterm.js and tmux rewrap lines differently on resize; tmux's grid is
      // authoritative, so reload the pane from it once resizing settles.
      if (this.live || this.seeding) this.scheduleReseed();
    }
  }

  scheduleReseed() {
    if (this.reseedTimer) clearTimeout(this.reseedTimer);
    this.reseedTimer = setTimeout(() => {
      this.reseedTimer = null;
      this.conn.seedPane(this);
    }, 80);
  }

  write(bytes) {
    if (!this.live || !this.term) return;
    this.term.write(this.decoder.decode(bytes, { stream: true }));
  }

  clearTmuxMode() {
    this.tmuxMode = null;
    if (this.modeOverlay) {
      try { this.modeOverlay.close(); } catch (e) {}
      this.modeOverlay = null;
    }
  }

  clearScrollbackBadge() {
    if (this.scrollbackBadge) {
      try { this.scrollbackBadge.remove(); } catch (e) {}
      this.scrollbackBadge = null;
    }
    if (this.scrollbackListener) {
      try { this.scrollbackListener.dispose(); } catch (e) {}
      this.scrollbackListener = null;
    }
  }

  destroy() {
    this.disposed = true;
    this.clearTmuxMode();
    this.clearScrollbackBadge();
    if (this.reseedTimer) clearTimeout(this.reseedTimer);
    if (this.dispose) {
      try { this.dispose(); } catch (e) {}
    }
    this.term = null;
    this.el.remove();
  }
}

class TmuxConnection {
  constructor(connId, origin, session) {
    this.id = connId;
    this.origin = origin;
    this.sessionName = session || '';
    this.tagSeq = 1;
    this.replyHandlers = new Map();
    this.windows = new Map();
    this.panes = new Map();
    this.mounts = new Map();
    this.cell = null;
    this.clientSize = null;
    this.activeWindow = null;
    this.closed = false;
    this.scrollback = Number((currentConfig().tmux || {}).scrollback) || DEFAULT_SCROLLBACK;
    this.resizeTimer = null;
    this.newWindowBroken = false;
    this.router = null;
    this.activePrompt = null;
    this.activeConfirm = null;
    this.activeOverlay = null;
    this.refreshRouterTimer = null;
    this.prefixBadgeTimer = null;
  }

  // ---- transport -------------------------------------------------------

  async start() {
    const ch = new window.__TAURI__.core.Channel();
    ch.onmessage = (msg) => this.onFrame(msg);
    const info = await invoke('tmux_subscribe', { connId: this.id, onEvent: ch });
    if (info && info.session) this.sessionName = info.session;
    this.noteOrigin(
      '\r\n\x1b[1;36m[mdterm] tmux control mode active — windows open as tabs.\x1b[0m\r\n' +
      '\x1b[2mPress Esc or q here to detach.\x1b[0m\r\n'
    );
    await this.measureCell();
    this.syncClientSize(this.measureVisibleArea());
    this.run(['refresh-client -f pause-after=5']);
    this.run(["display -p '#{version}'"], ([r]) => {
      this.newWindowBroken = r.ok && /^3\.5/.test(r.lines[0] || '');
    });
    this.refreshWindows(true);
    this.loadKeyTables();
  }

  loadKeyTables() {
    this.run(
      ['show -gv prefix', 'show -gv prefix2', 'show -gv repeat-time', 'list-keys'],
      ([rPrefix, rPrefix2, rRepeat, rKeys]) => {
        if (!rPrefix || !rKeys || !rPrefix.ok || !rKeys.ok) return;
        let prefix = (rPrefix.lines[0] || '').trim();
        if (!prefix || prefix === 'None') prefix = 'C-b';
        let prefix2 = rPrefix2 && rPrefix2.ok ? (rPrefix2.lines[0] || '').trim() : null;
        if (!prefix2 || prefix2 === 'None') prefix2 = null;
        const repeatTime = Number((rRepeat && rRepeat.lines[0] ? rRepeat.lines[0] : '').trim()) || 500;
        const { tables } = parseListKeys(rKeys.lines);
        this.router = createKeyRouter({
          tables,
          prefix,
          prefix2,
          repeatTime,
          now: () => Date.now()
        });
        this.repeatTime = repeatTime;
      }
    );
  }

  scheduleKeyTablesRefresh() {
    if (this.refreshRouterTimer) clearTimeout(this.refreshRouterTimer);
    this.refreshRouterTimer = setTimeout(() => {
      this.loadKeyTables();
    }, 500);
  }

  noteOrigin(text) {
    if (!this.origin) return;
    const sess = window._mdtermSessions && window._mdtermSessions[this.origin];
    if (sess && sess.term) sess.term.write(text);
  }

  onFrame(msg) {
    let bytes;
    if (msg instanceof ArrayBuffer) bytes = new Uint8Array(msg);
    else if (msg instanceof Uint8Array) bytes = msg;
    else if (Array.isArray(msg)) bytes = new Uint8Array(msg);
    else return;
    if (bytes.length === 0) return;
    if (bytes[0] === 1) {
      const pane = bytes[1] | (bytes[2] << 8) | (bytes[3] << 16) | (bytes[4] << 24);
      const view = this.panes.get(pane >>> 0);
      if (view) view.write(bytes.subarray(5));
      return;
    }
    if (bytes[0] === 2) {
      let ev;
      try {
        ev = JSON.parse(new TextDecoder().decode(bytes.subarray(1)));
      } catch (e) {
        return;
      }
      this.onEvent(ev);
    }
  }

  /**
   * Sends commands on one line (atomic w.r.t. pane output). `onDone` receives
   * one `{ok, lines}` per command and runs synchronously in the channel callback.
   */
  run(commands, onDone) {
    if (this.closed) return;
    const tags = commands.map(() => (onDone ? this.tagSeq++ : 0));
    if (onDone) {
      const results = new Array(commands.length);
      let remaining = commands.length;
      tags.forEach((tag, i) => {
        this.replyHandlers.set(tag, (r) => {
          results[i] = r;
          if (--remaining === 0) onDone(results);
        });
      });
    }
    invoke('tmux_command', { connId: this.id, commands, tags }).catch((e) => {
      console.error('tmux command failed:', commands, e);
      tags.forEach(t => this.replyHandlers.delete(t));
    });
  }

  query(command) {
    return new Promise((resolve, reject) => {
      this.run([command], ([r]) => (r.ok ? resolve(r.lines) : reject(new Error(r.lines.join('\n')))));
    });
  }

  sendKeys(pane, bytes) {
    if (this.closed || !bytes || bytes.length === 0) return Promise.resolve();
    return invoke('tmux_send_keys', { connId: this.id, pane, data: Array.from(bytes) }).catch(() => {});
  }

  detach() {
    if (this.activePaneDrag) {
      this.activePaneDrag.cancel();
    }
    return invoke('tmux_detach', { connId: this.id }).catch(() => {});
  }

  // ---- notifications ---------------------------------------------------

  onEvent(ev) {
    switch (ev.t) {
      case 'reply': {
        const h = this.replyHandlers.get(ev.tag);
        if (h) {
          this.replyHandlers.delete(ev.tag);
          h({ ok: ev.ok, lines: ev.lines || [] });
        }
        break;
      }
      case 'layout': {
        const w = this.windows.get(ev.window);
        if (w) {
          if (this.activePaneDrag && this.activePaneDrag.windowId === ev.window) {
            this.activePaneDrag.cancel();
          }
          w.layout = parseLayout(ev.layout);
          w.visible = parseLayout(ev.visible) || w.layout;
          w.flags = ev.flags || '';
          this.renderWindow(w);
        }
        break;
      }
      case 'window-add':
        this.addWindowById(ev.window);
        break;
      case 'window-close':
        this.removeWindow(ev.window);
        break;
      case 'window-renamed': {
        const w = this.windows.get(ev.window);
        if (w) {
          w.name = ev.name;
          dispatch('mdterm-tmux-window-renamed', { session_id: tmuxSessionId(this.id, w.id), name: ev.name });
        }
        break;
      }
      case 'window-pane-changed': {
        const w = this.windows.get(ev.window);
        if (w) {
          w.activePane = ev.pane;
          this.renderWindow(w);
          if (this.isTabActive(w.id)) this.focusPane(ev.pane);
        }
        break;
      }
      case 'session-window-changed':
        if (this.windows.has(ev.window) && this.activeWindow !== ev.window) {
          this.activeWindow = ev.window;
          dispatch('mdterm-tmux-window-select', { session_id: tmuxSessionId(this.id, ev.window) });
        }
        break;
      case 'pane-mode-changed': {
        const view = this.panes.get(ev.pane);
        if (view) {
          this.run([`display -p -t %${ev.pane} '#{pane_mode}'`], ([r]) => {
            if (r && r.ok) {
              const mode = (r.lines[0] || '').trim();
              if (mode) {
                view.tmuxMode = mode;
                if (!view.modeOverlay) {
                  view.modeOverlay = showPaneModeOverlay({ paneEl: view.el, mode });
                }
              } else {
                view.clearTmuxMode();
              }
            }
          });
        }
        break;
      }
      case 'session-changed':
        this.sessionName = ev.name;
        for (const id of Array.from(this.windows.keys())) this.removeWindow(id);
        this.refreshWindows(true);
        this.loadKeyTables();
        break;
      case 'session-renamed':
        this.sessionName = ev.name;
        break;
      case 'exit':
        this.shutdown(ev.reason);
        break;
      default:
        break;
    }
  }

  // ---- windows ---------------------------------------------------------

  refreshWindows(initial) {
    this.run([`list-windows -F ${formatOf(WINDOW_FIELDS)}`], ([r]) => {
      if (!r.ok) return;
      const rows = r.lines.filter(Boolean).map(l => parseRow(l, WINDOW_FIELDS));
      rows.sort((a, b) => Number(a.window_index) - Number(b.window_index));
      for (const row of rows) this.upsertWindow(row, initial && row.window_active === '1');
    });
  }

  addWindowById(windowId) {
    if (this.windows.has(windowId)) return;
    this.run([`display -p -t @${windowId} ${formatOf(WINDOW_FIELDS)}`], ([r]) => {
      if (!r.ok || !r.lines[0]) return;
      // %session-window-changed for a new window arrives before this reply,
      // so the query's own window_active decides whether its tab takes focus.
      const row = parseRow(r.lines[0], WINDOW_FIELDS);
      this.upsertWindow(row, row.window_active === '1');
    });
  }

  upsertWindow(row, activate) {
    const id = idNum(row.window_id);
    let w = this.windows.get(id);
    const isNew = !w;
    if (isNew) {
      w = { id, index: 0, name: '', layout: null, visible: null, flags: '', activePane: null };
      this.windows.set(id, w);
    }
    w.index = Number(row.window_index);
    w.name = row.window_name;
    w.layout = parseLayout(row.window_layout);
    w.visible = parseLayout(row.window_visible_layout) || w.layout;
    w.flags = row.window_flags;
    if (row.window_active === '1') this.activeWindow = id;
    if (isNew) {
      dispatch('mdterm-tmux-window-add', {
        conn_id: this.id,
        session_id: tmuxSessionId(this.id, id),
        name: w.name,
        index: w.index,
        activate: !!activate,
        origin: this.origin || null
      });
    }
    this.refreshActivePane(w);
    this.renderWindow(w);
  }

  refreshActivePane(w) {
    this.run([`display -p -t @${w.id} '#{pane_id}'`], ([r]) => {
      if (r.ok && r.lines[0]) {
        w.activePane = idNum(r.lines[0]);
        this.renderWindow(w);
      }
    });
  }

  removeWindow(windowId) {
    const w = this.windows.get(windowId);
    if (!w) return;
    this.windows.delete(windowId);
    const stillUsed = new Set();
    for (const other of this.windows.values()) layoutPanes(other.layout).forEach(p => stillUsed.add(p.pane));
    for (const [pid, view] of this.panes) {
      if (!stillUsed.has(pid) && view.windowId === windowId) {
        view.destroy();
        this.panes.delete(pid);
      }
    }
    dispatch('mdterm-tmux-window-close', { session_id: tmuxSessionId(this.id, windowId) });
  }

  // ---- mounting & layout ------------------------------------------------

  mount(container, windowId) {
    let el = container.querySelector('.tmux-window');
    if (!el) {
      container.innerHTML = '';
      container.style.position = 'relative';
      el = document.createElement('div');
      el.className = 'tmux-window';
      container.appendChild(el);
    }
    const prev = this.mounts.get(windowId);
    if (prev && prev.ro) prev.ro.disconnect();
    const ro = new ResizeObserver(() => this.scheduleClientResize());
    ro.observe(el);
    this.mounts.set(windowId, { el, ro });
    const w = this.windows.get(windowId);
    if (w) this.renderWindow(w);
    this.scheduleClientResize();
  }

  unmount(windowId) {
    if (this.activePaneDrag && this.activePaneDrag.windowId === windowId) {
      this.activePaneDrag.cancel();
    }
    const m = this.mounts.get(windowId);
    if (m) {
      if (m.ro) m.ro.disconnect();
      m.el.remove();
      this.mounts.delete(windowId);
    }
  }

  renderWindow(w) {
    const mount = this.mounts.get(w.id);
    if (!mount || !this.cell) return;
    const root = w.visible || w.layout;
    if (!root) return;
    const { width: cw, height: ch } = this.cell;
    const el = mount.el;
    const leaves = layoutPanes(root);
    const multi = leaves.length > 1;
    const zoomed = (w.flags || '').includes('Z');
    const canDrag = multi && !zoomed;
    const seen = new Set();
    for (const leaf of leaves) {
      seen.add(leaf.pane);
      let view = this.panes.get(leaf.pane);
      if (!view) {
        view = new PaneView(this, leaf.pane);
        this.panes.set(leaf.pane, view);
      }
      view.windowId = w.id;
      if (view.el.parentElement !== el) el.appendChild(view.el);
      Object.assign(view.el.style, {
        left: `${leaf.x * cw}px`,
        top: `${leaf.y * ch}px`,
        width: `${leaf.w * cw}px`,
        height: `${leaf.h * ch}px`,
        display: ''
      });
      view.el.classList.toggle('tmux-pane-active', multi && leaf.pane === w.activePane);
      view.el.classList.toggle('tmux-pane-inactive', multi && leaf.pane !== w.activePane);
      view.setSize(leaf.w, leaf.h);
      view.ensureTerm();

      if (!view.gripEl) {
        const grip = document.createElement('div');
        grip.className = 'tmux-pane-grip';
        grip.textContent = '⠿';
        grip.title = 'Drag to swap or move pane';
        grip.addEventListener('pointerdown', (e) => {
          if (e.button !== 0) return;
          const win = this.windows.get(view.windowId);
          if (!win) return;
          const z = (win.flags || '').includes('Z');
          const l = layoutPanes(win.visible || win.layout);
          if (l.length <= 1 || z) return;
          e.preventDefault();
          e.stopPropagation();
          this.startPaneDrag(e, view, win);
        });
        grip.addEventListener('mousedown', (e) => {
          if (e.button === 0) {
            e.preventDefault();
            e.stopPropagation();
          }
        });
        view.gripEl = grip;
        view.el.appendChild(grip);
      }
      view.gripEl.style.display = canDrag ? '' : 'none';
    }
    // Panes of this window hidden by zoom stay alive but invisible.
    for (const view of this.panes.values()) {
      if (view.windowId === w.id && !seen.has(view.id)) {
        view.el.style.display = 'none';
        if (view.gripEl) view.gripEl.style.display = 'none';
      }
    }
    el.querySelectorAll('.tmux-divider').forEach(d => d.remove());
    if (!this.activePaneDrag || this.activePaneDrag.windowId !== w.id) {
      el.querySelectorAll('.tmux-drop-indicator, .tmux-drag-ghost').forEach(x => x.remove());
    }
    if (multi) this.renderDividers(el, root, cw, ch);
  }

  renderDividers(el, node, cw, ch) {
    if (!node.children) return;
    node.children.forEach((child, i) => {
      if (i < node.children.length - 1) {
        const d = document.createElement('div');
        const horizontal = node.dir === 'horizontal';
        d.className = 'tmux-divider ' + (horizontal ? 'tmux-divider-v' : 'tmux-divider-h');
        if (horizontal) {
          Object.assign(d.style, {
            left: `${(child.x + child.w) * cw}px`, top: `${node.y * ch}px`,
            width: `${cw}px`, height: `${node.h * ch}px`
          });
        } else {
          Object.assign(d.style, {
            left: `${node.x * cw}px`, top: `${(child.y + child.h) * ch}px`,
            width: `${node.w * cw}px`, height: `${ch}px`
          });
        }
        d.addEventListener('mousedown', (e) => this.startDividerDrag(e, el, child, horizontal));
        el.appendChild(d);
      }
      this.renderDividers(el, child, cw, ch);
    });
  }

  startDividerDrag(e, el, child, horizontal) {
    e.preventDefault();
    e.stopPropagation();
    const target = firstPane(child);
    if (!target) return;
    const rect = el.getBoundingClientRect();
    const { width: cw, height: ch } = this.cell;
    let last = null;
    let raf = null;
    document.body.classList.add(horizontal ? 'tmux-dragging-v' : 'tmux-dragging-h');
    const onMove = (ev) => {
      const size = horizontal
        ? Math.round((ev.clientX - rect.left) / cw) - child.x
        : Math.round((ev.clientY - rect.top) / ch) - child.y;
      const clamped = Math.max(1, size);
      if (clamped === last) return;
      last = clamped;
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = null;
        this.run([`resize-pane -t %${target.pane} ${horizontal ? '-x' : '-y'} ${last}`]);
      });
    };
    const onUp = () => {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.classList.remove('tmux-dragging-v', 'tmux-dragging-h');
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }

  startPaneDrag(e, sourceView, w) {
    const mount = this.mounts.get(w.id);
    if (!mount || !this.cell) return;
    const leaves = layoutPanes(w.visible || w.layout);
    if (leaves.length <= 1 || (w.flags || '').includes('Z')) return;

    const mountRect = mount.el.getBoundingClientRect();
    const { width: cw, height: ch } = this.cell;
    const sourceLeaf = leaves.find(l => l.pane === sourceView.id);
    const sourceW = sourceLeaf ? sourceLeaf.w * cw : sourceView.el.offsetWidth;
    const sourceH = sourceLeaf ? sourceLeaf.h * ch : sourceView.el.offsetHeight;

    document.body.classList.add('tmux-dragging-pane');
    sourceView.el.classList.add('tmux-pane-drag-source');

    const ghost = document.createElement('div');
    ghost.className = 'tmux-drag-ghost';
    const gw = Math.max(60, Math.round(sourceW * 0.4));
    const gh = Math.max(30, Math.round(sourceH * 0.4));
    ghost.style.width = `${gw}px`;
    ghost.style.height = `${gh}px`;
    ghost.textContent = sourceView.command || 'pane';
    ghost.style.left = `${e.clientX - mountRect.left}px`;
    ghost.style.top = `${e.clientY - mountRect.top}px`;
    mount.el.appendChild(ghost);

    let ended = false;
    if (!sourceView.command) {
      this.run([`display -p -t %${sourceView.id} '#{pane_current_command}'`], ([r]) => {
        if (r && r.ok && r.lines[0]) {
          sourceView.command = r.lines[0];
          if (!ended) ghost.textContent = r.lines[0];
        }
      });
    }

    const indicator = document.createElement('div');
    indicator.className = 'tmux-drop-indicator';
    indicator.style.display = 'none';
    const dropLabel = document.createElement('div');
    dropLabel.className = 'tmux-drop-label';
    indicator.appendChild(dropLabel);
    mount.el.appendChild(indicator);

    let currentDrop = null;

    const captureTarget = e.target;
    if (captureTarget && captureTarget.setPointerCapture && e.pointerId !== undefined) {
      try { captureTarget.setPointerCapture(e.pointerId); } catch (_) {}
    }

    const updateZone = (clientX, clientY) => {
      const relX = clientX - mountRect.left;
      const relY = clientY - mountRect.top;
      ghost.style.left = `${relX}px`;
      ghost.style.top = `${relY}px`;

      const targetLeaf = leaves.find(l => {
        const lx = l.x * cw, ly = l.y * ch;
        const lw = l.w * cw, lh = l.h * ch;
        return relX >= lx && relX < lx + lw && relY >= ly && relY < ly + lh;
      });

      if (!targetLeaf || targetLeaf.pane === sourceView.id) {
        currentDrop = null;
        indicator.style.display = 'none';
        return;
      }

      const lx = targetLeaf.x * cw;
      const ly = targetLeaf.y * ch;
      const lw = targetLeaf.w * cw;
      const lh = targetLeaf.h * ch;
      const px = (relX - lx) / lw;
      const py = (relY - ly) / lh;

      const inCentre = px >= 0.25 && px <= 0.75 && py >= 0.25 && py <= 0.75;
      if (inCentre) {
        currentDrop = { targetPaneId: targetLeaf.pane, type: 'swap' };
        indicator.style.display = 'flex';
        indicator.style.left = `${lx}px`;
        indicator.style.top = `${ly}px`;
        indicator.style.width = `${lw}px`;
        indicator.style.height = `${lh}px`;
        dropLabel.textContent = 'Swap';
      } else {
        const dLeft = px;
        const dRight = 1 - px;
        const dTop = py;
        const dBottom = 1 - py;
        const min = Math.min(dLeft, dRight, dTop, dBottom);

        if (min === dLeft) {
          currentDrop = { targetPaneId: targetLeaf.pane, type: 'move', edge: 'left' };
          indicator.style.display = 'flex';
          indicator.style.left = `${lx}px`;
          indicator.style.top = `${ly}px`;
          indicator.style.width = `${Math.round(lw / 2)}px`;
          indicator.style.height = `${lh}px`;
          dropLabel.textContent = 'Move left';
        } else if (min === dRight) {
          const halfW = Math.round(lw / 2);
          currentDrop = { targetPaneId: targetLeaf.pane, type: 'move', edge: 'right' };
          indicator.style.display = 'flex';
          indicator.style.left = `${lx + halfW}px`;
          indicator.style.top = `${ly}px`;
          indicator.style.width = `${lw - halfW}px`;
          indicator.style.height = `${lh}px`;
          dropLabel.textContent = 'Move right';
        } else if (min === dTop) {
          currentDrop = { targetPaneId: targetLeaf.pane, type: 'move', edge: 'top' };
          indicator.style.display = 'flex';
          indicator.style.left = `${lx}px`;
          indicator.style.top = `${ly}px`;
          indicator.style.width = `${lw}px`;
          indicator.style.height = `${Math.round(lh / 2)}px`;
          dropLabel.textContent = 'Move up';
        } else {
          const halfH = Math.round(lh / 2);
          currentDrop = { targetPaneId: targetLeaf.pane, type: 'move', edge: 'bottom' };
          indicator.style.display = 'flex';
          indicator.style.left = `${lx}px`;
          indicator.style.top = `${ly + halfH}px`;
          indicator.style.width = `${lw}px`;
          indicator.style.height = `${lh - halfH}px`;
          dropLabel.textContent = 'Move down';
        }
      }
    };

    updateZone(e.clientX, e.clientY);

    const cleanup = () => {
      if (ended) return;
      ended = true;
      document.body.classList.remove('tmux-dragging-pane');
      sourceView.el.classList.remove('tmux-pane-drag-source');
      ghost.remove();
      indicator.remove();
      if (captureTarget && captureTarget.releasePointerCapture && e.pointerId !== undefined) {
        try { captureTarget.releasePointerCapture(e.pointerId); } catch (_) {}
      }
      window.removeEventListener('pointermove', onPointerMove, true);
      window.removeEventListener('pointerup', onPointerUp, true);
      window.removeEventListener('pointercancel', onPointerCancel, true);
      document.removeEventListener('keydown', onKeyDown, true);
      window.removeEventListener('blur', onBlur, true);
      if (this.activePaneDrag?.cancel === cancelDrag) {
        this.activePaneDrag = null;
      }
    };

    const cancelDrag = () => {
      cleanup();
    };

    const onPointerMove = (ev) => {
      if (ended) return;
      updateZone(ev.clientX, ev.clientY);
    };

    const onPointerUp = (ev) => {
      if (ended) return;
      const drop = currentDrop;
      cleanup();
      if (!drop) return;

      const sourceId = sourceView.id;
      const targetId = drop.targetPaneId;
      if (sourceId === targetId) return;

      let cmds = [];
      if (drop.type === 'swap') {
        cmds = [`swap-pane -s %${sourceId} -t %${targetId}`, `select-pane -t %${sourceId}`];
      } else if (drop.type === 'move') {
        let flag = '';
        if (drop.edge === 'right') flag = '-h';
        else if (drop.edge === 'left') flag = '-h -b';
        else if (drop.edge === 'bottom') flag = '-v';
        else if (drop.edge === 'top') flag = '-v -b';
        cmds = [`move-pane -s %${sourceId} -t %${targetId} ${flag}`, `select-pane -t %${sourceId}`];
      }

      if (cmds.length > 0) {
        mount.el.classList.add('tmux-animate');
        setTimeout(() => { mount.el.classList.remove('tmux-animate'); }, 200);
        this.run(cmds, (replies) => {
          for (const r of replies) this.showReply(r);
        });
      }
    };

    const onPointerCancel = () => {
      cancelDrag();
    };

    const onKeyDown = (ev) => {
      if (ev.key === 'Escape') {
        ev.preventDefault();
        ev.stopPropagation();
        cancelDrag();
      }
    };

    const onBlur = () => {
      cancelDrag();
    };

    window.addEventListener('pointermove', onPointerMove, true);
    window.addEventListener('pointerup', onPointerUp, true);
    window.addEventListener('pointercancel', onPointerCancel, true);
    document.addEventListener('keydown', onKeyDown, true);
    window.addEventListener('blur', onBlur, true);

    this.activePaneDrag = { windowId: w.id, cancel: cancelDrag };
  }

  // ---- sizing ----------------------------------------------------------

  async measureCell() {
    const probe = document.createElement('div');
    probe.className = 'tmux-measure';
    document.body.appendChild(probe);
    try {
      const res = await createTerminal(probe, currentConfig(), { observe: false });
      const cell = res.term._core?._renderService?.dimensions?.css?.cell;
      if (cell && cell.width > 0 && cell.height > 0) {
        this.cell = { width: cell.width, height: cell.height };
      }
      res.dispose();
    } catch (e) {
      console.error('tmux: failed to measure cell size', e);
    }
    probe.remove();
    if (!this.cell) {
      const fs = Number(currentConfig().font_size) || 13;
      this.cell = { width: Math.ceil(fs * 0.6), height: Math.ceil(fs * 1.2) };
    }
  }

  measureVisibleArea() {
    for (const m of this.mounts.values()) {
      const r = m.el.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) return r;
    }
    const fallback = document.querySelector('.tab-workspace-active .terminal-pane-container');
    if (fallback) {
      const r = fallback.getBoundingClientRect();
      // Same insets as .tmux-window.
      return { width: r.width - 12, height: r.height - 8 };
    }
    return { width: window.innerWidth, height: window.innerHeight };
  }

  scheduleClientResize() {
    if (this.resizeTimer) clearTimeout(this.resizeTimer);
    this.resizeTimer = setTimeout(() => {
      this.resizeTimer = null;
      this.syncClientSize(this.measureVisibleArea());
    }, 40);
  }

  syncClientSize(rect) {
    if (!this.cell || !rect || rect.width <= 0 || rect.height <= 0) return;
    const cols = Math.max(2, Math.floor(rect.width / this.cell.width));
    const rows = Math.max(2, Math.floor(rect.height / this.cell.height));
    if (this.clientSize && this.clientSize.cols === cols && this.clientSize.rows === rows) return;
    this.clientSize = { cols, rows };
    this.run([`refresh-client -C ${cols}x${rows}`]);
  }

  async applyConfig() {
    await this.measureCell();
    const cfg = currentConfig();
    const opts = buildTerminalOptions(cfg);
    for (const view of this.panes.values()) {
      if (!view.term) continue;
      view.term.options.fontFamily = opts.fontFamily;
      view.term.options.fontSize = opts.fontSize;
      view.term.options.lineHeight = opts.lineHeight;
    }
    this.clientSize = null;
    for (const w of this.windows.values()) this.renderWindow(w);
    this.scheduleClientResize();
    const tmuxCfg = cfg.tmux || {};
    if (tmuxCfg.prefix_emulation !== false && tmuxCfg.prefixEmulation !== false) {
      this.loadKeyTables();
    } else {
      this.router = null;
      this.setPrefixIndicatorState(null);
    }
  }

  setTheme(theme) {
    for (const view of this.panes.values()) {
      if (view.term) view.term.options.theme = theme;
    }
  }

  // ---- panes -----------------------------------------------------------

  seedPane(view) {
    if (view.seeding) {
      view.reseedAfter = true;
      return;
    }
    view.seeding = true;
    view.reseedAfter = false;
    view.live = false;
    const t = `%${view.id}`;
    this.run([
      `display -p -t ${t} ${formatOf(PANE_FIELDS)}`,
      `capture-pane -p -e -N -t ${t} -S -${this.scrollback}`,
      `capture-pane -p -e -N -a -q -t ${t}`
    ], ([meta, screen, normal]) => {
      view.seeding = false;
      if (!view.term) return;
      if (view.reseedAfter) {
        // The pane changed size while this capture was in flight.
        this.seedPane(view);
        return;
      }
      if (!meta.ok || !screen.ok) {
        view.live = true;
        return;
      }
      const m = parseRow(meta.lines[0] || '', PANE_FIELDS);
      view.cwd = m.pane_current_path;
      view.command = m.pane_current_command || '';
      view.sessionObj.cwd = m.pane_current_path;
      const w = Number(m.pane_width), h = Number(m.pane_height);
      if (w > 0 && h > 0) view.setSize(w, h);
      view.term.reset();
      view.decoder = new TextDecoder('utf-8');
      view.term.write(buildSeed(m, screen.lines, normal.ok ? normal.lines : []));
      view.live = true;
    });
  }

  paneCwd(pane) {
    return this.query(`display -p -t %${pane} '#{pane_current_path}'`).then(lines => lines[0] || '');
  }

  selectPane(pane) {
    const view = this.panes.get(pane);
    if (!view) return;
    const w = this.windows.get(view.windowId);
    if (w && w.activePane !== pane) {
      // Only a pane switch cancels a pending prefix; focus moves that just
      // follow tmux (e.g. a late %window-pane-changed) must not eat the key.
      this.resetPrefix();
      w.activePane = pane;
      this.renderWindow(w);
      this.run([`select-pane -t %${pane}`]);
    }
    window._mdtermActiveSessionId = tmuxSessionId(this.id, view.windowId);
  }

  focusPane(pane) {
    const view = this.panes.get(pane);
    if (view && view.term) {
      try { view.term.focus(); } catch (e) {}
    }
  }

  sessionIdForPane(pane) {
    const view = this.panes.get(pane);
    return tmuxSessionId(this.id, view ? view.windowId : 0);
  }

  isTabActive(windowId) {
    const m = this.mounts.get(windowId);
    return !!(m && m.el.offsetParent !== null);
  }

  focusWindow(windowId) {
    const w = this.windows.get(windowId);
    if (!w) return;
    if (this.activeWindow !== windowId) {
      this.resetPrefix();
      this.activeWindow = windowId;
      this.run([`select-window -t @${windowId}`]);
    }
    this.scheduleClientResize();
    for (const view of this.panes.values()) {
      if (view.windowId === windowId) revalidateTerm(view.term);
    }
    const pane = w.activePane ?? (firstPane(w.visible || w.layout) || {}).pane;
    if (pane !== undefined) {
      this.focusPane(pane);
      const view = this.panes.get(pane);
      if (view && !view.term) view.ensureTerm().then(() => this.focusPane(pane));
    }
  }

  activePaneOf(windowId) {
    const w = this.windows.get(windowId);
    return w ? w.activePane : null;
  }

  // ---- actions ---------------------------------------------------------

  action(windowId, name) {
    const pane = this.activePaneOf(windowId);
    const t = pane !== null && pane !== undefined ? `-t %${pane}` : `-t @${windowId}`;
    const cwd = pane !== null && pane !== undefined ? ` -c '#{pane_current_path}'` : '';
    switch (name) {
      case 'split-right': this.run([`split-window -h ${t}${cwd}`]); break;
      case 'split-down': this.run([`split-window -v ${t}${cwd}`]); break;
      case 'new-window':
        // new-window through control mode crashes tmux 3.5a; split + break instead.
        this.run(this.newWindowBroken
          ? [`split-window ${t}${cwd}`, `break-pane -a`]
          : [`new-window -a -t @${windowId}${cwd}`]);
        break;
      case 'kill-pane': this.run([`kill-pane ${t}`]); break;
      case 'kill-window': this.run([`kill-window -t @${windowId}`]); break;
      case 'zoom': this.run([`resize-pane -Z ${t}`]); break;
      case 'pane-left': this.run([`select-pane -L ${t}`]); break;
      case 'pane-right': this.run([`select-pane -R ${t}`]); break;
      case 'pane-up': this.run([`select-pane -U ${t}`]); break;
      case 'pane-down': this.run([`select-pane -D ${t}`]); break;
      case 'pane-next': this.run([`select-pane -t @${windowId}.+`]); break;
      case 'pane-prev': this.run([`select-pane -t @${windowId}.-`]); break;
      case 'detach': this.detach(); break;
      default: break;
    }
  }

  /** Pane-level shortcuts and tmux prefix key routing. Returns true if handled. */
  handleKey(pane, ev) {
    if (ev.type !== 'keydown') return false;
    const view = this.panes.get(pane);
    if (!view) return false;

    // Safety net: pane in a tmux mode (e.g. external copy-mode)
    if (view.tmuxMode) {
      if (ev.key === 'Escape') {
        this.run([`send-keys -X -t %${pane} cancel`]);
        view.clearTmuxMode();
      }
      return true; // Swallow all keys while in tmux mode
    }

    const tmuxCfg = currentConfig().tmux || {};
    const prefixEmulation = tmuxCfg.prefix_emulation !== false && tmuxCfg.prefixEmulation !== false;

    if (prefixEmulation && this.router) {
      const name = keyEventToTmux(ev);
      if (name) {
        const r = this.router.handle(name);
        this.updatePrefixIndicator(view);
        if (r.consume) {
          this.applyRouted(view, r);
          return true;
        }
      }
    }

    const act = tmuxShortcut(ev);
    if (!act) return false;
    this.action(view.windowId, act);
    return true;
  }

  /** Focuses whichever pane tmux has active in the window (after an overlay closes). */
  focusActiveIn(windowId) {
    const w = this.windows.get(windowId);
    if (!w) return;
    const pane = w.activePane ?? (firstPane(w.visible || w.layout) || {}).pane;
    if (pane !== undefined) this.focusPane(pane);
  }

  resetPrefix() {
    if (this.router) this.router.reset();
    this.setPrefixIndicatorState(null);
  }

  updatePrefixIndicator(view) {
    if (!this.router) {
      this.setPrefixIndicatorState(null);
      return;
    }
    const s = this.router.state();
    if (s.table !== 'root') {
      let text = s.table === 'prefix' ? 'PREFIX' : s.table.toUpperCase();
      if (s.inRepeat) text += ' REPEAT';
      this.setPrefixIndicatorState(text, view);
      if (this.prefixBadgeTimer) clearTimeout(this.prefixBadgeTimer);
      const delay = (this.repeatTime || 500) + 50;
      this.prefixBadgeTimer = setTimeout(() => {
        this.updatePrefixIndicator(view);
      }, delay);
    } else {
      if (this.prefixBadgeTimer) clearTimeout(this.prefixBadgeTimer);
      this.setPrefixIndicatorState(null);
    }
  }

  setPrefixIndicatorState(text, activeView) {
    const key = text ? `${text}|${activeView ? activeView.id : ''}` : null;
    if (key === this.prefixIndicatorKey) return;
    this.prefixIndicatorKey = key;
    for (const mount of this.mounts.values()) {
      if (text) mount.el.classList.add('tmux-prefix-active');
      else mount.el.classList.remove('tmux-prefix-active');
    }
    for (const view of this.panes.values()) {
      if (text && activeView && view.id === activeView.id) {
        setPrefixBadge(view.el, text);
      } else {
        setPrefixBadge(view.el, null);
      }
    }
  }

  applyRouted(view, r) {
    this.scheduleKeyTablesRefresh();
    if (r.run) {
      const cmd = r.run.trim();
      // Workaround for tmux 3.5a crash on bound new-window:
      if (this.newWindowBroken && /^(?:new-window|neww)(?:\s+-[ac](?:\s+\S+)?)*$/.test(cmd)) {
        const t = view.id !== null && view.id !== undefined ? `-t %${view.id}` : '';
        const cwd = view.id !== null && view.id !== undefined ? ` -c '#{pane_current_path}'` : '';
        this.run([`split-window ${t}${cwd}`, 'break-pane -a']);
      } else {
        this.run([cmd], ([res]) => this.showReply(res));
      }
      return;
    }

    if (r.native) {
      this.handleNativeUI(view, r.native);
    }
  }

  handleNativeUI(view, native) {
    const mount = this.mounts.get(view.windowId);
    if (!mount) return;

    switch (native.name) {
      case 'copy-mode': {
        if (view.term) {
          view.term.focus();
          if (native.scrollUp) {
            view.term.scrollPages(-1);
          }
          view.clearScrollbackBadge();
          view.scrollbackBadge = showPaneBadge(view.el, 'SCROLLBACK');
          view.scrollbackListener = view.term.onScroll(() => {
            const buf = view.term.buffer.active;
            if (buf.viewportY >= buf.baseY) {
              view.clearScrollbackBadge();
            }
          });
        }
        break;
      }
      case 'command-prompt': {
        const paneId = view.id;
        const initialToExpand = native.initial || '';
        const promptToExpand = native.prompt || ':';

        const doShow = (initialVal, promptVal) => {
          if (this.activePrompt) {
            try { this.activePrompt.close(); } catch (e) {}
            this.activePrompt = null;
          }
          this.activePrompt = showPrompt({
            mount: mount.el,
            prompt: promptVal,
            initial: initialVal,
            onSubmit: (val, { close, setError }) => {
              if (native.template) {
                let cmd = native.template;
                if (cmd.startsWith('{') && cmd.endsWith('}')) {
                  cmd = cmd.slice(1, -1).trim();
                }
                const escaped = tmuxQuote(val).slice(1, -1);
                cmd = cmd.replace(/%%%/g, tmuxQuote(val)).replace(/%%/g, escaped).replace(/%1/g, escaped);
                this.run([cmd], ([res]) => {
                  if (res && !res.ok) {
                    setError(res.lines[0] || 'Error');
                  } else {
                    close();
                    this.activePrompt = null;
                    this.focusActiveIn(view.windowId);
                    if (res) this.showReply(res);
                  }
                });
              } else {
                this.run([val], ([res]) => {
                  if (res && !res.ok) {
                    setError(res.lines[0] || 'Error');
                  } else {
                    close();
                    this.activePrompt = null;
                    this.focusActiveIn(view.windowId);
                    if (res) this.showReply(res);
                  }
                });
              }
            },
            onCancel: () => {
              this.activePrompt = null;
              this.focusActiveIn(view.windowId);
            }
          });
        };

        if (initialToExpand.includes('#') || promptToExpand.includes('#')) {
          this.run([
            `display -p -t %${paneId} ${tmuxQuote(initialToExpand)}`,
            `display -p -t %${paneId} ${tmuxQuote(promptToExpand)}`
          ], ([rInit, rPrompt]) => {
            const initVal = rInit && rInit.ok ? rInit.lines[0] : initialToExpand;
            const pVal = rPrompt && rPrompt.ok ? rPrompt.lines[0] : promptToExpand;
            doShow(initVal, pVal);
          });
        } else {
          doShow(initialToExpand, promptToExpand);
        }
        break;
      }
      case 'confirm-before': {
        const paneId = view.id;
        const promptToExpand = native.prompt || `Confirm '${native.cmd}'? (y/n)`;

        const doConfirm = (text) => {
          if (this.activeConfirm) {
            try { this.activeConfirm.close(); } catch (e) {}
            this.activeConfirm = null;
          }
          this.activeConfirm = showConfirm({
            mount: mount.el,
            text,
            onYes: () => {
              this.activeConfirm = null;
              this.run([native.cmd], ([res]) => this.showReply(res));
              this.focusActiveIn(view.windowId);
            },
            onNo: () => {
              this.activeConfirm = null;
              this.focusActiveIn(view.windowId);
            }
          });
        };

        if (promptToExpand.includes('#')) {
          this.run([`display -p -t %${paneId} ${tmuxQuote(promptToExpand)}`], ([rPrompt]) => {
            const text = rPrompt && rPrompt.ok ? rPrompt.lines[0] : promptToExpand;
            doConfirm(text);
          });
        } else {
          doConfirm(promptToExpand);
        }
        break;
      }
      case 'detach-client': {
        this.detach();
        break;
      }
      case 'unsupported': {
        toast(`${native.command || 'Command'} isn't available in mdterm's tmux mode`);
        break;
      }
      default:
        break;
    }
  }

  showReply(res) {
    if (!res) return;
    if (!res.ok) {
      toast(res.lines[0] || 'Command failed');
      return;
    }
    const lines = res.lines || [];
    for (const line of lines) {
      if (line.startsWith('%message ')) {
        toast(line.slice(9));
      }
    }
    const printLines = lines.filter(l => !l.startsWith('%message ') && l.trim().length > 0);
    if (printLines.length > 0) {
      const w = this.windows.get(this.activeWindow);
      const mount = w ? this.mounts.get(w.id) : null;
      if (mount) {
        showOverlayText({
          mount: mount.el,
          lines: printLines,
          onClose: () => {
            if (w) this.focusActiveIn(w.id);
          }
        });
      }
    }
  }


  shutdown(reason) {
    if (this.closed) return;
    this.closed = true;
    if (this.refreshRouterTimer) clearTimeout(this.refreshRouterTimer);
    if (this.prefixBadgeTimer) clearTimeout(this.prefixBadgeTimer);
    if (this.activePrompt) {
      try { this.activePrompt.close(); } catch (e) {}
      this.activePrompt = null;
    }
    if (this.activeConfirm) {
      try { this.activeConfirm.close(); } catch (e) {}
      this.activeConfirm = null;
    }
    for (const id of Array.from(this.windows.keys())) this.removeWindow(id);
    for (const view of this.panes.values()) view.destroy();
    this.panes.clear();
    for (const id of Array.from(this.mounts.keys())) this.unmount(id);
    connections.delete(this.id);
    this.noteOrigin('\r\n\x1b[1;36m[mdterm] tmux detached' + (reason ? ` (${reason})` : '') + '.\x1b[0m\r\n');
    dispatch('mdterm-tmux-exit', { conn_id: this.id, origin: this.origin || null });
    toast('tmux ' + (reason ? reason : 'detached'));
  }
}

/** Maps a keydown to a tmux action name, or null. */
export function tmuxShortcut(ev) {
  const key = (ev.key || '').toLowerCase();
  const isSuper = !!(ev.metaKey || (typeof ev.getModifierState === 'function' &&
    (ev.getModifierState('Meta') || ev.getModifierState('Super') || ev.getModifierState('OS'))));
  const ctrlShift = ev.ctrlKey && ev.shiftKey;
  const mod = isSuper || ctrlShift;
  if (!mod) return null;
  if (key === 'd') {
    if (ev.altKey) return 'detach';
    return isSuper && ev.shiftKey ? 'split-down' : 'split-right';
  }
  if (key === 'e' && ctrlShift && !isSuper) return 'split-down';
  if (key === 'enter') return 'zoom';
  if (isSuper && !ev.altKey && key === '[') return 'pane-prev';
  if (isSuper && !ev.altKey && key === ']') return 'pane-next';
  if ((isSuper && ev.altKey) || ctrlShift) {
    if (key === 'arrowleft') return 'pane-left';
    if (key === 'arrowright') return 'pane-right';
    if (key === 'arrowup') return 'pane-up';
    if (key === 'arrowdown') return 'pane-down';
  }
  return null;
}

// ---------------------------------------------------------------------------
// Entry points used by terminal-core.js and the Leptos bridge
// ---------------------------------------------------------------------------

function connFor(sId) {
  const p = parseSessionId(sId);
  if (!p) return null;
  const conn = connections.get(p.connId);
  return conn ? { conn, windowId: p.windowId } : null;
}

export function mountTmuxWindow(container, sId) {
  const r = connFor(sId);
  if (r) r.conn.mount(container, r.windowId);
}

export function focusTmuxWindow(sId) {
  const r = connFor(sId);
  if (r) {
    window._mdtermActiveSessionId = sId;
    r.conn.focusWindow(r.windowId);
  }
}

export function fitTmuxWindow(sId) {
  const r = connFor(sId);
  if (r) r.conn.scheduleClientResize();
}

export function unmountTmuxWindow(sId) {
  const r = connFor(sId);
  if (r) r.conn.unmount(r.windowId);
}

export function tmuxAction(sId, name) {
  const r = connFor(sId);
  if (r) r.conn.action(r.windowId, name);
}

export function tmuxApplyConfig() {
  for (const conn of connections.values()) conn.applyConfig();
}

export function tmuxSetTheme(themeName) {
  const theme = typeof themeName === 'object' && themeName !== null
    ? themeName
    : getTerminalTheme(themeName);
  for (const conn of connections.values()) conn.setTheme(theme);
}

export async function tmuxDetect() {
  if (!window.__TAURI__) return null;
  try {
    return await invoke('tmux_detect');
  } catch (e) {
    return null;
  }
}

export async function tmuxAttach(session, create) {
  return invoke('tmux_attach', { session: session || null, create: !!create });
}

function onConnection(payload) {
  if (!payload || connections.has(payload.conn_id)) return;
  const conn = new TmuxConnection(payload.conn_id, payload.origin, payload.session);
  connections.set(payload.conn_id, conn);
  conn.start().catch((e) => {
    console.error('tmux: failed to start connection', e);
    toast('tmux: ' + (e && e.message ? e.message : e));
  });
}

if (typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.event) {
  window.__TAURI__.event.listen('tmux-connection', (e) => onConnection(e.payload)).catch(() => {});
  window.__mdtermTmux = { connections, parseLayout, buildSeed };
}
