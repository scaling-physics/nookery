//! What to do with a lot and when: the "use first" order.
//!
//! M0 uses only printed best-before dates and how long since anything
//! confirmed the lot. Shelf-life estimates (USDA FoodKeeper: pantry, fridge,
//! freezer, once opened) arrive in M4 and fill in missing dates.

use chrono::NaiveDate;
use serde::Serialize;

use crate::{Location, Lot};

/// Days before the best-before date that count as "use soon".
pub const USE_SOON_DAYS: i64 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "days")]
pub enum Status {
    /// Past its best-before date (days ago).
    Expired(i64),
    /// Within `USE_SOON_DAYS` of its date (days left).
    UseSoon(i64),
    /// Not confirmed for a long time: probably used up; ask before removing.
    Unconfirmed(i64),
    /// Has a date further out (days left).
    Fresh(i64),
    /// No date known.
    Undated,
}

/// After this many days without a receipt, photo or tap, a lot is "probably
/// gone" and the app asks.
pub fn confirm_window_days(location: Location) -> i64 {
    match location {
        Location::Fridge => 14,
        Location::Pantry => 90,
        Location::Freezer => 180,
    }
}

pub fn status(lot: &Lot, today: NaiveDate) -> Status {
    // Freezing pauses the printed date; freezer quality windows come in M4.
    let date = if lot.frozen_at.is_some() {
        None
    } else {
        lot.best_before
    };
    if let Some(bb) = date {
        let left = (bb - today).num_days();
        if left < 0 {
            return Status::Expired(-left);
        }
        if left <= USE_SOON_DAYS {
            return Status::UseSoon(left);
        }
    }
    let unseen = (today - lot.confirmed_at.date_naive()).num_days();
    if unseen > confirm_window_days(lot.location) {
        return Status::Unconfirmed(unseen);
    }
    match date {
        Some(bb) => Status::Fresh((bb - today).num_days()),
        None => Status::Undated,
    }
}

/// Lots in the order to use them: expired, then soonest dates, then the rest.
pub fn use_first<'a>(
    lots: impl IntoIterator<Item = &'a Lot>,
    today: NaiveDate,
) -> Vec<(&'a Lot, Status)> {
    let mut v: Vec<_> = lots.into_iter().map(|l| (l, status(l, today))).collect();
    v.sort_by(|(a, sa), (b, sb)| rank(sa).cmp(&rank(sb)).then_with(|| a.item.cmp(&b.item)));
    v
}

/// Sort key: expired (longest ago first), use soon / fresh by days left,
/// then unconfirmed, then undated.
fn rank(s: &Status) -> (u8, i64) {
    match *s {
        Status::Expired(d) => (0, -d),
        Status::UseSoon(d) => (1, d),
        Status::Fresh(d) => (2, d),
        Status::Unconfirmed(d) => (3, -d),
        Status::Undated => (4, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LotId, Qty, Source};
    use chrono::{DateTime, Utc};

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn lot(item: &str, location: Location, bb: Option<u32>, confirmed: u32) -> Lot {
        let at: DateTime<Utc> = day(confirmed).and_hms_opt(12, 0, 0).unwrap().and_utc();
        Lot {
            id: LotId::new(),
            item: item.into(),
            bought: Qty::pieces(1),
            left: Qty::pieces(1),
            location,
            source: Source::Manual,
            acquired_at: at,
            best_before: bb.map(day),
            opened_at: None,
            frozen_at: (location == Location::Freezer).then_some(at),
            confirmed_at: at,
            gone: None,
        }
    }

    #[test]
    fn classifies_by_date_and_confirmation() {
        let today = day(10);
        assert_eq!(
            status(&lot("a", Location::Fridge, Some(8), 9), today),
            Status::Expired(2)
        );
        assert_eq!(
            status(&lot("b", Location::Fridge, Some(12), 9), today),
            Status::UseSoon(2)
        );
        assert_eq!(
            status(&lot("c", Location::Fridge, Some(20), 9), today),
            Status::Fresh(10)
        );
        assert_eq!(
            status(&lot("d", Location::Pantry, None, 9), today),
            Status::Undated
        );
        // Fridge window is 14 days.
        let mut old = lot("e", Location::Fridge, Some(30), 1);
        old.confirmed_at =
            day(1).and_hms_opt(0, 0, 0).unwrap().and_utc() - chrono::Duration::days(20);
        assert!(matches!(status(&old, today), Status::Unconfirmed(_)));
    }

    #[test]
    fn freezing_pauses_the_printed_date() {
        let frozen = lot("mince", Location::Freezer, Some(5), 4);
        assert_eq!(status(&frozen, day(10)), Status::Undated);
    }

    #[test]
    fn use_first_puts_expired_and_soonest_first() {
        let today = day(10);
        let lots = [
            lot("rice", Location::Pantry, None, 9),
            lot("yoghurt", Location::Fridge, Some(11), 9),
            lot("cream", Location::Fridge, Some(7), 9),
            lot("cheese", Location::Fridge, Some(25), 9),
            lot("spinach", Location::Fridge, Some(10), 9),
        ];
        let order: Vec<_> = use_first(&lots, today)
            .iter()
            .map(|(l, _)| l.item.as_str())
            .collect();
        assert_eq!(order, ["cream", "spinach", "yoghurt", "cheese", "rice"]);
    }
}
