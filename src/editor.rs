use crate::models::TimeEntry;
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::Builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditFormat {
    Yaml,
    Toml,
}

impl EditFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "yaml" | "yml" => Ok(Self::Yaml),
            "toml" => Ok(Self::Toml),
            other => Err(anyhow!("unsupported editor format: {other}")),
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Yaml => ".yaml",
            Self::Toml => ".toml",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShellEditor {
    command: String,
    format: EditFormat,
}

impl ShellEditor {
    pub fn new(command: String, format: EditFormat) -> Self {
        Self { command, format }
    }

    pub fn edit_entry(&self, entry: &TimeEntry) -> Result<TimeEntry> {
        let mut tmp = Builder::new()
            .suffix(self.format.suffix())
            .tempfile()
            .context("creating edit temp file")?;
        tmp.write_all(self.format_entry(entry)?.as_bytes())?;
        tmp.flush()?;
        let tmp_path = tmp.path().to_path_buf();

        let status = Command::new(&self.command)
            .arg(&tmp_path)
            .status()
            .with_context(|| format!("running editor {}", self.command))?;
        if !status.success() {
            return Err(anyhow!("editor exited with status {status}"));
        }

        let content = fs::read_to_string(&tmp_path)
            .with_context(|| format!("reading {}", tmp_path.display()))?;
        self.parse_entry(&content)
    }

    fn format_entry(&self, entry: &TimeEntry) -> Result<String> {
        match self.format {
            EditFormat::Yaml => {
                serde_yaml::to_string(entry).context("serializing entry for YAML editing")
            }
            EditFormat::Toml => format_toml_entry(entry),
        }
    }

    fn parse_entry(&self, content: &str) -> Result<TimeEntry> {
        match self.format {
            EditFormat::Yaml => serde_yaml::from_str(content).context("Invalid entry format"),
            EditFormat::Toml => {
                let mut value: toml::Value =
                    toml::from_str(content).context("Invalid entry format")?;
                remove_toml_null_end_time(&mut value);
                let json = toml_to_json_value(value);
                serde_json::from_value(json).context("Invalid entry format")
            }
        }
    }
}

fn format_toml_entry(entry: &TimeEntry) -> Result<String> {
    let fields = [
        ("id", toml::Value::String(entry.id.clone())),
        (
            "start_time",
            toml::Value::String(entry.start_time.to_rfc3339()),
        ),
        (
            "end_time",
            toml::Value::String(
                entry
                    .end_time
                    .map(|time| time.to_rfc3339())
                    .unwrap_or_else(|| "null".to_string()),
            ),
        ),
        ("project", toml::Value::String(entry.project.clone())),
        ("comment", toml::Value::String(entry.comment.clone())),
        (
            "tags",
            toml::Value::Array(
                entry
                    .tags
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        ),
    ];

    let mut content = String::new();
    for (field, value) in fields {
        content.push_str(field);
        content.push_str(" = ");
        content.push_str(&format_toml_value(value)?);
        content.push('\n');
    }
    Ok(content)
}

fn format_toml_value(value: toml::Value) -> Result<String> {
    let mut table = toml::Table::new();
    table.insert("value".to_string(), value);
    let content = toml::to_string_pretty(&toml::Value::Table(table))
        .context("serializing entry for TOML editing")?;

    content
        .trim_end()
        .split_once(" = ")
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| anyhow!("serialized TOML value did not contain assignment"))
}

fn remove_toml_null_end_time(value: &mut toml::Value) {
    let toml::Value::Table(table) = value else {
        return;
    };
    if table.get("end_time").and_then(toml::Value::as_str) == Some("null") {
        table.remove("end_time");
    }
}

fn toml_to_json_value(value: toml::Value) -> Value {
    match value {
        toml::Value::String(value) => Value::String(value),
        toml::Value::Integer(value) => Value::Number(value.into()),
        toml::Value::Float(value) => {
            serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
        }
        toml::Value::Boolean(value) => Value::Bool(value),
        toml::Value::Datetime(value) => Value::String(value.to_string()),
        toml::Value::Array(values) => {
            Value::Array(values.into_iter().map(toml_to_json_value).collect())
        }
        toml::Value::Table(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, toml_to_json_value(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{EditFormat, ShellEditor};
    use crate::models::TimeEntry;
    use chrono::{Local, TimeZone};
    use std::collections::BTreeSet;

    #[test]
    fn toml_editor_format_accepts_null_end_time_string() {
        let editor = ShellEditor::new("true".to_string(), EditFormat::Toml);
        let content = r#"
id = "abc123"
start_time = "2026-06-01T09:00:00-05:00"
end_time = "null"
project = "demo"
tags = ["work"]
comment = "editing"
"#;

        let entry = editor.parse_entry(content).unwrap();
        assert_eq!(entry.id, "abc123");
        assert!(entry.end_time.is_none());
    }

    #[test]
    fn yaml_editor_round_trips_entry() {
        let editor = ShellEditor::new("true".to_string(), EditFormat::Yaml);
        let entry = TimeEntry {
            id: "abc123".to_string(),
            start_time: Local.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap(),
            end_time: None,
            project: "demo".to_string(),
            tags: BTreeSet::from(["work".to_string()]),
            comment: "editing".to_string(),
        };

        let content = editor.format_entry(&entry).unwrap();
        let parsed = editor.parse_entry(&content).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn toml_editor_format_uses_entry_field_order() {
        let editor = ShellEditor::new("true".to_string(), EditFormat::Toml);
        let entry = TimeEntry {
            id: "abc123".to_string(),
            start_time: Local.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap(),
            end_time: None,
            project: "demo".to_string(),
            tags: BTreeSet::from(["work".to_string()]),
            comment: "editing".to_string(),
        };

        let content = editor.format_entry(&entry).unwrap();
        let fields = content
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(field, _)| field))
            .collect::<Vec<_>>();

        assert_eq!(
            fields,
            ["id", "start_time", "end_time", "project", "comment", "tags"]
        );
        assert!(content.contains("end_time = \"null\""));
        assert_eq!(editor.parse_entry(&content).unwrap(), entry);
    }
}
