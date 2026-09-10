use leptos::prelude::*;
use crate::tauri_bridge::init_terminal_session;

#[component]
pub fn TerminalPane() -> impl IntoView {
    let container_id = "mdterm-xterm-container";

    Effect::new(move |_| {
        // Initialize terminal session once mounted
        init_terminal_session(container_id);
    });

    view! {
        <div class="terminal-pane">
            <div id=container_id class="terminal-container"></div>
        </div>
    }
}
