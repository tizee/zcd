# zcd

z implementation in Rust.

## Features

- z-like behavior with script
- fzy matching algorithm
- z compatible data format

## Usage

```
zcd CLI tool

USAGE:
    zcd [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -v, --verbose

SUBCOMMANDS:
    clear     clear all history
    config    config management
    delete    delete an entry
    export    Export data into datafile
    help      Print this message or the help of the given subcommand(s)
    import    Import data from datafile
    insert    insert or update an entry
    list      list all entries
    query     query an entry based on keyword
```

## Build from source

```
cargo build --path .
```

## Roadmap

- [x] list entries
- [x] insert entries
- [x] zsh shell script wrapper
  - [x] zsh
- [x] use better algorithm
  - [x] fzy
- [x] cli mode
- ~~[x] server mode~~
