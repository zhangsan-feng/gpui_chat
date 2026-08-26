use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use log::{info, warn};
use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::LoginResponseMsg;

const DATABASE_EXTENSION: &str = "db";
static PROJECT_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

fn database_directory() -> anyhow::Result<PathBuf> {
    let project_directory = PROJECT_DIRECTORY
        .get()
        .ok_or_else(|| anyhow::anyhow!("session store is not initialized"))?;
    let directory = project_directory.join("data").join("client");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

fn database_path(login_name: &str) -> anyhow::Result<PathBuf> {
    if !is_valid_login_name(login_name) {
        return Err(anyhow::anyhow!("invalid login name for session database"));
    }
    Ok(database_directory()?.join(format!("{login_name}.{DATABASE_EXTENSION}")))
}

fn is_valid_login_name(login_name: &str) -> bool {
    !login_name.is_empty()
        && login_name != "."
        && login_name != ".."
        && login_name.trim() == login_name
        && !login_name.chars().any(|character| {
            matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        })
}

fn open_connection(login_name: &str) -> anyhow::Result<Connection> {
    let path = database_path(login_name)?;
    info!("using session database at {}", path.display());
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS auth_session (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            session_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS notification_read (
            notification_id TEXT PRIMARY KEY,
            read_at INTEGER NOT NULL
        );
        ",
    )?;
    Ok(connection)
}

pub fn initialize(project_directory: PathBuf) -> anyhow::Result<()> {
    let project_directory = project_directory.canonicalize()?;
    PROJECT_DIRECTORY
        .set(project_directory)
        .map_err(|_| anyhow::anyhow!("session store is already initialized"))?;
    database_directory()?;
    migrate_legacy_database()
}

fn migrate_legacy_database() -> anyhow::Result<()> {
    let legacy_path = database_directory()?.join("session.sqlite3");
    if !legacy_path.exists() {
        return Ok(());
    }

    let connection = Connection::open(&legacy_path)?;
    let table_exists = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'auth_session')",
        [],
        |row| row.get::<_, i64>(0),
    )? == 1;
    let legacy_session = if table_exists {
        connection
            .query_row(
                "SELECT session_json FROM auth_session WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
    } else {
        None
    };
    drop(connection);

    if let Some(session_json) = legacy_session {
        let mut session: LoginResponseMsg = serde_json::from_str(&session_json)?;
        if session.login_name.is_empty() {
            session.login_name = session.username.clone();
        }
        if session.is_authenticated() && is_valid_login_name(&session.login_name) {
            save(&session)?;
        }
    }
    fs::remove_file(legacy_path)?;
    Ok(())
}

pub fn load_all() -> anyhow::Result<Vec<LoginResponseMsg>> {
    let directory = database_directory()?;
    let mut sessions = Vec::new();
    let entries = fs::read_dir(directory)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some(DATABASE_EXTENSION) {
            continue;
        }

        let Some(login_name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        match load(login_name) {
            Ok(Some(session)) if session.is_authenticated() => sessions.push(session),
            Ok(Some(_)) => {
                warn!(
                    "ignoring unauthenticated session database {}",
                    path.display()
                );
            }
            Ok(None) => {}
            Err(error) => warn!(
                "failed to load session database {}: {}",
                path.display(),
                error
            ),
        }
    }

    sessions.sort_by(|left, right| left.login_name.cmp(&right.login_name));
    Ok(sessions)
}

fn load(login_name: &str) -> anyhow::Result<Option<LoginResponseMsg>> {
    let connection = open_connection(login_name)?;
    let session_json = connection
        .query_row(
            "SELECT session_json FROM auth_session WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    session_json
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

pub fn save(session: &LoginResponseMsg) -> anyhow::Result<()> {
    if session.user_id.is_empty() {
        return Err(anyhow::anyhow!("cannot save a session without user id"));
    }
    if session.login_name.is_empty() {
        return Err(anyhow::anyhow!("cannot save a session without login name"));
    }

    let connection = open_connection(&session.login_name)?;
    connection.execute(
        "
        INSERT INTO auth_session (id, session_json, updated_at)
        VALUES (1, ?1, ?2)
        ON CONFLICT(id) DO UPDATE SET
            session_json = excluded.session_json,
            updated_at = excluded.updated_at
        ",
        params![
            serde_json::to_string(session)?,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

pub fn load_notification_read_ids(login_name: &str) -> anyhow::Result<HashSet<String>> {
    let connection = open_connection(login_name)?;
    let mut statement = connection
        .prepare("SELECT notification_id FROM notification_read ORDER BY notification_id")?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<HashSet<_>, _>>()?;
    Ok(ids)
}

pub fn mark_notifications_read(
    login_name: &str,
    notification_ids: &[String],
) -> anyhow::Result<()> {
    if notification_ids.is_empty() {
        return Ok(());
    }

    let mut connection = open_connection(login_name)?;
    let transaction = connection.transaction()?;
    let read_at = chrono::Utc::now().timestamp_millis();
    for notification_id in notification_ids {
        if notification_id.is_empty() {
            continue;
        }
        transaction.execute(
            "INSERT OR IGNORE INTO notification_read (notification_id, read_at) VALUES (?1, ?2)",
            params![notification_id, read_at],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn mark_notification_unread(login_name: &str, notification_id: &str) -> anyhow::Result<()> {
    if notification_id.is_empty() {
        return Ok(());
    }

    let connection = open_connection(login_name)?;
    connection.execute(
        "DELETE FROM notification_read WHERE notification_id = ?1",
        params![notification_id],
    )?;
    Ok(())
}

pub fn remove(login_name: &str) -> anyhow::Result<()> {
    let path = database_path(login_name)?;
    if Path::new(&path).exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
