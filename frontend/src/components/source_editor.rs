use leptos::prelude::*;
use web_sys::{HtmlTextAreaElement, KeyboardEvent};

#[component]
pub fn SourceEditor(
    content: RwSignal<String>,
    #[prop(optional)] is_html: Option<Signal<bool>>,
    on_change: Callback<String>,
) -> impl IntoView {
    let is_html_sig = is_html.unwrap_or_else(|| Signal::derive(|| false));
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();
    let gutter_ref = NodeRef::<leptos::html::Div>::new();

    let lines_count = Memo::new(move |_| {
        let text = content.get();
        text.lines().count().max(1)
    });

    let gutter_html = Memo::new(move |_| {
        let count = lines_count.get();
        let mut html = String::with_capacity(count * 38);
        for i in 1..=count {
            use std::fmt::Write;
            let _ = write!(html, "<div class=\"gutter-line-num\">{}</div>", i);
        }
        html
    });

    let handle_input = move |ev: web_sys::Event| {
        let val = event_target_value(&ev);
        content.set(val.clone());
        on_change.run(val);
    };

    let handle_scroll = move |_| {
        if let (Some(ta), Some(gt)) = (textarea_ref.get(), gutter_ref.get()) {
            let ta_el: &HtmlTextAreaElement = ta.as_ref();
            let gt_el: &web_sys::HtmlElement = gt.as_ref();
            gt_el.set_scroll_top(ta_el.scroll_top());
        }
    };

    let handle_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();

        if key == "Tab" {
            ev.prevent_default();
            if let Some(ta) = textarea_ref.get() {
                let ta_el: &HtmlTextAreaElement = ta.as_ref();
                let start = ta_el.selection_start().unwrap_or(Some(0)).unwrap_or(0) as usize;
                let end = ta_el.selection_end().unwrap_or(Some(0)).unwrap_or(0) as usize;
                let mut text = content.get();

                text.replace_range(start..end, "  ");
                content.set(text.clone());
                on_change.run(text);

                let new_cursor = (start + 2) as u32;
                let _ = ta_el.set_selection_range(new_cursor, new_cursor);
            }
        }
    };

    view! {
        <div class="source-editor-container">
            <div
                node_ref=gutter_ref
                class="source-gutter"
                inner_html=move || gutter_html.get()
            ></div>

            <textarea
                node_ref=textarea_ref
                class="source-textarea"
                spellcheck="false"
                prop:value=move || content.get()
                on:input=handle_input
                on:scroll=handle_scroll
                on:keydown=handle_keydown
                placeholder=move || if is_html_sig.get() { "Type raw HTML here..." } else { "Type raw Markdown here..." }
            ></textarea>
        </div>
    }
}
