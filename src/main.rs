use clap::{Parser, Subcommand};
use rusqlite::{params, Connection, OptionalExtension};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const BD_MAX_BACK: u32 = 999;
const BD_DEFAULT_LIST: u32 = 10;

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

    let mut latest_id = None;
    if last_path.as_deref() != Some(pwd) {
        let ts = current_ts();
        tx.execute(
            "INSERT INTO events (session_key, path, ts) VALUES (?1, ?2, ?3)",
            params![session, pwd, ts],
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
            let ts = current_ts();
            tx.execute(
                "INSERT INTO events (session_key, path, ts) VALUES (?1, ?2, ?3)",
                params![session, pwd, ts],
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

    tx.execute(
        "INSERT INTO undo_moves (session_key, from_id, to_id) VALUES (?1, ?2, ?3)",
        params![session, cursor_id, target_id],
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
    for (step, path) in lines.into_iter().rev() {
        println!("{:>width$} {}", step, path, width = width);
    }

    Ok(())
}

fn cmd_cancel(session: &str) -> Result<(), String> {
    let mut conn = open_db()?;
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

    tx.execute(
        "UPDATE sessions SET cursor_id = ?1, last_bd_delta = 0, last_bd_from_id = 0, last_bd_to_id = 0, last_bd_armed = 0
         WHERE session_key = ?2",
        params![last_bd_from_id, session],
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
           last_bd_armed INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS undo_moves (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           session_key TEXT NOT NULL,
           from_id INTEGER NOT NULL,
           to_id INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_undo_moves_session_id ON undo_moves(session_key, id);
         ",
    )
    .map_err(|e| format!("bd: db error: {e}"))?;

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

fn current_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
