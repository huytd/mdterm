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
