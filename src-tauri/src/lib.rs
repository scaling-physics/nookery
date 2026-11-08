//! The Tauri app: thin commands over `nookery-store`. All rules live in the
//! crates; this file only converts between them and the UI's types.

use std::sync::Mutex;
use std::time::Instant;

use chrono::{Local, Utc};
use nookery_core::{
    parse_date, status, use_first, Event, EventKind, Gone, Location, Lot, LotId, Qty, Source,
    Status,
};
use nookery_store::{default_data_dir, Store, StoreError};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tauri_specta::{collect_commands, Builder};

pub struct AppState {
    store: Mutex<Store>,
    /// How long opening the database and replaying the ledger took at start.
    open_ms: f64,
}

/// What the UI is told when a command fails.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum CommandError {
    /// A field the user typed can't be read (which field).
    BadInput(String),
    NotFound,
    Internal,
}

impl From<StoreError> for CommandError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NoMatch(_) | StoreError::Ambiguous(_) => Self::NotFound,
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
    }
}

fn bad(field: &str) -> CommandError {
    CommandError::BadInput(field.into())
}

#[tauri::command]
#[specta::specta]
fn get_inventory(state: State<'_, AppState>) -> Result<InventoryView, CommandError> {
    let t = Instant::now();
    let store = state.store.lock().map_err(|_| CommandError::Internal)?;
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
        },
    };
    let lot = LotId::new();
    let mut store = state.store.lock().map_err(|_| CommandError::Internal)?;
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
    let mut store = state.store.lock().map_err(|_| CommandError::Internal)?;
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
    Builder::<tauri::Wry>::new().commands(collect_commands![get_inventory, add_item, lot_action])
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
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    let t = Instant::now();
    let store = Store::open_dir(&default_data_dir()).expect("can't open the Nookery database");
    let open_ms = t.elapsed().as_secs_f64() * 1e3;

    tauri::Builder::default()
        .manage(AppState {
            store: Mutex::new(store),
            open_ms,
        })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
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
