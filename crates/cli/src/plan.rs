//! `nookery plan` (a week of dinners from stock), `nookery shopping` and
//! `nookery favourites`. The rules live in `nookery_plan`; this parses
//! flags and prints. The model is a parameter so tests use a fake.

use std::io::Write;

use chrono::{NaiveDate, Utc};
use clap::{Args, Subcommand};
use nookery_plan::ops;
use nookery_plan::{
    default_start, from_plan, gather_pool, plan_week, Day, PlanDoc, PlanOptions, PlanRequest,
};
use nookery_recipes::{stock_from, RecipeError, Request, TextModel};
use nookery_store::{Store, DOC_FAVOURITE, DOC_SHOPPING};

use crate::settings::{cloud_note, sign_in_message, AiOverride, Who, RECIPES_SENT};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// Used when no template is named.
const DEFAULT_TEMPLATE: &str = "Dinners for the week, for two, mostly from what I have. Varied.";

#[derive(Args)]
pub struct PlanArgs {
    /// How many dinners (1-7).
    #[arg(long, default_value_t = 5)]
    pub days: u32,
    /// Template name (case-insensitive); default: a generic weekly request.
    #[arg(short, long)]
    pub template: Option<String>,
    /// Anything on top: "no chilli".
    #[arg(long)]
    pub note: Option<String>,
    /// First dinner's date (default: the next Monday, or today if it is one).
    #[arg(long)]
    pub start: Option<NaiveDate>,
    #[command(flatten)]
    pub ai: AiOverride,
    #[command(subcommand)]
    pub cmd: Option<PlanCmd>,
}

#[derive(Subcommand)]
pub enum PlanCmd {
    /// Show the stored plan (default: the next/this Monday's).
    Show { week: Option<NaiveDate> },
    /// Replace the dinner on DATE with the next spare.
    Swap { date: NaiveDate },
}

#[derive(Args)]
pub struct ShoppingArgs {
    #[command(subcommand)]
    pub cmd: Option<ShoppingCmd>,
}

#[derive(Subcommand)]
pub enum ShoppingCmd {
    /// Add an item: `nookery shopping add milk 2l`.
    Add { item: String, qty: Option<String> },
    /// Tick an item (by the id the list shows).
    Done { id: String },
    /// Untick an item.
    Undone { id: String },
    /// Remove an item.
    Rm { id: String },
    /// Add what a plan still needs (default week: the next/this Monday's).
    FromPlan { week: Option<NaiveDate> },
    /// Remove ticked items.
    Clear,
    /// Ticked items go into the kitchen and leave the list.
    PutAway,
}

#[derive(Args)]
pub struct FavouritesArgs {
    #[command(subcommand)]
    pub cmd: Option<FavouritesCmd>,
}

#[derive(Subcommand)]
pub enum FavouritesCmd {
    /// Forget a saved recipe (by the id the list shows).
    Rm { id: String },
}

/// The doc whose id ends with `short` (ids are UUID v7: the start is a
/// timestamp, so the end is what differs).
fn find_id(store: &Store, kind: &str, short: &str) -> Res<String> {
    let p = short.trim().to_ascii_lowercase();
    if p.is_empty() {
        return Err("give an id".into());
    }
    let mut hits = store
        .docs(kind)?
        .into_iter()
        .map(|d| d.id)
        .filter(|id| id.ends_with(&p));
    match (hits.next(), hits.next()) {
        (Some(id), None) => Ok(id),
        (None, _) => Err(format!("nothing ends with {short:?}").into()),
        _ => Err(format!("{short:?} matches several; type more of the id").into()),
    }
}

fn short(id: &str) -> &str {
    &id[id.len().saturating_sub(8)..]
}

fn recipe_error(e: RecipeError) -> Box<dyn std::error::Error> {
    match e {
        RecipeError::SignIn => sign_in_message("recipes").into(),
        RecipeError::NoCredits => format!("{e}: `nookery settings recipes local`").into(),
        e => e.into(),
    }
}

fn print_plan(store: &Store, plan: &PlanDoc, out: &mut dyn Write) -> Res<()> {
    writeln!(out, "Week of {}", plan.week)?;
    let ids = |d: &Day| {
        d.have
            .iter()
            .map(|h| match store.find_lot(&h.lot) {
                Ok(l) => format!("{} ({})", h.ingredient, l.short()),
                Err(_) => format!("{} (gone)", h.ingredient),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    for d in &plan.days {
        let date: NaiveDate = d.date.parse()?;
        let mins = d.minutes.map(|m| format!(" ({m} min)")).unwrap_or_default();
        let soon = if d.uses_soon > 0 {
            format!("  * uses {} soon", d.uses_soon)
        } else {
            String::new()
        };
        let cooked = if d.cooked { "  [cooked]" } else { "" };
        writeln!(
            out,
            "{} {}  {}{mins}{soon}{cooked}",
            date.format("%a"),
            date.format("%d %b"),
            d.name
        )?;
        if !d.why.is_empty() {
            writeln!(out, "    {}", d.why)?;
        }
        writeln!(out, "    have: {}", ids(d))?;
        if !d.need.is_empty() {
            writeln!(out, "    need: {}", d.need.join(", "))?;
        }
    }
    if !plan.spares.is_empty() {
        let names: Vec<&str> = plan.spares.iter().map(|d| d.name.as_str()).collect();
        writeln!(out, "Spares (nookery plan swap DATE): {}", names.join("; "))?;
    }
    Ok(())
}

fn week_arg(week: Option<NaiveDate>, today: NaiveDate) -> String {
    nookery_plan::monday_of(week.unwrap_or_else(|| default_start(today))).to_string()
}

pub fn run_plan(
    store: &mut Store,
    args: PlanArgs,
    model: &dyn TextModel,
    who: &dyn Fn() -> Who,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Res<()> {
    match args.cmd {
        Some(PlanCmd::Show { week }) => {
            let plan = ops::get_plan(store, &week_arg(week, today))
                .map_err(|_| "no plan for that week (make one: nookery plan)")?;
            return print_plan(store, &plan, out);
        }
        Some(PlanCmd::Swap { date }) => {
            let week = nookery_plan::monday_of(date).to_string();
            let plan =
                ops::swap_plan_day(store, &week, &date.to_string(), Utc::now()).map_err(|_| {
                    "can't swap that day: no such day, already cooked, or no spare left"
                })?;
            return print_plan(store, &plan, out);
        }
        None => {}
    }
    if !(1..=7).contains(&args.days) {
        return Err("--days is 1 to 7".into());
    }
    let (template, template_name) = match &args.template {
        Some(name) => {
            let t = store
                .templates()?
                .into_iter()
                .find(|t| t.name.to_lowercase() == name.trim().to_lowercase())
                .ok_or_else(|| format!("no template called {name:?} (see: nookery templates)"))?;
            (t.body, Some(t.name))
        }
        None => (DEFAULT_TEMPLATE.to_string(), None),
    };
    let note = args.note.clone().unwrap_or_default().trim().to_string();
    let stock = stock_from(store.inventory().in_stock(), today);
    let req = Request {
        template,
        note: note.clone(),
        count: args.days + 4,
    };
    let days = args.days as usize;
    let (pool, ms) = gather_pool(model, &stock, &req, days).map_err(recipe_error)?;
    let mut opts = PlanOptions::new(
        args.start.unwrap_or_else(|| default_start(today)),
        &Utc::now().to_rfc3339(),
    );
    opts.request = PlanRequest {
        template: template_name,
        note,
    };
    let plan = plan_week(&pool, &stock, days, &opts);
    if plan.days.is_empty() {
        writeln!(out, "The model gave no usable ideas; try again.")?;
        return Ok(());
    }
    ops::put_plan(store, &plan, Utc::now())?;
    print_plan(store, &plan, out)?;
    writeln!(
        out,
        "{}: {:.1} s · balanced by food groups (no nutrition numbers)",
        who().by,
        ms / 1e3
    )?;
    if who().cloud {
        eprintln!("{}", cloud_note(RECIPES_SENT));
    }
    Ok(())
}

fn print_list(store: &Store, out: &mut dyn Write) -> Res<()> {
    let all = ops::list_shopping(store)?;
    if all.is_empty() {
        writeln!(
            out,
            "The shopping list is empty. Add: nookery shopping add milk 2l"
        )?;
    }
    for (id, s) in all {
        writeln!(
            out,
            "[{}] {:<24} {:<8} {}  ({})",
            if s.done { "x" } else { " " },
            s.item,
            s.qty.unwrap_or_default(),
            short(&id),
            s.from
        )?;
    }
    Ok(())
}

pub fn run_shopping(
    store: &mut Store,
    args: ShoppingArgs,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Res<()> {
    let now = Utc::now();
    match args.cmd {
        None => {}
        Some(ShoppingCmd::Add { item, qty }) => {
            ops::add_shopping(store, &item, qty.as_deref(), "manual", now)?;
        }
        Some(ShoppingCmd::Done { id }) => {
            let id = find_id(store, DOC_SHOPPING, &id)?;
            ops::set_shopping_done(store, &id, true, now)?;
        }
        Some(ShoppingCmd::Undone { id }) => {
            let id = find_id(store, DOC_SHOPPING, &id)?;
            ops::set_shopping_done(store, &id, false, now)?;
        }
        Some(ShoppingCmd::Rm { id }) => {
            let id = find_id(store, DOC_SHOPPING, &id)?;
            ops::delete_shopping(store, &id, now)?;
        }
        Some(ShoppingCmd::FromPlan { week }) => {
            let week = week_arg(week, today);
            let r = ops::add_plan_to_shopping(store, &week, now)
                .map_err(|_| format!("no plan for the week of {week} (make one: nookery plan)"))?;
            writeln!(
                out,
                "added {} for {} ({} already on the list or in stock)",
                r.added.len(),
                from_plan(&week),
                r.skipped
            )?;
        }
        Some(ShoppingCmd::Clear) => {
            let n = ops::clear_done_shopping(store, now)?;
            writeln!(out, "removed {n} ticked")?;
        }
        Some(ShoppingCmd::PutAway) => {
            let r = ops::put_away_shopping(store, now)?;
            for lot in &r.lots {
                if let Some(l) = store.inventory().get(*lot) {
                    writeln!(
                        out,
                        "put {} {} in the {}",
                        l.left,
                        l.item,
                        l.location.as_str()
                    )?;
                }
            }
            if r.lots.is_empty() {
                writeln!(out, "nothing ticked")?;
            }
        }
    }
    print_list(store, out)
}

pub fn run_favourites(store: &mut Store, args: FavouritesArgs, out: &mut dyn Write) -> Res<()> {
    if let Some(FavouritesCmd::Rm { id }) = args.cmd {
        let id = find_id(store, DOC_FAVOURITE, &id)?;
        ops::delete_favourite(store, &id, Utc::now())?;
    }
    let all = ops::list_favourites(store)?;
    if all.is_empty() {
        writeln!(
            out,
            "No saved recipes yet (the heart on a recipe saves it)."
        )?;
    }
    for (id, f) in all {
        let when = f.saved_at.get(..10).unwrap_or("");
        writeln!(
            out,
            "{}  {:<32} cooked {}x  saved {when}",
            short(&id),
            f.recipe.title,
            f.cooked
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Event, EventKind, Location, LotId, Qty, Source};
    use std::sync::Mutex;

    struct Fake(Mutex<Vec<String>>);
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
            Ok(self.0.lock().unwrap().remove(0))
        }
    }

    fn who() -> Who {
        Who {
            by: "fake".into(),
            cloud: false,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    fn stocked() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        for item in ["spinach", "eggs", "rice"] {
            s.append(&[Event {
                at: Utc::now(),
                lot: LotId::new(),
                kind: EventKind::Acquired {
                    item: item.into(),
                    qty: Qty::pieces(2),
                    location: Location::Fridge,
                    source: Source::Manual,
                    best_before: Some("2026-11-10".parse().unwrap()),
                },
            }])
            .unwrap();
        }
        s
    }

    const IDEAS: &str =
        "### Spinach omelette\nwhy: Green.\ntime: 15\nuses: spinach, eggs\nneeds: cheese\n\
### Egg fried rice\nwhy: Quick.\ntime: 20\nuses: eggs, rice\nneeds: spring onion\n\
### Spinach rice\nwhy: Plain.\ntime: 25\nuses: spinach, rice\nneeds: none\n";

    fn plan_args(days: u32) -> PlanArgs {
        PlanArgs {
            days,
            template: None,
            note: None,
            start: None,
            ai: AiOverride::default(),
            cmd: None,
        }
    }

    fn text(out: Vec<u8>) -> String {
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn plans_swaps_shops_and_puts_away() {
        let mut store = stocked();
        let model = Fake(Mutex::new(vec![IDEAS.into()]));
        let mut out = Vec::new();
        run_plan(
            &mut store,
            plan_args(2),
            &model,
            &|| who(),
            today(),
            &mut out,
        )
        .unwrap();
        let t = text(out);
        assert!(t.contains("Week of 2026-11-09"), "{t}");
        assert!(t.contains("Mon 09 Nov"), "{t}");
        assert!(t.contains("Spares"), "{t}");
        assert!(t.contains("fake:"), "{t}");

        let mut out = Vec::new();
        let swap = PlanArgs {
            cmd: Some(PlanCmd::Swap {
                date: "2026-11-10".parse().unwrap(),
            }),
            ..plan_args(2)
        };
        run_plan(&mut store, swap, &model, &|| who(), today(), &mut out).unwrap();
        assert!(text(out).contains("Tue 10 Nov"));

        let mut out = Vec::new();
        let from = ShoppingArgs {
            cmd: Some(ShoppingCmd::FromPlan { week: None }),
        };
        run_shopping(&mut store, from, today(), &mut out).unwrap();
        assert!(text(out).contains("added"));
        let (id, _) = ops::list_shopping(&store).unwrap().remove(0);
        let mut out = Vec::new();
        let done = ShoppingArgs {
            cmd: Some(ShoppingCmd::Done {
                id: short(&id).into(),
            }),
        };
        run_shopping(&mut store, done, today(), &mut out).unwrap();
        assert!(text(out).contains("[x]"));
        let mut out = Vec::new();
        let put = ShoppingArgs {
            cmd: Some(ShoppingCmd::PutAway),
        };
        run_shopping(&mut store, put, today(), &mut out).unwrap();
        assert!(text(out).contains("put "));
    }

    #[test]
    fn bad_days_template_and_missing_plan_are_friendly() {
        let mut store = stocked();
        let model = Fake(Mutex::new(vec![]));
        let mut out = Vec::new();
        let e = run_plan(
            &mut store,
            plan_args(9),
            &model,
            &|| who(),
            today(),
            &mut out,
        );
        assert!(e.unwrap_err().to_string().contains("1 to 7"));
        let args = PlanArgs {
            template: Some("nope".into()),
            ..plan_args(3)
        };
        let e = run_plan(&mut store, args, &model, &|| who(), today(), &mut out);
        assert!(e.unwrap_err().to_string().contains("no template"));
        let show = PlanArgs {
            cmd: Some(PlanCmd::Show { week: None }),
            ..plan_args(3)
        };
        let e = run_plan(&mut store, show, &model, &|| who(), today(), &mut out);
        assert!(e.unwrap_err().to_string().contains("no plan"));
    }

    #[test]
    fn favourites_list_and_remove() {
        let mut store = stocked();
        let mut out = Vec::new();
        run_favourites(&mut store, FavouritesArgs { cmd: None }, &mut out).unwrap();
        assert!(text(out).contains("No saved"));
        let r = nookery_plan::FavouriteRecipe {
            title: "Pancakes".into(),
            serves: None,
            minutes: None,
            ingredients: vec![],
            ingredient_categories: vec![],
            steps: vec![],
        };
        let (id, _) = ops::save_favourite(&mut store, r, None, Utc::now()).unwrap();
        let mut out = Vec::new();
        run_favourites(&mut store, FavouritesArgs { cmd: None }, &mut out).unwrap();
        assert!(text(out).contains("Pancakes"));
        let rm = FavouritesArgs {
            cmd: Some(FavouritesCmd::Rm {
                id: short(&id).into(),
            }),
        };
        let mut out = Vec::new();
        run_favourites(&mut store, rm, &mut out).unwrap();
        assert!(text(out).contains("No saved"));
    }
}
