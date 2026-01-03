#!/bin/sh
set -eu

REPO="01-mu/back-directory"
BIN_NAME="bd-core"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
CFG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/back-directory"
ZSH_FILE="$CFG_DIR/bd.zsh"
ZSHRC="${ZSHRC:-$HOME/.zshrc}"
BASH_FILE="$CFG_DIR/bd.bash"
BASHRC="${BASHRC:-$HOME/.bashrc}"
LEGACY_WRAPPER="$HOME/.bd.zsh"

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
wrapper_bash_url="https://raw.githubusercontent.com/$REPO/main/scripts/bd.bash"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$BIN_DIR"
mkdir -p "$CFG_DIR"

curl -fsSL "$url" -o "$tmpdir/$archive"

tar -xzf "$tmpdir/$archive" -C "$tmpdir"

if [ ! -f "$tmpdir/$BIN_NAME" ]; then
  echo "Archive did not contain $BIN_NAME" >&2
  exit 1
fi

install -m 0755 "$tmpdir/$BIN_NAME" "$BIN_DIR/$BIN_NAME"

curl -fsSL "$wrapper_url" -o "$ZSH_FILE"
curl -fsSL "$wrapper_bash_url" -o "$BASH_FILE"

if [ -f "$LEGACY_WRAPPER" ]; then
  if grep -q "back-directory (bd) - zsh wrapper" "$LEGACY_WRAPPER"; then
    cat > "$LEGACY_WRAPPER" <<'EOF'
# back-directory (bd) - legacy shim
source "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh"
EOF
    echo "Updated legacy wrapper at $LEGACY_WRAPPER"
  else
    echo "Found existing $LEGACY_WRAPPER; leaving it untouched."
  fi
fi

canonical_source='source "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh"'
canonical_bash_source='source "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.bash"'

if [ -f "$ZSHRC" ]; then
  if ! grep -Eq 'back-directory/bd\.zsh' "$ZSHRC"; then
    # Avoid duplicate sourcing when rerunning the installer.
    printf '\n%s\n' "$canonical_source" >> "$ZSHRC"
  fi
else
  printf '%s\n' "$canonical_source" >> "$ZSHRC"
fi

if [ -f "$BASHRC" ]; then
  if ! grep -Eq 'back-directory/bd\.bash' "$BASHRC"; then
    printf '\n%s\n' "$canonical_bash_source" >> "$BASHRC"
  fi
else
  printf '%s\n' "$canonical_bash_source" >> "$BASHRC"
fi

echo "Installed core binary to $BIN_DIR/$BIN_NAME"
echo "Installed wrappers to $ZSH_FILE and $BASH_FILE"
echo "Added wrapper to $ZSHRC and $BASHRC (start a new shell or source it)"
