use leptos::prelude::*;
use web_sys::{KeyboardEvent, MouseEvent};

use crate::components::samples::WELCOME_MD;
use crate::components::{Header, Modals, TerminalPane, WysiwygEditor};
use crate::markdown::markdown_to_html;
use crate::state::{ActiveModal, FindReplaceState, SlashMenuState, Theme};
use crate::tauri_bridge::{self, exec_editor_cmd, fit_terminal_session, window_find};

#[component]
pub fn App() -> impl IntoView {
    // 1. Core State
    let active_filename = RwSignal::new(String::from("Welcome.md"));
    let active_path = RwSignal::new(Some(String::from("Welcome.md")));
    let active_content = RwSignal::new(WELCOME_MD.to_string());
    let is_dirty = RwSignal::new(false);
    let current_theme = RwSignal::new(Theme::Dark);

    // Split ratio between Editor (left) and Terminal (right)
    let split_ratio = RwSignal::new(50.0f64);
    let is_dragging = RwSignal::new(false);

    // Modals and menus
    let active_modal = RwSignal::new(ActiveModal::None);
    let find_replace = RwSignal::new(FindReplaceState::default());
    let slash_menu = RwSignal::new(SlashMenuState::default());

    // Handle content updates from editor
    let handle_content_change = Callback::new(move |new_text: String| {
        active_content.set(new_text);
        is_dirty.set(true);
    });

    // Save active document
    let save_active_document = move || {
        let content = active_content.get();
        let target_path = active_path.get().unwrap_or_else(|| active_filename.get());

        leptos::task::spawn_local(async move {
            if tauri_bridge::write_file(&target_path, &content).await.is_ok() {
                is_dirty.set(false);
                active_path.set(Some(target_path.clone()));
                let name = std::path::Path::new(&target_path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or(target_path);
                active_filename.set(name);
            }
        });
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
            }
        });
    });

    // New Document
    let handle_new_file = Callback::new(move |_| {
        active_filename.set("Untitled.md".to_string());
        active_path.set(None);
        active_content.set("# Untitled Document\n\nStart typing here...".to_string());
        is_dirty.set(false);
    });

    // Toolbar / Shortcut formatting commands
    let handle_format = Callback::new(move |cmd: &'static str| {
        match cmd {
            "tasklist" => {
                exec_editor_cmd(
                    "insertHTML",
                    Some("<ul class=\"task-list\"><li class=\"task-list-item\"><input type=\"checkbox\" class=\"md-task-checkbox\" /> Task</li></ul>"),
                );
            }
            "code" => {
                exec_editor_cmd("insertHTML", Some("<code>code</code>"));
            }
            _ => {
                exec_editor_cmd(cmd, None);
            }
        }
    });

    let handle_format_block = Callback::new(move |block: &'static str| {
        let tag = format!("<{}>", block);
        exec_editor_cmd("formatBlock", Some(&tag));
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
        let md = active_content.get();
        let base_name = active_filename.get();
        let clean_name = base_name.trim_end_matches(".md");

        match format_choice {
            "html" => {
                let body_html = markdown_to_html(&md, false);
                let standalone_html = format!(
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
  </style>
</head>
<body>
{}
</body>
</html>"#,
                    clean_name, body_html
                );
                tauri_bridge::triggerDownload(&format!("{}.html", clean_name), &standalone_html, "text/html");
            }
            "print" => {
                tauri_bridge::openPrintDialog();
            }
            _ => {
                tauri_bridge::triggerDownload(&format!("{}.md", clean_name), &md, "text/markdown");
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
        let key = ev.key().to_lowercase();

        if is_ctrl {
            match key.as_str() {
                "s" => {
                    ev.prevent_default();
                    save_active_document();
                }
                "o" => {
                    ev.prevent_default();
                    active_modal.set(ActiveModal::OpenFile);
                }
                "n" => {
                    ev.prevent_default();
                    handle_new_file.run(());
                }
                "f" => {
                    ev.prevent_default();
                    find_replace.update(|s| s.is_open = !s.is_open);
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
                let clamped = pct.clamp(20.0, 80.0);
                split_ratio.set(clamped);
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
            tabindex="0"
        >
            <Header
                active_filename=active_filename.into()
                is_dirty=is_dirty.into()
                current_theme=current_theme
                on_new_file=handle_new_file
                on_open_file=Callback::new(move |_| active_modal.set(ActiveModal::OpenFile))
                on_save_file=Callback::new(move |_| save_active_document())
                on_export=Callback::new(move |_| active_modal.set(ActiveModal::Export))
                on_format=handle_format
                on_format_block=handle_format_block
            />

            <div class="app-workspace">
                <div
                    class="workspace-pane editor-pane"
                    style=move || format!("width: {}%;", split_ratio.get())
                >
                    <WysiwygEditor
                        content=active_content
                        on_change=handle_content_change
                        slash_menu=slash_menu
                    />
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

                <div
                    class="workspace-pane terminal-pane-container"
                    style=move || format!("width: {}%;", 100.0 - split_ratio.get())
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
                    active_filename.set(name.clone());
                    active_path.set(Some(name));
                    active_content.set("# New File\n\n".to_string());
                    is_dirty.set(false);
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
