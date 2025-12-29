# back-directory (bd) - zsh wrapper for bd-core

if [[ -n ${BD_LOADED-} ]]; then
  return 0
fi
BD_LOADED=1

BD_MAX_BACK=99

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

BD_CORE_BIN=${BD_CORE_BIN:-$(_bd_default_core_bin)}

_bd_sanitize_session_key() {
  emulate -L zsh
  local key="$1"
  key=${key#/dev/}
  key=${key//\//_}
  key=${key//[^A-Za-z0-9._-]/_}
  print -r -- "$key"
}

_bd_compute_session_id() {
  emulate -L zsh
  local key
  if [[ -n ${TTY-} ]]; then
    key="$TTY"
  else
    key="${PPID}-${HOST:-unknown}-${USER:-unknown}"
  fi
  _bd_sanitize_session_key "$key"
}

if [[ -z ${BD_SESSION_ID-} && -n ${BD_SESSION_KEY-} ]]; then
  BD_SESSION_ID="$BD_SESSION_KEY"
fi

BD_SESSION_ID=${BD_SESSION_ID:-$(_bd_compute_session_id)}
BD_SESSION_KEY=${BD_SESSION_KEY:-$BD_SESSION_ID}

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
    return 0
  fi
  _bd_record
}

bd() {
  emulate -L zsh
  local arg="${1-}"

  if [[ -z $arg ]]; then
    arg=1
  fi

  if [[ $arg == "c" || $arg == "cancel" ]]; then
    _bd_require_core || return 1
    local target
    target=$("$BD_CORE_BIN" cancel --session "$BD_SESSION_ID") || return $?
    BD_SUPPRESS_RECORD=1
    builtin cd -- "$target"
    return $?
  fi

  if (( $# > 1 )); then
    print -r -- "bd: too many arguments"
    return 1
  fi

  if [[ $arg != <-> || $arg -le 0 ]]; then
    print -r -- "bd: usage: bd [N|c]"
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

_bd_record >/dev/null 2>&1
