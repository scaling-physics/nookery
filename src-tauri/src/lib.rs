//! The Tauri app: thin commands over `nookery-store`. All rules live in the
//! crates; this file only converts between them and the UI's types.

mod phone;
mod photo;
mod plan;
mod receipt;
mod receipt_image;
mod recipes;
mod settings;
mod sync;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{Local, Utc};
use nookery_core::{
    category, parse_date, status, use_first, Event, EventKind, FoodCategory, Gone, Location, Lot,
    LotId, Qty, Source, Status,
};
#[cfg(not(mobile))]
use nookery_store::default_data_dir;
use nookery_store::{Store, StoreError};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, State};
use tauri_specta::{collect_commands, Builder};

pub struct AppState {
    store: Mutex<Store>,
    /// How long opening the database and replaying the ledger took at start.
    open_ms: f64,
    /// The last photo scan's plan, until it is applied or replaced.
    last_scan: Mutex<Option<photo::StoredScan>>,
    /// The last recipe suggestions, until replaced.
    last_suggestions: Mutex<Option<recipes::StoredSuggestions>>,
    /// The phone's model file, loaded model and running download.
    phone: Arc<phone::PhoneState>,
}

impl AppState {
    /// The store, reloaded first if another process (the CLI) appended
    /// since: one indexed query, so every command sees current data.
    pub(crate) fn store(&self) -> Result<std::sync::MutexGuard<'_, Store>, CommandError> {
        let mut s = self.store.lock().map_err(|_| CommandError::Internal)?;
        s.refresh()?;
        Ok(s)
    }
}

/// What the UI is told when a command fails.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum CommandError {
    /// A field the user typed can't be read (which field).
    BadInput(String),
    NotFound,
    /// The vision model isn't reachable (detail: its url): Ollama not running.
    VisionUnavailable(String),
    /// The vision model isn't installed (detail: its name).
    VisionModelMissing(String),
    /// The recipe model isn't reachable (detail: its url): Ollama not running.
    RecipeModelUnavailable(String),
    /// The recipe model isn't installed (detail: its name).
    RecipeModelMissing(String),
    /// A cloud model was chosen but Ollama isn't signed in
    /// (`ollama signin`).
    CloudSignIn,
    /// Signed in, but the Ollama account has no credits for the cloud
    /// model (ollama.com/plans).
    CloudCredits,
    /// This receipt is already in the ledger.
    AlreadyImported,
    /// Sync with the other device failed (detail: a friendly sentence).
    SyncUnreachable(String),
    /// The pairing code is wrong (or was replaced on the Mac).
    SyncBadCode,
    /// Phone: "On this phone" is chosen but the recipe model isn't
    /// downloaded yet.
    PhoneModelMissing,
    /// Phone: the paired Mac is needed and can't be used (detail: a
    /// friendly sentence).
    MacUnreachable(String),
    /// A receipt file can't be read (detail: `unknown_format`, `too_large`,
    /// `no_text`, `ocr_unavailable`, `pdf` or `image`).
    ReceiptUnreadable(String),
    Internal,
}

impl From<StoreError> for CommandError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NoMatch(_) | StoreError::Ambiguous(_) => Self::NotFound,
            StoreError::DuplicateReceipt(_) => Self::AlreadyImported,
            other => {
                eprintln!("nookery: {other}");
                Self::Internal
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LotStatus {
    Expired,
    UseSoon,
    Fresh,
    Unconfirmed,
    Undated,
    Used,
    Binned,
    /// Removed as entered by mistake.
    Retracted,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LotView {
    pub id: String,
    pub short: String,
    pub item: String,
    /// e.g. "500g", "1.5kg", "6".
    pub left: String,
    pub location: String,
    pub status: LotStatus,
    /// Days ago (expired, unconfirmed) or days left (use_soon, fresh).
    pub days: Option<i32>,
    pub best_before: Option<String>,
    pub opened: bool,
    pub frozen_on: Option<String>,
    pub store: Option<String>,
    /// For the little food icon (guessed from the name).
    pub category: FoodCategoryView,
}

/// `nookery_core::FoodCategory`, typed for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum FoodCategoryView {
    Vegetables,
    Fruit,
    Herbs,
    Dairy,
    Cheese,
    Eggs,
    Meat,
    Fish,
    Bread,
    Grains,
    Legumes,
    Spices,
    Sauces,
    Sweets,
    Drinks,
    Nuts,
    Leftovers,
    Other,
}

impl From<FoodCategory> for FoodCategoryView {
    fn from(c: FoodCategory) -> Self {
        use FoodCategory as C;
        match c {
            C::Vegetables => Self::Vegetables,
            C::Fruit => Self::Fruit,
            C::Herbs => Self::Herbs,
            C::Dairy => Self::Dairy,
            C::Cheese => Self::Cheese,
            C::Eggs => Self::Eggs,
            C::Meat => Self::Meat,
            C::Fish => Self::Fish,
            C::Bread => Self::Bread,
            C::Grains => Self::Grains,
            C::Legumes => Self::Legumes,
            C::Spices => Self::Spices,
            C::Sauces => Self::Sauces,
            C::Sweets => Self::Sweets,
            C::Drinks => Self::Drinks,
            C::Nuts => Self::Nuts,
            C::Leftovers => Self::Leftovers,
            C::Other => Self::Other,
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InventoryView {
    /// In stock, in the order to use them.
    pub lots: Vec<LotView>,
    pub events: u32,
    /// Open + replay at start, and this command, in milliseconds.
    pub open_ms: f64,
    pub query_ms: f64,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddInput {
    pub item: String,
    pub qty: String,
    pub location: String,
    pub best_before: Option<String>,
    pub store: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LotAction {
    /// Eat or cook `qty` (empty: all of it).
    Use {
        qty: Option<String>,
    },
    Open,
    Bin,
    /// Entered by mistake: remove without counting it as eaten or wasted.
    Wrong,
    Move {
        to: String,
    },
    /// Still there.
    Seen,
}

fn view(l: &Lot, s: Status) -> LotView {
    let (status, days) = match (l.gone, s) {
        (Some(Gone::Used), _) => (LotStatus::Used, None),
        (Some(Gone::Binned), _) => (LotStatus::Binned, None),
        (Some(Gone::Retracted), _) => (LotStatus::Retracted, None),
        (None, Status::Expired(d)) => (LotStatus::Expired, Some(d as i32)),
        (None, Status::UseSoon(d)) => (LotStatus::UseSoon, Some(d as i32)),
        (None, Status::Fresh(d)) => (LotStatus::Fresh, Some(d as i32)),
        (None, Status::Unconfirmed(d)) => (LotStatus::Unconfirmed, Some(d as i32)),
        (None, Status::Undated) => (LotStatus::Undated, None),
    };
    LotView {
        id: l.id.to_string(),
        short: l.id.short(),
        item: l.item.clone(),
        left: l.left.to_string(),
        location: l.location.as_str().into(),
        status,
        days,
        best_before: l.best_before.map(|d| d.to_string()),
        opened: l.opened_at.is_some(),
        frozen_on: l
            .frozen_at
            .map(|t| t.with_timezone(&Local).date_naive().to_string()),
        store: match &l.source {
            Source::Receipt { store, .. } => Some(store.clone()),
            _ => None,
        },
        category: category(&l.item).into(),
    }
}

fn bad(field: &str) -> CommandError {
    CommandError::BadInput(field.into())
}

#[tauri::command]
#[specta::specta]
fn get_inventory(state: State<'_, AppState>) -> Result<InventoryView, CommandError> {
    let t = Instant::now();
    let store = state.store()?;
    let today = Local::now().date_naive();
    let lots = use_first(store.inventory().in_stock(), today)
        .into_iter()
        .map(|(l, s)| view(l, s))
        .collect();
    Ok(InventoryView {
        lots,
        events: store.event_count() as u32,
        open_ms: state.open_ms,
        query_ms: t.elapsed().as_secs_f64() * 1e3,
    })
}

#[tauri::command]
#[specta::specta]
fn add_item(state: State<'_, AppState>, input: AddInput) -> Result<LotView, CommandError> {
    let today = Local::now().date_naive();
    let item = input.item.trim();
    if item.is_empty() {
        return Err(bad("item"));
    }
    let qty: Qty = if input.qty.trim().is_empty() {
        Qty::pieces(1)
    } else {
        input.qty.parse().map_err(|_| bad("qty"))?
    };
    let location: Location = input.location.parse().map_err(|_| bad("location"))?;
    let best_before = match input.best_before.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(s) => Some(parse_date(s, today).ok_or_else(|| bad("bestBefore"))?),
    };
    let source = match input.store.as_deref().map(str::trim) {
        None | Some("") => Source::Manual,
        Some(s) => Source::Receipt {
            store: s.into(),
            line: None,
            price_cents: None,
            receipt: None,
        },
    };
    let lot = LotId::new();
    let mut store = state.store()?;
    store.append(&[Event {
        at: Utc::now(),
        lot,
        kind: EventKind::Acquired {
            item: item.into(),
            qty,
            location,
            source,
            best_before,
        },
    }])?;
    let l = store.inventory().get(lot).expect("just added");
    Ok(view(l, status(l, today)))
}

#[tauri::command]
#[specta::specta]
fn lot_action(
    state: State<'_, AppState>,
    id: String,
    action: LotAction,
) -> Result<LotView, CommandError> {
    let today = Local::now().date_naive();
    let mut store = state.store()?;
    let lot = store.find_lot(&id)?;
    let kind = match action {
        LotAction::Use { qty } => EventKind::Used {
            qty: match qty.as_deref().map(str::trim) {
                None | Some("") => None,
                Some(q) => Some(q.parse().map_err(|_| bad("qty"))?),
            },
        },
        LotAction::Open => EventKind::Opened,
        LotAction::Bin => EventKind::Binned,
        LotAction::Wrong => EventKind::Retracted,
        LotAction::Move { to } => EventKind::Moved {
            to: to.parse().map_err(|_| bad("to"))?,
        },
        LotAction::Seen => {
            let location = store.inventory().get(lot).expect("found").location;
            EventKind::Seen {
                location,
                left: None,
            }
        }
    };
    store
        .append(&[Event {
            at: Utc::now(),
            lot,
            kind,
        }])
        .map_err(|e| match e {
            StoreError::Ledger(_) => bad("action"),
            other => other.into(),
        })?;
    let l = store.inventory().get(lot).expect("found");
    Ok(view(l, status(l, today)))
}

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        get_inventory,
        add_item,
        lot_action,
        receipt::parse_receipt,
        receipt::import_receipt,
        receipt::read_receipt_file,
        photo::scan_photo,
        photo::apply_photo,
        recipes::list_templates,
        recipes::save_template,
        recipes::delete_template,
        recipes::order_templates,
        recipes::suggest_recipes,
        recipes::write_recipe,
        recipes::recipe_chat,
        recipes::pick_dish_art,
        settings::get_ai_settings,
        settings::set_ai_settings,
        settings::check_cloud,
        plan::save_favourite,
        plan::list_favourites,
        plan::delete_favourite,
        plan::update_favourite,
        plan::cooked,
        plan::make_plan,
        plan::get_plan,
        plan::list_plans,
        plan::swap_day,
        plan::set_day_cooked,
        plan::delete_plan,
        plan::list_shopping,
        plan::add_shopping,
        plan::add_plan_to_shopping,
        plan::set_shopping_done,
        plan::delete_shopping,
        plan::clear_done_shopping,
        plan::put_away_shopping,
        sync::get_sync_status,
        sync::set_sharing,
        sync::new_pairing_code,
        sync::pair_with_mac,
        sync::forget_mac,
        sync::sync_now,
        phone::get_phone_ai,
        phone::set_phone_ai,
        phone::download_phone_model,
        phone::cancel_phone_download,
        phone::delete_phone_model
    ])
}

pub fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../ui/src/lib/bindings.ts",
        )
        .expect("failed to export typescript bindings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();
    #[cfg(all(debug_assertions, not(mobile)))]
    export_bindings(&builder);

    let tauri_builder = tauri::Builder::default();
    // The camera, for scanning the Mac's pairing QR code (phones only).
    #[cfg(mobile)]
    let tauri_builder = tauri_builder.plugin(tauri_plugin_barcode_scanner::init());
    tauri_builder
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            // Desktop: the shared data folder (CLI and app use one file).
            // Mobile: the app's private folder, from Tauri.
            #[cfg(mobile)]
            let dir = {
                let d = app.path().app_data_dir()?;
                std::fs::create_dir_all(&d)?;
                d
            };
            #[cfg(not(mobile))]
            let dir = default_data_dir();
            let models_dir = dir.join("models");
            let t = Instant::now();
            let store = Store::open_dir(&dir)?;
            let open_ms = t.elapsed().as_secs_f64() * 1e3;
            app.manage(AppState {
                store: Mutex::new(store),
                open_ms,
                last_scan: Mutex::new(None),
                last_suggestions: Mutex::new(None),
                phone: Arc::new(phone::PhoneState::new(models_dir)),
            });
            sync::init(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Nookery");
}

#[cfg(test)]
mod tests {
    /// Keeps `ui/src/lib/bindings.ts` in step with the commands: run
    /// `cargo test -p nookery` after changing one, and commit the result.
    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
