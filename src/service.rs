use crate::datetime::adjust_stop_time;
use crate::editor::ShellEditor;
use crate::models::{EntryFilter, TimeEntry};
use crate::storage::TimeEntryRepository;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};
use std::collections::BTreeSet;

pub struct TimeTrackingService<R> {
    repository: R,
}

impl<R: TimeEntryRepository> TimeTrackingService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn start_tracking(
        &mut self,
        project: String,
        start_time: Option<DateTime<Local>>,
        comment: String,
        tags: BTreeSet<String>,
    ) -> Result<TimeEntry> {
        let start_time = start_time.unwrap_or_else(Local::now);
        if let Some(mut active_entry) = self.repository.get_active_entry()? {
            active_entry.stop(start_time)?;
            self.repository.save(active_entry)?;
        }

        let entry = TimeEntry::new(project, start_time, comment, tags);
        self.repository.save(entry.clone())?;
        Ok(entry)
    }

    pub fn stop_tracking(
        &mut self,
        stop_time: Option<DateTime<Local>>,
        comment: Option<String>,
    ) -> Result<Option<TimeEntry>> {
        let Some(mut active_entry) = self.repository.get_active_entry()? else {
            return Ok(None);
        };
        if let Some(comment) = comment {
            if !comment.is_empty() {
                active_entry.comment = comment;
            }
        }
        let stop_time = stop_time
            .map(|time| adjust_stop_time(active_entry.start_time, time))
            .unwrap_or_else(Local::now);
        active_entry.stop(stop_time)?;
        self.repository.save(active_entry.clone())?;
        Ok(Some(active_entry))
    }

    pub fn status(&mut self) -> Result<Option<TimeEntry>> {
        self.repository.get_active_entry()
    }

    pub fn list(&mut self, mut filter: EntryFilter) -> Result<Vec<TimeEntry>> {
        filter.apply_time_period(Local::now().date_naive());
        if filter.time_period.is_none() && filter.start_date.is_none() && filter.end_date.is_none()
        {
            let today = Local::now().date_naive();
            let active_start_date = self
                .repository
                .get_active_entry()?
                .map(|entry| entry.start_time.date_naive());
            filter.start_date = active_start_date
                .filter(|date| *date < today)
                .or(Some(today));
            filter.end_date = Some(today);
        }
        self.repository.filter(&filter)
    }

    pub fn delete(&mut self, id_prefix: &str) -> Result<TimeEntry> {
        let entry = self.get_entry_by_partial_id(id_prefix)?;
        self.repository
            .delete_entry(&entry.id)?
            .ok_or_else(|| anyhow!("No entry found with id {id_prefix}"))
    }

    pub fn edit(&mut self, id_prefix: &str, editor: &ShellEditor) -> Result<TimeEntry> {
        let entry = self.get_entry_by_partial_id(id_prefix)?;
        let updated = editor.edit_entry(&entry)?;
        self.repository.save(updated.clone())?;
        Ok(updated)
    }

    fn get_entry_by_partial_id(&mut self, id_prefix: &str) -> Result<TimeEntry> {
        let matches = self.repository.filter(&EntryFilter {
            id_prefix: Some(id_prefix.to_string()),
            ..EntryFilter::default()
        })?;
        match matches.as_slice() {
            [entry] => Ok(entry.clone()),
            [] => Err(anyhow!("No entry found with id {id_prefix}")),
            _ => Err(anyhow!(
                "Multiple records found starting with id {id_prefix}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimeTrackingService;
    use crate::editor::{EditFormat, ShellEditor};
    use crate::models::{EntryFilter, TimePeriod};
    use crate::storage::TimeEntryRepository;
    use crate::storage::file::FileRepository;
    use chrono::{Local, TimeZone};
    use std::collections::BTreeSet;
    use tempfile::tempdir;

    #[test]
    fn starting_new_entry_stops_existing_active_entry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.toml");
        let repo = FileRepository::new(&path).unwrap();
        let mut service = TimeTrackingService::new(repo);

        let first_start = Local.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let second_start = Local.with_ymd_and_hms(2026, 6, 1, 10, 0, 0).unwrap();

        let first = service
            .start_tracking(
                "first".to_string(),
                Some(first_start),
                String::new(),
                BTreeSet::new(),
            )
            .unwrap();
        let second = service
            .start_tracking(
                "second".to_string(),
                Some(second_start),
                String::new(),
                BTreeSet::new(),
            )
            .unwrap();

        let mut repo = FileRepository::new(&path).unwrap();
        let entries = repo.get_all().unwrap();
        assert_eq!(entries.len(), 2);
        let first = entries.iter().find(|entry| entry.id == first.id).unwrap();
        let second = entries.iter().find(|entry| entry.id == second.id).unwrap();
        assert_eq!(first.end_time, Some(second_start));
        assert!(second.end_time.is_none());
    }

    #[test]
    fn list_all_does_not_apply_default_today_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.toml");
        let repo = FileRepository::new(&path).unwrap();
        let mut service = TimeTrackingService::new(repo);
        let old_start = Local.with_ymd_and_hms(2020, 1, 1, 9, 0, 0).unwrap();

        service
            .start_tracking(
                "old".to_string(),
                Some(old_start),
                String::new(),
                BTreeSet::new(),
            )
            .unwrap();

        let entries = service
            .list(EntryFilter {
                time_period: Some(TimePeriod::All),
                ..EntryFilter::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn default_list_includes_older_active_entry_date_range() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.toml");
        let repo = FileRepository::new(&path).unwrap();
        let mut service = TimeTrackingService::new(repo);
        let active_start = Local.with_ymd_and_hms(2020, 1, 1, 9, 0, 0).unwrap();

        service
            .start_tracking(
                "older-active".to_string(),
                Some(active_start),
                String::new(),
                BTreeSet::new(),
            )
            .unwrap();

        let entries = service.list(EntryFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].project, "older-active");
    }

    #[test]
    fn edit_updates_entry_from_shell_editor() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.toml");
        let script_path = dir.path().join("editor.sh");
        let repo = FileRepository::new(&path).unwrap();
        let mut service = TimeTrackingService::new(repo);
        let start = Local.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let entry = service
            .start_tracking(
                "old-project".to_string(),
                Some(start),
                "old comment".to_string(),
                BTreeSet::new(),
            )
            .unwrap();

        std::fs::write(
            &script_path,
            r#"#!/bin/sh
python3 - "$1" <<'PY'
import sys
from pathlib import Path
path = Path(sys.argv[1])
content = path.read_text()
content = content.replace("old-project", "new-project")
content = content.replace("old comment", "new comment")
path.write_text(content)
PY
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&script_path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&script_path, permissions).unwrap();
        }

        let editor = ShellEditor::new(script_path.display().to_string(), EditFormat::Yaml);
        let updated = service.edit(&entry.id[..4], &editor).unwrap();

        assert_eq!(updated.project, "new-project");
        assert_eq!(updated.comment, "new comment");
    }

    #[test]
    fn start_tracking_preserves_merged_manual_and_auto_tags() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entries.toml");
        let repo = FileRepository::new(&path).unwrap();
        let mut service = TimeTrackingService::new(repo);
        let start = Local.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();

        let entry = service
            .start_tracking(
                "PROJ-123".to_string(),
                Some(start),
                String::new(),
                BTreeSet::from(["manual".to_string(), "billable".to_string()]),
            )
            .unwrap();

        assert!(entry.tags.contains("manual"));
        assert!(entry.tags.contains("billable"));
    }
}
