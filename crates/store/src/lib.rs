//! SQLite storage for the ledger. Only events are stored; the inventory is
//! replayed from them in memory on open (a few thousand events replay in
//! well under a millisecond) and kept up to date as events are appended.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use nookery_core::{AiMode, Alias, Event, Inventory, LedgerError, LotId, CLOUD_MODEL};
use rusqlite::{params, Connection, Transaction};

/// Folder name, the same as the Tauri app's identifier, so the CLI and the
/// app share one database.
pub const APP_ID: &str = "dev.nookery.app";
pub const DB_FILE: &str = "nookery.db";
/// The `aliases` store name for names learned from photo confirmations
/// (key: `nookery_core::name_key` of what the model said).
pub const PHOTO_ALIASES: &str = "photo";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("stored event {seq} can't be read: {source}")]
    Corrupt { seq: i64, source: serde_json::Error },
    #[error("stored alias {key:?} of {store} can't be read: {source}")]
    CorruptAlias {
        store: String,
        key: String,
        source: serde_json::Error,
    },
    #[error("a template called {0:?} already exists")]
    DuplicateTemplate(String),
    #[error("no template {0}")]
    NoTemplate(i64),
    #[error("a template needs a name and some text")]
    EmptyTemplate,
    #[error("this receipt was already imported ({0})")]
    DuplicateReceipt(String),
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
    // 2: imported receipts (so one can't be imported twice) and learned
    // aliases (receipt text -> product, per store). Aliases are memory, not
    // inventory, so a plain upserted table is right; `body` is JSON.
    "CREATE TABLE receipts (
        id          TEXT PRIMARY KEY,
        store       TEXT NOT NULL,
        date        TEXT,
        total_cents INTEGER,
        imported_at TEXT NOT NULL,
        text        TEXT NOT NULL
    );
    CREATE TABLE aliases (
        store      TEXT NOT NULL,
        key        TEXT NOT NULL,
        body       TEXT NOT NULL,
        learned_at TEXT NOT NULL,
        PRIMARY KEY (store, key)
    );",
    // 3: recipe prompt templates the user picks from on the Recipes tab
    // (settings, not inventory). Seeded with a few the user can edit.
    "CREATE TABLE templates (
        id         INTEGER PRIMARY KEY,
        name       TEXT NOT NULL UNIQUE,
        body       TEXT NOT NULL,
        position   INTEGER NOT NULL,
        updated_at TEXT NOT NULL
    );
    INSERT INTO templates (name, body, position, updated_at) VALUES
      ('Use it up', 'Use as many of the use-soon items as possible. Simple, no-waste home cooking.', 0, '2026-11-09T00:00:00Z'),
      ('Quick weeknight', 'Dinner for two, ready in 30 minutes or less, one pan or pot if possible.', 1, '2026-11-09T00:00:00Z'),
      ('High protein', 'High in protein (at least 30 g per serving), filling, not deep-fried.', 2, '2026-11-09T00:00:00Z'),
      ('Vegetarian', 'Vegetarian (no meat or fish), hearty and colourful.', 3, '2026-11-09T00:00:00Z'),
      ('Something new', 'Surprise me: a dish from a cuisine I may not cook often, still mostly from what I have.', 4, '2026-11-09T00:00:00Z');",
    // 4: user settings (e.g. where each AI feature runs). Plain key/value.
    "CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
];

/// Where photo scanning runs (`AiMode`: local / fallback / cloud).
pub const SETTING_PHOTOS_AI: &str = "ai.photos";
/// Where recipe ideas and recipes are written.
pub const SETTING_RECIPES_AI: &str = "ai.recipes";
/// The cloud model (default `nookery_core::CLOUD_MODEL`).
pub const SETTING_CLOUD_MODEL: &str = "ai.cloud_model";

/// A recipe prompt template ("Quick weeknight": "Dinner for two, ready in
/// 30 minutes...").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    pub id: i64,
    pub name: String,
    /// What to ask for, in the user's words; added to the recipe prompt.
    pub body: String,
}

/// A receipt as recorded on import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptRecord {
    pub id: String,
    pub store: String,
    pub date: Option<NaiveDate>,
    pub total_cents: Option<i64>,
    pub text: String,
}

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
        let next = self.validate(events)?;
        let tx = self.conn.transaction()?;
        insert_events(&tx, events)?;
        tx.commit()?;
        self.inventory = next;
        self.events += events.len();
        Ok(())
    }

    /// The inventory after `events`, or the first ledger error.
    fn validate(&self, events: &[Event]) -> Result<Inventory, StoreError> {
        let mut next = self.inventory.clone();
        for e in events {
            next.apply(e)?;
        }
        Ok(next)
    }

    pub fn has_receipt(&self, id: &str) -> Result<bool, StoreError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM receipts WHERE id = ?1)",
            [id],
            |r| r.get(0),
        )?)
    }

    /// Every learned alias of one store, by key.
    pub fn aliases(&self, store: &str) -> Result<HashMap<String, Alias>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT key, body FROM aliases WHERE store = ?1")?;
        let rows = stmt.query_map([store], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (key, body) = row?;
            match serde_json::from_str(&body) {
                Ok(alias) => Ok((key, alias)),
                Err(source) => Err(StoreError::CorruptAlias {
                    store: store.to_string(),
                    key,
                    source,
                }),
            }
        })
        .collect()
    }

    /// Records the receipt, appends its events and upserts the learned
    /// aliases, all in ONE transaction (all or nothing). Events are
    /// validated against the inventory first, exactly like `append`.
    pub fn import_receipt(
        &mut self,
        receipt: &ReceiptRecord,
        events: &[Event],
        learned: &[(String, Alias)],
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        if self.has_receipt(&receipt.id)? {
            return Err(StoreError::DuplicateReceipt(receipt.id.clone()));
        }
        let next = self.validate(events)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO receipts (id, store, date, total_cents, imported_at, text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                receipt.id,
                receipt.store,
                receipt.date.map(|d| d.to_string()),
                receipt.total_cents,
                at.to_rfc3339(),
                receipt.text
            ],
        )?;
        insert_events(&tx, events)?;
        upsert_aliases(&tx, &receipt.store, learned, at)?;
        tx.commit()?;
        self.inventory = next;
        self.events += events.len();
        Ok(())
    }

    /// Appends events and upserts aliases learned for `store` in ONE
    /// transaction (all or nothing), validated like `append`. Photo
    /// confirmations use this with `store = PHOTO_ALIASES`.
    pub fn append_learning(
        &mut self,
        events: &[Event],
        store: &str,
        learned: &[(String, Alias)],
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let next = self.validate(events)?;
        let tx = self.conn.transaction()?;
        insert_events(&tx, events)?;
        upsert_aliases(&tx, store, learned, at)?;
        tx.commit()?;
        self.inventory = next;
        self.events += events.len();
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        Ok(match rows.next()? {
            Some(r) => Some(r.get(0)?),
            None => None,
        })
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// The mode stored under `key`; Local when unset or unreadable (never
    /// send anything off the Mac by accident).
    pub fn ai_mode(&self, key: &str) -> Result<AiMode, StoreError> {
        Ok(self
            .setting(key)?
            .and_then(|v| v.parse().ok())
            .unwrap_or_default())
    }

    /// The cloud model to use.
    pub fn cloud_model(&self) -> Result<String, StoreError> {
        Ok(self
            .setting(SETTING_CLOUD_MODEL)?
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| CLOUD_MODEL.to_string()))
    }

    /// Every template, in the user's order.
    pub fn templates(&self) -> Result<Vec<Template>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, name, body FROM templates ORDER BY position, id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Template {
                id: r.get(0)?,
                name: r.get(1)?,
                body: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Adds a template at the end (`id` None) or changes one. Names are
    /// unique (case-sensitive) and trimmed; both fields must be non-empty.
    pub fn save_template(
        &mut self,
        id: Option<i64>,
        name: &str,
        body: &str,
        at: DateTime<Utc>,
    ) -> Result<Template, StoreError> {
        let (name, body) = (name.trim(), body.trim());
        if name.is_empty() || body.is_empty() {
            return Err(StoreError::EmptyTemplate);
        }
        let result = match id {
            None => self.conn.execute(
                "INSERT INTO templates (name, body, position, updated_at)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM templates), ?3)",
                params![name, body, at.to_rfc3339()],
            ),
            Some(id) => self.conn.execute(
                "UPDATE templates SET name = ?2, body = ?3, updated_at = ?4 WHERE id = ?1",
                params![id, name, body, at.to_rfc3339()],
            ),
        };
        let changed = result.map_err(|e| match e {
            rusqlite::Error::SqliteFailure(f, _)
                if f.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                StoreError::DuplicateTemplate(name.to_string())
            }
            other => other.into(),
        })?;
        let id = match id {
            Some(id) if changed == 0 => return Err(StoreError::NoTemplate(id)),
            Some(id) => id,
            None => self.conn.last_insert_rowid(),
        };
        Ok(Template {
            id,
            name: name.to_string(),
            body: body.to_string(),
        })
    }

    pub fn delete_template(&mut self, id: i64) -> Result<(), StoreError> {
        match self
            .conn
            .execute("DELETE FROM templates WHERE id = ?1", [id])?
        {
            0 => Err(StoreError::NoTemplate(id)),
            _ => Ok(()),
        }
    }

    /// Puts the templates in this order (every id exactly once).
    pub fn order_templates(&mut self, ids: &[i64]) -> Result<(), StoreError> {
        let current: Vec<i64> = self.templates()?.iter().map(|t| t.id).collect();
        let (mut a, mut b) = (current.clone(), ids.to_vec());
        a.sort_unstable();
        b.sort_unstable();
        if a != b {
            let unknown = ids.iter().find(|i| !current.contains(i));
            return Err(StoreError::NoTemplate(
                *unknown.or(current.first()).unwrap_or(&0),
            ));
        }
        let tx = self.conn.transaction()?;
        for (pos, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE templates SET position = ?2 WHERE id = ?1",
                params![id, pos as i64],
            )?;
        }
        tx.commit()?;
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

/// A newer confirmation of the same (store, key) replaces the old alias.
fn upsert_aliases(
    tx: &Transaction,
    store: &str,
    learned: &[(String, Alias)],
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    let mut stmt = tx.prepare_cached(
        "INSERT INTO aliases (store, key, body, learned_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(store, key) DO UPDATE
         SET body = excluded.body, learned_at = excluded.learned_at",
    )?;
    for (key, alias) in learned {
        let body = serde_json::to_string(alias).expect("aliases always serialise");
        stmt.execute(params![store, key, body, at.to_rfc3339()])?;
    }
    Ok(())
}

fn insert_events(tx: &Transaction, events: &[Event]) -> Result<(), StoreError> {
    let mut stmt =
        tx.prepare_cached("INSERT INTO events (at, lot, kind, body) VALUES (?1, ?2, ?3, ?4)")?;
    for e in events {
        let body = serde_json::to_string(e).expect("events always serialise");
        stmt.execute(params![
            e.at.to_rfc3339(),
            e.lot.to_string(),
            e.kind.name(),
            body
        ])?;
    }
    Ok(())
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

    fn record(id: &str) -> ReceiptRecord {
        ReceiptRecord {
            id: id.into(),
            store: "rewe".into(),
            date: NaiveDate::from_ymd_opt(2026, 11, 7),
            total_cents: Some(1234),
            text: "BIO H-MILCH 1,5%  1,19".into(),
        }
    }

    fn alias(item: &str) -> Alias {
        Alias {
            item: item.into(),
            each: Some(Qty::pieces(1)),
            location: Location::Fridge,
        }
    }

    fn count(s: &Store, table: &str) -> i64 {
        s.conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn an_imported_receipt_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut s = Store::open_dir(dir.path()).unwrap();
            s.import_receipt(
                &record("0123456789abcdef"),
                &[acquire("milk")],
                &[("BIO H-MILCH 1,5%".into(), alias("milk"))],
                Utc::now(),
            )
            .unwrap();
        }
        let s = Store::open_dir(dir.path()).unwrap();
        assert!(s.has_receipt("0123456789abcdef").unwrap());
        assert!(!s.has_receipt("ffffffffffffffff").unwrap());
        assert_eq!(s.event_count(), 1);
        assert_eq!(
            s.aliases("rewe").unwrap()["BIO H-MILCH 1,5%"],
            alias("milk")
        );
    }

    #[test]
    fn importing_the_same_receipt_twice_fails_and_writes_nothing() {
        let mut s = Store::open_in_memory().unwrap();
        let r = record("0123456789abcdef");
        s.import_receipt(&r, &[acquire("milk")], &[], Utc::now())
            .unwrap();
        let again = s.import_receipt(
            &r,
            &[acquire("bread")],
            &[("X".into(), alias("bread"))],
            Utc::now(),
        );
        assert!(matches!(again, Err(StoreError::DuplicateReceipt(id)) if id == r.id));
        assert_eq!(s.event_count(), 1);
        assert_eq!(count(&s, "events"), 1);
        assert_eq!(count(&s, "receipts"), 1);
        assert!(s.aliases("rewe").unwrap().is_empty());
    }

    #[test]
    fn an_import_with_an_invalid_event_writes_nothing() {
        let mut s = Store::open_in_memory().unwrap();
        let bad = Event {
            at: Utc::now(),
            lot: LotId::new(),
            kind: EventKind::Binned,
        };
        let res = s.import_receipt(
            &record("0123456789abcdef"),
            &[acquire("milk"), bad],
            &[("MILCH".into(), alias("milk"))],
            Utc::now(),
        );
        assert!(matches!(res, Err(StoreError::Ledger(_))));
        assert_eq!(count(&s, "receipts"), 0);
        assert_eq!(count(&s, "aliases"), 0);
        assert_eq!(count(&s, "events"), 0);
        assert_eq!(s.event_count(), 0);
        assert!(s.inventory().all().is_empty());
    }

    #[test]
    fn a_later_alias_replaces_the_earlier_one_and_aliases_are_per_store() {
        let mut s = Store::open_in_memory().unwrap();
        s.import_receipt(
            &record("aaaaaaaaaaaaaaaa"),
            &[],
            &[("MILCH".into(), alias("milk"))],
            Utc::now(),
        )
        .unwrap();
        s.import_receipt(
            &record("bbbbbbbbbbbbbbbb"),
            &[],
            &[("MILCH".into(), alias("oat drink"))],
            Utc::now(),
        )
        .unwrap();
        let rewe = s.aliases("rewe").unwrap();
        assert_eq!(rewe.len(), 1);
        assert_eq!(rewe["MILCH"].item, "oat drink");
        assert!(s.aliases("lidl").unwrap().is_empty());

        let mut other = record("cccccccccccccccc");
        other.store = "lidl".into();
        s.import_receipt(&other, &[], &[("MILCH".into(), alias("milk"))], Utc::now())
            .unwrap();
        assert_eq!(s.aliases("lidl").unwrap()["MILCH"].item, "milk");
        assert_eq!(s.aliases("rewe").unwrap()["MILCH"].item, "oat drink");
    }

    #[test]
    fn a_corrupt_alias_surfaces_as_an_error() {
        let s = Store::open_in_memory().unwrap();
        s.conn
            .execute(
                "INSERT INTO aliases (store, key, body, learned_at) VALUES ('rewe', 'K', '{', 'x')",
                [],
            )
            .unwrap();
        assert!(matches!(
            s.aliases("rewe"),
            Err(StoreError::CorruptAlias { .. })
        ));
    }

    #[test]
    fn a_v1_database_migrates_keeping_its_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(DB_FILE);
        let e = acquire("rice");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO events (at, lot, kind, body) VALUES (?1, ?2, ?3, ?4)",
                params![
                    e.at.to_rfc3339(),
                    e.lot.to_string(),
                    e.kind.name(),
                    serde_json::to_string(&e).unwrap()
                ],
            )
            .unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.event_count(), 1);
        assert!(s.inventory().get(e.lot).is_some());
        let v: i64 = s
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
        assert!(MIGRATIONS.len() >= 2);
        assert!(s.aliases("rewe").unwrap().is_empty());
    }

    #[test]
    fn photo_learning_writes_events_and_aliases_together() {
        let mut s = Store::open_in_memory().unwrap();
        let alias = Alias {
            item: "pesto".into(),
            each: None,
            location: nookery_core::Location::Fridge,
        };
        let learned = vec![("contents glass green".to_string(), alias.clone())];
        s.append_learning(&[acquire("pesto")], PHOTO_ALIASES, &learned, Utc::now())
            .unwrap();
        assert_eq!(s.event_count(), 1);
        assert_eq!(
            s.aliases(PHOTO_ALIASES).unwrap()["contents glass green"],
            alias
        );
        let bad = Event {
            at: Utc::now(),
            lot: LotId::new(),
            kind: EventKind::Binned,
        };
        assert!(s
            .append_learning(&[bad], PHOTO_ALIASES, &[], Utc::now())
            .is_err());
        assert_eq!(s.event_count(), 1);
    }

    #[test]
    fn templates_start_with_defaults_and_can_be_edited() {
        let mut s = Store::open_in_memory().unwrap();
        let names: Vec<String> = s.templates().unwrap().into_iter().map(|t| t.name).collect();
        assert_eq!(names[0], "Use it up");
        assert_eq!(names.len(), 5);

        let now = Utc::now();
        let t = s
            .save_template(None, " Indian ", "North Indian, mild.", now)
            .unwrap();
        assert_eq!(t.name, "Indian");
        assert_eq!(s.templates().unwrap().last().unwrap().id, t.id);

        let edited = s
            .save_template(Some(t.id), "Indian", "South Indian.", now)
            .unwrap();
        assert_eq!(edited.body, "South Indian.");
        assert!(matches!(
            s.save_template(None, "Indian", "again", now),
            Err(StoreError::DuplicateTemplate(_))
        ));
        assert!(matches!(
            s.save_template(None, "x", "  ", now),
            Err(StoreError::EmptyTemplate)
        ));
        assert!(matches!(
            s.save_template(Some(999), "y", "z", now),
            Err(StoreError::NoTemplate(999))
        ));

        let mut ids: Vec<i64> = s.templates().unwrap().iter().map(|t| t.id).collect();
        ids.reverse();
        s.order_templates(&ids).unwrap();
        assert_eq!(s.templates().unwrap()[0].name, "Indian");
        assert!(s.order_templates(&ids[1..]).is_err());

        s.delete_template(t.id).unwrap();
        assert_eq!(s.templates().unwrap().len(), 5);
        assert!(matches!(
            s.delete_template(t.id),
            Err(StoreError::NoTemplate(_))
        ));
    }

    #[test]
    fn ai_settings_default_to_local_and_can_be_changed() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.ai_mode(SETTING_PHOTOS_AI).unwrap(), AiMode::Local);
        assert_eq!(s.cloud_model().unwrap(), CLOUD_MODEL);
        s.set_setting(SETTING_PHOTOS_AI, "cloud").unwrap();
        s.set_setting(SETTING_RECIPES_AI, "nonsense").unwrap();
        assert_eq!(s.ai_mode(SETTING_PHOTOS_AI).unwrap(), AiMode::Cloud);
        assert_eq!(s.ai_mode(SETTING_RECIPES_AI).unwrap(), AiMode::Local);
        s.set_setting(SETTING_PHOTOS_AI, "fallback").unwrap();
        assert_eq!(s.ai_mode(SETTING_PHOTOS_AI).unwrap(), AiMode::Fallback);
        s.set_setting(SETTING_CLOUD_MODEL, "gpt-oss:20b-cloud")
            .unwrap();
        assert_eq!(s.cloud_model().unwrap(), "gpt-oss:20b-cloud");
    }
}
