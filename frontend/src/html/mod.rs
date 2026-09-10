use crate::state::OutlineItem;
use super::markdown::outline::slugify;

/// Represents the decomposition of an HTML document into its outer envelope
/// (doctype, html, head, body tags) and inner body content for editing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlEnvelope {
    /// Everything preceding the inner body content (e.g. `<!DOCTYPE html><html><head>...</head><body ...>`).
    pub header: String,
    /// The inner HTML content of the `<body>` element (or the whole snippet if no `<body>` tag).
    pub body: String,
    /// Everything following the inner body content (e.g. `</body></html>`).
    pub footer: String,
    /// True if an actual HTML document structure (like `<body>` or `<html>`) was detected.
    pub has_envelope: bool,
}

impl HtmlEnvelope {
    /// Parses an HTML document into envelope and body.
    pub fn parse(raw_html: &str) -> Self {
        let lower = raw_html.to_lowercase();

        // 1. Try to locate `<body` tag
        if let Some(body_start_pos) = find_tag_start(&lower, "<body") {
            // Find closing `>` of `<body...>`
            if let Some(body_open_end) = find_tag_close(raw_html, body_start_pos) {
                let content_start = body_open_end + 1;
                // Find `</body`
                let (content_end, footer_start) = if let Some(body_close_pos) = lower[content_start..].find("</body") {
                    let abs_close = content_start + body_close_pos;
                    (abs_close, abs_close)
                } else {
                    (raw_html.len(), raw_html.len())
                };

                let header = raw_html[..content_start].to_string();
                let body = raw_html[content_start..content_end].to_string();
                let footer = if footer_start < raw_html.len() {
                    raw_html[footer_start..].to_string()
                } else {
                    "\n</body>\n</html>".to_string()
                };

                return HtmlEnvelope {
                    header,
                    body,
                    footer,
                    has_envelope: true,
                };
            }
        }

        // 2. Try to locate `<html` tag if no `<body>`
        if let Some(html_start_pos) = find_tag_start(&lower, "<html") {
            if let Some(html_open_end) = find_tag_close(raw_html, html_start_pos) {
                let content_start = html_open_end + 1;
                let (content_end, footer_start) = if let Some(html_close_pos) = lower[content_start..].find("</html") {
                    let abs_close = content_start + html_close_pos;
                    (abs_close, abs_close)
                } else {
                    (raw_html.len(), raw_html.len())
                };

                let header = raw_html[..content_start].to_string();
                let body = raw_html[content_start..content_end].to_string();
                let footer = if footer_start < raw_html.len() {
                    raw_html[footer_start..].to_string()
                } else {
                    "\n</html>".to_string()
                };

                return HtmlEnvelope {
                    header,
                    body,
                    footer,
                    has_envelope: true,
                };
            }
        }

        // 3. HTML fragment / snippet
        HtmlEnvelope {
            header: String::new(),
            body: raw_html.to_string(),
            footer: String::new(),
            has_envelope: false,
        }
    }

    /// Reassembles the full document with updated body content.
    pub fn reassemble(&self, new_body: &str) -> String {
        if self.has_envelope {
            format!("{}{}{}", self.header, new_body, self.footer)
        } else {
            new_body.to_string()
        }
    }

    /// Prepares HTML for rendering in Split mode preview safely.
    pub fn render_preview(&self) -> String {
        if !self.has_envelope {
            return self.body.clone();
        }

        // Extract any `<style>` tags from header to apply scoped preview styles
        let mut preview = String::new();
        let lower_header = self.header.to_lowercase();
        let mut search_from = 0;

        while let Some(style_start) = lower_header[search_from..].find("<style") {
            let abs_start = search_from + style_start;
            if let Some(close_tag_start) = lower_header[abs_start..].find("</style>") {
                let abs_end = abs_start + close_tag_start + 8;
                let style_block = &self.header[abs_start..abs_end];
                // Scope `body` selectors in styles to `.preview-surface` so it doesn't leak
                let scoped_style = style_block
                    .replace("body {", ".preview-surface {")
                    .replace("body{", ".preview-surface {")
                    .replace("html {", ".preview-surface {")
                    .replace("html{", ".preview-surface {");
                preview.push_str(&scoped_style);
                search_from = abs_end;
            } else {
                break;
            }
        }

        preview.push_str(&self.body);
        preview
    }

    /// Ensures the content is a complete, well-formed standalone HTML document.
    pub fn ensure_full_document(&self, default_title: &str) -> String {
        if self.has_envelope && self.header.to_lowercase().contains("<!doctype") {
            self.reassemble(&self.body)
        } else {
            format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{}</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; max-width: 860px; margin: 40px auto; padding: 0 20px; color: #1e293b; background: #ffffff; }}
    h1, h2, h3, h4 {{ color: #0f172a; margin-top: 1.5em; margin-bottom: 0.5em; }}
    h1 {{ border-bottom: 2px solid #e2e8f0; padding-bottom: 8px; }}
    code {{ background: #f1f5f9; padding: 2px 6px; border-radius: 4px; font-family: monospace; font-size: 0.9em; }}
    pre {{ background: #0f172a; color: #f8fafc; padding: 16px; border-radius: 8px; overflow-x: auto; }}
    blockquote {{ border-left: 4px solid #3b82f6; margin: 1.5em 0; padding-left: 16px; color: #475569; font-style: italic; }}
    table {{ width: 100%; border-collapse: collapse; margin: 1.5em 0; }}
    th, td {{ border: 1px solid #cbd5e1; padding: 10px 14px; text-align: left; }}
    th {{ background: #f8fafc; font-weight: 600; }}
  </style>
</head>
<body>
{}
</body>
</html>"#,
                default_title, self.body
            )
        }
    }
}

/// Helper to locate an HTML opening tag like `<body` or `<html`
fn find_tag_start(lower_html: &str, tag_prefix: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(pos) = lower_html[search_from..].find(tag_prefix) {
        let abs_pos = search_from + pos;
        let after_tag = abs_pos + tag_prefix.len();
        if after_tag >= lower_html.len() {
            return Some(abs_pos);
        }
        let next_ch = lower_html.as_bytes()[after_tag];
        if next_ch == b' ' || next_ch == b'\t' || next_ch == b'\r' || next_ch == b'\n' || next_ch == b'>' || next_ch == b'/' {
            return Some(abs_pos);
        }
        search_from = after_tag;
    }
    None
}

/// Helper to find the matching `>` closing an HTML tag, ignoring quoted attribute values.
fn find_tag_close(html: &str, start_pos: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut in_quote: Option<u8> = None;
    let mut i = start_pos;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == q {
                in_quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            in_quote = Some(b);
        } else if b == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Checks if a file should be treated as HTML based on its filename and content.
pub fn is_html_file(filename: &str, content: &str) -> bool {
    let lower = filename.to_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") || lower.ends_with(".xhtml") {
        return true;
    }
    if lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".mdown") || lower.ends_with(".mkd") {
        return false;
    }
    is_html_content(content)
}

/// Checks if text content looks like an HTML document or snippet.
pub fn is_html_content(content: &str) -> bool {
    let trimmed = content.trim_start();
    let lower = trimmed.to_lowercase();
    lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || (lower.contains("<head") && lower.contains("</head>"))
        || (lower.contains("<body") && lower.contains("</body>"))
        || (lower.starts_with("<div") && lower.ends_with("</div>"))
        || (lower.starts_with("<p>") && lower.contains("</p>"))
}

/// Cleans temporary editor artifacts from innerHTML before storing as document content.
pub fn clean_html_editor_output(html: &str) -> String {
    let mut clean = html.to_string();

    // 1. Remove mermaid preview containers
    while let Some(start) = clean.find("<!-- MERMAID_PREVIEW_START -->") {
        if let Some(end) = clean[start..].find("<!-- MERMAID_PREVIEW_END -->") {
            let end_idx = start + end + "<!-- MERMAID_PREVIEW_END -->".len();
            clean.replace_range(start..end_idx, "");
        } else {
            break;
        }
    }

    // 2. Remove code block headers (copy button, switcher, language tag)
    while let Some(start) = clean.find("<div class=\"code-block-header\"") {
        if let Some(end) = clean[start..].find("</div>") {
            clean.replace_range(start..start + end + 6, "");
        } else {
            break;
        }
    }

    // 3. Remove display: none added to code block pre when mermaid preview is active
    clean = clean.replace("style=\"display: none;\"", "");
    clean = clean.replace("style=\"display:none;\"", "");
    clean = clean.replace("class=\"mermaid-code-pre\"", "");

    clean
}

/// Extracts document outline (headings H1-H6) from HTML.
pub fn extract_html_outline(html: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    let lower = html.to_lowercase();
    let bytes = html.as_bytes();
    let mut search_from = 0;

    while search_from < bytes.len() {
        // Look for `<h`
        if let Some(pos) = lower[search_from..].find("<h") {
            let tag_start = search_from + pos;
            if tag_start + 2 < bytes.len() {
                let level_byte = bytes[tag_start + 2];
                if (b'1'..=b'6').contains(&level_byte) {
                    let level = (level_byte - b'0') as usize;
                    let after_level = tag_start + 3;
                    if after_level < bytes.len() {
                        let next_b = bytes[after_level];
                        if next_b == b' ' || next_b == b'>' || next_b == b'\t' || next_b == b'\n' {
                            if let Some(tag_close) = find_tag_close(html, tag_start) {
                                let tag_header = &html[tag_start..=tag_close];
                                let close_tag = format!("</h{}>", level);
                                let close_tag_lower = format!("</h{}>", level);
                                let content_start = tag_close + 1;

                                if let Some(end_pos) = lower[content_start..].find(&close_tag_lower) {
                                    let content_end = content_start + end_pos;
                                    let inner_html = &html[content_start..content_end];
                                    let clean_text = strip_html_tags(inner_html).trim().to_string();

                                    if !clean_text.is_empty() {
                                        // Check if tag had an id attribute
                                        let id = extract_tag_attribute(tag_header, "id")
                                            .unwrap_or_else(|| slugify(&clean_text));

                                        items.push(OutlineItem {
                                            level,
                                            text: clean_text,
                                            id,
                                        });
                                    }
                                    search_from = content_end + close_tag.len();
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            search_from = tag_start + 2;
        } else {
            break;
        }
    }

    items
}

/// Extracts a named attribute value from an HTML tag string.
fn extract_tag_attribute(tag_str: &str, attr_name: &str) -> Option<String> {
    let lower = tag_str.to_lowercase();
    let search = format!("{}=", attr_name.to_lowercase());
    if let Some(pos) = lower.find(&search) {
        let after_eq = pos + search.len();
        let bytes = tag_str.as_bytes();
        if after_eq < bytes.len() {
            let quote = bytes[after_eq];
            if quote == b'"' || quote == b'\'' {
                let val_start = after_eq + 1;
                if let Some(quote_end) = bytes[val_start..].iter().position(|&b| b == quote) {
                    return Some(tag_str[val_start..val_start + quote_end].to_string());
                }
            } else {
                let val_start = after_eq;
                let end = bytes[val_start..]
                    .iter()
                    .position(|&b| b == b' ' || b == b'>' || b == b'\t' || b == b'\r' || b == b'\n')
                    .unwrap_or(bytes.len() - val_start);
                return Some(tag_str[val_start..val_start + end].to_string());
            }
        }
    }
    None
}

/// Strips HTML tags and decodes basic entities to produce clean readable text.
pub fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut is_block_tag = false;
    let lower = html.to_lowercase();

    while i < bytes.len() {
        if !in_tag && bytes[i] == b'<' {
            let rem = &lower[i..];
            if rem.starts_with("<script") {
                in_script = true;
            } else if rem.starts_with("</script>") {
                in_script = false;
            } else if rem.starts_with("<style") {
                in_style = true;
            } else if rem.starts_with("</style>") {
                in_style = false;
            }

            // Check if block tag boundary
            if rem.starts_with("<p") || rem.starts_with("</p")
                || rem.starts_with("<div") || rem.starts_with("</div")
                || rem.starts_with("<h1") || rem.starts_with("</h1")
                || rem.starts_with("<h2") || rem.starts_with("</h2")
                || rem.starts_with("<h3") || rem.starts_with("</h3")
                || rem.starts_with("<h4") || rem.starts_with("</h4")
                || rem.starts_with("<h5") || rem.starts_with("</h5")
                || rem.starts_with("<h6") || rem.starts_with("</h6")
                || rem.starts_with("<li") || rem.starts_with("</li")
                || rem.starts_with("<br") || rem.starts_with("<hr")
                || rem.starts_with("<tr") || rem.starts_with("</tr")
                || rem.starts_with("<blockquote") || rem.starts_with("</blockquote")
            {
                is_block_tag = true;
            }

            in_tag = true;
            i += 1;
            continue;
        }

        if in_tag {
            if bytes[i] == b'>' {
                in_tag = false;
                if is_block_tag {
                    if !out.ends_with(' ') && !out.ends_with('\n') && !out.is_empty() {
                        out.push(' ');
                    }
                    is_block_tag = false;
                }
            }
            i += 1;
            continue;
        }

        if !in_script && !in_style {
            out.push(bytes[i] as char);
        }
        i += 1;
    }

    let res = out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    res.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_html_file() {
        assert!(is_html_file("index.html", ""));
        assert!(is_html_file("page.HTM", ""));
        assert!(is_html_file("app.xhtml", ""));
        assert!(!is_html_file("README.md", ""));
        assert!(!is_html_file("notes.markdown", ""));

        // Content detection fallback
        assert!(is_html_file("stdin", "<!DOCTYPE html><html><body>Test</body></html>"));
        assert!(is_html_file("unnamed", "<html><head><title>T</title></head><body>B</body></html>"));
        assert!(!is_html_file("stdin", "# Markdown Title\nSome text"));
    }

    #[test]
    fn test_parse_full_html_envelope() {
        let raw = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Test Page</title>
  <style>body { color: red; }</style>
</head>
<body class="main-body" data-theme="dark">
  <h1>Welcome</h1>
  <p>Hello world</p>
</body>
</html>"#;

        let env = HtmlEnvelope::parse(raw);
        assert!(env.has_envelope);
        assert!(env.header.contains("<!DOCTYPE html>"));
        assert!(env.header.contains("<title>Test Page</title>"));
        assert!(env.header.contains("<body class=\"main-body\" data-theme=\"dark\">"));
        assert!(env.body.contains("<h1>Welcome</h1>"));
        assert!(env.body.contains("<p>Hello world</p>"));
        assert!(env.footer.contains("</body>"));
        assert!(env.footer.contains("</html>"));

        // Reassemble with updated body
        let updated = env.reassemble("\n  <h1>Updated Welcome</h1>\n  <p>New paragraph</p>\n");
        assert!(updated.contains("<title>Test Page</title>"));
        assert!(updated.contains("<body class=\"main-body\" data-theme=\"dark\">"));
        assert!(updated.contains("Updated Welcome"));
        assert!(updated.contains("</body>"));
    }

    #[test]
    fn test_parse_html_fragment() {
        let fragment = "<div class=\"card\"><h2>Card Title</h2><p>Description</p></div>";
        let env = HtmlEnvelope::parse(fragment);
        assert!(!env.has_envelope);
        assert_eq!(env.body, fragment);

        let updated = env.reassemble("<h2>New Title</h2>");
        assert_eq!(updated, "<h2>New Title</h2>");
    }

    #[test]
    fn test_extract_html_outline() {
        let html = r#"
            <h1 id="intro">Introduction</h1>
            <p>Some text</p>
            <h2>Getting Started</h2>
            <p>Details</p>
            <h3 class="custom">Step 1: Install</h3>
            <h1 id="conclusion">Conclusion</h1>
        "#;

        let outline = extract_html_outline(html);
        assert_eq!(outline.len(), 4);
        assert_eq!(outline[0].level, 1);
        assert_eq!(outline[0].text, "Introduction");
        assert_eq!(outline[0].id, "intro");

        assert_eq!(outline[1].level, 2);
        assert_eq!(outline[1].text, "Getting Started");
        assert_eq!(outline[1].id, "getting-started");

        assert_eq!(outline[2].level, 3);
        assert_eq!(outline[2].text, "Step 1: Install");

        assert_eq!(outline[3].level, 1);
        assert_eq!(outline[3].text, "Conclusion");
        assert_eq!(outline[3].id, "conclusion");
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "<h1>Title</h1><p>This is <strong>bold</strong> &amp; <em>italic</em> text.</p>";
        let stripped = strip_html_tags(html);
        assert_eq!(stripped, "Title This is bold & italic text.");
    }

    #[test]
    fn test_render_preview_scopes_body() {
        let raw = "<!DOCTYPE html><html><head><style>body { font-size: 18px; }</style></head><body><p>Content</p></body></html>";
        let env = HtmlEnvelope::parse(raw);
        let preview = env.render_preview();
        assert!(preview.contains(".preview-surface { font-size: 18px; }"));
        assert!(!preview.contains("body { font-size: 18px; }"));
    }

    #[test]
    fn test_clean_html_editor_output() {
        let dirty = r#"
            <div>Content</div>
            <!-- MERMAID_PREVIEW_START -->
            <div class="mermaid-preview-container">Preview</div>
            <!-- MERMAID_PREVIEW_END -->
            <div class="code-block-header"><button>Copy</button></div>
            <pre class="mermaid-code-pre" style="display: none;"><code>graph TD</code></pre>
        "#;
        let cleaned = clean_html_editor_output(dirty);
        assert!(!cleaned.contains("MERMAID_PREVIEW_START"));
        assert!(!cleaned.contains("mermaid-preview-container"));
        assert!(!cleaned.contains("code-block-header"));
        assert!(!cleaned.contains("display: none;"));
        assert!(!cleaned.contains("mermaid-code-pre"));
        assert!(cleaned.contains("<div>Content</div>"));
        assert!(cleaned.contains("<code>graph TD</code>"));
    }

    #[test]
    fn test_ensure_full_document() {
        let fragment = "<h1>Title</h1><p>Body</p>";
        let env = HtmlEnvelope::parse(fragment);
        let full = env.ensure_full_document("My Document");
        assert!(full.contains("<!DOCTYPE html>"));
        assert!(full.contains("<title>My Document</title>"));
        assert!(full.contains("<h1>Title</h1><p>Body</p>"));
    }
}
