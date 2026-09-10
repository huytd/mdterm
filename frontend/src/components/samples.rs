pub const WELCOME_MD: &str = r#"# Welcome to MdTerm ✍️

**MdTerm** is a blazing-fast, elegant **Markdown WYSIWYG Editor** built natively with **Rust**, **Leptos**, and **Tauri**.

---

## ✨ Key Features

*   **True WYSIWYG Editing**: Write and format your documents visually in real time.
*   **Split Mode & Source Mode**: Switch effortlessly between visual editing, side-by-side split view, and raw Markdown.
*   **Interactive Tasks**: Click checkboxes directly in the document to toggle tasks.
*   **Table Management**: Insert, customize, and navigate tables with ease.
*   **Code Blocks**: Highlighted code blocks with a 1-click copy button.
*   **Document Outline**: Live Table of Contents generated from your headings.
*   **Multiple Themes**: Switch between Dark, Light, Nord, and Monokai modes.

---

## 📋 Task Checklist

- [x] Set up Rust, Leptos, and Tauri application
- [x] Implement Markdown parser and HTML serializer
- [x] Build rich WYSIWYG editing surface
- [x] Add interactive task list toggle
- [ ] Write my first markdown document in MdTerm
- [ ] Export document to clean HTML or PDF

---

## 📊 Feature Comparison

| Feature | MdTerm | Traditional Editors |
| :--- | :---: | :---: |
| **Instant WYSIWYG** | ✅ Yes | ❌ Split Only |
| **Rust Native Speed** | ✅ Yes | ❌ Heavy Webview |
| **Offline Local Files** | ✅ Full Support | ⚠️ Cloud Tied |
| **Multi-tab Workflow** | ✅ Yes | ❌ Single File |

---

## 💻 Code Snippet Showcase

```rust
// A fast and elegant Rust function
fn main() {
    println!("Hello from MdTerm WYSIWYG Markdown Editor!");
    let features = vec!["Fast", "Native", "Reactive"];
    for feature in features {
        println!("- {}", feature);
    }
}
```

---

## 📊 Interactive Mermaid Diagrams

```mermaid
graph LR
    A[Markdown File] -->|Open| B[mdterm Editor]
    B --> C{View Mode}
    C -->|WYSIWYG| D[Visual Diagram]
    C -->|Code Tab| E[Mermaid Source]
    D --> F[Terminal Split]
    E --> F
```

> "Simplicity is prerequisite for reliability."
> — Edsger W. Dijkstra

---

## ⌨️ Keyboard Shortcuts

*   `Ctrl + B`: Toggle **Bold**
*   `Ctrl + I`: Toggle *Italic*
*   `Ctrl + K`: Insert Link
*   `Ctrl + S`: Save Document
*   `Ctrl + O`: Open Document
*   `Ctrl + 1` to `6`: Heading Level 1 to 6
*   Type `/` on a new line to open the **Slash Command Menu**!
"#;

#[allow(dead_code)]
pub const GUIDE_MD: &str = r#"# Markdown Syntax Quick Reference

A handy reference guide for CommonMark and GitHub Flavored Markdown (GFM).

---

## Headings

# Heading 1
## Heading 2
### Heading 3
#### Heading 4

---

## Emphasis

*Italic text* with asterisks or _underscores_.
**Bold text** with double asterisks.
***Bold and italic*** with triple asterisks.
~~Strikethrough~~ with double tildes.
`Inline code` with backticks.

---

## Blockquotes

> Markdown is intended to be as easy-to-read and easy-to-write as is feasible.
>
> Multiple paragraphs can be included in a blockquote.

---

## Lists

### Unordered
* Apple
* Banana
* Cherry

### Ordered
1. First step
2. Second step
3. Third step

---

## Links & Images

[Visit Tauri Homepage](https://tauri.app)

![MdTerm](https://picsum.photos/600/200)
"#;

#[allow(dead_code)]
pub const SAMPLE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Welcome to MdTerm HTML</title>
</head>
<body>
  <h1>Welcome to HTML Editing in MdTerm 🌐</h1>
  <p>MdTerm provides first-class support for opening, editing, viewing, and saving HTML files.</p>

  <h2>Features</h2>
  <ul>
    <li><strong>WYSIWYG Visual Editing</strong>: Edit HTML documents directly as rendered web pages.</li>
    <li><strong>Split & Source Modes</strong>: Switch to Split or Source view to inspect and edit raw markup.</li>
    <li><strong>Document Integrity</strong>: Keeps your <code>&lt;!DOCTYPE&gt;</code>, <code>&lt;head&gt;</code>, meta tags, and styles intact when saving.</li>
  </ul>

  <h2>Sample Table</h2>
  <table class="md-table">
    <thead>
      <tr><th>Format</th><th>Visual Mode</th><th>Source Mode</th></tr>
    </thead>
    <tbody>
      <tr><td>HTML (.html)</td><td>Supported</td><td>Supported</td></tr>
      <tr><td>Markdown (.md)</td><td>Supported</td><td>Supported</td></tr>
    </tbody>
  </table>
</body>
</html>
"#;
