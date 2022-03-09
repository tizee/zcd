# zcd

z implementation in Rust.

## Features

- z-like navigator
- interactive fuzzy finder

## zsh

```zsh
zcd install zsh # echo 'source ~/.config/zcd/zcd.plugin.zsh >> ~/.zshrc'
```

This will create script file `zcd.plugin.zsh` in your `$HOME/.config/zcd`.

## Usage

```
zcd -l/--list # list of entries
zcd -a/--add <entry> # add a path into db
zcd -d/--delete <entry> # remove a path from db
zcd -i/--interactive # manage db
zcd --daemon # run as daemon process to query in memory, by default it would write data into disk when needed
zcd --init <shell> # create cli script for given shell
```

```
z <path>
```

## Advanced usage

```
z -gi repo
```

### Run as daemon in background

```
zcd --daemon
```

## Roadmap

- [ ] list entries
- [ ] insert entries
- [ ] configure priorities
- [ ] std output
- [ ] use fuzzy search algorithm
- [ ] cli mode
- [ ] daemon mode
