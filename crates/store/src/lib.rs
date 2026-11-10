//! SQLite storage for the ledger. Only events are stored; the inventory is
//! replayed from them in memory on open (a few thousand events replay in
//! well under a millisecond) and kept up to date as events are appended.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use nookery_core::{AiMode, Alias, Event, Inventory, LedgerError, LotId, CLOUD_MODEL};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

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
    #[error("stored doc {kind}/{id} can't be read: {source}")]
    CorruptDoc {
        kind: String,
        id: String,
        source: serde_json::Error,
    },
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
    // 5: sync between devices (the Mac and the phone). Every event gets a
    // stable `uid` (existing rows: random hex; new rows: UUID v7) so a
    // device can tell which events it already has. `docs` holds small
    // synced documents that are not inventory (favourite recipes, weekly
    // plans, shopping items): last writer wins by `updated_at`, deletions
    // are kept as tombstones so they sync too. `body` is JSON.
    "ALTER TABLE events ADD COLUMN uid TEXT;
    UPDATE events SET uid = lower(hex(randomblob(16))) WHERE uid IS NULL;
    CREATE UNIQUE INDEX events_uid ON events(uid);
    CREATE TABLE docs (
        kind       TEXT NOT NULL,
        id         TEXT NOT NULL,
        body       TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        deleted    INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (kind, id)
    );",
];

/// `docs` kinds. A favourite recipe (body: the recipe plus how to draw it).
pub const DOC_FAVOURITE: &str = "favourite";
/// A weekly plan (id: the Monday, `YYYY-MM-DD`).
pub const DOC_PLAN: &str = "plan";
/// One shopping-list item.
pub const DOC_SHOPPING: &str = "shopping";

/// A small synced document (see migration 5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub kind: String,
    pub id: String,
    pub body: serde_json::Value,
    pub updated_at: DateTime<Utc>,
    /// A tombstone: deleted on some device, kept so the deletion syncs.
    #[serde(default)]
    pub deleted: bool,
}

/// An event with its sync identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncedEvent {
    /// This database's order (differs between devices).
    pub seq: i64,
    /// The same on every device.
    pub uid: String,
    pub event: Event,
}

/// What `merge_events` did with each incoming event (by uid).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeReport {
    pub accepted: Vec<String>,
    /// Already here (a retried sync).
    pub duplicate: Vec<String>,
    /// Not valid against this ledger (e.g. the lot was used up here
    /// meanwhile): `(uid, reason)`. Nothing of it was written.
    pub rejected: Vec<(String, String)>,
}

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
    /// The highest event `seq` this inventory includes. The app and the CLI
    /// share the database: when another process appended, the in-memory
    /// inventory is stale, and validating against it could write an event
    /// on a lot that is already gone (which would make the ledger refuse to
    /// replay). Writes re-check it inside their transaction.
    last_seq: i64,
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
            last_seq: 0,
        };
        store.reload()?;
        Ok(store)
    }

    /// Re-reads every event and rebuilds the inventory.
    pub fn reload(&mut self) -> Result<(), StoreError> {
        let (events, last) = read_events(&self.conn)?;
        self.inventory = Inventory::replay(&events)?;
        self.events = events.len();
        self.last_seq = last;
        Ok(())
    }

    /// Reloads if another process (the CLI, the app) appended since; true
    /// if it did. One indexed query, so callers can do it before every read.
    pub fn refresh(&mut self) -> Result<bool, StoreError> {
        if max_seq(&self.conn)? == self.last_seq {
            return Ok(false);
        }
        self.reload()?;
        Ok(true)
    }

    /// Inside a write transaction (which holds SQLite's write lock): the
    /// inventory as of the latest event, re-read if another process wrote.
    fn current(&self, tx: &Transaction) -> Result<(Inventory, usize), StoreError> {
        if max_seq(tx)? == self.last_seq {
            return Ok((self.inventory.clone(), self.events));
        }
        let (events, _) = read_events(tx)?;
        Ok((Inventory::replay(&events)?, events.len()))
    }

    /// Validates `events` against the current inventory and inserts them,
    /// inside `tx`; returns the new inventory and event count.
    fn validate_insert(
        &self,
        tx: &Transaction,
        events: &[Event],
    ) -> Result<(Inventory, usize), StoreError> {
        let (mut next, count) = self.current(tx)?;
        for e in events {
            next.apply(e)?;
        }
        insert_events(tx, events)?;
        Ok((next, count + events.len()))
    }

    fn committed(&mut self, next: Inventory, count: usize) -> Result<(), StoreError> {
        self.inventory = next;
        self.events = count;
        self.last_seq = max_seq(&self.conn)?;
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
        Ok(read_events(&self.conn)?.0)
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
        let (next, count) = {
            let tx = self.write_tx()?;
            let r = self.validate_insert(&tx, events)?;
            tx.commit()?;
            r
        };
        self.committed(next, count)
    }

    /// `BEGIN IMMEDIATE`: takes the write lock at once, so nothing can be
    /// appended between checking the latest event and inserting.
    fn write_tx(&self) -> Result<Transaction<'_>, StoreError> {
        Ok(Transaction::new_unchecked(
            &self.conn,
            TransactionBehavior::Immediate,
        )?)
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
        let (next, count) = {
            let tx = self.write_tx()?;
            let r = self.validate_insert(&tx, events)?;
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
            upsert_aliases(&tx, &receipt.store, learned, at)?;
            tx.commit()?;
            r
        };
        self.committed(next, count)
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
        let (next, count) = {
            let tx = self.write_tx()?;
            let r = self.validate_insert(&tx, events)?;
            upsert_aliases(&tx, store, learned, at)?;
            tx.commit()?;
            r
        };
        self.committed(next, count)
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

    // ---- sync (migration 5) ----

    /// The highest event `seq` in this database (0 when empty).
    pub fn last_seq(&self) -> Result<i64, StoreError> {
        max_seq(&self.conn)
    }

    /// Events with `seq > after`, oldest first, with their uids.
    pub fn synced_events_after(&self, after: i64) -> Result<Vec<SyncedEvent>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT seq, uid, body FROM events WHERE seq > ?1 ORDER BY seq")?;
        let rows = stmt.query_map([after], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (seq, uid, body) = row?;
            let event = serde_json::from_str(&body)
                .map_err(|source| StoreError::Corrupt { seq, source })?;
            Ok(SyncedEvent { seq, uid, event })
        })
        .collect()
    }

    /// Appends events from another device, each judged on its own, in one
    /// transaction: a uid already here is skipped, an event that isn't valid
    /// against this ledger is rejected (nothing of it written), the rest are
    /// appended in the given order with their uids.
    pub fn merge_events(
        &mut self,
        incoming: &[(String, Event)],
    ) -> Result<MergeReport, StoreError> {
        let mut report = MergeReport::default();
        let (next, count) = {
            let tx = self.write_tx()?;
            let (mut next, mut count) = self.current(&tx)?;
            for (uid, e) in incoming {
                let known: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM events WHERE uid = ?1)",
                    [uid],
                    |r| r.get(0),
                )?;
                if known {
                    report.duplicate.push(uid.clone());
                    continue;
                }
                let mut trial = next.clone();
                match trial.apply(e) {
                    Ok(()) => {
                        insert_event(&tx, uid, e)?;
                        next = trial;
                        count += 1;
                        report.accepted.push(uid.clone());
                    }
                    Err(err) => report.rejected.push((uid.clone(), err.to_string())),
                }
            }
            tx.commit()?;
            (next, count)
        };
        self.committed(next, count)?;
        Ok(report)
    }

    /// Replaces the whole ledger with another device's copy (the phone
    /// taking the Mac's), uids kept, in one transaction. Refuses (writing
    /// nothing) if it doesn't replay.
    pub fn replace_ledger(&mut self, events: &[(String, Event)]) -> Result<(), StoreError> {
        let list: Vec<Event> = events.iter().map(|(_, e)| e.clone()).collect();
        let next = Inventory::replay(&list)?;
        {
            let tx = self.write_tx()?;
            tx.execute("DELETE FROM events", [])?;
            for (uid, e) in events {
                insert_event(&tx, uid, e)?;
            }
            tx.commit()?;
        }
        self.committed(next, events.len())
    }

    /// The live (not deleted) docs of one kind, by id.
    pub fn docs(&self, kind: &str) -> Result<Vec<Doc>, StoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT kind, id, body, updated_at, deleted FROM docs
             WHERE kind = ?1 AND deleted = 0 ORDER BY id",
        )?;
        let rows = stmt.query_map([kind], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.map(|row| {
            let (k, i, b, u, d) = row?;
            read_doc(k, i, b, u, d)
        })
        .collect()
    }

    /// One doc, tombstones included.
    pub fn doc(&self, kind: &str, id: &str) -> Result<Option<Doc>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT kind, id, body, updated_at, deleted FROM docs WHERE kind = ?1 AND id = ?2",
                [kind, id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, bool>(4)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(k, i, b, u, d)| read_doc(k, i, b, u, d))
            .transpose()
    }

    /// Creates or replaces a doc (and revives a deleted one).
    pub fn put_doc(
        &mut self,
        kind: &str,
        id: &str,
        body: &serde_json::Value,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.write_doc(&Doc {
            kind: kind.into(),
            id: id.into(),
            body: body.clone(),
            updated_at: at,
            deleted: false,
        })
    }

    /// Deletes a doc, leaving a tombstone (body kept) so the deletion syncs.
    /// Unknown ids are a no-op.
    pub fn delete_doc(
        &mut self,
        kind: &str,
        id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE docs SET deleted = 1, updated_at = ?3 WHERE kind = ?1 AND id = ?2",
            params![kind, id, at.to_rfc3339()],
        )?;
        Ok(())
    }

    fn write_doc(&self, d: &Doc) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO docs (kind, id, body, updated_at, deleted) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(kind, id) DO UPDATE SET body = excluded.body,
               updated_at = excluded.updated_at, deleted = excluded.deleted",
            params![
                d.kind,
                d.id,
                d.body.to_string(),
                d.updated_at.to_rfc3339(),
                d.deleted
            ],
        )?;
        Ok(())
    }

    /// Docs (tombstones included) changed after `since` (all when None),
    /// oldest change first: what a sync sends.
    pub fn docs_changed_since(&self, since: Option<DateTime<Utc>>) -> Result<Vec<Doc>, StoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT kind, id, body, updated_at, deleted FROM docs ORDER BY updated_at, kind, id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (k, i, b, u, d) = row?;
            let doc = read_doc(k, i, b, u, d)?;
            if since.is_none_or(|t| doc.updated_at > t) {
                out.push(doc);
            }
        }
        Ok(out)
    }

    /// Takes docs from another device where they are newer (last writer
    /// wins, see `newer`); returns how many changed here.
    pub fn merge_docs(&mut self, incoming: &[Doc]) -> Result<usize, StoreError> {
        let mut changed = 0;
        let tx = self.write_tx()?;
        for d in incoming {
            let take = match self.doc(&d.kind, &d.id)? {
                Some(current) => newer(d, &current),
                None => true,
            };
            if take {
                self.write_doc(d)?;
                changed += 1;
            }
        }
        tx.commit()?;
        Ok(changed)
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

/// Every event, oldest first, and the highest `seq` (0 when empty).
fn read_events(conn: &Connection) -> Result<(Vec<Event>, i64), StoreError> {
    let mut stmt = conn.prepare_cached("SELECT seq, body FROM events ORDER BY seq")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    let mut events = Vec::new();
    let mut last = 0;
    for row in rows {
        let (seq, body) = row?;
        events.push(
            serde_json::from_str(&body).map_err(|source| StoreError::Corrupt { seq, source })?,
        );
        last = seq;
    }
    Ok((events, last))
}

fn max_seq(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM events", [], |r| r.get(0))?)
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
    for e in events {
        insert_event(tx, &new_uid(), e)?;
    }
    Ok(())
}

/// A new event uid: UUID v7 (time-ordered), simple hex.
fn new_uid() -> String {
    uuid::Uuid::now_v7().simple().to_string()
}

fn insert_event(tx: &Transaction, uid: &str, e: &Event) -> Result<(), StoreError> {
    let mut stmt = tx.prepare_cached(
        "INSERT INTO events (at, lot, kind, body, uid) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let body = serde_json::to_string(e).expect("events always serialise");
    stmt.execute(params![
        e.at.to_rfc3339(),
        e.lot.to_string(),
        e.kind.name(),
        body,
        uid
    ])?;
    Ok(())
}

fn read_doc(
    kind: String,
    id: String,
    body: String,
    updated_at: String,
    deleted: bool,
) -> Result<Doc, StoreError> {
    let body = serde_json::from_str(&body).map_err(|source| StoreError::CorruptDoc {
        kind: kind.clone(),
        id: id.clone(),
        source,
    })?;
    let updated_at = DateTime::parse_from_rfc3339(&updated_at)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_default();
    Ok(Doc {
        kind,
        id,
        body,
        updated_at,
        deleted,
    })
}

/// Last writer wins; on the same instant a deletion wins, then the larger
/// body (so every device picks the same one).
fn newer(incoming: &Doc, current: &Doc) -> bool {
    (
        incoming.updated_at,
        incoming.deleted,
        incoming.body.to_string(),
    ) > (
        current.updated_at,
        current.deleted,
        current.body.to_string(),
    )
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

    #[test]
    fn a_second_process_writing_is_seen_before_reading_and_writing() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Store::open_dir(dir.path()).unwrap();
        let milk = acquire("milk");
        let id = milk.lot;
        app.append(&[milk]).unwrap();

        // The CLI, a separate connection, bins it.
        let mut cli = Store::open_dir(dir.path()).unwrap();
        cli.append(&[Event {
            at: Utc::now(),
            lot: id,
            kind: EventKind::Binned,
        }])
        .unwrap();

        // The app's in-memory view is stale, but writing re-checks: using
        // a binned lot is refused, and nothing is written.
        assert!(app.inventory().get(id).unwrap().in_stock());
        let used = Event {
            at: Utc::now(),
            lot: id,
            kind: EventKind::Used { qty: None },
        };
        assert!(matches!(app.append(&[used]), Err(StoreError::Ledger(_))));
        assert_eq!(Store::open_dir(dir.path()).unwrap().event_count(), 2);

        // And refresh picks the change up for reads.
        let mut app2 = Store::open_dir(dir.path()).unwrap();
        assert!(!app2.refresh().unwrap());
        cli.append(&[acquire("bread")]).unwrap();
        assert!(app2.refresh().unwrap());
        assert_eq!(app2.event_count(), 3);
    }

    #[test]
    fn events_have_uids_and_another_device_can_merge_them() {
        let mut mac = Store::open_in_memory().unwrap();
        let mut phone = Store::open_in_memory().unwrap();
        let milk = acquire("milk");
        mac.append(std::slice::from_ref(&milk)).unwrap();
        let from_mac: Vec<(String, Event)> = mac
            .synced_events_after(0)
            .unwrap()
            .into_iter()
            .map(|e| (e.uid, e.event))
            .collect();
        assert_eq!(from_mac.len(), 1);
        assert_eq!(from_mac[0].0.len(), 32);
        phone.replace_ledger(&from_mac).unwrap();
        assert_eq!(phone.inventory().all().len(), 1);

        // The phone uses the milk; the Mac (meanwhile) bins it: the phone's
        // event arrives second and is rejected, a retry is a duplicate.
        let used = Event {
            at: Utc::now(),
            lot: milk.lot,
            kind: EventKind::Used { qty: None },
        };
        phone.append(std::slice::from_ref(&used)).unwrap();
        let binned = Event {
            at: Utc::now(),
            lot: milk.lot,
            kind: EventKind::Binned,
        };
        mac.append(&[binned]).unwrap();
        let after = phone.synced_events_after(1).unwrap();
        let push: Vec<(String, Event)> = after.into_iter().map(|e| (e.uid, e.event)).collect();
        let r = mac.merge_events(&push).unwrap();
        assert!(r.accepted.is_empty());
        assert_eq!(r.rejected.len(), 1);
        assert_eq!(mac.event_count(), 2);

        // A fresh event merges; sending it again is a duplicate.
        let eggs = acquire("eggs");
        let push = vec![("abc".to_string(), eggs)];
        assert_eq!(mac.merge_events(&push).unwrap().accepted, vec!["abc"]);
        assert_eq!(mac.merge_events(&push).unwrap().duplicate, vec!["abc"]);
        assert_eq!(mac.event_count(), 3);
        let all: Vec<(String, Event)> = mac
            .synced_events_after(0)
            .unwrap()
            .into_iter()
            .map(|e| (e.uid, e.event))
            .collect();
        phone.replace_ledger(&all).unwrap();
        assert_eq!(phone.event_count(), 3);
    }

    #[test]
    fn a_ledger_that_does_not_replay_replaces_nothing() {
        let mut s = Store::open_in_memory().unwrap();
        s.append(&[acquire("milk")]).unwrap();
        let orphan = Event {
            at: Utc::now(),
            lot: LotId::new(),
            kind: EventKind::Opened,
        };
        assert!(s.replace_ledger(&[("x".into(), orphan)]).is_err());
        assert_eq!(s.event_count(), 1);
    }

    #[test]
    fn docs_sync_last_writer_wins_with_tombstones() {
        let mut a = Store::open_in_memory().unwrap();
        let mut b = Store::open_in_memory().unwrap();
        let t1: DateTime<Utc> = "2026-11-09T10:00:00Z".parse().unwrap();
        let t2: DateTime<Utc> = "2026-11-09T11:00:00Z".parse().unwrap();
        let t3: DateTime<Utc> = "2026-11-09T12:00:00Z".parse().unwrap();
        a.put_doc(DOC_SHOPPING, "1", &serde_json::json!({"item": "milk"}), t1)
            .unwrap();
        a.put_doc(DOC_SHOPPING, "2", &serde_json::json!({"item": "eggs"}), t1)
            .unwrap();
        assert_eq!(
            b.merge_docs(&a.docs_changed_since(None).unwrap()).unwrap(),
            2
        );
        // b ticks milk later; a deletes eggs later still.
        b.put_doc(
            DOC_SHOPPING,
            "1",
            &serde_json::json!({"item": "milk", "done": true}),
            t2,
        )
        .unwrap();
        a.delete_doc(DOC_SHOPPING, "2", t3).unwrap();
        assert_eq!(a.docs(DOC_SHOPPING).unwrap().len(), 1);
        let from_a = a.docs_changed_since(Some(t1)).unwrap();
        assert_eq!(from_a.len(), 1);
        b.merge_docs(&from_a).unwrap();
        a.merge_docs(&b.docs_changed_since(Some(t1)).unwrap())
            .unwrap();
        for s in [&a, &b] {
            let live = s.docs(DOC_SHOPPING).unwrap();
            assert_eq!(live.len(), 1);
            assert_eq!(live[0].body["done"], true);
            assert!(s.doc(DOC_SHOPPING, "2").unwrap().unwrap().deleted);
        }
        // An older copy doesn't overwrite.
        let stale = Doc {
            kind: DOC_SHOPPING.into(),
            id: "1".into(),
            body: serde_json::json!({"item": "milk"}),
            updated_at: t1,
            deleted: false,
        };
        assert_eq!(a.merge_docs(&[stale]).unwrap(), 0);
    }

    #[test]
    fn a_v4_database_gets_uids_on_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("n.db");
        {
            let mut s = Store::open(&path).unwrap();
            s.append(&[acquire("milk"), acquire("eggs")]).unwrap();
            s.conn
                .execute_batch("DROP INDEX events_uid; DROP TABLE docs;")
                .unwrap();
            s.conn.execute("UPDATE events SET uid = NULL", []).unwrap();
            s.conn.pragma_update(None, "user_version", 4).unwrap();
        }
        // SQLite can't drop a column easily; re-running migration 5 on a
        // table that has the column would fail, so emulate the old schema
        // by rebuilding the table without it.
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(
                "CREATE TABLE e2 AS SELECT seq, at, lot, kind, body FROM events;
                 DROP TABLE events;
                 CREATE TABLE events (seq INTEGER PRIMARY KEY, at TEXT NOT NULL,
                   lot TEXT NOT NULL, kind TEXT NOT NULL, body TEXT NOT NULL);
                 INSERT INTO events SELECT * FROM e2; DROP TABLE e2;",
            )
            .unwrap();
        }
        let s = Store::open(&path).unwrap();
        let ev = s.synced_events_after(0).unwrap();
        assert_eq!(ev.len(), 2);
        assert_ne!(ev[0].uid, ev[1].uid);
    }
}
