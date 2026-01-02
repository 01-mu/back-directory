use clap::{Parser, Subcommand};
use rusqlite::{params, Connection, OptionalExtension};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const BD_MAX_BACK: u32 = 999;
const BD_DEFAULT_LIST: u32 = 10;
const CLEANUP_INTERVAL_SECS: i64 = 10 * 24 * 60 * 60; //  10 days
const SESSION_RETENTION_SECS: i64 = 180 * 24 * 60 * 60; // 180 days
const UNDO_RETENTION_SECS: i64 = 90 * 24 * 60 * 60; //  90 days
const META_LAST_CLEANUP_KEY: &str = "last_cleanup_at";

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Record {
        #[arg(long)]
        session: String,
        #[arg(long)]
        pwd: String,
    },
    Back {
        #[arg(long)]
        session: String,
        #[arg(long)]
        n: u32,
        #[arg(long)]
        print_path: bool,
    },
    List {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = BD_DEFAULT_LIST)]
        limit: u32,
    },
    Cancel {
        #[arg(long)]
        session: String,
    },
    Doctor {
        #[arg(long)]
        full: bool,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Record { session, pwd } => cmd_record(&session, &pwd),
        Commands::Back {
            session,
            n,
            print_path,
        } => cmd_back(&session, n, print_path),
        Commands::List { session, limit } => cmd_list(&session, limit),
        Commands::Cancel { session } => cmd_cancel(&session),
        Commands::Doctor { full, json } => cmd_doctor(full, json),
    };

    if let Err(msg) = result {
        eprintln!("{msg}");
        std::process::exit(1);
    }
}

fn cmd_record(session: &str, pwd: &str) -> Result<(), String> {
    let path = Path::new(pwd);
    if !path.is_dir() {
        return Err("bd: pwd is not a directory".to_string());
    }

    let mut conn = open_db()?;
    maybe_run_cleanup(&mut conn, session)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let last_path: Option<String> = tx
        .query_row(
            "SELECT path FROM events WHERE session_key = ?1 ORDER BY id DESC LIMIT 1",
            params![session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let now = current_ts();
    let mut latest_id = None;
    if last_path.as_deref() != Some(pwd) {
        tx.execute(
            "INSERT INTO events (session_key, path, ts) VALUES (?1, ?2, ?3)",
            params![session, pwd, now],
        )
        .map_err(|e| format!("bd: db error: {e}"))?;
        latest_id = Some(tx.last_insert_rowid());
    }

    if latest_id.is_none() {
        latest_id = tx
            .query_row(
                "SELECT id FROM events WHERE session_key = ?1 ORDER BY id DESC LIMIT 1",
                params![session],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("bd: db error: {e}"))?;
    }

    let latest_id = match latest_id {
        Some(id) => id,
        None => {
            tx.execute(
                "INSERT INTO events (session_key, path, ts) VALUES (?1, ?2, ?3)",
                params![session, pwd, now],
            )
            .map_err(|e| format!("bd: db error: {e}"))?;
            tx.last_insert_rowid()
        }
    };

    tx.execute(
        "INSERT INTO sessions (session_key, cursor_id, last_bd_delta, last_bd_from_id, last_bd_to_id, last_bd_armed, last_seen_at)
         VALUES (?1, ?2, 0, 0, 0, 0, ?3)
         ON CONFLICT(session_key) DO UPDATE SET
           cursor_id = excluded.cursor_id,
           last_bd_delta = 0,
           last_bd_from_id = 0,
           last_bd_to_id = 0,
           last_bd_armed = 0,
           last_seen_at = excluded.last_seen_at",
        params![session, latest_id, now],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.execute(
        "DELETE FROM undo_moves WHERE session_key = ?1",
        params![session],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    rotate_events(&tx, session)?;
    tx.commit().map_err(|e| format!("bd: db error: {e}"))?;
    Ok(())
}

fn cmd_back(session: &str, n: u32, _print_path: bool) -> Result<(), String> {
    if n == 0 {
        return Err("bd: usage: bd [N|c|ls]".to_string());
    }
    if n > BD_MAX_BACK {
        return Err(format!("bd: max is {BD_MAX_BACK}"));
    }

    let mut conn = open_db()?;
    maybe_run_cleanup(&mut conn, session)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let latest_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM events WHERE session_key = ?1 ORDER BY id DESC LIMIT 1",
            params![session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut cursor_id: i64 = match tx
        .query_row(
            "SELECT cursor_id FROM sessions WHERE session_key = ?1",
            params![session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?
    {
        Some(id) => id,
        None => latest_id.ok_or_else(|| "bd: no earlier directory".to_string())?,
    };

    let cursor_exists: Option<i64> = tx
        .query_row(
            "SELECT id FROM events WHERE id = ?1 AND session_key = ?2",
            params![cursor_id, session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    if cursor_exists.is_none() {
        cursor_id = latest_id.ok_or_else(|| "bd: no earlier directory".to_string())?;
    }

    let (target_id, target_path, actual_steps) = {
        let mut stmt = tx
            .prepare(
                "SELECT id, path FROM events WHERE session_key = ?1 AND id < ?2 ORDER BY id DESC",
            )
            .map_err(|e| format!("bd: db error: {e}"))?;

        let mut rows = stmt
            .query(params![session, cursor_id])
            .map_err(|e| format!("bd: db error: {e}"))?;

        let mut steps: u32 = 0;
        let mut target: Option<(i64, String, u32)> = None;
        let mut oldest_existing: Option<(i64, String, u32)> = None;

        while let Some(row) = rows.next().map_err(|e| format!("bd: db error: {e}"))? {
            let id: i64 = row.get(0).map_err(|e| format!("bd: db error: {e}"))?;
            let path: String = row.get(1).map_err(|e| format!("bd: db error: {e}"))?;
            steps += 1;

            if Path::new(&path).is_dir() {
                oldest_existing = Some((id, path.clone(), steps));
                if steps >= n {
                    target = Some((id, path, steps));
                    break;
                }
            }
        }

        match target.or(oldest_existing) {
            Some(value) => value,
            None => return Err("bd: no earlier directory".to_string()),
        }
    };

    let now = current_ts();
    tx.execute(
        "INSERT INTO sessions (session_key, cursor_id, last_bd_delta, last_bd_from_id, last_bd_to_id, last_bd_armed, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
         ON CONFLICT(session_key) DO UPDATE SET
           cursor_id = excluded.cursor_id,
           last_bd_delta = excluded.last_bd_delta,
           last_bd_from_id = excluded.last_bd_from_id,
           last_bd_to_id = excluded.last_bd_to_id,
           last_bd_armed = 1,
           last_seen_at = excluded.last_seen_at",
        params![session, target_id, actual_steps, cursor_id, target_id, now],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.execute(
        "INSERT INTO undo_moves (session_key, from_id, to_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![session, cursor_id, target_id, now],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.commit().map_err(|e| format!("bd: db error: {e}"))?;

    println!("{target_path}");
    Ok(())
}

fn cmd_list(session: &str, limit: u32) -> Result<(), String> {
    if limit == 0 {
        return Err("bd: usage: bd ls [N]".to_string());
    }
    if limit > BD_MAX_BACK {
        return Err(format!("bd: max is {BD_MAX_BACK}"));
    }

    let mut conn = open_db()?;
    maybe_run_cleanup(&mut conn, session)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let latest_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM events WHERE session_key = ?1 ORDER BY id DESC LIMIT 1",
            params![session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut cursor_id: i64 = match tx
        .query_row(
            "SELECT cursor_id FROM sessions WHERE session_key = ?1",
            params![session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?
    {
        Some(id) => id,
        None => latest_id.ok_or_else(|| "bd: no history in this session".to_string())?,
    };

    let cursor_exists: Option<i64> = tx
        .query_row(
            "SELECT id FROM events WHERE id = ?1 AND session_key = ?2",
            params![cursor_id, session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    if cursor_exists.is_none() {
        cursor_id = latest_id.ok_or_else(|| "bd: no history in this session".to_string())?;
    }

    let now = current_ts();
    tx.execute(
        "UPDATE sessions SET last_seen_at = ?1 WHERE session_key = ?2",
        params![now, session],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    let mut stmt = tx
        .prepare("SELECT id, path FROM events WHERE session_key = ?1 AND id < ?2 ORDER BY id DESC")
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut rows = stmt
        .query(params![session, cursor_id])
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut steps: u32 = 0;
    let mut printed = 0;
    let mut lines: Vec<(u32, String)> = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("bd: db error: {e}"))? {
        let _id: i64 = row.get(0).map_err(|e| format!("bd: db error: {e}"))?;
        let path: String = row.get(1).map_err(|e| format!("bd: db error: {e}"))?;
        steps += 1;
        if Path::new(&path).is_dir() {
            lines.push((steps, path));
            printed += 1;
            if printed >= limit {
                break;
            }
        }
    }

    if printed == 0 {
        return Err("bd: no history in this session".to_string());
    }

    let max_step = lines.iter().map(|(step, _)| *step).max().unwrap_or(0);
    let width = max_step.to_string().len();
    let home_raw = std::env::var("HOME").unwrap_or_default();
    let home = if home_raw.ends_with('/') && home_raw.len() > 1 {
        home_raw.trim_end_matches('/').to_string()
    } else {
        home_raw
    };
    for (step, path) in lines.into_iter().rev() {
        let display_path = if !home.is_empty() && path == home {
            "~".to_string()
        } else if !home.is_empty() && path.starts_with(&(home.clone() + "/")) {
            format!("~{}", &path[home.len()..])
        } else {
            path
        };
        println!("[{:>width$}] {}", step, display_path, width = width);
    }

    Ok(())
}

fn cmd_cancel(session: &str) -> Result<(), String> {
    let mut conn = open_db()?;
    maybe_run_cleanup(&mut conn, session)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let row: Option<(i64, i64)> = tx
        .query_row(
            "SELECT id, from_id FROM undo_moves WHERE session_key = ?1 ORDER BY id DESC LIMIT 1",
            params![session],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let (undo_id, last_bd_from_id) = match row {
        Some(value) => value,
        None => return Err("bd: nothing to cancel".to_string()),
    };

    let target_path: Option<String> = tx
        .query_row(
            "SELECT path FROM events WHERE id = ?1 AND session_key = ?2",
            params![last_bd_from_id, session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let target_path = match target_path {
        Some(path) if Path::new(&path).is_dir() => path,
        _ => return Err("bd: nothing to cancel".to_string()),
    };

    let now = current_ts();
    tx.execute(
        "UPDATE sessions SET cursor_id = ?1, last_bd_delta = 0, last_bd_from_id = 0, last_bd_to_id = 0, last_bd_armed = 0,
         last_seen_at = ?2 WHERE session_key = ?3",
        params![last_bd_from_id, now, session],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.execute("DELETE FROM undo_moves WHERE id = ?1", params![undo_id])
        .map_err(|e| format!("bd: db error: {e}"))?;

    tx.commit().map_err(|e| format!("bd: db error: {e}"))?;

    println!("{target_path}");
    Ok(())
}

fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("bd: db error: {e}"))?;
    }

    let conn = Connection::open(path).map_err(|e| format!("bd: db error: {e}"))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         CREATE TABLE IF NOT EXISTS events (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           session_key TEXT NOT NULL DEFAULT '',
           path TEXT NOT NULL,
           ts INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_key, id);
         CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts);
         CREATE TABLE IF NOT EXISTS sessions (
           session_key TEXT PRIMARY KEY,
           cursor_id INTEGER NOT NULL,
           last_bd_delta INTEGER NOT NULL DEFAULT 0,
           last_bd_from_id INTEGER NOT NULL DEFAULT 0,
           last_bd_to_id INTEGER NOT NULL DEFAULT 0,
           last_bd_armed INTEGER NOT NULL DEFAULT 0,
           last_seen_at INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS undo_moves (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           session_key TEXT NOT NULL,
           from_id INTEGER NOT NULL,
           to_id INTEGER NOT NULL,
           created_at INTEGER NOT NULL DEFAULT 0
         );
         CREATE INDEX IF NOT EXISTS idx_undo_moves_session_id ON undo_moves(session_key, id);
         CREATE TABLE IF NOT EXISTS meta (
           key TEXT PRIMARY KEY,
           value INTEGER NOT NULL
         );
         ",
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    ensure_schema(&conn)?;
    Ok(conn)
}

fn xdg_state_dir() -> Result<PathBuf, String> {
    if let Ok(state_home) = env::var("XDG_STATE_HOME") {
        return Ok(PathBuf::from(state_home).join("back-directory"));
    }

    let home = env::var("HOME").map_err(|_| "bd: HOME not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("back-directory"))
}

fn db_path() -> Result<PathBuf, String> {
    Ok(xdg_state_dir()?.join("bd.sqlite3"))
}

fn rotate_events(tx: &rusqlite::Transaction<'_>, session: &str) -> Result<(), String> {
    let min_session_cursor_id: Option<i64> = tx
        .query_row(
            "SELECT MIN(val) FROM (
               SELECT cursor_id AS val FROM sessions WHERE session_key = ?1 AND cursor_id != 0
               UNION ALL
               SELECT last_bd_from_id FROM sessions WHERE session_key = ?1 AND last_bd_from_id != 0
               UNION ALL
               SELECT last_bd_to_id FROM sessions WHERE session_key = ?1 AND last_bd_to_id != 0
               UNION ALL
               SELECT from_id FROM undo_moves WHERE session_key = ?1
               UNION ALL
               SELECT to_id FROM undo_moves WHERE session_key = ?1
             )",
            params![session],
            |row| row.get(0),
        )
        .map_err(|e| format!("bd: db error: {e}"))?;

    let rotation_cutoff_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM events WHERE session_key = ?1 ORDER BY id DESC LIMIT 1 OFFSET 9999",
            params![session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let (rotation_cutoff_id, min_session_cursor_id) =
        match (rotation_cutoff_id, min_session_cursor_id) {
            (Some(rotation_cutoff_id), Some(min_session_cursor_id)) => {
                (rotation_cutoff_id, min_session_cursor_id)
            }
            _ => return Ok(()),
        };

    let delete_before = rotation_cutoff_id.min(min_session_cursor_id);
    if delete_before <= 0 {
        return Ok(());
    }

    tx.execute(
        "DELETE FROM events WHERE session_key = ?1 AND id < ?2",
        params![session, delete_before],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    Ok(())
}

fn ensure_schema(conn: &Connection) -> Result<(), String> {
    ensure_column(
        conn,
        "sessions",
        "last_seen_at",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "undo_moves",
        "created_at",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| format!("bd: db error: {e}"))?;
    let mut rows = stmt.query([]).map_err(|e| format!("bd: db error: {e}"))?;
    while let Some(row) = rows.next().map_err(|e| format!("bd: db error: {e}"))? {
        let name: String = row.get(1).map_err(|e| format!("bd: db error: {e}"))?;
        if name == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;
    Ok(())
}

fn maybe_run_cleanup(conn: &mut Connection, session: &str) -> Result<(), String> {
    let now = current_ts();
    let last_cleanup_at: i64 = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![META_LAST_CLEANUP_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?
        .unwrap_or(0);

    if now.saturating_sub(last_cleanup_at) < CLEANUP_INTERVAL_SECS {
        return Ok(());
    }

    let session_cutoff = now - SESSION_RETENTION_SECS;
    let undo_cutoff = now - UNDO_RETENTION_SECS;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    tx.execute(
        "DELETE FROM sessions WHERE last_seen_at < ?1 AND session_key != ?2",
        params![session_cutoff, session],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.execute(
        "DELETE FROM undo_moves WHERE created_at < ?1",
        params![undo_cutoff],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![META_LAST_CLEANUP_KEY, now],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.commit().map_err(|e| format!("bd: db error: {e}"))?;
    Ok(())
}

fn cmd_doctor(full: bool, json: bool) -> Result<(), String> {
    let conn = open_db()?;
    let path = db_path()?;
    let db_size = file_size(&path);
    let wal_path = PathBuf::from(format!("{}-wal", path.display()));
    let shm_path = PathBuf::from(format!("{}-shm", path.display()));
    let wal_size = file_size(&wal_path);
    let shm_size = file_size(&shm_path);

    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .map_err(|e| format!("bd: db error: {e}"))?;
    let freelist_count: i64 = conn
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .map_err(|e| format!("bd: db error: {e}"))?;
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .map_err(|e| format!("bd: db error: {e}"))?;

    let events_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
        .map_err(|e| format!("bd: db error: {e}"))?;
    let sessions_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .map_err(|e| format!("bd: db error: {e}"))?;
    let undo_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM undo_moves", [], |row| row.get(0))
        .map_err(|e| format!("bd: db error: {e}"))?;

    let last_cleanup_at: i64 = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![META_LAST_CLEANUP_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?
        .unwrap_or(0);

    let integrity = if full {
        let mut stmt = conn
            .prepare("PRAGMA integrity_check")
            .map_err(|e| format!("bd: db error: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("bd: db error: {e}"))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("bd: db error: {e}"))?);
        }
        Some(results)
    } else {
        None
    };

    let now = current_ts();
    if json {
        let db_path_json = json_escape(&path.to_string_lossy());
        let integrity_json = integrity.as_ref().map(|rows| {
            format!(
                "[{}]",
                rows.iter()
                    .map(|r| format!("\"{}\"", json_escape(r)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        });
        let last_cleanup_age_days = if last_cleanup_at > 0 {
            Some((now - last_cleanup_at) / 86_400)
        } else {
            None
        };
        let last_cleanup_rfc3339 = format_ts(last_cleanup_at);

        println!(
            "{{\"database\":\"{db_path}\",\"db_size_bytes\":{db_size},\"wal_size_bytes\":{wal_size},\"shm_size_bytes\":{shm_size},\"page_count\":{page_count},\"freelist_count\":{freelist_count},\"page_size\":{page_size},\"events\":{events},\"sessions\":{sessions},\"undo_moves\":{undo},\"last_cleanup_at\":{last_cleanup_at},\"last_cleanup_at_rfc3339\":{last_cleanup_rfc3339},\"last_cleanup_age_days\":{last_cleanup_age},{integrity}}}",
            db_path = db_path_json,
            db_size = db_size.map_or("null".to_string(), |v| v.to_string()),
            wal_size = wal_size.map_or("null".to_string(), |v| v.to_string()),
            shm_size = shm_size.map_or("null".to_string(), |v| v.to_string()),
            page_count = page_count,
            freelist_count = freelist_count,
            page_size = page_size,
            events = events_count,
            sessions = sessions_count,
            undo = undo_count,
            last_cleanup_at = last_cleanup_at,
            last_cleanup_rfc3339 = last_cleanup_rfc3339
                .map(|v| format!("\"{}\"", json_escape(&v)))
                .unwrap_or_else(|| "null".to_string()),
            last_cleanup_age = last_cleanup_age_days.map_or("null".to_string(), |v| v.to_string()),
            integrity = integrity_json.map_or(String::new(), |v| format!(",\"integrity_check\":{v}")),
        );
        return Ok(());
    }

    println!("sqlite.database");
    println!("  path: {}", path.display());
    if let Some(size) = db_size {
        println!("  size: {} ({} bytes)", format_bytes(size), size);
    } else {
        println!("  size: unknown");
    }
    if wal_size.is_some() || shm_size.is_some() {
        println!("sqlite.wal");
        if let Some(size) = wal_size {
            println!("  size: {} ({} bytes)", format_bytes(size), size);
        }
        if let Some(size) = shm_size {
            println!("  shm size: {} ({} bytes)", format_bytes(size), size);
        }
    }
    println!("sqlite.stats");
    println!("  page_count: {page_count}");
    println!("  freelist_count: {freelist_count}");
    println!("  page_size: {page_size}");
    println!("app.tables");
    println!("  events: {events_count}");
    println!("  sessions: {sessions_count}");
    println!("  undo_moves: {undo_count}");
    if last_cleanup_at > 0 {
        let age_days = (now - last_cleanup_at) / 86_400;
        let formatted = format_ts(last_cleanup_at).unwrap_or_else(|| "unknown".to_string());
        println!("cleanup");
        println!("  last_cleanup_at: {formatted} ({age_days} days ago)");
    } else {
        println!("cleanup");
        println!("  last_cleanup_at: never");
    }
    if let Some(rows) = integrity {
        if rows.len() == 1 && rows[0] == "ok" {
            println!("integrity_check: ok");
        } else {
            println!("integrity_check:");
            for line in rows {
                println!("- {line}");
            }
        }
    }
    Ok(())
}

fn file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GiB", value / GB)
    } else if value >= MB {
        format!("{:.2} MiB", value / MB)
    } else if value >= KB {
        format!("{:.2} KiB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_ts(ts: i64) -> Option<String> {
    if ts <= 0 {
        return None;
    }
    OffsetDateTime::from_unix_timestamp(ts)
        .ok()
        .and_then(|dt| dt.format(&Rfc3339).ok())
}

fn current_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
