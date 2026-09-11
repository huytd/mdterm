use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{KeyboardEvent, MouseEvent};

use crate::components::samples::WELCOME_MD;
use crate::components::{
    DocumentPreview, EditorHeader, FloatingControls, Modals, SourceEditor, SplitEditor,
    TerminalPane, WysiwygEditor,
};
use crate::html::{self, HtmlEnvelope};
use crate::markdown::{html_to_markdown, markdown_to_html};
use crate::state::{ActiveModal, EditorMode, EditorPosition, FindReplaceState, SlashMenuState, Theme};
use crate::tauri_bridge::{
    self, exec_editor_cmd, fit_terminal_session, focus_terminal_session, set_terminal_theme,
    window_find,
};

const THEME_STORAGE_KEY: &str = "mdterm_theme";

fn load_persisted_theme() -> Theme {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(THEME_STORAGE_KEY).ok().flatten())
        .and_then(|class_name| Theme::from_class_name(&class_name))
        .unwrap_or(Theme::Dark)
}

fn persist_theme(theme: Theme) {
    if let Some(storage) = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(THEME_STORAGE_KEY, theme.class_name());
    }
}

#[component]
pub fn App() -> impl IntoView {
    // 1. Core State
    let is_editor_open = RwSignal::new(false);
    let editor_position = RwSignal::new(EditorPosition::Right);
    let active_mode = RwSignal::new(EditorMode::Wysiwyg);
    let active_filename = RwSignal::new(String::from("Untitled.md"));
    let active_path = RwSignal::new(None::<String>);
    let active_content = RwSignal::new(WELCOME_MD.to_string());
    let is_dirty = RwSignal::new(false);
    let is_remote_doc = RwSignal::new(false);
    let current_theme = RwSignal::new(load_persisted_theme());

    let is_html_doc = Memo::new(move |_| {
        html::is_html_file(&active_filename.get(), &active_content.get())
    });

    // Persist and synchronize the terminal and native window chrome with the app theme.
    Effect::new(move |_| {
        let theme = current_theme.get();
        let theme_name = theme.class_name();
        persist_theme(theme);
        set_terminal_theme(theme_name);
        leptos::task::spawn_local(async move {
            let _ = tauri_bridge::set_window_theme(theme_name).await;
        });
    });

    // Load terminal config from ~/.config/mdterm/config.yml on app start
    leptos::task::spawn_local(async move {
        if let Ok(cfg) = tauri_bridge::get_terminal_config().await {
            tauri_bridge::apply_terminal_config(&cfg);
            if let Some(theme_val) = &cfg.theme {
                if let tauri_bridge::ThemeValue::Name(ref name) = theme_val {
                    if let Some(t) = Theme::from_name_or_class(name) {
                        current_theme.set(t);
                    }
                }
            }
        }
    });

    // Listen for live config changes emitted by background watcher
    Effect::new(move |_| {
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
                if let Ok(detail) = js_sys::Reflect::get(&ev, &"detail".into()) {
                    if let Ok(cfg) = serde_wasm_bindgen::from_value::<tauri_bridge::TerminalConfig>(detail) {
                        if let Some(theme_val) = &cfg.theme {
                            if let tauri_bridge::ThemeValue::Name(ref name) = theme_val {
                                if let Some(t) = Theme::from_name_or_class(name) {
                                    current_theme.set(t);
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let _ = win.add_event_listener_with_callback(
                "mdterm-config-changed",
                cb.as_ref().unchecked_ref(),
            );
            cb.forget();
        }
    });

    // Split ratio: width percentage of the editor pane (20% - 80%, default 50%)
    let split_ratio = RwSignal::new(50.0f64);
    let is_dragging = RwSignal::new(false);

    // Modals and menus
    let active_modal = RwSignal::new(ActiveModal::None);
    let find_replace = RwSignal::new(FindReplaceState::default());
    let slash_menu = RwSignal::new(SlashMenuState::default());

    // Listen for file open requests triggered from terminal (OSC 5337 / mdterm CLI / click)
    Effect::new(move |_| {
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
                if let Ok(detail) = js_sys::Reflect::get(&ev, &"detail".into()) {
                    let name = js_sys::Reflect::get(&detail, &"name".into())
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_else(|| "document.md".to_string());
                    let path = js_sys::Reflect::get(&detail, &"path".into())
                        .ok()
                        .and_then(|v| v.as_string())
                        .filter(|s| !s.is_empty());
                    let content = js_sys::Reflect::get(&detail, &"content".into())
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    let side = js_sys::Reflect::get(&detail, &"side".into())
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_else(|| "right".to_string());
                    let is_remote = js_sys::Reflect::get(&detail, &"is_remote".into())
                        .ok()
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    active_filename.set(name);
                    active_path.set(path.clone());
                    is_dirty.set(false);
                    is_remote_doc.set(is_remote);
                    editor_position.set(if side == "left" { EditorPosition::Left } else { EditorPosition::Right });
                    is_editor_open.set(true);
                    fit_terminal_session();

                    if !content.is_empty() {
                        active_content.set(content);
                    } else if let Some(p) = path {
                        let p_clone = p.clone();
                        leptos::task::spawn_local(async move {
                            if let Ok(disk_content) = tauri_bridge::read_file(&p_clone).await {
                                active_content.set(disk_content);
                            }
                        });
                    }
                }
            }) as Box<dyn FnMut(web_sys::CustomEvent)>);

            let _ = win.add_event_listener_with_callback("mdterm-open-file", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    });

    // Listen for file save events and close events from remote sessions
    Effect::new(move |_| {
        if let Some(win) = web_sys::window() {
            let cb_saved = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::CustomEvent| {
                is_dirty.set(false);
            }) as Box<dyn FnMut(web_sys::CustomEvent)>);
            let _ = win.add_event_listener_with_callback("mdterm-file-saved", cb_saved.as_ref().unchecked_ref());
            cb_saved.forget();

            let cb_closed = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::CustomEvent| {
                is_remote_doc.set(false);
            }) as Box<dyn FnMut(web_sys::CustomEvent)>);
            let _ = win.add_event_listener_with_callback("mdterm-remote-closed", cb_closed.as_ref().unchecked_ref());
            cb_closed.forget();
        }
    });

    // Open file passed via command line / file manager association
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(Some(file_path)) = tauri_bridge::get_cli_file().await {
                let name = std::path::Path::new(&file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "document.md".to_string());
                if let Ok(disk_content) = tauri_bridge::read_file(&file_path).await {
                    active_filename.set(name);
                    active_path.set(Some(file_path));
                    active_content.set(disk_content);
                    is_dirty.set(false);
                    is_remote_doc.set(false);
                    is_editor_open.set(true);
                    fit_terminal_session();
                }
            }
        });
    });

    // Handle content updates from editor
    let handle_content_change = Callback::new(move |new_text: String| {
        active_content.set(new_text);
        is_dirty.set(true);
    });

    // Save active document
    let save_active_document = move || {
        let content = active_content.get();
        let target_path = active_path.get().unwrap_or_else(|| active_filename.get());
        let is_remote = is_remote_doc.get();

        if is_remote {
            leptos::task::spawn_local(async move {
                if let Err(e) = tauri_bridge::send_remote_save(&target_path, &content).await {
                    tauri_bridge::show_toast(&format!("Remote save error: {}", e));
                }
            });
        } else {
            leptos::task::spawn_local(async move {
                match tauri_bridge::write_file(&target_path, &content).await {
                    Ok(_) => {
                        is_dirty.set(false);
                        active_path.set(Some(target_path.clone()));
                        let name = std::path::Path::new(&target_path)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or(target_path.clone());
                        active_filename.set(name.clone());
                        tauri_bridge::show_toast(&format!("Saved '{}'", name));
                    }
                    Err(e) => {
                        tauri_bridge::show_toast(&format!("Save failed: {}", e));
                    }
                }
            });
        }
    };

    // Open existing file
    let open_file_by_path = Callback::new(move |path: String| {
        let path_clone = path.clone();
        leptos::task::spawn_local(async move {
            if let Ok(content) = tauri_bridge::read_file(&path_clone).await {
                let name = std::path::Path::new(&path_clone)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_clone.clone());

                active_content.set(content);
                active_filename.set(name);
                active_path.set(Some(path_clone));
                is_dirty.set(false);
                is_remote_doc.set(false);
                is_editor_open.set(true);
                fit_terminal_session();
            }
        });
    });

    // New Document
    let handle_new_file = Callback::new(move |open_on_left: bool| {
        active_filename.set("Untitled.md".to_string());
        active_path.set(None);
        active_content.set("# Untitled Document\n\nStart typing here...".to_string());
        is_dirty.set(false);
        is_remote_doc.set(false);
        editor_position.set(if open_on_left { EditorPosition::Left } else { EditorPosition::Right });
        is_editor_open.set(true);
        fit_terminal_session();
    });

    // Close Editor (return to full-screen terminal)
    let handle_close_editor = Callback::new(move |_| {
        if is_remote_doc.get() {
            leptos::task::spawn_local(async move {
                tauri_bridge::close_remote_session().await;
            });
            is_remote_doc.set(false);
        }
        is_editor_open.set(false);
        fit_terminal_session();
        focus_terminal_session();
    });



    let handle_insert_table_confirm = Callback::new(move |(rows, cols): (usize, usize)| {
        let mut table_html = String::from("<table class=\"md-table\"><thead><tr>");
        for c in 1..=cols {
            table_html.push_str(&format!("<th>Header {}</th>", c));
        }
        table_html.push_str("</tr></thead><tbody>");
        for r in 1..=rows {
            table_html.push_str("<tr>");
            for c in 1..=cols {
                table_html.push_str(&format!("<td>Row {} Col {}</td>", r, c));
            }
            table_html.push_str("</tr>");
        }
        table_html.push_str("</tbody></table><p></p>");

        exec_editor_cmd("insertHTML", Some(&table_html));
    });

    let handle_insert_link_confirm = Callback::new(move |(url, text): (String, String)| {
        let display_text = if text.trim().is_empty() { &url } else { &text };
        let link_html = format!("<a href=\"{}\">{}</a>", url, display_text);
        exec_editor_cmd("insertHTML", Some(&link_html));
    });

    let handle_insert_image_confirm = Callback::new(move |(url, alt): (String, String)| {
        let img_html = format!("<img src=\"{}\" alt=\"{}\" />", url, alt);
        exec_editor_cmd("insertHTML", Some(&img_html));
    });

    // Slash command insertion
    let handle_slash_select = Callback::new(move |item_type: &'static str| {
        slash_menu.set(SlashMenuState::default());
        match item_type {
            "h1" => { exec_editor_cmd("formatBlock", Some("<h1>")); }
            "h2" => { exec_editor_cmd("formatBlock", Some("<h2>")); }
            "h3" => { exec_editor_cmd("formatBlock", Some("<h3>")); }
            "tasklist" => {
                exec_editor_cmd("insertHTML", Some("<ul class=\"task-list\"><li class=\"task-list-item\"><input type=\"checkbox\" class=\"md-task-checkbox\" /> Task</li></ul>"));
            }
            "bullet" => { exec_editor_cmd("insertUnorderedList", None); }
            "number" => { exec_editor_cmd("insertOrderedList", None); }
            "quote" => { exec_editor_cmd("formatBlock", Some("<blockquote>")); }
            "code" => {
                exec_editor_cmd("insertHTML", Some("<pre><code>// code here\n</code></pre><p></p>"));
            }
            "mermaid" => {
                let sample_mermaid = "<div class=\"code-block-wrapper mermaid-block-wrapper\" data-lang=\"mermaid\">\
                    <div class=\"code-block-header\" contenteditable=\"false\">\
                        <div class=\"code-lang-tag\">\
                            <svg viewBox=\"0 0 24 24\" width=\"13\" height=\"13\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" style=\"margin-right: 5px; vertical-align: -1px;\">\
                                <circle cx=\"6\" cy=\"6\" r=\"3\"></circle>\
                                <circle cx=\"6\" cy=\"18\" r=\"3\"></circle>\
                                <circle cx=\"18\" cy=\"12\" r=\"3\"></circle>\
                                <line x1=\"8.5\" y1=\"7.5\" x2=\"15.5\" y2=\"10.5\"></line>\
                                <line x1=\"8.5\" y1=\"16.5\" x2=\"15.5\" y2=\"13.5\"></line>\
                            </svg>\
                            <span>mermaid</span>\
                        </div>\
                        <div class=\"mermaid-view-switcher\">\
                            <button type=\"button\" class=\"mermaid-tab-btn active\" data-tab=\"diagram\" onclick=\"window._toggleMermaidView(this, 'diagram')\">Diagram</button>\
                            <button type=\"button\" class=\"mermaid-tab-btn\" data-tab=\"code\" onclick=\"window._toggleMermaidView(this, 'code')\">Code</button>\
                        </div>\
                        <button type=\"button\" class=\"code-copy-btn\" onclick=\"navigator.clipboard.writeText(this.closest('.code-block-wrapper').querySelector('code').innerText);this.innerText='Copied!';setTimeout(()=>this.innerText='Copy',1500)\">Copy</button>\
                    </div>\
                    <!-- MERMAID_PREVIEW_START -->\
                    <div class=\"mermaid-preview-container\" contenteditable=\"false\">\
                        <div class=\"mermaid-preview-target\">\
                            <div class=\"mermaid-loading\">Rendering diagram...</div>\
                        </div>\
                    </div>\
                    <!-- MERMAID_PREVIEW_END -->\
                    <pre class=\"mermaid-code-pre\" style=\"display: none;\"><code class=\"language-mermaid\">graph TD\n    A[Start] --&gt; B{Decision}\n    B --&gt;|Yes| C[Result 1]\n    B --&gt;|No| D[Result 2]</code></pre>\
                </div><p></p>";
                exec_editor_cmd("insertHTML", Some(sample_mermaid));
                tauri_bridge::render_mermaid_diagrams();
            }
            "table" => {
                handle_insert_table_confirm.run((3, 3));
            }
            "divider" => {
                exec_editor_cmd("insertHorizontalRule", None);
            }
            _ => {}
        }
    });

    // Export confirmation
    let handle_export_confirm = Callback::new(move |format_choice: &'static str| {
        let current_text = active_content.get();
        let base_name = active_filename.get();
        let is_html = is_html_doc.get();
        let clean_name = base_name
            .strip_suffix(".html")
            .or_else(|| base_name.strip_suffix(".htm"))
            .or_else(|| base_name.strip_suffix(".xhtml"))
            .or_else(|| base_name.strip_suffix(".md"))
            .or_else(|| base_name.strip_suffix(".markdown"))
            .unwrap_or(&base_name);

        match format_choice {
            "html" => {
                let standalone_html = if is_html {
                    let env = HtmlEnvelope::parse(&current_text);
                    env.ensure_full_document(clean_name)
                } else {
                    let body_html = markdown_to_html(&current_text, false);
                    format!(
                        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{}</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; max-width: 860px; margin: 40px auto; padding: 0 20px; color: #1e293b; background: #ffffff; }}
    h1, h2, h3, h4 {{ color: #0f172a; margin-top: 1.5em; margin-bottom: 0.5em; }}
    h1 {{ border-bottom: 2px solid #e2e8f0; padding-bottom: 8px; }}
    code {{ background: #f1f5f9; padding: 2px 6px; border-radius: 4px; font-family: monospace; font-size: 0.9em; }}
    pre {{ background: #0f172a; color: #f8fafc; padding: 16px; border-radius: 8px; overflow-x: auto; }}
    blockquote {{ border-left: 4px solid #3b82f6; margin: 1.5em 0; padding-left: 16px; color: #475569; font-style: italic; }}
    table {{ width: 100%; border-collapse: collapse; margin: 1.5em 0; }}
    th, td {{ border: 1px solid #cbd5e1; padding: 10px 14px; text-align: left; }}
    th {{ background: #f8fafc; font-weight: 600; }}
    .task-list {{ list-style-type: none; padding-left: 0; }}
    img {{ max-width: 100%; height: auto; border-radius: 6px; }}
    .mermaid {{ display: flex; justify-content: center; margin: 24px 0; background: #f8fafc; padding: 16px; border-radius: 8px; border: 1px solid #e2e8f0; }}
  </style>
  <script type="module">
    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
    mermaid.initialize({{ startOnLoad: true }});
  </script>
</head>
<body>
{}
</body>
</html>"#,
                        clean_name, body_html
                    )
                };
                tauri_bridge::triggerDownload(&format!("{}.html", clean_name), &standalone_html, "text/html");
            }
            "print" => {
                tauri_bridge::openPrintDialog();
            }
            _ => {
                let md_content = if is_html {
                    html_to_markdown(&current_text)
                } else {
                    current_text
                };
                tauri_bridge::triggerDownload(&format!("{}.md", clean_name), &md_content, "text/markdown");
            }
        }
    });

    // Find and Replace logic
    let handle_find_next = Callback::new(move |_| {
        let query = find_replace.get().search_query;
        if !query.is_empty() {
            window_find(&query, find_replace.get().match_case, false);
        }
    });

    let handle_find_prev = Callback::new(move |_| {
        let query = find_replace.get().search_query;
        if !query.is_empty() {
            window_find(&query, find_replace.get().match_case, true);
        }
    });

    let handle_replace_one = Callback::new(move |_| {
        let query = find_replace.get().search_query;
        let replace = find_replace.get().replace_query;
        if query.is_empty() { return; }

        let current_text = active_content.get();
        if let Some(pos) = current_text.find(&query) {
            let mut new_text = current_text.clone();
            new_text.replace_range(pos..pos + query.len(), &replace);
            active_content.set(new_text.clone());
            handle_content_change.run(new_text);
        }
    });

    let handle_replace_all = Callback::new(move |_| {
        let query = find_replace.get().search_query;
        let replace = find_replace.get().replace_query;
        if query.is_empty() { return; }

        let current_text = active_content.get();
        let new_text = current_text.replace(&query, &replace);
        active_content.set(new_text.clone());
        handle_content_change.run(new_text);
    });

    // Global keyboard shortcut listener
    let on_window_keydown = move |ev: KeyboardEvent| {
        let is_ctrl = ev.ctrl_key() || ev.meta_key();
        let is_alt = ev.alt_key();
        let key = ev.key().to_lowercase();

        if is_alt {
            match key.as_str() {
                "1" => {
                    ev.prevent_default();
                    active_mode.set(EditorMode::Wysiwyg);
                }
                "2" => {
                    ev.prevent_default();
                    active_mode.set(EditorMode::Split);
                }
                "3" => {
                    ev.prevent_default();
                    active_mode.set(EditorMode::Source);
                }
                _ => {}
            }
        }

        if is_ctrl {
            match key.as_str() {
                "s" => {
                    ev.prevent_default();
                    save_active_document();
                }
                "o" => {
                    ev.prevent_default();
                    if ev.meta_key() {
                        editor_position.set(EditorPosition::Left);
                    } else {
                        editor_position.set(EditorPosition::Right);
                    }
                    active_modal.set(ActiveModal::OpenFile);
                }
                "n" => {
                    ev.prevent_default();
                    handle_new_file.run(ev.meta_key());
                }
                "f" => {
                    ev.prevent_default();
                    find_replace.update(|s| s.is_open = !s.is_open);
                }
                "w" => {
                    if is_editor_open.get() {
                        ev.prevent_default();
                        handle_close_editor.run(());
                    }
                }
                "t" => {
                    ev.prevent_default();
                    current_theme.update(|t| *t = t.next());
                }
                "m" => {
                    ev.prevent_default();
                    active_mode.update(|m| *m = match *m {
                        EditorMode::Wysiwyg => EditorMode::Split,
                        EditorMode::Split => EditorMode::Source,
                        EditorMode::Source => EditorMode::Wysiwyg,
                    });
                }
                _ => {}
            }
        }
    };

    // Resizing mouse handlers
    let on_mouse_move = move |ev: MouseEvent| {
        if is_dragging.get() {
            if let Some(win) = web_sys::window() {
                let inner_width = win.inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(1200.0);
                let client_x = ev.client_x() as f64;
                let pct = (client_x / inner_width) * 100.0;
                let new_split = if editor_position.get() == EditorPosition::Left {
                    pct.clamp(20.0, 80.0)
                } else {
                    (100.0 - pct).clamp(20.0, 80.0)
                };
                split_ratio.set(new_split);
                fit_terminal_session();
            }
        }
    };

    let on_mouse_up = move |_| {
        if is_dragging.get() {
            is_dragging.set(false);
            fit_terminal_session();
        }
    };

    view! {
        <div
            class=move || format!("app-root {}", current_theme.get().class_name())
            on:mousemove=on_mouse_move
            on:mouseup=on_mouse_up
            on:keydown=on_window_keydown
            tabindex="-1"
        >
            <div class=move || {
                if editor_position.get() == EditorPosition::Left {
                    "app-workspace pos-editor-left"
                } else {
                    "app-workspace pos-editor-right"
                }
            }>
                {move || if is_editor_open.get() {
                    view! {
                        <div
                            class="workspace-pane editor-pane"
                            style=move || format!("width: {}%;", split_ratio.get())
                        >
                            <EditorHeader
                                active_filename=active_filename.into()
                                is_dirty=is_dirty.into()
                                is_remote=is_remote_doc.into()
                                is_html=is_html_doc.into()
                                active_mode=active_mode
                                current_theme=current_theme
                                editor_position=editor_position
                                on_close_editor=handle_close_editor
                                on_save_file=Callback::new(move |_| save_active_document())
                                on_export=Callback::new(move |_| active_modal.set(ActiveModal::Export))
                            />

                            {move || match active_mode.get() {
                                EditorMode::Wysiwyg => {
                                    if is_html_doc.get() {
                                        view! {
                                            <DocumentPreview
                                                content=active_content.into()
                                                is_html=is_html_doc.into()
                                                on_change=handle_content_change
                                            />
                                        }.into_any()
                                    } else {
                                        view! {
                                            <WysiwygEditor
                                                content=active_content
                                                is_html=is_html_doc.into()
                                                on_change=handle_content_change
                                                slash_menu=slash_menu
                                            />
                                        }.into_any()
                                    }
                                },
                                EditorMode::Split => view! {
                                    <SplitEditor
                                        content=active_content
                                        is_html=is_html_doc.into()
                                        on_change=handle_content_change
                                    />
                                }.into_any(),
                                EditorMode::Source => view! {
                                    <SourceEditor
                                        content=active_content
                                        is_html=is_html_doc.into()
                                        on_change=handle_content_change
                                    />
                                }.into_any(),
                            }}
                        </div>

                        <div
                            class="workspace-divider"
                            title="Drag to resize, double-click to reset (50/50)"
                            on:mousedown=move |_| is_dragging.set(true)
                            on:dblclick=move |_| {
                                split_ratio.set(50.0);
                                fit_terminal_session();
                            }
                        >
                            <div class="divider-line"></div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <FloatingControls
                            current_theme=current_theme
                            on_new_file=handle_new_file
                            on_open_file=Callback::new(move |open_on_left: bool| {
                                editor_position.set(if open_on_left { EditorPosition::Left } else { EditorPosition::Right });
                                active_modal.set(ActiveModal::OpenFile);
                            })
                        />
                    }.into_any()
                }}

                <div
                    class=move || if is_editor_open.get() {
                        "workspace-pane terminal-pane-container"
                    } else {
                        "workspace-pane terminal-pane-container full-width"
                    }
                    style=move || if is_editor_open.get() {
                        format!("width: {}%;", 100.0 - split_ratio.get())
                    } else {
                        "width: 100%;".to_string()
                    }
                >
                    <TerminalPane />
                </div>
            </div>

            <Modals
                active_modal=active_modal
                find_replace=find_replace
                slash_menu=slash_menu
                on_insert_link_confirm=handle_insert_link_confirm
                on_insert_image_confirm=handle_insert_image_confirm
                on_insert_table_confirm=handle_insert_table_confirm
                on_export_confirm=handle_export_confirm
                on_new_file_confirm=Callback::new(move |name: String| {
                    let is_html = html::is_html_file(&name, "");
                    active_filename.set(name.clone());
                    active_path.set(Some(name.clone()));
                    if is_html {
                        let clean = name
                            .strip_suffix(".html")
                            .or_else(|| name.strip_suffix(".htm"))
                            .or_else(|| name.strip_suffix(".xhtml"))
                            .unwrap_or(&name);
                        active_content.set(format!(
                            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{}</title>
</head>
<body>
  <h1>{}</h1>
  <p>Start typing here...</p>
</body>
</html>"#,
                            clean, clean
                        ));
                    } else {
                        active_content.set("# New File\n\n".to_string());
                    }
                    is_dirty.set(false);
                    is_remote_doc.set(false);
                    is_editor_open.set(true);
                    fit_terminal_session();
                })
                on_open_file_confirm=open_file_by_path
                on_find_next=handle_find_next
                on_find_prev=handle_find_prev
                on_replace_one=handle_replace_one
                on_replace_all=handle_replace_all
                on_slash_select=handle_slash_select
            />
        </div>
    }
}
