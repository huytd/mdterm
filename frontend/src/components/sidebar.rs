use leptos::prelude::*;
use crate::state::{DocumentStats, FileEntry, OutlineItem, SidebarTab};

#[component]
pub fn Sidebar(
    is_open: RwSignal<bool>,
    current_tab: RwSignal<SidebarTab>,
    files: RwSignal<Vec<FileEntry>>,
    current_dir: RwSignal<String>,
    outline: Signal<Vec<OutlineItem>>,
    stats: Signal<DocumentStats>,
    on_open_file: Callback<String>,
    on_new_file: Callback<()>,
    on_delete_file: Callback<String>,
    on_load_template: Callback<&'static str>,
    on_jump_to_heading: Callback<String>,
) -> impl IntoView {
    view! {
        <aside class=move || if is_open.get() { "app-sidebar open" } else { "app-sidebar closed" }>
            <div class="sidebar-tabs-nav">
                <button
                    type="button"
                    class=move || if current_tab.get() == SidebarTab::Files { "sb-nav-btn active" } else { "sb-nav-btn" }
                    on:click=move |_| current_tab.set(SidebarTab::Files)
                >
                    <svg class="sb-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                    </svg>
                    <span>"Files"</span>
                </button>
                <button
                    type="button"
                    class=move || if current_tab.get() == SidebarTab::Outline { "sb-nav-btn active" } else { "sb-nav-btn" }
                    on:click=move |_| current_tab.set(SidebarTab::Outline)
                >
                    <svg class="sb-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="8" y1="6" x2="21" y2="6"></line>
                        <line x1="8" y1="12" x2="21" y2="12"></line>
                        <line x1="8" y1="18" x2="21" y2="18"></line>
                        <line x1="3" y1="6" x2="3.01" y2="6"></line>
                        <line x1="3" y1="12" x2="3.01" y2="12"></line>
                        <line x1="3" y1="18" x2="3.01" y2="18"></line>
                    </svg>
                    <span>"Outline"</span>
                </button>
                <button
                    type="button"
                    class=move || if current_tab.get() == SidebarTab::Stats { "sb-nav-btn active" } else { "sb-nav-btn" }
                    on:click=move |_| current_tab.set(SidebarTab::Stats)
                >
                    <svg class="sb-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="18" y1="20" x2="18" y2="10"></line>
                        <line x1="12" y1="20" x2="12" y2="4"></line>
                        <line x1="6" y1="20" x2="6" y2="14"></line>
                    </svg>
                    <span>"Stats"</span>
                </button>
            </div>

            <div class="sidebar-content">
                {move || match current_tab.get() {
                    SidebarTab::Files => view! {
                        <div class="sidebar-pane files-pane">
                            <div class="pane-header">
                                <div class="dir-info" title=move || current_dir.get()>
                                    <svg class="dir-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                                    </svg>
                                    <span class="dir-name">{move || current_dir.get()}</span>
                                </div>
                                <button
                                    type="button"
                                    class="pane-action-btn"
                                    title="New File"
                                    on:click=move |_| on_new_file.run(())
                                >
                                    <svg class="pane-action-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M12 5v14M5 12h14"></path>
                                    </svg>
                                </button>
                            </div>

                            <div class="files-list">
                                {move || {
                                    let list = files.get();
                                    if list.is_empty() {
                                        view! {
                                            <div class="empty-state">"No files in folder"</div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="file-entries">
                                                {list.into_iter().map(|f| {
                                                    let path_clone = f.path.clone();
                                                    let path_del = f.path.clone();
                                                    let is_dir = f.is_dir;
                                                    let name = f.name.clone();

                                                    view! {
                                                        <div class="file-entry-item">
                                                            <div
                                                                class="file-entry-content"
                                                                on:click=move |_| {
                                                                    if !is_dir {
                                                                        on_open_file.run(path_clone.clone());
                                                                    }
                                                                }
                                                            >
                                                                <svg class="file-item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                                    {if is_dir {
                                                                        view! { <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path> }.into_any()
                                                                    } else {
                                                                        view! {
                                                                            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                                                                            <polyline points="14 2 14 8 20 8"></polyline>
                                                                        }.into_any()
                                                                    }}
                                                                </svg>
                                                                <span class="file-item-name">{name}</span>
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="file-del-btn"
                                                                title="Delete file"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    on_delete_file.run(path_del.clone());
                                                                }
                                                            >
                                                                "×"
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>

                            <div class="templates-section">
                                <div class="section-title">"SAMPLE TEMPLATES"</div>
                                <div class="template-buttons">
                                    <button
                                        type="button"
                                        class="template-item-btn"
                                        on:click=move |_| on_load_template.run("welcome")
                                    >
                                        <span class="tpl-icon">"✨"</span>
                                        <span>"Welcome & Features"</span>
                                    </button>
                                    <button
                                        type="button"
                                        class="template-item-btn"
                                        on:click=move |_| on_load_template.run("guide")
                                    >
                                        <span class="tpl-icon">"📖"</span>
                                        <span>"Markdown Syntax Guide"</span>
                                    </button>
                                    <button
                                        type="button"
                                        class="template-item-btn"
                                        on:click=move |_| on_load_template.run("html")
                                    >
                                        <span class="tpl-icon">"🌐"</span>
                                        <span>"HTML Document Sample"</span>
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any(),

                    SidebarTab::Outline => view! {
                        <div class="sidebar-pane outline-pane">
                            <div class="pane-header">
                                <span class="pane-title">"DOCUMENT OUTLINE"</span>
                            </div>
                            <div class="outline-list">
                                {move || {
                                    let items = outline.get();
                                    if items.is_empty() {
                                        view! {
                                            <div class="empty-state">"No headings in document"</div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="outline-items">
                                                {items.into_iter().map(|item| {
                                                    let id = item.id.clone();
                                                    let level_class = format!("outline-item level-{}", item.level);
                                                    view! {
                                                        <div
                                                            class=level_class
                                                            on:click=move |_| on_jump_to_heading.run(id.clone())
                                                        >
                                                            <span class="outline-dot">"•"</span>
                                                            <span class="outline-text">{item.text}</span>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any(),

                    SidebarTab::Stats => view! {
                        <div class="sidebar-pane stats-pane">
                            <div class="pane-header">
                                <span class="pane-title">"STATISTICS"</span>
                            </div>
                            <div class="stats-grid">
                                <div class="stat-card">
                                    <span class="stat-val">{move || stats.get().words}</span>
                                    <span class="stat-lbl">"Words"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-val">{move || stats.get().chars}</span>
                                    <span class="stat-lbl">"Characters"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-val">{move || stats.get().chars_no_spaces}</span>
                                    <span class="stat-lbl">"Chars (no spaces)"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-val">{move || stats.get().lines}</span>
                                    <span class="stat-lbl">"Lines"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-val">{move || stats.get().paragraphs}</span>
                                    <span class="stat-lbl">"Paragraphs"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-val">{move || format!("{:.1}m", stats.get().reading_time_mins)}</span>
                                    <span class="stat-lbl">"Reading Time"</span>
                                </div>
                            </div>
                        </div>
                    }.into_any(),
                }}
            </div>
        </aside>
    }
}
