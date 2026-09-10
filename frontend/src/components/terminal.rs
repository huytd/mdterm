use leptos::prelude::*;
use crate::tauri_bridge::{clear_terminal_session, init_terminal_session};

#[component]
pub fn TerminalPane() -> impl IntoView {
    let container_id = "mdterm-xterm-container";

    Effect::new(move |_| {
        // Initialize terminal session once mounted
        init_terminal_session(container_id);
    });

    let handle_clear = move |_| {
        clear_terminal_session();
    };

    let handle_restart = move |_| {
        init_terminal_session(container_id);
    };

    view! {
        <div class="terminal-pane">
            <div class="terminal-header">
                <div class="terminal-title">
                    <span class="terminal-status-dot"></span>
                    <span class="terminal-label">"Terminal"</span>
                </div>
                <div class="terminal-actions">
                    <button
                        type="button"
                        class="terminal-btn"
                        title="Clear screen (Ctrl+L)"
                        on:click=handle_clear
                    >
                        <svg class="terminal-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M3 6h18"></path>
                            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"></path>
                            <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                        </svg>
                        <span>"Clear"</span>
                    </button>
                    <button
                        type="button"
                        class="terminal-btn"
                        title="Restart terminal shell"
                        on:click=handle_restart
                    >
                        <svg class="terminal-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <polyline points="23 4 23 10 17 10"></polyline>
                            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                        </svg>
                        <span>"Restart"</span>
                    </button>
                </div>
            </div>
            <div id=container_id class="terminal-container"></div>
        </div>
    }
}
