use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent, MouseEvent};
use crate::markdown::{html_to_markdown, markdown_to_html};
use crate::state::SlashMenuState;
use crate::tauri_bridge::{exec_editor_cmd, get_cursor_pos};

#[component]
pub fn WysiwygEditor(
    content: RwSignal<String>,
    on_change: Callback<String>,
    slash_menu: RwSignal<SlashMenuState>,
) -> impl IntoView {
    let editor_ref = NodeRef::<leptos::html::Div>::new();
    let is_internal_update = StoredValue::new(false);

    // Synchronize HTML when external content changes
    Effect::new(move |_| {
        let md = content.get();
        if is_internal_update.get_value() {
            is_internal_update.set_value(false);
            return;
        }

        if let Some(el) = editor_ref.get() {
            let rendered_html = markdown_to_html(&md, true);
            let raw_el: &HtmlElement = el.as_ref();
            raw_el.set_inner_html(&rendered_html);
        }
    });

    let handle_input = move |_: web_sys::Event| {
        if let Some(el) = editor_ref.get() {
            let raw_el: &HtmlElement = el.as_ref();
            let html = raw_el.inner_html();
            let md = html_to_markdown(&html);

            is_internal_update.set_value(true);
            content.set(md.clone());
            on_change.run(md);
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
                        let md = html_to_markdown(&html);

                        is_internal_update.set_value(true);
                        content.set(md.clone());
                        on_change.run(md);
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

    view! {
        <div class="wysiwyg-container">
            <div
                node_ref=editor_ref
                class="wysiwyg-surface"
                contenteditable="true"
                spellcheck="false"
                attr:data-placeholder="Start typing or press '/' for commands..."
                on:input=handle_input
                on:click=handle_click
                on:keydown=handle_keydown
            ></div>
        </div>
    }
}
