use leptos::prelude::*;
use crate::tauri_bridge::{focus_terminal_session, init_terminal_session};

#[component]
pub fn TerminalPane() -> impl IntoView {
    let container_id = "mdterm-xterm-container";

    Effect::new(move |_| {
        // Initialize terminal session once mounted
        init_terminal_session(container_id);
        focus_terminal_session();
    });

    view! {
        <div
            class="terminal-pane"
            on:click=move |_| {
                focus_terminal_session();
            }
        >
            <div id=container_id class="terminal-container"></div>
        </div>
    }
}
