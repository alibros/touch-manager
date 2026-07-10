use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;
use uuid::Uuid;

pub struct HistoryStore {
    path: Mutex<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at: String,
    pub firmware_id: String,
    pub firmware_name: String,
    pub version: String,
    pub sha256: String,
    pub target_profile: String,
    pub status: String,
    pub transcript: String,
}

pub struct NewHistoryEntry<'a> {
    pub firmware_id: &'a str,
    pub firmware_name: &'a str,
    pub version: &'a str,
    pub sha256: &'a str,
    pub target_profile: &'a str,
    pub status: &'a str,
    pub transcript: &'a str,
}

impl HistoryStore {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let store = Self {
            path: Mutex::new(path),
        };
        store.initialize()?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, String> {
        Connection::open(self.path.lock().clone()).map_err(|error| error.to_string())
    }

    fn initialize(&self) -> Result<(), String> {
        let connection = self.connection()?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS install_history (
                    id TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    firmware_id TEXT NOT NULL,
                    firmware_name TEXT NOT NULL,
                    version TEXT NOT NULL,
                    sha256 TEXT NOT NULL,
                    target_profile TEXT NOT NULL,
                    status TEXT NOT NULL,
                    transcript TEXT NOT NULL
                );",
            )
            .map_err(|error| error.to_string())
    }

    pub fn record(&self, record: NewHistoryEntry<'_>) -> Result<HistoryEntry, String> {
        let entry = HistoryEntry {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            firmware_id: record.firmware_id.into(),
            firmware_name: record.firmware_name.into(),
            version: record.version.into(),
            sha256: record.sha256.into(),
            target_profile: record.target_profile.into(),
            status: record.status.into(),
            transcript: record.transcript.into(),
        };
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO install_history VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    entry.id,
                    entry.created_at,
                    entry.firmware_id,
                    entry.firmware_name,
                    entry.version,
                    entry.sha256,
                    entry.target_profile,
                    entry.status,
                    entry.transcript,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(entry)
    }

    pub fn list(&self) -> Result<Vec<HistoryEntry>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, created_at, firmware_id, firmware_name, version, sha256,
                        target_profile, status, transcript
                 FROM install_history ORDER BY created_at DESC LIMIT 200",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    firmware_id: row.get(2)?,
                    firmware_name: row.get(3)?,
                    version: row.get(4)?,
                    sha256: row.get(5)?,
                    target_profile: row.get(6)?,
                    status: row.get(7)?,
                    transcript: row.get(8)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }
}
