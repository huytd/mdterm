use leptos::prelude::*;
use web_sys::KeyboardEvent;

use crate::components::samples::{GUIDE_MD, WELCOME_MD};
use crate::components::{
    Header, Modals, Sidebar, SplitEditor, StatusBar, Toolbar, WysiwygEditor,
};
use crate::markdown::{extract_outline, markdown_to_html};
use crate::state::{
    ActiveModal, DocumentStats, DocumentTab, EditorMode, FileEntry, FindReplaceState,
    SidebarTab, SlashMenuState, Theme,
};
use crate::tauri_bridge::{self, exec_editor_cmd, window_find};

#[component]
pub fn App() -> impl IntoView {
    // 1. Core State
    let initial_tabs = vec![
        DocumentTab {
            id: "tab-1".to_string(),
            title: "Welcome.md".to_string(),
            path: Some("Welcome.md".to_string()),
            content: WELCOME_MD.to_string(),
            is_dirty: false,
        },
        DocumentTab {
            id: "tab-2".to_string(),
            title: "Syntax-Guide.md".to_string(),
            path: Some("Syntax-Guide.md".to_string()),
            content: GUIDE_MD.to_string(),
            is_dirty: false,
        },
    ];

    let tabs = RwSignal::new(initial_tabs);
    let active_tab_idx = RwSignal::new(0usize);
    let active_mode = RwSignal::new(EditorMode::Wysiwyg);
    let current_theme = RwSignal::new(Theme::Dark);

    let sidebar_open = RwSignal::new(true);
    let sidebar_tab = RwSignal::new(SidebarTab::Files);
    let files = RwSignal::new(Vec::<FileEntry>::new());
    let current_dir = RwSignal::new(String::from("Documents"));

    let active_modal = RwSignal::new(ActiveModal::None);
    let find_replace = RwSignal::new(FindReplaceState::default());
    let slash_menu = RwSignal::new(SlashMenuState::default());

    // 2. Active document content signal
    let active_content = RwSignal::new(WELCOME_MD.to_string());

    // When tab index changes, update active_content
    Effect::new(move |_| {
        let idx = active_tab_idx.get();
        let tab_list = tabs.get();
        if let Some(tab) = tab_list.get(idx) {
            active_content.set(tab.content.clone());
        }
    });

    // 3. Derived Outline and Stats
    let outline = Memo::new(move |_| {
        let md = active_content.get();
        extract_outline(&md)
    });

    let stats = Memo::new(move |_| {
        let md = active_content.get();
        DocumentStats::compute(&md)
    });

    // Load initial files from current directory
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(dir) = tauri_bridge::get_current_dir().await {
                current_dir.set(dir.clone());
                if let Ok(list) = tauri_bridge::read_dir(&dir).await {
                    files.set(list);
                }
            }
        });
    });

    // Handle content updates from editor
    let handle_content_change = Callback::new(move |new_text: String| {
        let idx = active_tab_idx.get();
        tabs.update(|list| {
            if let Some(tab) = list.get_mut(idx) {
                tab.content = new_text;
                tab.is_dirty = true;
            }
        });
    });

    // Save active document
    let save_active_document = move || {
        let idx = active_tab_idx.get();
        let (path_opt, content) = {
            let list = tabs.get();
            if let Some(tab) = list.get(idx) {
                (tab.path.clone(), tab.content.clone())
            } else {
                return;
            }
        };

        let target_path = path_opt.unwrap_or_else(|| format!("Untitled-{}.md", idx + 1));
        leptos::task::spawn_local(async move {
            if tauri_bridge::write_file(&target_path, &content).await.is_ok() {
                tabs.update(|list| {
                    if let Some(tab) = list.get_mut(idx) {
                        tab.is_dirty = false;
                        if tab.path.is_none() {
                            tab.path = Some(target_path.clone());
                            tab.title = target_path.clone();
                        }
                    }
                });

                // Refresh files
                let dir = current_dir.get();
                if let Ok(list) = tauri_bridge::read_dir(&dir).await {
                    files.set(list);
                }
            }
        });
    };

    // Open existing file
    let open_file_by_path = Callback::new(move |path: String| {
        let path_clone = path.clone();
        leptos::task::spawn_local(async move {
            if let Ok(content) = tauri_bridge::read_file(&path_clone).await {
                let filename = std::path::Path::new(&path_clone)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_clone.clone());

                let mut found_idx = None;
                let list = tabs.get();
                for (i, t) in list.iter().enumerate() {
                    if t.path.as_deref() == Some(&path_clone) {
                        found_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = found_idx {
                    active_tab_idx.set(idx);
                } else {
                    let new_tab = DocumentTab {
                        id: format!("tab-{}", list.len() + 1),
                        title: filename,
                        path: Some(path_clone),
                        content,
                        is_dirty: false,
                    };
                    tabs.update(|l| l.push(new_tab));
                    active_tab_idx.set(tabs.get().len() - 1);
                }
            }
        });
    });

    // Delete file
    let delete_file_by_path = Callback::new(move |path: String| {
        leptos::task::spawn_local(async move {
            let _ = tauri_bridge::delete_file(&path).await;
            let dir = current_dir.get();
            if let Ok(list) = tauri_bridge::read_dir(&dir).await {
                files.set(list);
            }
        });
    });

    // New Tab
    let handle_new_tab = Callback::new(move |_| {
        let list_len = tabs.get().len();
        let new_tab = DocumentTab {
            id: format!("tab-{}", list_len + 1),
            title: format!("Untitled-{}.md", list_len + 1),
            path: None,
            content: "# Untitled Document\n\nStart typing here...".to_string(),
            is_dirty: false,
        };
        tabs.update(|l| l.push(new_tab));
        active_tab_idx.set(list_len);
    });

    // Close Tab
    let handle_close_tab = Callback::new(move |close_idx: usize| {
        tabs.update(|list| {
            if list.len() > 1 {
                list.remove(close_idx);
            }
        });
        let cur = active_tab_idx.get();
        if cur >= tabs.get().len() {
            active_tab_idx.set(tabs.get().len() - 1);
        }
    });

    // Load sample template
    let handle_load_template = Callback::new(move |template: &'static str| {
        let (title, content) = match template {
            "guide" => ("Syntax-Guide.md", GUIDE_MD),
            _ => ("Welcome.md", WELCOME_MD),
        };
        let list_len = tabs.get().len();
        let new_tab = DocumentTab {
            id: format!("tab-{}", list_len + 1),
            title: title.to_string(),
            path: None,
            content: content.to_string(),
            is_dirty: true,
        };
        tabs.update(|l| l.push(new_tab));
        active_tab_idx.set(list_len);
    });

    // Jump to heading anchor
    let handle_jump_to_heading = Callback::new(move |anchor_id: String| {
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(el) = doc.get_element_by_id(&anchor_id) {
                    el.scroll_into_view();
                }
            }
        }
    });

    // Toolbar formatting commands
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

    let handle_insert_hr = Callback::new(move |_| {
        exec_editor_cmd("insertHorizontalRule", None);
    });

    let handle_undo = Callback::new(move |_| {
        exec_editor_cmd("undo", None);
    });

    let handle_redo = Callback::new(move |_| {
        exec_editor_cmd("redo", None);
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
        let idx = active_tab_idx.get();
        let base_name = tabs.get().get(idx).map(|t| t.title.clone()).unwrap_or_else(|| "document".to_string());
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

    // New file dialog confirm
    let handle_new_file_confirm = Callback::new(move |filename: String| {
        let dir = current_dir.get();
        let full_path = format!("{}/{}", dir, filename);
        let path_clone = full_path.clone();
        leptos::task::spawn_local(async move {
            if tauri_bridge::create_file(&path_clone).await.is_ok() {
                open_file_by_path.run(path_clone);
            }
        });
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
                    sidebar_open.set(true);
                    sidebar_tab.set(SidebarTab::Files);
                }
                "n" => {
                    ev.prevent_default();
                    handle_new_tab.run(());
                }
                "w" => {
                    ev.prevent_default();
                    handle_close_tab.run(active_tab_idx.get());
                }
                "f" => {
                    ev.prevent_default();
                    find_replace.update(|s| s.is_open = !s.is_open);
                }
                "1" => {
                    if ev.alt_key() {
                        ev.prevent_default();
                        active_mode.set(EditorMode::Wysiwyg);
                    }
                }
                "2" => {
                    if ev.alt_key() {
                        ev.prevent_default();
                        active_mode.set(EditorMode::Split);
                    }
                }
                "3" => {
                    if ev.alt_key() {
                        ev.prevent_default();
                        active_mode.set(EditorMode::Source);
                    }
                }
                _ => {}
            }
        }
    };

    let active_is_dirty = Signal::derive(move || {
        let idx = active_tab_idx.get();
        tabs.get().get(idx).map(|t| t.is_dirty).unwrap_or(false)
    });

    let active_path = Signal::derive(move || {
        let idx = active_tab_idx.get();
        tabs.get().get(idx).and_then(|t| t.path.clone())
    });

    view! {
        <div
            class=move || format!("app-root {}", current_theme.get().class_name())
            on:keydown=on_window_keydown
            tabindex="0"
        >
            <Header
                tabs=tabs
                active_tab_idx=active_tab_idx
                active_mode=active_mode
                current_theme=current_theme
                sidebar_open=sidebar_open
                on_new_tab=handle_new_tab
                on_close_tab=handle_close_tab
                on_open_file=Callback::new(move |_| {
                    sidebar_open.set(true);
                    sidebar_tab.set(SidebarTab::Files);
                })
                on_save_file=Callback::new(move |_| save_active_document())
                on_export=Callback::new(move |_| active_modal.set(ActiveModal::Export))
                on_help=Callback::new(move |_| active_modal.set(ActiveModal::Help))
            />

            <Toolbar
                on_format=handle_format
                on_format_block=handle_format_block
                on_insert_table=Callback::new(move |_| active_modal.set(ActiveModal::InsertTable))
                on_insert_link=Callback::new(move |_| active_modal.set(ActiveModal::InsertLink))
                on_insert_image=Callback::new(move |_| active_modal.set(ActiveModal::InsertImage))
                on_insert_hr=handle_insert_hr
                on_find_replace=Callback::new(move |_| find_replace.update(|s| s.is_open = !s.is_open))
                on_undo=handle_undo
                on_redo=handle_redo
            />

            <div class="app-workspace">
                <Sidebar
                    is_open=sidebar_open
                    current_tab=sidebar_tab
                    files=files
                    current_dir=current_dir
                    outline=outline.into()
                    stats=stats.into()
                    on_open_file=open_file_by_path
                    on_new_file=Callback::new(move |_| active_modal.set(ActiveModal::NewFile))
                    on_delete_file=delete_file_by_path
                    on_load_template=handle_load_template
                    on_jump_to_heading=handle_jump_to_heading
                />

                <main class="app-editor-area">
                    {move || match active_mode.get() {
                        EditorMode::Wysiwyg => view! {
                            <WysiwygEditor
                                content=active_content
                                on_change=handle_content_change
                                slash_menu=slash_menu
                            />
                        }.into_any(),

                        EditorMode::Split => view! {
                            <SplitEditor
                                content=active_content
                                on_change=handle_content_change
                            />
                        }.into_any(),

                        EditorMode::Source => view! {
                            <div class="source-mode-container">
                                <crate::components::SourceEditor
                                    content=active_content
                                    on_change=handle_content_change
                                />
                            </div>
                        }.into_any(),
                    }}
                </main>
            </div>

            <StatusBar
                stats=stats
                mode=active_mode
                is_dirty=active_is_dirty
                file_path=active_path
            />

            <Modals
                active_modal=active_modal
                find_replace=find_replace
                slash_menu=slash_menu
                on_insert_link_confirm=handle_insert_link_confirm
                on_insert_image_confirm=handle_insert_image_confirm
                on_insert_table_confirm=handle_insert_table_confirm
                on_export_confirm=handle_export_confirm
                on_new_file_confirm=handle_new_file_confirm
                on_find_next=handle_find_next
                on_find_prev=handle_find_prev
                on_replace_one=handle_replace_one
                on_replace_all=handle_replace_all
                on_slash_select=handle_slash_select
            />
        </div>
    }
}
