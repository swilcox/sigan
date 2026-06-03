pub mod file;

use crate::models::{EntryFilter, TimeEntry};
use anyhow::Result;

pub trait TimeEntryRepository {
    fn get_all(&mut self) -> Result<Vec<TimeEntry>>;
    fn save_all(&mut self, entries: &[TimeEntry]) -> Result<()>;

    fn get_active_entry(&mut self) -> Result<Option<TimeEntry>> {
        Ok(self
            .get_all()?
            .into_iter()
            .find(|entry| entry.end_time.is_none()))
    }

    fn save(&mut self, entry: TimeEntry) -> Result<()> {
        let mut entries = self.get_all()?;
        let mut replaced = false;
        for existing in &mut entries {
            if existing.id == entry.id {
                *existing = entry.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            entries.push(entry);
        }
        entries.sort_by_key(|entry| entry.start_time);
        self.save_all(&entries)
    }

    fn delete_entry(&mut self, id: &str) -> Result<Option<TimeEntry>> {
        let mut entries = self.get_all()?;
        let Some(index) = entries.iter().position(|entry| entry.id == id) else {
            return Ok(None);
        };
        let removed = entries.remove(index);
        self.save_all(&entries)?;
        Ok(Some(removed))
    }

    fn filter(&mut self, filter: &EntryFilter) -> Result<Vec<TimeEntry>> {
        Ok(self
            .get_all()?
            .into_iter()
            .filter(|entry| matches_filter(entry, filter))
            .collect())
    }
}

fn matches_filter(entry: &TimeEntry, filter: &EntryFilter) -> bool {
    if let Some(prefix) = &filter.id_prefix {
        if !entry.id.starts_with(prefix) {
            return false;
        }
    }
    if !filter.projects.is_empty()
        && !filter
            .projects
            .iter()
            .any(|project| project_matches(project, &entry.project))
    {
        return false;
    }
    if let Some(start_date) = filter.start_date {
        if entry.start_time.date_naive() < start_date {
            return false;
        }
    }
    if let Some(end_date) = filter.end_date {
        if entry.start_time.date_naive() > end_date {
            return false;
        }
    }
    if !filter.tags.is_empty() && !filter.tags.iter().any(|tag| entry.tags.contains(tag)) {
        return false;
    }
    true
}

fn project_matches(filter_project: &str, project: &str) -> bool {
    filter_project
        .strip_suffix(['*', '+', '.'])
        .is_some_and(|prefix| project.starts_with(prefix))
        || filter_project == project
}

#[cfg(test)]
mod tests {
    use super::project_matches;

    #[test]
    fn project_prefix_markers_match_sigye_behavior() {
        assert!(project_matches("abc+", "abc-123"));
        assert!(project_matches("abc.", "abc-123"));
        assert!(project_matches("abc*", "abc-123"));
        assert!(project_matches("abc-123", "abc-123"));
        assert!(!project_matches("abc", "abc-123"));
    }
}
