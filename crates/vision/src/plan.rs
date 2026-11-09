//! From a scan to what the user reviews, and from their answers to events.
//! Shared by the CLI and the app so both behave the same.
//!
//! - Each sighting's name goes through the learned photo aliases first
//!   ("clear jar with green contents" -> "pesto" once the user said so),
//!   then `nookery_core::reconcile` against the lots in that place.
//! - Matched lots get a `Seen` (keeps them out of "probably used up").
//! - New sightings are only added when the user says so: photo guesses
//!   are noisy, and a wrong lot is worse than a missing one.
//! - Missing lots are only listed; the user decides (use / bin / still
//!   there), through the normal lot actions.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use nookery_core::{
    name_key, reconcile, Alias, Event, EventKind, Location, Lot, LotId, Observed, Qty, Source,
};

use crate::Scan;

/// One sighting, ready for review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanItem {
    /// Index into `Scan::sightings`.
    pub sighting: usize,
    /// What the model said.
    pub seen_as: String,
    /// The learned name if there is one, else what the model said.
    pub item: String,
    /// `item` came from a learned photo alias.
    pub learned: bool,
    /// In-stock lots in this place it matched (empty: new).
    pub lots: Vec<LotId>,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub place: Location,
    /// One per sighting, in scan order.
    pub items: Vec<PlanItem>,
    /// In stock in this place but not in the photo: ask.
    pub missing: Vec<LotId>,
}

/// `aliases`: `Store::aliases(PHOTO_ALIASES)` (key: `name_key` of the
/// model's name).
pub fn plan<'a>(
    scan: &Scan,
    aliases: &HashMap<String, Alias>,
    lots: impl IntoIterator<Item = &'a Lot>,
    place: Location,
) -> Plan {
    let named: Vec<(String, bool)> = scan
        .sightings
        .iter()
        .map(|s| match aliases.get(&name_key(&s.name)) {
            Some(a) => (a.item.clone(), true),
            None => (s.name.clone(), false),
        })
        .collect();
    let observed: Vec<Observed<'_>> = named
        .iter()
        .zip(&scan.sightings)
        .map(|((name, _), s)| Observed {
            name,
            count: s.count,
        })
        .collect();
    let r = reconcile(&observed, lots, place);
    let items = scan
        .sightings
        .iter()
        .enumerate()
        .map(|(i, s)| PlanItem {
            sighting: i,
            seen_as: s.name.clone(),
            item: named[i].0.clone(),
            learned: named[i].1,
            lots: r
                .matched
                .iter()
                .filter(|(si, _)| *si == i)
                .map(|(_, id)| *id)
                .collect(),
            count: s.count,
        })
        .collect();
    Plan {
        place,
        items,
        missing: r.missing,
    }
}

/// What the user decided for one sighting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// It's the matched lot(s): mark them seen. Only valid for matched items.
    Seen,
    /// Add a new lot (the user may have corrected the name).
    Add {
        item: String,
        qty: Qty,
        best_before: Option<NaiveDate>,
    },
    /// Not food, or a wrong guess: nothing is written.
    Ignore,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecisionError {
    #[error("sighting {0} is not in the scan")]
    BadSighting(usize),
    #[error("sighting {0} matched nothing, so it can't be marked seen")]
    NothingMatched(usize),
    #[error("sighting {0} is decided twice")]
    Duplicate(usize),
    #[error("sighting {0} needs a name")]
    NoName(usize),
}

/// `(events, learned aliases)`.
pub type Decided = (Vec<Event>, Vec<(String, Alias)>);

/// The events to append and the photo aliases to learn
/// (`Store::append_learning(events, PHOTO_ALIASES, learned, at)`).
/// An alias is learned whenever the user names an added item differently
/// from what the model said, so the next photo uses their name.
pub fn decide(
    plan: &Plan,
    decisions: &[(usize, Decision)],
    at: DateTime<Utc>,
) -> Result<Decided, DecisionError> {
    let mut events = Vec::new();
    let mut learned = Vec::new();
    let mut done = vec![false; plan.items.len()];
    for (i, d) in decisions {
        let p = plan.items.get(*i).ok_or(DecisionError::BadSighting(*i))?;
        if std::mem::replace(&mut done[*i], true) {
            return Err(DecisionError::Duplicate(*i));
        }
        match d {
            Decision::Seen => {
                if p.lots.is_empty() {
                    return Err(DecisionError::NothingMatched(*i));
                }
                events.extend(p.lots.iter().map(|lot| Event {
                    at,
                    lot: *lot,
                    kind: EventKind::Seen {
                        location: plan.place,
                        left: None,
                    },
                }));
            }
            Decision::Add {
                item,
                qty,
                best_before,
            } => {
                let item = item.trim();
                if item.is_empty() {
                    return Err(DecisionError::NoName(*i));
                }
                events.push(Event {
                    at,
                    lot: LotId::new(),
                    kind: EventKind::Acquired {
                        item: item.to_string(),
                        qty: *qty,
                        location: plan.place,
                        source: Source::Photo,
                        best_before: *best_before,
                    },
                });
                if !item.eq_ignore_ascii_case(p.seen_as.trim()) {
                    learned.push((
                        name_key(&p.seen_as),
                        Alias {
                            item: item.to_string(),
                            each: None,
                            location: plan.place,
                        },
                    ));
                }
            }
            Decision::Ignore => {}
        }
    }
    Ok((events, learned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sighting;
    use nookery_core::Inventory;

    fn scan(names: &[&str]) -> Scan {
        Scan {
            sightings: names
                .iter()
                .map(|n| Sighting {
                    name: n.to_string(),
                    text: None,
                    count: 1,
                    tiles: vec![0],
                })
                .collect(),
            model: "fake".into(),
            tiles: vec![],
            prepare_ms: 0.0,
            total_ms: 0.0,
        }
    }

    fn stock(items: &[&str]) -> (Inventory, Vec<LotId>) {
        let events: Vec<Event> = items
            .iter()
            .map(|i| Event {
                at: "2026-11-01T10:00:00Z".parse().unwrap(),
                lot: LotId::new(),
                kind: EventKind::Acquired {
                    item: i.to_string(),
                    qty: Qty::pieces(1),
                    location: Location::Fridge,
                    source: Source::Manual,
                    best_before: None,
                },
            })
            .collect();
        let ids = events.iter().map(|e| e.lot).collect();
        (Inventory::replay(&events).unwrap(), ids)
    }

    #[test]
    fn learned_names_are_used_before_matching() {
        let (inv, ids) = stock(&["pesto", "skyr"]);
        let aliases = HashMap::from([(
            name_key("clear jar with green contents"),
            Alias {
                item: "pesto".into(),
                each: None,
                location: Location::Fridge,
            },
        )]);
        let p = plan(
            &scan(&["clear jar with green contents", "zucchini"]),
            &aliases,
            inv.all(),
            Location::Fridge,
        );
        assert_eq!(p.items[0].item, "pesto");
        assert!(p.items[0].learned);
        assert_eq!(p.items[0].lots, vec![ids[0]]);
        assert!(p.items[1].lots.is_empty());
        assert_eq!(p.missing, vec![ids[1]]);
    }

    #[test]
    fn decisions_become_seen_and_acquired_events_and_learn_renames() {
        let (inv, ids) = stock(&["skyr"]);
        let p = plan(
            &scan(&["Arla Skyr", "jar with green contents", "fish & meat label"]),
            &HashMap::new(),
            inv.all(),
            Location::Fridge,
        );
        let at = "2026-11-09T10:00:00Z".parse().unwrap();
        let (events, learned) = decide(
            &p,
            &[
                (0, Decision::Seen),
                (
                    1,
                    Decision::Add {
                        item: "pesto".into(),
                        qty: Qty::pieces(1),
                        best_before: None,
                    },
                ),
                (2, Decision::Ignore),
            ],
            at,
        )
        .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].lot, ids[0]);
        assert!(matches!(events[0].kind, EventKind::Seen { .. }));
        assert!(matches!(
            &events[1].kind,
            EventKind::Acquired { item, source: Source::Photo, .. } if item == "pesto"
        ));
        assert_eq!(learned.len(), 1);
        assert_eq!(learned[0].0, name_key("jar with green contents"));
        let mut next = inv.clone();
        for e in &events {
            next.apply(e).unwrap();
        }
    }

    #[test]
    fn bad_decisions_are_refused() {
        let (inv, _) = stock(&[]);
        let p = plan(
            &scan(&["zucchini"]),
            &HashMap::new(),
            inv.all(),
            Location::Fridge,
        );
        let at = Utc::now();
        assert_eq!(
            decide(&p, &[(0, Decision::Seen)], at),
            Err(DecisionError::NothingMatched(0))
        );
        assert_eq!(
            decide(&p, &[(5, Decision::Ignore)], at),
            Err(DecisionError::BadSighting(5))
        );
        assert_eq!(
            decide(&p, &[(0, Decision::Ignore), (0, Decision::Ignore)], at),
            Err(DecisionError::Duplicate(0))
        );
        let blank = Decision::Add {
            item: " ".into(),
            qty: Qty::pieces(1),
            best_before: None,
        };
        assert_eq!(decide(&p, &[(0, blank)], at), Err(DecisionError::NoName(0)));
    }
}
