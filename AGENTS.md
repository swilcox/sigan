# AGENTS.md

## Project

This is `sigan`, a Rust command-line time tracker. It started as a Rust replacement for the Python `sigye` project, but it is intentionally its own CLI and config system.

Core compatibility goal: `sigan` must be able to read and write existing `sigye` TOML/YAML/JSON time-entry files.

SQLite support from `sigye` is intentionally out of scope for now.

## Commands

Run from this directory:

```sh
cargo test
cargo fmt
cargo run -- list all
```

Global options must currently appear before the subcommand:

```sh
cargo run -- -o json list all
```

not:

```sh
cargo run -- list all -o json
```

## Defaults

Preferred config file:

```text
~/.config/sigan/config.toml
```

Fallback config file, used only if the preferred file does not exist:

```text
~/Library/Application Support/sigan/config.toml
```

Do not support the loose file `~/.config/sigan.toml`.

Default data file:

```text
~/.config/sigan/time_entries.toml
```

## Current Config Shape

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

`data_filename` can be omitted; it defaults to `~/.config/sigan/time_entries.toml`.

## Important Behavior

- `list` with no period shows today's entries only, plus an active older entry if relevant.
- `list all` and `ls all` should show every entry.
- `start` stops any active entry before creating the new one.
- Manual tags and auto tags are merged and deduplicated.
- Partial IDs are supported for `edit` and `delete`.
- ANSI output uses `comfy-table` with Unicode borders, colors, dynamic wrapping, and terminal-width detection.
- ANSI table style is configurable; the default is condensed UTF-8 with solid inner borders.
- `locale = "ko_KR"` translates the same output labels covered by the Python version's Korean gettext file.
- In non-TTY output, table width uses `$COLUMNS` if set and at least 80, otherwise 100.
- Default delta output is decimal/tenths of an hour, e.g. `0.1 hours` for 6 minutes.
- `--delta-format human` preserves the `1h 10m` style.

## Editing

`edit <id-prefix>` shells out to the configured editor using a temporary YAML or TOML file. YAML is the default edit format.

The TOML edit format stores active `end_time` as `"null"` because TOML has no native null.

## Storage Compatibility

Supported legacy file formats:

- TOML
- YAML
- JSON

Expected shape:

```toml
[[entries]]
id = "..."
start_time = "2026-06-02T09:00:00-05:00"
end_time = "2026-06-02T10:00:00-05:00"
project = "example"
tags = ["work"]
comment = "..."
```

## Development Guidance

- Keep the storage layer compatible with existing `sigye` files.
- Prefer focused tests around storage round-trips, config parsing, filtering, and command behavior.
- Keep output-specific logic in `src/output/mod.rs`; avoid leaking display concerns into models or storage.
- Keep config parsing in `src/config.rs`; avoid adding environment-variable compatibility with Python `sigye` unless explicitly requested.
- Do not add SQLite until it is explicitly brought back into scope.
