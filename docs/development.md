# Developer guide

## How it works

`bd` is the user command provided by the shell wrappers (bash/zsh). The wrapper installs
lightweight hooks, validates arguments, and calls the Rust binary `bd-core`. `bd-core`
stores and queries history in SQLite, computes the target path (including cancel
behavior), and returns it to the wrapper, which then runs `builtin cd`.

We avoid heavy logic in zsh because the original pure-zsh version was slow and unreliable
under frequent directory changes and in multi-shell use. The Rust core centralizes state
management, enforces the `1..999` constraint, and keeps per-session cursor/cancel state.
Each session has its own history and cursor, so `bd` moves by directory-change events
rather than lines of history. The wrapper stays minimal to avoid conflicts with other
shell hooks like auto-`ls`. bash uses `PROMPT_COMMAND`, while zsh uses `chpwd` to detect
directory changes.

## Local setup

### Install (cargo)

```sh
# from a local clone
cargo install --path .

# or from git
# cargo install --git https://github.com/01-mu/back-directory
```

Ensure `bd-core` is on your `PATH` (default is `~/.cargo/bin`).

### Debugging from a local clone

If you want to run the wrapper from this repo while iterating on the core:

```sh
# from a local clone
cargo install --path . --force

# use the wrapper from this repo
mkdir -p ~/.config/back-directory
cp ./scripts/bd.bash ~/.config/back-directory/bd.bash
# bash
# source ~/.config/back-directory/bd.bash
cp ./scripts/bd.zsh ~/.config/back-directory/bd.zsh
# zsh
source ~/.config/back-directory/bd.zsh
```

If you prefer not to copy the wrapper, you can source it directly:

```sh
export BD_CORE_BIN="$HOME/.cargo/bin/bd-core"
source /path/to/your/clone/scripts/bd.bash
# or for zsh:
# source /path/to/your/clone/scripts/bd.zsh
```

## Development / CI

CI runs on pull requests and pushes to `main`. The recommended local check is:

```sh
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release
```

To keep `main` healthy, enable branch protection and require the `ci / build-test` job
to pass before merging.

## Notes

- State lives in `~/.local/state/back-directory/bd.sqlite3` (or `$XDG_STATE_HOME`).
- History is isolated per session; each session has its own cursor and cancel state.
- Session keys default to TTY + shell PID, so each shell is its own session unless
  overridden via `BD_SESSION_ID`.
- Directory changes are captured via `PROMPT_COMMAND` (bash) or `chpwd` (zsh); no `cd` wrapper or per-prompt writes.
