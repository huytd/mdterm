use leptos::prelude::*;
use crate::state::Theme;

#[component]
pub fn Header(
    active_filename: Signal<String>,
    is_dirty: Signal<bool>,
    current_theme: RwSignal<Theme>,
    on_new_file: Callback<()>,
    on_open_file: Callback<()>,
    on_save_file: Callback<()>,
    on_export: Callback<()>,
    on_format: Callback<&'static str>,
    on_format_block: Callback<&'static str>,
) -> impl IntoView {
    view! {
        <header class="app-header">
            <div class="header-left">
                <div class="app-brand">
                    <span class="brand-badge">"MD"</span>
                    <span class="brand-title">"MdTerm"</span>
                </div>

                <div class="doc-title-pill">
                    <svg class="doc-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                    </svg>
                    <span class="doc-name">{move || active_filename.get()}</span>
                    {move || if is_dirty.get() {
                        view! { <span class="dirty-dot" title="Unsaved changes">"•"</span> }.into_any()
                    } else {
                        view! { <span class="clean-dot"></span> }.into_any()
                    }}
                </div>

                <div class="header-divider"></div>

                <div class="action-btn-group">
                    <button
                        type="button"
                        class="header-btn"
                        title="New Document (Ctrl+N)"
                        on:click=move |_| on_new_file.run(())
                    >
                        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="12" y1="5" x2="12" y2="19"></line>
                            <line x1="5" y1="12" x2="19" y2="12"></line>
                        </svg>
                        <span>"New"</span>
                    </button>

                    <button
                        type="button"
                        class="header-btn"
                        title="Open Document (Ctrl+O)"
                        on:click=move |_| on_open_file.run(())
                    >
                        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                        </svg>
                        <span>"Open"</span>
                    </button>

                    <button
                        type="button"
                        class="header-btn primary-save-btn"
                        title="Save Document (Ctrl+S)"
                        on:click=move |_| on_save_file.run(())
                    >
                        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                            <polyline points="17 21 17 13 7 13 7 21"></polyline>
                            <polyline points="7 3 7 8 15 8"></polyline>
                        </svg>
                        <span>"Save"</span>
                    </button>
                </div>

                <div class="header-divider"></div>

                <div class="format-btn-group">
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Bold (Ctrl+B)"
                        on:click=move |_| on_format.run("bold")
                    >
                        <strong>"B"</strong>
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Italic (Ctrl+I)"
                        on:click=move |_| on_format.run("italic")
                    >
                        <em>"I"</em>
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Heading 1"
                        on:click=move |_| on_format_block.run("h1")
                    >
                        "H1"
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Heading 2"
                        on:click=move |_| on_format_block.run("h2")
                    >
                        "H2"
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Bullet List"
                        on:click=move |_| on_format.run("insertUnorderedList")
                    >
                        "• List"
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Task Checkbox List"
                        on:click=move |_| on_format.run("tasklist")
                    >
                        "☑ Task"
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Inline Code"
                        on:click=move |_| on_format.run("code")
                    >
                        "<code>"
                    </button>
                    <button
                        type="button"
                        class="format-tool-btn"
                        title="Blockquote"
                        on:click=move |_| on_format_block.run("blockquote")
                    >
                        "“ Quote"
                    </button>
                </div>
            </div>

            <div class="header-right">
                <button
                    type="button"
                    class="header-btn"
                    title="Export as HTML or Markdown"
                    on:click=move |_| on_export.run(())
                >
                    <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                        <polyline points="7 10 12 15 17 10"></polyline>
                        <line x1="12" y1="15" x2="12" y2="3"></line>
                    </svg>
                    <span>"Export"</span>
                </button>

                <div class="theme-picker">
                    <select
                        class="theme-select"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            match val.as_str() {
                                "light" => current_theme.set(Theme::Light),
                                "nord" => current_theme.set(Theme::Nord),
                                "monokai" => current_theme.set(Theme::Monokai),
                                _ => current_theme.set(Theme::Dark),
                            }
                        }
                        prop:value=move || match current_theme.get() {
                            Theme::Light => "light",
                            Theme::Nord => "nord",
                            Theme::Monokai => "monokai",
                            Theme::Dark => "dark",
                        }
                    >
                        <option value="dark">"🌙 Dark"</option>
                        <option value="light">"☀️ Light"</option>
                        <option value="nord">"❄️ Nord"</option>
                        <option value="monokai">"⚡ Monokai"</option>
                    </select>
                </div>
            </div>
        </header>
    }
}
