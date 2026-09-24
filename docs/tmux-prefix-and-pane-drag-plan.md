# Plan: tmux prefix-key emulation + drag-to-swap panes

Handoff plan for the next increment of mdterm's tmux control-mode integration (PR #1, branch
`tmux-control-mode`). It is written to be implemented without the original author: read
**§0 Context** first, then work through the tasks in order. Every tmux behaviour this plan relies
on was checked against tmux 3.6 on an isolated socket; the transcripts are summarised in **§1**.

Two features:

1. **Prefix-key emulation.** Pressing the user's tmux prefix (e.g. `C-b`) followed by a key does what
   the user's `~/.tmux.conf` binds that key to — including custom bindings, `bind -r` repeat, custom
   key tables and root-table (`bind -n`) bindings. Bindings that open tmux's own on-screen UI
   (copy mode, command prompt, choosers…) get mdterm-native equivalents.
2. **Drag a pane to swap it with another.** Drag a pane onto another pane's centre to swap them
   (`swap-pane`); drop on an edge to move it there (`move-pane`). Optional: drop on a tab to move
   it to that window, or on the tab strip to break it out into a new window.

---

## 0. Context (read this first)

### 0.1 How control mode works in mdterm today

- mdterm attaches to tmux as a **control-mode client** (`tmux -C`/`-CC`). tmux draws nothing for
  such a client; mdterm renders each tmux pane in its own xterm.js and each tmux window as a tab.
- **Keystrokes never go through tmux's key tables.** `term.onData` → `conn.sendKeys(pane, bytes)` →
  `send-keys -H -t %N <hex…>`, which writes bytes straight into the pane. That is why the prefix
  key currently does nothing: tmux never sees it as a key press.
- Commands are sent with `conn.run(commands, onDone)` (`frontend/js/tmux-client.js`). Multiple
  commands in one call go out on one line joined by ` ; ` and each gets its own reply
  (`{ok, lines}`), delivered **synchronously** in the channel callback, in tmux's order.
- tmux notifications (`%layout-change`, `%window-*`, `%exit` …) are handled in
  `TmuxConnection.onEvent`. A `%layout-change` re-renders the window from the layout tree
  (`renderWindow`), re-using each pane's `PaneView` (same xterm) and resizing it; a resized pane
  re-seeds itself from `capture-pane` (`PaneView.setSize` → `scheduleReseed`).

### 0.2 Files you will touch

| File | What's there now | Anchors |
|---|---|---|
| `frontend/js/tmux-client.js` | `PaneView` (one xterm per pane), `TmuxConnection` (transport, windows, layout, dividers, seeding, actions), `tmuxShortcut`, exported entry points | `class PaneView` ~L196, `ensureTerm` ~L216, `class TmuxConnection` ~L291, `start` ~L312, `run` ~L364, `onEvent` ~L400, `renderWindow` ~L569, `renderDividers`/`startDividerDrag` ~L608/633, `selectPane` ~L783, `action` ~L838, `handleKey` ~L866, `tmuxShortcut` ~L891 |
| `frontend/js/terminal-core.js` | `installTerminalIntegration(term, container, ctx)` — shared key handler, clipboard, OSC, links. Calls `ctx.handleKey(event)` first on **keydown only** | key handler ~L618–626 |
| `frontend/styles/main.css` | `.tmux-window`, `.tmux-pane`, `.tmux-divider*`, `.tmux-measure` (end of file) | |
| `src-tauri/src/config.rs` | `TmuxConfig { integration, session, scrollback }` + commented template | `pub struct TmuxConfig` |
| `src-tauri/src/tmux/parser.rs` | control-mode parser | only if you add `%message` handling (§2.8) |
| `tests/terminal/visual/tmux.spec.mjs` | Playwright E2E against a real tmux; `attach(page)`, `oracle(pane)`, `expectPaneMatchesTmux` | |
| `tests/terminal/visual/tmux-backend.mjs` | Node stand-in for the Rust backend; `useSocket`, `tmux(...)`, `killServer`, `ControlConnection` | |
| `tests/terminal/visual/tmux-harness.html` | test page mocking `window.__TAURI__`; `window.harness` helpers | |
| `tests/terminal/package.json` | `"test": "node --test replay.test.mjs"` | add new unit test file |
| `README.md` | "Native tmux integration" section + shortcut table | |

New files proposed: `frontend/js/tmux-keys.js` (pure, DOM-free, unit-testable), `frontend/js/tmux-ui.js`
(overlays: prompt, confirm, message, display-panes, prefix indicator), `tests/terminal/tmux-keys.test.mjs`.

**Bundling note:** Trunk only copies JS files that are reachable from a `#[wasm_bindgen(module = …)]`
import *or* imported relatively from such a file. `tmux-client.js` is referenced from
`frontend/src/tauri_bridge.rs`; files you import relatively from it (`./tmux-keys.js`,
`./tmux-ui.js`) land in the same `dist/snippets/<crate>/js/` directory. **Verify after
`cd frontend && trunk build`** that both new files appear under `dist/snippets/*/js/`. If they
don't, add a trivial `#[wasm_bindgen(module = "/js/tmux-ui.js")]` extern block in `tauri_bridge.rs`.

### 0.3 How to run things

```bash
cargo test -p mdterm                                   # Rust (incl. real-tmux tests)
cargo build -p frontend --target wasm32-unknown-unknown # frontend type-check
cd tests/terminal
npm test                                               # node unit + replay tests
npx playwright test visual/tmux.spec.mjs --reporter=line            # tmux E2E
npx playwright test visual/tmux.spec.mjs --repeat-each=6            # flake check (must be 100%)
npx playwright test                                    # everything (visual + tmux)
cd frontend && trunk build                             # bundle check (see note above)
```

Never point tests at the user's default tmux server. The E2E suite gives each test its own
socket (`useSocket`) and removes it afterwards (`killServer`); keep that pattern.

---

## 1. Verified tmux facts (tmux 3.6)

| Fact | Evidence / consequence |
|---|---|
| `show -gv prefix` → `C-b`; `show -gv prefix2` → `None`; `show -gv repeat-time` → `500` | Query these on attach. `None` means unset. |
| `list-keys` prints one binding per line: `bind-key [-r] -T <table> <key> <command…>`, key padded with spaces, special keys backslash-escaped: `\"` `\#` `\$` `\%` `\'` `\;` `\{` `\}` `\~` | Parse with a regex, unescape a leading `\` in the key field. `-r` = repeatable. |
| Key names seen: `Space`, `Enter`, `Escape`, `Tab`, `BTab`, `BSpace`, `DC`, `IC`, `Home`, `End`, `PPage`, `NPage`, `Up`/`Down`/`Left`/`Right`, `F1`… , modifiers `C-`, `M-`, `S-` (e.g. `C-b`, `M-1`, `S-Up`, `C-Up`, `M-Right`), printable chars (`%`, `"`, `c`, `C`, `[`) | Browser `KeyboardEvent` → tmux key name mapping in §2.1. |
| The **command text printed by `list-keys` can be sent back verbatim** on the control connection, including `{ … }` blocks (`if-shell -F 1 { split-window -h }` worked) | Emulation = look up the binding and `conn.run([commandText])`. |
| A command run on the control connection acts on the **client's current pane/window** | mdterm already keeps tmux's current window/pane in sync (`select-window` on tab focus, `select-pane` on click), so untargeted bound commands hit the pane the user is in. |
| `display-message "…"` (without `-p`) returns `%message <text>` **as a reply line** of that command | Show it as a toast (§2.8). |
| Bindings that open tmux UI are **invisible** to a control client: `copy-mode` (pane enters `pane_mode=copy-mode`, `%pane-mode-changed` fires, nothing is drawn), `command-prompt`, `confirm-before`, `choose-tree/-buffer/-client`, `display-menu`, `display-popup`, `display-panes`, `customize-mode`, `clock-mode`, `show-messages`, `list-keys -N` | Must be intercepted and emulated or refused (§2.4). |
| `send-keys -K <key>` (tmux ≥ 3.4) routes a key through the control client's key tables, and bound commands then run (split, `-r` repeat, zoom verified) | **Not recommended as the main mechanism** — see §2.0. |
| `swap-pane -s %A -t %B` swaps two panes (emits `%layout-change` with pane ids exchanged, geometry unchanged). `swap-pane -d` still changed the active pane in a test, so always follow with an explicit `select-pane`. | Drag-to-swap = one command + `select-pane`. |
| `move-pane -s %A -t %B -v -b` moved `%A` above `%B` (layout became `[…]`) | Edge drops (§3.3). |
| Commands started by key bindings report their reply block with flags `0`; the parser already ignores those | No backend changes needed for emulation. |

---

## 2. Feature A — prefix-key emulation

### 2.0 Approach (decided)

mdterm keeps its own copy of the user's key tables and **executes the bound command text on the
control connection**, instead of forwarding keys with `send-keys -K`.

Why not `send-keys -K`:
- It needs tmux ≥ 3.4 (we support ≥ 3.2).
- The client's key-table state lives inside tmux, so mdterm would need a round trip after every
  key to learn whether it is still in the prefix/repeat table, and buffer typing meanwhile.
- UI bindings (`[`, `:`, `x`, `s`…) would still fire invisibly; we must know the binding before
  sending anyway to intercept them.

With the local table, routing is synchronous, testable as a pure function, and works on 3.2+.
The cost is emulating a few tmux semantics ourselves (repeat, `switch-client -T`), specified below.

### 2.1 `frontend/js/tmux-keys.js` (new, pure — no DOM, no Tauri)

Export:

```js
// KeyboardEvent-like {key, code, ctrlKey, altKey, shiftKey, metaKey} → tmux key name or null.
export function keyEventToTmux(ev) {}
// list-keys output lines → { tables: { [table]: Map<keyName, { command, repeat }> } }
export function parseListKeys(lines) {}
// Command text → { kind: 'run' } | { kind: 'native', name, args } (see §2.4)
export function classifyCommand(commandText) {}
// Minimal tmux command tokenizer: words, "double", 'single', {braces} (kept as one token), \escapes, top-level ';'
export function tokenize(commandText) {}
// Pure key router state machine (§2.2)
export function createKeyRouter({ tables, prefix, prefix2, repeatTime, now }) {}
```

`keyEventToTmux` rules:
- Ignore pure modifier presses (`Shift`, `Control`, `Alt`, `Meta`, `AltGraph`) → `null`. Ignore `isComposing`.
- Named keys: `ArrowUp→Up`, `ArrowDown→Down`, `ArrowLeft→Left`, `ArrowRight→Right`, `Enter→Enter`,
  `Escape→Escape`, `Tab→Tab` (`Shift+Tab→BTab`), `Backspace→BSpace`, `Delete→DC`, `Insert→IC`,
  `Home`, `End`, `PageUp→PPage`, `PageDown→NPage`, `F1…F12`, `' '→Space`.
- Printable single-character `ev.key`: use the character itself (`%`, `"`, `C`); Shift is already
  folded into the character, so **don't** add `S-` for printables.
- Ctrl + letter: `C-<lowercase letter>` (`C-b`). Ctrl + other printable: `C-<char>` (`C-Space` for space).
- Alt: `M-` prefix (`M-1`, `M-Up`). On macOS use `ev.code` to recover the unmodified character when
  Option produced a symbol (`KeyA` → `a`).
- Shift with named keys: `S-Up` etc. Combine in tmux order **`C-M-S-`**: `C-M-Up`.
- `metaKey` (Super/Cmd) → return `null` (reserved for mdterm shortcuts).

`parseListKeys`:
- Regex per line: `^bind-key\s+((?:-r\s+)?)-T\s+(\S+)\s+(\S+)\s+(.*)$`. Group 1 non-empty → repeat.
- Key: if it matches `^\\.$` strip the backslash. Command: trim.
- Unknown line formats: skip (tolerate `-N` note flags if present by allowing `(?:-N\s+"[^"]*"\s+)?`).
- Unit test against the exact default table captured in §1 (copy it into the test as a fixture).

### 2.2 Router state machine (`createKeyRouter`)

State: `table` (`'root'` normally), `repeatUntil` (ms timestamp or 0).

`router.handle(keyName) → { consume: boolean, run?: string, native?: {...}, sendPrefix?: true }`

1. `table === 'root'`:
   - `keyName` equals `prefix` or `prefix2` → `table = 'prefix'`, `consume: true` (show indicator).
   - Else if `tables.root` has a binding for `keyName` → treat as a bound key (step 3), stay in root.
     (Supports `bind -n M-h select-pane -L`.)
   - Else → `consume: false` (normal typing, goes to the pane).
2. `table !== 'root'` and `now() > repeatUntil` and we're in *repeat* mode (entered via step 3) →
   the repeat window expired: reset to root and re-run step 1 with this key.
3. Look up `tables[table].get(keyName)`:
   - **Found:** result = `classifyCommand(command)`.
     - If the binding is `repeat` → stay in `table`, `repeatUntil = now() + repeatTime`.
       Else → `table = 'root'`, `repeatUntil = 0`.
     - If the command is `switch-client -T <name>` (and nothing else) → `table = name`, consume,
       don't run anything. (tmux semantics: next key is looked up in `<name>`.)
     - `send-prefix` → return `{ consume: true, run: 'send-prefix' }` (runs fine on the control
       connection; sends the prefix to the current pane). `send-prefix -2` likewise.
     - Otherwise return `{ consume: true, run: command }` or `{ consume: true, native }`.
   - **Not found:** reset to root, `consume: true` (tmux swallows unbound keys after the prefix).
     Exception: if we were in the repeat window (not freshly after the prefix), tmux treats the key
     as a normal key → reset to root and re-run step 1 so it reaches the pane.
4. `Escape` right after the prefix with no `Escape` binding → cancel (reset), consume.

`now` is injected so tests can control time. No timers inside the router; the UI polls
`router.state()` to show/hide the indicator (or set a `setTimeout(repeatTime)` in the UI layer to
clear it).

### 2.3 Wiring in `tmux-client.js`

- On `start()` (after `refresh-client -f pause-after=5`) and again after every successful
  prefix-sequence (debounced, background), load:
  ```
  show -gv prefix ; show -gv prefix2 ; show -gv repeat-time ; list-keys
  ```
  in **one** `run([...])` call; build the router. `list-keys` without `-T` returns all tables.
  If the user sources a new config the next refresh picks it up.
  Store `this.router`; until it's loaded, prefix emulation is inactive (keys pass through).
- Config toggle: add `prefix_emulation: Option<bool>` to `TmuxConfig` in `src-tauri/src/config.rs`
  (default true; document in the template and README). Read via
  `currentConfig().tmux?.prefix_emulation !== false`.
- In `TmuxConnection.handleKey(pane, ev)` **before** `tmuxShortcut(ev)`:
  ```js
  const name = keyEventToTmux(ev);
  if (this.router && name) {
    const r = this.router.handle(name);
    if (r.consume) { this.applyRouted(pane, r); return true; }
  }
  ```
  `applyRouted`:
  - `run` → `this.run([r.run], ([res]) => this.showReply(res))` (§2.8).
  - `native` → dispatch to `tmux-ui.js` / native handlers (§2.4).
  - Update the prefix indicator (§2.6).
- **Keypress/keyup leakage:** `installTerminalIntegration` returns early for non-keydown events
  *before* calling `ctx.handleKey`. When a keydown was consumed, the browser suppresses keypress
  (we `preventDefault`), but verify with the E2E test that no stray character reaches the pane
  (e.g. after `C-b %`, the pane must not receive `%`). If it leaks, make `handleKey` also swallow
  the matching keypress (track `this.lastConsumedKey`).
- mdterm's own shortcuts (`Super+…`, `Ctrl+Shift+…`) must keep working. `keyEventToTmux` returns
  null for `metaKey`, so Super shortcuts are unaffected. **Do not** treat `Ctrl+Shift+<letter>` as a
  tmux key: return `null` from `keyEventToTmux` when both ctrl and shift are held with a letter,
  so the existing `Ctrl+Shift+T/W/D/E/…` mdterm shortcuts win.
- When the tab/pane loses focus mid-prefix, reset the router to root (call `router.reset()` from
  `focusWindow`/`selectPane` when the target changes).

### 2.4 Commands that need native handling (`classifyCommand`)

Tokenize, split into commands on top-level `;`, look at each command's first word (resolve
aliases below). If **any** command in the list is a UI command, return `{kind:'native', …}` for
the whole binding; otherwise `{kind:'run'}`.

| tmux command (aliases) | Native behaviour | Priority |
|---|---|---|
| `copy-mode` (`copy-mode -u`, `PPage` binding) | Focus the active pane's xterm and scroll its viewport up one page (`-u`) or leave it (plain `[`); show a small "SCROLLBACK" badge until the viewport is back at the bottom. mdterm's native selection + copy already replaces tmux copy mode. **Never** run `copy-mode` on the connection. | P1 |
| `command-prompt` (`-I inputs`, `-p prompts`, template `{ … "%%" … }` or string) | Prompt overlay at the bottom of the window mount (like tmux's status prompt). `-I` values are format-expanded first via `display -p '<value>'` (e.g. `#W` → window name). On Enter: if a template was given, substitute `%%`/`%1`… (and `%%%` → quoted) then run it; with no template, run the typed text as a command. Esc cancels. Show the command's error text (reply `ok:false`) inline in red. | P1 |
| `confirm-before` (`confirm`) `-p "prompt" CMD` | Confirm overlay "prompt (y/n)"; `y` runs `CMD`, anything else cancels. Expand formats in the prompt via `display -p`. Default prompt when `-p` absent: `Confirm '<CMD>'? (y/n)`. | P1 |
| `detach-client` (`detach`) | `this.detach()` (already exists). | P1 |
| `display-message` (`display`) without `-p` | Run it; show the `%message …` reply line as a toast (§2.8). | P1 |
| `display-panes` (`displayp`) | Overlay a large number on each visible pane (`#{pane_index}` from `list-panes -F`), for `display-panes-time` ms (query the option; default 1000). A digit key while visible → `select-pane -t %id`. Consumes that digit. | P2 |
| `choose-tree` (`-s` sessions, `-w` windows), `choose-session`, `choose-window` | Overlay list from `list-windows -a -F '#{session_name}\t#{window_id}\t#{window_index}\t#{window_name}'` (or sessions only for `-s`). Arrow keys/Enter/Esc. Selecting a window in the current session → `select-window -t @id`; another session → `switch-client -t <session>` (mdterm already resyncs on `%session-changed`). | P2 |
| `display-menu` (`menu`), `display-popup` (`popup`), `customize-mode`, `clock-mode`, `choose-buffer`, `choose-client`, `show-messages` (`showmsgs`), `list-keys` (`lsk`), `find-window` (`findw`) | Toast: "`<command>` isn't available in mdterm's tmux mode". Consume. | P1 (the toast) |
| `list-buffers` (`lsb`), `show-options`, other `list-*`/`show-*` read commands | Run on the connection and show the reply lines in a dismissible overlay (they would open tmux's view-mode on a normal client). | P2 |
| `suspend-client` (`suspendc`) | Ignore (toast). | P1 |
| everything else | `{kind:'run'}` | — |

Alias table to include in code: `splitw split-window`, `neww new-window`, `selectp select-pane`,
`selectw select-window`, `killp kill-pane`, `killw kill-window`, `resizep resize-pane`,
`swapp swap-pane`, `breakp break-pane`, `joinp join-pane`, `movep move-pane`, `detach
detach-client`, `display display-message`, `displayp display-panes`, `confirm confirm-before`,
`menu display-menu`, `popup display-popup`, `lsk list-keys`, `lsb list-buffers`, `showmsgs
show-messages`, `findw find-window`, `switchc switch-client`, `suspendc suspend-client`.

**`new-window` on tmux 3.5a:** bound `new-window` commands (`prefix c`) must go through the same
3.5a workaround that `action('new-window')` uses (`this.newWindowBroken`). In `applyRouted`, if
`newWindowBroken` and the command is exactly `new-window`/`neww` with only `-a`/`-c` flags, run
`split-window` + `break-pane -a` instead; otherwise run as-is.

### 2.5 Pane in a tmux mode (safety net)

If a pane ends up in copy/view mode anyway (a script ran `copy-mode`, or a binding we classified as
`run` enters a mode), mdterm's keystrokes (`send-keys -H`) are consumed invisibly by the mode.
Handle `%pane-mode-changed` (already parsed, currently ignored in `onEvent`):
- Query `display -p -t %N '#{pane_mode}'`. If non-empty, show a pane overlay:
  "tmux <mode> — press Esc to exit" and route **all** keys for that pane to
  `send-keys -X -t %N cancel` on Esc (other keys: swallow). When it becomes empty, remove the overlay.

### 2.6 Prefix indicator

While the router is out of the root table, add class `tmux-prefix-active` to the window mount and
render a small badge at the bottom-right of the active pane: the table name (`PREFIX`, or the
custom table name), and `REPEAT` while inside the repeat window. Clear it on reset / timeout.
Style next to the existing `.tmux-*` rules in `main.css` (use `var(--accent)`).

### 2.7 `tmux-ui.js` (new)

DOM-only helpers, each takes the window mount element and returns `{ close() }`:
`showPrompt({ prompt, initial, onSubmit, onCancel, error })`, `showConfirm({ text, onYes })`,
`showOverlayText({ title, lines })`, `showPaneNumbers({ panes: [{id, index, rect}], timeout, onPick })`,
`showChooser({ items, onPick })`, `setPrefixBadge(el, textOrNull)`.

Rules: overlays capture keyboard focus (an `<input>` for the prompt) and must stop propagation so
keys don't reach xterm or the Leptos global handler; restore focus to the active pane's xterm on
close (`conn.focusPane`). Style under `.tmux-overlay*` in `main.css` using existing CSS variables
(`--bg-terminal`, `--border-color`, `--accent`).

### 2.8 Reply handling for bound commands

`showReply({ok, lines})`:
- `ok:false` → toast with the first line (e.g. "can't find pane").
- lines starting with `%message ` → toast with the rest.
- other non-empty lines (a bound command that prints) → `showOverlayText`.

No Rust change is needed: these arrive as normal reply lines because we send the command ourselves.

### 2.9 Tests for Feature A

**Unit (`tests/terminal/tmux-keys.test.mjs`, add to `npm test`:
`"test": "node --test replay.test.mjs tmux-keys.test.mjs"`):**
- `keyEventToTmux`: `C-b`, `%`, `"`, `C` (shift), `Space`, `Up`, `S-Up`, `C-Up`, `M-1`, `C-M-Up`,
  `PPage`, `DC`, `BTab`, `Escape`; `metaKey` → null; modifier-only → null; `Ctrl+Shift+T` → null.
- `parseListKeys` on the §1 default table: count, `\%` → `%`, `-r` flags on `Up`/`M-Up`/`C-Up`,
  `{…}` commands kept intact.
- `tokenize`: quotes, braces, escaped `\;`, top-level `;` splitting.
- `classifyCommand`: `split-window -h` → run; `copy-mode` → native; `confirm-before -p "kill-pane #P? (y/n)" kill-pane`
  → native confirm with prompt+command; `command-prompt -I "#W" { rename-window "%%" }` → native prompt with
  initial `#W` and template; `if-shell -F 1 { copy-mode }` → native (UI command nested).
- Router: prefix then `%` → run `split-window -h`, back to root; unbound key after prefix →
  consumed, root; `-r` binding repeat within `repeatTime` (fake clock) runs again without prefix;
  after expiry the key passes through (`consume:false`); `switch-client -T mytable` then key
  looks up `mytable`; `prefix2`; root binding `M-h`; `C-b C-b` → `send-prefix`.

**E2E (`visual/tmux.spec.mjs`):**
- Extend `useSocket(name, confPath)` / `tmux()` in `tmux-backend.mjs` so a test can start its
  server with a config file (write it to `os.tmpdir()`; today everything uses `-f /dev/null`).
  `ControlConnection` must use the same `-f`.
- Default prefix: focus pane, `Control+b` then `%` → 2 panes, and the pane's screen (via
  `oracle`) does **not** contain a stray `%`. `C-b "` → vertical split. `C-b z` → window flags
  contain `Z` (`tmux display -p '#{window_zoomed_flag}'` = 1). `C-b c` → a second tab appears.
  `C-b n` / `C-b p` → active tab changes. `C-b C-b` in a pane running `cat -v` → oracle shows `^B`.
- Custom config: `set -g prefix C-a` + `bind | split-window -h` + `bind -r H resize-pane -L 5` +
  `bind -n M-h select-pane -L`: `C-a |` splits; `C-a H H` shrinks by 10 columns; wait
  `repeat-time`+100ms, press `H` → reaches the pane (oracle shows `H`); `Alt+h` moves focus left.
- Native UI: `C-b :` → prompt overlay visible; type `split-window -v`, Enter → split happens.
  `C-b ,` → prompt prefilled with the window name; replace, Enter → tab title changes (listen for
  `mdterm-tmux-window-renamed` in the harness). `C-b x` → confirm overlay; `y` → pane killed;
  `n` → not killed. `C-b [` → pane scrolled up / badge shown, pane **not** in copy mode
  (`#{pane_mode}` empty). `C-b d` → `__tmuxExited`. `C-b s` (if P2 implemented) → chooser.
- Safety net: `tmux copy-mode -t %N` from outside → overlay appears; Esc → `#{pane_mode}` empty.
- Run with `--repeat-each=6`; must be 100% green.

---

## 3. Feature B — drag a pane to swap/move it

### 3.1 Interaction design

- **Grab handle:** when a window has ≥ 2 visible panes and isn't zoomed (`w.flags` has no `Z`),
  hovering a pane shows a small grip (`⠿`, 22×14 px) centred at the pane's top edge
  (`.tmux-pane-grip`, `opacity: 0` → `1` on `.tmux-pane:hover`). Mousedown on it starts a drag.
  Also allow **`Alt+Shift` + drag anywhere** in a pane (capture-phase `mousedown` listener on the
  pane element, so xterm's selection doesn't start). Avoid `Super`+drag (window managers use it).
- **While dragging:**
  - add `body.tmux-dragging-pane` (cursor `grabbing`, `user-select: none`);
  - the source pane gets `.tmux-pane-drag-source` (dimmed, dashed outline);
  - a ghost (`.tmux-drag-ghost`, a translucent rectangle ~40% of the source's size, labelled with the
    pane's command, from `#{pane_current_command}`, which you can add to `PANE_FIELDS` or query on drag start)
    follows the cursor;
  - the pane under the cursor shows a **drop indicator** for the zone the cursor is in:
    - **centre** (inner 50% box) → full-pane highlight, label "Swap";
    - **edge zones** (outer 25% band on each side) → half-pane highlight on that side, label
      "Move left/right/up/down".
  - dropping on the source itself or outside any pane → cancel.
  - `Escape` cancels (listen on `document` in capture phase during the drag).
- Use pointer events (`pointerdown/move/up` + `setPointerCapture`) so the drag keeps working over
  xterm elements and outside the window.
- Hit-testing: compute from layout geometry, not from DOM `elementFromPoint` (xterm layers
  intercept): `leaf.x*cw … (leaf.x+leaf.w)*cw` relative to the `.tmux-window` rect, using the
  current window's `w.visible` layout (`layoutPanes`).

### 3.2 Commands

| Drop | Command(s) (one `run([...])` call, same line) |
|---|---|
| centre of `%B` | `swap-pane -s %A -t %B` then `select-pane -t %A` |
| left/right edge of `%B` | `move-pane -s %A -t %B -h` (right) / `-h -b` (left), then `select-pane -t %A` |
| top/bottom edge of `%B` | `move-pane -s %A -t %B -v` (bottom) / `-v -b` (top), then `select-pane -t %A` |
| *(optional P3)* a tab of another tmux window `@W` in the title bar | `move-pane -s %A -t @W` then `select-window -t @W` |
| *(optional P3)* empty space in the tab strip | `break-pane -s %A` (new window becomes current; tab appears via `%window-add`) |

Errors (`ok:false`, e.g. "pane too small") → toast via `showReply`.

No other state changes are needed: tmux answers with `%layout-change` (and `%window-pane-changed`),
`renderWindow` moves the existing `PaneView` elements, and panes whose size changed re-seed
themselves. `move-pane` across windows produces `%layout-change` for both windows; `renderWindow`
re-parents the `PaneView` element and sets `view.windowId`. **Check** that `removeWindow` doesn't
destroy a pane that just moved to another window (it only destroys panes not present in any
remaining layout — confirm with a test).

### 3.3 Implementation outline

Put the drag code in `tmux-client.js` next to `startDividerDrag` (or a new `tmux-drag.js` imported by
it):

1. In `renderWindow`, when `multi && !zoomed`, ensure each pane element has a grip child
   (create once per `PaneView`, toggle `display`). Grip `pointerdown` → `startPaneDrag(e, view, w)`.
2. `startPaneDrag`: snapshot `rect = mount.el.getBoundingClientRect()`, `leaves = layoutPanes(w.visible)`,
   `{width:cw,height:ch} = this.cell`; create ghost + indicator elements inside `mount.el`;
   on move: find the leaf under the pointer, compute zone, position indicator; on up: run the
   command for the zone; always clean up (also on `pointercancel`, Escape, window blur, and if a
   `%layout-change` for this window arrives mid-drag — then cancel, the geometry is stale).
3. Keep `renderWindow` idempotent: it already removes/recreates dividers; do the same for any
   drag-indicator it doesn't own (it shouldn't touch the ghost during a drag).
4. Optional polish: `transition: left/top/width/height 120ms` on `.tmux-pane` **only** while a class
   `tmux-animate` is on the mount, added right before running swap/move and removed after 200 ms,
   so divider drags stay instant.

### 3.4 Tests for Feature B (E2E)

Add helpers to `tmux-harness.html`: `harness.paneGeometry()` → `[{id, rect}]` from the DOM (already
available via `panes()`), and nothing else is needed.

- **Swap:** split (`C-b %` or `Control+Shift+D`), print distinct markers in each pane
  (`echo LEFT-PANE` / `echo RIGHT-PANE` via `tmux send-keys -t %N`). Drag the left pane's grip to
  the centre of the right pane with `page.mouse` (move in ≥5 steps). Expect
  `tmux list-panes -F '#{pane_id} #{pane_left}'` order swapped; the pane now on the left shows
  `RIGHT-PANE` (`expectPaneMatchesTmux` for both); tmux's active pane is the dragged pane.
- **Move to edge:** with two side-by-side panes, drop pane A on the bottom edge of pane B →
  `#{window_layout}` contains `[`; both panes still match the oracle.
- **Cancel:** start a drag, press Escape → layout unchanged (compare `#{window_layout}` before/after).
  Drop on itself → unchanged.
- **Zoomed / single pane:** no grip visible (`.tmux-pane-grip` hidden) and `Alt+Shift` drag does nothing.
- **Selection still works:** plain drag inside a pane (no modifier, not on the grip) selects text
  (`term.hasSelection()` true) and doesn't move panes.
- *(P3)* drag onto another tab → pane moves windows, source tab still exists with the remaining pane.
- Repeat ×6, 100% green.

---

## 4. Task list (suggested order, each ends green)

1. **`tmux-keys.js` + unit tests** — `keyEventToTmux`, `parseListKeys`, `tokenize`,
   `classifyCommand`, `createKeyRouter`. Wire `npm test`. *(No UI yet.)*
2. **Router wiring** — load options + `list-keys` in `start()`, `handleKey` integration, `run`
   path, `showReply` with toasts, prefix badge, `prefix_emulation` config flag (Rust config +
   template + README). E2E: default prefix tests, custom config tests (needs the `-f` support in
   `tmux-backend.mjs`).
3. **Native UI P1** — `tmux-ui.js` prompt/confirm/overlay text; `command-prompt`, `confirm-before`,
   `copy-mode` → scrollback, `detach`, unsupported-command toast, `%pane-mode-changed` safety net,
   3.5a `new-window` rewrite for bound commands. E2E native-UI tests.
4. **Drag-to-swap** — grip, pointer drag, zones, swap + move commands, cancel paths. E2E swap/move/cancel/zoom/selection tests.
5. **Native UI P2** *(optional)* — `display-panes`, `choose-tree`, read-command overlay.
6. **Cross-window drag P3** *(optional)* — tab-strip drop targets (needs a small hook from the
   Leptos title bar: expose `data-session-id` on tab elements in `titlebar.rs` so JS can hit-test them).
7. **Docs** — README shortcut table ("tmux prefix works; these bindings are emulated…; drag the
   grip to swap"), update the status table in `docs/tmux-integration-plan.md`.

## 5. Acceptance criteria

- With a default tmux config, every binding in `list-keys -T prefix` either runs, has a native
  equivalent, or shows a clear "not available" toast — nothing fails silently and no pane is left
  stuck in an invisible mode.
- Custom prefix, `prefix2`, `bind -r` repeat, `bind -n` root bindings and `switch-client -T` tables
  from the user's config work.
- Prefix keys never leak into the pane; mdterm's `Super+…` / `Ctrl+Shift+…` shortcuts keep working.
- Panes can be swapped by drag (centre) and moved by drag (edges); Escape cancels; text selection
  inside panes is unaffected; zoomed windows show no grip.
- `cargo test -p mdterm`, `npm test`, and the full Playwright suite pass; the tmux E2E suite passes
  6/6 repeats.

## 6. Risks / gotchas

- **Keyboard layouts:** `ev.key` for Ctrl+letter on non-US layouts can be a non-Latin character; fall
  back to `ev.code` (`KeyB` → `b`) when `ctrlKey` and `ev.key` isn't ASCII.
- **IME / composition:** never route keys while `ev.isComposing`.
- **Stale bindings:** bindings are cached; refresh after each prefix sequence (background) and on
  `%session-changed`. Document that `source-file` takes effect on the next prefix press.
- **Targets:** bound commands without `-t` act on tmux's current pane. mdterm keeps it in sync on
  click/tab focus; if a test shows a race (click then immediate `C-b x`), prepend
  `select-pane -t %<focused>` to the same `run([...])` line (same line = atomic).
- **Commands that print:** a bound `run-shell` with output shows up as reply lines → overlay; this
  is intended.
- **`%message` outside a reply block:** not observed, but if it shows up, add a `Message` event to
  `parser.rs` and a `message` JSON frame in `event_json` (`src-tauri/src/tmux/mod.rs`), then toast it.
- **Drag over xterm:** xterm's own mouse handling (selection, app mouse reporting) must not start when
  the drag begins on the grip or with `Alt+Shift`; use capture-phase listeners and
  `stopPropagation`/`preventDefault`.
