# back-directory (bd)

A bash/zsh wrapper with a Rust core that records directory navigation history in SQLite to support correct directory backtracking.

Early-stage note: behaviors may change between releases. Currently supported shells: bash, zsh.

## Getting started

![back-directory demo](examples/bd-demo/media/bd-demo.webp)

## Usage

```sh
bd       # same as: bd 1
bd 3     # go back 3 directories (1 <= N <= 999)
bd c     # cancel the last bd command in the current session
bd ls    # list recent targets (default: 10)
bd ls 5  # list 5 recent targets (1 <= N <= 999)
bd doctor # show database status
bd optimize # reclaim SQLite DB space (can be slow)
bd vacuum # reset SQLite DB (deletes all history)
bd h     # show help
```

`bd ls` numbers match the `N` you pass to `bd`. `bd c` repeats to undo multiple `bd`
commands, but any other directory move clears that undo history.

Warning: `bd vacuum` deletes all history. Use with care.

## Install

#### Recommended: install.sh

1) Run the installer directly from GitHub:

```sh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/install.sh | sh
```

If you prefer to inspect the script first:

```sh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/install.sh -o install.sh
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

#### What changes on your machine

- Downloads the release asset `bd-core-<target>.tar.gz` and wrapper scripts from `scripts/bd.bash` and `scripts/bd.zsh`.
- Places the core binary at `${XDG_BIN_HOME:-$HOME/.local/bin}/bd-core`.
- Writes wrapper files to `${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.bash` and `bd.zsh`.
- Appends a `source "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.*"` line to `$BASHRC` and `$ZSHRC`
  (defaults to `~/.bashrc` and `~/.zshrc`; creates the files if missing).
- To opt out, use the manual install steps below and add the `source` line yourself.

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

Checksums (planned): future releases will publish SHA256 sums for verification.

## Data and privacy

`bd` stores local state in SQLite to support session-scoped backtracking and undo:

- Data stored: directory paths, timestamps, session id, and undo/cancel bookkeeping (from/to ids).
- Location: `${XDG_STATE_HOME:-$HOME/.local/state}/back-directory/bd.sqlite3`
  (SQLite may also create `bd.sqlite3-wal` and `bd.sqlite3-shm`).
- No external telemetry or network calls from the core binary.

## Uninstall

Uninstall is a manual cleanup of files and shell config changes.

1) Locate the installed core binary (the path will be used below):

```sh
command -v bd-core
# or
which bd-core
```

2) Remove the core binary at the path printed above (examples):

```sh
rm -f ~/.local/bin/bd-core
# or
sudo rm -f /usr/local/bin/bd-core
```

3) Remove local state/database files (if present):

```sh
rm -f "${XDG_STATE_HOME:-$HOME/.local/state}/back-directory/bd.sqlite3"
rm -f "${XDG_STATE_HOME:-$HOME/.local/state}/back-directory/bd.sqlite3-wal"
rm -f "${XDG_STATE_HOME:-$HOME/.local/state}/back-directory/bd.sqlite3-shm"
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory"
```

4) Revert shell setup changes:
   - Remove any PATH, alias, or `source .../bd.bash` / `source .../bd.zsh` lines you added to `.bashrc` or `.zshrc`,
     then restart your shell.

Disable temporarily: comment out the `source .../bd.*` line in your shell rc file and reload your shell.

## Why bd (vs `cd -`, `pushd` / `popd`)

`bd` is not a replacement for `cd -` or `pushd` / `popd`. It focuses on two things:
**session-scoped backtracking** and **safe single-step cancel**.

- `cd -`: only toggles the last directory; no real history to walk
- `pushd` / `popd`: requires manual stack management; not as natural for quick backtracking
- `bd`: keeps session history so `bd N` can jump back any number of steps, and `bd c` safely
  cancels just the last `bd` move

In short, `bd` complements existing commands by making session history navigable and making
backtracking reversible.

## Layout

- scripts/: distribution scripts (install.sh, bd.bash, bd.zsh)
- src/: Rust implementation (bd-core)

## Docs

- `docs/development.md`: development setup and internals
- `docs/dataflow.md`: SQLite dataflow and cleanup lifecycle
- `docs/maintenance.md`: optimize and doctor best practices
