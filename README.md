# back-directory (bd)

A bash/zsh wrapper plus a Rust core for fast, correct directory backtracking with a single-step cancel.

## Getting started

![back-directory demo](examples/bd-demo/media/bd-demo.webp)

## Usage

```sh
bd       # same as: bd 1
bd 3     # go back 3 directories (1 <= N <= 999)
bd c     # cancel the last bd command in the current session
bd ls    # list recent targets with their N values
bd ls 5  # list 5 recent targets
```

`bd ls` numbers match the `N` you pass to `bd`. `bd c` repeats to undo multiple `bd`
commands, but any other directory move clears that undo history.

## Install

#### Recommended: one-liner (GitHub Releases)

```sh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/install.sh | sh
```

This installs the core binary to `${XDG_BIN_HOME:-$HOME/.local/bin}`, wrappers to
`${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.bash` and
`${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh`, and adds `source` lines to
`.bashrc` / `.zshrc`. If `${XDG_BIN_HOME:-$HOME/.local/bin}` is not on `PATH`, add:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

#### Manual download (Releases)

1) Download the matching `.tar.gz` for your OS/arch from the latest GitHub Release:
   - `bd-core-aarch64-apple-darwin.tar.gz`
   - `bd-core-x86_64-apple-darwin.tar.gz`
   - `bd-core-x86_64-unknown-linux-gnu.tar.gz`

2) Extract and install:

```sh
mkdir -p ~/.local/bin
tar -xzf bd-core-<target>.tar.gz
mv bd-core ~/.local/bin/bd-core
```

#### Wrapper configuration

Start a new shell or `source ~/.bashrc` / `source ~/.zshrc`.

If the core binary lives elsewhere, set `BD_CORE_BIN` before sourcing:

```sh
export BD_CORE_BIN=/path/to/bd-core
```

## Uninstall

Uninstall is a manual cleanup of files and shell config changes.

1) Locate the installed binary (the path will be used below):

```sh
command -v bd
# or
which bd
```

2) Remove the binary at the path printed above (examples):

```sh
rm -f ~/.local/bin/bd
# or
sudo rm -f /usr/local/bin/bd
```

3) Remove optional config/state/data if present (only if you created them):

```sh
rm -rf ~/.config/back-directory
rm -rf ~/.local/share/back-directory
rm -rf ~/.cache/back-directory
```

4) Revert shell setup changes:
   - Remove any PATH, alias, or `source .../bd.bash` / `source .../bd.zsh` lines you added to `.bashrc` or `.zshrc`,
     then restart your shell.

## Layout

- scripts/: distribution scripts (install.sh, bd.bash, bd.zsh)
- src/: Rust implementation (bd-core)

## Developer guide

See `docs/development.md` for development setup, local debugging, and implementation
details.
