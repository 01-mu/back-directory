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
bd doctor # show database status
```

`bd ls` numbers match the `N` you pass to `bd`. `bd c` repeats to undo multiple `bd`
commands, but any other directory move clears that undo history.

## Install

#### Recommended: install.sh

1) Download `install.sh` from this repository.

2) Run it:

```sh
sh ./install.sh
```

This installs the core binary to `${XDG_BIN_HOME:-$HOME/.local/bin}`, wrappers to
`${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.bash` and
`${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh`, and adds `source` lines to
`.bashrc` / `.zshrc`. Start a new shell or `source ~/.bashrc` / `source ~/.zshrc`.
If `${XDG_BIN_HOME:-$HOME/.local/bin}` is not on `PATH`, add:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

#### Manual download

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

3) Install the wrapper scripts:

```sh
mkdir -p ~/.config/back-directory
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/bd.bash \
  -o ~/.config/back-directory/bd.bash
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/bd.zsh \
  -o ~/.config/back-directory/bd.zsh
```

4) Add `source` lines to your shell rc, then start a new shell or `source ~/.bashrc` / `source ~/.zshrc`.

If the core binary lives elsewhere, set `BD_CORE_BIN` before sourcing:

```sh
export BD_CORE_BIN=/path/to/bd-core
```

## Data retention

- State is stored in `~/.local/state/back-directory/bd.sqlite3` (or `$XDG_STATE_HOME`).
- `events` is rotated per session at 10,000 entries.
- `sessions` rows are kept for 180 days since last seen.
- `undo_moves` rows are kept for 90 days since created.
- Cleanup runs about every 10 days.
- vacuum is manual only. Removing the SQLite file resets all history.

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

## Docs

- `docs/development.md`: development setup and internals
- `docs/dataflow.md`: SQLite dataflow and cleanup lifecycle
- `docs/maintenance.md`: vacuum and doctor best practices

## Developer guide

See `docs/development.md` for development setup, local debugging, and implementation
details.
