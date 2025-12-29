#!/bin/sh
set -eu

REPO="01-mu/back-directory"
BIN_NAME="bd-core"
INSTALL_DIR="$HOME/.local/bin"
WRAPPER_DEST="$HOME/.bd.zsh"
ZSHRC="${ZSHRC:-$HOME/.zshrc}"

uname_s="$(uname -s)"
uname_m="$(uname -m)"

case "$uname_s" in
  Darwin)
    os="apple-darwin"
    ;;
  Linux)
    os="unknown-linux-gnu"
    ;;
  *)
    echo "Unsupported OS: $uname_s" >&2
    exit 1
    ;;
 esac

case "$uname_m" in
  arm64|aarch64)
    arch="aarch64"
    ;;
  x86_64|amd64)
    arch="x86_64"
    ;;
  *)
    echo "Unsupported architecture: $uname_m" >&2
    exit 1
    ;;
 esac

target="$arch-$os"
archive="$BIN_NAME-$target.tar.gz"
url="https://github.com/$REPO/releases/latest/download/$archive"
wrapper_url="https://raw.githubusercontent.com/$REPO/main/scripts/bd.zsh"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$INSTALL_DIR"

curl -fsSL "$url" -o "$tmpdir/$archive"

tar -xzf "$tmpdir/$archive" -C "$tmpdir"

if [ ! -f "$tmpdir/$BIN_NAME" ]; then
  echo "Archive did not contain $BIN_NAME" >&2
  exit 1
fi

install -m 0755 "$tmpdir/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"

curl -fsSL "$wrapper_url" -o "$WRAPPER_DEST"

if [ -f "$ZSHRC" ]; then
  if ! grep -q 'source ~/.bd.zsh' "$ZSHRC"; then
    printf '\nsource ~/.bd.zsh\n' >> "$ZSHRC"
  fi
else
  printf 'source ~/.bd.zsh\n' >> "$ZSHRC"
fi

echo "Installed core binary to $INSTALL_DIR/$BIN_NAME"
echo "Installed wrapper to $WRAPPER_DEST"
echo "Added wrapper to $ZSHRC (start a new shell or source it)"
