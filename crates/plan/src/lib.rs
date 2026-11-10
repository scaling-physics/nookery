//! Weekly meal plan and shopping list. Pure: no I/O, no clock.
//!
//! The plan picks dinners from a pool of checked ideas
//! (`nookery_recipes::Suggestion`) with a small integer program (`good_lp`
//! with the pure-Rust `microlp` solver, so it also builds for Android):
//!
//! * binary `x_i` per idea (cooked this week), binary `y_l` per use-soon lot
//!   ("covered"): `y_l <= sum of x_i that use lot l`, so a lot counts once
//!   however many dinners use it;
//! * maximise `sum(score_i x_i) + 4 sum(y_l) - sum(need_i x_i)`;
//! * exactly `days` dinners (fewer if the pool is short);
//! * variety: at most 2 dinners with the same main protein category (meat,
//!   fish, legumes, eggs, cheese, from the categories of the `have` lots);
//! * at least `ceil(days / 2)` dinners with a vegetable.
//!
//! "Balanced" means by food category only. There are no nutrition numbers
//! (USDA data is a later step; the model never writes numbers like that).
//!
//! The synced documents (`docs` table; see `nookery_store`) have the shapes
//! below, all JSON with camelCase keys:
//!
//! * favourite (id = UUID v7): [`Favourite`];
//! * plan (id = the Monday, `YYYY-MM-DD`): [`PlanDoc`];
//! * shopping item (id = UUID v7): [`ShoppingItem`].

#[cfg(feature = "store")]
pub mod ops;
mod shopping;

use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate};
use good_lp::{microlp, variable, variables, Expression, Solution, SolverModel};
use nookery_core::{category, FoodCategory};
use nookery_recipes::{suggest, RecipeError, Request, StockItem, Suggestion, TextModel, Urgency};
use serde::{Deserialize, Serialize};

pub use shopping::*;

// ---- document shapes ----

/// A saved recipe. Body of a `favourite` doc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Favourite {
    pub recipe: FavouriteRecipe,
    /// The idea's one-line reason, if it came from an idea.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// RFC 3339.
    pub saved_at: String,
    /// How many times it was cooked from here.
    #[serde(default)]
    pub cooked: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavouriteRecipe {
    pub title: String,
    pub serves: Option<u32>,
    pub minutes: Option<u32>,
    pub ingredients: Vec<String>,
    /// One per ingredient line.
    pub ingredient_categories: Vec<FoodCategory>,
    pub steps: Vec<String>,
}

/// An ingredient in stock, as the plan remembers it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaveRef {
    pub ingredient: String,
    /// The lot's full id.
    pub lot: String,
}

/// One dinner. Used for planned days and for spares.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// `YYYY-MM-DD`. Spares carry the date of the day they would replace
    /// only after a swap; before that it is empty.
    #[serde(default)]
    pub date: String,
    pub name: String,
    pub why: String,
    pub minutes: Option<u32>,
    pub have: Vec<HaveRef>,
    pub need: Vec<String>,
    pub need_categories: Vec<FoodCategory>,
    /// Use-soon lots this dinner uses (informs the "uses soon" badge).
    #[serde(default)]
    pub uses_soon: u32,
    #[serde(default)]
    pub cooked: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(default)]
    pub note: String,
}

/// A week's dinners. Body of a `plan` doc (id = `week`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDoc {
    /// The Monday of the week, `YYYY-MM-DD`.
    pub week: String,
    pub days: Vec<Day>,
    pub spares: Vec<Day>,
    pub request: PlanRequest,
    /// RFC 3339.
    pub created_at: String,
}

/// A shopping-list item. Body of a `shopping` doc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingItem {
    pub item: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty: Option<String>,
    pub category: FoodCategory,
    #[serde(default)]
    pub done: bool,
    /// `plan:<week>`, `recipe` or `manual`.
    pub from: String,
}

// ---- dates ----

/// The Monday on or before `d`.
pub fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Duration::days(i64::from(d.weekday().num_days_from_monday()))
}

/// Today if it is a Monday, else the next Monday.
pub fn default_start(today: NaiveDate) -> NaiveDate {
    let back = i64::from(today.weekday().num_days_from_monday());
    if back == 0 {
        today
    } else {
        today + Duration::days(7 - back)
    }
}

// ---- planning ----

#[derive(Clone, Debug)]
pub struct PlanOptions {
    /// The first dinner's date; the plan's week is its Monday.
    pub start: NaiveDate,
    pub request: PlanRequest,
    /// RFC 3339, stored as `createdAt`.
    pub created_at: String,
    /// How many unused ideas to keep for swapping.
    pub max_spares: usize,
}

impl PlanOptions {
    pub fn new(start: NaiveDate, created_at: &str) -> Self {
        PlanOptions {
            start,
            request: PlanRequest::default(),
            created_at: created_at.to_string(),
            max_spares: 6,
        }
    }
}

pub type WeekPlan = PlanDoc;

const PROTEINS: [FoodCategory; 5] = [
    FoodCategory::Meat,
    FoodCategory::Fish,
    FoodCategory::Legumes,
    FoodCategory::Eggs,
    FoodCategory::Cheese,
];

/// Categories of the lots an idea uses.
fn have_categories(s: &Suggestion, stock: &[StockItem]) -> Vec<FoodCategory> {
    s.have
        .iter()
        .map(|(ing, lot)| {
            let name = stock
                .iter()
                .find(|x| x.lot == *lot)
                .map_or(ing.as_str(), |x| x.item.as_str());
            category(name)
        })
        .collect()
}

/// Use-soon lots in `stock` used by `s`.
fn soon_lots<'a>(s: &'a Suggestion, stock: &[StockItem]) -> Vec<&'a nookery_core::LotId> {
    s.have
        .iter()
        .map(|(_, l)| l)
        .filter(|l| {
            stock
                .iter()
                .any(|x| x.lot == **l && x.urgency == Urgency::UseSoon)
        })
        .collect()
}

fn to_day(s: &Suggestion, date: String, stock: &[StockItem]) -> Day {
    Day {
        date,
        name: s.idea.name.clone(),
        why: s.idea.why.clone(),
        minutes: s.idea.minutes,
        have: s
            .have
            .iter()
            .map(|(ing, lot)| HaveRef {
                ingredient: ing.clone(),
                lot: lot.to_string(),
            })
            .collect(),
        need: s.need.clone(),
        need_categories: s.need.iter().map(|n| category(n)).collect(),
        uses_soon: soon_lots(s, stock).len() as u32,
        cooked: false,
    }
}

#[derive(Clone, Copy)]
struct Rules {
    variety: bool,
    veg: bool,
}

/// Solves for exactly `k` ideas; None if infeasible under `rules`.
fn solve(
    pool: &[Suggestion],
    stock: &[StockItem],
    k: usize,
    days: usize,
    rules: Rules,
) -> Option<Vec<usize>> {
    let n = pool.len();
    let cats: Vec<Vec<FoodCategory>> = pool.iter().map(|s| have_categories(s, stock)).collect();
    // Use-soon lots, each once.
    let mut soon: Vec<&nookery_core::LotId> = Vec::new();
    for s in pool {
        for l in soon_lots(s, stock) {
            if !soon.contains(&l) {
                soon.push(l);
            }
        }
    }
    let mut vars = variables!();
    let x: Vec<_> = (0..n).map(|_| vars.add(variable().binary())).collect();
    let y: Vec<_> = soon.iter().map(|_| vars.add(variable().binary())).collect();

    let mut obj = Expression::from(0.0);
    for (i, s) in pool.iter().enumerate() {
        // The tiny tie-break keeps earlier (better scored) ideas first.
        let w = f64::from(s.score) - s.need.len() as f64 - 0.001 * i as f64;
        obj += w * x[i];
    }
    for v in &y {
        obj += 4.0 * *v;
    }
    let mut model = vars.maximise(obj).using(microlp);
    let mut total = Expression::from(0.0);
    for v in &x {
        total += *v;
    }
    model = model.with(total.eq(k as f64));
    for (j, lot) in soon.iter().enumerate() {
        let mut users = Expression::from(0.0);
        for (i, s) in pool.iter().enumerate() {
            if s.have.iter().any(|(_, l)| l == *lot) {
                users += x[i];
            }
        }
        model = model.with(Expression::from(y[j]).leq(users));
    }
    if rules.variety {
        for p in PROTEINS {
            let mut sum = Expression::from(0.0);
            let mut any = false;
            for i in 0..n {
                if cats[i].contains(&p) {
                    sum += x[i];
                    any = true;
                }
            }
            if any {
                model = model.with(sum.leq(2.0));
            }
        }
    }
    if rules.veg {
        let mut sum = Expression::from(0.0);
        for i in 0..n {
            if cats[i].contains(&FoodCategory::Vegetables) {
                sum += x[i];
            }
        }
        model = model.with(sum.geq(days.div_ceil(2) as f64));
    }
    let sol = model.solve().ok()?;
    let chosen: Vec<usize> = (0..n).filter(|i| sol.value(x[*i]) > 0.5).collect();
    (chosen.len() == k).then_some(chosen)
}

/// Picks dinners for `days` days from `pool` (best scored first, as
/// `nookery_recipes::check` returns it), most urgent first. Unchosen ideas
/// become `spares`, best first.
pub fn plan_week(
    pool: &[Suggestion],
    stock: &[StockItem],
    days: usize,
    opts: &PlanOptions,
) -> WeekPlan {
    let days = days.clamp(1, 7);
    let k = days.min(pool.len());
    let chosen: Vec<usize> = if k == 0 {
        vec![]
    } else {
        [
            Rules {
                variety: true,
                veg: true,
            },
            Rules {
                variety: true,
                veg: false,
            },
            Rules {
                variety: false,
                veg: false,
            },
        ]
        .into_iter()
        .find_map(|r| solve(pool, stock, k, days, r))
        // Should not happen (the last set has no constraints but the count).
        .unwrap_or_else(|| (0..k).collect())
    };
    // Order: the dinner with the most urgent use-soon lot first. `stock` is
    // in use-first order, so its index is the urgency.
    let urgency = |i: usize| -> usize {
        pool[i]
            .have
            .iter()
            .filter_map(|(_, l)| {
                stock
                    .iter()
                    .position(|x| x.lot == *l && x.urgency == Urgency::UseSoon)
            })
            .min()
            .unwrap_or(usize::MAX)
    };
    let mut order = chosen.clone();
    order.sort_by_key(|i| (urgency(*i), *i));
    let day_docs = order
        .iter()
        .enumerate()
        .map(|(n, i)| {
            let date = opts.start + Duration::days(n as i64);
            to_day(&pool[*i], date.to_string(), stock)
        })
        .collect();
    let chosen_set: HashSet<usize> = chosen.into_iter().collect();
    let spares = (0..pool.len())
        .filter(|i| !chosen_set.contains(i))
        .take(opts.max_spares)
        .map(|i| to_day(&pool[i], String::new(), stock))
        .collect();
    PlanDoc {
        week: monday_of(opts.start).to_string(),
        days: day_docs,
        spares,
        request: opts.request.clone(),
        created_at: opts.created_at.clone(),
    }
}

/// Asks the model for `days + 4` ideas (`Request::count` is not capped
/// here) and, only if fewer than `days` came back usable, once more for
/// different ones. Best scored first. Returns the pool and the model's time
/// in ms.
pub fn gather_pool(
    model: &dyn TextModel,
    stock: &[StockItem],
    request: &Request,
    days: usize,
) -> Result<(Vec<Suggestion>, f64), RecipeError> {
    let mut req = request.clone();
    req.count = (days + 4) as u32;
    let first = suggest(model, stock, &req)?;
    let mut ms = first.ms;
    let mut pool = first.items;
    if pool.len() < days {
        let names: Vec<&str> = pool.iter().map(|s| s.idea.name.as_str()).collect();
        if !names.is_empty() {
            req.note = format!("{} (not: {})", req.note, names.join(", "))
                .trim()
                .to_string();
        }
        // A failing second call is not fatal: plan with what we have.
        if let Ok(more) = suggest(model, stock, &req) {
            ms += more.ms;
            for s in more.items {
                if !pool
                    .iter()
                    .any(|p| p.idea.name.eq_ignore_ascii_case(&s.idea.name))
                {
                    pool.push(s);
                }
            }
            pool.sort_by_key(|a| std::cmp::Reverse(a.score));
        }
    }
    Ok((pool, ms))
}

/// Replaces the dinner on `date` with the best spare; the dinner it
/// replaces goes to the end of the spares. A cooked day stays. Returns
/// whether anything changed.
pub fn swap_day(plan: &mut PlanDoc, date: &str) -> bool {
    let Some(i) = plan.days.iter().position(|d| d.date == date && !d.cooked) else {
        return false;
    };
    if plan.spares.is_empty() {
        return false;
    }
    let mut next = plan.spares.remove(0);
    next.date = date.to_string();
    let mut old = std::mem::replace(&mut plan.days[i], next);
    old.date = String::new();
    plan.spares.push(old);
    true
}

#[cfg(test)]
mod tests;
