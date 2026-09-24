# MdTerm - Markdown WYSIWYG Editor ✍️

A native, high-performance **Markdown WYSIWYG Editor** desktop application built with **Rust**, **Leptos**, and **Tauri v2**.

---

## 🌟 Key Features

* **First-Class HTML Document Support**: Open, visually edit, and save full HTML documents (`.html`, `.htm`, `.xhtml`) as well as HTML fragments.
  * **Envelope Preservation**: Preserves `<!DOCTYPE html>`, `<html...>`, `<head>...</head>` (meta tags, `<title>`, stylesheets, scripts), and `<body...>` attributes intact when editing and saving.
  * **No Markdown Mangling**: HTML files are edited and saved purely as HTML—never converted to or overwritten by Markdown syntax.
  * **Format Badge**: Editor header automatically detects and indicates document type (`HTML` vs `MD`).
* **True WYSIWYG Visual Editing**: Write, format, and interact with your documents visually in real-time. Headings, bold, italic, code, quotes, tables, and task lists render inline.
* **Triple Editing Modes**:
  * **Visual / Preview Mode**: Rich visual WYSIWYG editing surface for Markdown; clean rendered document preview for HTML documents.
  * **Split Mode**: Synchronized side-by-side view (raw source on left, live rendered preview on right for both Markdown and HTML).
  * **Source Mode**: Focused raw source text editor with line numbers gutter, syntax formatting, and hotkeys.
  * **Quick Mode Cycling**: Use `Ctrl + M` or the header mode switch buttons to effortlessly toggle between Visual, Split, and Source views.
* **Interactive Task Checklists**: Click `- [ ]` / `- [x]` checkboxes directly in WYSIWYG or preview to toggle tasks.
* **Inline Markdown & HTML Triggers**: Typing `# `, `## `, `- `, `* `, `1. `, `[] `, `> `, or ` ``` ` automatically transforms blocks into semantic elements on the fly.
* **Slash Command Menu**: Press `/` on any line to open a floating block insertion menu (Headings, Tables, Lists, Quotes, Dividers, Code Blocks).
* **Document Outline (Table of Contents)**: Automatically extracts hierarchical headings (H1–H6) from both Markdown and HTML documents with clickable jump-to-section navigation.
* **Document Analytics**: Real-time word count, character count, line count, paragraph count, and estimated reading time (with HTML tag stripping for accurate metric analysis).
* **Multi-Tab Workspace**: Open and edit multiple Markdown and HTML files simultaneously with dirty indicators (`•`) and tab management.
* **Workspace File Explorer**: Built-in file browser to explore directories, open documents, create new documents, and delete files.
* **Find & Replace**: Floating search bar with match counter, next/prev navigation, replace, and replace all.
* **Multi-Theme Support**: Dark, Light, Nord, and Monokai color schemes adhering to Swiss Modernism 2.0 aesthetics with WCAG AAA contrast.
* **Export Options**:
  * Standalone HTML with modern embedded styling
  * Clean Markdown file (`.md`)
  * PDF via Print Dialog

---

## 🏗️ Architecture

```
mdterm/
├── Cargo.toml                  # Cargo workspace manifest
├── README.md                   # Documentation
├── frontend/                   # Client-Side Leptos application (WebAssembly)
│   ├── Cargo.toml              # Leptos, Pulldown-cmark, Htmd, wasm-bindgen
│   ├── Trunk.toml              # Trunk bundler configuration
│   ├── index.html              # HTML shell & font imports
│   ├── styles/
│   │   ├── main.css            # Design tokens (Dark, Light, Nord, Monokai) & layout
│   │   ├── editor.css          # WYSIWYG surface, typography, tables, tasks, code blocks
│   │   ├── toolbar.css         # Formatting toolbar, modals, slash command menu
│   │   └── sidebar.css         # File tree, outline TOC, statistics cards
│   └── src/
│       ├── main.rs             # App entry point
│       ├── app.rs              # Root application state & global shortcuts
│       ├── state.rs            # Reactive state models (Tabs, Themes, Modes, Stats)
│       ├── tauri_bridge.rs     # Tauri IPC bridge with browser fallback
│       ├── html/
│       │   └── mod.rs          # HTML envelope parser, tag stripper, outline extractor, DOM cleaner
│       ├── markdown/
│       │   ├── parser.rs       # Markdown -> HTML (pulldown-cmark with GFM extensions)
│       │   ├── serializer.rs   # HTML -> Markdown (htmd with tasklist normalization)
│       │   └── outline.rs      # Hierarchical heading extractor
│       └── components/
│           ├── header.rs       # Top window bar, tabs, mode switchers, theme dropdown
│           ├── toolbar.rs      # Formatting actions (headings, lists, tables, links, images)
│           ├── wysiwyg_editor.rs # ContentEditable WYSIWYG engine with markdown triggers
│           ├── preview.rs        # Rendered document preview surface for HTML & Markdown
│           ├── source_editor.rs  # Line-numbered raw Markdown editor
│           ├── split_editor.rs   # Side-by-side synchronized view
│           ├── sidebar.rs      # Files tree, Document Outline TOC, Statistics
│           ├── modals.rs       # Insert Link, Image, Table, Export, Find & Replace
│           ├── status_bar.rs   # Path, word/char counts, reading time, save status
│           └── samples.rs      # Built-in sample documents & templates
└── src-tauri/                  # Tauri v2 Backend (Native Rust)
    ├── Cargo.toml              # Tauri 2, Serde, Dirs
    ├── build.rs                # Tauri build hook
    ├── tauri.conf.json         # Window size, bundle settings, security permissions
    ├── capabilities/
    │   └── default.json        # Core permissions
    └── src/
        ├── main.rs             # Application runner
        └── lib.rs              # Tauri IPC commands (read_file, write_file, read_dir, etc.)
```

---

## 🚀 Getting Started

### Prerequisites

* **Rust & Cargo** (stable toolchain)
* **Trunk**: `cargo install trunk` (or prebuilt binary)
* **Tauri CLI**: `cargo install cargo-tauri` or `npx @tauri-apps/cli`
* **Target**: `rustup target add wasm32-unknown-unknown`

### Running in Development Mode

```bash
# Run Tauri desktop app with live reload:
cargo tauri dev

# Or run the Leptos frontend standalone in a browser:
cd frontend
trunk serve
```

### Building for Production

```bash
# Build standalone release binary:
cargo build --package mdterm --bin mdterm --release

# Standalone desktop executable will be located at:
# ./target/release/mdterm
```

### Building Installer Bundles (Linux & macOS)

`cargo tauri build` automatically compiles the WebAssembly frontend via Trunk and creates native installer bundles.

#### Linux (.deb package)

```bash
# 1. Install build dependencies on Ubuntu/Debian:
sudo apt-get update && sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

# 2. Build the .deb installer package:
cargo tauri build --bundles deb

# Output located at:
# ./target/release/bundle/deb/mdterm_0.1.0_amd64.deb
```

#### macOS (.dmg installer)

```bash
# Build DMG for current architecture:
cargo tauri build --bundles dmg

# Or build Universal DMG (Apple Silicon M-series + Intel x86_64):
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo tauri build --target universal-apple-darwin

# Output located at:
# ./target/release/bundle/dmg/*.dmg
# (or ./target/universal-apple-darwin/release/bundle/dmg/*.dmg)
```

---

### Installing the Application

#### On Linux (via `dpkg`)

Install the generated `.deb` package using `dpkg`:

```bash
# 1. Install the Debian package:
sudo dpkg -i target/release/bundle/deb/*.deb

# If any shared library dependencies are missing, fix them with:
sudo apt-get install -f

# 2. (Optional) Install the mdterm CLI tool to your PATH:
sudo cp bin/mdterm /usr/local/bin/mdterm
sudo chmod +x /usr/local/bin/mdterm
```

#### On macOS (via DMG)

You can install the app either using Finder (GUI) or directly from the terminal (CLI):

**Option A: Using Finder / GUI**
```bash
# Open the DMG image in Finder:
open target/release/bundle/dmg/*.dmg
# (or target/universal-apple-darwin/release/bundle/dmg/*.dmg)
```
Drag **mdterm.app** into the **Applications** folder.

**Option B: Using Terminal (`hdiutil`)**
```bash
# 1. Mount the DMG image:
hdiutil attach target/release/bundle/dmg/*.dmg

# 2. Copy mdterm.app into /Applications:
cp -R /Volumes/mdterm*/mdterm.app /Applications/

# 3. Unmount the DMG:
hdiutil detach /Volumes/mdterm*

```

---

## 🌐 Remote Editing over SSH (Zero Server Binaries)

`mdterm` allows opening and saving Markdown files directly across SSH connections and tmux panes without needing to install any binaries on the remote host.

To set up your remote shell:

```bash
# Print setup function for all shells:
mdterm --setup-remote

# Or target a specific shell:
mdterm --setup-remote bash   # for ~/.bashrc or ~/.zshrc
mdterm --setup-remote fish   # for ~/.config/fish/functions/mdterm.fish
```

### Usage over SSH:

```bash
# Opens file in mdterm on your local machine and enables bidirectional saving:
mdterm README.md

# Open on left pane:
mdterm --left README.md

# Pipe content from stdin (read-only):
curl -s https://example.com/doc.md | mdterm
```

When editing, `Ctrl + S` in the editor saves directly back to the remote server, and closing the editor tab returns your terminal prompt.

---

## ⚙️ Configuration

`mdterm` can be configured via a YAML file located at `$HOME/.config/mdterm/config.yml`.

If the file does not exist, `mdterm` creates a commented default configuration automatically upon startup. Changes to `config.yml` are automatically watched and reloaded live without needing to restart the application.

```yaml
# ~/.config/mdterm/config.yml

# Terminal theme: "dark", "light", "nord", "monokai"
# (You can also provide custom color mappings)
theme: "dark"

# Font family for the terminal
font_family: 'JetBrains Mono, Menlo, Monaco, Consolas, "Courier New", monospace'

# Font size in points/pixels
font_size: 13

# Character height / line height multiplier (e.g. 1.0, 1.25, 1.5)
character_height: 1.25

# Terminal renderer: "dom" (default, most reliable) or "webgl" (GPU-dependent)
renderer: "dom"
```

### tmux

mdterm sets `TERM=xterm-256color` and `COLORTERM=truecolor`. For truecolor and flicker-free
(synchronized) redraws inside tmux, add to `~/.tmux.conf`:

```tmux
set -as terminal-features ',xterm-256color:RGB,sync'
```

#### Native tmux integration (control mode)

mdterm can drive tmux through its control mode (`tmux -CC`), the same protocol iTerm2 uses. tmux
keeps running your sessions; mdterm draws them natively:

* every tmux **pane** is its own terminal view, laid out exactly where tmux puts it, with
  draggable dividers (drag → `resize-pane`) and click-to-focus;
* every tmux **window** is an mdterm tab (marked `TMUX`);
* scrollback, selection, copy/paste, file links and the `mdterm` editor command work per pane.

Ways in:

* **At launch** — if tmux sessions exist, mdterm offers to attach (`tmux.integration: ask`).
  `auto` attaches without asking, `off` disables the prompt.
* **From any shell, local or over ssh** — run `tmux -CC attach` (or `tmux -CC new`,
  `ssh -t host tmux -CC attach`). mdterm detects control mode in the stream and opens the
  windows as tabs; nothing needs installing on the remote host. The original tab shows a notice;
  press `Esc` or `q` there to detach.

```yaml
# ~/.config/mdterm/config.yml
tmux:
  integration: "ask"   # ask | auto | off
  session: ""          # attach target for "auto"; empty = most recent session
  scrollback: 2000     # history lines loaded into each pane on attach
```

In control mode keystrokes go straight to the pane, so tmux's prefix key is not interpreted;
use the shortcuts below (or tmux commands from a shell). Requires tmux 3.2 or newer
(3.4+ recommended). Detaching leaves the tmux session running.

### Terminal rendering tests

The terminal pipeline has a regression harness under `tests/terminal/` (see its README):
Rust PTY stream tests, headless xterm.js replay of recorded fixtures (with tmux itself as the
oracle for tmux scenarios), and Playwright/WebKit geometry and screenshot tests. tmux control
mode is covered by Rust tests against a real tmux (spawned and in-band `-CC` in a PTY) and by
`visual/tmux.spec.mjs`, which drives the real frontend client against an isolated tmux server
and compares every pane with `capture-pane`.

```bash
just test-term          # cargo tests + headless replay
just test-term-visual   # WebKit geometry + screenshot tests
```

To turn a rendering glitch into a test, run mdterm with `MDTERM_RECORD_DIR=/tmp/mdterm-rec`,
reproduce it, then `node tests/terminal/scripts/import-recording.mjs /tmp/mdterm-rec/<file>.bin <name>`.

---

## ⌨️ Keyboard Shortcuts Cheatsheet

| Shortcut | Context | Action |
| :--- | :--- | :--- |
| `Ctrl + Shift + T` / `Super + T` | Anywhere | Create new workspace tab (unlimited tabs) |
| `Ctrl + T` | In Editor / UI | Create new workspace tab |
| `Ctrl + Shift + W` / `Super + W` | Anywhere | Close active editor (returns to terminal) / close active tab |
| `Ctrl + W` | In Editor / UI | Close editor pane (returns to full-screen terminal) |
| `Ctrl + Tab` / `Ctrl + Shift + Tab` | Anywhere | Cycle forward / backward through tabs |
| `Ctrl + PageUp` / `Ctrl + PageDown` | Anywhere | Switch to previous / next tab |
| `Ctrl + Shift + 1..9` / `Super + 1..9` | Anywhere | Switch directly to Tab 1 through 9 |
| `Ctrl + 1..9` | In Editor / UI | Switch directly to Tab 1 through 9 |
| `Ctrl + Shift + F` / `Super + F` | Anywhere | Toggle Find & Replace bar (leaves `Ctrl + F` for terminal readline) |
| `Ctrl + F` | In Editor / UI | Toggle Find & Replace bar |
| `Ctrl + Shift + S` / `Super + S` | Anywhere | Save active document (local disk or remote SSH) |
| `Ctrl + S` | In Editor / UI | Save active document |
| `Ctrl + Shift + O` / `Super + O` | Anywhere | Open file modal |
| `Ctrl + O` | In Editor / UI | Open file modal |
| `Ctrl + Shift + N` / `Super + N` | Anywhere | Create new document |
| `Ctrl + N` | In Editor / UI | Create new document |
| `Ctrl + Shift + M` / `Super + M` | Anywhere | Cycle editor mode (Visual → Split → Source) |
| `Ctrl + M` | In Editor / UI | Cycle editor mode |
| `Alt + 1` | Anywhere | Switch to Visual WYSIWYG Mode |
| `Alt + 2` | Anywhere | Switch to Split Mode |
| `Alt + 3` | Anywhere | Switch to Source Mode |
| `Ctrl + Alt + T` / `Super + Alt + T` | Anywhere | Cycle Theme (Dark → Light → Nord → Monokai) |
| `Ctrl + Shift + Q` / `Super + Q` | Anywhere | Close application window |
| `/` | In WYSIWYG | Open Slash Command popup on any line |
| `Tab` / `Shift + Tab` | In Editor | Indent / Outdent list item or insert spaces |
| `Ctrl + F / B / A / E / ...` | In Terminal | Terminal Emacs readline navigation (forward, backward, line start/end) |
| `Super + T` / `Ctrl + Shift + T` | tmux tab | New tmux window (opens as a tab) |
| `Super + W` / `Ctrl + Shift + W` | tmux tab | Close the active tmux pane (tab close button kills the window) |
| `Super + D` / `Ctrl + Shift + D` | tmux pane | Split right |
| `Super + Shift + D` / `Ctrl + Shift + E` | tmux pane | Split down |
| `Super + Alt + Arrows` / `Ctrl + Shift + Arrows` | tmux pane | Move focus to the pane in that direction |
| `Super + [` / `Super + ]` | tmux pane | Previous / next pane |
| `Super + Enter` / `Ctrl + Shift + Enter` | tmux pane | Zoom / unzoom pane |
| `Super + Alt + D` / `Ctrl + Shift + Alt + D` | tmux pane | Detach from tmux |

---

## 📄 License

MIT License
