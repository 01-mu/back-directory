use clap::{Parser, Subcommand};
use rusqlite::{params, Connection, OptionalExtension};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const BD_MAX_BACK: u32 = 99;

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
    },
    Cancel {
        #[arg(long)]
        session: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Record { session, pwd } => cmd_record(&session, &pwd),
        Commands::Back { session, n } => cmd_back(&session, n),
        Commands::Cancel { session } => cmd_cancel(&session),
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

    let conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let last_path: Option<String> = tx
        .query_row(
            "SELECT path FROM events ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut latest_id = None;
    if last_path.as_deref() != Some(pwd) {
        let ts = current_ts();
        tx.execute(
            "INSERT INTO events (path, ts) VALUES (?1, ?2)",
            params![pwd, ts],
        )
        .map_err(|e| format!("bd: db error: {e}"))?;
        latest_id = Some(tx.last_insert_rowid());
    }

    if latest_id.is_none() {
        latest_id = tx
            .query_row(
                "SELECT id FROM events ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("bd: db error: {e}"))?;
    }

    let latest_id = match latest_id {
        Some(id) => id,
        None => {
            let ts = current_ts();
            tx.execute(
                "INSERT INTO events (path, ts) VALUES (?1, ?2)",
                params![pwd, ts],
            )
            .map_err(|e| format!("bd: db error: {e}"))?;
            tx.last_insert_rowid()
        }
    };

    tx.execute(
        "INSERT INTO sessions (session_key, cursor_id, last_bd_delta, last_bd_from_id, last_bd_to_id, last_bd_armed)
         VALUES (?1, ?2, 0, 0, 0, 0)
         ON CONFLICT(session_key) DO UPDATE SET
           cursor_id = excluded.cursor_id,
           last_bd_delta = 0,
           last_bd_from_id = 0,
           last_bd_to_id = 0,
           last_bd_armed = 0",
        params![session, latest_id],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.commit()
        .map_err(|e| format!("bd: db error: {e}"))?;
    Ok(())
}

fn cmd_back(session: &str, n: u32) -> Result<(), String> {
    if n == 0 {
        return Err("bd: usage: bd [N|c]".to_string());
    }
    if n > BD_MAX_BACK {
        return Err(format!("bd: max is {BD_MAX_BACK}"));
    }

    let conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let latest_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM events ORDER BY id DESC LIMIT 1",
            [],
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
            "SELECT id FROM events WHERE id = ?1",
            params![cursor_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    if cursor_exists.is_none() {
        cursor_id = latest_id.ok_or_else(|| "bd: no earlier directory".to_string())?;
    }

    let mut stmt = tx
        .prepare("SELECT id, path FROM events WHERE id < ?1 ORDER BY id DESC")
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut rows = stmt
        .query(params![cursor_id])
        .map_err(|e| format!("bd: db error: {e}"))?;

    let mut steps: u32 = 0;
    let mut target: Option<(i64, String, u32)> = None;
    let mut oldest_existing: Option<(i64, String, u32)> = None;

    while let Some(row) = rows
        .next()
        .map_err(|e| format!("bd: db error: {e}"))?
    {
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

    let (target_id, target_path, actual_steps) = match target.or(oldest_existing) {
        Some(value) => value,
        None => return Err("bd: no earlier directory".to_string()),
    };

    tx.execute(
        "INSERT INTO sessions (session_key, cursor_id, last_bd_delta, last_bd_from_id, last_bd_to_id, last_bd_armed)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)
         ON CONFLICT(session_key) DO UPDATE SET
           cursor_id = excluded.cursor_id,
           last_bd_delta = excluded.last_bd_delta,
           last_bd_from_id = excluded.last_bd_from_id,
           last_bd_to_id = excluded.last_bd_to_id,
           last_bd_armed = 1",
        params![session, target_id, actual_steps, cursor_id, target_id],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.commit()
        .map_err(|e| format!("bd: db error: {e}"))?;

    println!("{target_path}");
    Ok(())
}

fn cmd_cancel(session: &str) -> Result<(), String> {
    let conn = open_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let row: Option<(i64, i64, i64)> = tx
        .query_row(
            "SELECT cursor_id, last_bd_from_id, last_bd_armed FROM sessions WHERE session_key = ?1",
            params![session],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let (cursor_id, last_bd_from_id, last_bd_armed) = match row {
        Some(value) => value,
        None => return Err("bd: nothing to cancel".to_string()),
    };

    if last_bd_armed != 1 || last_bd_from_id == 0 {
        return Err("bd: nothing to cancel".to_string());
    }

    let target_path: Option<String> = tx
        .query_row(
            "SELECT path FROM events WHERE id = ?1",
            params![last_bd_from_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let target_path = match target_path {
        Some(path) if Path::new(&path).is_dir() => path,
        _ => return Err("bd: nothing to cancel".to_string()),
    };

    tx.execute(
        "UPDATE sessions SET cursor_id = ?1, last_bd_delta = 0, last_bd_from_id = 0, last_bd_to_id = 0, last_bd_armed = 0
         WHERE session_key = ?2",
        params![last_bd_from_id, session],
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    tx.commit()
        .map_err(|e| format!("bd: db error: {e}"))?;

    let _ = cursor_id;
    println!("{target_path}");
    Ok(())
}

fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("bd: db error: {e}"))?;
    }

    let conn = Connection::open(path).map_err(|e| format!("bd: db error: {e}"))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         CREATE TABLE IF NOT EXISTS events (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           path TEXT NOT NULL,
           ts INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts);
         CREATE TABLE IF NOT EXISTS sessions (
           session_key TEXT PRIMARY KEY,
           cursor_id INTEGER NOT NULL,
           last_bd_delta INTEGER NOT NULL DEFAULT 0,
           last_bd_from_id INTEGER NOT NULL DEFAULT 0,
           last_bd_to_id INTEGER NOT NULL DEFAULT 0,
           last_bd_armed INTEGER NOT NULL DEFAULT 0
         );",
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

    Ok(conn)
}

fn db_path() -> Result<PathBuf, String> {
    if let Ok(state_home) = env::var("XDG_STATE_HOME") {
        return Ok(PathBuf::from(state_home).join("back-directory").join("bd.sqlite3"));
    }

    let home = env::var("HOME").map_err(|_| "bd: HOME not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("back-directory")
        .join("bd.sqlite3"))
}

fn current_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
