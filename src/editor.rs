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
            EditFormat::Toml => {
                let value = serde_json::to_value(entry)?;
                let mut value = json_to_toml_value(value)?;
                if let toml::Value::Table(table) = &mut value {
                    table
                        .entry("end_time")
                        .or_insert_with(|| toml::Value::String("null".to_string()));
                }
                toml::to_string_pretty(&value).context("serializing entry for TOML editing")
            }
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

fn remove_toml_null_end_time(value: &mut toml::Value) {
    let toml::Value::Table(table) = value else {
        return;
    };
    if table.get("end_time").and_then(toml::Value::as_str) == Some("null") {
        table.remove("end_time");
    }
}

fn json_to_toml_value(value: Value) -> Result<toml::Value> {
    Ok(match value {
        Value::Null => toml::Value::String("null".to_string()),
        Value::Bool(value) => toml::Value::Boolean(value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                toml::Value::Integer(value)
            } else if let Some(value) = value.as_f64() {
                toml::Value::Float(value)
            } else {
                return Err(anyhow!("unsupported JSON number"));
            }
        }
        Value::String(value) => toml::Value::String(value),
        Value::Array(values) => toml::Value::Array(
            values
                .into_iter()
                .map(json_to_toml_value)
                .collect::<Result<Vec<_>>>()?,
        ),
        Value::Object(values) => toml::Value::Table(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, json_to_toml_value(value)?)))
                .collect::<Result<toml::Table>>()?,
        ),
    })
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
}
