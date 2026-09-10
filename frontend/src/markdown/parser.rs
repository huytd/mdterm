use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use super::outline::slugify;

pub fn markdown_to_html(markdown: &str, is_editable: bool) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(markdown, options);
    let mut custom_events = Vec::new();

    let mut in_heading = false;
    let mut current_heading_level = 1;
    let mut current_heading_text = String::new();

    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();

    let mut task_counter = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                current_heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let id = slugify(&current_heading_text);
                let tag_open = format!(
                    "<h{} id=\"{}\" class=\"md-heading md-h{}\">",
                    current_heading_level, id, current_heading_level
                );
                let tag_close = format!("</h{}>", current_heading_level);
                
                custom_events.push(Event::Html(tag_open.into()));
                custom_events.push(Event::Text(current_heading_text.clone().into()));
                custom_events.push(Event::Html(tag_close.into()));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code_block_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                let display_lang = if code_block_lang.trim().is_empty() {
                    "text".to_string()
                } else {
                    code_block_lang.trim().to_string()
                };

                // Escape HTML for code content
                let escaped_code = html_escape::encode_text(&code_block_content).to_string();

                let code_html = if display_lang.eq_ignore_ascii_case("mermaid") {
                    if is_editable {
                        format!(
                            "<div class=\"code-block-wrapper mermaid-block-wrapper\" data-lang=\"mermaid\">\
                                <div class=\"code-block-header\" contenteditable=\"false\">\
                                    <div class=\"code-lang-tag\">\
                                        <svg viewBox=\"0 0 24 24\" width=\"13\" height=\"13\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" style=\"margin-right: 5px; vertical-align: -1px;\">\
                                            <circle cx=\"6\" cy=\"6\" r=\"3\"></circle>\
                                            <circle cx=\"6\" cy=\"18\" r=\"3\"></circle>\
                                            <circle cx=\"18\" cy=\"12\" r=\"3\"></circle>\
                                            <line x1=\"8.5\" y1=\"7.5\" x2=\"15.5\" y2=\"10.5\"></line>\
                                            <line x1=\"8.5\" y1=\"16.5\" x2=\"15.5\" y2=\"13.5\"></line>\
                                        </svg>\
                                        <span>mermaid</span>\
                                    </div>\
                                    <div class=\"mermaid-view-switcher\">\
                                        <button type=\"button\" class=\"mermaid-tab-btn active\" data-tab=\"diagram\" onclick=\"window._toggleMermaidView(this, 'diagram')\">Diagram</button>\
                                        <button type=\"button\" class=\"mermaid-tab-btn\" data-tab=\"code\" onclick=\"window._toggleMermaidView(this, 'code')\">Code</button>\
                                    </div>\
                                    <button type=\"button\" class=\"code-copy-btn\" onclick=\"navigator.clipboard.writeText(this.closest('.code-block-wrapper').querySelector('code').innerText);this.innerText='Copied!';setTimeout(()=>this.innerText='Copy',1500)\">Copy</button>\
                                </div>\
                                <!-- MERMAID_PREVIEW_START -->\
                                <div class=\"mermaid-preview-container\" contenteditable=\"false\">\
                                    <div class=\"mermaid-preview-target\" data-raw-code=\"{}\">\
                                        <div class=\"mermaid-loading\">Rendering diagram...</div>\
                                    </div>\
                                </div>\
                                <!-- MERMAID_PREVIEW_END -->\
                                <pre class=\"mermaid-code-pre\" style=\"display: none;\"><code class=\"language-mermaid\">{}</code></pre>\
                            </div>",
                            escaped_code, escaped_code
                        )
                    } else {
                        format!(
                            "<div class=\"mermaid-export-wrapper\"><div class=\"mermaid\">{}</div></div>",
                            code_block_content
                        )
                    }
                } else if is_editable {
                    format!(
                        "<div class=\"code-block-wrapper\" data-lang=\"{}\">\
                            <div class=\"code-block-header\" contenteditable=\"false\">\
                                <span class=\"code-lang-tag\">{}</span>\
                                <button type=\"button\" class=\"code-copy-btn\" onclick=\"navigator.clipboard.writeText(this.closest('.code-block-wrapper').querySelector('code').innerText);this.innerText='Copied!';setTimeout(()=>this.innerText='Copy',1500)\">Copy</button>\
                            </div>\
                            <pre><code class=\"language-{}\">{}</code></pre>\
                        </div>",
                        display_lang, display_lang, display_lang, escaped_code
                    )
                } else {
                    format!(
                        "<div class=\"code-block-wrapper\">\
                            <div class=\"code-block-header\">\
                                <span class=\"code-lang-tag\">{}</span>\
                                <button type=\"button\" class=\"code-copy-btn\" onclick=\"navigator.clipboard.writeText(this.closest('.code-block-wrapper').querySelector('code').innerText);this.innerText='Copied!';setTimeout(()=>this.innerText='Copy',1500)\">Copy</button>\
                            </div>\
                            <pre><code class=\"language-{}\">{}</code></pre>\
                        </div>",
                        display_lang, display_lang, escaped_code
                    )
                };

                custom_events.push(Event::Html(code_html.into()));
            }
            Event::TaskListMarker(checked) => {
                let disabled_attr = if is_editable { "" } else { "disabled" };
                let checked_attr = if checked { "checked" } else { "" };
                let checkbox_html = format!(
                    "<input type=\"checkbox\" class=\"md-task-checkbox\" data-task-idx=\"{}\" {} {} /> ",
                    task_counter, checked_attr, disabled_attr
                );
                task_counter += 1;
                custom_events.push(Event::Html(checkbox_html.into()));
            }
            Event::Start(Tag::Table(_)) => {
                custom_events.push(Event::Html("<div class=\"table-responsive\"><table class=\"md-table\">".into()));
            }
            Event::End(TagEnd::Table) => {
                custom_events.push(Event::Html("</table></div>".into()));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                custom_events.push(Event::Html("<blockquote class=\"md-blockquote\">".into()));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                custom_events.push(Event::Html("</blockquote>".into()));
            }
            _ => {
                if in_heading {
                    if let Event::Text(text) = &event {
                        current_heading_text.push_str(text);
                    }
                } else if in_code_block {
                    if let Event::Text(text) = &event {
                        code_block_content.push_str(text);
                    }
                } else {
                    custom_events.push(event);
                }
            }
        }
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, custom_events.into_iter());
    html_output
}
