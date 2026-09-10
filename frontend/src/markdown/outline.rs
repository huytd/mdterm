use crate::state::OutlineItem;

pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut last_was_dash = false;

    for c in text.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_was_dash = false;
        } else if (c == ' ' || c == '-' || c == '_') && !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    }
}

#[allow(dead_code)]
pub fn extract_outline(markdown: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        if trimmed.starts_with('#') {
            let mut level = 0;
            for c in trimmed.chars() {
                if c == '#' {
                    level += 1;
                } else {
                    break;
                }
            }

            if level > 0 && level <= 6 {
                let rest = trimmed[level..].trim();
                if !rest.is_empty() {
                    let clean_text = rest
                        .trim_start_matches(|c| c == ' ' || c == '\t')
                        .to_string();
                    let id = slugify(&clean_text);
                    items.push(OutlineItem {
                        level,
                        text: clean_text,
                        id,
                    });
                }
            }
        }
    }

    items
}
