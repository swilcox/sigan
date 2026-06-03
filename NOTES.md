# Sigan Handoff Notes

## What Was Built

This Rust prototype lives in `rust/sigan` and is ready to be moved into its own repository.

The core design is intentionally simple:

- `src/cli.rs`: clap command definitions and top-level command dispatch
- `src/config.rs`: TOML config loading, default paths, auto-tag rules
- `src/models.rs`: `TimeEntry`, filters, time periods
- `src/service.rs`: business logic around start/stop/list/edit/delete
- `src/storage/file.rs`: TOML/YAML/JSON legacy-compatible storage
- `src/output/mod.rs`: all output rendering, including `comfy-table`
- `src/editor.rs`: shell editor integration
- `src/datetime.rs`: user-entered time parsing and stop-time adjustment

## Key Decisions

- New binary/project name: `sigan`
- Preferred config path: `~/.config/sigan/config.toml`
- Platform config fallback only if preferred config does not exist
- Default data path: `~/.config/sigan/time_entries.toml`
- Existing `sigye` TOML/YAML/JSON files are supported directly
- SQLite intentionally excluded for now
- Default delta format is decimal/tenths of an hour
- ANSI/rich output is table-based using `comfy-table`

## Known Gotchas

Global clap options currently need to come before the subcommand:

```sh
sigan -o json list all
```

This does not currently work:

```sh
sigan list all -o json
```

The default `list` behavior is today-only. Use `list all` or `ls all` to show all entries.

## Useful Manual Checks

```sh
cargo run -- -o json list all
cargo run -- -f /Users/steven/.sigye/time_entries.toml -o json list all
cargo run -- start demo "testing" --tag manual --start-time 09:00
cargo run -- stop "done" --stop-time 09:06
cargo run -- --delta-format human list all
```

Long-comment wrapping check:

```sh
COLUMNS=100 cargo run -- list all
```

## Last Known Test State

Before these notes were added:

```text
cargo test
17 passed
```
