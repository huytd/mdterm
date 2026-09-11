use leptos::prelude::*;
use crate::components::preview::DocumentPreview;
use crate::components::source_editor::SourceEditor;

#[component]
pub fn SplitEditor(
    content: RwSignal<String>,
    #[prop(optional)] is_html: Option<Signal<bool>>,
    on_change: Callback<String>,
) -> impl IntoView {
    let is_html_sig = is_html.unwrap_or_else(|| Signal::derive(|| false));

    view! {
        <div class="split-editor-container">
            <div class="split-pane split-left">
                <SourceEditor content=content is_html=is_html_sig on_change=on_change />
            </div>

            <div class="split-divider"></div>

            <DocumentPreview
                content=content.into()
                is_html=is_html_sig
                on_change=on_change
                class="split-pane split-right"
            />
        </div>
    }
}
