use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ThemeValue {
    Name(String),
    Custom(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalConfig {
    #[serde(default)]
    pub theme: Option<ThemeValue>,

    #[serde(default, alias = "font-family", alias = "fontFamily")]
    pub font_family: Option<String>,

    #[serde(
        default,
        alias = "font-size",
        alias = "fontSize",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub font_size: Option<f64>,

    #[serde(
        default,
        alias = "character-height",
        alias = "characterHeight",
        alias = "line_height",
        alias = "line-height",
        alias = "lineHeight",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub character_height: Option<f64>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            theme: Some(ThemeValue::Name("dark".to_string())),
            font_family: Some(
                "JetBrains Mono, Menlo, Monaco, Consolas, \"Courier New\", monospace".to_string(),
            ),
            font_size: Some(13.0),
            character_height: Some(1.0),
        }
    }
}

pub fn deserialize_flexible_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Number(f64),
        String(String),
    }

    match Option::<NumOrStr>::deserialize(deserializer)? {
        Some(NumOrStr::Number(n)) => Ok(Some(n)),
        Some(NumOrStr::String(s)) => {
            let trimmed = s.trim().trim_end_matches("px").trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<f64>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawConfigFile {
    #[serde(default)]
    pub theme: Option<ThemeValue>,

    #[serde(default, alias = "font-family", alias = "fontFamily")]
    pub font_family: Option<String>,

    #[serde(
        default,
        alias = "font-size",
        alias = "fontSize",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub font_size: Option<f64>,

    #[serde(
        default,
        alias = "character-height",
        alias = "characterHeight",
        alias = "line_height",
        alias = "line-height",
        alias = "lineHeight",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub character_height: Option<f64>,

    #[serde(default)]
    pub terminal: Option<RawTerminalSection>,

    #[serde(default)]
    pub font: Option<RawFontSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawTerminalSection {
    #[serde(default)]
    pub theme: Option<ThemeValue>,

    #[serde(default, alias = "font-family", alias = "fontFamily")]
    pub font_family: Option<String>,

    #[serde(
        default,
        alias = "font-size",
        alias = "fontSize",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub font_size: Option<f64>,

    #[serde(
        default,
        alias = "character-height",
        alias = "characterHeight",
        alias = "line_height",
        alias = "line-height",
        alias = "lineHeight",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub character_height: Option<f64>,

    #[serde(default)]
    pub font: Option<RawFontSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawFontSection {
    #[serde(default, alias = "font-family", alias = "fontFamily")]
    pub family: Option<String>,

    #[serde(
        default,
        alias = "font-size",
        alias = "fontSize",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub size: Option<f64>,

    #[serde(
        default,
        alias = "character-height",
        alias = "characterHeight",
        alias = "line_height",
        alias = "line-height",
        alias = "lineHeight",
        deserialize_with = "deserialize_flexible_f64"
    )]
    pub character_height: Option<f64>,
}

impl RawConfigFile {
    pub fn into_terminal_config(self) -> TerminalConfig {
        let defaults = TerminalConfig::default();

        let theme = self
            .terminal
            .as_ref()
            .and_then(|t| t.theme.clone())
            .or(self.theme)
            .or(defaults.theme);

        let font_family = self
            .terminal
            .as_ref()
            .and_then(|t| t.font_family.clone())
            .or_else(|| {
                self.terminal
                    .as_ref()
                    .and_then(|t| t.font.as_ref())
                    .and_then(|f| f.family.clone())
            })
            .or_else(|| self.font.as_ref().and_then(|f| f.family.clone()))
            .or(self.font_family)
            .or(defaults.font_family);

        let font_size = self
            .terminal
            .as_ref()
            .and_then(|t| t.font_size)
            .or_else(|| {
                self.terminal
                    .as_ref()
                    .and_then(|t| t.font.as_ref())
                    .and_then(|f| f.size)
            })
            .or_else(|| self.font.as_ref().and_then(|f| f.size))
            .or(self.font_size)
            .or(defaults.font_size);

        let character_height = self
            .terminal
            .as_ref()
            .and_then(|t| t.character_height)
            .or_else(|| {
                self.terminal
                    .as_ref()
                    .and_then(|t| t.font.as_ref())
                    .and_then(|f| f.character_height)
            })
            .or_else(|| self.font.as_ref().and_then(|f| f.character_height))
            .or(self.character_height)
            .or(defaults.character_height);

        TerminalConfig {
            theme,
            font_family,
            font_size,
            character_height,
        }
    }
}

pub fn parse_config_yaml(content: &str) -> Result<TerminalConfig, String> {
    let raw: RawConfigFile = serde_yaml::from_str(content)
        .map_err(|e| format!("Failed to parse YAML config: {}", e))?;
    Ok(raw.into_terminal_config())
}

pub fn get_config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)?;
    Some(home.join(".config").join("mdterm"))
}

pub fn get_config_path() -> Option<PathBuf> {
    let dir = get_config_dir()?;
    let yml = dir.join("config.yml");
    if yml.exists() {
        return Some(yml);
    }
    let yaml = dir.join("config.yaml");
    if yaml.exists() {
        return Some(yaml);
    }
    Some(yml)
}

pub const DEFAULT_CONFIG_TEMPLATE: &str = r##"# mdterm terminal configuration
# Location: ~/.config/mdterm/config.yml

# Terminal theme: "dark", "light", "nord", "monokai"
# You can also provide custom colors:
# theme:
#   background: "#0f141c"
#   foreground: "#e2e8f0"
#   cursor: "#38bdf8"
theme: "dark"

# Terminal font family
font_family: 'JetBrains Mono, Menlo, Monaco, Consolas, "Courier New", monospace'

# Terminal font size in points/pixels
font_size: 13

# Terminal character height (line height multiplier, e.g. 1.0, 1.25, 1.5)
character_height: 1.0
"##;

pub fn ensure_default_config_exists() -> Result<PathBuf, String> {
    let dir = get_config_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let yml_path = dir.join("config.yml");
    let yaml_path = dir.join("config.yaml");

    if yml_path.exists() {
        return Ok(yml_path);
    }
    if yaml_path.exists() {
        return Ok(yaml_path);
    }

    if let Err(e) = fs::create_dir_all(&dir) {
        log::warn!("Could not create config directory {:?}: {}", dir, e);
        return Ok(yml_path);
    }

    if let Err(e) = fs::write(&yml_path, DEFAULT_CONFIG_TEMPLATE) {
        log::warn!("Could not write default config file {:?}: {}", yml_path, e);
    }

    Ok(yml_path)
}

pub fn load_terminal_config() -> TerminalConfig {
    let _ = ensure_default_config_exists();

    let path = match get_config_path() {
        Some(p) => p,
        None => return TerminalConfig::default(),
    };

    if !path.exists() {
        return TerminalConfig::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match parse_config_yaml(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("Error parsing {:?}: {}. Using default config.", path, e);
                TerminalConfig::default()
            }
        },
        Err(e) => {
            log::warn!("Error reading {:?}: {}. Using default config.", path, e);
            TerminalConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flat_config() {
        let yaml = r#"
theme: "nord"
font_family: "Fira Code"
font_size: 14
character_height: 1.3
"#;
        let cfg = parse_config_yaml(yaml).expect("Failed to parse flat YAML");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("nord".to_string())));
        assert_eq!(cfg.font_family, Some("Fira Code".to_string()));
        assert_eq!(cfg.font_size, Some(14.0));
        assert_eq!(cfg.character_height, Some(1.3));
    }

    #[test]
    fn test_parse_with_aliases_and_strings() {
        let yaml = r#"
theme: monokai
font-family: 'Hack'
font-size: '16px'
line-height: '1.5'
"#;
        let cfg = parse_config_yaml(yaml).expect("Failed to parse YAML with aliases");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("monokai".to_string())));
        assert_eq!(cfg.font_family, Some("Hack".to_string()));
        assert_eq!(cfg.font_size, Some(16.0));
        assert_eq!(cfg.character_height, Some(1.5));
    }

    #[test]
    fn test_parse_nested_terminal_section() {
        let yaml = r#"
terminal:
  theme: light
  font_family: "Cascadia Code"
  font_size: 15
  character_height: 1.2
"#;
        let cfg = parse_config_yaml(yaml).expect("Failed to parse nested terminal YAML");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("light".to_string())));
        assert_eq!(cfg.font_family, Some("Cascadia Code".to_string()));
        assert_eq!(cfg.font_size, Some(15.0));
        assert_eq!(cfg.character_height, Some(1.2));
    }

    #[test]
    fn test_parse_nested_font_section() {
        let yaml = r#"
theme: "nord"
font:
  family: "Ubuntu Mono"
  size: 18
character_height: 1.4
"#;
        let cfg = parse_config_yaml(yaml).expect("Failed to parse nested font YAML");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("nord".to_string())));
        assert_eq!(cfg.font_family, Some("Ubuntu Mono".to_string()));
        assert_eq!(cfg.font_size, Some(18.0));
        assert_eq!(cfg.character_height, Some(1.4));
    }

    #[test]
    fn test_parse_defaults_for_missing_fields() {
        let yaml = r#"
font_size: 16
"#;
        let cfg = parse_config_yaml(yaml).expect("Failed to parse partial YAML");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("dark".to_string())));
        assert_eq!(
            cfg.font_family,
            Some("JetBrains Mono, Menlo, Monaco, Consolas, \"Courier New\", monospace".to_string())
        );
        assert_eq!(cfg.font_size, Some(16.0));
        assert_eq!(cfg.character_height, Some(1.0));
    }

    #[test]
    fn test_parse_custom_color_theme() {
        let yaml = r##"
theme:
  background: "#1e1e1e"
  foreground: "#f0f0f0"
  cursor: "#ffffff"
font_size: 14
"##;
        let cfg = parse_config_yaml(yaml).expect("Failed to parse custom color map");
        match cfg.theme {
            Some(ThemeValue::Custom(val)) => {
                assert_eq!(val["background"], "#1e1e1e");
                assert_eq!(val["foreground"], "#f0f0f0");
            }
            _ => panic!("Expected Custom ThemeValue"),
        }
    }

    #[test]
    fn test_default_config_template_parses() {
        let cfg = parse_config_yaml(DEFAULT_CONFIG_TEMPLATE).expect("Template should parse");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("dark".to_string())));
        assert_eq!(
            cfg.font_family,
            Some("JetBrains Mono, Menlo, Monaco, Consolas, \"Courier New\", monospace".to_string())
        );
        assert_eq!(cfg.font_size, Some(13.0));
        assert_eq!(cfg.character_height, Some(1.0));
    }

    #[test]
    fn test_empty_config_uses_defaults() {
        let cfg = parse_config_yaml("").expect("Empty YAML should parse");
        assert_eq!(cfg, TerminalConfig::default());
    }

    #[test]
    fn test_read_and_parse_file() {
        let temp_dir = std::env::temp_dir().join("mdterm_test_config");
        let _ = fs::create_dir_all(&temp_dir);
        let config_file = temp_dir.join("config.yml");
        fs::write(
            &config_file,
            r#"
theme: nord
font_family: "Ubuntu Mono"
font_size: 17
character_height: 1.35
"#,
        )
        .expect("write test config");

        let content = fs::read_to_string(&config_file).expect("read test config");
        let cfg = parse_config_yaml(&content).expect("parse test config");
        assert_eq!(cfg.theme, Some(ThemeValue::Name("nord".to_string())));
        assert_eq!(cfg.font_family, Some("Ubuntu Mono".to_string()));
        assert_eq!(cfg.font_size, Some(17.0));
        assert_eq!(cfg.character_height, Some(1.35));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
