# tmux Integration (Control Mode) — Research & Plan

Goal: when tmux is in play, mdterm stops being "a terminal that tmux draws into" and becomes
**a native UI for tmux** — tmux panes are real mdterm panes (one xterm.js each), tmux windows are
tabs, splits/resizes/focus are mouse- and shortcut-driven, and scrollback/selection are native.
Inspired by [tmuxy](https://github.com/flplima/tmuxy) and iTerm2's `tmux -CC` integration.

## Status (implemented)

Decisions taken: tmux windows → mdterm tabs; launch behaviour `ask`; native shortcuts first
(prefix emulation deferred); tmux ≥ 3.2 required, 3.4+ recommended.

| Area | Where | State |
|---|---|---|
| Control-mode parser (octal, blocks, notifications, `%exit` + `ESC \` hand-back) | `src-tauri/src/tmux/parser.rs` | done |
| Connections, reply tags, flow control auto-continue, spawned + in-band transports | `src-tauri/src/tmux/mod.rs` | done |
| In-band `tmux -CC` detection in the PTY reader (works over ssh), input guard | `src-tauri/src/lib.rs`, `InbandTracker` | done |
| `tmux:` config block | `src-tauri/src/config.rs` | done |
| Per-pane xterm.js, layout rendering, dividers, seeding (history, alt screen, modes, cursor) | `frontend/js/tmux-client.js` | done |
| Re-seed from `capture-pane` after a pane resize (xterm/tmux rewrap differ) | `tmux-client.js` | done |
| Windows ↔ tabs, launch prompt, shortcuts, `TMUX` tab badge | `frontend/src/app.rs`, `components/tmux_prompt.rs`, `titlebar.rs` | done |
| Shared terminal integration (clipboard, OSC 5337/52/7, links) for PTY tabs and panes | `terminal-core.js: installTerminalIntegration` | done |
| `pty_get_cwd` no longer shells out to tmux while a control client is attached | `lib.rs` | done |
| 3.5a `new-window` crash workaround (`split-window ; break-pane`) | `tmux-client.js` | done (untested on 3.5a) |
| Prefix-key emulation, hidden-window output suppression, session switcher, drag-to-rearrange | — | not started |

Notes from implementation:

* `kill-window` is reported as `%unlinked-window-close`, not `%window-close`; both are handled.
* `%session-window-changed` for a new window arrives before the window's metadata can be
  queried, so a new window's tab activation uses the query's own `window_active`.
* xterm.js already recovers mdterm's OSC 5337 from tmux's `DCS tmux;` passthrough wrapper, so no
  extra DCS handler was needed.
* Replies are applied synchronously inside the channel callback so the capture-pane seed and
  following `%output` keep tmux's order.
* As built, the backend is thinner than §2 sketches: Rust has `parser.rs` + `mod.rs` (transport,
  reply tags, flow control); layout parsing and the session/window/pane model live in
  `tmux-client.js`, which consumes raw layout strings.

---

## 1. Research findings

### 1.1 How tmuxy works

```
tmux server ──(tmux -CC, one connection per session)──> tmuxy-core (Rust)
    parser → StateAggregator (sans-IO) → vendored vt100 grid per pane
      → JSON state snapshots/deltas (~60fps throttle) → SSE / Tauri IPC → React + XState renders cells
```

Lessons worth copying:

| Lesson | Detail |
|---|---|
| **Everything through the control connection** | Running external `tmux …` subprocesses while a control client is attached has crashed tmux 3.3a/3.5a. All mutations *and* reads go in-band (`%begin/%end` replies). |
| **`neww` crash on 3.5a** | tmuxy rewrites `new-window` → `splitw ; breakp`. We must test this on 3.5a and keep the workaround behind a version check. |
| **Stable IDs only** | Target `%N` panes / `@N` windows / `$N` sessions, never indices (they shift, `renumber-windows` doesn't notify). |
| **Initial content** | `list-panes` (cursor, modes) *before* `capture-pane -e` per pane; output arriving during the capture has to be ordered correctly. |
| **Flow control** | `refresh-client -f pause-after=N`; on `%pause %N` reply `refresh-client -A '%N:continue'`. |
| **Sizing** | Client size set via `refresh-client -C WxH` / `resizew`; pane rects come back in `%layout-change`. |
| **`pane_in_mode` ≠ copy mode** | Use `#{==:#{pane_mode},copy-mode}`; 3.4 starts some panes in view-mode. |
| **Minimum version** | tmuxy requires 3.4 (Ubuntu 24.04 LTS). |

Where we should **diverge** from tmuxy: tmuxy parses pane bytes with a server-side vt100 and ships
cell grids to React. mdterm already has a tuned xterm.js pipeline (render-accuracy fixes F1–F7,
replay harness, WebGL/DOM renderers). We feed each pane's raw `%output` bytes straight into
**its own xterm.js instance** (the iTerm2 model). Far less code, full VT fidelity, and the
existing test harness keeps working.

### 1.2 Verified on local tmux 3.6 (isolated socket)

- `tmux -C attach` works over **plain pipes** (no PTY needed) → easy local spawn.
- `tmux -CC` inside a PTY emits `ESC P 1000 p` first and `%exit\r\n ESC \` last; lines are CRLF.
  → We can detect a user typing `tmux -CC attach` in a normal mdterm shell — **including over SSH** —
  and switch that tab into tmux mode (iTerm2 behaviour, zero remote install).
- `%output %N <data>`: raw application bytes; only bytes < 0x20 and `\` are octal-escaped, UTF-8
  passes through raw. A notification can split a UTF-8 sequence → decode per pane with a streaming decoder.
- App escape sequences arrive verbatim, e.g. mdterm's CLI output shows up as
  `\033Ptmux;\033\033]5337;open;…\007\033\134` (the DCS passthrough wrapper is **not** stripped).
- `send-keys -H -t %N 68 69 1b 5b 41 0d` delivers exact bytes (keys, mouse reports, paste).
- `%layout-change @0 8205,80x24,0,0{40x24,0,0,0,39x24,41,0,1} … *` — layout tree with pane ids.
- `refresh-client -C 120x40` resizes and triggers `%layout-change`.
- `capture-pane -p -e -t %N -S -N` and `display -p -t %N '#{cursor_x},…'` answer in-band.

### 1.3 Current mdterm pieces this touches

| Piece | Where | Notes |
|---|---|---|
| PTY spawn/IO | `src-tauri/src/lib.rs` (`pty_spawn`, `pty_write*`, `pty_resize`) | One PTY per workspace tab, `Channel<InvokeResponseBody::Raw>` for output |
| Output pump | `src-tauri/src/pty_stream.rs` (`pump`, `SessionRecorder`) | Byte-exact, 4ms coalescing — reuse for the control stream |
| xterm.js lifecycle | `frontend/js/terminal-core.js` (`createTerminal`, `initTerminalSession`) | Assumes 1 tab = 1 PTY = 1 xterm, FitAddon drives size |
| OSC hooks | `terminal-core.js:899` (5337/7777/1337/52/7) | Must also unwrap `DCS tmux;` when fed from control mode |
| Tab model | `frontend/src/state.rs` (`WorkspaceTab`), `app.rs` | Tab has one `session_id` + editor split |
| CWD lookup | `lib.rs:pty_get_cwd` | Shells out to `tmux display-message` — unsafe under control mode & picks an arbitrary client |
| CLI | `bin/mdterm` | Wraps OSC 5337 in DCS passthrough when `$TMUX` set |

---

## 2. Architecture

```
                 ┌──────────────── Transport (trait ControlTransport) ────────────────┐
                 │ A. Spawned:   tmux -C attach|new -A -s <name>   (pipes, local)       │
                 │ B. In-band:   existing PTY tab, after ESC P1000p is seen (local/SSH)  │
                 └───────────────┬───────────────────────────────▲─────────────────────┘
                   bytes (lines) │                               │ command lines
┌────────────────────────────────▼───────────────────────────────┴────────────────────┐
│ src-tauri/src/tmux/                                                                   │
│  parser.rs   sans-IO byte-line parser → Notification enum (octal decode, %begin/%end) │
│  layout.rs   layout string → tree { Split(H|V, children) | Pane(%id, x,y,w,h) }       │
│  client.rs   command queue: FIFO correlation of %begin/%end/%error → oneshot replies  │
│  model.rs    sessions / windows / panes state; diff → structural events               │
│  mod.rs      TmuxState (managed), Tauri commands                                      │
└───────────┬───────────────────────────────────────────────────┬─────────────────────┘
  per-pane Channel<Raw> (%output bytes)             structural events (JSON: layout,
            │                                        windows, active pane, titles, exit)
┌───────────▼───────────────────────────────────────────────────▼─────────────────────┐
│ frontend/js/tmux-client.js   one xterm per pane (reusing createTerminal, no FitAddon │
│                              sizing), layout renderer, key/mouse → send-keys -H      │
│ frontend/src/components/tmux_workspace.rs   Leptos view: window tabs, dividers       │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Backend (Rust)

**Parser** (`tmux/parser.rs`) — pure, byte-based (never lossy UTF-8), unit-testable:
`%begin/%end/%error` blocks, `%output`, `%extended-output`, `%layout-change`, `%window-add`,
`%window-close`, `%unlinked-window-*`, `%window-renamed`, `%window-pane-changed`,
`%session-changed`, `%session-renamed`, `%session-window-changed`, `%sessions-changed`,
`%pane-mode-changed`, `%pause`, `%continue`, `%client-detached`, `%exit`, `%subscription-changed`.
Handles CRLF (in-band) and LF (pipes). Unknown notifications are ignored, not fatal.

**Command client** (`tmux/client.rs`) — tmux answers commands strictly in order, so a FIFO of
pending `oneshot::Sender<Result<Vec<String>,String>>` is enough. Keystroke `send-keys` are
fire-and-forget but still consume a slot. Multiple commands can be pipelined.

**Model** (`tmux/model.rs`) — `Session { id, name, windows }`, `Window { id, name, index, layout,
zoomed, active_pane }`, `Pane { id, window, rect, title, cwd, cmd, alternate_on, mode }`.
Refreshed by notifications + in-band `list-windows -F` / `list-panes -s -F` with one
tab-separated format string (constant + lockstep test, as tmuxy does).

**Attach sequence**
1. `refresh-client -C <cols>x<rows>` (frontend's measured size in cells)
2. `refresh-client -f pause-after=5` (3.2+)
3. `list-windows -F …`, `list-panes -s -F …` (cursor, `alternate_on`, `mouse_*_flag`,
   `keypad_flag`, `cursor_flag`, `insert_flag`, `wrap_flag`, `pane_current_path`, title)
4. Per pane: `capture-pane -p -e -N -t %N -S -<scrollback>` (plus `-a` for alternate screen if active)
5. Emit window/pane structure, then seed each xterm: history → screen → CUP to cursor → restore
   modes (DECCKM, mouse tracking, bracketed paste, cursor visibility) from the flags.

Ordering rule for 4: `%output %N` that arrives *before* the capture's `%begin` was already in the
grid when capture ran → drop it; everything after `%end` is applied. Buffer per pane while a
capture is in flight.

**Tauri commands** (new):
`tmux_detect() -> {installed, version, sessions[]}`, `tmux_attach({session?, create})`,
`tmux_command(cmd) -> Vec<String>`, `tmux_send(pane, bytes)`, `tmux_resize_client(cols, rows)`,
`tmux_capture(pane)`, `tmux_detach()`, `tmux_pane_channel(pane, Channel)`.
All go through the control connection — including a replacement for `pty_get_cwd`
(`display -p -t %N '#{pane_current_path}'`).

### 2.2 Frontend

- **Mode per tab**: `WorkspaceTab` gains `kind: Shell | Tmux { conn_id, window_id }`.
- **Layout render**: absolute positioning in *cell units* from the layout tree; the 1-cell gaps
  tmux leaves between panes become our divider elements (drag → `resize-pane -t %N -x/-y`).
  Each pane's xterm gets `term.resize(w, h)` from the layout — no FitAddon per pane. The *window*
  container is measured once → `refresh-client -C`.
- **Input**: `term.onData/onBinary` → bytes → `tmux_send(pane, bytes)` → `send-keys -H -t %N …`
  (batched, chunked for paste). Mouse reports ride the same path since each app gets its own
  xterm's encoding. Click on a pane → `select-pane -t %N`.
- **Per-pane decode**: separate `TextDecoder({stream:true})` per pane.
- **Escape hooks**: `registerDcsHandler({final:'t'})` unwraps `tmux;` payloads (`ESC ESC`→`ESC`)
  and re-feeds them, so mdterm's OSC 5337 editor-open/save and OSC 52 keep working unchanged.
- **Native shortcuts → tmux**: Super+T `new-window` (3.5a: `splitw ; breakp`), Super+D / Super+Shift+D
  split, Super+W `kill-pane`, Super+[ ] / Alt+arrows `select-pane -L/R/U/D`, Super+Enter `resize-pane -Z`.
- **Prefix key**: in control mode keystrokes go to the pane, so tmux's prefix table is bypassed.
  Emulate it client-side: read `list-keys -T prefix` in-band, capture the prefix chord, map the next
  key to the bound command, run it through the control connection.
- **Scrollback & selection**: native xterm.js, seeded from `capture-pane -S`; tmux copy-mode unneeded.
- **Exit/detach**: `%exit` → in-band transport returns the tab to its shell (the PTY keeps
  running); spawned transport closes the tmux tabs or offers "reattach".

### 2.3 Detection ("if tmux is running")

Config (`~/.config/mdterm/config.yml`):

```yaml
tmux:
  integration: ask      # auto | ask | off
  session: ""           # "" = most recently used; or a name; created if missing when auto
  scrollback: 2000      # lines seeded per pane on attach
```

- **At launch**: `tmux -V` + `tmux list-sessions` (safe: nothing attached in control mode yet).
  If sessions exist → `auto`: attach; `ask`: small picker (sessions + "plain shell").
- **In-band anytime**: typing `tmux -CC attach` / `tmux -CC new` in any mdterm shell (local or
  `ssh host -t tmux -CC attach`) is detected by `ESC P1000p` in the PTY stream; that tab converts.
- Optional helper: `mdterm --tmux [session]` from any shell.

---

## 3. Phased implementation

### Phase 0 — Spike (prove fidelity)
- `tmux/parser.rs` + `layout.rs` with unit tests from recorded transcripts.
- Spawned transport (`tmux -C attach`), single active pane → existing xterm via a Channel.
- Exit criteria: vim/htop/`ls --color` in a tmux pane render identically to direct PTY.

### Phase 1 — MVP tmux mode
- Command client + model + attach sequence (capture seeding, mode restore, ordering rule).
- Multi-pane layout rendering, focus, `send-keys -H` input, `refresh-client -C` resize.
- Windows ↔ tabs, `%window-add/close/renamed`, `%window-pane-changed`, `%exit` handling.
- Launch-time detection + config block.
- Replace `pty_get_cwd`'s external `tmux display-message` with in-band query when attached.

### Phase 2 — Native UX + editor integration
- Shortcuts → tmux commands, divider drag → `resize-pane`, zoom (`*Z` layout flag).
- DCS `tmux;` unwrap so `mdterm file.md` in a tmux pane opens the editor; editor split beside
  the tmux window; `bin/mdterm` can skip wrapping when `MDTERM_TMUX_CC=1` is set in the session env.
- Pane titles / cwd / running command in pane headers; bell → tab indicator.
- **In-band `-CC` transport** (DCS 1000p detection in `pty_stream` output path) → works over SSH.

### Phase 3 — Robustness & performance
- Flow control (`pause-after`, `%extended-output`, auto-continue).
- `refresh-client -A '%N:off'` for panes in hidden windows, re-capture on show.
- Prefix-key emulation from `list-keys`.
- Version gates & workarounds (3.2 min, 3.4 recommended; 3.5a `neww`).
- Session switcher, reattach on connection loss, multiple simultaneous tmux connections.

### Phase 4 — Polish (optional)
- Drag-to-rearrange panes (`swap-pane`, `join-pane`), break-pane to new tab.
- Per-window sizes on tmux ≥3.4 (`refresh-client -C @id:WxH`) so hidden tabs don't reflow.
- Session/window overview grid (tmuxy's Tab Overview idea).

---

## 4. Testing

- **Parser/layout/client**: pure Rust unit tests + property test for octal decode round-trip.
- **Integration (Rust)**: spawn tmux on a dedicated socket (`-L mdterm-test`, `-f /dev/null`),
  drive via the client: split, resize, capture, `%exit`. Never touch the user's default server.
- **Replay oracle** (fits the existing `tests/terminal/` harness): record `%output` for a pane,
  replay into headless xterm.js, compare buffer with `capture-pane -p` from tmux itself.
- **Visual** (Playwright/WebKit): multi-pane layout geometry — pane rects match layout cells,
  dividers aligned, no clipped rows.

---

## 5. Risks

| Risk | Mitigation |
|---|---|
| tmux crashes with external commands during control mode (3.3a/3.5a) | Route every command in-band; audit `lib.rs`/`bin/mdterm` tmux calls |
| Seeding races (output during capture) | Ordering rule in §2.1, per-pane buffering |
| Size mismatch between cell grid and tmux layout (fractional cells, borders) | Single cell-metrics source; layout drives xterm size; visual tests |
| Other normal tmux clients attached change window size | Respect `window-size` policy; re-layout on every `%layout-change` |
| Prefix-bound workflows break | Prefix emulation (Phase 3); document native shortcuts |
| Heavy output (e.g. `yes`) floods IPC | Existing 4ms coalescing per pane + `pause-after` |

---

## 6. Open decisions

1. **tmux windows → mdterm tabs** (iTerm2 style) vs. a window strip *inside* one mdterm tab.
2. **Launch behaviour default**: `ask`, `auto`, or `off`.
3. **Prefix key**: emulate early (Phase 1) or rely on native shortcuts first.
4. **Minimum tmux**: 3.2 (flow control) vs 3.4 (tmuxy's floor, better tested).
