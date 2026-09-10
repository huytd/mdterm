use leptos::prelude::*;
use crate::state::{DocumentStats, EditorMode};

#[component]
pub fn StatusBar(
    stats: Memo<DocumentStats>,
    mode: RwSignal<EditorMode>,
    is_dirty: Signal<bool>,
    file_path: Signal<Option<String>>,
    #[prop(optional)]
    is_html: Option<Signal<bool>>,
) -> impl IntoView {
    view! {
        <footer class="app-status-bar">
            <div class="status-bar-left">
                <span class="status-item file-status">
                    {move || {
                        let dirty = is_dirty.get();
                        let path = file_path.get().unwrap_or_else(|| {
                            let html = is_html.map(|s| s.get()).unwrap_or(false);
                            if html { "Untitled.html".to_string() } else { "Untitled.md".to_string() }
                        });
                        view! {
                            <span class=if dirty { "dirty-badge is-dirty" } else { "dirty-badge" }>
                                {if dirty { "• Unsaved" } else { "✓ Saved" }}
                            </span>
                            <span class="status-path">{path}</span>
                        }
                    }}
                </span>
            </div>

            <div class="status-bar-right">
                <span class="status-item">
                    <span class="status-label">"Mode:"</span>
                    <span class="status-val">{move || mode.get().label()}</span>
                </span>
                <span class="status-item-separator">"|"</span>
                <span class="status-item">
                    <span class="status-val">{move || stats.get().words}</span>
                    <span class="status-label">" words"</span>
                </span>
                <span class="status-item-separator">"|"</span>
                <span class="status-item">
                    <span class="status-val">{move || stats.get().chars}</span>
                    <span class="status-label">" characters"</span>
                </span>
                <span class="status-item-separator">"|"</span>
                <span class="status-item">
                    <span class="status-val">{move || stats.get().lines}</span>
                    <span class="status-label">" lines"</span>
                </span>
                <span class="status-item-separator">"|"</span>
                <span class="status-item">
                    <span class="status-val">
                        {move || format!("{:.1} min read", stats.get().reading_time_mins)}
                    </span>
                </span>
                <span class="status-item-separator">"|"</span>
                <span class="status-item format-badge">
                    {move || {
                        let html = is_html.map(|s| s.get()).unwrap_or(false);
                        if html { "UTF-8 · HTML" } else { "UTF-8 · GFM" }
                    }}
                </span>
            </div>
        </footer>
    }
}
