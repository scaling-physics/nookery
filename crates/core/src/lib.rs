//! Nookery's domain: quantities, the event ledger, the inventory computed
//! from it, and what to use first. Pure: no I/O, no clock (callers pass
//! `today`), so everything here is unit-tested.

mod ai;
mod alias;
mod category;
mod dates;
mod event;
mod inventory;
mod qty;
mod reconcile;
mod status;

pub use ai::{AiMode, CLOUD_MODEL};
pub use alias::Alias;
pub use category::{category, FoodCategory};
pub use dates::parse_date;
pub use event::{Event, EventKind, Location, LotId, Source};
pub use inventory::{Gone, Inventory, LedgerError, Lot};
pub use qty::{Qty, QtyParseError, Unit};
pub use reconcile::{name_key, names_match, reconcile, Observed, Reconciled};
pub use status::{confirm_window_days, status, use_first, Status, USE_SOON_DAYS};
