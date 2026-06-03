use crate::models::TimeEntry;
use crate::storage::TimeEntryRepository;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileRepository {
    path: PathBuf,
    format: StorageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageFormat {
    Toml,
    Yaml,
    Json,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct EntryStore {
    #[serde(default)]
    entries: Vec<TimeEntry>,
}

impl FileRepository {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let format = StorageFormat::from_path(&path)?;
        let repo = Self { path, format };
        repo.ensure_exists()?;
        Ok(repo)
    }

    fn ensure_exists(&self) -> Result<()> {
        if self.path.exists() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        self.write_store(&EntryStore::default())
    }

    fn read_store(&self) -> Result<EntryStore> {
        let content = fs::read_to_string(&self.path)
            .with_context(|| format!("reading {}", self.path.display()))?;
        if content.trim().is_empty() {
            return Ok(EntryStore::default());
        }
        match self.format {
            StorageFormat::Toml => toml::from_str(&content).context("parsing TOML time entry file"),
            StorageFormat::Yaml => {
                serde_yaml::from_str(&content).context("parsing YAML time entry file")
            }
            StorageFormat::Json => {
                serde_json::from_str(&content).context("parsing JSON time entry file")
            }
        }
    }

    fn write_store(&self, store: &EntryStore) -> Result<()> {
        let content = match self.format {
            StorageFormat::Toml => {
                toml::to_string_pretty(store).context("serializing TOML time entry file")?
            }
            StorageFormat::Yaml => {
                serde_yaml::to_string(store).context("serializing YAML time entry file")?
            }
            StorageFormat::Json => {
                serde_json::to_string_pretty(store).context("serializing JSON time entry file")?
            }
        };
        fs::write(&self.path, content).with_context(|| format!("writing {}", self.path.display()))
    }
}

impl StorageFormat {
    fn from_path(path: &Path) -> Result<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml") | Some("yml") => Ok(Self::Yaml),
            Some("json") => Ok(Self::Json),
            Some(ext) => Err(anyhow!("unsupported storage format: {ext}")),
            None => Err(anyhow!("storage filename must have an extension")),
        }
    }
}

impl TimeEntryRepository for FileRepository {
    fn get_all(&mut self) -> Result<Vec<TimeEntry>> {
        Ok(self.read_store()?.entries)
    }

    fn save_all(&mut self, entries: &[TimeEntry]) -> Result<()> {
        let mut entries = entries.to_vec();
        entries.sort_by_key(|entry| entry.start_time);
        self.write_store(&EntryStore { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::FileRepository;
    use crate::storage::TimeEntryRepository;
    use tempfile::tempdir;

    #[test]
    fn reads_legacy_toml_shape() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.toml");
        std::fs::write(
            &path,
            r#"
[[entries]]
id = "abc123"
start_time = "2026-06-01T09:00:00-05:00"
end_time = "2026-06-01T10:15:00-05:00"
project = "demo"
tags = ["work", "deep"]
comment = "legacy file"
"#,
        )
        .unwrap();

        let mut repo = FileRepository::new(&path).unwrap();
        let entries = repo.get_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "abc123");
        assert_eq!(entries[0].project, "demo");
        assert!(entries[0].tags.contains("work"));
    }

    #[test]
    fn round_trips_json_shape() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.json");
        let mut repo = FileRepository::new(&path).unwrap();
        assert!(repo.get_all().unwrap().is_empty());
    }

    #[test]
    fn reads_legacy_yaml_shape() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.yaml");
        std::fs::write(
            &path,
            r#"
entries:
  - id: def456
    start_time: "2026-06-01T11:00:00-05:00"
    end_time: null
    project: yaml-demo
    tags:
      - writing
    comment: active yaml file
"#,
        )
        .unwrap();

        let mut repo = FileRepository::new(&path).unwrap();
        let entries = repo.get_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "def456");
        assert!(entries[0].end_time.is_none());
        assert!(entries[0].tags.contains("writing"));
    }

    #[test]
    fn reads_legacy_json_shape() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.json");
        std::fs::write(
            &path,
            r#"
{
  "entries": [
    {
      "id": "ghi789",
      "start_time": "2026-06-01T12:00:00-05:00",
      "end_time": "2026-06-01T12:30:00-05:00",
      "project": "json-demo",
      "tags": ["review"],
      "comment": "json file"
    }
  ]
}
"#,
        )
        .unwrap();

        let mut repo = FileRepository::new(&path).unwrap();
        let entries = repo.get_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "ghi789");
        assert_eq!(entries[0].project, "json-demo");
        assert!(entries[0].end_time.is_some());
    }
}
