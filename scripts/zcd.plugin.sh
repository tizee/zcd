#!/usr/bin/env bash
# vim:fmr={,}

__zcd_pwd() {
  builtin pwd -P
}

__zcd_cd() {
  builtin cd "$@"
  [[ -n $ZCD_ECHO ]] && __zcd_pwd
}

__zcd_cdi() {
  builtin cd "$@"
  [[ -n $ZCD_ECHO ]] && __zcd_pwd
}

# utils
__zcd_unset() {
  builtin unalias "$@" &>/dev/null
  builtin unset "$@" &>/dev/null
}

# jump to a directory with keywords
__zcd_z() {
  if [[ "$#" -eq 0 ]]; then
    __zcd_cd ~
  elif [[ "$#" -eq 1 ]] && [[ "$1" = "-" ]]; then
    # jumps to zsh's $OLDPWD
    if [ -n "$OLDPWD" ]; then
      __zcd_cd "${OLDPWD}"
    else
      builtin printf -n 'zcd: $OLDPWD not set'
      return 1
    fi
  elif [[ "$#" -eq 1 ]] && [[ -d "$1" ]]; then
    # use cd directly if $1 is a valid path
    __zcd_cd $1
  else
    # sorting candidates
    __zcd_result="$(zcd query -- "$@")" && __zcd_cd "$__zcd_result"
    return 0
  fi
}

# query interactively
__zcd_zi(){
    __zcd_result="$(zcd list | fzf "$@")" && __zcd_cd "$__zcd_result"
}

# zsh hook
__zcd_insert_or_update() {
  zcd insert -- "$(__zcd_pwd)"
}

# zcd
__zcd_unset "z"
z() {
  __zcd_z "$@"
}

# interactive
zi() {
  __zcd_zi "$@"
}

