//! SQLite event-store prototype with explicit durability settings.

mod snapshot;

pub use snapshot::{EntitySnapshot, PendingWorkSnapshot, Snapshot, SnapshotCodec, SnapshotError};

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

use palimpsest_sim_events::{EVENT_SCHEMA_VERSION, EventId, EventRecord};
use rusqlite::{Connection, OptionalExtension, params};

/// SQLite-backed structured event prototype.
pub struct EventStore {
    connection: Connection,
}

impl EventStore {
    /// Opens or creates a file store, enables WAL/NORMAL/foreign keys, and migrates schema v1.
    ///
    /// # Errors
    /// Returns [`StorageError`] when SQLite cannot open or configure the store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        let mut store = Self { connection };
        store.configure(true)?;
        store.migrate()?;
        Ok(store)
    }

    /// Creates an in-memory store for focused tests.
    ///
    /// # Errors
    /// Returns [`StorageError`] when SQLite cannot initialize the store.
    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        let mut store = Self { connection };
        store.configure(false)?;
        store.migrate()?;
        Ok(store)
    }

    /// Appends a batch atomically.
    ///
    /// Causes must already exist or occur earlier in this batch. Any failure
    /// rolls back the entire batch.
    ///
    /// # Errors
    /// Returns [`StorageError`] for invalid serialization, duplicate IDs,
    /// missing causal references, or SQLite failures.
    pub fn append_batch(&mut self, events: &[EventRecord]) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        {
            let mut event_statement = transaction.prepare_cached(
                "INSERT INTO events(event_id, timestamp, event_type, schema_version, payload_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            let mut cause_statement = transaction.prepare_cached(
                "INSERT INTO event_causes(event_id, cause_event_id) VALUES (?1, ?2)",
            )?;
            for event in events {
                event
                    .validate()
                    .map_err(|error| StorageError::InvalidEvent(error.to_string()))?;
                let event_id = to_sql_id(event.event_id())?;
                let payload = serde_json::to_vec(event)?;
                event_statement.execute(params![
                    event_id,
                    event.timestamp().as_seconds(),
                    event.event_type(),
                    i64::from(EVENT_SCHEMA_VERSION),
                    payload,
                ])?;
                for &cause in event.causes() {
                    cause_statement.execute(params![event_id, to_sql_id(cause)?])?;
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Loads one structured event by stable ID.
    ///
    /// # Errors
    /// Returns [`StorageError`] for SQLite or event decoding failures.
    pub fn get(&self, event_id: EventId) -> Result<Option<EventRecord>, StorageError> {
        let payload: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT payload_json FROM events WHERE event_id = ?1",
                [to_sql_id(event_id)?],
                |row| row.get(0),
            )
            .optional()?;
        payload
            .map(|bytes| serde_json::from_slice(&bytes).map_err(StorageError::from))
            .transpose()
    }

    /// Returns stored event count.
    ///
    /// # Errors
    /// Returns [`StorageError`] for query failures.
    pub fn event_count(&self) -> Result<u64, StorageError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        u64::try_from(count).map_err(|_| StorageError::InvalidCount(count))
    }

    /// Runs SQLite integrity checking.
    ///
    /// # Errors
    /// Returns [`StorageError`] for query failures or a failed integrity verdict.
    pub fn integrity_check(&self) -> Result<(), StorageError> {
        let verdict: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if verdict == "ok" {
            Ok(())
        } else {
            Err(StorageError::Integrity(verdict))
        }
    }

    /// Checkpoints and truncates the WAL.
    ///
    /// # Errors
    /// Returns [`StorageError`] when checkpointing fails.
    pub fn checkpoint(&self) -> Result<(), StorageError> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    fn configure(&mut self, file_backed: bool) -> Result<(), StorageError> {
        self.connection.pragma_update(None, "foreign_keys", "ON")?;
        self.connection
            .pragma_update(None, "synchronous", "NORMAL")?;
        if file_backed {
            let mode: String = self
                .connection
                .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
            if !mode.eq_ignore_ascii_case("wal") {
                return Err(StorageError::JournalMode(mode));
            }
        }
        Ok(())
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                event_id INTEGER PRIMARY KEY CHECK(event_id > 0),
                timestamp INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                payload_json BLOB NOT NULL
            );
            CREATE INDEX IF NOT EXISTS events_timestamp_idx ON events(timestamp, event_id);
            CREATE INDEX IF NOT EXISTS events_type_idx ON events(event_type, timestamp);
            CREATE TABLE IF NOT EXISTS event_causes (
                event_id INTEGER NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
                cause_event_id INTEGER NOT NULL REFERENCES events(event_id),
                PRIMARY KEY(event_id, cause_event_id),
                CHECK(event_id != cause_event_id)
            );",
        )?;
        Ok(())
    }
}

fn to_sql_id(id: EventId) -> Result<i64, StorageError> {
    i64::try_from(id.get()).map_err(|_| StorageError::EventIdOutOfRange(id.get()))
}

/// Event-store failure.
#[derive(Debug)]
pub enum StorageError {
    /// SQLite failure.
    Sqlite(rusqlite::Error),
    /// JSON failure.
    Json(serde_json::Error),
    /// Event failed validation.
    InvalidEvent(String),
    /// `u64` event ID cannot fit SQLite INTEGER.
    EventIdOutOfRange(u64),
    /// Invalid count returned by SQLite.
    InvalidCount(i64),
    /// WAL was unavailable.
    JournalMode(String),
    /// Integrity check failed.
    Integrity(String),
}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}
impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "event store error: {self:?}")
    }
}
impl Error for StorageError {}

#[cfg(test)]
mod tests {
    use super::EventStore;
    use palimpsest_sim_events::{EventId, EventRecord};
    use palimpsest_sim_time::SimInstant;
    use tempfile::tempdir;

    fn event(raw: u64) -> EventRecord {
        EventRecord::new(
            EventId::new(raw).expect("non-zero"),
            SimInstant::from_seconds(i64::try_from(raw).expect("fits")),
            "test",
        )
        .expect("valid")
    }

    #[test]
    fn batch_round_trip_and_duplicate_rollback() {
        let mut store = EventStore::in_memory().expect("store");
        store.append_batch(&[event(1), event(2)]).expect("append");
        assert_eq!(store.event_count().expect("count"), 2);
        assert_eq!(
            store.get(EventId::new(2).expect("non-zero")).expect("get"),
            Some(event(2))
        );
        assert!(store.append_batch(&[event(3), event(2)]).is_err());
        assert_eq!(store.event_count().expect("count"), 2);
        store.integrity_check().expect("integrity");
    }

    #[test]
    fn file_store_survives_checkpoint_and_reopen() {
        let directory = tempdir().expect("temp directory");
        let path = directory.path().join("world.db");
        {
            let mut store = EventStore::open(&path).expect("open");
            store.append_batch(&[event(1)]).expect("append");
            store.checkpoint().expect("checkpoint");
        }
        let store = EventStore::open(&path).expect("reopen");
        assert_eq!(store.event_count().expect("count"), 1);
        store.integrity_check().expect("integrity");
    }
}
