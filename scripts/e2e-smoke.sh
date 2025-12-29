#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN="${BD_CORE_BIN:-$ROOT_DIR/target/release/bd-core}"

if [ ! -x "$BIN" ]; then
  echo "bd-core binary not found at $BIN" >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$tmpdir/a" "$tmpdir/b"

zsh -c '
  set -eu
  export BD_CORE_BIN="$1"
  export BD_SESSION_ID="e2e"
  source "$2/scripts/bd.zsh"

  cd "$3/a"
  cd "$3/b"

  bd 1
  if [ "$PWD" != "$3/a" ]; then
    echo "expected back to a, got $PWD" >&2
    exit 1
  fi

  bd c
  if [ "$PWD" != "$3/b" ]; then
    echo "expected cancel back to b, got $PWD" >&2
    exit 1
  fi
' zsh "$BIN" "$ROOT_DIR" "$tmpdir"
