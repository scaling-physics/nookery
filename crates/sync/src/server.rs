use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::{DateTime, Utc};
use nookery_core::Event;
use nookery_recipes::{OllamaText, TextModel};
use nookery_store::{Doc, Store};

use crate::channel::{open, read_frame, session_keys, Channel, Role, MAGIC, NONCE_LEN};
use crate::secret::Secret;
use crate::{
    Request, Response, ScanAnswer, SyncError, SyncReply, MAX_CHUNK, PHONE_MODELS, PROTOCOL,
};
use nookery_core::Location;
use nookery_vision::{Ollama, TileOptions, VisionModel};

/// Settings key holding the pairing secret (hex).
pub const SETTING_SYNC_SECRET: &str = "sync.secret";

/// What the serving device offers. Called on connection threads.
pub trait Provider: Send + Sync {
    fn device(&self) -> String;
    /// The pairing secret accepted right now (read per connection, so
    /// rotating it takes effect at once). `None` = accept nobody.
    fn secret(&self) -> Option<Secret>;
    fn sync(
        &self,
        events: &[(String, Event)],
        docs: &[Doc],
        docs_since: Option<DateTime<Utc>>,
    ) -> Result<SyncReply, String>;
    fn models_dir(&self) -> PathBuf {
        default_models_dir()
    }
    /// Always the Mac's own Ollama, never the cloud.
    fn text_model(&self) -> Box<dyn TextModel> {
        Box::new(OllamaText::from_env())
    }
    /// Reads a photo with the Mac's vision model (default: the local Ollama
    /// one). The app overrides this to follow the user's AI setting.
    fn scan(
        &self,
        image: &[u8],
        place: Location,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<ScanAnswer, String> {
        let model = Ollama::from_env();
        let answered_by = format!("{} (this Mac)", model.name());
        scan_with(&model, answered_by, false, image, place, progress)
    }
    /// Reads a receipt file into text (PDF text layer, or Apple Vision OCR
    /// on a Mac). Blocking.
    fn read_receipt(&self, bytes: &[u8]) -> Result<nookery_extract::Extracted, String> {
        nookery_extract::extract(bytes).map_err(|e| friendly_extract_error(&e))
    }
}

/// A message for people for a failed receipt read.
pub fn friendly_extract_error(e: &nookery_extract::ExtractError) -> String {
    use nookery_extract::ExtractError as E;
    match e {
        E::UnknownFormat => "That file isn't a PDF, PNG, JPEG, HEIC or text file.".into(),
        E::NoText => "No text could be found in that file. Try a sharper, well-lit photo.".into(),
        E::OcrUnavailable => "This Mac can't read photos of receipts.".into(),
        E::TooLarge { max_mb } => format!("That file is too large (over {max_mb} MB)."),
        other => format!("That receipt couldn't be read: {other}."),
    }
}

/// Runs the normal scan with `model`.
pub fn scan_with(
    model: &dyn VisionModel,
    answered_by: String,
    used_cloud: bool,
    image: &[u8],
    place: Location,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<ScanAnswer, String> {
    let scan = nookery_vision::scan(
        model,
        image,
        place,
        &TileOptions::for_place(place),
        progress,
    )
    .map_err(|e| e.to_string())?;
    Ok(ScanAnswer {
        scan,
        answered_by,
        used_cloud,
    })
}

/// `$OLLAMA_MODELS`, else `~/.ollama/models`.
pub fn default_models_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("OLLAMA_MODELS").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    dirs::home_dir().unwrap_or_default().join(".ollama/models")
}

/// The store side of a sync request: merge the phone's events and docs, then
/// hand back the whole ledger and the docs changed since `docs_since`.
pub fn serve_sync(
    store: &mut Store,
    events: &[(String, Event)],
    docs: &[Doc],
    docs_since: Option<DateTime<Utc>>,
) -> Result<SyncReply, String> {
    let e = |x: nookery_store::StoreError| x.to_string();
    store.refresh().map_err(e)?;
    let report = store.merge_events(events).map_err(e)?;
    store.merge_docs(docs).map_err(e)?;
    // Taken before reading, so a change landing meanwhile is sent next time.
    let server_time = Utc::now();
    let ledger = store
        .synced_events_after(0)
        .map_err(e)?
        .into_iter()
        .map(|s| (s.uid, s.event))
        .collect();
    let docs = store.docs_changed_since(docs_since).map_err(e)?;
    Ok(SyncReply {
        report,
        ledger,
        docs,
        server_time,
    })
}

/// A `Provider` over a shared store; the secret is the store's
/// `sync.secret` setting. The store is locked per request, never across
/// network reads.
pub struct StoreProvider {
    pub store: Arc<Mutex<Store>>,
    pub device: String,
}

impl Provider for StoreProvider {
    fn device(&self) -> String {
        self.device.clone()
    }
    fn secret(&self) -> Option<Secret> {
        let s = self.store.lock().ok()?;
        Secret::from_hex(&s.setting(SETTING_SYNC_SECRET).ok()??)
    }
    fn sync(
        &self,
        events: &[(String, Event)],
        docs: &[Doc],
        docs_since: Option<DateTime<Utc>>,
    ) -> Result<SyncReply, String> {
        let mut s = self.store.lock().map_err(|_| "busy".to_string())?;
        serve_sync(&mut s, events, docs, docs_since)
    }
}

const MAX_CONNECTIONS: usize = 16;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Server {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    addr: SocketAddr,
}

impl Server {
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Starts listening on `port` (or one of the next few if it is taken). `bind`
/// is `0.0.0.0` for the whole network, `127.0.0.1` in tests.
pub fn start(bind: &str, port: u16, provider: Arc<dyn Provider>) -> std::io::Result<Server> {
    let mut last = None;
    let mut listener = None;
    for p in port..port.saturating_add(8) {
        match TcpListener::bind((bind, p)) {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => last = Some(e),
        }
    }
    let listener = listener.ok_or_else(|| last.unwrap_or_else(|| ErrorKind::AddrInUse.into()))?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicUsize::new(0));
    let flag = stop.clone();
    let handle = thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if active.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                        continue; // drop it: too many at once
                    }
                    active.fetch_add(1, Ordering::SeqCst);
                    let (provider, active) = (provider.clone(), active.clone());
                    thread::spawn(move || {
                        let _ = serve_connection(stream, provider.as_ref());
                        active.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100))
                }
                Err(_) => thread::sleep(Duration::from_millis(500)),
            }
        }
    });
    Ok(Server {
        stop,
        handle: Some(handle),
        addr,
    })
}

/// Frame plaintext: one kind byte (0 = JSON, 1 = raw bytes), then the body.
pub(crate) const KIND_JSON: u8 = 0;
pub(crate) const KIND_BYTES: u8 = 1;

pub(crate) fn send_json<S: Read + Write>(
    chan: &mut Channel<S>,
    value: &impl serde::Serialize,
) -> Result<(), SyncError> {
    let mut msg = vec![KIND_JSON];
    serde_json::to_writer(&mut msg, value).map_err(|_| SyncError::Protocol)?;
    chan.send(&msg)
}

fn send_bytes<S: Read + Write>(chan: &mut Channel<S>, bytes: &[u8]) -> Result<(), SyncError> {
    let mut msg = Vec::with_capacity(1 + bytes.len());
    msg.push(KIND_BYTES);
    msg.extend_from_slice(bytes);
    chan.send(&msg)
}

fn serve_connection(stream: TcpStream, provider: &dyn Provider) -> Result<(), SyncError> {
    stream.set_nonblocking(false).map_err(|_| SyncError::Io)?;
    stream
        .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|_| SyncError::Io)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| SyncError::Io)?;
    let mut stream = stream;

    // 1. magic + client nonce  ->  server nonce
    let mut head = [0u8; MAGIC.len() + NONCE_LEN];
    stream.read_exact(&mut head).map_err(|_| SyncError::Io)?;
    if &head[..MAGIC.len()] != MAGIC {
        return Err(SyncError::Protocol);
    }
    let mut client_nonce = [0u8; NONCE_LEN];
    client_nonce.copy_from_slice(&head[MAGIC.len()..]);
    let mut server_nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut server_nonce).map_err(|_| SyncError::Io)?;
    stream.write_all(&server_nonce).map_err(|_| SyncError::Io)?;

    // 2. the client's first message must open under the pairing secret
    let hello = read_frame(&mut stream)?;
    let found = provider.secret().and_then(|secret| {
        let (c2s, s2c) = session_keys(&secret, &client_nonce, &server_nonce);
        open(&c2s, Role::Client, 0, &hello).map(|_| (c2s, s2c))
    });
    let Some((c2s, s2c)) = found else {
        // Slow down guessing; reveal nothing.
        thread::sleep(Duration::from_millis(700));
        return Err(SyncError::Auth);
    };
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| SyncError::Io)?;
    let mut chan = Channel::new(stream, Role::Server, c2s, s2c, 0, 1);
    chan.send(b"ok")?;

    // 3. requests, one at a time
    loop {
        let req = match chan.recv() {
            Ok(r) => r,
            Err(_) => return Ok(()), // closed, timed out or tampered: just end
        };
        let Some((&KIND_JSON, body)) = req.split_first() else {
            return Ok(());
        };
        let Ok(request) = serde_json::from_slice::<Request>(body) else {
            return Ok(());
        };
        handle(&mut chan, provider, request)?;
    }
}

fn handle<S: Read + Write>(
    chan: &mut Channel<S>,
    provider: &dyn Provider,
    request: Request,
) -> Result<(), SyncError> {
    match request {
        Request::Hello { version, .. } => {
            if version != PROTOCOL {
                return send_json(
                    chan,
                    &Response::Error(
                        "The Mac and the phone run different versions of Nookery. Update both."
                            .into(),
                    ),
                );
            }
            send_json(
                chan,
                &Response::Welcome {
                    device: provider.device(),
                    server_time: Utc::now(),
                },
            )
        }
        Request::Sync {
            events,
            docs,
            docs_since,
        } => match provider.sync(&events, &docs, docs_since) {
            Ok(reply) => send_json(chan, &Response::Synced(reply)),
            Err(e) => send_json(chan, &Response::Error(e)),
        },
        Request::ModelInfo { name } => match find_model(&provider.models_dir(), &name) {
            Some(m) => send_json(
                chan,
                &Response::Model {
                    name,
                    bytes: m.bytes,
                    sha256: m.sha256,
                },
            ),
            None => send_json(chan, &Response::NoModel),
        },
        Request::ModelChunk { name, offset, len } => {
            match find_model(&provider.models_dir(), &name)
                .and_then(|m| read_chunk(&m.path, offset, len.min(MAX_CHUNK)).ok())
            {
                Some(bytes) => send_bytes(chan, &bytes),
                None => send_json(chan, &Response::Error("no such model".into())),
            }
        }
        Request::ScanPhoto { place, image } => {
            use base64::Engine;
            let (Ok(place), Ok(bytes)) = (
                place.parse::<Location>(),
                base64::engine::general_purpose::STANDARD.decode(image.trim()),
            ) else {
                return send_json(chan, &Response::Error("That photo can't be read.".into()));
            };
            let mut sent = Ok(());
            let result = provider.scan(&bytes, place, &mut |done, total| {
                if sent.is_ok() {
                    sent = send_json(
                        chan,
                        &Response::ScanProgress {
                            done: done as u32,
                            total: total as u32,
                        },
                    );
                }
            });
            sent?;
            match result {
                Ok(a) => send_json(chan, &Response::Scanned(a)),
                Err(e) => send_json(chan, &Response::Error(e)),
            }
        }
        Request::ReadReceipt { file } => {
            use base64::Engine;
            let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(file.trim()) else {
                return send_json(chan, &Response::Error("That file can't be read.".into()));
            };
            match provider.read_receipt(&bytes) {
                Ok(x) => send_json(chan, &Response::ReceiptText(x)),
                Err(e) => send_json(chan, &Response::Error(e)),
            }
        }
        Request::Generate { prompt, max_tokens } => {
            let model = provider.text_model();
            let mut sent = Ok(());
            let result = model.generate(&prompt, max_tokens, &mut |piece| {
                if sent.is_ok() {
                    sent = send_json(chan, &Response::Text(piece.to_string()));
                }
            });
            sent?;
            match result {
                Ok(_) => send_json(
                    chan,
                    &Response::Done {
                        model: model.name(),
                    },
                ),
                Err(e) => send_json(chan, &Response::Error(e.to_string())),
            }
        }
    }
}

struct ModelFile {
    path: PathBuf,
    bytes: u64,
    sha256: String,
}

/// Only allow-listed names; the manifest's model layer -> blob file.
fn find_model(dir: &Path, name: &str) -> Option<ModelFile> {
    if !PHONE_MODELS.contains(&name) {
        return None;
    }
    let (model, tag) = name.split_once(':').unwrap_or((name, "latest"));
    let manifest = dir
        .join("manifests/registry.ollama.ai/library")
        .join(model)
        .join(tag);
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    let digest = json["layers"]
        .as_array()?
        .iter()
        .find(|l| l["mediaType"] == "application/vnd.ollama.image.model")?["digest"]
        .as_str()?
        .strip_prefix("sha256:")?
        .to_ascii_lowercase();
    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let path = dir.join("blobs").join(format!("sha256-{digest}"));
    let bytes = std::fs::metadata(&path).ok()?.len();
    Some(ModelFile {
        path,
        bytes,
        sha256: digest,
    })
}

fn read_chunk(path: &Path, offset: u64, len: u64) -> std::io::Result<Vec<u8>> {
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    f.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}
