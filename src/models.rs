use anyhow::{Result, anyhow};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntry {
    #[serde(default = "new_id")]
    pub id: String,
    pub start_time: DateTime<Local>,
    #[serde(default)]
    pub end_time: Option<DateTime<Local>>,
    pub project: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    #[serde(default)]
    pub comment: String,
}

impl TimeEntry {
    pub fn new(
        project: String,
        start_time: DateTime<Local>,
        comment: String,
        tags: BTreeSet<String>,
    ) -> Self {
        Self {
            id: new_id(),
            start_time,
            end_time: None,
            project,
            tags,
            comment,
        }
    }

    pub fn stop(&mut self, end_time: DateTime<Local>) -> Result<()> {
        if self.end_time.is_some() {
            return Err(anyhow!("already stopped"));
        }
        if end_time < self.start_time {
            return Err(anyhow!("end time is before start time"));
        }
        self.end_time = Some(end_time);
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub id_prefix: Option<String>,
    pub projects: BTreeSet<String>,
    pub tags: BTreeSet<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub time_period: Option<TimePeriod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePeriod {
    Today,
    Yesterday,
    Week,
    Month,
    All,
}

impl EntryFilter {
    pub fn apply_time_period(&mut self, today: NaiveDate) {
        match self.time_period {
            Some(TimePeriod::Today) => self.start_date = Some(today),
            Some(TimePeriod::Yesterday) => {
                let yesterday = today - Duration::days(1);
                self.start_date = Some(yesterday);
                self.end_date = Some(yesterday);
            }
            Some(TimePeriod::Week) => {
                self.start_date =
                    Some(today - Duration::days(today.weekday().num_days_from_monday() as i64));
            }
            Some(TimePeriod::Month) => {
                self.start_date = today.with_day(1);
            }
            Some(TimePeriod::All) | None => {}
        }
    }
}

fn new_id() -> String {
    Uuid::new_v4().simple().to_string()
}
