# back-directory (bd)

A zsh wrapper plus a Rust core for fast, correct directory backtracking with a single-step cancel.

## Install

### Recommended: one-liner (GitHub Releases)

```zsh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/install.sh | sh
```

This installs `bd-core` to `~/.local/bin/bd-core` (creates `~/.local/bin` if needed).
If `~/.local/bin` is not on your `PATH`, add this to your shell config:

```zsh
export PATH="$HOME/.local/bin:$PATH"
```

### Manual: download from Releases

1) Download the matching `.tar.gz` for your OS/arch from the latest GitHub Release:
   - `bd-core-aarch64-apple-darwin.tar.gz`
   - `bd-core-x86_64-apple-darwin.tar.gz`
   - `bd-core-x86_64-unknown-linux-gnu.tar.gz`

2) Extract and install:

```zsh
mkdir -p ~/.local/bin
tar -xzf bd-core-<target>.tar.gz
mv bd-core ~/.local/bin/bd-core
```

### Developer install (cargo)

```zsh
# from a local clone
cargo install --path .

# or from git
# cargo install --git https://github.com/01-mu/back-directory
```

Ensure `bd-core` is on your `PATH` (default is `~/.cargo/bin`).

### Install the zsh wrapper

After `bd-core` is installed, add the wrapper:

```zsh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/bd.zsh -o ~/.bd.zsh

echo 'source ~/.bd.zsh' >> ~/.zshrc
```

Start a new shell or `source ~/.zshrc`.

If `bd-core` lives elsewhere, set `BD_CORE_BIN` before sourcing:

```zsh
export BD_CORE_BIN=/path/to/bd-core
```

## Usage

```zsh
bd       # same as: bd 1
bd 3     # go back 3 directories (1 <= N <= 99)
bd c     # cancel the last bd command in the current session
```

Optional alias:

```zsh
bd cancel
```

## Notes

- State lives in `~/.local/state/back-directory/bd.sqlite3` (or `$XDG_STATE_HOME`).
- History is shared across shells, but each session has its own cursor and cancel state.
- Directory changes are captured via `chpwd`; no `cd` wrapper or per-prompt writes.
