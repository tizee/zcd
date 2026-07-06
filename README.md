# zcd

A fast and intelligent directory jumping tool written in Rust, inspired by z.

## Features

- **Smart Ranking**: Improved frecency algorithm that combines visit frequency and recency
- **Typo-tolerant Matching**: fzy-based fuzzy engine with skip-needle tolerance — transposed and mistyped characters still match
- **Path Validation**: Automatic cleanup of non-existent directories
- **Robust Error Handling**: Graceful handling of deleted or moved directories
- **Shell Integration**: Easy integration with zsh (bash support planned)
- **Data Compatibility**: Compatible with z data format for easy migration

## Installation

### From Source

```bash
git clone <repository-url>
cd zcd

# Install directly (release profile with LTO)
cargo install --path . --locked

# Or build manually
cargo build --release
# Binary at target/release/zcd
```

After installation, `zcd` will be available in your `$PATH` (usually `~/.cargo/bin/zcd`).

## Shell Integration

### Zsh

Add the following to your `.zshrc`:

```bash
# Source the zcd plugin
source /path/to/zcd/scripts/zcd.plugin.zsh
```

This provides:
- `z <pattern>` - Jump to a directory matching the pattern
- `zi` - Interactive directory selection using fzf

## Usage

### Basic Commands

```
zcd CLI tool

USAGE:
    zcd [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -v, --verbose    Enable verbose output

SUBCOMMANDS:
    clear     Clear all history
    config    Configuration management
    delete    Delete an entry
    export    Export data to file
    help      Print help information
    import    Import data from file
    insert    Insert or update an entry
    list      List all entries (use --rank to show scores)
    query     Query entries by keyword
    version   Display version information
```

### Examples

```bash
# Add current directory to database
zcd insert .

# Jump to a directory containing "project"
z project

# List all directories with ranking scores
zcd list --rank

# Interactive directory selection
zi

# Clear all history
zcd clear
```

## Algorithm

zcd uses a **frecency** algorithm (zoxide model) combining visit frequency with recency:

- **Rank**: +1 per visit, never saturates — the more you visit, the higher the rank
- **Recency multiplier** at query time: ×4 (&lt; 1 hour), ×2 (&lt; 1 day), ×0.5 (&lt; 1 week), ×0.25 (older)
- **Aging**: when total rank exceeds `max_age`, all entries decay ×0.9 and those below 1.0 are pruned
- **Typo tolerance**: up to 25% of needle characters may go unmatched (skip-needle penalty); needles ≤ 3 characters stay strict

Query results are ordered by fuzzy score (bucketed to 0.1), with frecency breaking ties.

## Configuration

zcd stores its configuration in `~/.config/zcd/config` and data in the configured datafile location. The configuration supports:

- `max_age`: Entry lifetime in seconds
- `datafile`: Path to the data storage file
- `exclude_dirs`: Directories to exclude from tracking
- `debug`: Enable debug mode

## Recent Changes (v1.3.0)

- **Typo-tolerant matching**: transposed/mistyped characters in queries still find targets (skip-needle algorithm)
- **Inlined fuzzy engine**: simplified to a single crate — no extra workspace dependency
- **Optimized release profile**: fat LTO, single codegen unit, stripped symbols, panic=abort
- **Previous (v1.2.0)**: Improved ranking, path validation, enhanced error handling

## Roadmap

- [x] Typo-tolerant fuzzy matching (skip-needle)
- [x] Frevency algorithm (zoxide model)
- [x] Path validation and cleanup
- [x] Zsh shell integration
- [x] z-compatible data import/export
- [ ] Bash shell integration
- [ ] Performance optimizations
