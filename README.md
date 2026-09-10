# MdTerm - Markdown WYSIWYG Editor ✍️

A native, high-performance **Markdown WYSIWYG Editor** desktop application built with **Rust**, **Leptos**, and **Tauri v2**.

---

## 🌟 Key Features

* **True WYSIWYG Visual Editing**: Write, format, and interact with your documents visually in real-time. Headings, bold, italic, code, quotes, tables, and task lists render inline.
* **Triple Editing Modes**:
  * **WYSIWYG Mode**: Rich visual document editing surface with instant markdown synchronization.
  * **Split Mode**: Synchronized side-by-side view (raw Markdown editor on left, live GFM preview on right).
  * **Source Mode**: Focused raw Markdown text editor with line numbers gutter and code editing hotkeys.
* **Interactive Task Checklists**: Click `- [ ]` / `- [x]` checkboxes directly in WYSIWYG or preview to toggle tasks.
* **Inline Markdown Triggers**: Typing `# `, `## `, `- `, `* `, `1. `, `[] `, `> `, or ` ``` ` automatically transforms blocks into semantic elements on the fly.
* **Slash Command Menu**: Press `/` on any line to open a floating block insertion menu (Headings, Tables, Lists, Quotes, Dividers, Code Blocks).
* **Document Outline (Table of Contents)**: Automatically extracts hierarchical headings (H1–H6) with clickable jump-to-section navigation.
* **Document Analytics**: Real-time word count, character count, line count, paragraph count, and estimated reading time.
* **Multi-Tab Workspace**: Open and edit multiple Markdown files simultaneously with dirty indicators (`•`) and tab management.
* **Workspace File Explorer**: Built-in file browser to explore directories, open files, create new documents, and delete files.
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
│       ├── markdown/
│       │   ├── parser.rs       # Markdown -> HTML (pulldown-cmark with GFM extensions)
│       │   ├── serializer.rs   # HTML -> Markdown (htmd with tasklist normalization)
│       │   └── outline.rs      # Hierarchical heading extractor
│       └── components/
│           ├── header.rs       # Top window bar, tabs, mode switchers, theme dropdown
│           ├── toolbar.rs      # Formatting actions (headings, lists, tables, links, images)
│           ├── wysiwyg_editor.rs # ContentEditable WYSIWYG engine with markdown triggers
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

# 4. (Optional) Create a symlink to use `mdterm` from any terminal:
sudo ln -sf /Applications/mdterm.app/Contents/MacOS/mdterm /usr/local/bin/mdterm
```

---

## ⌨️ Keyboard Shortcuts Cheatsheet

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + S` | Save active document |
| `Ctrl + O` | Open file / show files sidebar |
| `Ctrl + N` | Create new document tab |
| `Ctrl + W` | Close active tab |
| `Ctrl + F` | Toggle Find & Replace bar |
| `Ctrl + B` | Bold (or toggle sidebar) |
| `Ctrl + I` | Italic |
| `Ctrl + U` | Underline |
| `Ctrl + K` | Insert link |
| `Ctrl + 1..6` | Format as Heading 1..6 |
| `Alt + 1` | Switch to WYSIWYG Mode |
| `Alt + 2` | Switch to Split Mode |
| `Alt + 3` | Switch to Source Mode |
| `/` | Open Slash Command popup on any line |
| `Tab` / `Shift+Tab` | Indent / Outdent list item or insert spaces |

---

## 📄 License

MIT License
