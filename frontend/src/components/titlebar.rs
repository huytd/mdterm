use leptos::prelude::*;
use web_sys::MouseEvent;
use crate::state::WorkspaceTab;
use crate::tauri_bridge::{
    window_start_dragging, window_start_resize, window_toggle_maximize,
};

#[component]
pub fn TitleBar(
    tabs: Signal<Vec<WorkspaceTab>>,
    active_tab_id: Signal<String>,
    on_select_tab: Callback<String>,
    on_new_tab: Callback<()>,
    on_close_tab: Callback<String>,
    is_maximized: RwSignal<bool>,
) -> impl IntoView {
    let handle_double_click = move |_: MouseEvent| {
        leptos::task::spawn_local(async move {
            let new_state = window_toggle_maximize().await;
            is_maximized.set(new_state);
        });
    };

    let handle_mousedown = move |ev: MouseEvent| {
        if ev.button() == 0 {
            window_start_dragging();
        }
    };

    view! {
        <header
            class="window-titlebar glass-titlebar"
            data-tauri-drag-region="true"
            on:dblclick=handle_double_click
            on:mousedown=handle_mousedown
            title="Double-click to maximize/restore • Drag to move window"
        >
            <div class="titlebar-left" data-tauri-drag-region="true">
                <span class="titlebar-app-name" data-tauri-drag-region="true">"mdterm"</span>

                <div class="titlebar-tab-strip" data-tauri-drag-region="true">
                    <For
                        each=move || tabs.get()
                        key=|tab| tab.id.get()
                        children=move |tab| {
                            let tid = tab.id.get();
                            let tid_close = tid.clone();
                            let tid_select = tid.clone();
                            let is_dirty_val = tab.is_dirty;
                            let is_editor = tab.is_editor_open;
                            let active_filename = tab.active_filename;

                            let tab_index = Memo::new({
                                let tid = tid.clone();
                                move |_| tabs.get().iter().position(|t| t.id.get() == tid).unwrap_or(0)
                            });
                            let tab_count = Memo::new(move |_| tabs.get().len());
                            let is_active = Memo::new({
                                let tid = tid.clone();
                                move |_| active_tab_id.get() == tid
                            });

                            let display_title = move || {
                                if is_editor.get() {
                                    active_filename.get()
                                } else {
                                    format!("Terminal {}", tab_index.get() + 1)
                                }
                            };

                            let tab_class = move || {
                                if is_active.get() {
                                    "titlebar-tab active"
                                } else {
                                    "titlebar-tab"
                                }
                            };

                            let num_key = move || {
                                let idx = tab_index.get();
                                if idx < 9 {
                                    format!("{}", idx + 1)
                                } else {
                                    "".to_string()
                                }
                            };

                            view! {
                                <div
                                    class=tab_class
                                    title=move || format!("Tab {} (Ctrl+Shift+{} / Super+{})", tab_index.get() + 1, tab_index.get() + 1, tab_index.get() + 1)
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        on_select_tab.run(tid_select.clone());
                                    }
                                >
                                    {move || {
                                        let nk = num_key();
                                        if !nk.is_empty() {
                                            view! {
                                                <span class="tab-num-badge">{nk}</span>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }
                                    }}

                                    <span class="tab-title-text">{display_title}</span>

                                    {move || {
                                        if is_dirty_val.get() {
                                            view! {
                                                <span class="tab-dirty-indicator" title="Unsaved changes">"•"</span>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }
                                    }}

                                    {move || {
                                        if tab_count.get() > 1 {
                                            let tid_c = tid_close.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class="tab-close-btn"
                                                    title="Close tab (Ctrl+Shift+W / Super+W)"
                                                    on:click=move |ev: MouseEvent| {
                                                        ev.stop_propagation();
                                                        on_close_tab.run(tid_c.clone());
                                                    }
                                                >
                                                    "×"
                                                </button>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }
                                    }}
                                </div>
                            }
                        }
                    />

                    <button
                        type="button"
                        class="titlebar-new-tab-btn"
                        title="New Tab (Ctrl+Shift+T / Super+T)"
                        on:click=move |ev: MouseEvent| {
                            ev.stop_propagation();
                            on_new_tab.run(());
                        }
                    >
                        <svg class="new-tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="12" y1="5" x2="12" y2="19"></line>
                            <line x1="5" y1="12" x2="19" y2="12"></line>
                        </svg>
                    </button>
                </div>
            </div>

            <div class="titlebar-center" data-tauri-drag-region="true">
                <div class="titlebar-drag-pill" data-tauri-drag-region="true"></div>
            </div>

            <div class="titlebar-right" data-tauri-drag-region="true">
            </div>
        </header>
    }
}

#[component]
pub fn WindowResizeHandles(
    is_maximized: Signal<bool>,
) -> impl IntoView {
    view! {
        {move || if !is_maximized.get() {
            view! {
                <div class="window-resize-handles">
                    // 4 Edge handles
                    <div
                        class="window-resize-handle resize-edge-top"
                        title="Resize Top"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("North");
                            }
                        }
                    />
                    <div
                        class="window-resize-handle resize-edge-bottom"
                        title="Resize Bottom"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("South");
                            }
                        }
                    />
                    <div
                        class="window-resize-handle resize-edge-left"
                        title="Resize Left"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("West");
                            }
                        }
                    />
                    <div
                        class="window-resize-handle resize-edge-right"
                        title="Resize Right"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("East");
                            }
                        }
                    />

                    // 4 Corner handles
                    <div
                        class="window-resize-handle resize-corner-top-left"
                        title="Resize Top-Left"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("NorthWest");
                            }
                        }
                    />
                    <div
                        class="window-resize-handle resize-corner-top-right"
                        title="Resize Top-Right"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("NorthEast");
                            }
                        }
                    />
                    <div
                        class="window-resize-handle resize-corner-bottom-left"
                        title="Resize Bottom-Left"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("SouthWest");
                            }
                        }
                    />
                    <div
                        class="window-resize-handle resize-corner-bottom-right"
                        title="Resize Bottom-Right"
                        on:mousedown=move |ev: MouseEvent| {
                            if ev.button() == 0 {
                                ev.prevent_default();
                                window_start_resize("SouthEast");
                            }
                        }
                    />
                </div>
            }.into_any()
        } else {
            ().into_any()
        }}
    }
}
