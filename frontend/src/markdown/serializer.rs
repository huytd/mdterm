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
    crate::html::remove_matched_div(&mut clean_html, "<div class=\"code-block-header\"");

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
        .replace("</table></div>", "</table>")
        .replace("<div class=\"code-block-wrapper mermaid-block-wrapper\" data-lang=\"mermaid\">", "")
        .replace("<div class=\"code-block-wrapper\">", "")
        .replace("style=\"display: none;\"", "")
        .replace("class=\"mermaid-code-pre\"", "");

    // 4. Convert using htmd
    let converter = HtmlToMarkdown::new();
    let mut md = match converter.convert(&unwrapped_html) {
        Ok(markdown) => markdown,
        Err(_) => unwrapped_html,
    };

    // 5. Post-process markdown
    // Unescape task list markers
    md = md.replace(r"\[x\]", "[x]").replace(r"\[ \]", "[ ]");

    // Fix task list bullet items to standard markdown format: `- [x]` or `- [ ]`
    let mut final_lines = Vec::new();
    for line in md.lines() {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        if let Some(rest) = trimmed.strip_prefix("- [x]")
            .or_else(|| trimmed.strip_prefix("* [x]"))
            .or_else(|| trimmed.strip_prefix("-   [x]"))
            .or_else(|| trimmed.strip_prefix("*   [x]"))
        {
            final_lines.push(format!("{}- [x] {}", indent, rest.trim_start()));
        } else if let Some(rest) = trimmed.strip_prefix("- [ ]")
            .or_else(|| trimmed.strip_prefix("* [ ]"))
            .or_else(|| trimmed.strip_prefix("-   [ ]"))
            .or_else(|| trimmed.strip_prefix("*   [ ]"))
        {
            final_lines.push(format!("{}- [ ] {}", indent, rest.trim_start()));
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

    #[test]
    fn test_nested_unclosed_divs() {
        let mut s = String::new();
        for i in 0..2500 {
            s.push_str(&format!("<div>Line {}: this is some long content to make it 40KB+</div>", i));
        }
        assert!(s.len() > 40000);
        println!("Input length: {}", s.len());
        let md = html_to_markdown(&s);
        println!("Converted length: {}", md.len());
        assert!(md.contains("Line 0"));
        assert!(md.contains("Line 2499"));
    }

    #[test]
    fn test_pasted_markdown_in_divs() {
        let html = "<div># My Heading</div><div>Some text</div><div>* bullet 1</div>";
        let md = html_to_markdown(html);
        println!("Converted markdown:\n{}", md);
    }

    #[test]
    fn test_code_block_roundtrip() {
        let original_md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let html = markdown_to_html(original_md, true);
        assert!(html.contains("code-block-wrapper"));
        assert!(html.contains("code-block-header"));

        let converted_md = html_to_markdown(&html);
        assert!(converted_md.contains("```rust"));
        assert!(converted_md.contains("fn main()"));
        assert!(converted_md.contains("println!(\"hello\")"));
    }

    #[test]
    fn test_table_roundtrip() {
        let original_md = "| Col 1 | Col 2 |\n|---|---|\n| Val 1 | Val 2 |";
        let html = markdown_to_html(original_md, true);
        assert!(html.contains("table-responsive"));
        assert!(html.contains("md-table"));

        let converted_md = html_to_markdown(&html);
        assert!(converted_md.contains("Col 1"));
        assert!(converted_md.contains("Col 2"));
        assert!(converted_md.contains("Val 1"));
        assert!(converted_md.contains("Val 2"));
    }

    #[test]
    fn test_large_complex_document_roundtrip() {
        let mut original_md = String::new();
        original_md.push_str("# Large Document Test\n\n");
        for i in 0..500 {
            original_md.push_str(&format!("## Section {}\n\nParagraph for section {} with **bold** and *italic* text.\n\n", i, i));
            if i % 10 == 0 {
                original_md.push_str("```rust\nfn test() {\n    let x = 1;\n}\n```\n\n");
            }
            if i % 20 == 0 {
                original_md.push_str("| Col A | Col B |\n|---|---|\n| Data 1 | Data 2 |\n\n");
            }
            if i % 15 == 0 {
                original_md.push_str("- [x] Done item\n- [ ] Todo item\n\n");
            }
        }
        assert!(original_md.len() >= 40000);
        println!("Original markdown length: {} bytes", original_md.len());

        let html = markdown_to_html(&original_md, true);
        println!("Rendered HTML length: {} bytes", html.len());

        let converted_md = html_to_markdown(&html);
        println!("Converted markdown length: {} bytes", converted_md.len());

        assert!(converted_md.contains("# Large Document Test"));
        assert!(converted_md.contains("## Section 0"));
        assert!(converted_md.contains("## Section 499"));
        assert!(converted_md.contains("```rust"));
        assert!(converted_md.contains("Col A"));
        if let Some(pos) = converted_md.find("Done item") {
            println!("Snippet: {:?}", &converted_md[pos.saturating_sub(10)..pos + 20]);
        }
        assert!(converted_md.contains("- [x] Done item"));
        assert!(converted_md.contains("- [ ] Todo item"));
    }
}
