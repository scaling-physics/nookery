//! Mac <-> phone sync over the home network. See AGENTS.md.
//!
//! The Mac is the hub (`start` a server over a `Provider`), the phone is the
//! client (`connect`, then `sync_now`, `download_model`, `MacText`). Both
//! share a 128-bit pairing secret typed in once as a code. Every connection
//! derives fresh keys from it and two random nonces; everything after the
//! nonces is XChaCha20-Poly1305 encrypted with a message counter in the
//! associated data (no replay, drop or reorder). Nothing here talks to the
//! internet; `Generate` uses the Mac's local Ollama only.

mod channel;
mod client;
mod secret;
mod server;

pub use client::{
    connect, download_model, download_model_cancellable, finish, prepare, sync_now, MacText,
    Session, SyncOutcome, SyncRequest,
};
pub use secret::Secret;
pub use server::{
    default_models_dir, scan_with, serve_sync, start, Provider, Server, StoreProvider,
    SETTING_SYNC_SECRET,
};

use chrono::{DateTime, Utc};
use nookery_core::Event;
use nookery_store::{Doc, MergeReport};
use serde::{Deserialize, Serialize};

/// Where the Mac listens (the next few ports are tried if it is taken).
pub const DEFAULT_PORT: u16 = 47480;
/// Bumped when the protocol changes incompatibly.
pub const PROTOCOL: u32 = 1;
/// The only models the Mac hands out (what the phone runs, ~2.5 GB).
pub const PHONE_MODELS: &[&str] = &["qwen3:4b-instruct"];
/// Largest piece of a model file in one frame.
pub const MAX_CHUNK: u64 = 4 * 1024 * 1024;

/// Settings keys (in the store's `settings` table).
pub const SETTING_PUSHED_SEQ: &str = "sync.pushed_seq";
pub const SETTING_DOCS_SINCE: &str = "sync.docs_since";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Request {
    Hello {
        device: String,
        version: u32,
    },
    Sync {
        events: Vec<(String, Event)>,
        docs: Vec<Doc>,
        docs_since: Option<DateTime<Utc>>,
    },
    ModelInfo {
        name: String,
    },
    ModelChunk {
        name: String,
        offset: u64,
        len: u64,
    },
    Generate {
        prompt: String,
        max_tokens: u32,
    },
    /// Read a photo of `place` with the Mac's vision model. `image` is a
    /// JPEG (resized on the phone to at most 2000 px on the long side),
    /// base64 in the JSON.
    ScanPhoto {
        place: String,
        image: String,
    },
}

/// What the Mac's scan found, and which model said so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanAnswer {
    pub scan: nookery_vision::Scan,
    /// "ollama qwen3-vl:8b-instruct (this Mac)".
    pub answered_by: String,
    pub used_cloud: bool,
}

/// What the Mac answers a `Sync` with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncReply {
    pub report: MergeReport,
    /// The Mac's whole ledger, with uids.
    pub ledger: Vec<(String, Event)>,
    /// The Mac's docs changed since the phone's `docs_since`.
    pub docs: Vec<Doc>,
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Welcome {
        device: String,
        server_time: DateTime<Utc>,
    },
    Synced(SyncReply),
    Model {
        name: String,
        bytes: u64,
        sha256: String,
    },
    NoModel,
    Text(String),
    Done {
        model: String,
    },
    /// One tile of a photo scan is done.
    ScanProgress {
        done: u32,
        total: u32,
    },
    Scanned(ScanAnswer),
    Error(String),
}

/// Errors carry no data from the peer except `Peer`, which is a message the
/// other app wrote for people; all may be shown in the UI.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("could not reach the other device")]
    Connect,
    #[error("the other device did not accept the pairing code")]
    Auth,
    #[error("the connection was lost or timed out")]
    Io,
    #[error("unexpected reply from the other device")]
    Protocol,
    #[error("{0}")]
    Peer(String),
    #[error("the downloaded file is damaged (checksum mismatch)")]
    BadHash,
    #[error("the other device does not have that model")]
    NoModel,
    #[error("{0}")]
    Store(String),
    /// A download was stopped by the user; what was fetched is kept.
    #[error("the download was stopped")]
    Cancelled,
    /// The phone's ledger changed while the Mac was answering; try again.
    #[error("the kitchen changed during the sync; trying again is safe")]
    Changed,
}

impl From<nookery_store::StoreError> for SyncError {
    fn from(e: nookery_store::StoreError) -> Self {
        SyncError::Store(e.to_string())
    }
}

/// For the replay test only.
#[doc(hidden)]
pub mod __test {
    use crate::{channel, Secret};
    pub fn keys(s: &Secret, c: &[u8; 24], sv: &[u8; 24]) -> ([u8; 32], [u8; 32]) {
        let (a, b) = channel::session_keys(s, c, sv);
        (*a, *b)
    }
    pub fn seal(key: &[u8; 32], counter: u64, plain: &[u8]) -> Vec<u8> {
        let k = zeroize::Zeroizing::new(*key);
        channel::seal(&k, channel::Role::Client, counter, plain).unwrap()
    }
}
