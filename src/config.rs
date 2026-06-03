use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_data_path")]
    pub data_filename: PathBuf,
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default = "default_editor")]
    pub editor: String,
    #[serde(default = "default_editor_format")]
    pub editor_format: String,
    #[serde(default = "default_delta_format")]
    pub delta_format: String,
    #[serde(default = "default_table_style")]
    pub table_style: String,
    #[serde(default = "default_table_inner_borders")]
    pub table_inner_borders: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub auto_tag_rules: Vec<AutoTagRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTagRule {
    pub pattern: String,
    #[serde(default = "default_match_type")]
    pub match_type: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let path = path.unwrap_or_else(resolve_default_config_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn apply_auto_tags(&self, project: &str) -> Result<BTreeSet<String>> {
        let mut tags = BTreeSet::new();
        for rule in &self.auto_tag_rules {
            if rule.match_type != "regex" {
                return Err(anyhow!(
                    "unsupported auto tag match_type: {}",
                    rule.match_type
                ));
            }
            if Regex::new(&rule.pattern)
                .with_context(|| format!("compiling auto tag pattern {}", rule.pattern))?
                .is_match(project)
            {
                tags.extend(rule.tags.iter().cloned());
            }
        }
        Ok(tags)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_filename: default_data_path(),
            output: default_output(),
            editor: default_editor(),
            editor_format: default_editor_format(),
            delta_format: default_delta_format(),
            table_style: default_table_style(),
            table_inner_borders: default_table_inner_borders(),
            locale: default_locale(),
            auto_tag_rules: Vec::new(),
        }
    }
}

fn default_output() -> String {
    "ansi".to_string()
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string())
}

fn default_editor_format() -> String {
    "yaml".to_string()
}

fn default_delta_format() -> String {
    "decimal".to_string()
}

fn default_table_style() -> String {
    "utf8-condensed".to_string()
}

fn default_table_inner_borders() -> String {
    "solid".to_string()
}

fn default_locale() -> String {
    "en_US".to_string()
}

fn default_match_type() -> String {
    "regex".to_string()
}

fn default_data_path() -> PathBuf {
    default_config_dir().join("time_entries.toml")
}

#[cfg(test)]
mod tests {
    use super::{Config, default_data_path};
    use tempfile::tempdir;

    #[test]
    fn parses_toml_auto_tag_rules() {
        let config: Config = toml::from_str(
            r#"
data_filename = "/tmp/time_entries.toml"

[[auto_tag_rules]]
pattern = "^PROJ-\\d+"
match_type = "regex"
tags = ["work", "billable"]
"#,
        )
        .unwrap();

        let tags = config.apply_auto_tags("PROJ-123").unwrap();
        assert!(tags.contains("work"));
        assert!(tags.contains("billable"));
        assert!(config.apply_auto_tags("OTHER").unwrap().is_empty());
    }

    #[test]
    fn loads_explicit_config_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sigan.toml");
        std::fs::write(
            &path,
            r#"
data_filename = "/tmp/custom-time.toml"
output = "json"
delta_format = "human"
table_style = "ascii"
table_inner_borders = "dotted"
locale = "ko_KR"
"#,
        )
        .unwrap();

        let config = Config::load(Some(path)).unwrap();
        assert_eq!(config.output, "json");
        assert_eq!(config.delta_format, "human");
        assert_eq!(config.table_style, "ascii");
        assert_eq!(config.table_inner_borders, "dotted");
        assert_eq!(config.locale, "ko_KR");
        assert_eq!(
            config.data_filename,
            std::path::PathBuf::from("/tmp/custom-time.toml")
        );
    }

    #[test]
    fn config_data_filename_defaults_when_omitted() {
        let config: Config = toml::from_str(
            r#"
output = "json"
delta_format = "human"
"#,
        )
        .unwrap();

        assert_eq!(config.data_filename, default_data_path());
        assert_eq!(config.output, "json");
        assert_eq!(config.delta_format, "human");
        assert_eq!(config.table_style, "utf8-condensed");
        assert_eq!(config.table_inner_borders, "solid");
        assert_eq!(config.locale, "en_US");
    }
}

fn default_config_path() -> PathBuf {
    default_config_dir().join("config.toml")
}

fn resolve_default_config_path() -> PathBuf {
    let preferred = default_config_path();
    if preferred.exists() {
        return preferred;
    }

    let platform = platform_config_path();
    if platform.exists() {
        return platform;
    }

    preferred
}

fn default_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("sigan")
}

fn platform_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(default_config_dir)
        .join("sigan")
        .join("config.toml")
}
