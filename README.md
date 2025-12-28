# back-directory (bd)

A zsh wrapper plus a Rust core for fast, correct directory backtracking with a single-step cancel.

## Install

1) Install the Rust core (`bd-core`):

```zsh
# from a local clone
cargo install --path .

# or from git
# cargo install --git https://github.com/01-mu/back-directory
```

Ensure `bd-core` is on your `PATH` (default is `~/.cargo/bin`).

2) Install the zsh wrapper:

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
