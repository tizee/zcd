# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Build & Development Commands

```bash
# Build (default member is `zcd` binary)
cargo build
cargo build --release

# Run all tests
cargo test --all

# Format code
make fmt
# Check formatting without modifying (for CI/pre-commit)
make fmt-check

# Lint (treats warnings as errors)
make lint
# Equivalent to: cargo clippy --all-targets --all-features -- -D warnings

# Install from source
cargo install --path ./zcd --locked
```

There is no CI/CD pipeline configured. All checks are manual.

## Architecture

This is a Cargo workspace with a single crate. Full algorithm design doc:
`analysis/matching-and-ranking.md`.

### `zcd/` — binary crate (the CLI)
A `z`-inspired directory jumper. Module layout:

| Module | Purpose |
|--------|---------|
| `cli/mod.rs` | clap derive CLI definition, subcommand dispatch, `AppExt::run()` |
| `cli/client.rs` | `Client` wraps `Database`, provides the callable API surface |
| `config/mod.rs` | `Config` struct, custom key=value config parser, `config_file()` resolution |
| `db/mod.rs` | `Database` facade — load/save, import/export, aging trigger, dirty flag |
| `db/dir.rs` | Core data model: `Dir` (path, rank, last_accessed), `DirList`, `OpsDelegate` trait, `frecency()` and `DirList::age()` |
| `db/data.rs` | Single z-compatible datafile codec: `path|rank|last_accessed` |
| `fuzzy/mod.rs` | Typo-tolerant fuzzy matching engine (formerly a separate crate, now inlined) |
| `fuzzy/fzy.rs` | fzy DP scorer ported from C, extended with skip-needle tolerance |
| `fuzzy/score.rs` | Scoring constants: `SCORE_MIN`/`SCORE_MAX` (±∞) |

### Data Flow

1. On shell `chpwd` hook, the zsh plugin calls `zcd insert -- <pwd>`
2. `Client::insert()` calls `Database::insert_or_update()` → rank += 1 → aging sweep if needed → `save()` to disk
3. `z <pattern>` calls `zcd query -- <pattern>` → `Database::query()` → fuzzy match via `zcd::fuzzy` module → sort by (score bucket, frecency) → prints top path to stdout (the shell's `cd` target); exits non-zero on no match

### Shell Integration

`scripts/zcd.plugin.zsh` provides `z` (jump by fuzzy query) and `zi` (interactive fzf picker). It registers a `chpwd` hook that calls `zcd insert -- "$PWD"` on every directory change. Currently zsh-only.

## Key Design Details

- **Config format is custom**: `~/.config/zcd/config` uses a hand-rolled key=value parser (`config/mod.rs`). Not TOML/JSON/YAML.
- **Data format is plain text and z-compatible**: entries stored as `path|rank|last_accessed` lines, sorted by rank on write. `zcd import/export` read/write the same format for migration to/from other z-like tools.
- **Frecency algorithm (zoxide model)**: stored rank accumulates +1 per visit (no cap, never saturates). `frecency()` in `db/dir.rs` scales rank at query time by a recency bucket: ×4 (< 1h), ×2 (< 1d), ×0.5 (< 1w), ×0.25 (older). When total rank exceeds config `max_age`, all ranks decay ×0.9 and entries below 1.0 are dropped (`DirList::age`).
- **Query ordering**: fuzzy score bucketed to 0.1 is the primary key; frecency breaks ties. `query -r`/`list -r` display the frecency value, not the stored rank.
- **Path validation**: `query`/`list` skip entries whose directories no longer exist on the filesystem.
- **Environment variable**: `$ZCD_CONFIG_FILE` overrides the config file path. Config file path defaults to `$XDG_CONFIG_HOME/zcd/config` or `~/.config/zcd/config`.
