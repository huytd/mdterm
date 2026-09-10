use leptos::prelude::*;
use crate::state::{EditorPosition, Theme};

#[component]
pub fn EditorHeader(
    active_filename: Signal<String>,
    is_dirty: Signal<bool>,
    is_remote: Signal<bool>,
    current_theme: RwSignal<Theme>,
    editor_position: RwSignal<EditorPosition>,
    on_close_editor: Callback<()>,
    on_save_file: Callback<()>,
    on_export: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="editor-tab-bar">
            <div class="tab-title">
                <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                    <polyline points="14 2 14 8 20 8"></polyline>
                </svg>
                <span class="tab-filename">{move || active_filename.get()}</span>
                {move || if is_remote.get() {
                    view! { <span class="tab-remote-tag" title="Connected to remote SSH session">"SSH"</span> }.into_any()
                } else {
                    ().into_any()
                }}
                {move || if is_dirty.get() {
                    view! { <span class="tab-dirty" title="Unsaved changes">"•"</span> }.into_any()
                } else {
                    ().into_any()
                }}
            </div>

            <div class="tab-actions">
                <button
                    type="button"
                    class="tab-btn"
                    title=move || if editor_position.get() == EditorPosition::Left {
                        "Move editor to right side"
                    } else {
                        "Move editor to left side"
                    }
                    on:click=move |_| {
                        editor_position.update(|p| *p = match *p {
                            EditorPosition::Left => EditorPosition::Right,
                            EditorPosition::Right => EditorPosition::Left,
                        });
                        crate::tauri_bridge::fit_terminal_session();
                    }
                >
                    <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="8 7 4 11 8 15"></polyline>
                        <polyline points="16 7 20 11 16 15"></polyline>
                        <line x1="4" y1="11" x2="20" y2="11"></line>
                    </svg>
                </button>

                <button
                    type="button"
                    class="tab-btn"
                    title="Save (Ctrl+S)"
                    on:click=move |_| on_save_file.run(())
                >
                    <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                        <polyline points="17 21 17 13 7 13 7 21"></polyline>
                        <polyline points="7 3 7 8 15 8"></polyline>
                    </svg>
                </button>

                <button
                    type="button"
                    class="tab-btn"
                    title="Export as HTML or Markdown"
                    on:click=move |_| on_export.run(())
                >
                    <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                        <polyline points="7 10 12 15 17 10"></polyline>
                        <line x1="12" y1="15" x2="12" y2="3"></line>
                    </svg>
                </button>

                <button
                    type="button"
                    class="tab-btn"
                    title="Cycle Theme"
                    on:click=move |_| current_theme.update(|t| *t = t.next())
                >
                    <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="5"></circle>
                        <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"></path>
                    </svg>
                </button>

                <button
                    type="button"
                    class="tab-btn close-tab-btn"
                    title="Close editor (Ctrl+W)"
                    on:click=move |_| on_close_editor.run(())
                >
                    <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="18" y1="6" x2="6" y2="18"></line>
                        <line x1="6" y1="6" x2="18" y2="18"></line>
                    </svg>
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn FloatingControls(
    current_theme: RwSignal<Theme>,
    on_new_file: Callback<bool>,
    on_open_file: Callback<bool>,
) -> impl IntoView {
    view! {
        <div class="floating-terminal-bar">
            <button
                type="button"
                class="float-btn"
                title="New Document (Ctrl+N, Super+Click for left)"
                on:click=move |ev: web_sys::MouseEvent| {
                    let is_super = ev.meta_key();
                    on_new_file.run(is_super);
                }
            >
                <svg class="float-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="12" y1="5" x2="12" y2="19"></line>
                    <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
            </button>

            <button
                type="button"
                class="float-btn"
                title="Open Document (Ctrl+O, Super+Click for left)"
                on:click=move |ev: web_sys::MouseEvent| {
                    let is_super = ev.meta_key();
                    on_open_file.run(is_super);
                }
            >
                <svg class="float-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                </svg>
            </button>

            <button
                type="button"
                class="float-btn"
                title="Cycle Theme"
                on:click=move |_| current_theme.update(|t| *t = t.next())
            >
                <svg class="float-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="5"></circle>
                    <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"></path>
                </svg>
            </button>
        </div>
    }
}
