# back-directory (bd) - simple directory backtracking for zsh

# Guard against multiple loads
if [[ -n ${BD_LOADED-} ]]; then
  return 0
fi
BD_LOADED=1

BD_STATE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/back-directory"
BD_HISTORY_FILE="$BD_STATE_DIR/history.txt"
BD_CURSOR_FILE="$BD_STATE_DIR/cursor"
BD_LAST_BD_FILE="$BD_STATE_DIR/last_bd"
BD_LAST_BD_ARMED_FILE="$BD_STATE_DIR/last_bd_armed"
BD_LAST_BD_TOKEN_FILE="$BD_STATE_DIR/last_bd_token"
BD_LAST_CMD_FILE="$BD_STATE_DIR/last_cmd"

_bd_mkdir_state() {
  emulate -L zsh
  mkdir -p "$BD_STATE_DIR"
}

_bd_write_atomic() {
  emulate -L zsh
  local file="$1"
  local tmp="${file}.tmp.$$"
  shift
  printf '%s\n' "$@" > "$tmp" && mv "$tmp" "$file"
}

_bd_lock_acquire() {
  emulate -L zsh
  local lockdir="$BD_STATE_DIR/.lock"
  local i
  for i in {1..25}; do
    if mkdir "$lockdir" 2>/dev/null; then
      return 0
    fi
    sleep 0.02
  done
  return 1
}

_bd_lock_release() {
  emulate -L zsh
  rmdir "$BD_STATE_DIR/.lock" 2>/dev/null || true
}

_bd_with_lock() {
  emulate -L zsh
  if _bd_lock_acquire; then
    "$@"
    local status=$?
    _bd_lock_release
    return $status
  fi
  "$@"
}

_bd_get_cursor() {
  emulate -L zsh
  local cursor=0
  if [[ -f "$BD_CURSOR_FILE" ]]; then
    local val
    val=$(<"$BD_CURSOR_FILE")
    if [[ $val == <-> ]]; then
      cursor=$val
    fi
  fi
  print -r -- "$cursor"
}

_bd_set_cursor() {
  emulate -L zsh
  local cursor="$1"
  _bd_write_atomic "$BD_CURSOR_FILE" "$cursor"
}

_bd_set_last_bd() {
  emulate -L zsh
  local delta="$1"
  local token="$2"
  _bd_write_atomic "$BD_LAST_BD_FILE" "$delta"
  _bd_write_atomic "$BD_LAST_BD_ARMED_FILE" "1"
  _bd_write_atomic "$BD_LAST_BD_TOKEN_FILE" "$token"
}

_bd_disarm_last_bd() {
  emulate -L zsh
  _bd_write_atomic "$BD_LAST_BD_ARMED_FILE" "0"
}

_bd_is_last_bd_armed() {
  emulate -L zsh
  local armed=0
  if [[ -f "$BD_LAST_BD_ARMED_FILE" ]]; then
    local val
    val=$(<"$BD_LAST_BD_ARMED_FILE")
    if [[ $val == 1 ]]; then
      armed=1
    fi
  fi
  return $(( 1 - armed ))
}

_bd_get_last_bd_delta() {
  emulate -L zsh
  local delta=0
  if [[ -f "$BD_LAST_BD_FILE" ]]; then
    local val
    val=$(<"$BD_LAST_BD_FILE")
    if [[ $val == <-> ]]; then
      delta=$val
    fi
  fi
  print -r -- "$delta"
}

_bd_get_last_cmd() {
  emulate -L zsh
  if [[ -n ${BD_LAST_CMD-} ]]; then
    print -r -- "$BD_LAST_CMD"
    return 0
  fi
  if [[ -f "$BD_LAST_CMD_FILE" ]]; then
    print -r -- "$(<"$BD_LAST_CMD_FILE")"
    return 0
  fi
  print -r -- ""
}

_bd_set_last_cmd() {
  emulate -L zsh
  local cmd="$1"
  BD_LAST_CMD="$cmd"
  _bd_write_atomic "$BD_LAST_CMD_FILE" "$cmd"
}

_bd_init_state() {
  emulate -L zsh
  _bd_mkdir_state

  local -a history
  local cursor
  history=(${(@f)$(<"$BD_HISTORY_FILE" 2>/dev/null)})

  if (( ${#history} == 0 )); then
    history=("$PWD")
    _bd_write_atomic "$BD_HISTORY_FILE" "${history[@]}"
    _bd_set_cursor 0
  else
    cursor=$(_bd_get_cursor)
    if (( cursor < 0 || cursor >= ${#history} )); then
      _bd_set_cursor $(( ${#history} - 1 ))
    fi
  fi

  if [[ ! -f "$BD_LAST_BD_ARMED_FILE" ]]; then
    _bd_disarm_last_bd
  fi
}

back_directory_chpwd() {
  emulate -L zsh

  if [[ -n ${BD_SUPPRESS_CHWPD-} ]]; then
    unset BD_SUPPRESS_CHWPD
    return 0
  fi

  _bd_mkdir_state

  _bd_with_lock back_directory__record_pwd
}

back_directory__record_pwd() {
  emulate -L zsh
  local -a history
  history=(${(@f)$(<"$BD_HISTORY_FILE" 2>/dev/null)})

  if (( ${#history} == 0 )); then
    history=("$PWD")
  else
    if [[ "${history[-1]}" != "$PWD" ]]; then
      history+=("$PWD")
    fi
  fi

  _bd_write_atomic "$BD_HISTORY_FILE" "${history[@]}"
  _bd_set_cursor $(( ${#history} - 1 ))
}

back_directory_precmd() {
  emulate -L zsh
  local cmd
  cmd=$(fc -ln -1 2>/dev/null)
  cmd=${cmd##[[:space:]]#}
  _bd_mkdir_state
  _bd_set_last_cmd "$cmd"
}

_bd_find_target() {
  emulate -L zsh
  local n="$1"
  local -a history
  local cursor
  history=(${(@f)$(<"$BD_HISTORY_FILE" 2>/dev/null)})
  cursor=$(_bd_get_cursor)

  if (( ${#history} == 0 )); then
    BD_TARGET_DIR=""
    BD_TARGET_INDEX=-1
    BD_TARGET_DELTA=0
    return 1
  fi

  if (( cursor < 0 || cursor >= ${#history} )); then
    cursor=$(( ${#history} - 1 ))
  fi

  local target_index=$(( cursor - n ))
  if (( target_index < 0 )); then
    target_index=0
  fi

  local i
  for (( i = target_index; i >= 0; i-- )); do
    if [[ -d "${history[i+1]}" ]]; then
      BD_TARGET_DIR="${history[i+1]}"
      BD_TARGET_INDEX=$i
      BD_TARGET_DELTA=$(( cursor - i ))
      return 0
    fi
  done

  BD_TARGET_DIR=""
  BD_TARGET_INDEX=-1
  BD_TARGET_DELTA=0
  return 1
}

_bd_go_back() {
  emulate -L zsh
  local n="$1"

  _bd_mkdir_state
  BD_TARGET_DIR=""
  BD_TARGET_INDEX=-1
  BD_TARGET_DELTA=0

  _bd_with_lock _bd_find_target "$n"

  if [[ -z $BD_TARGET_DIR || $BD_TARGET_INDEX -lt 0 || $BD_TARGET_DELTA -le 0 ]]; then
    print -r -- "bd: no earlier directory"
    return 1
  fi

  BD_SUPPRESS_CHWPD=1
  if ! builtin cd -- "$BD_TARGET_DIR"; then
    unset BD_SUPPRESS_CHWPD
    print -r -- "bd: failed to change directory"
    return 1
  fi

  local token="${RANDOM}${RANDOM}"
  _bd_with_lock _bd_set_cursor "$BD_TARGET_INDEX"
  _bd_with_lock _bd_set_last_bd "$BD_TARGET_DELTA" "$token"
}

_bd_cancel() {
  emulate -L zsh
  local last_cmd
  last_cmd=$(_bd_get_last_cmd)

  local -a tokens
  tokens=(${(z)last_cmd})

  if ! _bd_is_last_bd_armed || [[ ${tokens[1]-} != "bd" ]]; then
    _bd_disarm_last_bd
    print -r -- "bd: nothing to cancel"
    return 1
  fi

  local delta
  delta=$(_bd_get_last_bd_delta)
  if (( delta <= 0 )); then
    _bd_disarm_last_bd
    print -r -- "bd: nothing to cancel"
    return 1
  fi

  _bd_mkdir_state

  local -a history
  local cursor
  history=(${(@f)$(<"$BD_HISTORY_FILE" 2>/dev/null)})
  cursor=$(_bd_get_cursor)

  if (( ${#history} == 0 )); then
    _bd_disarm_last_bd
    print -r -- "bd: nothing to cancel"
    return 1
  fi

  local target_index=$(( cursor + delta ))
  if (( target_index >= ${#history} )); then
    target_index=$(( ${#history} - 1 ))
  fi

  local i
  local target_dir=""
  for (( i = target_index; i < ${#history}; i++ )); do
    if [[ -d "${history[i+1]}" ]]; then
      target_dir="${history[i+1]}"
      target_index=$i
      break
    fi
  done

  if [[ -z $target_dir ]]; then
    _bd_disarm_last_bd
    print -r -- "bd: nothing to cancel"
    return 1
  fi

  BD_SUPPRESS_CHWPD=1
  if ! builtin cd -- "$target_dir"; then
    unset BD_SUPPRESS_CHWPD
    _bd_disarm_last_bd
    print -r -- "bd: failed to change directory"
    return 1
  fi

  _bd_with_lock _bd_set_cursor "$target_index"
  _bd_disarm_last_bd
}

bd() {
  emulate -L zsh
  local arg="${1-}"

  if [[ -z $arg ]]; then
    arg=1
  fi

  if [[ $arg == "c" || $arg == "cancel" ]]; then
    _bd_cancel
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

  _bd_go_back "$arg"
}

# Initialize and register hooks
_bd_init_state

if (( ${chpwd_functions[(I)back_directory_chpwd]} == 0 )); then
  chpwd_functions+=(back_directory_chpwd)
fi

if (( ${precmd_functions[(I)back_directory_precmd]} == 0 )); then
  precmd_functions+=(back_directory_precmd)
fi
