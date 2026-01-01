# Dataflow and Cleanup (SQLite)

This document describes how data accumulates in the SQLite DB and how cleanup runs,
using simple diagrams and sample transitions.

## Overview: tables and purpose

```
events      : directory-change history (per session, rotated)
sessions    : per-session cursor and last bd state
undo_moves  : cancel stack (per session)
meta        : last_cleanup_at
```

## Dataflow: record, back, cancel

### 1) `bd record` (directory changed)

```
events:      + (session_key, path, ts)
sessions:    upsert cursor_id + reset last_bd_* + last_seen_at
undo_moves:  delete all rows for this session
```

Sample (session = "S1"):

```
events
+----+---------+---------+---------+
| id | session | path    | ts      |
+----+---------+---------+---------+
|  1 | S1      | /work   | 1000    |
+----+---------+---------+---------+

sessions
+---------+-----------+--------------+--------------+------------+-------------+-------------+
| session | cursor_id | last_bd_delta| last_bd_from | last_bd_to | last_bd_armed| last_seen_at|
+---------+-----------+--------------+--------------+------------+-------------+-------------+
| S1      | 1         | 0            | 0            | 0          | 0           | 1000        |
+---------+-----------+--------------+--------------+------------+-------------+-------------+

undo_moves
+----+---------+---------+-------+------------+
| id | session | from_id | to_id | created_at |
+----+---------+---------+-------+------------+
| (cleared for S1 on record)                  |
+---------------------------------------------+
```

### 2) `bd 3` (move back)

```
sessions:   cursor_id moves to target_id, last_bd_* set, last_seen_at updated
undo_moves: + (session_key, from_id, to_id, created_at)
events:     unchanged
```

Sample (cursor moves from id=8 to id=5):

```
sessions (S1)
cursor_id: 8 -> 5
last_bd_delta: 3
last_bd_from_id: 8
last_bd_to_id: 5
last_bd_armed: 1

undo_moves
+----+---------+---------+-------+------------+
| id | session | from_id | to_id | created_at |
+----+---------+---------+-------+------------+
|  7 | S1      | 8       | 5     | 2000       |
+----+---------+---------+-------+------------+
```

### 3) `bd c` (cancel)

```
sessions:   cursor_id restored to last_bd_from_id, last_bd_* reset, last_seen_at updated
undo_moves: delete the latest row (stack pop)
events:     unchanged
```

## Cleanup cycle

Cleanup runs **about once every 10 days**. The run is skipped if the last cleanup
is more recent than 10 days (tracked in `meta.last_cleanup_at`).

```
if now - last_cleanup_at >= 10 days:
  delete sessions where last_seen_at < now - 180 days (except current session)
  delete undo_moves where created_at < now - 90 days
  update last_cleanup_at
```

## Events rotation (per session)

To cap growth, `events` is rotated per session:

```
if events(session) >= 10,000:
  cutoff_id = 10,000th newest id
  delete events older than min(cutoff_id, any referenced id in sessions/undo_moves)
```

This ensures `events` rows still referenced by the current session state are preserved.

## Notes

- Cleanup does not run on every command; it runs only when the 10-day interval is exceeded.
- VACUUM is manual. Deleting `bd.sqlite3` resets all history and metadata.
