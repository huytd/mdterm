use leptos::prelude::*;
use crate::state::{DocumentTab, EditorMode, Theme};

#[component]
pub fn Header(
    tabs: RwSignal<Vec<DocumentTab>>,
    active_tab_idx: RwSignal<usize>,
    active_mode: RwSignal<EditorMode>,
    current_theme: RwSignal<Theme>,
    sidebar_open: RwSignal<bool>,
    on_new_tab: Callback<()>,
    on_close_tab: Callback<usize>,
    on_open_file: Callback<()>,
    on_save_file: Callback<()>,
    on_export: Callback<()>,
    on_help: Callback<()>,
) -> impl IntoView {
    view! {
        <header class="app-header">
            <div class="header-left">
                <button
                    type="button"
                    class="header-icon-btn sidebar-toggle-btn"
                    title="Toggle Sidebar (Ctrl+B)"
                    on:click=move |_| sidebar_open.update(|open| *open = !*open)
                >
                    <svg class="header-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                        <line x1="9" y1="3" x2="9" y2="21"></line>
                    </svg>
                </button>

                <div class="app-brand">
                    <span class="brand-badge">"Md"</span>
                    <span class="brand-title">"MdTerm"</span>
                </div>

                <div class="tabs-container">
                    {move || {
                        let current_idx = active_tab_idx.get();
                        let all_tabs = tabs.get();
                        all_tabs.into_iter().enumerate().map(|(idx, tab)| {
                            let is_active = idx == current_idx;
                            let title = if tab.title.is_empty() { "Untitled.md".to_string() } else { tab.title };
                            let is_dirty = tab.is_dirty;

                            view! {
                                <div
                                    class=if is_active { "tab-item active" } else { "tab-item" }
                                    on:click=move |_| active_tab_idx.set(idx)
                                >
                                    <svg class="tab-file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                                        <polyline points="14 2 14 8 20 8"></polyline>
                                    </svg>
                                    <span class="tab-title">{title}</span>
                                    {if is_dirty {
                                        view! { <span class="tab-dirty-indicator" title="Unsaved changes">"•"</span> }.into_any()
                                    } else {
                                        view! { <span class="tab-clean-space"></span> }.into_any()
                                    }}
                                    <button
                                        type="button"
                                        class="tab-close-btn"
                                        title="Close tab"
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            on_close_tab.run(idx);
                                        }
                                    >
                                        "×"
                                    </button>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}

                    <button
                        type="button"
                        class="tab-new-btn"
                        title="New Tab (Ctrl+N)"
                        on:click=move |_| on_new_tab.run(())
                    >
                        "+"
                    </button>
                </div>
            </div>

            <div class="header-right">
                <div class="mode-switch-group">
                    <button
                        type="button"
                        class=move || if active_mode.get() == EditorMode::Wysiwyg { "mode-btn active" } else { "mode-btn" }
                        title="WYSIWYG Visual Editor"
                        on:click=move |_| active_mode.set(EditorMode::Wysiwyg)
                    >
                        <svg class="mode-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                            <circle cx="12" cy="12" r="3"></circle>
                        </svg>
                        <span>"WYSIWYG"</span>
                    </button>
                    <button
                        type="button"
                        class=move || if active_mode.get() == EditorMode::Split { "mode-btn active" } else { "mode-btn" }
                        title="Split Source & Preview"
                        on:click=move |_| active_mode.set(EditorMode::Split)
                    >
                        <svg class="mode-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                            <line x1="12" y1="3" x2="12" y2="21"></line>
                        </svg>
                        <span>"Split"</span>
                    </button>
                    <button
                        type="button"
                        class=move || if active_mode.get() == EditorMode::Source { "mode-btn active" } else { "mode-btn" }
                        title="Raw Markdown Source"
                        on:click=move |_| active_mode.set(EditorMode::Source)
                    >
                        <svg class="mode-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <polyline points="16 18 22 12 16 6"></polyline>
                            <polyline points="8 6 2 12 8 18"></polyline>
                        </svg>
                        <span>"Source"</span>
                    </button>
                </div>

                <div class="header-actions">
                    <button
                        type="button"
                        class="header-btn"
                        title="Open File (Ctrl+O)"
                        on:click=move |_| on_open_file.run(())
                    >
                        <svg class="header-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
                        <svg class="header-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                            <polyline points="17 21 17 13 7 13 7 21"></polyline>
                            <polyline points="7 3 7 8 15 8"></polyline>
                        </svg>
                        <span>"Save"</span>
                    </button>

                    <button
                        type="button"
                        class="header-btn"
                        title="Export Document (HTML / PDF / MD)"
                        on:click=move |_| on_export.run(())
                    >
                        <svg class="header-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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

                    <button
                        type="button"
                        class="header-icon-btn help-btn"
                        title="Help & Shortcuts"
                        on:click=move |_| on_help.run(())
                    >
                        <svg class="header-svg-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="10"></circle>
                            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
                            <line x1="12" y1="17" x2="12.01" y2="17"></line>
                        </svg>
                    </button>
                </div>
            </div>
        </header>
    }
}
