//! The phone's own AI: which model writes recipes (this phone, or the paired
//! Mac), the recipe-model download from the Mac, and reading photos through
//! the Mac.
//!
//! Settings: `ai.phone_recipes` = `phone` | `mac` | `auto` (default `auto`:
//! the phone if its model is downloaded, else the Mac) and
//! `ai.phone_threads` (default 6). On the desktop none of this applies:
//! `Writer::for_state` just wraps the usual `ChosenText`.
//!
//! The model file is `<app data>/models/qwen3-4b-instruct.gguf`, fetched
//! from the Mac over the paired, encrypted connection (resumable via a
//! `.part` file, checked against the Mac's sha256). `LlamaText` exists only
//! in the Android build (feature `llama` needs cmake); it is loaded on first
//! use inside the blocking thread that generates, kept for later calls, and
//! calls are serialised by the mutex that guards it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nookery_core::{AiMode, Location};
use nookery_recipes::{ChosenText, RecipeError, TextModel};
use nookery_store::Store;
use nookery_sync::{
    connect, download_model_cancellable, MacText, ScanAnswer, Secret, SyncError, PHONE_MODELS,
    SETTING_SYNC_SECRET,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::Channel;
use tauri::State;

use super::{AppState, CommandError};

pub const SETTING_PHONE_RECIPES: &str = "ai.phone_recipes";
const SETTING_PHONE_THREADS: &str = "ai.phone_threads";
const SETTING_PEER: &str = "sync.peer";
pub const MODEL_FILE: &str = "qwen3-4b-instruct.gguf";
const MODEL_NAME: &str = PHONE_MODELS[0];
const DEFAULT_THREADS: i32 = 6;
/// What the phone model's file is about (for the "free space" hint before the
/// Mac has told us the exact size).
const MODEL_APPROX_BYTES: u64 = 2_600_000_000;

/// Where the phone writes recipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PhoneAiPref {
    Auto,
    Phone,
    Mac,
}

impl PhoneAiPref {
    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Phone => "phone",
            Self::Mac => "mac",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "phone" => Self::Phone,
            "mac" => Self::Mac,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Phone,
    Mac,
}

/// The choice, without any I/O: what `pref` means given what exists.
pub fn pick(pref: PhoneAiPref, downloaded: bool, paired: bool) -> Result<Pick, CommandError> {
    let mac = || {
        if paired {
            Ok(Pick::Mac)
        } else {
            Err(no_mac())
        }
    };
    match pref {
        PhoneAiPref::Phone if downloaded => Ok(Pick::Phone),
        PhoneAiPref::Phone => Err(CommandError::PhoneModelMissing),
        PhoneAiPref::Mac => mac(),
        PhoneAiPref::Auto if downloaded => Ok(Pick::Phone),
        PhoneAiPref::Auto => mac(),
    }
}

fn no_mac() -> CommandError {
    CommandError::MacUnreachable(
        "No Mac is paired yet. In Settings, download the recipe model for this phone, or pair it with your Mac.".into(),
    )
}

/// Shared by the commands: the models folder, the loaded model, the running
/// download.
pub struct PhoneState {
    models_dir: PathBuf,
    #[cfg(target_os = "android")]
    llama: Arc<Mutex<Option<nookery_llm::LlamaText>>>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl PhoneState {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            models_dir,
            #[cfg(target_os = "android")]
            llama: Arc::default(),
            cancel: Mutex::new(None),
        }
    }

    fn model_path(&self) -> PathBuf {
        self.models_dir.join(MODEL_FILE)
    }

    fn part_path(&self) -> PathBuf {
        let mut s = self.model_path().into_os_string();
        s.push(".part");
        s.into()
    }

    fn downloaded(&self) -> bool {
        self.model_path().is_file()
    }

    /// Free the loaded model (after Delete, so the file's space is returned).
    fn unload(&self) {
        #[cfg(target_os = "android")]
        if let Ok(mut g) = self.llama.lock() {
            *g = None;
        }
    }
}

/// The recipe model on the phone: loaded on first use.
#[cfg(target_os = "android")]
pub struct PhoneModel {
    path: PathBuf,
    threads: i32,
    slot: Arc<Mutex<Option<nookery_llm::LlamaText>>>,
}

#[cfg(target_os = "android")]
impl TextModel for PhoneModel {
    fn name(&self) -> String {
        "qwen3 4b (on this phone)".into()
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        // One generation at a time, and one loaded model.
        let mut slot = self
            .slot
            .lock()
            .map_err(|_| RecipeError::Model("busy".into()))?;
        if slot.is_none() {
            let opts = nookery_llm::LlamaOptions {
                threads: self.threads,
                ..Default::default()
            };
            let m = nookery_llm::LlamaText::load(&self.path, opts)
                .map_err(|e| RecipeError::Model(e.to_string()))?;
            *slot = Some(m);
        }
        let model = slot.as_ref().expect("just loaded");
        let out = model.generate(prompt, max_tokens, on_text)?;
        let s = model.last_stats();
        eprintln!(
            "nookery: phone model: prompt {} tok in {:.1}s ({:.1} t/s), generated {} tok in {:.1}s ({:.1} t/s)",
            s.prompt_tokens,
            s.prompt_ms / 1e3,
            s.prompt_tps(),
            s.generated_tokens,
            s.generate_ms / 1e3,
            s.generate_tps()
        );
        Ok(out)
    }
}

/// The text model for recipes and plans, wherever it runs.
pub enum Writer {
    /// The desktop: the user's AI setting (Ollama on this Mac, maybe cloud).
    Desktop(ChosenText),
    #[cfg(target_os = "android")]
    Phone(PhoneModel),
    /// The phone, asking the paired Mac.
    Mac(MacText),
}

impl Writer {
    /// Reads the settings (store locked briefly); loads nothing.
    pub fn for_state(
        state: &AppState,
        mode: AiMode,
        cloud_model: &str,
    ) -> Result<Self, CommandError> {
        if !cfg!(mobile) {
            return Ok(Writer::Desktop(ChosenText::for_mode(mode, cloud_model)));
        }
        let (pref, paired, threads) = {
            let store = state.store()?;
            (
                pref_in(&store)?,
                mac_in(&store)?,
                store
                    .setting(SETTING_PHONE_THREADS)?
                    .and_then(|v| v.parse().ok())
                    .filter(|n| (1..=16).contains(n))
                    .unwrap_or(DEFAULT_THREADS),
            )
        };
        let _ = threads;
        match pick(pref, state.phone.downloaded(), paired.is_some())? {
            Pick::Mac => {
                let (addr, secret) = paired.expect("checked by pick");
                Ok(Writer::Mac(MacText::new(&addr, secret)))
            }
            Pick::Phone => {
                #[cfg(target_os = "android")]
                {
                    Ok(Writer::Phone(PhoneModel {
                        path: state.phone.model_path(),
                        threads,
                        slot: state.phone.llama.clone(),
                    }))
                }
                #[cfg(not(target_os = "android"))]
                Err(CommandError::PhoneModelMissing)
            }
        }
    }

    /// Where it ran, for the page ("qwen3 4b (on this phone)").
    pub fn answered_by(&self) -> String {
        match self {
            Writer::Desktop(c) => c.answered_by(),
            #[cfg(target_os = "android")]
            Writer::Phone(p) => p.name(),
            Writer::Mac(m) => {
                let name = m.name();
                let inner = name
                    .strip_prefix("your Mac (")
                    .and_then(|n| n.strip_suffix(')'))
                    .unwrap_or("model");
                format!("{inner} (on your Mac)")
            }
        }
    }

    pub fn used_cloud(&self) -> bool {
        match self {
            Writer::Desktop(c) => c.used_cloud(),
            _ => false,
        }
    }
}

impl TextModel for Writer {
    fn name(&self) -> String {
        match self {
            Writer::Desktop(c) => c.name(),
            #[cfg(target_os = "android")]
            Writer::Phone(p) => p.name(),
            Writer::Mac(m) => m.name(),
        }
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        match self {
            Writer::Desktop(c) => c.generate(prompt, max_tokens, on_text),
            #[cfg(target_os = "android")]
            Writer::Phone(p) => p.generate(prompt, max_tokens, on_text),
            Writer::Mac(m) => m.generate(prompt, max_tokens, on_text),
        }
    }
}

fn pref_in(store: &Store) -> Result<PhoneAiPref, CommandError> {
    Ok(store
        .setting(SETTING_PHONE_RECIPES)?
        .map(|v| PhoneAiPref::parse(&v))
        .unwrap_or(PhoneAiPref::Auto))
}

/// The paired Mac: address and secret.
fn mac_in(store: &Store) -> Result<Option<(String, Secret)>, CommandError> {
    let addr = store.setting(SETTING_PEER)?.filter(|a| !a.is_empty());
    let secret = store
        .setting(SETTING_SYNC_SECRET)?
        .and_then(|h| Secret::from_hex(&h));
    Ok(addr.zip(secret))
}

fn mac_error(e: SyncError) -> CommandError {
    match e {
        SyncError::Connect => CommandError::MacUnreachable(
            "Couldn't reach your Mac. Is Nookery open there, sharing switched on, and are you on the home Wi-Fi?".into(),
        ),
        SyncError::Auth => CommandError::SyncBadCode,
        SyncError::NoModel => CommandError::MacUnreachable(
            "Your Mac doesn't have the recipe model yet. On the Mac run “ollama pull qwen3:4b-instruct”.".into(),
        ),
        SyncError::Io => CommandError::MacUnreachable("The connection to your Mac dropped. Try again; the download picks up where it stopped.".into()),
        SyncError::BadHash => CommandError::MacUnreachable("The downloaded model was damaged. Try again.".into()),
        SyncError::Peer(m) => CommandError::MacUnreachable(m),
        other => CommandError::MacUnreachable(other.to_string()),
    }
}

/// The paired Mac's address and secret (store locked briefly).
pub fn mac_target(state: &AppState) -> Result<(String, Secret), CommandError> {
    let store = state.store()?;
    mac_in(&store)?.ok_or_else(no_mac)
}

/// Reads a photo through the Mac (blocking: call from a blocking thread).
/// Returns the scan, who answered and whether the cloud did (the Mac's own
/// setting).
pub fn scan_via_mac(
    target: &(String, Secret),
    jpeg: &[u8],
    place: Location,
    progress: &mut dyn FnMut(u32, u32),
) -> Result<ScanAnswer, CommandError> {
    let mut peer = connect(&target.0, &target.1).map_err(mac_error)?;
    peer.scan_photo(place, jpeg, progress).map_err(mac_error)
}

// --- commands ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PhoneModelView {
    pub downloaded: bool,
    /// Size of the downloaded file.
    pub bytes: f64,
    /// A half-finished download that can be resumed.
    pub partial_bytes: f64,
    pub free_bytes: Option<f64>,
    /// About how big the download is.
    pub expected_bytes: f64,
    pub downloading: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PhoneAiView {
    /// False on the desktop: nothing of this applies there.
    pub available: bool,
    pub pref: PhoneAiPref,
    pub model: PhoneModelView,
    /// A Mac is paired (needed to download and for "On my Mac").
    pub paired: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub done: f64,
    pub total: f64,
}

fn free_space(dir: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        let c = std::ffi::CString::new(dir.to_str()?).ok()?;
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: `c` is a valid C string and `st` a valid out pointer.
        if unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0 {
            return Some(st.f_bavail as u64 * st.f_frsize as u64);
        }
    }
    let _ = dir;
    None
}

fn view_in(state: &AppState, store: &Store) -> Result<PhoneAiView, CommandError> {
    let p = &state.phone;
    let len = |path: PathBuf| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let downloading = p.cancel.lock().map(|c| c.is_some()).unwrap_or(false);
    Ok(PhoneAiView {
        available: cfg!(mobile),
        pref: pref_in(store)?,
        model: PhoneModelView {
            downloaded: p.downloaded(),
            bytes: len(p.model_path()) as f64,
            partial_bytes: len(p.part_path()) as f64,
            free_bytes: free_space(&p.models_dir).map(|b| b as f64),
            expected_bytes: MODEL_APPROX_BYTES as f64,
            downloading,
        },
        paired: mac_in(store)?.is_some(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_phone_ai(state: State<'_, AppState>) -> Result<PhoneAiView, CommandError> {
    let store = state.store()?;
    view_in(&state, &store)
}

#[tauri::command]
#[specta::specta]
pub fn set_phone_ai(
    state: State<'_, AppState>,
    pref: PhoneAiPref,
) -> Result<PhoneAiView, CommandError> {
    let mut store = state.store()?;
    store.set_setting(SETTING_PHONE_RECIPES, pref.as_str())?;
    view_in(&state, &store)
}

/// Clears the running flag however the download ends.
struct Running<'a>(&'a Mutex<Option<Arc<AtomicBool>>>);
impl Drop for Running<'_> {
    fn drop(&mut self) {
        if let Ok(mut g) = self.0.lock() {
            *g = None;
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn download_phone_model(
    state: State<'_, AppState>,
    on_progress: Channel<DownloadProgress>,
) -> Result<PhoneAiView, CommandError> {
    if !cfg!(mobile) {
        return Err(CommandError::BadInput("phone".into()));
    }
    let (addr, secret) = {
        let store = state.store()?;
        mac_in(&store)?.ok_or_else(no_mac)?
    };
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = state
            .phone
            .cancel
            .lock()
            .map_err(|_| CommandError::Internal)?;
        if slot.is_some() {
            return Err(CommandError::BadInput("downloading".into()));
        }
        *slot = Some(cancel.clone());
    }
    let phone = state.phone.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _running = Running(&phone.cancel);
        std::fs::create_dir_all(&phone.models_dir).map_err(|_| CommandError::Internal)?;
        let mut peer = connect(&addr, &secret).map_err(mac_error)?;
        peer.hello("phone").map_err(mac_error)?;
        let mut last = Instant::now();
        download_model_cancellable(
            &mut peer,
            MODEL_NAME,
            &phone.model_path(),
            &mut |done, total| {
                if last.elapsed().as_millis() >= 250 || done == total {
                    last = Instant::now();
                    let _ = on_progress.send(DownloadProgress {
                        done: done as f64,
                        total: total as f64,
                    });
                }
            },
            &cancel,
        )
        .map_err(|e| match e {
            // Not an error for the user: they pressed Stop.
            SyncError::Cancelled => CommandError::BadInput("cancelled".into()),
            other => mac_error(other),
        })
    })
    .await
    .map_err(|_| CommandError::Internal)?;
    result?;
    let store = state.store()?;
    view_in(&state, &store)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_phone_download(state: State<'_, AppState>) -> Result<(), CommandError> {
    if let Some(flag) = state
        .phone
        .cancel
        .lock()
        .map_err(|_| CommandError::Internal)?
        .as_ref()
    {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_phone_model(state: State<'_, AppState>) -> Result<PhoneAiView, CommandError> {
    state.phone.unload();
    for p in [state.phone.model_path(), state.phone.part_path()] {
        match std::fs::remove_file(p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(CommandError::Internal),
        }
    }
    let store = state.store()?;
    view_in(&state, &store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_prefers_the_phone_when_its_model_is_there() {
        assert_eq!(pick(PhoneAiPref::Auto, true, true).unwrap(), Pick::Phone);
        assert_eq!(pick(PhoneAiPref::Auto, false, true).unwrap(), Pick::Mac);
        assert!(matches!(
            pick(PhoneAiPref::Auto, false, false),
            Err(CommandError::MacUnreachable(_))
        ));
    }

    #[test]
    fn explicit_choices_say_what_is_missing() {
        assert!(matches!(
            pick(PhoneAiPref::Phone, false, true),
            Err(CommandError::PhoneModelMissing)
        ));
        assert_eq!(pick(PhoneAiPref::Phone, true, false).unwrap(), Pick::Phone);
        assert_eq!(pick(PhoneAiPref::Mac, true, true).unwrap(), Pick::Mac);
        assert!(pick(PhoneAiPref::Mac, true, false).is_err());
    }

    #[test]
    fn preference_round_trips_and_unknown_means_auto() {
        for p in [PhoneAiPref::Auto, PhoneAiPref::Phone, PhoneAiPref::Mac] {
            assert_eq!(PhoneAiPref::parse(p.as_str()), p);
        }
        assert_eq!(PhoneAiPref::parse("???"), PhoneAiPref::Auto);
    }
}
