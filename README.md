# zcd

z implementation in Rust.

## Features

- z-like behavior with script
- longest substring matching algorithm for keyword
- z compatible data format

## Usage

```
zcd

USAGE:
    zcd [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -v, --verbose

SUBCOMMANDS:
    config    config management
    delete    delete an entry
    export    Export data into datafile
    help      Print this message or the help of the given subcommand(s)
    import    Import data from datafile
    init      install shell script
    insert    insert or update an entry
    list      list all entries
    query     query an entry based on keyword
    server    Server management
```

### Run as daemon in background

```
zcd --daemon
```

## Roadmap

- [x] list entries
- [x] insert entries
- [x] zsh shell script wrapper
  - [x] zsh
- [x] use better algorithm
  - [x] fzy
- [x] cli mode
- [x] server mode
