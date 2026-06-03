use crate::models::TimeEntry;
use anyhow::{Result, anyhow};
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use tabled::{
    builder::Builder as TabledBuilder,
    settings::{
        Modify, Panel, Style as TabledStyle, Theme, Width as TabledWidth, object::Columns,
        style::HorizontalLine,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Ansi,
    Text,
    Json,
    Yaml,
    Csv,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaFormat {
    DecimalHours,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStyle {
    Utf8,
    Utf8Condensed,
    Utf8BordersOnly,
    Utf8HorizontalOnly,
    Utf8NoBorders,
    Ascii,
    AsciiCondensed,
    AsciiBordersOnly,
    AsciiBordersOnlyCondensed,
    AsciiHorizontalOnly,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableInnerBorders {
    Dotted,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    English,
    Korean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputOptions {
    pub delta_format: DeltaFormat,
    pub table_style: TableStyle,
    pub table_inner_borders: TableInnerBorders,
    pub locale: Locale,
}

impl DeltaFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "decimal" | "decimal-hours" | "tenths" | "tenths-of-hour" | "hours" => {
                Ok(Self::DecimalHours)
            }
            "human" | "clock" | "hours-minutes" => Ok(Self::Human),
            other => Err(anyhow!("unsupported delta format: {other}")),
        }
    }
}

impl TableStyle {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "utf8" | "unicode" | "full" => Ok(Self::Utf8),
            "utf8-condensed" | "unicode-condensed" | "condensed" => Ok(Self::Utf8Condensed),
            "utf8-borders-only" | "unicode-borders-only" | "borders-only" => {
                Ok(Self::Utf8BordersOnly)
            }
            "utf8-horizontal-only" | "unicode-horizontal-only" | "horizontal-only" => {
                Ok(Self::Utf8HorizontalOnly)
            }
            "utf8-no-borders" | "unicode-no-borders" | "no-borders" => Ok(Self::Utf8NoBorders),
            "ascii" | "ascii-full" => Ok(Self::Ascii),
            "ascii-condensed" => Ok(Self::AsciiCondensed),
            "ascii-borders-only" => Ok(Self::AsciiBordersOnly),
            "ascii-borders-only-condensed" => Ok(Self::AsciiBordersOnlyCondensed),
            "ascii-horizontal-only" => Ok(Self::AsciiHorizontalOnly),
            "none" | "plain" => Ok(Self::None),
            other => Err(anyhow!("unsupported table style: {other}")),
        }
    }

    fn is_utf8(self) -> bool {
        matches!(
            self,
            Self::Utf8
                | Self::Utf8Condensed
                | Self::Utf8BordersOnly
                | Self::Utf8HorizontalOnly
                | Self::Utf8NoBorders
        )
    }

    fn is_condensed(self) -> bool {
        matches!(
            self,
            Self::Utf8Condensed | Self::AsciiCondensed | Self::AsciiBordersOnlyCondensed
        )
    }
}

impl TableInnerBorders {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "dotted" | "dashed" | "default" => Ok(Self::Dotted),
            "solid" => Ok(Self::Solid),
            other => Err(anyhow!("unsupported table inner border style: {other}")),
        }
    }
}

impl Locale {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "en" | "en_US" | "en_GB" => Ok(Self::English),
            "ko" | "ko_KR" => Ok(Self::Korean),
            other => Err(anyhow!("unsupported locale: {other}")),
        }
    }
}

impl OutputFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "ansi" | "rich" => Ok(Self::Ansi),
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "yaml" => Ok(Self::Yaml),
            "csv" => Ok(Self::Csv),
            "markdown" | "md" => Ok(Self::Markdown),
            other => Err(anyhow!("unsupported output format: {other}")),
        }
    }
}

pub fn print_single(
    format: OutputFormat,
    options: OutputOptions,
    entry: Option<&TimeEntry>,
) -> Result<()> {
    match format {
        OutputFormat::Ansi => {
            print_ansi_single(options, entry);
            Ok(())
        }
        OutputFormat::Text => {
            if let Some(entry) = entry {
                println!("{}", entry_to_text(entry));
            } else {
                println!("{}", translate(options.locale, "No active time record."));
            }
            Ok(())
        }
        OutputFormat::Json => {
            if let Some(entry) = entry {
                println!("{}", serde_json::to_string(entry)?);
            }
            Ok(())
        }
        OutputFormat::Yaml => {
            if let Some(entry) = entry {
                print!("{}", serde_yaml::to_string(entry)?);
            }
            Ok(())
        }
        OutputFormat::Csv => print_csv(entry.into_iter()),
        OutputFormat::Markdown => {
            if let Some(entry) = entry {
                println!("| id | start_time | end_time | project | comment | tags |");
                println!("| --- | --- | --- | --- | --- | --- |");
                println!("{}", entry_to_markdown_row(entry));
            }
            Ok(())
        }
    }
}

pub fn print_many(
    format: OutputFormat,
    options: OutputOptions,
    entries: &[TimeEntry],
) -> Result<()> {
    match format {
        OutputFormat::Ansi => {
            print_ansi_many(options, entries);
            Ok(())
        }
        OutputFormat::Text => {
            for entry in entries {
                println!("{}", entry_to_text(entry));
            }
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(entries)?);
            Ok(())
        }
        OutputFormat::Yaml => {
            println!(
                "{}",
                serde_yaml::to_string(&serde_json::json!({ "entries": entries }))?
            );
            Ok(())
        }
        OutputFormat::Csv => print_csv(entries.iter()),
        OutputFormat::Markdown => {
            println!("| id | start_time | end_time | project | comment | tags |");
            println!("| --- | --- | --- | --- | --- | --- |");
            for entry in entries {
                println!("{}", entry_to_markdown_row(entry));
            }
            Ok(())
        }
    }
}

pub fn print_export(
    format: OutputFormat,
    options: OutputOptions,
    count: usize,
    path: &Path,
) -> Result<()> {
    match format {
        OutputFormat::Ansi => {
            println!(
                "{}",
                render_tabled_two_column(
                    options,
                    "Export",
                    [
                        colorize(
                            translate(options.locale, "records exported"),
                            AnsiColor::Cyan
                        ),
                        colorize(translate(options.locale, "filename"), AnsiColor::Cyan),
                    ],
                    vec![[
                        colorize(&count.to_string(), AnsiColor::Green),
                        colorize(&path.display().to_string(), AnsiColor::Blue),
                    ]],
                    18,
                )
            );
        }
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({ "count": count, "filename": path }).to_string()
        ),
        _ => println!("Exported {count} entries to {}", path.display()),
    }
    Ok(())
}

fn print_ansi_single(options: OutputOptions, entry: Option<&TimeEntry>) {
    let Some(entry) = entry else {
        println!(
            "{}",
            render_tabled_message(
                options,
                "Status",
                translate(options.locale, "No active time record."),
                AnsiColor::Yellow,
            )
        );
        return;
    };

    let split = entry.id.len().min(4);
    println!(
        "{}",
        render_tabled_two_column(
            options,
            "Time Record",
            [
                colorize(translate(options.locale, "field"), AnsiColor::DarkGrey),
                colorize(translate(options.locale, "value"), AnsiColor::DarkGrey),
            ],
            vec![
                [
                    translate(options.locale, "ID").to_string(),
                    colorize(
                        &format!("{}{}", &entry.id[..split], &entry.id[split..]),
                        AnsiColor::Magenta,
                    ),
                ],
                [
                    translate(options.locale, "start time").to_string(),
                    colorize(
                        &entry.start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                        AnsiColor::Cyan,
                    ),
                ],
                [
                    translate(options.locale, "end time").to_string(),
                    entry
                        .end_time
                        .map(|time| colorize(
                            &time.format("%H:%M:%S").to_string(),
                            AnsiColor::Magenta
                        ))
                        .unwrap_or_else(|| "-".to_string()),
                ],
                [
                    translate(options.locale, "delta").to_string(),
                    colorize(
                        &format_entry_duration(entry, options.delta_format),
                        AnsiColor::Cyan,
                    ),
                ],
                [
                    translate(options.locale, "project").to_string(),
                    colorize(&entry.project, AnsiColor::Green),
                ],
                [
                    translate(options.locale, "comments").to_string(),
                    colorize(&entry.comment, AnsiColor::Blue),
                ],
                [
                    translate(options.locale, "tags").to_string(),
                    colorize(
                        &entry.tags.iter().cloned().collect::<Vec<_>>().join(", "),
                        AnsiColor::Red,
                    ),
                ],
            ],
            13,
        )
    );
}

fn print_ansi_many(options: OutputOptions, entries: &[TimeEntry]) {
    println!("{}", render_tabled_ansi_many(options, entries));
}

fn render_tabled_ansi_many(options: OutputOptions, entries: &[TimeEntry]) -> String {
    render_tabled_ansi_many_with_width(options, entries, ansi_table_width() as usize)
}

fn render_tabled_ansi_many_with_width(
    options: OutputOptions,
    entries: &[TimeEntry],
    table_width: usize,
) -> String {
    let mut builder = TabledBuilder::default();
    builder.push_record([
        colorize(translate(options.locale, "id"), AnsiColor::DarkGrey),
        colorize(translate(options.locale, "start"), AnsiColor::Cyan),
        colorize(translate(options.locale, "end"), AnsiColor::Magenta),
        colorize(translate(options.locale, "delta"), AnsiColor::Cyan),
        colorize(translate(options.locale, "project"), AnsiColor::Green),
        colorize(translate(options.locale, "comments"), AnsiColor::Blue),
        colorize(translate(options.locale, "tags"), AnsiColor::Red),
    ]);

    let mut horizontal_lines = Vec::new();
    let mut row_index = 1;
    let mut current_date = None;
    let mut subtotal = chrono::Duration::zero();
    let mut total = chrono::Duration::zero();

    for entry in entries {
        let entry_date = entry.start_time.date_naive();
        if current_date != Some(entry_date) {
            if subtotal != chrono::Duration::zero() {
                builder.push_record(summary_record(
                    translate(options.locale, "subtotal"),
                    subtotal,
                    options.delta_format,
                    AnsiColor::DarkGrey,
                ));
                row_index += 1;
                if options.table_style.is_condensed() {
                    horizontal_lines.push(row_index);
                }
            }
            subtotal = chrono::Duration::zero();
            current_date = Some(entry_date);
            builder.push_record([
                String::new(),
                colorize(
                    &entry_date.format("%Y-%m-%d").to_string(),
                    AnsiColor::Yellow,
                ),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]);
            row_index += 1;
        }

        let duration = entry_duration(entry);
        subtotal += duration;
        total += duration;
        let split = entry.id.len().min(4);
        builder.push_record([
            colorize(&entry.id[..split], AnsiColor::DarkGrey),
            colorize(
                &entry.start_time.format("%H:%M:%S").to_string(),
                AnsiColor::Cyan,
            ),
            entry
                .end_time
                .map(|time| colorize(&time.format("%H:%M:%S").to_string(), AnsiColor::Magenta))
                .unwrap_or_else(|| "-".to_string()),
            colorize(
                &format_duration(duration, options.delta_format),
                AnsiColor::Cyan,
            ),
            colorize(&entry.project, AnsiColor::Green),
            colorize(&entry.comment, AnsiColor::Blue),
            colorize(
                &entry.tags.iter().cloned().collect::<Vec<_>>().join(", "),
                AnsiColor::Red,
            ),
        ]);
        row_index += 1;
    }

    if subtotal != chrono::Duration::zero() {
        builder.push_record(summary_record(
            translate(options.locale, "subtotal"),
            subtotal,
            options.delta_format,
            AnsiColor::DarkGrey,
        ));
        row_index += 1;
        if options.table_style.is_condensed() {
            horizontal_lines.push(row_index);
        }
    }
    builder.push_record(summary_record(
        translate(options.locale, "total"),
        total,
        options.delta_format,
        AnsiColor::Yellow,
    ));
    let mut table = builder.build();
    table.with(tabled_theme(options, &horizontal_lines));
    apply_tabled_list_width(&mut table, table_width);
    with_centered_title(table.to_string(), translate(options.locale, "Time Records"))
}

fn apply_tabled_list_width(table: &mut tabled::Table, table_width: usize) {
    let table_width = table_width.max(1);
    let comments_width = comment_column_width(table_width);

    table
        .with(Modify::new(Columns::one(4)).with(TabledWidth::wrap(12).keep_words(true)))
        .with(Modify::new(Columns::one(5)).with(TabledWidth::wrap(comments_width).keep_words(true)))
        .with(Modify::new(Columns::one(6)).with(TabledWidth::wrap(10).keep_words(true)))
        .with(TabledWidth::wrap(table_width));
}

fn comment_column_width(table_width: usize) -> usize {
    const LIST_COLUMNS: usize = 7;
    const CELL_PADDING: usize = LIST_COLUMNS * 2;
    const VERTICAL_BORDERS: usize = LIST_COLUMNS + 1;
    const FIXED_CONTENT_WIDTH: usize = 4 + 10 + 8 + 9 + 12 + 10;

    let minimum = if table_width < 80 { 4 } else { 12 };
    table_width
        .saturating_sub(CELL_PADDING + VERTICAL_BORDERS + FIXED_CONTENT_WIDTH)
        .clamp(minimum, 72)
}

fn render_tabled_two_column(
    options: OutputOptions,
    title: &str,
    header: [String; 2],
    rows: Vec<[String; 2]>,
    label_width: usize,
) -> String {
    let mut builder = TabledBuilder::default();
    builder.push_record(header);
    for row in rows {
        builder.push_record(row);
    }

    let width = ansi_table_width() as usize;
    let value_width = two_column_value_width(width, label_width);
    let label_content_width = label_width.saturating_sub(2).max(1);
    let mut table = builder.build();
    table
        .with(tabled_theme(options, &[1]))
        .with(
            Modify::new(Columns::one(0))
                .with(TabledWidth::wrap(label_content_width).keep_words(true))
                .with(TabledWidth::increase(label_content_width)),
        )
        .with(Modify::new(Columns::one(1)).with(TabledWidth::wrap(value_width).keep_words(true)))
        .with(TabledWidth::wrap(width));
    with_centered_title(table.to_string(), translate(options.locale, title))
}

fn with_centered_title(table: String, title: &str) -> String {
    let table_width = table
        .lines()
        .next()
        .map(visible_width)
        .unwrap_or_else(|| title.chars().count());
    let title_width = title.chars().count();
    let left_padding = table_width.saturating_sub(title_width) / 2;

    format!(
        "{}{}\n{table}",
        " ".repeat(left_padding),
        italicize(title, AnsiColor::White)
    )
}

fn render_tabled_message(
    options: OutputOptions,
    title: &str,
    message: &str,
    color: AnsiColor,
) -> String {
    let mut builder = TabledBuilder::default();
    builder.push_record([colorize(message, color)]);

    let width = ansi_table_width() as usize;
    let mut table = builder.build();
    table
        .with(tabled_theme(options, &[]))
        .with(Panel::header(colorize(
            translate(options.locale, title),
            AnsiColor::White,
        )))
        .with(
            Modify::new(Columns::one(0))
                .with(TabledWidth::wrap(width.saturating_sub(4).max(1)).keep_words(true)),
        )
        .with(TabledWidth::wrap(width));
    table.to_string()
}

fn two_column_value_width(table_width: usize, label_width: usize) -> usize {
    const CELL_PADDING: usize = 4;
    const VERTICAL_BORDERS: usize = 3;

    table_width
        .saturating_sub(CELL_PADDING + VERTICAL_BORDERS + label_width)
        .clamp(8, 100)
}

fn tabled_theme(options: OutputOptions, horizontal_lines: &[usize]) -> Theme {
    let mut theme = match options.table_style {
        TableStyle::Ascii | TableStyle::AsciiCondensed => Theme::from_style(TabledStyle::ascii()),
        TableStyle::None => Theme::from_style(TabledStyle::blank()),
        TableStyle::Utf8Condensed
        | TableStyle::Utf8BordersOnly
        | TableStyle::Utf8HorizontalOnly
        | TableStyle::Utf8NoBorders
        | TableStyle::AsciiBordersOnly
        | TableStyle::AsciiBordersOnlyCondensed
        | TableStyle::AsciiHorizontalOnly => Theme::from_style(TabledStyle::rounded()),
        TableStyle::Utf8 => Theme::from_style(TabledStyle::modern_rounded()),
    };

    if options.table_style.is_condensed() {
        let line = if options.table_style.is_utf8() {
            HorizontalLine::inherit(TabledStyle::modern())
        } else {
            HorizontalLine::inherit(TabledStyle::ascii())
        };
        for row in horizontal_lines {
            theme.insert_horizontal_line(*row, line);
        }
    }

    theme
}

#[derive(Clone, Copy)]
enum AnsiColor {
    DarkGrey,
    White,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
}

impl AnsiColor {
    fn code(self) -> &'static str {
        match self {
            Self::DarkGrey => "90",
            Self::White => "37",
            Self::Red => "31",
            Self::Green => "32",
            Self::Yellow => "33",
            Self::Blue => "34",
            Self::Magenta => "35",
            Self::Cyan => "36",
        }
    }
}

fn colorize(value: &str, color: AnsiColor) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m{value}\x1b[0m", color.code())
    }
}

fn italicize(value: &str, color: AnsiColor) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!("\x1b[3;{}m{value}\x1b[0m", color.code())
    }
}

fn visible_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code_char in chars.by_ref() {
                if code_char.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }
    width
}

fn summary_record(
    label: &str,
    duration: chrono::Duration,
    delta_format: DeltaFormat,
    color: AnsiColor,
) -> [String; 7] {
    [
        String::new(),
        colorize(label, color),
        String::new(),
        colorize(&format_duration(duration, delta_format), color),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn ansi_table_width() -> u16 {
    if io::stdout().is_terminal() {
        return crossterm::terminal::size()
            .ok()
            .map(|(width, _)| width)
            .filter(|width| *width > 0)
            .unwrap_or(100);
    }

    columns_env_width()
        .filter(|width| *width >= 80)
        .unwrap_or(100)
}

fn columns_env_width() -> Option<u16> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|width| *width > 0)
}

fn translate(locale: Locale, message: &str) -> &'static str {
    if locale != Locale::Korean {
        return english_message(message);
    }
    match message {
        "Time Record" => "시간 기록",
        "Time Records" => "시간 기록",
        "Export" => "내보내기",
        "Status" => "상태",
        "field" => "필드 이름",
        "value" => "값",
        "id" => "아이디",
        "ID" => "아이디",
        "start" => "시작 시간",
        "start time" => "시작 시간",
        "end" => "종료 시간",
        "end time" => "종료 시간",
        "delta" => "지속 시간",
        "duration" => "지속 시간",
        "tags" => "태그",
        "comments" => "댓글",
        "project" => "프로젝트",
        "subtotal" => "소계",
        "total" => "총",
        "No active time record." => "시간 기록 없음.",
        "records exported" => "내보낸 기록 수",
        "filename" => "파일 이름",
        _ => english_message(message),
    }
}

fn english_message(message: &str) -> &'static str {
    match message {
        "Time Record" => "Time Record",
        "Time Records" => "Time Records",
        "Export" => "Export",
        "Status" => "Status",
        "field" => "field",
        "value" => "value",
        "id" => "id",
        "ID" => "ID",
        "start" => "start",
        "start time" => "start time",
        "end" => "end",
        "end time" => "end time",
        "delta" => "delta",
        "duration" => "duration",
        "tags" => "tags",
        "comments" => "comments",
        "project" => "project",
        "subtotal" => "subtotal",
        "total" => "total",
        "No active time record." => "No active time record.",
        "records exported" => "records exported",
        "filename" => "filename",
        _ => "",
    }
}

fn format_entry_duration(entry: &TimeEntry, delta_format: DeltaFormat) -> String {
    format_duration(entry_duration(entry), delta_format)
}

fn entry_duration(entry: &TimeEntry) -> chrono::Duration {
    entry.end_time.unwrap_or_else(chrono::Local::now) - entry.start_time
}

fn format_duration(duration: chrono::Duration, delta_format: DeltaFormat) -> String {
    let minutes = duration.num_minutes().max(0);
    match delta_format {
        DeltaFormat::DecimalHours => format!("{:.1} hours", minutes as f64 / 60.0),
        DeltaFormat::Human => {
            let hours = minutes / 60;
            let minutes = minutes % 60;
            if minutes == 0 {
                format!("{hours}.0 hours")
            } else {
                format!("{hours}h {minutes}m")
            }
        }
    }
}

fn entry_to_text(entry: &TimeEntry) -> String {
    format!(
        "{} {} {} {} {} {:?}",
        entry.id,
        entry.start_time,
        entry
            .end_time
            .map(|time| time.to_string())
            .unwrap_or_else(|| "None".to_string()),
        entry.project,
        entry.comment,
        entry.tags
    )
}

fn entry_to_markdown_row(entry: &TimeEntry) -> String {
    format!(
        "| {} | {} | {} | {} | {} | {} |",
        entry.id,
        entry.start_time,
        entry
            .end_time
            .map(|time| time.to_string())
            .unwrap_or_default(),
        entry.project,
        entry.comment,
        entry.tags.iter().cloned().collect::<Vec<_>>().join(", ")
    )
}

fn print_csv<'a>(entries: impl Iterator<Item = &'a TimeEntry>) -> Result<()> {
    let stdout = io::stdout();
    let mut writer = csv::Writer::from_writer(stdout.lock());
    writer.write_record(["id", "start_time", "end_time", "project", "comment", "tags"])?;
    for entry in entries {
        writer.write_record([
            entry.id.clone(),
            entry.start_time.to_string(),
            entry
                .end_time
                .map(|time| time.to_string())
                .unwrap_or_default(),
            entry.project.clone(),
            entry.comment.clone(),
            entry.tags.iter().cloned().collect::<Vec<_>>().join(","),
        ])?;
    }
    writer.flush()?;
    io::stdout().flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        DeltaFormat, Locale, OutputOptions, TableInnerBorders, TableStyle, format_duration,
        render_tabled_ansi_many, render_tabled_ansi_many_with_width, render_tabled_two_column,
        translate,
    };
    use crate::models::TimeEntry;
    use chrono::{Duration, Local, TimeZone};
    use std::collections::BTreeSet;

    #[test]
    fn delta_format_parses_aliases() {
        assert_eq!(
            DeltaFormat::parse("decimal").unwrap(),
            DeltaFormat::DecimalHours
        );
        assert_eq!(
            DeltaFormat::parse("tenths").unwrap(),
            DeltaFormat::DecimalHours
        );
        assert_eq!(DeltaFormat::parse("human").unwrap(), DeltaFormat::Human);
    }

    #[test]
    fn decimal_delta_format_uses_tenths_of_an_hour() {
        assert_eq!(
            format_duration(Duration::minutes(12), DeltaFormat::DecimalHours),
            "0.2 hours"
        );
        assert_eq!(
            format_duration(Duration::minutes(54), DeltaFormat::DecimalHours),
            "0.9 hours"
        );
    }

    #[test]
    fn human_delta_format_uses_hours_and_minutes() {
        assert_eq!(
            format_duration(Duration::minutes(70), DeltaFormat::Human),
            "1h 10m"
        );
    }

    #[test]
    fn table_style_parses_condensed_and_solid_border_options() {
        assert_eq!(
            TableStyle::parse("utf8-condensed").unwrap(),
            TableStyle::Utf8Condensed
        );
        assert_eq!(TableStyle::parse("ascii").unwrap(), TableStyle::Ascii);
        assert_eq!(
            TableInnerBorders::parse("solid").unwrap(),
            TableInnerBorders::Solid
        );
    }

    #[test]
    fn korean_locale_translates_python_supported_labels() {
        assert_eq!(Locale::parse("ko_KR").unwrap(), Locale::Korean);
        assert_eq!(translate(Locale::Korean, "Time Records"), "시간 기록");
        assert_eq!(translate(Locale::Korean, "subtotal"), "소계");
        assert_eq!(
            translate(Locale::Korean, "No active time record."),
            "시간 기록 없음."
        );
    }

    #[test]
    fn condensed_list_uses_real_separator_after_subtotal_only() {
        let options = OutputOptions {
            delta_format: DeltaFormat::DecimalHours,
            table_style: TableStyle::Utf8Condensed,
            table_inner_borders: TableInnerBorders::Solid,
            locale: Locale::English,
        };
        let rendered = render_tabled_ansi_many(options, &[finished_entry()]);
        let visible = strip_ansi(&rendered);
        let horizontal_count = rendered
            .lines()
            .filter(|line| line.starts_with('├'))
            .count();

        assert!(rendered.contains("\x1b[3;37mTime Records\x1b[0m"));
        assert_eq!(visible.lines().next().unwrap().trim(), "Time Records");
        assert_eq!(horizontal_count, 2);
        assert!(!rendered.contains('┅'));
        assert!(rendered.find("subtotal").unwrap() < rendered.rfind('├').unwrap());
        assert!(rendered.rfind('├').unwrap() < rendered.rfind("total").unwrap());
    }

    #[test]
    fn full_list_uses_real_solid_row_separators() {
        let options = OutputOptions {
            delta_format: DeltaFormat::DecimalHours,
            table_style: TableStyle::Utf8,
            table_inner_borders: TableInnerBorders::Solid,
            locale: Locale::English,
        };
        let rendered = render_tabled_ansi_many(options, &[finished_entry()]);

        assert!(
            rendered
                .lines()
                .filter(|line| line.starts_with('├'))
                .count()
                > 2
        );
        assert!(rendered.contains('─'));
        assert!(rendered.contains('┼'));
        assert!(!rendered.contains('╌'));
        assert!(!rendered.contains('┆'));
    }

    #[test]
    fn single_record_table_uses_wider_label_column() {
        let options = OutputOptions {
            delta_format: DeltaFormat::DecimalHours,
            table_style: TableStyle::Utf8Condensed,
            table_inner_borders: TableInnerBorders::Solid,
            locale: Locale::English,
        };

        let rendered = render_tabled_two_column(
            options,
            "Time Record",
            ["field".to_string(), "value".to_string()],
            vec![["start time".to_string(), "2026-05-03 10:00:00".to_string()]],
            13,
        );
        let visible = strip_ansi(&rendered);
        let mut lines = visible.lines();

        assert!(rendered.contains("\x1b[3;37mTime Record\x1b[0m"));
        assert_eq!(lines.next().unwrap().trim(), "Time Record");
        assert!(lines.next().unwrap().starts_with('╭'));
        assert!(visible.contains("│ field       │ value"));
        assert!(visible.contains("├─────────────┼"));
        assert!(visible.contains("│ start time  │"));
    }

    #[test]
    fn list_wraps_long_comments_to_configured_width() {
        let options = OutputOptions {
            delta_format: DeltaFormat::DecimalHours,
            table_style: TableStyle::Utf8Condensed,
            table_inner_borders: TableInnerBorders::Solid,
            locale: Locale::English,
        };
        let mut entry = finished_entry();
        entry.comment = "this is a deliberately long comment that should wrap instead of pushing the table past the configured terminal width".to_string();

        let rendered = render_tabled_ansi_many_with_width(options, &[entry], 80);
        let visible = strip_ansi(&rendered);

        assert!(
            visible.lines().all(|line| line.chars().count() <= 80),
            "{rendered}"
        );
        assert!(visible.contains("deliberately"));
    }

    #[test]
    fn list_wraps_to_narrow_terminal_width() {
        let options = OutputOptions {
            delta_format: DeltaFormat::DecimalHours,
            table_style: TableStyle::Utf8Condensed,
            table_inner_borders: TableInnerBorders::Solid,
            locale: Locale::English,
        };
        let mut entry = finished_entry();
        entry.comment = "narrow terminals should still get bounded list output".to_string();

        let rendered = render_tabled_ansi_many_with_width(options, &[entry], 70);
        let visible = strip_ansi(&rendered);

        assert!(
            visible.lines().all(|line| line.chars().count() <= 70),
            "{rendered}"
        );
    }

    fn finished_entry() -> TimeEntry {
        TimeEntry {
            id: "abcdef123456".to_string(),
            start_time: Local.with_ymd_and_hms(2026, 6, 2, 9, 0, 0).unwrap(),
            end_time: Some(Local.with_ymd_and_hms(2026, 6, 2, 10, 0, 0).unwrap()),
            project: "demo".to_string(),
            tags: BTreeSet::from(["work".to_string()]),
            comment: "render check".to_string(),
        }
    }

    fn strip_ansi(value: &str) -> String {
        let mut stripped = String::new();
        let mut chars = value.chars().peekable();
        while let Some(char) = chars.next() {
            if char == '\u{1b}' && chars.peek() == Some(&'[') {
                chars.next();
                for code_char in chars.by_ref() {
                    if code_char.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            stripped.push(char);
        }
        stripped
    }
}
