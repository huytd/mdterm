use leptos::prelude::*;

#[component]
pub fn Toolbar(
    on_format: Callback<&'static str>,
    on_format_block: Callback<&'static str>,
    on_insert_table: Callback<()>,
    on_insert_link: Callback<()>,
    on_insert_image: Callback<()>,
    on_insert_hr: Callback<()>,
    on_find_replace: Callback<()>,
    on_undo: Callback<()>,
    on_redo: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="app-toolbar">
            <div class="toolbar-group">
                <select
                    class="toolbar-select format-block-select"
                    title="Text Style"
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        match val.as_str() {
                            "h1" => on_format_block.run("h1"),
                            "h2" => on_format_block.run("h2"),
                            "h3" => on_format_block.run("h3"),
                            "h4" => on_format_block.run("h4"),
                            "blockquote" => on_format_block.run("blockquote"),
                            "pre" => on_format_block.run("pre"),
                            _ => on_format_block.run("p"),
                        }
                    }
                >
                    <option value="p">"Paragraph"</option>
                    <option value="h1">"Heading 1"</option>
                    <option value="h2">"Heading 2"</option>
                    <option value="h3">"Heading 3"</option>
                    <option value="h4">"Heading 4"</option>
                    <option value="blockquote">"Quote"</option>
                    <option value="pre">"Code Block"</option>
                </select>
            </div>

            <div class="toolbar-separator"></div>

            <div class="toolbar-group">
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Bold (Ctrl+B)"
                    on:click=move |_| on_format.run("bold")
                >
                    <strong>"B"</strong>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Italic (Ctrl+I)"
                    on:click=move |_| on_format.run("italic")
                >
                    <em>"I"</em>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Underline (Ctrl+U)"
                    on:click=move |_| on_format.run("underline")
                >
                    <u>"U"</u>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Strikethrough"
                    on:click=move |_| on_format.run("strikeThrough")
                >
                    <s>"S"</s>
                </button>
                <button
                    type="button"
                    class="toolbar-btn code-btn"
                    title="Inline Code"
                    on:click=move |_| on_format.run("code")
                >
                    "‹/›"
                </button>
            </div>

            <div class="toolbar-separator"></div>

            <div class="toolbar-group">
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Bullet List"
                    on:click=move |_| on_format.run("insertUnorderedList")
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="8" y1="6" x2="21" y2="6"></line>
                        <line x1="8" y1="12" x2="21" y2="12"></line>
                        <line x1="8" y1="18" x2="21" y2="18"></line>
                        <line x1="3" y1="6" x2="3.01" y2="6"></line>
                        <line x1="3" y1="12" x2="3.01" y2="12"></line>
                        <line x1="3" y1="18" x2="3.01" y2="18"></line>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Numbered List"
                    on:click=move |_| on_format.run("insertOrderedList")
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="10" y1="6" x2="21" y2="6"></line>
                        <line x1="10" y1="12" x2="21" y2="12"></line>
                        <line x1="10" y1="18" x2="21" y2="18"></line>
                        <path d="M4 6h1v4"></path>
                        <path d="M4 10h2"></path>
                        <path d="M6 18H4c0-1 2-2 2-3s-1-1.5-2-1"></path>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Task Checklist"
                    on:click=move |_| on_format.run("tasklist")
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="9 11 12 14 22 4"></polyline>
                        <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
                    </svg>
                </button>
            </div>

            <div class="toolbar-separator"></div>

            <div class="toolbar-group">
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Insert Table"
                    on:click=move |_| on_insert_table.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                        <line x1="3" y1="9" x2="21" y2="9"></line>
                        <line x1="3" y1="15" x2="21" y2="15"></line>
                        <line x1="12" y1="3" x2="12" y2="21"></line>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Insert Link (Ctrl+K)"
                    on:click=move |_| on_insert_link.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                        <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Insert Image"
                    on:click=move |_| on_insert_image.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                        <circle cx="8.5" cy="8.5" r="1.5"></circle>
                        <polyline points="21 15 16 10 5 21"></polyline>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Horizontal Rule"
                    on:click=move |_| on_insert_hr.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="3" y1="12" x2="21" y2="12"></line>
                    </svg>
                </button>
            </div>

            <div class="toolbar-separator"></div>

            <div class="toolbar-group">
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Undo (Ctrl+Z)"
                    on:click=move |_| on_undo.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="1 4 1 10 7 10"></polyline>
                        <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Redo (Ctrl+Y)"
                    on:click=move |_| on_redo.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="23 4 23 10 17 10"></polyline>
                        <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                    </svg>
                </button>
                <button
                    type="button"
                    class="toolbar-btn"
                    title="Find and Replace (Ctrl+F)"
                    on:click=move |_| on_find_replace.run(())
                >
                    <svg class="tb-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="11" cy="11" r="8"></circle>
                        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                    </svg>
                </button>
            </div>
        </div>
    }
}
