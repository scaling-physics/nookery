use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, Utc};
use nookery_recipes::{RecipeError, TextModel};
use nookery_store::Store;
use sha2::{Digest, Sha256};

use crate::channel::{seal_first, session_keys, write_frame, Channel, Role, MAGIC, NONCE_LEN};
use crate::secret::Secret;
use crate::server::{send_json, KIND_BYTES, KIND_JSON};
use crate::{
    Request, Response, SyncError, SyncReply, DEFAULT_PORT, MAX_CHUNK, PROTOCOL, SETTING_DOCS_SINCE,
    SETTING_PUSHED_SEQ,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
const IO_TIMEOUT: Duration = Duration::from_secs(120);

/// An authenticated connection to the Mac.
pub struct Session {
    chan: Channel<TcpStream>,
}

/// Connects to `address` (`host` or `host:port`) and proves knowledge of the
/// pairing secret.
pub fn connect(address: &str, secret: &Secret) -> Result<Session, SyncError> {
    let address = address.trim();
    let with_port = if address.contains(':') {
        address.to_string()
    } else {
        format!("{address}:{DEFAULT_PORT}")
    };
    let addr = with_port
        .to_socket_addrs()
        .map_err(|_| SyncError::Connect)?
        .next()
        .ok_or(SyncError::Connect)?;
    let mut stream =
        TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|_| SyncError::Connect)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| SyncError::Io)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| SyncError::Io)?;

    let mut client_nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut client_nonce).map_err(|_| SyncError::Io)?;
    let mut head = Vec::with_capacity(MAGIC.len() + NONCE_LEN);
    head.extend_from_slice(MAGIC);
    head.extend_from_slice(&client_nonce);
    stream.write_all(&head).map_err(|_| SyncError::Io)?;
    let mut server_nonce = [0u8; NONCE_LEN];
    stream
        .read_exact(&mut server_nonce)
        .map_err(|_| SyncError::Connect)?;

    let (c2s, s2c) = session_keys(secret, &client_nonce, &server_nonce);
    let hello = seal_first(&c2s, Role::Client, b"hello")?;
    write_frame(&mut stream, &hello)?;
    let mut chan = Channel::new(stream, Role::Client, c2s, s2c, 1, 0);
    // A server that does not know the secret just closes the connection.
    match chan.recv() {
        Ok(ok) if ok.as_slice() == b"ok" => Ok(Session { chan }),
        Ok(_) => Err(SyncError::Protocol),
        Err(SyncError::Io) => Err(SyncError::Auth),
        Err(e) => Err(e),
    }
}

/// What a sync did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncOutcome {
    /// Phone events the Mac took.
    pub pushed: usize,
    /// Phone events the Mac refused (uid, why): dropped, the phone now
    /// matches the Mac.
    pub rejected: Vec<(String, String)>,
    /// Events in the Mac's ledger (the phone's whole ledger afterwards).
    pub pulled_events: usize,
    pub docs_changed: usize,
}

impl Session {
    fn recv_json(&mut self) -> Result<Response, SyncError> {
        let msg = self.chan.recv()?;
        match msg.split_first() {
            Some((&KIND_JSON, body)) => {
                serde_json::from_slice(body).map_err(|_| SyncError::Protocol)
            }
            _ => Err(SyncError::Protocol),
        }
    }

    fn call(&mut self, req: &Request) -> Result<Response, SyncError> {
        send_json(&mut self.chan, req)?;
        match self.recv_json()? {
            Response::Error(e) => Err(SyncError::Peer(e)),
            r => Ok(r),
        }
    }

    /// Checks the pairing and the version; returns the Mac's name and clock.
    pub fn hello(&mut self, device: &str) -> Result<(String, DateTime<Utc>), SyncError> {
        match self.call(&Request::Hello {
            device: device.to_string(),
            version: PROTOCOL,
        })? {
            Response::Welcome {
                device,
                server_time,
            } => Ok((device, server_time)),
            _ => Err(SyncError::Protocol),
        }
    }

    /// The raw exchange (see `prepare`/`finish` to keep the store unlocked
    /// while this runs).
    pub fn exchange(&mut self, req: &SyncRequest) -> Result<SyncReply, SyncError> {
        match self.call(&Request::Sync {
            events: req.events.clone(),
            docs: req.docs.clone(),
            docs_since: req.docs_since,
        })? {
            Response::Synced(r) => Ok(r),
            _ => Err(SyncError::Protocol),
        }
    }

    /// `(bytes, sha256)` of a model the Mac serves.
    pub fn model_info(&mut self, name: &str) -> Result<(u64, String), SyncError> {
        match self.call(&Request::ModelInfo { name: name.into() })? {
            Response::Model { bytes, sha256, .. } => Ok((bytes, sha256)),
            Response::NoModel => Err(SyncError::NoModel),
            _ => Err(SyncError::Protocol),
        }
    }

    pub fn model_chunk(&mut self, name: &str, offset: u64, len: u64) -> Result<Vec<u8>, SyncError> {
        send_json(
            &mut self.chan,
            &Request::ModelChunk {
                name: name.into(),
                offset,
                len: len.min(MAX_CHUNK),
            },
        )?;
        let msg = self.chan.recv()?;
        match msg.split_first() {
            Some((&KIND_BYTES, body)) => Ok(body.to_vec()),
            Some((&KIND_JSON, _)) => Err(SyncError::NoModel),
            _ => Err(SyncError::Protocol),
        }
    }

    /// Has the Mac read a photo (JPEG bytes) of `place`. `on_progress(done,
    /// total)` is called per tile.
    pub fn scan_photo(
        &mut self,
        place: nookery_core::Location,
        jpeg: &[u8],
        on_progress: &mut dyn FnMut(u32, u32),
    ) -> Result<crate::ScanAnswer, SyncError> {
        use base64::Engine;
        send_json(
            &mut self.chan,
            &Request::ScanPhoto {
                place: place.as_str().into(),
                image: base64::engine::general_purpose::STANDARD.encode(jpeg),
            },
        )?;
        loop {
            match self.recv_json()? {
                Response::ScanProgress { done, total } => on_progress(done, total),
                Response::Scanned(a) => return Ok(a),
                Response::Error(e) => return Err(SyncError::Peer(e)),
                _ => return Err(SyncError::Protocol),
            }
        }
    }

    /// Has the Mac read a receipt file (PDF, PNG, JPEG, HEIC; at most
    /// `nookery_extract::MAX_BYTES`) into text.
    pub fn read_receipt(&mut self, file: &[u8]) -> Result<nookery_extract::Extracted, SyncError> {
        use base64::Engine;
        send_json(
            &mut self.chan,
            &Request::ReadReceipt {
                file: base64::engine::general_purpose::STANDARD.encode(file),
            },
        )?;
        match self.recv_json()? {
            Response::ReceiptText(x) => Ok(x),
            Response::Error(e) => Err(SyncError::Peer(e)),
            _ => Err(SyncError::Protocol),
        }
    }

    /// Runs a prompt on the Mac's local model, streaming the text.
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<(String, String), SyncError> {
        send_json(
            &mut self.chan,
            &Request::Generate {
                prompt: prompt.into(),
                max_tokens,
            },
        )?;
        let mut all = String::new();
        loop {
            match self.recv_json()? {
                Response::Text(t) => {
                    on_text(&t);
                    all.push_str(&t);
                }
                Response::Done { model } => return Ok((all, model)),
                Response::Error(e) => return Err(SyncError::Peer(e)),
                _ => return Err(SyncError::Protocol),
            }
        }
    }
}

/// What the phone sends; built under the store lock by `prepare`.
#[derive(Debug, Clone)]
pub struct SyncRequest {
    events: Vec<(String, nookery_core::Event)>,
    docs: Vec<nookery_store::Doc>,
    docs_since: Option<DateTime<Utc>>,
    /// The ledger position this was built at.
    at_seq: i64,
}

impl SyncRequest {
    /// The phone's own events being sent.
    pub fn events(&self) -> &[(String, nookery_core::Event)] {
        &self.events
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

fn setting_num(store: &Store, key: &str) -> i64 {
    store
        .setting(key)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Step 1 (store locked briefly): the phone's own events since the last
/// sync and the docs changed since then.
pub fn prepare(store: &Store) -> Result<SyncRequest, SyncError> {
    let pushed = setting_num(store, SETTING_PUSHED_SEQ);
    let docs_since = store
        .setting(SETTING_DOCS_SINCE)?
        .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
        .map(|t| t.with_timezone(&Utc));
    Ok(SyncRequest {
        events: store
            .synced_events_after(pushed)?
            .into_iter()
            .map(|s| (s.uid, s.event))
            .collect(),
        docs: store.docs_changed_since(docs_since)?,
        docs_since,
        at_seq: store.last_seq()?,
    })
}

/// Step 3 (store locked): take the Mac's ledger and docs. If the phone's
/// ledger moved since `prepare`, nothing is written and `Changed` is
/// returned (start again; replacing the ledger would lose the new events).
pub fn finish(
    store: &mut Store,
    req: &SyncRequest,
    reply: SyncReply,
) -> Result<SyncOutcome, SyncError> {
    if store.last_seq()? != req.at_seq {
        return Err(SyncError::Changed);
    }
    let pulled_events = reply.ledger.len();
    store.replace_ledger(&reply.ledger)?;
    let docs_changed = store.merge_docs(&reply.docs)?;
    store.set_setting(SETTING_PUSHED_SEQ, &store.last_seq()?.to_string())?;
    store.set_setting(SETTING_DOCS_SINCE, &reply.server_time.to_rfc3339())?;
    Ok(SyncOutcome {
        pushed: reply.report.accepted.len(),
        rejected: reply.report.rejected,
        pulled_events,
        docs_changed,
    })
}

/// A whole sync with an exclusively held store (the CLI, tests). The app
/// uses `prepare` / `Session::exchange` / `finish` to unlock in between.
pub fn sync_now(store: &mut Store, peer: &mut Session) -> Result<SyncOutcome, SyncError> {
    for _ in 0..3 {
        let req = prepare(store)?;
        let reply = peer.exchange(&req)?;
        match finish(store, &req, reply) {
            Err(SyncError::Changed) => continue,
            other => return other,
        }
    }
    Err(SyncError::Changed)
}

/// The Mac's model, as a `TextModel` (one connection per call).
pub struct MacText {
    pub address: String,
    pub secret: Secret,
    /// Filled from the Mac's answer ("ollama ...").
    answered: std::sync::Mutex<Option<String>>,
}

impl MacText {
    pub fn new(address: &str, secret: Secret) -> Self {
        Self {
            address: address.into(),
            secret,
            answered: Default::default(),
        }
    }
}

impl TextModel for MacText {
    fn name(&self) -> String {
        match self.answered.lock().ok().and_then(|a| a.clone()) {
            Some(m) => format!("your Mac ({m})"),
            None => "your Mac".into(),
        }
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        let mut peer = connect(&self.address, &self.secret)
            .map_err(|_| RecipeError::Unreachable(self.address.clone()))?;
        let (text, model) = peer
            .generate(prompt, max_tokens, on_text)
            .map_err(|e| RecipeError::Model(e.to_string()))?;
        if let Ok(mut a) = self.answered.lock() {
            *a = Some(model);
        }
        Ok(text)
    }
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Downloads a model from the Mac to `dest`, resuming `<dest>.part`,
/// verifying the checksum before renaming. `progress(done, total)`.
pub fn download_model(
    peer: &mut Session,
    name: &str,
    dest: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), SyncError> {
    download_model_cancellable(peer, name, dest, progress, &AtomicBool::new(false))
}

/// `download_model`, stopping with `SyncError::Cancelled` (the `.part` file
/// stays, so a later download resumes) once `cancel` is set.
pub fn download_model_cancellable(
    peer: &mut Session,
    name: &str,
    dest: &Path,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), SyncError> {
    let io = |_| SyncError::Io;
    let (total, sha) = peer.model_info(name)?;
    let part: PathBuf = {
        let mut s = dest.as_os_str().to_owned();
        s.push(".part");
        s.into()
    };
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&part)
        .map_err(io)?;
    let mut done = file.metadata().map_err(io)?.len();
    if done > total {
        file.set_len(0).map_err(io)?;
        done = 0;
    }
    file.seek(SeekFrom::Start(done)).map_err(io)?;
    progress(done, total);
    while done < total {
        if cancel.load(Ordering::SeqCst) {
            file.flush().map_err(io)?;
            return Err(SyncError::Cancelled);
        }
        let chunk = peer.model_chunk(name, done, MAX_CHUNK)?;
        if chunk.is_empty() {
            return Err(SyncError::Protocol);
        }
        file.write_all(&chunk).map_err(io)?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().map_err(io)?;
    drop(file);
    if sha256_file(&part).map_err(io)? != sha {
        let _ = std::fs::remove_file(&part);
        return Err(SyncError::BadHash);
    }
    std::fs::rename(&part, dest).map_err(io)?;
    Ok(())
}
