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

## How it works

`bd` is the user command provided by the zsh wrapper. The wrapper installs lightweight
hooks, validates arguments, and calls the Rust binary `bd-core`. `bd-core` stores and
queries history in SQLite, computes the target path (including cancel behavior), and
returns it to the wrapper, which then runs `builtin cd`.

We avoid heavy logic in zsh because the original pure-zsh version was slow and unreliable
under frequent directory changes and in multi-shell use. The Rust core centralizes state
management, enforces the `1..99` constraint, and keeps per-session cursor/cancel state
while sharing history across shells. History is shared, but each shell keeps its own
cursor so `bd` moves by directory-change events rather than lines of history. The wrapper
stays minimal to avoid conflicts with other shell hooks like auto-`ls`.

## Development / CI

CI runs on pull requests and pushes to `main`. The recommended local check is:

```zsh
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release
```

To keep `main` healthy, enable branch protection and require the `ci / build-test` job
to pass before merging.

## Notes

- State lives in `~/.local/state/back-directory/bd.sqlite3` (or `$XDG_STATE_HOME`).
- History is shared across shells, but each session has its own cursor and cancel state.
- Directory changes are captured via `chpwd`; no `cd` wrapper or per-prompt writes.
