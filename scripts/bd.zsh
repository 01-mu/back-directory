# back-directory (bd) - zsh wrapper for bd-core

# Skip initialization when this file is sourced more than once.
if [[ -n ${BD_LOADED-} ]]; then
  return 0
fi
BD_LOADED=1

BD_MAX_BACK=999

_bd_default_core_bin() {
  emulate -L zsh
  local bin
  bin=$(command -v bd-core 2>/dev/null)
  if [[ -n $bin ]]; then
    print -r -- "$bin"
    return 0
  fi
  local bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
  if [[ -x "$bin_dir/bd-core" ]]; then
    print -r -- "$bin_dir/bd-core"
    return 0
  fi
  print -r -- "bd-core"
}

# BD_CORE_BIN can be set via an environment variable at startup (e.g., in zshrc).
BD_CORE_BIN=${BD_CORE_BIN:-$(_bd_default_core_bin)}

_bd_sanitize_session_key() {
  emulate -L zsh
  local key="$1"
  key=${key#/dev/}                 # Example: /dev/pts/2 -> pts/2
  key=${key//\//_}                 # Example: pts/2 -> pts_2
  key=${key//[^A-Za-z0-9._-]/_}    # Example: a b -> a_b
  print -r -- "$key"
}

_bd_compute_session_id() {
  emulate -L zsh
  local key
  if [[ -n ${TTY-} ]]; then
    key="${TTY}-$$"
  else
    key="${PPID}-$$-${HOST:-unknown}-${USER:-unknown}"
  fi
  _bd_sanitize_session_key "$key"
}

BD_SESSION_ID=${BD_SESSION_ID:-$(_bd_compute_session_id)}

if [[ -z ${chpwd_functions-} ]]; then
  typeset -ga chpwd_functions
fi

_bd_require_core() {
  emulate -L zsh
  if [[ -x $BD_CORE_BIN ]]; then
    return 0
  fi
  if command -v "$BD_CORE_BIN" >/dev/null 2>&1; then
    return 0
  fi
  print -r -- "bd: bd-core not found"
  return 1
}

_bd_record() {
  emulate -L zsh
  _bd_require_core || return 1
  "$BD_CORE_BIN" record --session "$BD_SESSION_ID" --pwd "$PWD"
}

back_directory_chpwd() {
  emulate -L zsh
  if [[ -n ${BD_SUPPRESS_RECORD-} ]]; then
    unset BD_SUPPRESS_RECORD
    BD_LAST_PWD=$PWD
    return 0
  fi
  _bd_record
  BD_LAST_PWD=$PWD
}

bd() {
  emulate -L zsh
  local arg="${1-}"

  if [[ -z $arg ]]; then
    arg=1
  fi

  if [[ $arg == "h" || $arg == "help" || $arg == "-h" || $arg == "--help" ]]; then
    cat <<'EOF'
usage: bd [N|c|ls|doctor|h]

Commands:
  bd                 go back 1 directory
  bd N               go back N directories (1 <= N <= 999)
  bd c               cancel the last bd command
  bd ls [N]          list recent targets with their N values (default 10)
  bd doctor [opts]   show database status
  bd h               show this help

Aliases:
  bd cancel          same as: bd c
  bd list [N]        same as: bd ls [N]
  bd help            same as: bd h
  bd -h, bd --help   show this help

Doctor options:
  --integrity        run SQLite integrity check (can be slow)
  --json             output machine-readable JSON

Note:
  back-directory uses a local SQLite database.
EOF
    return 0
  fi

  if [[ $arg == "doctor" ]]; then
    shift
    _bd_require_core || return 1
    "$BD_CORE_BIN" doctor "$@" || return $?
    return 0
  fi

  if [[ $arg == "c" || $arg == "cancel" ]]; then
    _bd_require_core || return 1
    local target
    target=$("$BD_CORE_BIN" cancel --session "$BD_SESSION_ID") || return $?
    BD_SUPPRESS_RECORD=1
    builtin cd -- "$target"
    return $?
  fi

  if [[ $arg == "ls" || $arg == "list" ]]; then
    if (( $# > 2 )); then
      print -r -- "bd: too many arguments"
      return 1
    fi
    local limit="${2-}"
    if [[ -z $limit ]]; then
      limit=10
    fi
    if [[ $limit != <-> || $limit -le 0 ]]; then
      print -r -- "bd: usage: bd ls [N]"
      return 1
    fi
    if (( limit > BD_MAX_BACK )); then
      print -r -- "bd: max is $BD_MAX_BACK"
      return 1
    fi
    _bd_require_core || return 1
    "$BD_CORE_BIN" list --session "$BD_SESSION_ID" --limit "$limit" || return $?
    return 0
  fi

  if (( $# > 1 )); then
    print -r -- "bd: too many arguments"
    return 1
  fi

  if [[ $arg != <-> || $arg -le 0 ]]; then
    print -r -- "bd: usage: bd [N|c|ls|h]"
    return 1
  fi

  if (( arg > BD_MAX_BACK )); then
    print -r -- "bd: max is $BD_MAX_BACK"
    return 1
  fi

  _bd_require_core || return 1
  local target
  target=$("$BD_CORE_BIN" back --print-path --session "$BD_SESSION_ID" --n "$arg") || return $?
  BD_SUPPRESS_RECORD=1
  builtin cd -- "$target"
}

if (( ${chpwd_functions[(I)back_directory_chpwd]} == 0 )); then
  chpwd_functions+=(back_directory_chpwd)
fi

BD_LAST_PWD=$PWD
_bd_record >/dev/null 2>&1
