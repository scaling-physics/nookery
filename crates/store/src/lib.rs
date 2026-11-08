//! SQLite storage for the ledger. Only events are stored; the inventory is
//! replayed from them in memory on open (a few thousand events replay in
//! well under a millisecond) and kept up to date as events are appended.

use std::path::{Path, PathBuf};

use nookery_core::{Event, Inventory, LedgerError, LotId};
use rusqlite::{params, Connection};

/// Folder name, the same as the Tauri app's identifier, so the CLI and the
/// app share one database.
pub const APP_ID: &str = "dev.nookery.app";
pub const DB_FILE: &str = "nookery.db";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("stored event {seq} can't be read: {source}")]
    Corrupt { seq: i64, source: serde_json::Error },
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    #[error("database is from a newer Nookery (schema {0})")]
    TooNew(i64),
    #[error("no lot ends with {0:?}")]
    NoMatch(String),
    #[error("{0:?} matches several lots; type more of the id")]
    Ambiguous(String),
    #[error("can't create {0}: {1}")]
    Dir(PathBuf, std::io::Error),
}

/// `$NOOKERY_DATA_DIR`, else the OS app-data folder (macOS:
/// `~/Library/Application Support/dev.nookery.app`).
pub fn default_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("NOOKERY_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
}

const MIGRATIONS: &[&str] = &[
    // 1: the ledger. `body` is the event as JSON; `lot`, `kind` and `at` are
    // copies for querying.
    "CREATE TABLE events (
        seq  INTEGER PRIMARY KEY,
        at   TEXT NOT NULL,
        lot  TEXT NOT NULL,
        kind TEXT NOT NULL,
        body TEXT NOT NULL
    );
    CREATE INDEX events_lot ON events(lot);",
];

pub struct Store {
    conn: Connection,
    inventory: Inventory,
    events: usize,
}

impl Store {
    /// Opens (creating if needed) `<dir>/nookery.db`.
    pub fn open_dir(dir: &Path) -> Result<Self, StoreError> {
        std::fs::create_dir_all(dir).map_err(|e| StoreError::Dir(dir.to_path_buf(), e))?;
        Self::open(&dir.join(DB_FILE))
    }

    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::from_conn(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::from_conn(Connection::open_in_memory()?)
    }

    fn from_conn(conn: Connection) -> Result<Self, StoreError> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&conn)?;
        let mut store = Store {
            conn,
            inventory: Inventory::default(),
            events: 0,
        };
        store.reload()?;
        Ok(store)
    }

    /// Re-reads every event and rebuilds the inventory.
    pub fn reload(&mut self) -> Result<(), StoreError> {
        let events = self.events()?;
        self.inventory = Inventory::replay(&events)?;
        self.events = events.len();
        Ok(())
    }

    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    pub fn event_count(&self) -> usize {
        self.events
    }

    /// Every event, oldest first.
    pub fn events(&self) -> Result<Vec<Event>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT seq, body FROM events ORDER BY seq")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.map(|row| {
            let (seq, body) = row?;
            serde_json::from_str(&body).map_err(|source| StoreError::Corrupt { seq, source })
        })
        .collect()
    }

    /// The events of one lot, oldest first.
    pub fn lot_events(&self, lot: LotId) -> Result<Vec<Event>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT seq, body FROM events WHERE lot = ?1 ORDER BY seq")?;
        let rows = stmt.query_map([lot.to_string()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (seq, body) = row?;
            serde_json::from_str(&body).map_err(|source| StoreError::Corrupt { seq, source })
        })
        .collect()
    }

    /// Appends events in one transaction, all or nothing: each must be valid
    /// against the inventory as the ones before it leave it.
    pub fn append(&mut self, events: &[Event]) -> Result<(), StoreError> {
        let mut next = self.inventory.clone();
        for e in events {
            next.apply(e)?;
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO events (at, lot, kind, body) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for e in events {
                let body = serde_json::to_string(e).expect("events always serialise");
                stmt.execute(params![
                    e.at.to_rfc3339(),
                    e.lot.to_string(),
                    e.kind.name(),
                    body
                ])?;
            }
        }
        tx.commit()?;
        self.inventory = next;
        self.events += events.len();
        Ok(())
    }

    /// The lot whose id ends with `short` (the CLI prints the last 8
    /// characters: UUID v7 ids begin with a timestamp, so their starts
    /// collide for lots added together).
    pub fn find_lot(&self, short: &str) -> Result<LotId, StoreError> {
        let p = short.trim().to_ascii_lowercase().replace('-', "");
        if p.is_empty() {
            return Err(StoreError::NoMatch(short.to_string()));
        }
        let mut found = self
            .inventory
            .all()
            .iter()
            .filter(|l| l.id.to_string().ends_with(&p));
        match (found.next(), found.next()) {
            (Some(l), None) => Ok(l.id),
            (None, _) => Err(StoreError::NoMatch(short.to_string())),
            _ => Err(StoreError::Ambiguous(short.to_string())),
        }
    }
}

fn migrate(conn: &Connection) -> Result<(), StoreError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let latest = MIGRATIONS.len() as i64;
    if version > latest {
        return Err(StoreError::TooNew(version));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use nookery_core::{EventKind, Location, Qty, Source};

    fn acquire(item: &str) -> Event {
        Event {
            at: Utc::now(),
            lot: LotId::new(),
            kind: EventKind::Acquired {
                item: item.into(),
                qty: Qty::pieces(2),
                location: Location::Fridge,
                source: Source::Manual,
                best_before: None,
            },
        }
    }

    #[test]
    fn events_survive_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let milk = acquire("milk");
        let id = milk.lot;
        {
            let mut s = Store::open_dir(dir.path()).unwrap();
            s.append(&[milk]).unwrap();
            s.append(&[Event {
                at: Utc::now(),
                lot: id,
                kind: EventKind::Opened,
            }])
            .unwrap();
        }
        let s = Store::open_dir(dir.path()).unwrap();
        assert_eq!(s.event_count(), 2);
        assert!(s.inventory().get(id).unwrap().opened_at.is_some());
        assert_eq!(s.lot_events(id).unwrap().len(), 2);
    }

    #[test]
    fn an_invalid_batch_writes_nothing() {
        let mut s = Store::open_in_memory().unwrap();
        let ok = acquire("bread");
        let bad = Event {
            at: Utc::now(),
            lot: LotId::new(),
            kind: EventKind::Binned,
        };
        assert!(matches!(s.append(&[ok, bad]), Err(StoreError::Ledger(_))));
        assert_eq!(s.event_count(), 0);
        assert_eq!(s.events().unwrap().len(), 0);
        assert_eq!(s.inventory().all().len(), 0);
    }

    #[test]
    fn finds_lots_by_short_id() {
        let mut s = Store::open_in_memory().unwrap();
        let e = acquire("cheese");
        let id = e.lot;
        s.append(&[e]).unwrap();
        let full = id.to_string();
        assert_eq!(s.find_lot(&full[full.len() - 8..]).unwrap(), id);
        assert!(matches!(s.find_lot("zzzz"), Err(StoreError::NoMatch(_))));
    }

    #[test]
    fn refuses_a_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(DB_FILE);
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        assert!(matches!(Store::open(&path), Err(StoreError::TooNew(99))));
    }
}
