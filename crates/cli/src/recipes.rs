//! `nookery recipes` (ideas from what's in stock, optionally the full recipe
//! for one) and `nookery templates` (the saved "what I want" texts). The
//! logic lives in `nookery_recipes`; this parses flags and prints. The model
//! is a parameter so tests use a fake.

use std::io::Write;

use chrono::{NaiveDate, Utc};
use clap::{Args, Subcommand};
use nookery_recipes::{
    stock_from, suggest, write_recipe, RecipeError, Request, StockItem, TextModel,
};
use nookery_store::Store;

use crate::settings::{cloud_note, sign_in_message, AiOverride, Who, RECIPES_SENT};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Args)]
pub struct RecipesArgs {
    /// Template name (case-insensitive); default: the first template.
    #[arg(short, long)]
    pub template: Option<String>,
    /// Anything on top: "something Indian", "no chilli".
    #[arg(long)]
    pub note: Option<String>,
    /// How many ideas (1-8).
    #[arg(short = 'n', long, default_value_t = 4)]
    pub count: u32,
    /// After the ideas, write the full recipe for idea N.
    #[arg(long, value_name = "N")]
    pub cook: Option<usize>,
    #[command(flatten)]
    pub ai: AiOverride,
}

#[derive(Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    pub cmd: Option<TemplatesCmd>,
}

#[derive(Subcommand)]
pub enum TemplatesCmd {
    /// Add a template at the end.
    Add { name: String, text: String },
    /// Change a template's name and/or text.
    Edit {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        text: Option<String>,
    },
    /// Delete a template.
    Rm { id: i64 },
    /// Put the templates in this order (every id once).
    Order { ids: Vec<i64> },
}

pub fn run_templates(store: &mut Store, args: TemplatesArgs, out: &mut dyn Write) -> Res<()> {
    match args.cmd {
        None => {}
        Some(TemplatesCmd::Add { name, text }) => {
            store.save_template(None, &name, &text, Utc::now())?;
        }
        Some(TemplatesCmd::Edit { id, name, text }) => {
            let cur = store
                .templates()?
                .into_iter()
                .find(|t| t.id == id)
                .ok_or(format!("no template {id}"))?;
            store.save_template(
                Some(id),
                name.as_deref().unwrap_or(&cur.name),
                text.as_deref().unwrap_or(&cur.body),
                Utc::now(),
            )?;
        }
        Some(TemplatesCmd::Rm { id }) => store.delete_template(id)?,
        Some(TemplatesCmd::Order { ids }) => store.order_templates(&ids)?,
    }
    for t in store.templates()? {
        writeln!(out, "{:>3}  {:<20} {}", t.id, t.name, t.body)?;
    }
    Ok(())
}

fn request(store: &Store, args: &RecipesArgs) -> Res<Request> {
    let templates = store.templates()?;
    let template = match &args.template {
        Some(name) => templates
            .iter()
            .find(|t| t.name.to_lowercase() == name.trim().to_lowercase())
            .ok_or_else(|| format!("no template called {name:?} (see: nookery templates)"))?
            .body
            .clone(),
        None => templates
            .first()
            .map(|t| t.body.clone())
            .unwrap_or_default(),
    };
    Ok(Request {
        template,
        note: args.note.clone().unwrap_or_default(),
        count: args.count.clamp(1, 8),
    })
}

fn recipe_error(e: RecipeError) -> Box<dyn std::error::Error> {
    match e {
        RecipeError::SignIn => sign_in_message("recipes").into(),
        RecipeError::NoCredits => format!("{e}: `nookery settings recipes local`").into(),
        e => e.into(),
    }
}

fn ids_of(names: &[(String, nookery_core::LotId)]) -> String {
    names
        .iter()
        .map(|(n, l)| format!("{n} ({})", l.short()))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn run_recipes(
    store: &Store,
    args: RecipesArgs,
    model: &dyn TextModel,
    who: &dyn Fn() -> Who,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Res<()> {
    let req = request(store, &args)?;
    let stock: Vec<StockItem> = stock_from(store.inventory().all(), today);
    let s = suggest(model, &stock, &req).map_err(recipe_error)?;
    if s.items.is_empty() {
        writeln!(out, "The model gave no usable ideas; try again.")?;
    }
    for (i, x) in s.items.iter().enumerate() {
        let mins = x
            .idea
            .minutes
            .map(|m| format!(" ({m} min)"))
            .unwrap_or_default();
        writeln!(out, "{}. {}{}", i + 1, x.idea.name, mins)?;
        if !x.idea.why.is_empty() {
            writeln!(out, "   {}", x.idea.why)?;
        }
        writeln!(out, "   have: {}", ids_of(&x.have))?;
        if !x.need.is_empty() {
            writeln!(out, "   need: {}", x.need.join(", "))?;
        }
        if !x.expired.is_empty() {
            let names: Vec<(String, _)> = x
                .expired
                .iter()
                .map(|l| {
                    let n = stock.iter().find(|s| s.lot == *l).map(|s| s.item.clone());
                    (n.unwrap_or_default(), *l)
                })
                .collect();
            writeln!(out, "   ⚠ expired: {}", ids_of(&names))?;
        }
    }
    writeln!(out, "{}: {:.1} s", who().by, s.ms / 1e3)?;
    if let Some(n) = args.cook {
        let top = n
            .checked_sub(1)
            .and_then(|i| s.items.get(i))
            .ok_or_else(|| format!("no idea number {n}"))?;
        writeln!(out, "\n--- {} ---", top.idea.name)?;
        let t = std::time::Instant::now();
        let mut write_err = None;
        write_recipe(model, top, &stock, &req, &mut |piece| {
            if let Err(e) = write!(out, "{piece}").and_then(|_| out.flush()) {
                write_err.get_or_insert(e);
            }
        })
        .map_err(recipe_error)?;
        if let Some(e) = write_err {
            return Err(e.into());
        }
        writeln!(out, "\nrecipe: {:.1} s", t.elapsed().as_secs_f64())?;
    }
    if who().cloud {
        eprintln!("{}", cloud_note(RECIPES_SENT));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Event, EventKind, Location, LotId, Qty, Source};
    use nookery_recipes::RecipeError;
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
            on_text: &mut dyn FnMut(&str),
        ) -> Result<String, RecipeError> {
            let a = self.0.lock().unwrap().remove(0);
            for piece in a.split_inclusive('\n') {
                on_text(piece);
            }
            Ok(a)
        }
    }

    fn fake_who() -> Who {
        Who {
            by: "fake".into(),
            cloud: false,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    fn args(cook: Option<usize>) -> RecipesArgs {
        RecipesArgs {
            template: None,
            note: Some("no chilli".into()),
            count: 4,
            cook,
            ai: AiOverride::default(),
        }
    }

    fn with_stock() -> (Store, LotId) {
        let mut s = Store::open_in_memory().unwrap();
        let lot = LotId::new();
        s.append(&[Event {
            at: Utc::now(),
            lot,
            kind: EventKind::Acquired {
                item: "skyr".into(),
                qty: Qty::grams(500),
                location: Location::Fridge,
                source: Source::Manual,
                best_before: None,
            },
        }])
        .unwrap();
        (s, lot)
    }

    fn text(out: Vec<u8>) -> String {
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn templates_can_be_added_edited_ordered_and_removed() {
        let mut s = Store::open_in_memory().unwrap();
        let before = s.templates().unwrap().len();
        let mut out = Vec::new();
        let add = TemplatesArgs {
            cmd: Some(TemplatesCmd::Add {
                name: "Snack".into(),
                text: "A quick snack".into(),
            }),
        };
        run_templates(&mut s, add, &mut out).unwrap();
        assert!(text(out).contains("Snack"));
        let id = s.templates().unwrap().last().unwrap().id;
        let edit = TemplatesArgs {
            cmd: Some(TemplatesCmd::Edit {
                id,
                name: None,
                text: Some("Something small".into()),
            }),
        };
        run_templates(&mut s, edit, &mut Vec::new()).unwrap();
        let t = s.templates().unwrap().pop().unwrap();
        assert_eq!(
            (t.name.as_str(), t.body.as_str()),
            ("Snack", "Something small")
        );
        let mut ids: Vec<i64> = s.templates().unwrap().iter().map(|t| t.id).collect();
        ids.reverse();
        let order = TemplatesArgs {
            cmd: Some(TemplatesCmd::Order { ids }),
        };
        run_templates(&mut s, order, &mut Vec::new()).unwrap();
        assert_eq!(s.templates().unwrap()[0].id, id);
        let rm = TemplatesArgs {
            cmd: Some(TemplatesCmd::Rm { id }),
        };
        run_templates(&mut s, rm, &mut Vec::new()).unwrap();
        assert_eq!(s.templates().unwrap().len(), before);
        let bad = TemplatesArgs {
            cmd: Some(TemplatesCmd::Rm { id }),
        };
        assert!(run_templates(&mut s, bad, &mut Vec::new()).is_err());
    }

    #[test]
    fn recipes_prints_ideas_with_lot_ids_and_timing() {
        let (s, lot) = with_stock();
        let m = Fake(Mutex::new(vec![
            "### Skyr bowl\nwhy: easy\ntime: 5\nuses: skyr, egg\nneeds: honey\n".into(),
        ]));
        let mut out = Vec::new();
        run_recipes(&s, args(None), &m, &fake_who, today(), &mut out).unwrap();
        let o = text(out);
        assert!(o.contains("1. Skyr bowl (5 min)"), "{o}");
        assert!(o.contains(&format!("have: skyr ({})", lot.short())), "{o}");
        assert!(o.contains("need: egg, honey"), "{o}");
        assert!(o.contains("fake: "), "{o}");
    }

    #[test]
    fn cook_streams_the_full_recipe() {
        let (s, _) = with_stock();
        let m = Fake(Mutex::new(vec![
            "### Skyr bowl\ntime: 5\nuses: skyr\nneeds: none\n".into(),
            "# Skyr bowl\n## Steps\n1. Eat.\n".into(),
        ]));
        let mut out = Vec::new();
        run_recipes(&s, args(Some(1)), &m, &fake_who, today(), &mut out).unwrap();
        let o = text(out);
        assert!(o.contains("1. Eat."), "{o}");
        assert!(o.contains("recipe: "), "{o}");
    }

    #[test]
    fn cook_with_a_bad_number_and_unknown_template_are_errors() {
        let (s, _) = with_stock();
        let answer = "### A\nuses: skyr\n".to_string();
        let m = Fake(Mutex::new(vec![answer]));
        assert!(run_recipes(&s, args(Some(5)), &m, &fake_who, today(), &mut Vec::new()).is_err());
        let mut a = args(None);
        a.template = Some("nope".into());
        assert!(run_recipes(&s, a, &m, &fake_who, today(), &mut Vec::new()).is_err());
    }

    #[test]
    fn sign_in_points_at_the_recipes_setting() {
        struct Out;
        impl TextModel for Out {
            fn name(&self) -> String {
                "out".into()
            }
            fn generate(
                &self,
                _: &str,
                _: u32,
                _: &mut dyn FnMut(&str),
            ) -> Result<String, RecipeError> {
                Err(RecipeError::SignIn)
            }
        }
        let (s, _) = with_stock();
        let e = run_recipes(&s, args(None), &Out, &fake_who, today(), &mut Vec::new())
            .unwrap_err()
            .to_string();
        assert!(e.contains("ollama signin"), "{e}");
        assert!(e.contains("nookery settings recipes local"), "{e}");
    }
}
