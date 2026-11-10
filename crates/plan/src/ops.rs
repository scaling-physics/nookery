//! Store-backed operations over the `docs` API: favourites, plans, shopping
//! and the "I cooked it" write. Shared by the app and the CLI so both do the
//! same thing. Everything here is state that syncs to the phone, so it lives
//! in docs (and the ledger for lots), never in tables of its own.

use chrono::{DateTime, Utc};
use nookery_core::{category, names_match, Event, EventKind, LotId, Qty, Source};
use nookery_store::{Doc, Store, StoreError, DOC_FAVOURITE, DOC_PLAN, DOC_SHOPPING};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    aisle_rank, from_plan, new_to_buy, plan_needs, put_away_place, swap_day, Favourite,
    FavouriteRecipe, PlanDoc, ShoppingItem,
};

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("not found")]
    NotFound,
    /// A field the user typed can't be used (the field, e.g. `amount:2`).
    #[error("bad input: {0}")]
    BadInput(String),
    #[error(transparent)]
    Store(#[from] StoreError),
}

fn bad(field: impl Into<String>) -> OpError {
    OpError::BadInput(field.into())
}

fn new_id() -> String {
    LotId::new().to_string()
}

/// Live docs of a kind, parsed; one that can't be read is skipped.
fn read_all<T: DeserializeOwned>(store: &Store, kind: &str) -> Result<Vec<(String, T)>, OpError> {
    Ok(store
        .docs(kind)?
        .into_iter()
        .filter_map(|Doc { id, body, .. }| match serde_json::from_value(body) {
            Ok(v) => Some((id, v)),
            Err(e) => {
                eprintln!("nookery: skipping unreadable {kind} doc {id}: {e}");
                None
            }
        })
        .collect())
}

fn read_one<T: DeserializeOwned>(store: &Store, kind: &str, id: &str) -> Result<T, OpError> {
    match store.doc(kind, id)? {
        Some(d) if !d.deleted => serde_json::from_value(d.body).map_err(|_| OpError::NotFound),
        _ => Err(OpError::NotFound),
    }
}

fn write<T: Serialize>(
    store: &mut Store,
    kind: &str,
    id: &str,
    v: &T,
    now: DateTime<Utc>,
) -> Result<(), OpError> {
    let body = serde_json::to_value(v).expect("docs always serialise");
    store.put_doc(kind, id, &body, now)?;
    Ok(())
}

// ---- favourites ----

/// Saves a recipe. A recipe with the same title (any case) is already
/// saved: that one is returned, nothing changes.
pub fn save_favourite(
    store: &mut Store,
    recipe: FavouriteRecipe,
    why: Option<String>,
    now: DateTime<Utc>,
) -> Result<(String, Favourite), OpError> {
    let title = recipe.title.trim().to_string();
    if title.is_empty() {
        return Err(bad("title"));
    }
    if let Some(same) = list_favourites(store)?
        .into_iter()
        .find(|(_, f)| f.recipe.title.trim().eq_ignore_ascii_case(&title))
    {
        return Ok(same);
    }
    let fav = Favourite {
        recipe: FavouriteRecipe { title, ..recipe },
        why: why.map(|w| w.trim().to_string()).filter(|w| !w.is_empty()),
        saved_at: now.to_rfc3339(),
        cooked: 0,
    };
    let id = new_id();
    write(store, DOC_FAVOURITE, &id, &fav, now)?;
    Ok((id, fav))
}

/// Replaces a saved recipe's content (after the recipe chat changed it),
/// keeping its id, reason, saved date and cooked count.
pub fn update_favourite(
    store: &mut Store,
    id: &str,
    recipe: FavouriteRecipe,
    now: DateTime<Utc>,
) -> Result<Favourite, OpError> {
    let title = recipe.title.trim().to_string();
    if title.is_empty() {
        return Err(bad("title"));
    }
    let mut f: Favourite = read_one(store, DOC_FAVOURITE, id)?;
    f.recipe = FavouriteRecipe { title, ..recipe };
    write(store, DOC_FAVOURITE, id, &f, now)?;
    Ok(f)
}

/// Newest first.
pub fn list_favourites(store: &Store) -> Result<Vec<(String, Favourite)>, OpError> {
    let mut all: Vec<(String, Favourite)> = read_all(store, DOC_FAVOURITE)?;
    all.sort_by(|a, b| b.1.saved_at.cmp(&a.1.saved_at).then(a.0.cmp(&b.0)));
    Ok(all)
}

pub fn delete_favourite(store: &mut Store, id: &str, now: DateTime<Utc>) -> Result<(), OpError> {
    read_one::<Favourite>(store, DOC_FAVOURITE, id)?;
    store.delete_doc(DOC_FAVOURITE, id, now)?;
    Ok(())
}

/// Counts one more time cooked on the favourite with this title, if any.
pub fn bump_cooked(store: &mut Store, title: &str, now: DateTime<Utc>) -> Result<bool, OpError> {
    let title = title.trim();
    let Some((id, mut f)) = list_favourites(store)?
        .into_iter()
        .find(|(_, f)| f.recipe.title.trim().eq_ignore_ascii_case(title))
    else {
        return Ok(false);
    };
    f.cooked += 1;
    write(store, DOC_FAVOURITE, &id, &f, now)?;
    Ok(true)
}

// ---- cooked ----

#[derive(Clone, Debug)]
pub struct CookedItem {
    /// A lot id, or the end of one (the short id).
    pub lot: String,
    /// `None` = all of it. Otherwise an amount like `200g` or `1`.
    pub amount: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CookedSummary {
    /// Lots written to.
    pub lots: u32,
    /// Of those, lots now used up.
    pub used_up: u32,
}

/// One batch of `Used` events. All or nothing: every amount is checked
/// first (unit matches the lot, more than zero, no more than what is
/// left), and a bad one is `BadInput("amount:<index>")`, a lot that is
/// unknown or gone `BadInput("lot:<index>")`. When `title` names a
/// favourite, its cooked count goes up.
pub fn cook(
    store: &mut Store,
    items: &[CookedItem],
    title: Option<&str>,
    now: DateTime<Utc>,
) -> Result<CookedSummary, OpError> {
    let mut events = Vec::new();
    let mut seen: Vec<LotId> = Vec::new();
    let mut summary = CookedSummary::default();
    for (i, it) in items.iter().enumerate() {
        let lot_id = store
            .find_lot(&it.lot)
            .map_err(|_| bad(format!("lot:{i}")))?;
        let lot = store.inventory().get(lot_id).expect("found");
        if !lot.in_stock() || seen.contains(&lot_id) {
            return Err(bad(format!("lot:{i}")));
        }
        seen.push(lot_id);
        let qty = match it.amount.as_deref() {
            None => None,
            Some(a) => {
                let q: Qty = a.parse().map_err(|_| bad(format!("amount:{i}")))?;
                if q.unit != lot.left.unit || q.milli > lot.left.milli {
                    return Err(bad(format!("amount:{i}")));
                }
                Some(q)
            }
        };
        summary.lots += 1;
        if qty.is_none_or(|q| q.milli == lot.left.milli) {
            summary.used_up += 1;
        }
        events.push(Event {
            at: now,
            lot: lot_id,
            kind: EventKind::Used { qty },
        });
    }
    if !events.is_empty() {
        store.append(&events)?;
    }
    if let Some(t) = title {
        bump_cooked(store, t, now)?;
    }
    Ok(summary)
}

// ---- plans ----

pub fn get_plan(store: &Store, week: &str) -> Result<PlanDoc, OpError> {
    read_one(store, DOC_PLAN, week)
}

pub fn put_plan(store: &mut Store, plan: &PlanDoc, now: DateTime<Utc>) -> Result<(), OpError> {
    write(store, DOC_PLAN, &plan.week, plan, now)
}

/// Newest week first.
pub fn list_plans(store: &Store) -> Result<Vec<PlanDoc>, OpError> {
    let mut all: Vec<PlanDoc> = read_all::<PlanDoc>(store, DOC_PLAN)?
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    all.sort_by(|a, b| b.week.cmp(&a.week));
    Ok(all)
}

pub fn delete_plan(store: &mut Store, week: &str, now: DateTime<Utc>) -> Result<(), OpError> {
    read_one::<PlanDoc>(store, DOC_PLAN, week)?;
    store.delete_doc(DOC_PLAN, week, now)?;
    Ok(())
}

/// Swaps in the next spare. `NotFound` if no such day, a cooked day, or no
/// spare is left.
pub fn swap_plan_day(
    store: &mut Store,
    week: &str,
    date: &str,
    now: DateTime<Utc>,
) -> Result<PlanDoc, OpError> {
    let mut plan = get_plan(store, week)?;
    if !swap_day(&mut plan, date) {
        return Err(OpError::NotFound);
    }
    put_plan(store, &plan, now)?;
    Ok(plan)
}

pub fn set_day_cooked(
    store: &mut Store,
    week: &str,
    date: &str,
    cooked: bool,
    now: DateTime<Utc>,
) -> Result<PlanDoc, OpError> {
    let mut plan = get_plan(store, week)?;
    let day = plan
        .days
        .iter_mut()
        .find(|d| d.date == date)
        .ok_or(OpError::NotFound)?;
    day.cooked = cooked;
    put_plan(store, &plan, now)?;
    Ok(plan)
}

// ---- shopping ----

/// The list in aisle order, then by name; ticked items last in their aisle.
pub fn list_shopping(store: &Store) -> Result<Vec<(String, ShoppingItem)>, OpError> {
    let mut all: Vec<(String, ShoppingItem)> = read_all(store, DOC_SHOPPING)?;
    all.sort_by(|a, b| {
        (
            aisle_rank(a.1.category),
            a.1.done,
            a.1.item.to_lowercase(),
            &a.0,
        )
            .cmp(&(
                aisle_rank(b.1.category),
                b.1.done,
                b.1.item.to_lowercase(),
                &b.0,
            ))
    });
    Ok(all)
}

/// Adds an item (`from`: `manual`, `recipe` or `plan:<week>`). The same
/// thing already waiting (not ticked) is returned instead of a twin.
pub fn add_shopping(
    store: &mut Store,
    item: &str,
    qty: Option<&str>,
    from: &str,
    now: DateTime<Utc>,
) -> Result<(String, ShoppingItem), OpError> {
    let item = item.trim();
    if item.is_empty() {
        return Err(bad("item"));
    }
    if let Some(same) = list_shopping(store)?
        .into_iter()
        .find(|(_, s)| !s.done && s.item.trim().eq_ignore_ascii_case(item))
    {
        return Ok(same);
    }
    let it = ShoppingItem {
        item: item.to_string(),
        qty: qty
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(String::from),
        category: category(item),
        done: false,
        from: from.to_string(),
    };
    let id = new_id();
    write(store, DOC_SHOPPING, &id, &it, now)?;
    Ok((id, it))
}

#[derive(Debug, Default)]
pub struct AddedFromPlan {
    pub added: Vec<(String, ShoppingItem)>,
    /// Needs left out because they were on the list or in stock.
    pub skipped: usize,
}

/// The plan's still-needed ingredients, minus what is waiting on the list
/// and what is in stock.
pub fn add_plan_to_shopping(
    store: &mut Store,
    week: &str,
    now: DateTime<Utc>,
) -> Result<AddedFromPlan, OpError> {
    let plan = get_plan(store, week)?;
    let needs = plan_needs(&plan);
    let total = needs.len();
    let undone: Vec<String> = list_shopping(store)?
        .into_iter()
        .filter(|(_, s)| !s.done)
        .map(|(_, s)| s.item)
        .collect();
    let stock: Vec<String> = store
        .inventory()
        .in_stock()
        .map(|l| l.item.clone())
        .collect();
    let fresh = new_to_buy(needs, &undone, &stock);
    let mut out = AddedFromPlan {
        skipped: total - fresh.len(),
        ..Default::default()
    };
    for (name, cat) in fresh {
        let it = ShoppingItem {
            item: name,
            qty: None,
            category: cat,
            done: false,
            from: from_plan(week),
        };
        let id = new_id();
        write(store, DOC_SHOPPING, &id, &it, now)?;
        out.added.push((id, it));
    }
    Ok(out)
}

pub fn set_shopping_done(
    store: &mut Store,
    id: &str,
    done: bool,
    now: DateTime<Utc>,
) -> Result<ShoppingItem, OpError> {
    let mut it: ShoppingItem = read_one(store, DOC_SHOPPING, id)?;
    it.done = done;
    write(store, DOC_SHOPPING, id, &it, now)?;
    Ok(it)
}

pub fn delete_shopping(store: &mut Store, id: &str, now: DateTime<Utc>) -> Result<(), OpError> {
    read_one::<ShoppingItem>(store, DOC_SHOPPING, id)?;
    store.delete_doc(DOC_SHOPPING, id, now)?;
    Ok(())
}

/// Removes every ticked item from the list; how many.
pub fn clear_done_shopping(store: &mut Store, now: DateTime<Utc>) -> Result<usize, OpError> {
    let done: Vec<String> = list_shopping(store)?
        .into_iter()
        .filter(|(_, s)| s.done)
        .map(|(id, _)| id)
        .collect();
    for id in &done {
        store.delete_doc(DOC_SHOPPING, id, now)?;
    }
    Ok(done.len())
}

#[derive(Debug, Default)]
pub struct PutAway {
    pub lots: Vec<LotId>,
    /// The items that left the list (for Undo).
    pub items: Vec<(String, ShoppingItem)>,
}

/// Ticked items become lots (`Source::Manual`; fridge for fresh food, else
/// the pantry; the amount if it can be read, else 1 piece) and leave the
/// list. The lots are written first, in one batch.
pub fn put_away_shopping(store: &mut Store, now: DateTime<Utc>) -> Result<PutAway, OpError> {
    let done: Vec<(String, ShoppingItem)> = list_shopping(store)?
        .into_iter()
        .filter(|(_, s)| s.done)
        .collect();
    let mut events = Vec::new();
    let mut lots = Vec::new();
    for (_, s) in &done {
        let lot = LotId::new();
        lots.push(lot);
        events.push(Event {
            at: now,
            lot,
            kind: EventKind::Acquired {
                item: s.item.clone(),
                qty: s
                    .qty
                    .as_deref()
                    .and_then(|q| q.parse().ok())
                    .unwrap_or(Qty::pieces(1)),
                location: put_away_place(s.category),
                source: Source::Manual,
                best_before: None,
            },
        });
    }
    if !events.is_empty() {
        store.append(&events)?;
    }
    for (id, _) in &done {
        store.delete_doc(DOC_SHOPPING, id, now)?;
    }
    Ok(PutAway { lots, items: done })
}

/// For tests and callers that look an item up by name.
pub fn on_list(store: &Store, name: &str) -> Result<bool, OpError> {
    Ok(list_shopping(store)?
        .iter()
        .any(|(_, s)| !s.done && names_match(&s.item, name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Day, PlanRequest};
    use nookery_core::{FoodCategory, Location};

    fn at() -> DateTime<Utc> {
        "2026-11-09T10:00:00Z".parse().unwrap()
    }

    fn acquire(s: &mut Store, item: &str, qty: Qty) -> LotId {
        let lot = LotId::new();
        s.append(&[Event {
            at: at(),
            lot,
            kind: EventKind::Acquired {
                item: item.into(),
                qty,
                location: Location::Fridge,
                source: Source::Manual,
                best_before: None,
            },
        }])
        .unwrap();
        lot
    }

    fn recipe(title: &str) -> FavouriteRecipe {
        FavouriteRecipe {
            title: title.into(),
            serves: Some(2),
            minutes: Some(20),
            ingredients: vec!["2 eggs".into()],
            ingredient_categories: vec![FoodCategory::Eggs],
            steps: vec!["Cook.".into()],
        }
    }

    fn day(date: &str, need: &[&str], cooked: bool) -> Day {
        Day {
            date: date.into(),
            name: "x".into(),
            why: String::new(),
            minutes: None,
            have: vec![],
            need: need.iter().map(|s| s.to_string()).collect(),
            need_categories: need.iter().map(|n| category(n)).collect(),
            uses_soon: 0,
            cooked,
        }
    }

    fn plan(days: Vec<Day>, spares: Vec<Day>) -> PlanDoc {
        PlanDoc {
            week: "2026-11-09".into(),
            days,
            spares,
            request: PlanRequest::default(),
            created_at: at().to_rfc3339(),
        }
    }

    #[test]
    fn a_changed_favourite_keeps_its_id_and_count() {
        let mut s = Store::open_in_memory().unwrap();
        let (id, _) = save_favourite(&mut s, recipe("Skillet"), Some("why".into()), at()).unwrap();
        bump_cooked(&mut s, "Skillet", at()).unwrap();
        let mut r = recipe("Skillet with rice");
        r.serves = Some(3);
        let f = update_favourite(&mut s, &id, r, at()).unwrap();
        assert_eq!(f.recipe.title, "Skillet with rice");
        assert_eq!(f.cooked, 1);
        assert_eq!(f.why.as_deref(), Some("why"));
        let all = list_favourites(&s).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, id);
        assert_eq!(all[0].1.recipe.serves, Some(3));
        assert!(update_favourite(&mut s, "nope", recipe("x"), at()).is_err());
        assert!(update_favourite(&mut s, &id, recipe(" "), at()).is_err());
    }

    #[test]
    fn favourites_save_once_count_and_delete() {
        let mut s = Store::open_in_memory().unwrap();
        let (id, f) =
            save_favourite(&mut s, recipe(" Omelette "), Some("why".into()), at()).unwrap();
        assert_eq!(f.recipe.title, "Omelette");
        let (again, _) = save_favourite(&mut s, recipe("omelette"), None, at()).unwrap();
        assert_eq!(again, id);
        assert_eq!(list_favourites(&s).unwrap().len(), 1);
        assert!(bump_cooked(&mut s, "OMELETTE", at()).unwrap());
        assert!(!bump_cooked(&mut s, "Soup", at()).unwrap());
        assert_eq!(list_favourites(&s).unwrap()[0].1.cooked, 1);
        assert!(matches!(
            save_favourite(&mut s, recipe("  "), None, at()),
            Err(OpError::BadInput(f)) if f == "title"
        ));
        delete_favourite(&mut s, &id, at()).unwrap();
        assert!(list_favourites(&s).unwrap().is_empty());
        assert!(matches!(
            delete_favourite(&mut s, &id, at()),
            Err(OpError::NotFound)
        ));
    }

    #[test]
    fn cooked_uses_some_all_or_rejects() {
        let mut s = Store::open_in_memory().unwrap();
        let flour = acquire(&mut s, "flour", Qty::grams(500));
        let eggs = acquire(&mut s, "eggs", Qty::pieces(4));
        let item = |lot: LotId, a: Option<&str>| CookedItem {
            lot: lot.short(),
            amount: a.map(String::from),
        };
        let field = |r: Result<CookedSummary, OpError>| match r.unwrap_err() {
            OpError::BadInput(f) => f,
            e => panic!("{e:?}"),
        };
        assert_eq!(
            field(cook(&mut s, &[item(flour, Some("2"))], None, at())),
            "amount:0"
        );
        assert_eq!(
            field(cook(&mut s, &[item(flour, Some("600g"))], None, at())),
            "amount:0"
        );
        assert_eq!(
            field(cook(&mut s, &[item(flour, Some("lots"))], None, at())),
            "amount:0"
        );
        assert_eq!(
            field(cook(&mut s, &[item(flour, Some("0"))], None, at())),
            "amount:0"
        );
        assert_eq!(
            field(cook(
                &mut s,
                &[item(eggs, Some("1")), item(flour, Some("1l"))],
                None,
                at()
            )),
            "amount:1"
        );
        assert_eq!(
            field(cook(
                &mut s,
                &[CookedItem {
                    lot: "nope".into(),
                    amount: None
                }],
                None,
                at()
            )),
            "lot:0"
        );
        // Nothing was written by the failures.
        assert_eq!(s.inventory().get(flour).unwrap().left, Qty::grams(500));

        let r = cook(
            &mut s,
            &[item(flour, Some("200g")), item(eggs, None)],
            None,
            at(),
        )
        .unwrap();
        assert_eq!(
            r,
            CookedSummary {
                lots: 2,
                used_up: 1
            }
        );
        assert_eq!(s.inventory().get(flour).unwrap().left, Qty::grams(300));
        assert!(!s.inventory().get(eggs).unwrap().in_stock());
        // A lot that is gone can't be used again.
        assert_eq!(
            field(cook(&mut s, &[item(eggs, None)], None, at())),
            "lot:0"
        );
        // The rest by amount equal to what is left counts as used up.
        let r = cook(&mut s, &[item(flour, Some("0.3kg"))], None, at()).unwrap();
        assert_eq!(r.used_up, 1);
    }

    #[test]
    fn cooked_bumps_the_favourite() {
        let mut s = Store::open_in_memory().unwrap();
        let flour = acquire(&mut s, "flour", Qty::grams(500));
        save_favourite(&mut s, recipe("Pancakes"), None, at()).unwrap();
        let it = CookedItem {
            lot: flour.short(),
            amount: Some("100g".into()),
        };
        cook(&mut s, &[it], Some("pancakes"), at()).unwrap();
        assert_eq!(list_favourites(&s).unwrap()[0].1.cooked, 1);
    }

    #[test]
    fn plans_store_swap_and_cook() {
        let mut s = Store::open_in_memory().unwrap();
        let p = plan(
            vec![day("2026-11-09", &[], false), day("2026-11-10", &[], false)],
            vec![day("", &[], false)],
        );
        put_plan(&mut s, &p, at()).unwrap();
        assert_eq!(list_plans(&s).unwrap().len(), 1);
        let swapped = swap_plan_day(&mut s, "2026-11-09", "2026-11-10", at()).unwrap();
        assert_eq!(swapped.days[1].date, "2026-11-10");
        assert!(matches!(
            swap_plan_day(&mut s, "2026-11-09", "2026-11-12", at()),
            Err(OpError::NotFound)
        ));
        let done = set_day_cooked(&mut s, "2026-11-09", "2026-11-09", true, at()).unwrap();
        assert!(done.days[0].cooked);
        assert!(get_plan(&s, "2026-11-09").unwrap().days[0].cooked);
        delete_plan(&mut s, "2026-11-09", at()).unwrap();
        assert!(matches!(get_plan(&s, "2026-11-09"), Err(OpError::NotFound)));
    }

    #[test]
    fn plan_to_shopping_dedups_against_list_and_stock() {
        let mut s = Store::open_in_memory().unwrap();
        acquire(&mut s, "red onions", Qty::pieces(3));
        add_shopping(&mut s, "Lemons", None, "manual", at()).unwrap();
        let p = plan(
            vec![
                day("2026-11-09", &["onion", "lemon", "tahini"], false),
                day("2026-11-10", &["tahini", "kale"], false),
                day("2026-11-11", &["leek"], true),
            ],
            vec![],
        );
        put_plan(&mut s, &p, at()).unwrap();
        let r = add_plan_to_shopping(&mut s, "2026-11-09", at()).unwrap();
        let names: Vec<&str> = r.added.iter().map(|(_, i)| i.item.as_str()).collect();
        assert_eq!(names, ["tahini", "kale"]);
        assert_eq!(r.skipped, 2);
        assert_eq!(r.added[0].1.from, "plan:2026-11-09");
        // A second run adds nothing.
        assert!(add_plan_to_shopping(&mut s, "2026-11-09", at())
            .unwrap()
            .added
            .is_empty());
        assert!(on_list(&s, "kale").unwrap());
    }

    #[test]
    fn shopping_order_tick_clear_and_put_away() {
        let mut s = Store::open_in_memory().unwrap();
        let (milk, _) = add_shopping(&mut s, "milk", Some("1l"), "manual", at()).unwrap();
        let (rice, _) = add_shopping(&mut s, "basmati rice", Some("1kg"), "manual", at()).unwrap();
        let (kale, _) = add_shopping(&mut s, "kale", None, "manual", at()).unwrap();
        let (_junk, _) =
            add_shopping(&mut s, "paper towels", Some("two packs"), "manual", at()).unwrap();
        // Twin of an undone item is not added.
        let (again, _) = add_shopping(&mut s, "Milk", None, "manual", at()).unwrap();
        assert_eq!(again, milk);
        assert!(matches!(
            add_shopping(&mut s, " ", None, "manual", at()),
            Err(OpError::BadInput(f)) if f == "item"
        ));
        let order: Vec<String> = list_shopping(&s)
            .unwrap()
            .into_iter()
            .map(|(_, i)| i.item)
            .collect();
        assert_eq!(order, ["kale", "milk", "basmati rice", "paper towels"]);

        for id in [&milk, &rice, &kale] {
            set_shopping_done(&mut s, id, true, at()).unwrap();
        }
        let r = put_away_shopping(&mut s, at()).unwrap();
        assert_eq!(r.lots.len(), 3);
        assert_eq!(r.items.len(), 3);
        let inv = s.inventory();
        let get = |n: &str| inv.in_stock().find(|l| l.item == n).unwrap();
        assert_eq!(get("milk").location, Location::Fridge);
        assert_eq!(get("milk").left, Qty::millilitres(1000));
        assert_eq!(get("kale").location, Location::Fridge);
        assert_eq!(get("kale").left, Qty::pieces(1));
        assert_eq!(get("basmati rice").location, Location::Pantry);
        assert_eq!(list_shopping(&s).unwrap().len(), 1);

        // Unreadable amount becomes one piece; clear_done removes ticked.
        let (towels, _) = list_shopping(&s).unwrap().remove(0);
        set_shopping_done(&mut s, &towels, true, at()).unwrap();
        assert_eq!(clear_done_shopping(&mut s, at()).unwrap(), 1);
        assert!(list_shopping(&s).unwrap().is_empty());
        assert_eq!(put_away_shopping(&mut s, at()).unwrap().lots.len(), 0);
        assert!(matches!(
            set_shopping_done(&mut s, &towels, true, at()),
            Err(OpError::NotFound)
        ));
    }
}
