//! The ledger: everything that happens to food is an event about one lot
//! (a batch of one thing: "the 6 eggs bought at Lidl on 3 Nov"). The
//! inventory is computed from the events and never stored on its own, so
//! every number can be explained and nothing is lost.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Qty;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LotId(pub Uuid);

impl LotId {
    /// Time-ordered (UUID v7), so ids sort by creation.
    pub fn new() -> Self {
        LotId(Uuid::now_v7())
    }

    /// The last 8 hex characters: random, unlike the timestamp at the start.
    pub fn short(&self) -> String {
        let s = self.to_string();
        s[s.len() - 8..].to_string()
    }
}

impl Default for LotId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.simple().fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    Fridge,
    Freezer,
    Pantry,
}

impl Location {
    pub const ALL: [Location; 3] = [Location::Fridge, Location::Freezer, Location::Pantry];

    pub fn as_str(&self) -> &'static str {
        match self {
            Location::Fridge => "fridge",
            Location::Freezer => "freezer",
            Location::Pantry => "pantry",
        }
    }
}

impl std::str::FromStr for Location {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fridge" => Ok(Location::Fridge),
            "freezer" => Ok(Location::Freezer),
            "pantry" | "cupboard" => Ok(Location::Pantry),
            _ => Err(format!("unknown place {s:?} (fridge, freezer, pantry)")),
        }
    }
}

/// Where a lot first came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Source {
    /// A line on a store receipt (paper photo or digital).
    Receipt {
        store: String,
        /// What the receipt printed, e.g. "BIO H-MILCH 1,5%".
        line: Option<String>,
        price_cents: Option<i64>,
    },
    /// Spotted in a fridge/freezer/pantry photo without a matching purchase
    /// (leftovers, gifts, things cooked at home).
    Photo,
    /// Typed in by hand.
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// A new lot enters the kitchen.
    Acquired {
        item: String,
        qty: Qty,
        location: Location,
        source: Source,
        best_before: Option<NaiveDate>,
    },
    /// A photo (or the user) confirmed the lot is still there; `left` when
    /// the amount could be judged.
    Seen {
        location: Location,
        left: Option<Qty>,
    },
    /// Put somewhere else; moving into the freezer is freezing.
    Moved {
        to: Location,
    },
    Opened,
    /// Some or (None) all of it eaten or cooked.
    Used {
        qty: Option<Qty>,
    },
    /// Thrown away.
    Binned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub at: DateTime<Utc>,
    pub lot: LotId,
    #[serde(flatten)]
    pub kind: EventKind,
}

impl EventKind {
    /// A stable short name, also the `kind` column in the store.
    pub fn name(&self) -> &'static str {
        match self {
            EventKind::Acquired { .. } => "acquired",
            EventKind::Seen { .. } => "seen",
            EventKind::Moved { .. } => "moved",
            EventKind::Opened => "opened",
            EventKind::Used { .. } => "used",
            EventKind::Binned => "binned",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_round_trip_through_json() {
        let e = Event {
            at: "2026-11-08T10:00:00Z".parse().unwrap(),
            lot: LotId::new(),
            kind: EventKind::Acquired {
                item: "milk".into(),
                qty: Qty::millilitres(1000),
                location: Location::Fridge,
                source: Source::Receipt {
                    store: "Lidl".into(),
                    line: Some("H-MILCH 1,5%".into()),
                    price_cents: Some(109),
                },
                best_before: NaiveDate::from_ymd_opt(2026, 11, 20),
            },
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains(r#""type":"acquired""#), "{json}");
        assert_eq!(serde_json::from_str::<Event>(&json).unwrap(), e);
    }

    #[test]
    fn lot_ids_sort_by_creation() {
        let a = LotId::new();
        let b = LotId::new();
        assert!(a < b);
    }
}
