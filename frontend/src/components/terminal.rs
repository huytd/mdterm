use leptos::prelude::*;
use crate::tauri_bridge::{focus_terminal_session, init_terminal_session, window_show};

#[component]
pub fn TerminalPane(
    session_id: String,
) -> impl IntoView {
    let session_id_clone = session_id.clone();
    let container_id = format!("mdterm-xterm-container-{}", session_id);
    let cid_for_mount = container_id.clone();
    let sid_for_click = session_id.clone();

    Effect::new(move |_| {
        let cid = cid_for_mount.clone();
        let sid = session_id_clone.clone();
        leptos::task::spawn_local(async move {
            init_terminal_session(&cid, &sid).await;
            focus_terminal_session(Some(&sid));
            window_show();
        });
    });

    view! {
        <div
            class="terminal-pane"
            on:click=move |_| {
                focus_terminal_session(Some(&sid_for_click));
            }
        >
            <div id=container_id class="terminal-container"></div>
        </div>
    }
}
