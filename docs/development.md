# Developer guide

## How it works

`bd` is the user command provided by the zsh wrapper. The wrapper installs lightweight
hooks, validates arguments, and calls the Rust binary `bd-core`. `bd-core` stores and
queries history in SQLite, computes the target path (including cancel behavior), and
returns it to the wrapper, which then runs `builtin cd`.

We avoid heavy logic in zsh because the original pure-zsh version was slow and unreliable
under frequent directory changes and in multi-shell use. The Rust core centralizes state
management, enforces the `1..999` constraint, and keeps per-session cursor/cancel state.
Each session has its own history and cursor, so `bd` moves by directory-change events
rather than lines of history. The wrapper stays minimal to avoid conflicts with other
shell hooks like auto-`ls`.

## Development / CI

CI runs on pull requests and pushes to `main`. The recommended local check is:

```zsh
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release
```

To keep `main` healthy, enable branch protection and require the `ci / build-test` job
to pass before merging.

## Notes

- State lives in `~/.local/state/back-directory/bd.sqlite3` (or `$XDG_STATE_HOME`).
- History is isolated per session; each session has its own cursor and cancel state.
- Session keys default to TTY + shell PID, so each shell is its own session unless
  overridden via `BD_SESSION_ID`.
- Directory changes are captured via `chpwd`; no `cd` wrapper or per-prompt writes.
