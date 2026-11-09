//! Matching what a photo showed against what the ledger says is there.
//!
//! - matched: a sighting is an item we have in that place: refresh it
//!   (`Seen`), so it stays out of "probably used up";
//! - missing: we have it in that place but the photo doesn't show it: ASK
//!   (it may be hidden behind something); never remove on a guess;
//! - new: seen but never acquired: propose adding it (`Source::Photo`).
//!
//! A receipt line and a photo of the same thing are one lot, never two:
//! that's why new sightings are only what no in-stock lot matches.

use crate::{Location, Lot, LotId};

/// What the photo showed, as names (already mapped through any learned
/// photo alias by the caller).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observed<'a> {
    pub name: &'a str,
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reconciled {
    /// `(sighting index, lot)`: a sighting can match several lots of the
    /// same item (two packs of butter bought on different days); each lot
    /// matches at most one sighting.
    pub matched: Vec<(usize, LotId)>,
    /// In stock in this place, not matched by any sighting.
    pub missing: Vec<LotId>,
    /// Sighting indexes no lot matched.
    pub new: Vec<usize>,
}

/// Words that describe the container or the phrasing, not the product
/// ("jar of pickles" is "pickles"). Singular forms, checked after
/// singularising. Colours and sizes are deliberately NOT here: "red bell
/// pepper" and "green bell pepper" differ.
const FILLER: &[&str] = &[
    "a",
    "an",
    "the",
    "of",
    "with",
    "and",
    "fresh",
    "pack",
    "package",
    "bag",
    "box",
    "jar",
    "can",
    "tin",
    "bottle",
    "carton",
    "cup",
    "container",
    "plastic",
    "glass",
    "clear",
    "some",
    "piece",
];

/// Colour and size words: they may sit in a key but never justify a match
/// on their own ("red" must not match "red bell pepper").
const DESCRIPTORS: &[&str] = &[
    "red", "green", "yellow", "orange", "white", "black", "brown", "purple", "pink", "blue",
    "dark", "light", "golden", "small", "large", "big", "medium", "mini", "baby", "little",
];

/// Simple English singular. Words of 3 letters or fewer, and words ending
/// in "ss", "us" or "is" (hummus, asparagus, couscous, swiss, glass), stay.
fn singular(w: &str) -> String {
    let n = w.chars().count();
    if n <= 3 || w.ends_with("ss") || w.ends_with("us") || w.ends_with("is") {
        return w.to_string();
    }
    if n > 4 && w.ends_with("ies") {
        return format!("{}y", &w[..w.len() - 3]);
    }
    for suffix in ["oes", "ches", "shes", "xes", "sses"] {
        if w.ends_with(suffix) {
            return w[..w.len() - 2].to_string();
        }
    }
    match w.strip_suffix('s') {
        Some(rest) => rest.to_string(),
        None => w.to_string(),
    }
}

/// The sorted, de-duplicated words of a name's key.
fn key_words(name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let mut words: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(singular)
        .filter(|w| !FILLER.contains(&w.as_str()))
        .collect();
    words.sort();
    words.dedup();
    words
}

/// A comparable form of an item name: lower case (Unicode-aware, umlauts
/// kept), punctuation turned into spaces, container/filler words dropped
/// (`a an the of with and fresh pack package bag box jar can tin bottle
/// carton cup container plastic glass clear some piece`; colours and sizes
/// are kept), simple English plurals made singular (`tomatoes` ->
/// `tomato`, `berries` -> `berry`, `peppers` -> `pepper`; words ending in
/// "ss", "us" or "is" and words of 3 letters or fewer are left alone), then
/// words sorted, de-duplicated and joined by one space. "Red Bell Peppers"
/// and "bell pepper, red" give the same key. A name made only of filler
/// words gives an empty key.
pub fn name_key(name: &str) -> String {
    key_words(name).join(" ")
}

/// True when `small` is a non-empty subset of `big` and has at least one
/// word that is not a colour/size word.
fn subset_match(small: &[String], big: &[String]) -> bool {
    !small.is_empty()
        && small.iter().any(|w| !DESCRIPTORS.contains(&w.as_str()))
        && small.iter().all(|w| big.contains(w))
}

/// Whether two names mean the same item, by the rules `reconcile` uses:
/// equal keys, or one key's words a subset of the other's with a
/// non-colour/size word on the smaller side ("chicken" ~ "chicken breast",
/// "red" !~ "red bell pepper"). For recipes checking an ingredient against
/// the stock.
pub fn names_match(a: &str, b: &str) -> bool {
    let (a, b) = (key_words(a), key_words(b));
    !a.is_empty() && !b.is_empty() && (a == b || subset_match(&a, &b) || subset_match(&b, &a))
}

/// Matches sightings against the in-stock lots in `place` (lots elsewhere
/// and gone lots are ignored).
///
/// Rules: a sighting matches a lot when their keys are equal, or when one
/// key's words are a subset of the other's and the smaller set has a word
/// that is not a colour/size ("bell pepper" ~ "red bell pepper", "skyr" ~
/// "arla skyr"; "red onion" !~ "red bell pepper"). Known limitation: the
/// spice "pepper" also matches "bell pepper". Exact matches are made first;
/// subset matches only for lots still unmatched, and a lot with several
/// subset candidates takes the one with the closest word count (then the
/// lowest index). A lot matches at most one sighting; a sighting may match
/// several lots. Sightings with an empty key are ignored.
pub fn reconcile<'a>(
    observed: &[Observed<'_>],
    lots: impl IntoIterator<Item = &'a Lot>,
    place: Location,
) -> Reconciled {
    let sights: Vec<Vec<String>> = observed.iter().map(|o| key_words(o.name)).collect();
    let lots: Vec<(LotId, Vec<String>)> = lots
        .into_iter()
        .filter(|l| l.in_stock() && l.location == place)
        .map(|l| (l.id, key_words(&l.item)))
        .collect();

    // For each lot: the sighting it matched.
    let mut assigned: Vec<Option<usize>> = vec![None; lots.len()];

    // Pass 1: equal keys.
    for (li, (_, lw)) in lots.iter().enumerate() {
        if lw.is_empty() {
            continue;
        }
        assigned[li] = sights.iter().position(|sw| sw == lw);
    }
    // Pass 2: subsets, for what is left.
    for (li, (_, lw)) in lots.iter().enumerate() {
        if assigned[li].is_some() {
            continue;
        }
        assigned[li] = sights
            .iter()
            .enumerate()
            .filter(|(_, sw)| subset_match(lw, sw) || subset_match(sw, lw))
            .min_by_key(|(_, sw)| sw.len().abs_diff(lw.len()))
            .map(|(si, _)| si);
    }

    let mut matched: Vec<(usize, usize, LotId)> = Vec::new();
    let mut missing = Vec::new();
    for (li, (id, _)) in lots.iter().enumerate() {
        match assigned[li] {
            Some(si) => matched.push((si, li, *id)),
            None => missing.push(*id),
        }
    }
    matched.sort_by_key(|&(si, li, _)| (si, li));

    let new = (0..sights.len())
        .filter(|&si| !sights[si].is_empty() && !assigned.contains(&Some(si)))
        .collect();

    Reconciled {
        matched: matched.into_iter().map(|(si, _, id)| (si, id)).collect(),
        missing,
        new,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, EventKind, Inventory, Qty, Source};

    fn lot_events(item: &str, location: Location) -> (LotId, Event) {
        let id = LotId::new();
        let e = Event {
            at: "2026-11-03T10:00:00Z".parse().unwrap(),
            lot: id,
            kind: EventKind::Acquired {
                item: item.into(),
                qty: Qty::pieces(1),
                location,
                source: Source::Manual,
                best_before: None,
            },
        };
        (id, e)
    }

    fn inventory(items: &[(&str, Location)]) -> (Inventory, Vec<LotId>) {
        let (ids, events): (Vec<_>, Vec<_>) = items.iter().map(|(n, l)| lot_events(n, *l)).unzip();
        (Inventory::replay(&events).unwrap(), ids)
    }

    fn obs<'a>(names: &[&'a str]) -> Vec<Observed<'a>> {
        names
            .iter()
            .map(|n| Observed { name: n, count: 1 })
            .collect()
    }

    fn run(items: &[(&str, Location)], seen: &[&str]) -> (Reconciled, Vec<LotId>) {
        let (inv, ids) = inventory(items);
        let r = reconcile(&obs(seen), inv.all(), Location::Fridge);
        (r, ids)
    }

    const F: Location = Location::Fridge;

    #[test]
    fn name_key_makes_plurals_singular() {
        assert_eq!(name_key("tomatoes"), "tomato");
        assert_eq!(name_key("berries"), "berry");
        assert_eq!(name_key("peppers"), "pepper");
        assert_eq!(name_key("peaches"), "peach");
    }

    #[test]
    fn name_key_keeps_words_ending_in_ss_or_us() {
        assert_eq!(name_key("hummus"), "hummus");
        assert_eq!(name_key("asparagus"), "asparagus");
        assert_eq!(name_key("couscous"), "couscous");
        assert_eq!(name_key("Swiss"), "swiss");
    }

    #[test]
    fn name_key_drops_punctuation_and_filler_words() {
        assert_eq!(name_key("Jar of pickles"), "pickle");
        assert_eq!(name_key("a clear jar with pickles!"), "pickle");
        assert_eq!(name_key("fresh milk (carton)"), "milk");
        assert_eq!(name_key("clear jar"), "");
    }

    #[test]
    fn name_key_ignores_word_order_and_case() {
        assert_eq!(name_key("Red Bell Peppers"), name_key("bell pepper, red"));
        assert_eq!(name_key("Red Bell Peppers"), "bell pepper red");
    }

    #[test]
    fn name_key_keeps_colour_words() {
        assert_ne!(name_key("red bell pepper"), name_key("green bell pepper"));
    }

    #[test]
    fn name_key_keeps_umlauts() {
        assert_eq!(name_key("Äpfel Müsli"), "müsli äpfel");
        assert_eq!(name_key("ÖL"), "öl");
    }

    #[test]
    fn equal_keys_match_exactly() {
        let (r, ids) = run(&[("tomato", F)], &["Tomatoes"]);
        assert_eq!(r.matched, vec![(0, ids[0])]);
        assert!(r.missing.is_empty() && r.new.is_empty());
    }

    #[test]
    fn a_shorter_lot_name_matches_a_longer_sighting() {
        let (r, ids) = run(&[("bell pepper", F)], &["red bell pepper"]);
        assert_eq!(r.matched, vec![(0, ids[0])]);
        let (r, ids) = run(&[("skyr", F)], &["Arla Skyr"]);
        assert_eq!(r.matched, vec![(0, ids[0])]);
    }

    #[test]
    fn a_longer_lot_name_matches_a_shorter_sighting() {
        let (r, ids) = run(&[("arla skyr", F)], &["skyr"]);
        assert_eq!(r.matched, vec![(0, ids[0])]);
    }

    #[test]
    fn a_colour_word_alone_does_not_match() {
        let (r, ids) = run(&[("red", F)], &["red bell pepper"]);
        assert!(r.matched.is_empty());
        assert_eq!(r.missing, vec![ids[0]]);
        assert_eq!(r.new, vec![0]);
    }

    #[test]
    fn red_onion_does_not_match_red_bell_pepper() {
        let (r, ids) = run(&[("red onion", F)], &["red bell pepper"]);
        assert!(r.matched.is_empty());
        assert_eq!(r.missing, vec![ids[0]]);
        assert_eq!(r.new, vec![0]);
    }

    #[test]
    fn two_lots_of_butter_both_match_one_sighting() {
        let (r, ids) = run(&[("butter", F), ("butter", F)], &["butter"]);
        assert_eq!(r.matched, vec![(0, ids[0]), (0, ids[1])]);
        assert!(r.missing.is_empty() && r.new.is_empty());
    }

    #[test]
    fn a_lot_in_another_place_is_neither_matched_nor_missing() {
        let (r, _) = run(&[("butter", Location::Freezer)], &["butter"]);
        assert!(r.matched.is_empty() && r.missing.is_empty());
        assert_eq!(r.new, vec![0]);
    }

    #[test]
    fn a_gone_lot_is_ignored() {
        let (id, acquired) = lot_events("butter", F);
        let used = Event {
            at: "2026-11-04T10:00:00Z".parse().unwrap(),
            lot: id,
            kind: EventKind::Used { qty: None },
        };
        let inv = Inventory::replay(&[acquired, used]).unwrap();
        let r = reconcile(&obs(&["butter"]), inv.all(), F);
        assert!(r.matched.is_empty() && r.missing.is_empty());
        assert_eq!(r.new, vec![0]);
    }

    #[test]
    fn a_moved_lot_follows_its_new_place() {
        let (id, acquired) = lot_events("peas", Location::Freezer);
        let moved = Event {
            at: "2026-11-04T10:00:00Z".parse().unwrap(),
            lot: id,
            kind: EventKind::Moved { to: F },
        };
        let inv = Inventory::replay(&[acquired, moved]).unwrap();
        let r = reconcile(&obs(&["peas"]), inv.all(), F);
        assert_eq!(r.matched, vec![(0, id)]);
    }

    #[test]
    fn a_lot_without_a_sighting_is_missing() {
        let (r, ids) = run(&[("milk", F), ("eggs", F)], &["milk"]);
        assert_eq!(r.matched, vec![(0, ids[0])]);
        assert_eq!(r.missing, vec![ids[1]]);
    }

    #[test]
    fn a_sighting_without_a_lot_is_new() {
        let (r, _) = run(&[("milk", F)], &["milk", "leftover pasta"]);
        assert_eq!(r.new, vec![1]);
    }

    #[test]
    fn an_exact_match_is_preferred_over_a_subset_match() {
        // The "skyr" lot could match either sighting; it takes the exact one.
        let (r, ids) = run(&[("skyr", F)], &["arla skyr", "skyr"]);
        assert_eq!(r.matched, vec![(1, ids[0])]);
        assert_eq!(r.new, vec![0]);
    }

    #[test]
    fn an_exact_match_leaves_no_lot_for_the_subset_sighting() {
        let (r, ids) = run(&[("skyr", F), ("arla skyr", F)], &["skyr"]);
        // Both lots match the sighting: one exactly, one by subset.
        assert_eq!(r.matched, vec![(0, ids[0]), (0, ids[1])]);
    }

    #[test]
    fn empty_name_sightings_are_ignored() {
        let (r, ids) = run(&[("milk", F)], &["", "clear jar", "milk"]);
        assert_eq!(r.matched, vec![(2, ids[0])]);
        assert!(r.new.is_empty() && r.missing.is_empty());
    }

    #[test]
    fn matches_are_ordered_by_sighting_then_lot() {
        let (r, ids) = run(&[("eggs", F), ("milk", F)], &["milk", "eggs"]);
        assert_eq!(r.matched, vec![(0, ids[1]), (1, ids[0])]);
    }

    #[test]
    fn names_match_like_reconcile_does() {
        assert!(names_match("chicken", "chicken breast"));
        assert!(names_match("Tomatoes", "tomato"));
        assert!(names_match("red bell pepper x2", "bell pepper"));
        assert!(!names_match("red", "red bell pepper"));
        assert!(!names_match("egg", "eggplant"));
        assert!(!names_match("", "salt"));
    }
}
