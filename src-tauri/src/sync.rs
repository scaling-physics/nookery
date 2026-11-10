//! Sync with the phone (Mac: the hub, listens while sharing is on) or with
//! the Mac (phone: the client). Rules and protocol live in `nookery-sync`;
//! this file is settings, status and the commands.
//!
//! Settings: `sync.sharing` ("1" = on, default off), `sync.secret` (hex; the
//! pairing code; on the phone: the Mac's), `sync.peer` (phone: Mac address),
//! `sync.peer_name`, `sync.last` (JSON of the last outcome).

use std::net::UdpSocket;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use nookery_core::{Event, EventKind};
use nookery_store::Store;
use nookery_sync::{
    connect, finish, prepare, Provider, Secret, Server, SyncError, DEFAULT_PORT,
    SETTING_SYNC_SECRET,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State};

use super::{AppState, CommandError};

const SETTING_SHARING: &str = "sync.sharing";
const SETTING_PEER: &str = "sync.peer";
const SETTING_PEER_NAME: &str = "sync.peer_name";
const SETTING_LAST: &str = "sync.last";

/// The running server (Mac), if any.
#[derive(Default)]
pub struct SyncState {
    server: Mutex<Option<Server>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SyncRole {
    Mac,
    Phone,
}

/// How the last sync went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LastSyncView {
    /// RFC 3339.
    pub at: String,
    pub ok: bool,
    /// Phone changes the Mac took.
    pub pushed: u32,
    /// Items in the kitchen list afterwards.
    pub pulled: u32,
    pub docs_changed: u32,
    /// Phone changes the Mac refused, in words ("Used up milk").
    pub dropped: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusView {
    pub role: SyncRole,
    /// Mac: listening for the phone.
    pub sharing: bool,
    /// Mac: this computer's address on the home network (`192.168.1.20`).
    pub addresses: Vec<String>,
    pub port: u32,
    /// Mac: the pairing code, grouped for reading.
    pub code: Option<String>,
    /// Mac, while sharing: `nookery://pair?code=..&addr=..&name=..`, what the
    /// QR code holds (the secret: show it only to the user).
    pub link: Option<String>,
    /// Mac, while sharing: the QR code as an SVG string (dark ink on a light
    /// tile, quiet zone included).
    pub qr: Option<String>,
    /// Phone: the paired Mac's address.
    pub peer: Option<String>,
    pub peer_name: Option<String>,
    pub last: Option<LastSyncView>,
}

fn lan_addresses() -> Vec<String> {
    // Connecting a UDP socket sends nothing; it only picks the interface
    // the system would use to reach the internet, i.e. the home network.
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.0.2.1:9")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .ok()
        .filter(|ip| ip != "0.0.0.0")
        .into_iter()
        .collect()
}

/// This computer's name for the phone to show ("Srikanth's MacBook").
fn device_name() -> String {
    std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Mac".into())
}

/// Percent-encodes everything but unreserved characters.
fn pct(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// What the QR code holds. The phone parses it in `ui/src/lib/phone/pairing.ts`.
pub fn pairing_link(code: &str, addr: &str, name: &str) -> String {
    format!(
        "nookery://pair?code={}&addr={}&name={}",
        pct(code),
        pct(addr),
        pct(name)
    )
}

/// An SVG of `text` as a QR code: ink on a light tile (readable in dark
/// mode too), with a quiet zone. Made here, offline.
fn qr_svg(text: &str) -> Option<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::M).ok()?;
    Some(
        code.render()
            .min_dimensions(200, 200)
            .quiet_zone(true)
            .dark_color(svg::Color("#2a2118"))
            .light_color(svg::Color("#fffdf8"))
            .build(),
    )
}

fn friendly(e: SyncError) -> CommandError {
    match e {
        SyncError::Auth => CommandError::SyncBadCode,
        SyncError::Connect => CommandError::SyncUnreachable(
            "Couldn't reach the Mac. Is Nookery open there, sharing switched on, and are both on the same Wi-Fi?".into(),
        ),
        SyncError::Io => CommandError::SyncUnreachable(
            "The connection to the Mac dropped. Try again.".into(),
        ),
        SyncError::Peer(m) | SyncError::Store(m) => CommandError::SyncUnreachable(m),
        other => CommandError::SyncUnreachable(other.to_string()),
    }
}

fn pairing_secret(store: &mut Store) -> Result<Secret, CommandError> {
    if let Some(s) = store
        .setting(SETTING_SYNC_SECRET)?
        .and_then(|h| Secret::from_hex(&h))
    {
        return Ok(s);
    }
    new_secret(store)
}

fn new_secret(store: &mut Store) -> Result<Secret, CommandError> {
    let s = Secret::generate().map_err(|_| CommandError::Internal)?;
    store.set_setting(SETTING_SYNC_SECRET, &s.to_hex())?;
    Ok(s)
}

fn last_in(store: &Store) -> Option<LastSyncView> {
    serde_json::from_str(&store.setting(SETTING_LAST).ok()??).ok()
}

fn record(store: &mut Store, last: &LastSyncView) {
    if let Ok(json) = serde_json::to_string(last) {
        let _ = store.set_setting(SETTING_LAST, &json);
    }
}

pub fn status_in(store: &mut Store, port: Option<u16>) -> Result<SyncStatusView, CommandError> {
    let last = last_in(store);
    if cfg!(mobile) {
        return Ok(SyncStatusView {
            role: SyncRole::Phone,
            sharing: false,
            addresses: vec![],
            port: u32::from(DEFAULT_PORT),
            code: None,
            link: None,
            qr: None,
            peer: store.setting(SETTING_PEER)?.filter(|p| !p.is_empty()),
            peer_name: store.setting(SETTING_PEER_NAME)?.filter(|p| !p.is_empty()),
            last,
        });
    }
    let secret = pairing_secret(store)?;
    let addresses = lan_addresses();
    let link = match (port, addresses.first()) {
        (Some(port), Some(ip)) => Some(pairing_link(
            &secret.to_code(),
            &format!("{ip}:{port}"),
            &device_name(),
        )),
        _ => None,
    };
    Ok(SyncStatusView {
        role: SyncRole::Mac,
        sharing: port.is_some(),
        addresses,
        port: u32::from(port.unwrap_or(DEFAULT_PORT)),
        code: Some(secret.to_code()),
        qr: link.as_deref().and_then(qr_svg),
        link,
        peer: None,
        peer_name: None,
        last,
    })
}

/// The Mac's side: the server asks the app's store per request.
struct AppProvider {
    app: AppHandle,
}

impl Provider for AppProvider {
    fn device(&self) -> String {
        "Mac".into()
    }
    fn secret(&self) -> Option<Secret> {
        let state = self.app.state::<AppState>();
        let s = state.store.lock().ok()?;
        Secret::from_hex(&s.setting(SETTING_SYNC_SECRET).ok()??)
    }
    fn sync(
        &self,
        events: &[(String, Event)],
        docs: &[nookery_store::Doc],
        since: Option<DateTime<Utc>>,
    ) -> Result<nookery_sync::SyncReply, String> {
        let state = self.app.state::<AppState>();
        let mut s = state.store.lock().map_err(|_| "busy".to_string())?;
        nookery_sync::serve_sync(&mut s, events, docs, since)
    }
    /// A photo from the phone: read with this Mac's own photo setting.
    fn scan(
        &self,
        image: &[u8],
        place: nookery_core::Location,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<nookery_sync::ScanAnswer, String> {
        let state = self.app.state::<AppState>();
        let (mode, cloud_model) = {
            let s = state.store().map_err(|_| "busy".to_string())?;
            (
                s.ai_mode(nookery_store::SETTING_PHOTOS_AI)
                    .map_err(|e| e.to_string())?,
                s.cloud_model().map_err(|e| e.to_string())?,
            )
        };
        let model = nookery_vision::Chosen::for_mode(mode, &cloud_model);
        let a = nookery_sync::scan_with(&model, String::new(), false, image, place, progress)?;
        Ok(nookery_sync::ScanAnswer {
            answered_by: model.answered_by(),
            used_cloud: model.used_cloud(),
            ..a
        })
    }
}

fn start_server(app: &AppHandle, sync: &SyncState) -> Result<u16, CommandError> {
    let mut slot = sync.server.lock().map_err(|_| CommandError::Internal)?;
    if let Some(s) = slot.as_ref() {
        return Ok(s.port());
    }
    let server = nookery_sync::start(
        "0.0.0.0",
        DEFAULT_PORT,
        Arc::new(AppProvider { app: app.clone() }),
    )
    .map_err(|_| {
        CommandError::SyncUnreachable("Couldn't start sharing: the network port is in use.".into())
    })?;
    let port = server.port();
    *slot = Some(server);
    Ok(port)
}

/// Called once from `setup`, after `AppState` is managed.
pub fn init(app: &AppHandle) {
    app.manage(SyncState::default());
    if cfg!(mobile) {
        return;
    }
    let on = {
        let state = app.state::<AppState>();
        state
            .store()
            .ok()
            .and_then(|s| s.setting(SETTING_SHARING).ok().flatten())
            .is_some_and(|v| v == "1")
    };
    if on {
        if let Err(e) = start_server(app, &app.state::<SyncState>()) {
            eprintln!("nookery: sharing didn't start: {e:?}");
        }
    }
}

fn running_port(sync: &SyncState) -> Option<u16> {
    sync.server.lock().ok()?.as_ref().map(Server::port)
}

#[tauri::command]
#[specta::specta]
pub fn get_sync_status(
    state: State<'_, AppState>,
    sync: State<'_, SyncState>,
) -> Result<SyncStatusView, CommandError> {
    let mut store = state.store()?;
    status_in(&mut store, running_port(&sync))
}

#[tauri::command]
#[specta::specta]
pub fn set_sharing(
    app: AppHandle,
    state: State<'_, AppState>,
    sync: State<'_, SyncState>,
    on: bool,
) -> Result<SyncStatusView, CommandError> {
    if cfg!(mobile) {
        return Err(CommandError::BadInput("sharing".into()));
    }
    let mut store = state.store()?;
    pairing_secret(&mut store)?;
    if on {
        start_server(&app, &sync)?;
    } else if let Ok(mut slot) = sync.server.lock() {
        if let Some(s) = slot.take() {
            s.stop();
        }
    }
    store.set_setting(SETTING_SHARING, if on { "1" } else { "0" })?;
    status_in(&mut store, running_port(&sync))
}

#[tauri::command]
#[specta::specta]
pub fn new_pairing_code(
    state: State<'_, AppState>,
    sync: State<'_, SyncState>,
) -> Result<SyncStatusView, CommandError> {
    let mut store = state.store()?;
    new_secret(&mut store)?;
    status_in(&mut store, running_port(&sync))
}

/// Phone, network part: checks the code and address with the Mac.
/// Returns the secret, the address and the Mac's name.
fn check_pairing(code: &str, address: &str) -> Result<(Secret, String, String), CommandError> {
    let secret = Secret::from_code(code).ok_or(CommandError::SyncBadCode)?;
    let address = address.trim();
    if address.is_empty() {
        return Err(CommandError::BadInput("address".into()));
    }
    let mut peer = connect(address, &secret).map_err(friendly)?;
    let (name, _) = peer.hello("phone").map_err(friendly)?;
    Ok((secret, address.to_string(), name))
}

fn save_pairing(store: &mut Store, p: &(Secret, String, String)) -> Result<(), CommandError> {
    store.set_setting(SETTING_SYNC_SECRET, &p.0.to_hex())?;
    store.set_setting(SETTING_PEER, &p.1)?;
    store.set_setting(SETTING_PEER_NAME, &p.2)?;
    Ok(())
}

#[cfg(test)]
pub fn pair_in(store: &mut Store, code: &str, address: &str) -> Result<(), CommandError> {
    save_pairing(store, &check_pairing(code, address)?)
}

#[tauri::command]
#[specta::specta]
pub async fn pair_with_mac(
    app: AppHandle,
    code: String,
    address: String,
) -> Result<SyncStatusView, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        // Network first; the store is locked only to save.
        let p = check_pairing(&code, &address)?;
        let state = app.state::<AppState>();
        let mut store = state.store()?;
        save_pairing(&mut store, &p)?;
        status_in(&mut store, None)
    })
    .await
    .map_err(|_| CommandError::Internal)?
}

#[tauri::command]
#[specta::specta]
pub fn forget_mac(state: State<'_, AppState>) -> Result<SyncStatusView, CommandError> {
    let mut store = state.store()?;
    for k in [SETTING_PEER, SETTING_PEER_NAME, SETTING_LAST] {
        store.set_setting(k, "")?;
    }
    store.set_setting(SETTING_SYNC_SECRET, "")?;
    status_in(&mut store, None)
}

fn describe(store: &Store, e: &Event) -> String {
    let item = store
        .inventory()
        .get(e.lot)
        .map(|l| l.item.clone())
        .unwrap_or_else(|| "an item".into());
    match &e.kind {
        EventKind::Acquired { item, .. } => format!("Added {item}"),
        EventKind::Used { .. } => format!("Used {item}"),
        EventKind::Binned => format!("Threw away {item}"),
        EventKind::Opened => format!("Opened {item}"),
        EventKind::Moved { .. } => format!("Moved {item}"),
        EventKind::Retracted => format!("Removed {item}"),
        _ => format!("Checked {item}"),
    }
}

/// One whole sync as the phone: the store is unlocked while the network is
/// used. Records the outcome (success or failure) for the settings page.
pub fn run_sync(app: &AppHandle) -> Result<LastSyncView, CommandError> {
    let state = app.state::<AppState>();
    let (addr, secret) = {
        let store = state.store()?;
        let addr = store.setting(SETTING_PEER)?.filter(|a| !a.is_empty());
        let secret = store
            .setting(SETTING_SYNC_SECRET)?
            .and_then(|h| Secret::from_hex(&h));
        match (addr, secret) {
            (Some(a), Some(s)) => (a, s),
            _ => return Err(CommandError::BadInput("peer".into())),
        }
    };
    let result = (|| -> Result<LastSyncView, SyncError> {
        for _ in 0..3 {
            let req = {
                let store = state.store.lock().map_err(|_| SyncError::Io)?;
                prepare(&store)?
            };
            let mut peer = connect(&addr, &secret)?;
            let reply = peer.exchange(&req)?;
            let mut store = state.store.lock().map_err(|_| SyncError::Io)?;
            let dropped: Vec<String> = req
                .events()
                .iter()
                .filter(|(uid, _)| reply.report.rejected.iter().any(|(r, _)| r == uid))
                .map(|(_, e)| describe(&store, e))
                .collect();
            match finish(&mut store, &req, reply) {
                Err(SyncError::Changed) => continue,
                Err(e) => return Err(e),
                Ok(o) => {
                    return Ok(LastSyncView {
                        at: Utc::now().to_rfc3339(),
                        ok: true,
                        pushed: o.pushed as u32,
                        pulled: o.pulled_events as u32,
                        docs_changed: o.docs_changed as u32,
                        dropped,
                        error: None,
                    })
                }
            }
        }
        Err(SyncError::Changed)
    })();
    let mut store = state.store()?;
    match result {
        Ok(last) => {
            record(&mut store, &last);
            Ok(last)
        }
        Err(e) => {
            let err = friendly(e);
            let msg = match &err {
                CommandError::SyncBadCode => {
                    "The Mac no longer accepts this phone. Pair again with the new code."
                        .to_string()
                }
                CommandError::SyncUnreachable(m) => m.clone(),
                _ => "Sync failed.".to_string(),
            };
            record(
                &mut store,
                &LastSyncView {
                    at: Utc::now().to_rfc3339(),
                    ok: false,
                    pushed: 0,
                    pulled: 0,
                    docs_changed: 0,
                    dropped: vec![],
                    error: Some(msg),
                },
            );
            Err(err)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn sync_now(app: AppHandle) -> Result<SyncStatusView, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let r = run_sync(&app);
        let state = app.state::<AppState>();
        let mut store = state.store()?;
        let status = status_in(&mut store, None)?;
        r.map(|_| status)
    })
    .await
    .map_err(|_| CommandError::Internal)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Location, LotId, Qty, Source};

    fn acquire(item: &str) -> Event {
        Event {
            at: Utc::now(),
            lot: LotId::new(),
            kind: EventKind::Acquired {
                item: item.into(),
                qty: Qty::pieces(1),
                location: Location::Fridge,
                source: Source::Manual,
                best_before: None,
            },
        }
    }

    #[test]
    fn describes_dropped_changes_in_words() {
        let mut s = Store::open_in_memory().unwrap();
        let milk = acquire("milk");
        s.append(std::slice::from_ref(&milk)).unwrap();
        let used = Event {
            at: Utc::now(),
            lot: milk.lot,
            kind: EventKind::Used { qty: None },
        };
        assert_eq!(describe(&s, &used), "Used milk");
    }

    #[test]
    fn pairing_link_is_url_safe_and_the_qr_is_an_svg() {
        let link = pairing_link("ABCD-EFGH", "192.168.0.5:47480", "Anna's Mac mini");
        assert_eq!(
            link,
            "nookery://pair?code=ABCD-EFGH&addr=192.168.0.5%3A47480&name=Anna%27s%20Mac%20mini"
        );
        let svg = qr_svg(&link).unwrap();
        assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"));
        assert!(svg.contains("#2a2118") && svg.contains("#fffdf8"));
    }

    #[cfg(not(mobile))]
    #[test]
    fn mac_status_makes_a_stable_code_and_pairing_works() {
        use nookery_sync::{start, StoreProvider};
        let mut mac = Store::open_in_memory().unwrap();
        let a = status_in(&mut mac, None).unwrap();
        let b = status_in(&mut mac, None).unwrap();
        assert_eq!(a.role, SyncRole::Mac);
        assert!(!a.sharing);
        assert_eq!(a.code, b.code);
        let code = a.code.unwrap();
        assert_eq!(code.len(), 32); // 26 letters + 6 dashes

        let shared = Arc::new(Mutex::new(mac));
        let server = start(
            "127.0.0.1",
            0,
            Arc::new(StoreProvider {
                store: shared,
                device: "Mac".into(),
            }),
        )
        .unwrap();
        let mut phone = Store::open_in_memory().unwrap();
        let addr = format!("127.0.0.1:{}", server.port());
        assert!(matches!(
            pair_in(&mut phone, "AAAA-AAAA-AAAA-AAAA-AAAA-AA", &addr),
            Err(CommandError::SyncBadCode)
        ));
        pair_in(&mut phone, &code, &addr).unwrap();
        assert_eq!(
            phone.setting(SETTING_PEER).unwrap().as_deref(),
            Some(addr.as_str())
        );
        assert!(matches!(
            pair_in(&mut phone, "nonsense", &addr),
            Err(CommandError::SyncBadCode)
        ));
    }
}
