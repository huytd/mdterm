use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent, MouseEvent};
use crate::html::{clean_html_editor_output, HtmlEnvelope};
use crate::markdown::{html_to_markdown, markdown_to_html};
use crate::state::SlashMenuState;
use crate::tauri_bridge::{exec_editor_cmd, get_cursor_pos};

#[component]
pub fn WysiwygEditor(
    content: RwSignal<String>,
    #[prop(optional)] is_html: Option<Signal<bool>>,
    on_change: Callback<String>,
    slash_menu: RwSignal<SlashMenuState>,
) -> impl IntoView {
    let is_html_sig = is_html.unwrap_or_else(|| Signal::derive(|| false));
    let editor_ref = NodeRef::<leptos::html::Div>::new();
    let is_internal_update = StoredValue::new(false);

    // Synchronize HTML when external content changes
    Effect::new(move |_| {
        let text = content.get();
        let is_h = is_html_sig.get();
        if is_internal_update.get_value() {
            is_internal_update.set_value(false);
            return;
        }

        if let Some(el) = editor_ref.get() {
            let raw_el: &HtmlElement = el.as_ref();
            if is_h {
                let env = HtmlEnvelope::parse(&text);
                raw_el.set_inner_html(&env.body);
            } else {
                let rendered_html = markdown_to_html(&text, true);
                raw_el.set_inner_html(&rendered_html);
            }
            crate::tauri_bridge::render_mermaid_diagrams();
        }
    });

    let handle_input = move |_: web_sys::Event| {
        if let Some(el) = editor_ref.get() {
            let raw_el: &HtmlElement = el.as_ref();
            let html = raw_el.inner_html();

            is_internal_update.set_value(true);
            if is_html_sig.get() {
                let clean_body = clean_html_editor_output(&html);
                let current_text = content.get_untracked();
                let env = HtmlEnvelope::parse(&current_text);
                let full_html = env.reassemble(&clean_body);
                content.set(full_html.clone());
                on_change.run(full_html);
            } else {
                let md = html_to_markdown(&html);
                content.set(md.clone());
                on_change.run(md);
            }
        }
    };

    let handle_click = move |ev: MouseEvent| {
        if let Some(target) = ev.target() {
            if let Ok(input_el) = target.dyn_into::<web_sys::HtmlInputElement>() {
                if input_el.class_name().contains("md-task-checkbox") {
                    if input_el.checked() {
                        let _ = input_el.set_attribute("checked", "");
                    } else {
                        let _ = input_el.remove_attribute("checked");
                    }

                    if let Some(el) = editor_ref.get() {
                        let raw_el: &HtmlElement = el.as_ref();
                        let html = raw_el.inner_html();

                        is_internal_update.set_value(true);
                        if is_html_sig.get() {
                            let clean_body = clean_html_editor_output(&html);
                            let current_text = content.get_untracked();
                            let env = HtmlEnvelope::parse(&current_text);
                            let full_html = env.reassemble(&clean_body);
                            content.set(full_html.clone());
                            on_change.run(full_html);
                        } else {
                            let md = html_to_markdown(&html);
                            content.set(md.clone());
                            on_change.run(md);
                        }
                    }
                }
            }
        }
    };

    let handle_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();

        if key == "/" {
            let (x, y) = get_cursor_pos();
            slash_menu.set(SlashMenuState {
                is_open: true,
                query: String::new(),
                selected_index: 0,
                x,
                y,
            });
        } else if key == "Escape" {
            slash_menu.update(|s| s.is_open = false);
        }

        if key == " " {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(selection)) = win.get_selection() {
                    if selection.range_count() > 0 {
                        if let Ok(range) = selection.get_range_at(0) {
                            if let Ok(node) = range.start_container() {
                                let text = node.text_content().unwrap_or_default();
                                let trimmed = text.trim();

                                let mut handled = false;
                                if trimmed == "#" {
                                    exec_editor_cmd("formatBlock", Some("<h1>"));
                                    handled = true;
                                } else if trimmed == "##" {
                                    exec_editor_cmd("formatBlock", Some("<h2>"));
                                    handled = true;
                                } else if trimmed == "###" {
                                    exec_editor_cmd("formatBlock", Some("<h3>"));
                                    handled = true;
                                } else if trimmed == "####" {
                                    exec_editor_cmd("formatBlock", Some("<h4>"));
                                    handled = true;
                                } else if trimmed == ">" {
                                    exec_editor_cmd("formatBlock", Some("<blockquote>"));
                                    handled = true;
                                } else if trimmed == "-" || trimmed == "*" {
                                    exec_editor_cmd("insertUnorderedList", None);
                                    handled = true;
                                } else if trimmed == "1." {
                                    exec_editor_cmd("insertOrderedList", None);
                                    handled = true;
                                }

                                if handled {
                                    ev.prevent_default();
                                    node.set_text_content(Some(""));
                                }
                            }
                        }
                    }
                }
            }
        }

        if key == "Tab" {
            ev.prevent_default();
            if ev.shift_key() {
                exec_editor_cmd("outdent", None);
            } else {
                exec_editor_cmd("indent", None);
            }
        }
    };

    let handle_paste = move |ev: web_sys::Event| {
        if let Ok(clip_ev) = ev.dyn_into::<web_sys::ClipboardEvent>() {
            if let Some(clipboard) = clip_ev.clipboard_data() {
                let is_h = is_html_sig.get();
                if !is_h {
                    let html_data = clipboard.get_data("text/html").unwrap_or_default();
                    let plain_data = clipboard.get_data("text/plain").unwrap_or_default();

                if html_data.trim().is_empty() && !plain_data.is_empty() {
                    let has_markdown_or_multiline = plain_data.contains('\n')
                        || plain_data.starts_with("# ")
                        || plain_data.starts_with("## ")
                        || plain_data.starts_with("### ")
                        || plain_data.starts_with("- ")
                        || plain_data.starts_with("* ")
                        || plain_data.starts_with("> ")
                        || plain_data.starts_with("```")
                        || plain_data.contains("```")
                        || plain_data.contains("| ");

                    if has_markdown_or_multiline {
                        clip_ev.prevent_default();
                        let rendered = markdown_to_html(&plain_data, true);
                        exec_editor_cmd("insertHTML", Some(&rendered));
                        crate::tauri_bridge::render_mermaid_diagrams();

                        if let Some(el) = editor_ref.get() {
                            let raw_el: &HtmlElement = el.as_ref();
                            let html = raw_el.inner_html();
                            is_internal_update.set_value(true);
                            let md = html_to_markdown(&html);
                            content.set(md.clone());
                            on_change.run(md);
                        }
                    }
                }
            }
        }
    }
};

    view! {
        <div class="wysiwyg-container">
            <div
                node_ref=editor_ref
                class="wysiwyg-surface"
                contenteditable="true"
                spellcheck="false"
                attr:data-placeholder=move || if is_html_sig.get() { "Start typing HTML content or press '/' for commands..." } else { "Start typing or press '/' for commands..." }
                on:input=handle_input
                on:paste=handle_paste
                on:click=handle_click
                on:keydown=handle_keydown
            ></div>
        </div>
    }
}
