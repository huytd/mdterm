use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;
use crate::html::HtmlEnvelope;
use crate::markdown::markdown_to_html;

#[component]
pub fn DocumentPreview(
    content: Signal<String>,
    #[prop(optional)] is_html: Option<Signal<bool>>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let is_html_sig = is_html.unwrap_or_else(|| Signal::derive(|| false));
    let preview_ref = NodeRef::<leptos::html::Div>::new();

    let rendered_html = Memo::new(move |_| {
        let text = content.get();
        if is_html_sig.get() {
            let env = HtmlEnvelope::parse(&text);
            env.render_preview()
        } else {
            markdown_to_html(&text, false)
        }
    });

    Effect::new(move |_| {
        let _ = rendered_html.get();
        crate::tauri_bridge::render_mermaid_diagrams();
    });

    let handle_preview_click = move |ev: MouseEvent| {
        if let Some(target) = ev.target() {
            if let Ok(input_el) = target.dyn_into::<web_sys::HtmlInputElement>() {
                if input_el.class_name().contains("md-task-checkbox") {
                    let task_idx = input_el
                        .get_attribute("data-task-idx")
                        .and_then(|v| v.parse::<usize>().ok());

                    if let Some(idx) = task_idx {
                        let current_text = content.get();
                        if !is_html_sig.get() {
                            let mut match_count = 0;
                            let mut new_lines = Vec::new();

                            for line in current_text.lines() {
                                let trimmed = line.trim_start();
                                if trimmed.starts_with("- [ ] ")
                                    || trimmed.starts_with("- [x] ")
                                    || trimmed.starts_with("* [ ] ")
                                    || trimmed.starts_with("* [x] ")
                                {
                                    if match_count == idx {
                                        let indent = &line[..line.len() - trimmed.len()];
                                        let is_checked = trimmed.contains("[x]");
                                        let rest = &trimmed[6..];
                                        let replacement = if is_checked {
                                            format!("{}- [ ] {}", indent, rest)
                                        } else {
                                            format!("{}- [x] {}", indent, rest)
                                        };
                                        new_lines.push(replacement);
                                    } else {
                                        new_lines.push(line.to_string());
                                    }
                                    match_count += 1;
                                } else {
                                    new_lines.push(line.to_string());
                                }
                            }

                            let updated = new_lines.join("\n");
                            if let Some(cb) = on_change {
                                cb.run(updated);
                            }
                        }
                    }
                }
            }
        }
    };

    let container_class = class.unwrap_or("preview-container");

    view! {
        <div class=container_class node_ref=preview_ref on:click=handle_preview_click>
            <div
                class="preview-surface"
                inner_html=move || rendered_html.get()
            ></div>
        </div>
    }
}
