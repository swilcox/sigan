use crate::config::Config;
use crate::datetime::parse_user_time;
use crate::editor::{EditFormat, ShellEditor};
use crate::models::{EntryFilter, TimePeriod};
use crate::output::{
    DeltaFormat, Locale, OutputFormat, OutputOptions, TableInnerBorders, TableStyle, print_export,
    print_many, print_single,
};
use crate::service::TimeTrackingService;
use crate::storage::TimeEntryRepository;
use crate::storage::file::FileRepository;
use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use clap::builder::styling::{AnsiColor, Styles};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Colored styling for `--help` output.
const HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Debug, Parser)]
#[command(name = "sigan")]
#[command(about = "Track time from the command line.")]
#[command(styles = HELP_STYLES)]
struct Cli {
    #[arg(
        long = "completion",
        value_name = "SHELL",
        help = "Generate a shell completion script and exit"
    )]
    completion: Option<Shell>,
    #[arg(short = 'c', long = "config-file")]
    config_file: Option<PathBuf>,
    #[arg(short = 'f', long = "filename")]
    filename: Option<PathBuf>,
    #[arg(short = 'o', long = "output", alias = "output_format", value_enum)]
    output: Option<OutputFormat>,
    #[arg(long = "delta-format", alias = "delta_format")]
    delta_format: Option<String>,
    #[arg(long = "table-style", alias = "table_style")]
    table_style: Option<String>,
    #[arg(long = "table-inner-borders", alias = "table_inner_borders")]
    table_inner_borders: Option<String>,
    #[arg(long = "locale")]
    locale: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Start {
        project: String,
        #[arg(default_value = "")]
        comment: String,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(short = 's', long = "start-time", alias = "start_time")]
        start_time: Option<String>,
    },
    Stop {
        #[arg(default_value = "")]
        comment: String,
        #[arg(short = 's', long = "stop-time", alias = "stop_time")]
        stop_time: Option<String>,
    },
    Status,
    #[command(visible_alias = "ls")]
    List {
        #[arg(value_enum)]
        time_period: Option<PeriodArg>,
        #[arg(long = "start-date", alias = "start_date", value_parser = parse_date)]
        start_date: Option<NaiveDate>,
        #[arg(long = "end-date", alias = "end_date", value_parser = parse_date)]
        end_date: Option<NaiveDate>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "project")]
        projects: Vec<String>,
    },
    #[command(visible_alias = "rm", visible_alias = "del")]
    Delete {
        id: String,
    },
    Edit {
        id: String,
    },
    Export {
        export_filename: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PeriodArg {
    Today,
    Yesterday,
    Week,
    Month,
    All,
}

pub fn run() -> Result<()> {
    run_from(std::env::args_os())
}

fn run_from<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    if let Some(shell) = cli.completion {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut std::io::stdout());
        return Ok(());
    }

    let command = match cli.command {
        Some(command) => command,
        None => {
            Cli::command().print_help()?;
            return Ok(());
        }
    };

    let mut config = Config::load(cli.config_file)?;
    if let Some(filename) = cli.filename {
        config.data_filename = filename;
    }
    if let Some(delta_format) = cli.delta_format {
        config.delta_format = delta_format;
    }
    if let Some(table_style) = cli.table_style {
        config.table_style = table_style;
    }
    if let Some(table_inner_borders) = cli.table_inner_borders {
        config.table_inner_borders = table_inner_borders;
    }
    if let Some(locale) = cli.locale {
        config.locale = locale;
    }
    let output = match cli.output {
        Some(output) => output,
        None => OutputFormat::parse(&config.output)?,
    };
    let options = OutputOptions {
        delta_format: DeltaFormat::parse(&config.delta_format)?,
        table_style: TableStyle::parse(&config.table_style)?,
        table_inner_borders: TableInnerBorders::parse(&config.table_inner_borders)?,
        locale: Locale::parse(&config.locale)?,
    };
    let repository = FileRepository::new(&config.data_filename)?;
    let mut service = TimeTrackingService::new(repository);

    match command {
        Command::Start {
            project,
            comment,
            tags,
            start_time,
        } => {
            let start_time = start_time.as_deref().map(parse_user_time).transpose()?;
            let mut tags: BTreeSet<String> = tags.into_iter().collect();
            tags.extend(config.apply_auto_tags(&project)?);
            let entry = service.start_tracking(project, start_time, comment, tags)?;
            print_single(output, options, Some(&entry))
        }
        Command::Stop { comment, stop_time } => {
            let stop_time = stop_time.as_deref().map(parse_user_time).transpose()?;
            let entry = service.stop_tracking(stop_time, Some(comment))?;
            print_single(output, options, entry.as_ref())
        }
        Command::Status => {
            let entry = service.status()?;
            print_single(output, options, entry.as_ref())
        }
        Command::List {
            time_period,
            start_date,
            end_date,
            tags,
            projects,
        } => {
            let entries = service.list(EntryFilter {
                time_period: time_period.map(Into::into),
                start_date,
                end_date,
                tags: into_set(tags),
                projects: into_set(projects),
                ..EntryFilter::default()
            })?;
            print_many(output, options, &entries)
        }
        Command::Delete { id } => {
            let entry = service.delete(&id)?;
            print_single(output, options, Some(&entry))
        }
        Command::Edit { id } => {
            let editor = ShellEditor::new(config.editor, EditFormat::parse(&config.editor_format)?);
            let entry = service.edit(&id, &editor)?;
            print_single(output, options, Some(&entry))
        }
        Command::Export { export_filename } => {
            let mut output_repo = FileRepository::new(&export_filename)?;
            let entries = service.list(EntryFilter {
                time_period: Some(TimePeriod::All),
                ..EntryFilter::default()
            })?;
            output_repo.save_all(&entries)?;
            print_export(output, options, entries.len(), &export_filename)
        }
    }
}

fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|err| anyhow!(err))
}

fn into_set(values: Vec<String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

impl From<PeriodArg> for TimePeriod {
    fn from(value: PeriodArg) -> Self {
        match value {
            PeriodArg::Today => Self::Today,
            PeriodArg::Yesterday => Self::Yesterday,
            PeriodArg::Week => Self::Week,
            PeriodArg::Month => Self::Month,
            PeriodArg::All => Self::All,
        }
    }
}
