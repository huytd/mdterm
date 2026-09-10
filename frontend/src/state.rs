use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EditorMode {
    Wysiwyg,
    Split,
    Source,
}

impl EditorMode {
    pub fn label(&self) -> &'static str {
        match self {
            EditorMode::Wysiwyg => "WYSIWYG",
            EditorMode::Split => "Split",
            EditorMode::Source => "Source",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    Nord,
    Monokai,
}

impl Theme {
    pub fn from_class_name(class_name: &str) -> Option<Self> {
        match class_name {
            "theme-dark" => Some(Theme::Dark),
            "theme-light" => Some(Theme::Light),
            "theme-nord" => Some(Theme::Nord),
            "theme-monokai" => Some(Theme::Monokai),
            _ => None,
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self {
            Theme::Dark => "theme-dark",
            Theme::Light => "theme-light",
            Theme::Nord => "theme-nord",
            Theme::Monokai => "theme-monokai",
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Nord => "Nord",
            Theme::Monokai => "Monokai",
        }
    }

    pub fn from_name_or_class(name: &str) -> Option<Self> {
        let clean = name.to_lowercase();
        let clean = clean.trim().strip_prefix("theme-").unwrap_or(&clean);
        match clean {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            "nord" => Some(Theme::Nord),
            "monokai" => Some(Theme::Monokai),
            _ => None,
        }
    }

    pub fn next(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Nord,
            Theme::Nord => Theme::Monokai,
            Theme::Monokai => Theme::Dark,
        }
    }
}

#[cfg(test)]
mod theme_tests {
    use super::Theme;

    #[test]
    fn theme_class_names_round_trip() {
        for theme in [Theme::Dark, Theme::Light, Theme::Nord, Theme::Monokai] {
            assert_eq!(Theme::from_class_name(theme.class_name()), Some(theme));
        }
    }

    #[test]
    fn theme_from_name_or_class() {
        assert_eq!(Theme::from_name_or_class("dark"), Some(Theme::Dark));
        assert_eq!(Theme::from_name_or_class("THEME-DARK"), Some(Theme::Dark));
        assert_eq!(Theme::from_name_or_class("nord"), Some(Theme::Nord));
        assert_eq!(Theme::from_name_or_class("theme-nord"), Some(Theme::Nord));
        assert_eq!(Theme::from_name_or_class("Nord"), Some(Theme::Nord));
        assert_eq!(Theme::from_name_or_class("monokai"), Some(Theme::Monokai));
        assert_eq!(Theme::from_name_or_class("light"), Some(Theme::Light));
        assert_eq!(Theme::from_name_or_class("other"), None);
    }

    #[test]
    fn unknown_theme_class_is_rejected() {
        assert_eq!(Theme::from_class_name("theme-unknown"), None);
    }
}

#[cfg(test)]
mod stats_tests {
    use super::DocumentStats;

    #[test]
    fn test_document_stats_markdown() {
        let md = "# Title\n\nThis is a paragraph with words.";
        let stats = DocumentStats::compute(md, false);
        assert_eq!(stats.words, 8);
        assert_eq!(stats.paragraphs, 2);
    }

    #[test]
    fn test_document_stats_html() {
        let html = "<div><h1>Title</h1><p>This is a paragraph with words.</p></div>";
        let stats = DocumentStats::compute(html, true);
        assert_eq!(stats.words, 7);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EditorPosition {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarTab {
    Files,
    Outline,
    Stats,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentTab {
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub content: String,
    pub is_dirty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    pub level: usize,
    pub text: String,
    pub id: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentStats {
    pub words: usize,
    pub chars: usize,
    pub chars_no_spaces: usize,
    pub lines: usize,
    pub paragraphs: usize,
    pub reading_time_mins: f32,
}

impl Default for DocumentStats {
    fn default() -> Self {
        Self {
            words: 0,
            chars: 0,
            chars_no_spaces: 0,
            lines: 1,
            paragraphs: 1,
            reading_time_mins: 0.0,
        }
    }
}

impl DocumentStats {
    #[allow(dead_code)]
    pub fn compute(text: &str, is_html: bool) -> Self {
        let clean_text = if is_html {
            crate::html::strip_html_tags(text)
        } else {
            text.to_string()
        };
        let chars = clean_text.chars().count();
        let chars_no_spaces = clean_text.chars().filter(|c| !c.is_whitespace()).count();
        let lines = clean_text.lines().count().max(1);
        let words = clean_text.split_whitespace().count();
        let paragraphs = clean_text
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .count()
            .max(1);
        let reading_time_mins = (words as f32 / 200.0).max(0.1);

        Self {
            words,
            chars,
            chars_no_spaces,
            lines,
            paragraphs,
            reading_time_mins,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    InsertLink,
    InsertImage,
    InsertTable,
    Export,
    Help,
    NewFile,
    OpenFile,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FindReplaceState {
    pub is_open: bool,
    pub search_query: String,
    pub replace_query: String,
    pub match_case: bool,
    pub whole_word: bool,
    pub current_match: usize,
    pub total_matches: usize,
}

impl Default for FindReplaceState {
    fn default() -> Self {
        Self {
            is_open: false,
            search_query: String::new(),
            replace_query: String::new(),
            match_case: false,
            whole_word: false,
            current_match: 0,
            total_matches: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SlashMenuState {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
    pub x: f64,
    pub y: f64,
}

impl Default for SlashMenuState {
    fn default() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected_index: 0,
            x: 0.0,
            y: 0.0,
        }
    }
}
