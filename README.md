# sigan

`sigan` is a Rust command-line time tracker.

It is a new project inspired by `sigye`, with a new name and config system, while preserving the important compatibility target: reading and writing existing `sigye` TOML/YAML/JSON time-entry files.

## Status

Usable and under active development. `sigan` supports the core daily time-tracking workflow:

- Start/stop/status/list/delete/export
- Edit entries in `$EDITOR`
- TOML/YAML/JSON storage
- JSON/YAML/CSV/Markdown/plain text output
- ANSI Unicode table output with colors and wrapping
- Decimal/tenths-of-hour and human delta formats
- Configurable ANSI table style and Korean output labels
- Auto-tag rules from config
- Legacy `sigye` file-shape compatibility

Still planned or intentionally deferred:

- SQLite storage is intentionally out of scope for now
- Shell completions
- Full exact output parity with Python `sigye`

## Install

Install from crates.io:

```sh
cargo install sigan
```

Run from a local checkout:

```sh
cargo run -- list all
cargo run -- start my-project "working on something" --tag work
cargo run -- stop "done"
```

Global options go before the subcommand:

```sh
cargo run -- -o json list all
cargo run -- --delta-format human list all
```

Build a local release binary:

```sh
cargo build --release
./target/release/sigan list all
```

## Config

Preferred config file:

```text
~/.config/sigan/config.toml
```

If that file does not exist, `sigan` falls back to the OS-native config path, such as:

```text
~/Library/Application Support/sigan/config.toml
```

Default data file:

```text
~/.config/sigan/time_entries.toml
```

Example config:

```toml
data_filename = "/Users/steven/.sigye/time_entries.toml"
output = "ansi"
editor = "nano"
editor_format = "yaml"
delta_format = "decimal"
table_style = "utf8-condensed"
table_inner_borders = "solid"
locale = "en_US"

[[auto_tag_rules]]
pattern = "^PROJ-\\d+"
match_type = "regex"
tags = ["work", "billable"]
```

## Output

Output formats:

- `ansi` or `rich`
- `text`
- `json`
- `yaml`
- `csv`
- `markdown` or `md`

Delta formats:

- `decimal`, `tenths`, `hours`: `0.1 hours`, `1.6 hours`
- `human`, `clock`, `hours-minutes`: `1h 10m`

Default delta format is `decimal`.

ANSI table style options:

- `utf8`, `utf8-condensed`, `utf8-borders-only`, `utf8-horizontal-only`, `utf8-no-borders`
- `ascii`, `ascii-condensed`, `ascii-borders-only`, `ascii-borders-only-condensed`, `ascii-horizontal-only`
- `none`

`table_inner_borders` can be `solid` or `dotted`. The default ANSI style is `utf8-condensed` with `solid` inner borders, which avoids horizontal separators between every row.

Locale options:

- `en`, `en_US`, `en_GB`
- `ko`, `ko_KR`

The Korean locale translates output labels for the ANSI and plain status message paths.

## Commands

```sh
sigan start <project> [comment] [--tag TAG] [--start-time HH:MM]
sigan stop [comment] [--stop-time HH:MM]
sigan status
sigan list [today|yesterday|week|month|all]
sigan ls [today|yesterday|week|month|all]
sigan delete <id-prefix>
sigan rm <id-prefix>
sigan del <id-prefix>
sigan edit <id-prefix>
sigan export <filename>
```

With no period, `list` shows today's entries. If an active entry started before today, `list` includes entries from that active entry's date through today.

Filters:

```sh
sigan list --project abc+
sigan list --tag billable
sigan list --start-date 2026-06-01 --end-date 2026-06-02
```

Project filters ending in `*`, `+`, or `.` are prefix matches.

## Storage Format

Legacy `sigye` TOML shape:

```toml
[[entries]]
id = "cc648e6e86f747b183f1554643a4543b"
start_time = "2024-11-06T19:40:44.137578-06:00"
end_time = "2024-11-06T20:01:05.040753-06:00"
project = "abc-1234"
tags = ["learning", "research"]
comment = "wow, what the?"
```

YAML and JSON use the same `entries` root and field names.

## Development

```sh
cargo fmt
cargo test
```

Current test coverage includes:

- Legacy TOML/YAML/JSON reads
- Storage round-trips
- Auto-tag config parsing
- Delta formatting
- Start/stop rollover behavior
- Edit command service path
- Filters and partial IDs

## Roadmap

- Add integration tests for CLI stdout/stderr and exit codes
- Consider a config inspection command, e.g. `sigan config path`
- Consider shell completions
- Decide whether SQLite compatibility should be added later
- Improve packaging beyond Cargo, such as Homebrew
