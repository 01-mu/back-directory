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

## SQLite schema

The local state database is created on first use.

### events

Directory-change events captured by the shell hooks.

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| id | INTEGER | PK, AUTOINCREMENT | Monotonic event id. |
| session_key | TEXT | NOT NULL, DEFAULT '' | Session identifier (TTY+PID by default). |
| path | TEXT | NOT NULL | Absolute path after the directory change. |
| ts | INTEGER | NOT NULL | Unix timestamp (seconds). |

Indexes:

- `idx_events_session_id` on `(session_key, id)`
- `idx_events_ts` on `(ts)`

### sessions

Per-session cursor and last `bd` move state.

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| session_key | TEXT | PK | Session identifier. |
| cursor_id | INTEGER | NOT NULL | Current event id cursor in `events`. |
| last_bd_delta | INTEGER | NOT NULL, DEFAULT 0 | Last `bd N` delta used for cancel. |
| last_bd_from_id | INTEGER | NOT NULL, DEFAULT 0 | Event id before the last `bd` move. |
| last_bd_to_id | INTEGER | NOT NULL, DEFAULT 0 | Event id after the last `bd` move. |
| last_bd_armed | INTEGER | NOT NULL, DEFAULT 0 | Cancel toggle (0/1). |
| last_seen_at | INTEGER | NOT NULL, DEFAULT 0 | Last activity timestamp (seconds). |

### undo_moves

Stack of cancelable moves.

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| id | INTEGER | PK, AUTOINCREMENT | Monotonic undo id. |
| session_key | TEXT | NOT NULL | Session identifier. |
| from_id | INTEGER | NOT NULL | Event id before the move. |
| to_id | INTEGER | NOT NULL | Event id after the move. |
| created_at | INTEGER | NOT NULL, DEFAULT 0 | Creation timestamp (seconds). |

Indexes:

- `idx_undo_moves_session_id` on `(session_key, id)`

### meta

Lightweight key/value store for maintenance metadata.

| Column | Type | Constraints | Description |
| --- | --- | --- | --- |
| key | TEXT | PK | Metadata key name. |
| value | INTEGER | NOT NULL | UNIX timestamp or numeric value. |

Current keys:

- `last_cleanup_at`: UNIX timestamp of the last cleanup run.

### Data cleanup

To prevent unbounded growth, the `events` table is rotated per session:

- When a session has 10,000+ events, the 10,000th newest event id becomes the rotation cutoff.
- Events older than the cutoff are deleted, but never past any id still referenced by
  `sessions` (`cursor_id`, `last_bd_from_id`, `last_bd_to_id`) or `undo_moves` (`from_id`, `to_id`).
- If there is no cursor/undo state yet, or the session has fewer than 10,000 events,
  no deletion occurs.

Additional retention cleanup runs about every 10 days:

- `sessions`: delete rows with `last_seen_at` older than 180 days (excluding the current session).
- `undo_moves`: delete rows with `created_at` older than 90 days.
- Cleanup scheduling uses `meta.last_cleanup_at`.


## Notes

- State lives in `~/.local/state/back-directory/bd.sqlite3` (or `$XDG_STATE_HOME`).
- History is isolated per session; each session has its own cursor and cancel state.
- Session keys default to TTY + shell PID, so each shell is its own session unless
  overridden via `BD_SESSION_ID`.
- Directory changes are captured via `PROMPT_COMMAND` (bash) or `chpwd` (zsh); no `cd` wrapper or per-prompt writes.
