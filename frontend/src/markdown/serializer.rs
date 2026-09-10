use htmd::HtmlToMarkdown;

pub fn html_to_markdown(html: &str) -> String {
    let mut clean_html = html.to_string();

    // 0. Remove mermaid preview containers
    while let Some(start) = clean_html.find("<!-- MERMAID_PREVIEW_START -->") {
        if let Some(end) = clean_html[start..].find("<!-- MERMAID_PREVIEW_END -->") {
            let end_idx = start + end + "<!-- MERMAID_PREVIEW_END -->".len();
            clean_html.replace_range(start..end_idx, "");
        } else {
            break;
        }
    }

    // 1. Remove code block headers (which contain the copy button, view switcher, and language tag)
    while let Some(start) = clean_html.find("<div class=\"code-block-header\"") {
        if let Some(end) = clean_html[start..].find("</div>") {
            clean_html.replace_range(start..start + end + 6, "");
        } else {
            break;
        }
    }

    // 2. Normalize checkboxes
    // Checked checkbox
    let mut processed_html = String::with_capacity(clean_html.len());
    let mut remainder = clean_html.as_str();

    while let Some(pos) = remainder.find("<input") {
        processed_html.push_str(&remainder[..pos]);
        let after_input = &remainder[pos..];
        if let Some(tag_end) = after_input.find('>') {
            let tag_content = &after_input[..=tag_end];
            if tag_content.contains("type=\"checkbox\"") || tag_content.contains("type='checkbox'") {
                if tag_content.contains("checked") {
                    processed_html.push_str("[x] ");
                } else {
                    processed_html.push_str("[ ] ");
                }
            } else {
                processed_html.push_str(tag_content);
            }
            remainder = &after_input[tag_end + 1..];
        } else {
            processed_html.push_str(after_input);
            remainder = "";
            break;
        }
    }
    processed_html.push_str(remainder);

    // 3. Strip table and code wrapper divs
    let unwrapped_html = processed_html
        .replace("<div class=\"table-responsive\">", "")
        .replace("<div class=\"code-block-wrapper mermaid-block-wrapper\" data-lang=\"mermaid\">", "")
        .replace("<div class=\"code-block-wrapper\">", "")
        .replace("style=\"display: none;\"", "")
        .replace("class=\"mermaid-code-pre\"", "")
        .replace("</div>", "");

    // 4. Convert using htmd
    let converter = HtmlToMarkdown::new();
    let mut md = match converter.convert(&unwrapped_html) {
        Ok(markdown) => markdown,
        Err(_) => unwrapped_html,
    };

    // 5. Post-process markdown
    // Unescape task list markers
    md = md.replace(r"\[x\]", "[x]").replace(r"\[ \]", "[ ]");

    // Fix task list bullet items to standard markdown format: `* [x]` or `- [x]`
    let mut final_lines = Vec::new();
    for line in md.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("*   [x]") || trimmed.starts_with("-   [x]") {
            let indent = &line[..line.len() - trimmed.len()];
            final_lines.push(format!("{}- [x] {}", indent, &trimmed[7..]));
        } else if trimmed.starts_with("*   [ ]") || trimmed.starts_with("-   [ ]") {
            let indent = &line[..line.len() - trimmed.len()];
            final_lines.push(format!("{}- [ ] {}", indent, &trimmed[7..]));
        } else {
            final_lines.push(line.to_string());
        }
    }

    final_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::markdown_to_html;

    #[test]
    fn test_mermaid_roundtrip() {
        let original_md = "```mermaid\ngraph TD\n    A[Start] --> B[End]\n```";
        let html = markdown_to_html(original_md, true);
        assert!(html.contains("mermaid-preview-container"));
        assert!(html.contains("class=\"language-mermaid\""));

        let converted_md = html_to_markdown(&html);
        assert!(converted_md.contains("```mermaid"));
        assert!(converted_md.contains("graph TD"));
        assert!(converted_md.contains("A[Start] --> B[End]"));
    }
}
