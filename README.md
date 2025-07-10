# zcd

A fast and intelligent directory jumping tool written in Rust, inspired by z.

## Features

- **Smart Ranking**: Improved frecency algorithm that combines visit frequency and recency
- **Fuzzy Matching**: Fast fzy matching algorithm for flexible directory search
- **Path Validation**: Automatic cleanup of non-existent directories
- **Robust Error Handling**: Graceful handling of deleted or moved directories
- **Shell Integration**: Easy integration with zsh (bash support planned)
- **Data Compatibility**: Compatible with z data format for easy migration

## Installation

### From Source

```bash
git clone <repository-url>
cd zcd

# Method 1: Install directly
cargo install --path ./zcd --locked

# Method 2: Build and copy manually
cargo build --release
# The binary will be available at target/release/zcd
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

zcd uses an improved frecency algorithm that combines:

- **Frequency**: How often you visit a directory (logarithmic scaling)
- **Recency**: How recently you visited a directory (logarithmic decay)
- **Path Validation**: Automatically removes non-existent directories

The ranking score is calculated as:
```
score = (ln(visit_count) + 1) × recency_factor × 100
```

Where `recency_factor` decreases logarithmically as time passes since the last visit.

## Configuration

zcd stores its configuration in `~/.config/zcd/config` and data in the configured datafile location. The configuration supports:

- `max_age`: Entry lifetime in seconds
- `datafile`: Path to the data storage file
- `exclude_dirs`: Directories to exclude from tracking
- `debug`: Enable debug mode

## Recent Changes (v1.2.0)

- ✅ **Improved Ranking Algorithm**: Now properly distinguishes high-frequency paths
- ✅ **Path Validation**: Automatic cleanup of non-existent directories
- ✅ **Enhanced Error Handling**: Graceful handling in zsh script
- ✅ **Visit Count Tracking**: Better frequency analysis
- ✅ **Code Quality**: Resolved all linting warnings

## Roadmap

- [x] List entries with ranking
- [x] Insert and delete entries  
- [x] Zsh shell integration
- [x] Fuzzy matching (fzy algorithm)
- [x] Improved frecency algorithm
- [x] Path validation and cleanup
- [x] Robust error handling
- [ ] Bash shell integration
- [ ] Configuration management improvements
- [ ] Performance optimizations
