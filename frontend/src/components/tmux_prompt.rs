use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;
use crate::tauri_bridge::{focus_terminal_session, TmuxDetectInfo};

/// A single selectable row in the prompt.
#[derive(Clone)]
enum TmuxChoice {
    /// Attach to an existing session by name.
    Attach(String),
    /// Create a fresh control-mode session.
    NewSession,
    /// Dismiss and keep the current terminal.
    KeepCurrent,
}

/// Every selectable row in display order, so keyboard navigation and mouse
/// clicks share a single source of truth.
fn choices_for(data: &TmuxDetectInfo) -> Vec<TmuxChoice> {
    let mut choices: Vec<TmuxChoice> = data
        .sessions
        .iter()
        .map(|s| TmuxChoice::Attach(s.name.clone()))
        .collect();
    choices.push(TmuxChoice::NewSession);
    choices.push(TmuxChoice::KeepCurrent);
    choices
}

/// Hand the keyboard back to the terminal that was behind the prompt.
///
/// Deferred to the next frame: `focus_terminal_session` deliberately does
/// nothing while the prompt is mounted, and Leptos removes it asynchronously
/// after `info.set(None)`.
fn refocus_terminal() {
    request_animation_frame(|| focus_terminal_session(None));
}

/// Close the prompt without choosing anything (×, backdrop click).
fn dismiss(info: RwSignal<Option<TmuxDetectInfo>>) {
    info.set(None);
    refocus_terminal();
}

/// Close the prompt and perform the action bound to row `idx`.
fn activate(
    info: RwSignal<Option<TmuxDetectInfo>>,
    on_attach: Callback<(Option<String>, bool)>,
    choices: &[TmuxChoice],
    idx: usize,
) {
    let choice = choices.get(idx).cloned();
    info.set(None);
    match choice {
        Some(TmuxChoice::Attach(name)) => on_attach.run((Some(name), false)),
        Some(TmuxChoice::NewSession) => on_attach.run((None, true)),
        Some(TmuxChoice::KeepCurrent) | None => refocus_terminal(),
    }
}

/// Launch-time offer to attach to a running tmux server through control mode.
///
/// Fully keyboard-interactive: `↑`/`↓` (or `Tab`/`Shift+Tab`) move the
/// selection, `Home`/`End` jump to the first/last choice, `Enter` activates
/// the highlighted choice, and `Escape` dismisses the prompt.
#[component]
pub fn TmuxPrompt(
    info: RwSignal<Option<TmuxDetectInfo>>,
    /// (session name, create if missing); `None` = new session.
    on_attach: Callback<(Option<String>, bool)>,
) -> impl IntoView {
    // Highlighted row index. Reset whenever a fresh prompt is shown.
    let selected = RwSignal::new(0usize);
    let dialog_ref = NodeRef::<leptos::html::Div>::new();

    // Move DOM focus out of xterm's hidden textarea and onto the dialog each
    // time the prompt opens. This matters beyond looks: a focused textarea
    // receives input-method commits that never pass through `keydown`.
    // `focus_terminal_session` stays hands-off while the prompt is mounted, so
    // nothing pulls focus back. The capture listeners below are the backstop.
    Effect::new(move |_| {
        if info.get().is_some() {
            selected.set(0);
            if let Some(el) = dialog_ref.get() {
                let _ = el.focus();
            }
        }
    });

    // Window-level *capture* listener: runs before xterm's helper textarea
    // (or any other focused element) sees the key, so the prompt works no
    // matter where focus is. While the prompt is open it is modal and swallows
    // every keydown so nothing leaks into the shell behind it. Registered once
    // for the component's lifetime (it is mounted once by `App`) and gated on
    // `info`, mirroring the global keydown listener in app.rs.
    if let Some(win) = web_sys::window() {
        let cb = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
            let Some(data) = info.get_untracked() else { return };
            ev.prevent_default();
            ev.stop_immediate_propagation();

            let choices = choices_for(&data);
            let count = choices.len();
            match ev.key().as_str() {
                "ArrowDown" => selected.update(|i| *i = (*i + 1) % count),
                "ArrowUp" => selected.update(|i| *i = (*i + count - 1) % count),
                "Tab" if ev.shift_key() => selected.update(|i| *i = (*i + count - 1) % count),
                "Tab" => selected.update(|i| *i = (*i + 1) % count),
                "Home" => selected.set(0),
                "End" => selected.set(count - 1),
                "Enter" | " " => activate(info, on_attach, &choices, selected.get_untracked()),
                "Escape" => activate(info, on_attach, &choices, count - 1),
                _ => {}
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);
        let _ = win.add_event_listener_with_callback_and_bool(
            "keydown",
            cb.as_ref().unchecked_ref(),
            true,
        );
        cb.forget();

        // Text can reach a focused xterm without a cancelable keydown — notably
        // GTK input-method commits (`beforeinput`/`input`/composition). Swallow
        // those too while the prompt is open.
        for name in [
            "keypress",
            "keyup",
            "beforeinput",
            "input",
            "compositionstart",
            "compositionupdate",
            "compositionend",
        ] {
            let cb = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                if info.get_untracked().is_none() {
                    return;
                }
                ev.prevent_default();
                ev.stop_immediate_propagation();
            }) as Box<dyn FnMut(web_sys::Event)>);
            let _ = win.add_event_listener_with_callback_and_bool(
                name,
                cb.as_ref().unchecked_ref(),
                true,
            );
            cb.forget();
        }
    }

    move || {
        let Some(data) = info.get() else {
            return ().into_any();
        };
        let version = data.version.clone().unwrap_or_default();
        let sessions = data.sessions.clone();
        let choices = choices_for(&data);
        let count = choices.len();

        // Perform the action bound to a given row index (mouse path).
        let activate = move |idx: usize| activate(info, on_attach, &choices, idx);

        // Render the session cards followed by the two fixed actions.
        let session_cards = sessions
            .into_iter()
            .enumerate()
            .map(|(idx, s)| {
                let detail = format!(
                    "{} window{}{}",
                    s.windows,
                    if s.windows == 1 { "" } else { "s" },
                    if s.attached > 0 {
                        format!(
                            " · {} client{} attached",
                            s.attached,
                            if s.attached == 1 { "" } else { "s" }
                        )
                    } else {
                        String::new()
                    }
                );
                let activate = activate.clone();
                let card_class = move || {
                    if selected.get() == idx {
                        "export-card-btn selected"
                    } else {
                        "export-card-btn"
                    }
                };
                view! {
                    <button
                        type="button"
                        class=card_class
                        on:mouseenter=move |_| selected.set(idx)
                        on:click=move |_| activate(idx)
                    >
                        <div class="exp-icon">"⧉"</div>
                        <div class="exp-info">
                            <h4>{s.name.clone()}</h4>
                            <p>{detail}</p>
                        </div>
                    </button>
                }
            })
            .collect_view();

        let new_idx = count - 2;
        let keep_idx = count - 1;
        let activate_new = activate.clone();
        let activate_keep = activate.clone();
        let new_class = move || {
            if selected.get() == new_idx { "export-card-btn selected" } else { "export-card-btn" }
        };
        let keep_class = move || {
            if selected.get() == keep_idx { "export-card-btn selected" } else { "export-card-btn" }
        };

        view! {
            <div class="modal-backdrop" on:click=move |_| dismiss(info)>
                <div
                    class="modal-dialog tmux-prompt"
                    tabindex="-1"
                    node_ref=dialog_ref
                    on:click=move |ev| ev.stop_propagation()
                >
                    <div class="modal-header">
                        <h3>"tmux is running"</h3>
                        <button type="button" class="modal-close-btn" on:click=move |_| dismiss(info)>"×"</button>
                    </div>
                    <div class="modal-body">
                        <p class="tmux-prompt-intro">
                            "Attach natively: panes become mdterm splits, windows become tabs. "
                            <span class="tmux-prompt-version">{version}</span>
                        </p>
                        <p class="tmux-prompt-hint">
                            "Use ↑ ↓ to navigate, Enter to select, Esc to dismiss."
                        </p>
                        <div class="export-options">
                            {session_cards}
                            <button
                                type="button"
                                class=new_class
                                on:mouseenter=move |_| selected.set(new_idx)
                                on:click=move |_| activate_new(new_idx)
                            >
                                <div class="exp-icon">"＋"</div>
                                <div class="exp-info">
                                    <h4>"New tmux session"</h4>
                                    <p>"Start a fresh session in control mode"</p>
                                </div>
                            </button>
                            <button
                                type="button"
                                class=keep_class
                                on:mouseenter=move |_| selected.set(keep_idx)
                                on:click=move |_| activate_keep(keep_idx)
                            >
                                <div class="exp-icon">"›_"</div>
                                <div class="exp-info">
                                    <h4>"Keep current terminal"</h4>
                                    <p>"Stay in this tab as-is (including a tmux your shell already started). Set tmux.integration to \"auto\" or \"off\" in config.yml to skip this prompt"</p>
                                </div>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        }.into_any()
    }
}
