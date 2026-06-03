use anyhow::{Result, anyhow};
use chrono::{DateTime, Local, NaiveTime, TimeZone};
use regex::Regex;

pub fn parse_user_time(value: &str) -> Result<DateTime<Local>> {
    let pattern = Regex::new(r"(?i)^(\d{1,2}):(\d{2})(?::(\d{2}))?\s*((?:am|pm)?)$").unwrap();
    let Some(captures) = pattern.captures(value.trim()) else {
        return Err(anyhow!(
            "Invalid time format. Use HH:MM or HH:MM:SS with optional AM/PM"
        ));
    };

    let mut hours: u32 = captures[1].parse()?;
    let minutes: u32 = captures[2].parse()?;
    let seconds: u32 = captures.get(3).map_or(Ok(0), |m| m.as_str().parse())?;
    let meridiem = captures
        .get(4)
        .map_or("", |m| m.as_str())
        .to_ascii_uppercase();

    if !meridiem.is_empty() && hours > 12 {
        return Err(anyhow!("Hours cannot exceed 12 in 12-hour format"));
    }
    if meridiem == "PM" && hours < 12 {
        hours += 12;
    } else if meridiem == "AM" && hours == 12 {
        hours = 0;
    }
    if hours > 23 {
        return Err(anyhow!("Hours cannot exceed 23"));
    }
    if minutes > 59 {
        return Err(anyhow!("Minutes cannot exceed 59"));
    }
    if seconds > 59 {
        return Err(anyhow!("Seconds cannot exceed 59"));
    }

    let today = Local::now().date_naive();
    let time = NaiveTime::from_hms_opt(hours, minutes, seconds)
        .ok_or_else(|| anyhow!("Invalid time format"))?;
    Local
        .from_local_datetime(&today.and_time(time))
        .single()
        .ok_or_else(|| anyhow!("Invalid local time"))
}

pub fn adjust_stop_time(
    start_time: DateTime<Local>,
    stop_time: DateTime<Local>,
) -> DateTime<Local> {
    if start_time.date_naive() < stop_time.date_naive() {
        let candidate =
            Local.from_local_datetime(&start_time.date_naive().and_time(stop_time.time()));
        if let Some(candidate) = candidate.single() {
            if candidate > start_time {
                return candidate;
            }
        }
    }
    stop_time
}
