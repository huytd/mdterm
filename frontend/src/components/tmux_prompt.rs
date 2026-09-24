use leptos::prelude::*;
use crate::tauri_bridge::TmuxDetectInfo;

/// Launch-time offer to attach to a running tmux server through control mode.
#[component]
pub fn TmuxPrompt(
    info: RwSignal<Option<TmuxDetectInfo>>,
    /// (session name, create if missing); `None` = new session.
    on_attach: Callback<(Option<String>, bool)>,
) -> impl IntoView {
    move || {
        let Some(data) = info.get() else {
            return ().into_any();
        };
        let version = data.version.clone().unwrap_or_default();
        let sessions = data.sessions.clone();
        view! {
            <div class="modal-backdrop" on:click=move |_| info.set(None)>
                <div class="modal-dialog tmux-prompt" on:click=move |ev| ev.stop_propagation()>
                    <div class="modal-header">
                        <h3>"tmux is running"</h3>
                        <button type="button" class="modal-close-btn" on:click=move |_| info.set(None)>"×"</button>
                    </div>
                    <div class="modal-body">
                        <p class="tmux-prompt-intro">
                            "Attach natively: panes become mdterm splits, windows become tabs. "
                            <span class="tmux-prompt-version">{version}</span>
                        </p>
                        <div class="export-options">
                            {sessions.into_iter().map(|s| {
                                let name = s.name.clone();
                                let detail = format!(
                                    "{} window{}{}",
                                    s.windows,
                                    if s.windows == 1 { "" } else { "s" },
                                    if s.attached > 0 { format!(" · {} client{} attached", s.attached, if s.attached == 1 { "" } else { "s" }) } else { String::new() }
                                );
                                view! {
                                    <button
                                        type="button"
                                        class="export-card-btn"
                                        on:click=move |_| {
                                            info.set(None);
                                            on_attach.run((Some(name.clone()), false));
                                        }
                                    >
                                        <div class="exp-icon">"⧉"</div>
                                        <div class="exp-info">
                                            <h4>{s.name.clone()}</h4>
                                            <p>{detail}</p>
                                        </div>
                                    </button>
                                }
                            }).collect_view()}
                            <button
                                type="button"
                                class="export-card-btn"
                                on:click=move |_| {
                                    info.set(None);
                                    on_attach.run((None, true));
                                }
                            >
                                <div class="exp-icon">"＋"</div>
                                <div class="exp-info">
                                    <h4>"New tmux session"</h4>
                                    <p>"Start a fresh session in control mode"</p>
                                </div>
                            </button>
                            <button
                                type="button"
                                class="export-card-btn"
                                on:click=move |_| info.set(None)
                            >
                                <div class="exp-icon">"›_"</div>
                                <div class="exp-info">
                                    <h4>"Plain shell"</h4>
                                    <p>"Set tmux.integration in config.yml to \"auto\" or \"off\" to skip this prompt"</p>
                                </div>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        }.into_any()
    }
}
