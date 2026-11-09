//! The inventory, computed by replaying ledger events in order.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use crate::{Event, EventKind, Location, LotId, Qty, Source};

/// How a lot left the kitchen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Gone {
    Used,
    Binned,
}

/// One batch of one thing, as the events left it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lot {
    pub id: LotId,
    pub item: String,
    /// What was acquired.
    pub bought: Qty,
    /// What is left.
    pub left: Qty,
    pub location: Location,
    pub source: Source,
    pub acquired_at: DateTime<Utc>,
    pub best_before: Option<NaiveDate>,
    pub opened_at: Option<DateTime<Utc>>,
    /// When it last went into the freezer (None once it comes out).
    pub frozen_at: Option<DateTime<Utc>>,
    /// The last time something confirmed it exists: acquiring, a photo, a tap.
    pub confirmed_at: DateTime<Utc>,
    pub gone: Option<Gone>,
}

impl Lot {
    pub fn in_stock(&self) -> bool {
        self.gone.is_none()
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum LedgerError {
    #[error("lot {0} was already acquired")]
    DuplicateLot(LotId),
    #[error("no lot {0}")]
    UnknownLot(LotId),
    #[error("lot {0} is already gone")]
    AlreadyGone(LotId),
    #[error("lot {0} is counted in {1:?}, not {2:?}")]
    UnitMismatch(LotId, crate::Unit, crate::Unit),
}

/// All lots ever seen, in the order they were acquired.
#[derive(Clone, Debug, Default)]
pub struct Inventory {
    lots: Vec<Lot>,
    index: HashMap<LotId, usize>,
}

impl Inventory {
    /// Replays `events` (oldest first). An invalid event is an error: the
    /// store only appends events that `apply` accepted.
    pub fn replay<'a>(events: impl IntoIterator<Item = &'a Event>) -> Result<Self, LedgerError> {
        let mut inv = Inventory::default();
        for e in events {
            inv.apply(e)?;
        }
        Ok(inv)
    }

    /// Checks `event` against the current state without changing it.
    pub fn check(&self, event: &Event) -> Result<(), LedgerError> {
        self.clone().apply(event)
    }

    pub fn apply(&mut self, event: &Event) -> Result<(), LedgerError> {
        let id = event.lot;
        if let EventKind::Acquired {
            item,
            qty,
            location,
            source,
            best_before,
        } = &event.kind
        {
            if self.index.contains_key(&id) {
                return Err(LedgerError::DuplicateLot(id));
            }
            self.index.insert(id, self.lots.len());
            self.lots.push(Lot {
                id,
                item: item.clone(),
                bought: *qty,
                left: *qty,
                location: *location,
                source: source.clone(),
                acquired_at: event.at,
                best_before: *best_before,
                opened_at: None,
                frozen_at: (*location == Location::Freezer).then_some(event.at),
                confirmed_at: event.at,
                gone: None,
            });
            return Ok(());
        }
        let &i = self.index.get(&id).ok_or(LedgerError::UnknownLot(id))?;
        let lot = &mut self.lots[i];
        if lot.gone.is_some() {
            return Err(LedgerError::AlreadyGone(id));
        }
        let unit_ok = |q: &Qty| {
            if q.unit == lot.left.unit {
                Ok(())
            } else {
                Err(LedgerError::UnitMismatch(id, lot.left.unit, q.unit))
            }
        };
        match &event.kind {
            EventKind::Acquired { .. } => unreachable!(),
            EventKind::Seen { location, left } => {
                if let Some(q) = left {
                    unit_ok(q)?;
                }
                move_to(lot, *location, event.at);
                lot.confirmed_at = event.at;
                if let Some(q) = left {
                    lot.left = *q;
                }
            }
            EventKind::Moved { to } => {
                move_to(lot, *to, event.at);
                lot.confirmed_at = event.at;
            }
            EventKind::Opened => {
                lot.opened_at.get_or_insert(event.at);
                lot.confirmed_at = event.at;
            }
            EventKind::Used { qty } => {
                if let Some(q) = qty {
                    unit_ok(q)?;
                }
                lot.left = match qty {
                    Some(q) => lot.left.minus(*q).expect("unit checked"),
                    None => Qty {
                        milli: 0,
                        unit: lot.left.unit,
                    },
                };
                if lot.left.is_zero() {
                    lot.gone = Some(Gone::Used);
                } else {
                    lot.confirmed_at = event.at;
                }
            }
            EventKind::Binned => lot.gone = Some(Gone::Binned),
        }
        Ok(())
    }

    pub fn get(&self, id: LotId) -> Option<&Lot> {
        self.index.get(&id).map(|&i| &self.lots[i])
    }

    /// Every lot, gone ones too.
    pub fn all(&self) -> &[Lot] {
        &self.lots
    }

    pub fn in_stock(&self) -> impl Iterator<Item = &Lot> {
        self.lots.iter().filter(|l| l.in_stock())
    }
}

fn move_to(lot: &mut Lot, to: Location, at: DateTime<Utc>) {
    if lot.location == to {
        return;
    }
    lot.frozen_at = (to == Location::Freezer).then_some(at);
    lot.location = to;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Utc> {
        format!("2026-11-{s}:00Z").parse().unwrap()
    }

    fn ev(lot: LotId, when: &str, kind: EventKind) -> Event {
        Event {
            at: at(when),
            lot,
            kind,
        }
    }

    fn eggs(lot: LotId) -> Event {
        ev(
            lot,
            "03T10:00",
            EventKind::Acquired {
                item: "eggs".into(),
                qty: Qty::pieces(6),
                location: Location::Fridge,
                source: Source::Receipt {
                    store: "Lidl".into(),
                    line: None,
                    price_cents: Some(229),
                    receipt: None,
                },
                best_before: NaiveDate::from_ymd_opt(2026, 11, 20),
            },
        )
    }

    #[test]
    fn using_part_then_the_rest_marks_the_lot_used() {
        let id = LotId::new();
        let events = [
            eggs(id),
            ev(
                id,
                "04T08:00",
                EventKind::Used {
                    qty: Some(Qty::pieces(2)),
                },
            ),
            ev(
                id,
                "05T08:00",
                EventKind::Used {
                    qty: Some(Qty::pieces(4)),
                },
            ),
        ];
        let inv = Inventory::replay(&events).unwrap();
        let lot = inv.get(id).unwrap();
        assert_eq!(lot.left, Qty::pieces(0));
        assert_eq!(lot.gone, Some(Gone::Used));
        assert_eq!(inv.in_stock().count(), 0);
    }

    #[test]
    fn freezing_records_when_and_thawing_clears_it() {
        let id = LotId::new();
        let mut inv = Inventory::replay(&[eggs(id)]).unwrap();
        inv.apply(&ev(
            id,
            "04T09:00",
            EventKind::Moved {
                to: Location::Freezer,
            },
        ))
        .unwrap();
        assert_eq!(inv.get(id).unwrap().frozen_at, Some(at("04T09:00")));
        inv.apply(&ev(
            id,
            "06T09:00",
            EventKind::Moved {
                to: Location::Fridge,
            },
        ))
        .unwrap();
        assert_eq!(inv.get(id).unwrap().frozen_at, None);
    }

    #[test]
    fn a_photo_confirms_and_can_correct_the_amount() {
        let id = LotId::new();
        let mut inv = Inventory::replay(&[eggs(id)]).unwrap();
        inv.apply(&ev(
            id,
            "07T18:00",
            EventKind::Seen {
                location: Location::Fridge,
                left: Some(Qty::pieces(3)),
            },
        ))
        .unwrap();
        let lot = inv.get(id).unwrap();
        assert_eq!(lot.left, Qty::pieces(3));
        assert_eq!(lot.confirmed_at, at("07T18:00"));
    }

    #[test]
    fn rejects_events_that_make_no_sense() {
        let id = LotId::new();
        let mut inv = Inventory::replay(&[eggs(id)]).unwrap();
        assert_eq!(inv.check(&eggs(id)), Err(LedgerError::DuplicateLot(id)));
        let other = LotId::new();
        assert_eq!(
            inv.check(&ev(other, "04T08:00", EventKind::Opened)),
            Err(LedgerError::UnknownLot(other))
        );
        assert!(matches!(
            inv.check(&ev(
                id,
                "04T08:00",
                EventKind::Used {
                    qty: Some(Qty::grams(10))
                }
            )),
            Err(LedgerError::UnitMismatch(..))
        ));
        inv.apply(&ev(id, "04T08:00", EventKind::Binned)).unwrap();
        assert_eq!(
            inv.check(&ev(id, "05T08:00", EventKind::Opened)),
            Err(LedgerError::AlreadyGone(id))
        );
    }

    #[test]
    fn opening_keeps_the_first_time() {
        let id = LotId::new();
        let mut inv = Inventory::replay(&[eggs(id)]).unwrap();
        inv.apply(&ev(id, "04T08:00", EventKind::Opened)).unwrap();
        inv.apply(&ev(id, "05T08:00", EventKind::Opened)).unwrap();
        assert_eq!(inv.get(id).unwrap().opened_at, Some(at("04T08:00")));
    }
}
