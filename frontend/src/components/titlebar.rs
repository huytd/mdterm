use leptos::prelude::*;
use web_sys::MouseEvent;
use crate::state::Theme;
use crate::tauri_bridge::{
    window_start_dragging, window_start_resize, window_toggle_maximize,
};

#[component]
pub fn TitleBar(
    active_filename: Signal<String>,
    is_dirty: Signal<bool>,
    is_editor_open: Signal<bool>,
    is_maximized: RwSignal<bool>,
    #[prop(optional)] current_theme: Option<RwSignal<Theme>>,
) -> impl IntoView {
    let _ = current_theme;

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

                {move || if is_editor_open.get() {
                    view! {
                        <div class="titlebar-doc-badge" data-tauri-drag-region="true">
                            <span class="titlebar-sep" data-tauri-drag-region="true">"·"</span>
                            <span class="titlebar-filename" data-tauri-drag-region="true">{move || active_filename.get()}</span>
                            {move || if is_dirty.get() {
                                view! { <span class="titlebar-dirty" title="Unsaved changes">"•"</span> }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
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
