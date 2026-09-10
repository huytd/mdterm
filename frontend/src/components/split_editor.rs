use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;
use crate::components::source_editor::SourceEditor;
use crate::markdown::markdown_to_html;

#[component]
pub fn SplitEditor(
    content: RwSignal<String>,
    on_change: Callback<String>,
) -> impl IntoView {
    let preview_ref = NodeRef::<leptos::html::Div>::new();

    let rendered_html = Memo::new(move |_| {
        let md = content.get();
        markdown_to_html(&md, false)
    });

    let handle_preview_click = move |ev: MouseEvent| {
        if let Some(target) = ev.target() {
            if let Ok(input_el) = target.dyn_into::<web_sys::HtmlInputElement>() {
                if input_el.class_name().contains("md-task-checkbox") {
                    let task_idx = input_el
                        .get_attribute("data-task-idx")
                        .and_then(|v| v.parse::<usize>().ok());

                    if let Some(idx) = task_idx {
                        let current_md = content.get();
                        let mut match_count = 0;
                        let mut new_lines = Vec::new();

                        for line in current_md.lines() {
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
                        content.set(updated.clone());
                        on_change.run(updated);
                    }
                }
            }
        }
    };

    view! {
        <div class="split-editor-container">
            <div class="split-pane split-left">
                <SourceEditor content=content on_change=on_change />
            </div>

            <div class="split-divider"></div>

            <div class="split-pane split-right" node_ref=preview_ref on:click=handle_preview_click>
                <div
                    class="preview-surface"
                    inner_html=move || rendered_html.get()
                ></div>
            </div>
        </div>
    }
}
