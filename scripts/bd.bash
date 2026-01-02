# back-directory (bd) - bash wrapper for bd-core

if [[ -n ${BD_LOADED-} ]]; then
  return 0
fi
BD_LOADED=1

BD_MAX_BACK=999

_bd_default_core_bin() {
  local bin
  bin=$(command -v bd-core 2>/dev/null)
  if [[ -n $bin ]]; then
    printf '%s\n' "$bin"
    return 0
  fi
  local bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
  if [[ -x "$bin_dir/bd-core" ]]; then
    printf '%s\n' "$bin_dir/bd-core"
    return 0
  fi
  printf '%s\n' "bd-core"
}

BD_CORE_BIN=${BD_CORE_BIN:-$(_bd_default_core_bin)}

_bd_sanitize_session_key() {
  local key="$1"
  key=${key#/dev/}
  key=${key//\//_}
  key=${key//[^A-Za-z0-9._-]/_}
  printf '%s\n' "$key"
}

_bd_compute_session_id() {
  local key
  if [[ -n ${TTY-} ]]; then
    key="${TTY}-$$"
  else
    key="${PPID-$$}-$$-${HOSTNAME:-unknown}-${USER:-unknown}"
  fi
  _bd_sanitize_session_key "$key"
}

BD_SESSION_ID=${BD_SESSION_ID:-$(_bd_compute_session_id)}

_bd_require_core() {
  if [[ -x $BD_CORE_BIN ]]; then
    return 0
  fi
  if command -v "$BD_CORE_BIN" >/dev/null 2>&1; then
    return 0
  fi
  printf '%s\n' "bd: bd-core not found"
  return 1
}

_bd_record() {
  _bd_require_core || return 1
  "$BD_CORE_BIN" record --session "$BD_SESSION_ID" --pwd "$PWD"
}

back_directory_prompt() {
  if [[ -n ${BD_SUPPRESS_RECORD-} ]]; then
    unset BD_SUPPRESS_RECORD
    BD_LAST_PWD=$PWD
    return 0
  fi
  if [[ $PWD != "$BD_LAST_PWD" ]]; then
    BD_LAST_PWD=$PWD
    _bd_record >/dev/null 2>&1
  fi
}

_bd_add_prompt_command() {
  local hook="back_directory_prompt"
  local decl
  decl=$(declare -p PROMPT_COMMAND 2>/dev/null || true)
  if [[ $decl == "declare -a"* ]]; then
    local cmd
    for cmd in "${PROMPT_COMMAND[@]}"; do
      if [[ $cmd == "$hook" ]]; then
        return 0
      fi
    done
    PROMPT_COMMAND+=("$hook")
  else
    if [[ -z ${PROMPT_COMMAND-} ]]; then
      PROMPT_COMMAND="$hook"
    elif [[ $PROMPT_COMMAND != *"$hook"* ]]; then
      PROMPT_COMMAND="${PROMPT_COMMAND};$hook"
    fi
  fi
}

bd() {
  local arg="${1-}"

  if [[ -z $arg ]]; then
    arg=1
  fi

  if [[ $arg=="h" || $arg == "help" || $arg == "-h" || $arg == "--help" ]]; then
    cat <<'EOF'
usage: bd [N|c|ls|doctor|optimize|vacuum|h]

Commands:
  bd                 go back 1 directory
  bd N               go back N directories (1 <= N <= 999)
  bd c               cancel the last bd command
  bd ls [N]          list recent targets with their N values (default 10)
  bd doctor [opts]   show database status
  bd optimize        rebuild SQLite DB to reclaim space (can be slow)
  bd vacuum          reset SQLite DB (deletes all history)
  bd h               show this help

Aliases:
  bd cancel          same as: bd c
  bd list [N]        same as: bd ls [N]
  bd help            same as: bd h
  bd -h, bd --help   show this help

Options:
  bd doctor --integrity   run SQLite integrity check (can be slow)
  bd doctor --json        output machine-readable JSON
  bd vacuum --yes|--y     skip confirmation prompt (deletes all history)

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

  if [[ $arg == "optimize" ]]; then
    _bd_require_core || return 1
    "$BD_CORE_BIN" optimize || return $?
    return 0
  fi

  if [[ $arg == "vacuum" ]]; then
    _bd_require_core || return 1
    if (( $# >= 2 )) && [[ ${2-} == "--yes" || ${2-} == "--y" ]]; then
      "$BD_CORE_BIN" vacuum --yes || return $?
      return $?
    fi
    printf '%s' "bd: vacuum deletes all history. Continue? [y/N] "
    local reply
    read -r reply
    if [[ $reply != "y" && $reply != "Y" ]]; then
      return 1
    fi
    "$BD_CORE_BIN" vacuum --yes || return $?
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
      printf '%s\n' "bd: too many arguments"
      return 1
    fi
    local limit="${2-}"
    if [[ -z $limit ]]; then
      limit=10
    fi
    if ! [[ $limit =~ ^[0-9]+$ ]] || (( limit <= 0 )); then
      printf '%s\n' "bd: usage: bd ls [N]"
      return 1
    fi
    if (( limit > BD_MAX_BACK )); then
      printf '%s\n' "bd: max is $BD_MAX_BACK"
      return 1
    fi
    _bd_require_core || return 1
    "$BD_CORE_BIN" list --session "$BD_SESSION_ID" --limit "$limit" || return $?
    return 0
  fi

  if (( $# > 1 )); then
    printf '%s\n' "bd: too many arguments"
    return 1
  fi

  if ! [[ $arg =~ ^[0-9]+$ ]] || (( arg <= 0 )); then
    printf '%s\n' "bd: usage: bd [N|c|ls]"
    return 1
  fi

  if (( arg > BD_MAX_BACK )); then
    printf '%s\n' "bd: max is $BD_MAX_BACK"
    return 1
  fi

  _bd_require_core || return 1
  local target
  target=$("$BD_CORE_BIN" back --print-path --session "$BD_SESSION_ID" --n "$arg") || return $?
  BD_SUPPRESS_RECORD=1
  builtin cd -- "$target"
}

_bd_add_prompt_command
BD_LAST_PWD=$PWD
_bd_record >/dev/null 2>&1
