# Maintenance: vacuum and doctor

This note documents best practices for database maintenance.

## Principles

- `vacuum` is manual only. Do not run it automatically.
- `doctor` is read-only by default; it should not change user data.
- Prefer predictable, low-impact checks for daily use.

## vacuum best practices

- Run when no other shells are using `bd` to avoid write locks.
- In WAL mode, run a checkpoint before vacuum:

```sh
sqlite3 ~/.local/state/back-directory/bd.sqlite3 "PRAGMA wal_checkpoint(TRUNCATE);"
```


## doctor best practices

- Default to quick checks (counts, size, freelist, last cleanup).
- Provide `--full` to include `PRAGMA integrity_check;`.
- Use `bd doctor --json` for machine-readable output.
- Surface clear OK/WARN thresholds, for example:
  - `freelist_count / page_count >= 0.2` → suggest vacuum.
  - `last_cleanup_at` older than 30 days → suggest cleanup check.

## Notes

- WAL can create side files (`-wal`, `-shm`); this is expected.
- Removing `bd.sqlite3` resets all history and metadata.
