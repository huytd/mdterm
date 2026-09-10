use leptos::prelude::*;
use crate::state::{ActiveModal, FindReplaceState, SlashMenuState};

#[component]
pub fn Modals(
    active_modal: RwSignal<ActiveModal>,
    find_replace: RwSignal<FindReplaceState>,
    slash_menu: RwSignal<SlashMenuState>,
    on_insert_link_confirm: Callback<(String, String)>,
    on_insert_image_confirm: Callback<(String, String)>,
    on_insert_table_confirm: Callback<(usize, usize)>,
    on_export_confirm: Callback<&'static str>,
    on_new_file_confirm: Callback<String>,
    on_find_next: Callback<()>,
    on_find_prev: Callback<()>,
    on_replace_one: Callback<()>,
    on_replace_all: Callback<()>,
    on_slash_select: Callback<&'static str>,
) -> impl IntoView {
    let link_url = RwSignal::new(String::new());
    let link_text = RwSignal::new(String::new());

    let img_url = RwSignal::new(String::new());
    let img_alt = RwSignal::new(String::new());

    let table_rows = RwSignal::new(3usize);
    let table_cols = RwSignal::new(3usize);

    let new_filename = RwSignal::new(String::from("New-Document.md"));

    view! {
        // --- Find & Replace Floating Bar ---
        {move || {
            let fr = find_replace.get();
            if fr.is_open {
                view! {
                    <div class="find-replace-bar">
                        <div class="fr-inputs">
                            <div class="fr-input-group">
                                <input
                                    type="text"
                                    class="fr-input"
                                    placeholder="Find..."
                                    prop:value=move || find_replace.get().search_query
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        find_replace.update(|s| s.search_query = v);
                                    }
                                />
                                <span class="fr-counter">
                                    {move || {
                                        let cur = find_replace.get().current_match;
                                        let tot = find_replace.get().total_matches;
                                        if tot > 0 { format!("{}/{}", cur, tot) } else { "No matches".to_string() }
                                    }}
                                </span>
                            </div>
                            <div class="fr-input-group">
                                <input
                                    type="text"
                                    class="fr-input"
                                    placeholder="Replace with..."
                                    prop:value=move || find_replace.get().replace_query
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        find_replace.update(|s| s.replace_query = v);
                                    }
                                />
                            </div>
                        </div>

                        <div class="fr-actions">
                            <button
                                type="button"
                                class="fr-btn"
                                title="Previous match"
                                on:click=move |_| on_find_prev.run(())
                            >
                                "▲"
                            </button>
                            <button
                                type="button"
                                class="fr-btn"
                                title="Next match"
                                on:click=move |_| on_find_next.run(())
                            >
                                "▼"
                            </button>
                            <button
                                type="button"
                                class="fr-btn"
                                title="Replace current"
                                on:click=move |_| on_replace_one.run(())
                            >
                                "Replace"
                            </button>
                            <button
                                type="button"
                                class="fr-btn"
                                title="Replace all"
                                on:click=move |_| on_replace_all.run(())
                            >
                                "All"
                            </button>
                            <button
                                type="button"
                                class="fr-btn fr-close-btn"
                                title="Close (Esc)"
                                on:click=move |_| find_replace.update(|s| s.is_open = false)
                            >
                                "×"
                            </button>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }
        }}

        // --- Slash Command Floating Menu ---
        {move || {
            let sm = slash_menu.get();
            if sm.is_open {
                let left_px = format!("{}px", sm.x);
                let top_px = format!("{}px", sm.y);
                view! {
                    <div
                        class="slash-menu-popup"
                        style=format!("left: {}; top: {};", left_px, top_px)
                    >
                        <div class="slash-menu-header">"Insert Block"</div>
                        <div class="slash-menu-items">
                            <div class="slash-item" on:click=move |_| on_slash_select.run("h1")>
                                <span class="slash-icon">"H1"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Heading 1"</span>
                                    <span class="slash-desc">"Large top-level title"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("h2")>
                                <span class="slash-icon">"H2"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Heading 2"</span>
                                    <span class="slash-desc">"Medium section header"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("h3")>
                                <span class="slash-icon">"H3"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Heading 3"</span>
                                    <span class="slash-desc">"Sub-section title"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("tasklist")>
                                <span class="slash-icon">"☑"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"To-Do List"</span>
                                    <span class="slash-desc">"Interactive checklist item"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("bullet")>
                                <span class="slash-icon">"•"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Bulleted List"</span>
                                    <span class="slash-desc">"Unordered bullet list"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("number")>
                                <span class="slash-icon">"1."</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Numbered List"</span>
                                    <span class="slash-desc">"Sequential ordered list"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("quote")>
                                <span class="slash-icon">"❝"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Blockquote"</span>
                                    <span class="slash-desc">"Quote callout block"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("code")>
                                <span class="slash-icon">"‹/›"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Code Block"</span>
                                    <span class="slash-desc">"Preformatted code container"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("table")>
                                <span class="slash-icon">"▦"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Table"</span>
                                    <span class="slash-desc">"Insert 3x3 table"</span>
                                </div>
                            </div>
                            <div class="slash-item" on:click=move |_| on_slash_select.run("divider")>
                                <span class="slash-icon">"―"</span>
                                <div class="slash-info">
                                    <span class="slash-title">"Divider"</span>
                                    <span class="slash-desc">"Horizontal divider line"</span>
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }
        }}

        // --- Dialog Modals ---
        {move || match active_modal.get() {
            ActiveModal::None => view! { <span></span> }.into_any(),

            ActiveModal::InsertLink => view! {
                <div class="modal-backdrop" on:click=move |_| active_modal.set(ActiveModal::None)>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Insert Link"</h3>
                            <button type="button" class="modal-close-btn" on:click=move |_| active_modal.set(ActiveModal::None)>"×"</button>
                        </div>
                        <div class="modal-body">
                            <label class="modal-label">"Link Text"</label>
                            <input
                                type="text"
                                class="modal-input"
                                placeholder="Display text"
                                prop:value=move || link_text.get()
                                on:input=move |ev| link_text.set(event_target_value(&ev))
                            />
                            <label class="modal-label">"URL / Web Address"</label>
                            <input
                                type="text"
                                class="modal-input"
                                placeholder="https://example.com"
                                prop:value=move || link_url.get()
                                on:input=move |ev| link_url.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-secondary" on:click=move |_| active_modal.set(ActiveModal::None)>"Cancel"</button>
                            <button
                                type="button"
                                class="btn btn-primary"
                                on:click=move |_| {
                                    let url = link_url.get();
                                    let text = link_text.get();
                                    if !url.trim().is_empty() {
                                        on_insert_link_confirm.run((url, text));
                                        link_url.set(String::new());
                                        link_text.set(String::new());
                                        active_modal.set(ActiveModal::None);
                                    }
                                }
                            >
                                "Insert Link"
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any(),

            ActiveModal::InsertImage => view! {
                <div class="modal-backdrop" on:click=move |_| active_modal.set(ActiveModal::None)>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Insert Image"</h3>
                            <button type="button" class="modal-close-btn" on:click=move |_| active_modal.set(ActiveModal::None)>"×"</button>
                        </div>
                        <div class="modal-body">
                            <label class="modal-label">"Alt Description"</label>
                            <input
                                type="text"
                                class="modal-input"
                                placeholder="Image description"
                                prop:value=move || img_alt.get()
                                on:input=move |ev| img_alt.set(event_target_value(&ev))
                            />
                            <label class="modal-label">"Image URL"</label>
                            <input
                                type="text"
                                class="modal-input"
                                placeholder="https://... or file path"
                                prop:value=move || img_url.get()
                                on:input=move |ev| img_url.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-secondary" on:click=move |_| active_modal.set(ActiveModal::None)>"Cancel"</button>
                            <button
                                type="button"
                                class="btn btn-primary"
                                on:click=move |_| {
                                    let url = img_url.get();
                                    let alt = img_alt.get();
                                    if !url.trim().is_empty() {
                                        on_insert_image_confirm.run((url, alt));
                                        img_url.set(String::new());
                                        img_alt.set(String::new());
                                        active_modal.set(ActiveModal::None);
                                    }
                                }
                            >
                                "Insert Image"
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any(),

            ActiveModal::InsertTable => view! {
                <div class="modal-backdrop" on:click=move |_| active_modal.set(ActiveModal::None)>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Insert Table"</h3>
                            <button type="button" class="modal-close-btn" on:click=move |_| active_modal.set(ActiveModal::None)>"×"</button>
                        </div>
                        <div class="modal-body">
                            <div class="modal-grid-2">
                                <div>
                                    <label class="modal-label">"Rows"</label>
                                    <input
                                        type="number"
                                        class="modal-input"
                                        min="1"
                                        max="50"
                                        prop:value=move || table_rows.get()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                                                table_rows.set(v.clamp(1, 50));
                                            }
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="modal-label">"Columns"</label>
                                    <input
                                        type="number"
                                        class="modal-input"
                                        min="1"
                                        max="20"
                                        prop:value=move || table_cols.get()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                                                table_cols.set(v.clamp(1, 20));
                                            }
                                        }
                                    />
                                </div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-secondary" on:click=move |_| active_modal.set(ActiveModal::None)>"Cancel"</button>
                            <button
                                type="button"
                                class="btn btn-primary"
                                on:click=move |_| {
                                    on_insert_table_confirm.run((table_rows.get(), table_cols.get()));
                                    active_modal.set(ActiveModal::None);
                                }
                            >
                                "Create Table"
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any(),

            ActiveModal::Export => view! {
                <div class="modal-backdrop" on:click=move |_| active_modal.set(ActiveModal::None)>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Export Document"</h3>
                            <button type="button" class="modal-close-btn" on:click=move |_| active_modal.set(ActiveModal::None)>"×"</button>
                        </div>
                        <div class="modal-body">
                            <div class="export-options">
                                <button
                                    type="button"
                                    class="export-card-btn"
                                    on:click=move |_| {
                                        on_export_confirm.run("html");
                                        active_modal.set(ActiveModal::None);
                                    }
                                >
                                    <div class="exp-icon">"🌐"</div>
                                    <div class="exp-info">
                                        <h4>"Standalone HTML Document"</h4>
                                        <p>"Export as ready-to-publish web page with embedded styling"</p>
                                    </div>
                                </button>
                                <button
                                    type="button"
                                    class="export-card-btn"
                                    on:click=move |_| {
                                        on_export_confirm.run("md");
                                        active_modal.set(ActiveModal::None);
                                    }
                                >
                                    <div class="exp-icon">"📝"</div>
                                    <div class="exp-info">
                                        <h4>"Markdown File (.md)"</h4>
                                        <p>"Download clean CommonMark / GFM formatted text file"</p>
                                    </div>
                                </button>
                                <button
                                    type="button"
                                    class="export-card-btn"
                                    on:click=move |_| {
                                        on_export_confirm.run("print");
                                        active_modal.set(ActiveModal::None);
                                    }
                                >
                                    <div class="exp-icon">"🖨️"</div>
                                    <div class="exp-info">
                                        <h4>"Print / Save to PDF"</h4>
                                        <p>"Open system print dialog formatted for print paper"</p>
                                    </div>
                                </button>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-secondary" on:click=move |_| active_modal.set(ActiveModal::None)>"Close"</button>
                        </div>
                    </div>
                </div>
            }.into_any(),

            ActiveModal::NewFile => view! {
                <div class="modal-backdrop" on:click=move |_| active_modal.set(ActiveModal::None)>
                    <div class="modal-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Create New File"</h3>
                            <button type="button" class="modal-close-btn" on:click=move |_| active_modal.set(ActiveModal::None)>"×"</button>
                        </div>
                        <div class="modal-body">
                            <label class="modal-label">"File Name"</label>
                            <input
                                type="text"
                                class="modal-input"
                                placeholder="notes.md"
                                prop:value=move || new_filename.get()
                                on:input=move |ev| new_filename.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-secondary" on:click=move |_| active_modal.set(ActiveModal::None)>"Cancel"</button>
                            <button
                                type="button"
                                class="btn btn-primary"
                                on:click=move |_| {
                                    let name = new_filename.get();
                                    if !name.trim().is_empty() {
                                        on_new_file_confirm.run(name);
                                        active_modal.set(ActiveModal::None);
                                    }
                                }
                            >
                                "Create"
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any(),

            ActiveModal::Help => view! {
                <div class="modal-backdrop" on:click=move |_| active_modal.set(ActiveModal::None)>
                    <div class="modal-dialog modal-lg" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-header">
                            <h3>"MdTerm Help & Shortcuts"</h3>
                            <button type="button" class="modal-close-btn" on:click=move |_| active_modal.set(ActiveModal::None)>"×"</button>
                        </div>
                        <div class="modal-body">
                            <div class="help-section">
                                <h4>"Editor Shortcuts"</h4>
                                <div class="shortcuts-table">
                                    <div class="sc-row"><kbd>"Ctrl + S"</kbd><span>"Save active document"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + O"</kbd><span>"Open file"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + N"</kbd><span>"New document"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + B"</kbd><span>"Toggle bold / toggle sidebar"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + I"</kbd><span>"Toggle italic"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + K"</kbd><span>"Insert link"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + F"</kbd><span>"Find and replace"</span></div>
                                    <div class="sc-row"><kbd>"Ctrl + 1..6"</kbd><span>"Format heading level 1..6"</span></div>
                                </div>
                            </div>
                            <div class="help-section">
                                <h4>"Markdown Quick Triggers (WYSIWYG mode)"</h4>
                                <div class="shortcuts-table">
                                    <div class="sc-row"><code>"# "</code><span>"Heading 1"</span></div>
                                    <div class="sc-row"><code>"## "</code><span>"Heading 2"</span></div>
                                    <div class="sc-row"><code>"### "</code><span>"Heading 3"</span></div>
                                    <div class="sc-row"><code>"- "</code> or <code>"* "</code><span>"Bulleted list"</span></div>
                                    <div class="sc-row"><code>"1. "</code><span>"Numbered list"</span></div>
                                    <div class="sc-row"><code>"[] "</code> or <code>"- [ ] "</code><span>"Interactive checklist"</span></div>
                                    <div class="sc-row"><code>"> "</code><span>"Blockquote"</span></div>
                                    <div class="sc-row"><code>"```"</code><span>"Code block"</span></div>
                                    <div class="sc-row"><code>"---"</code><span>"Horizontal rule divider"</span></div>
                                    <div class="sc-row"><code>"/"</code><span>"Open slash command menu"</span></div>
                                </div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-primary" on:click=move |_| active_modal.set(ActiveModal::None)>"Got it"</button>
                        </div>
                    </div>
                </div>
            }.into_any(),
        }}
    }
}
