//! Plan, shopping and favourites commands, and the "I cooked it" write.
//!
//! The rules live in `nookery-plan` (`plan_week`, `ops`); this file converts
//! to UI types. All of this state is docs in the store (they sync to the
//! phone), never tables. `make_plan` follows the recipes AI setting like
//! `suggest_recipes`: the store is locked only to read the stock and to
//! write the plan, the model runs on a blocking thread in between.
//!
//! Errors: `BadInput("days" | "start" | "stock" | "item" | "title" |
//! "amount:<i>" | "lot:<i>")`; a missing plan, day, item or favourite is
//! `NotFound`. Ids are strings (docs) or `u32` (templates).

use chrono::{Local, NaiveDate, Utc};
use nookery_core::{category, status, LotId};
use nookery_plan::ops::{self, CookedItem, OpError};
use nookery_plan::{
    default_start, from_plan, gather_pool, plan_week, Day, Favourite, FavouriteRecipe, PlanDoc,
    PlanOptions, PlanRequest, ShoppingItem,
};
use nookery_recipes::{stock_from, RecipeError, Request, StockItem, TextModel};
use nookery_store::{Store, SETTING_RECIPES_AI};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use super::phone::Writer;
use super::recipes::HaveView;
use super::{bad, view, AppState, CommandError, FoodCategoryView, LotView};

/// Used when no template is chosen.
const DEFAULT_TEMPLATE: &str = "Dinners for the week, for two, mostly from what I have. Varied.";

impl From<OpError> for CommandError {
    fn from(e: OpError) -> Self {
        match e {
            OpError::NotFound => Self::NotFound,
            OpError::BadInput(f) => Self::BadInput(f),
            OpError::Store(s) => s.into(),
        }
    }
}

// ---- favourites ----

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FavouriteRecipeView {
    pub title: String,
    pub serves: Option<u32>,
    pub minutes: Option<u32>,
    pub ingredients: Vec<String>,
    pub ingredient_categories: Vec<FoodCategoryView>,
    pub steps: Vec<String>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FavouriteView {
    pub id: String,
    pub recipe: FavouriteRecipeView,
    pub why: Option<String>,
    pub saved_at: String,
    /// How many times it was cooked.
    pub cooked: u32,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveFavouriteInput {
    pub title: String,
    pub serves: Option<u32>,
    pub minutes: Option<u32>,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    pub why: Option<String>,
}

fn favourite_view(id: String, f: Favourite) -> FavouriteView {
    FavouriteView {
        id,
        recipe: FavouriteRecipeView {
            title: f.recipe.title,
            serves: f.recipe.serves,
            minutes: f.recipe.minutes,
            ingredients: f.recipe.ingredients,
            ingredient_categories: f
                .recipe
                .ingredient_categories
                .into_iter()
                .map(Into::into)
                .collect(),
            steps: f.recipe.steps,
        },
        why: f.why,
        saved_at: f.saved_at,
        cooked: f.cooked,
    }
}

pub fn save_favourite_in(
    store: &mut Store,
    input: SaveFavouriteInput,
) -> Result<FavouriteView, CommandError> {
    let recipe = FavouriteRecipe {
        ingredient_categories: input.ingredients.iter().map(|i| category(i)).collect(),
        title: input.title,
        serves: input.serves,
        minutes: input.minutes,
        ingredients: input.ingredients,
        steps: input.steps,
    };
    let (id, f) = ops::save_favourite(store, recipe, input.why, Utc::now())?;
    Ok(favourite_view(id, f))
}

pub fn list_favourites_in(store: &Store) -> Result<Vec<FavouriteView>, CommandError> {
    Ok(ops::list_favourites(store)?
        .into_iter()
        .map(|(id, f)| favourite_view(id, f))
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn save_favourite(
    state: State<'_, AppState>,
    input: SaveFavouriteInput,
) -> Result<FavouriteView, CommandError> {
    let mut store = state.store()?;
    save_favourite_in(&mut store, input)
}

#[tauri::command]
#[specta::specta]
pub fn list_favourites(state: State<'_, AppState>) -> Result<Vec<FavouriteView>, CommandError> {
    let store = state.store()?;
    list_favourites_in(&store)
}

/// Saves the recipe chat's changes into an existing favourite (same id).
pub fn update_favourite_in(
    store: &mut Store,
    id: &str,
    input: SaveFavouriteInput,
) -> Result<FavouriteView, CommandError> {
    let recipe = FavouriteRecipe {
        ingredient_categories: input.ingredients.iter().map(|i| category(i)).collect(),
        title: input.title,
        serves: input.serves,
        minutes: input.minutes,
        ingredients: input.ingredients,
        steps: input.steps,
    };
    let f = ops::update_favourite(store, id, recipe, Utc::now())?;
    Ok(favourite_view(id.to_string(), f))
}

#[tauri::command]
#[specta::specta]
pub fn update_favourite(
    state: State<'_, AppState>,
    id: String,
    input: SaveFavouriteInput,
) -> Result<FavouriteView, CommandError> {
    let mut store = state.store()?;
    update_favourite_in(&mut store, &id, input)
}

#[tauri::command]
#[specta::specta]
pub fn delete_favourite(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    let mut store = state.store()?;
    Ok(ops::delete_favourite(&mut store, &id, Utc::now())?)
}

// ---- cooked ----

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CookedItemInput {
    /// A lot id (the full one or its short form).
    pub lot: String,
    /// `None`: all of it. Otherwise an amount such as `200g` or `1`.
    pub amount: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CookedInput {
    pub items: Vec<CookedItemInput>,
    /// The recipe's title: a favourite with that title counts one more cook.
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CookedView {
    /// Lots written to.
    pub lots: u32,
    /// Of those, lots now used up.
    pub used_up: u32,
}

pub fn cooked_in(store: &mut Store, input: CookedInput) -> Result<CookedView, CommandError> {
    let items: Vec<CookedItem> = input
        .items
        .into_iter()
        .map(|i| CookedItem {
            lot: i.lot,
            amount: i.amount,
        })
        .collect();
    let s = ops::cook(store, &items, input.title.as_deref(), Utc::now())?;
    Ok(CookedView {
        lots: s.lots,
        used_up: s.used_up,
    })
}

/// One batch of `Used` events: all of a lot, some of it, or (left out) none.
#[tauri::command]
#[specta::specta]
pub fn cooked(state: State<'_, AppState>, input: CookedInput) -> Result<CookedView, CommandError> {
    let mut store = state.store()?;
    cooked_in(&mut store, input)
}

// ---- plans ----

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DayView {
    /// `YYYY-MM-DD`; empty for a spare.
    pub date: String,
    pub name: String,
    pub why: String,
    pub minutes: Option<u32>,
    pub have: Vec<HaveView>,
    pub need: Vec<String>,
    pub need_categories: Vec<FoodCategoryView>,
    pub uses_soon: u32,
    pub cooked: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    /// The Monday, `YYYY-MM-DD`.
    pub week: String,
    pub days: Vec<DayView>,
    pub spares: Vec<DayView>,
    pub template: Option<String>,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanResultView {
    pub plan: PlanView,
    pub model: String,
    pub used_cloud: bool,
    pub ms: f64,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummaryView {
    pub week: String,
    pub days: u32,
    pub cooked: u32,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MakePlanInput {
    pub template_id: Option<u32>,
    pub note: String,
    /// 1 to 7.
    pub days: u32,
    /// `YYYY-MM-DD`; default the next (or this) Monday.
    pub start: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DayRef {
    pub week: String,
    pub date: String,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetDayCookedInput {
    pub week: String,
    pub date: String,
    pub cooked: bool,
}

fn day_view(store: &Store, d: Day, today: NaiveDate) -> DayView {
    let inv = store.inventory();
    let have = d
        .have
        .iter()
        .filter_map(|h| {
            let id: LotId = store.find_lot(&h.lot).ok()?;
            let l = inv.get(id).filter(|l| l.in_stock())?;
            Some(HaveView {
                ingredient: h.ingredient.clone(),
                lot: view(l, status(l, today)),
            })
        })
        .collect();
    DayView {
        date: d.date,
        name: d.name,
        why: d.why,
        minutes: d.minutes,
        have,
        need: d.need,
        need_categories: d.need_categories.into_iter().map(Into::into).collect(),
        uses_soon: d.uses_soon,
        cooked: d.cooked,
    }
}

pub fn plan_view(store: &Store, p: PlanDoc, today: NaiveDate) -> PlanView {
    PlanView {
        week: p.week,
        days: p
            .days
            .into_iter()
            .map(|d| day_view(store, d, today))
            .collect(),
        spares: p
            .spares
            .into_iter()
            .map(|d| day_view(store, d, today))
            .collect(),
        template: p.request.template,
        note: p.request.note,
        created_at: p.created_at,
    }
}

/// What `make_plan` needs from the store.
pub struct PlanPrep {
    stock: Vec<StockItem>,
    request: Request,
    template_name: Option<String>,
    note: String,
    days: usize,
    start: NaiveDate,
}

pub fn prepare_plan_in(
    store: &Store,
    input: &MakePlanInput,
    today: NaiveDate,
) -> Result<PlanPrep, CommandError> {
    if !(1..=7).contains(&input.days) {
        return Err(bad("days"));
    }
    let start = match input.start.as_deref().map(str::trim) {
        None | Some("") => default_start(today),
        Some(s) => s.parse().map_err(|_| bad("start"))?,
    };
    let (template, template_name) = match input.template_id {
        None => (DEFAULT_TEMPLATE.to_string(), None),
        Some(id) => {
            let t = store
                .templates()?
                .into_iter()
                .find(|t| t.id == i64::from(id))
                .ok_or(CommandError::NotFound)?;
            (t.body, Some(t.name))
        }
    };
    let note = input.note.trim().to_string();
    Ok(PlanPrep {
        stock: stock_from(store.inventory().in_stock(), today),
        request: Request {
            template,
            note: note.clone(),
            count: input.days + 4,
        },
        template_name,
        note,
        days: input.days as usize,
        start,
    })
}

/// The model part: no store. Returns the plan and the model's time.
pub fn build_plan(model: &dyn TextModel, prep: &PlanPrep) -> Result<(PlanDoc, f64), RecipeError> {
    let (pool, ms) = gather_pool(model, &prep.stock, &prep.request, prep.days)?;
    let mut opts = PlanOptions::new(prep.start, &Utc::now().to_rfc3339());
    opts.request = PlanRequest {
        template: prep.template_name.clone(),
        note: prep.note.clone(),
    };
    Ok((plan_week(&pool, &prep.stock, prep.days, &opts), ms))
}

/// Stores the plan (replacing that week's) and shows it.
pub fn finish_plan_in(
    store: &mut Store,
    plan: PlanDoc,
    today: NaiveDate,
) -> Result<PlanView, CommandError> {
    ops::put_plan(store, &plan, Utc::now())?;
    Ok(plan_view(store, plan, today))
}

#[cfg(test)]
/// All the steps in one go, for tests.
pub fn make_plan_in(
    model: &dyn TextModel,
    store: &mut Store,
    input: &MakePlanInput,
    today: NaiveDate,
) -> Result<PlanView, CommandError> {
    let prep = prepare_plan_in(store, input, today)?;
    let (plan, _) = build_plan(model, &prep)?;
    finish_plan_in(store, plan, today)
}

#[tauri::command]
#[specta::specta]
pub async fn make_plan(
    state: State<'_, AppState>,
    input: MakePlanInput,
) -> Result<PlanResultView, CommandError> {
    let today = Local::now().date_naive();
    let (prep, mode, cloud_model) = {
        let store = state.store()?;
        (
            prepare_plan_in(&store, &input, today)?,
            store.ai_mode(SETTING_RECIPES_AI)?,
            store.cloud_model()?,
        )
    };
    let model = Writer::for_state(&state, mode, &cloud_model)?;
    let (plan, ms, model, used_cloud) = tauri::async_runtime::spawn_blocking(move || {
        let (plan, ms) = build_plan(&model, &prep)?;
        Ok::<_, RecipeError>((plan, ms, model.answered_by(), model.used_cloud()))
    })
    .await
    .map_err(|_| CommandError::Internal)??;
    let mut store = state.store()?;
    let plan = finish_plan_in(&mut store, plan, today)?;
    Ok(PlanResultView {
        plan,
        model,
        used_cloud,
        ms,
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_plan(state: State<'_, AppState>, week: String) -> Result<PlanView, CommandError> {
    let store = state.store()?;
    let p = ops::get_plan(&store, &week)?;
    Ok(plan_view(&store, p, Local::now().date_naive()))
}

/// Newest week first.
#[tauri::command]
#[specta::specta]
pub fn list_plans(state: State<'_, AppState>) -> Result<Vec<PlanSummaryView>, CommandError> {
    let store = state.store()?;
    Ok(ops::list_plans(&store)?
        .into_iter()
        .map(|p| PlanSummaryView {
            week: p.week,
            days: p.days.len() as u32,
            cooked: p.days.iter().filter(|d| d.cooked).count() as u32,
            created_at: p.created_at,
        })
        .collect())
}

/// Replaces a day with the next best spare (`NotFound` when there is none,
/// or the day was already cooked).
#[tauri::command]
#[specta::specta]
pub fn swap_day(state: State<'_, AppState>, input: DayRef) -> Result<PlanView, CommandError> {
    let mut store = state.store()?;
    let p = ops::swap_plan_day(&mut store, &input.week, &input.date, Utc::now())?;
    Ok(plan_view(&store, p, Local::now().date_naive()))
}

#[tauri::command]
#[specta::specta]
pub fn set_day_cooked(
    state: State<'_, AppState>,
    input: SetDayCookedInput,
) -> Result<PlanView, CommandError> {
    let mut store = state.store()?;
    let p = ops::set_day_cooked(
        &mut store,
        &input.week,
        &input.date,
        input.cooked,
        Utc::now(),
    )?;
    Ok(plan_view(&store, p, Local::now().date_naive()))
}

#[tauri::command]
#[specta::specta]
pub fn delete_plan(state: State<'_, AppState>, week: String) -> Result<(), CommandError> {
    let mut store = state.store()?;
    Ok(ops::delete_plan(&mut store, &week, Utc::now())?)
}

// ---- shopping ----

#[derive(Debug, Serialize, Type, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingItemView {
    pub id: String,
    pub item: String,
    pub qty: Option<String>,
    pub category: FoodCategoryView,
    pub done: bool,
    /// `plan:<week>`, `recipe` or `manual`.
    pub from: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingGroupView {
    pub category: FoodCategoryView,
    pub items: Vec<ShoppingItemView>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingView {
    /// In aisle order: vegetables, fruit, herbs, bread, dairy, ...
    pub groups: Vec<ShoppingGroupView>,
    pub open: u32,
    pub done: u32,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddShoppingInput {
    pub item: String,
    pub qty: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetShoppingDoneInput {
    pub id: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddedToShoppingView {
    pub added: Vec<ShoppingItemView>,
    /// Needs not added: already on the list, or in stock.
    pub skipped: u32,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PutAwayView {
    /// The new lots (to show where they went).
    pub lots: Vec<LotView>,
    /// What left the list (to put back on Undo).
    pub items: Vec<ShoppingItemView>,
}

fn item_view(id: String, s: ShoppingItem) -> ShoppingItemView {
    ShoppingItemView {
        id,
        item: s.item,
        qty: s.qty,
        category: s.category.into(),
        done: s.done,
        from: s.from,
    }
}

pub fn list_shopping_in(store: &Store) -> Result<ShoppingView, CommandError> {
    let all = ops::list_shopping(store)?;
    let done = all.iter().filter(|(_, s)| s.done).count() as u32;
    let open = all.len() as u32 - done;
    let mut groups: Vec<ShoppingGroupView> = Vec::new();
    // Already in aisle order, so equal categories are next to each other.
    for (id, s) in all {
        let cat: FoodCategoryView = s.category.into();
        match groups.last_mut() {
            Some(g) if g.category == cat => g.items.push(item_view(id, s)),
            _ => groups.push(ShoppingGroupView {
                category: cat,
                items: vec![item_view(id, s)],
            }),
        }
    }
    Ok(ShoppingView { groups, open, done })
}

pub fn put_away_in(store: &mut Store, today: NaiveDate) -> Result<PutAwayView, CommandError> {
    let r = ops::put_away_shopping(store, Utc::now())?;
    let lots = r
        .lots
        .iter()
        .filter_map(|id| {
            store
                .inventory()
                .get(*id)
                .map(|l| view(l, status(l, today)))
        })
        .collect();
    Ok(PutAwayView {
        lots,
        items: r
            .items
            .into_iter()
            .map(|(id, s)| item_view(id, s))
            .collect(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn list_shopping(state: State<'_, AppState>) -> Result<ShoppingView, CommandError> {
    let store = state.store()?;
    list_shopping_in(&store)
}

#[tauri::command]
#[specta::specta]
pub fn add_shopping(
    state: State<'_, AppState>,
    input: AddShoppingInput,
) -> Result<ShoppingItemView, CommandError> {
    let mut store = state.store()?;
    let (id, s) = ops::add_shopping(
        &mut store,
        &input.item,
        input.qty.as_deref(),
        "manual",
        Utc::now(),
    )?;
    Ok(item_view(id, s))
}

#[tauri::command]
#[specta::specta]
pub fn add_plan_to_shopping(
    state: State<'_, AppState>,
    week: String,
) -> Result<AddedToShoppingView, CommandError> {
    let mut store = state.store()?;
    let r = ops::add_plan_to_shopping(&mut store, &week, Utc::now())?;
    debug_assert!(r.added.iter().all(|(_, s)| s.from == from_plan(&week)));
    Ok(AddedToShoppingView {
        added: r
            .added
            .into_iter()
            .map(|(id, s)| item_view(id, s))
            .collect(),
        skipped: r.skipped as u32,
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_shopping_done(
    state: State<'_, AppState>,
    input: SetShoppingDoneInput,
) -> Result<ShoppingItemView, CommandError> {
    let mut store = state.store()?;
    let s = ops::set_shopping_done(&mut store, &input.id, input.done, Utc::now())?;
    Ok(item_view(input.id, s))
}

#[tauri::command]
#[specta::specta]
pub fn delete_shopping(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    let mut store = state.store()?;
    Ok(ops::delete_shopping(&mut store, &id, Utc::now())?)
}

/// Removes every ticked item; how many.
#[tauri::command]
#[specta::specta]
pub fn clear_done_shopping(state: State<'_, AppState>) -> Result<u32, CommandError> {
    let mut store = state.store()?;
    Ok(ops::clear_done_shopping(&mut store, Utc::now())? as u32)
}

/// Ticked items become lots (fridge for fresh food, else pantry) and leave
/// the list.
#[tauri::command]
#[specta::specta]
pub fn put_away_shopping(state: State<'_, AppState>) -> Result<PutAwayView, CommandError> {
    let mut store = state.store()?;
    put_away_in(&mut store, Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Event, EventKind, Location, Qty, Source};
    use std::sync::Mutex;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    struct Fake(Mutex<Vec<Result<String, RecipeError>>>);

    impl TextModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn generate(
            &self,
            _: &str,
            _: u32,
            _: &mut dyn FnMut(&str),
        ) -> Result<String, RecipeError> {
            self.0.lock().unwrap().remove(0)
        }
    }

    fn stocked() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        for (item, bb) in [
            ("spinach", "2026-11-10"),
            ("eggs", "2026-11-20"),
            ("rice", "2027-01-01"),
        ] {
            s.append(&[Event {
                at: Utc::now(),
                lot: LotId::new(),
                kind: EventKind::Acquired {
                    item: item.into(),
                    qty: Qty::pieces(2),
                    location: Location::Fridge,
                    source: Source::Manual,
                    best_before: Some(bb.parse().unwrap()),
                },
            }])
            .unwrap();
        }
        s
    }

    const IDEAS: &str = "### Spinach omelette\nwhy: Uses the spinach.\ntime: 15\nuses: spinach, eggs\nneeds: cheese\n\
### Egg fried rice\nwhy: Quick.\ntime: 20\nuses: eggs, rice\nneeds: spring onion\n\
### Spinach rice\nwhy: Green.\ntime: 25\nuses: spinach, rice\nneeds: none\n";

    fn input(days: u32) -> MakePlanInput {
        MakePlanInput {
            template_id: Some(2),
            note: " no chilli ".into(),
            days,
            start: None,
        }
    }

    #[test]
    fn plans_with_a_fake_model_and_feeds_the_shopping_list() {
        let mut store = stocked();
        let model = Fake(Mutex::new(vec![Ok(IDEAS.into())]));
        let p = make_plan_in(&model, &mut store, &input(3), today()).unwrap();
        // Monday 2026-11-09 itself.
        assert_eq!(p.week, "2026-11-09");
        assert_eq!(p.days.len(), 3);
        assert_eq!(p.days[0].date, "2026-11-09");
        assert!(p.days[0].name.contains("pinach"), "{}", p.days[0].name);
        assert!(p.days[0].uses_soon >= 1);
        assert_eq!(p.template.as_deref(), Some("Quick weeknight"));
        assert_eq!(p.note, "no chilli");
        assert!(p.days.iter().all(|d| !d.have.is_empty()));

        // Stored, listed and read back.
        assert_eq!(ops::list_plans(&store).unwrap().len(), 1);
        let got = plan_view(
            &store,
            ops::get_plan(&store, "2026-11-09").unwrap(),
            today(),
        );
        assert_eq!(got.days.len(), 3);

        let added = ops::add_plan_to_shopping(&mut store, "2026-11-09", Utc::now()).unwrap();
        let names: Vec<&str> = added.added.iter().map(|(_, s)| s.item.as_str()).collect();
        assert!(
            names.contains(&"cheese") && names.contains(&"spring onion"),
            "{names:?}"
        );
        let v = list_shopping_in(&store).unwrap();
        assert_eq!(v.open, 2);
        assert!(v.groups.len() >= 2);
    }

    #[test]
    fn input_and_model_errors_map() {
        let mut store = stocked();
        let model = |r| Fake(Mutex::new(vec![r]));
        let field = |e: CommandError| match e {
            CommandError::BadInput(f) => f,
            e => panic!("{e:?}"),
        };
        for days in [0, 8] {
            let e = make_plan_in(&model(Ok(IDEAS.into())), &mut store, &input(days), today())
                .unwrap_err();
            assert_eq!(field(e), "days");
        }
        let mut i = input(3);
        i.start = Some("soon".into());
        assert_eq!(
            field(make_plan_in(&model(Ok(IDEAS.into())), &mut store, &i, today()).unwrap_err()),
            "start"
        );
        i.start = Some("2026-11-11".into());
        i.template_id = Some(999);
        assert!(matches!(
            make_plan_in(&model(Ok(IDEAS.into())), &mut store, &i, today()),
            Err(CommandError::NotFound)
        ));
        assert!(matches!(
            make_plan_in(
                &model(Err(RecipeError::Unreachable("http://x".into()))),
                &mut store,
                &input(3),
                today()
            ),
            Err(CommandError::RecipeModelUnavailable(_))
        ));
        let mut empty = Store::open_in_memory().unwrap();
        assert_eq!(
            field(
                make_plan_in(&model(Ok(IDEAS.into())), &mut empty, &input(3), today()).unwrap_err()
            ),
            "stock"
        );
        // A plan from a mid-week start belongs to that week's Monday.
        let mut i = input(2);
        i.start = Some("2026-11-11".into());
        let p = make_plan_in(&model(Ok(IDEAS.into())), &mut store, &i, today()).unwrap();
        assert_eq!(
            (p.week.as_str(), p.days[0].date.as_str()),
            ("2026-11-09", "2026-11-11")
        );
    }

    #[test]
    fn cooked_favourites_and_put_away_views() {
        let mut store = stocked();
        let lot = store.inventory().in_stock().next().unwrap().id;
        let bad = cooked_in(
            &mut store,
            CookedInput {
                items: vec![CookedItemInput {
                    lot: lot.short(),
                    amount: Some("5".into()),
                }],
                title: None,
            },
        );
        assert!(matches!(bad, Err(CommandError::BadInput(f)) if f == "amount:0"));
        let ok = cooked_in(
            &mut store,
            CookedInput {
                items: vec![CookedItemInput {
                    lot: lot.short(),
                    amount: Some("1".into()),
                }],
                title: Some("Omelette".into()),
            },
        )
        .unwrap();
        assert_eq!(
            ok,
            CookedView {
                lots: 1,
                used_up: 0
            }
        );

        let f = save_favourite_in(
            &mut store,
            SaveFavouriteInput {
                title: "Omelette".into(),
                serves: Some(2),
                minutes: None,
                ingredients: vec!["3 eggs".into(), "spinach".into()],
                steps: vec!["Cook.".into()],
                why: None,
            },
        )
        .unwrap();
        assert_eq!(f.recipe.ingredient_categories.len(), 2);
        assert_eq!(list_favourites_in(&store).unwrap().len(), 1);

        ops::add_shopping(&mut store, "kale", Some("2"), "manual", Utc::now()).unwrap();
        let id = list_shopping_in(&store).unwrap().groups[0].items[0]
            .id
            .clone();
        ops::set_shopping_done(&mut store, &id, true, Utc::now()).unwrap();
        let r = put_away_in(&mut store, today()).unwrap();
        assert_eq!((r.lots.len(), r.items.len()), (1, 1));
        assert_eq!(r.lots[0].location, "fridge");
        assert_eq!(r.lots[0].left, "2");
        assert!(list_shopping_in(&store).unwrap().groups.is_empty());
    }
}
