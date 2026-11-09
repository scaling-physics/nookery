//! `nookery`: the kitchen ledger from the terminal. Shares the app's database
//! (`$NOOKERY_DATA_DIR` or the OS app-data folder; `--data` overrides).

mod photo;
mod receipt;
mod recipes;
mod settings;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use chrono::{Local, NaiveDate, Utc};
use clap::{Parser, Subcommand};
use nookery_core::{parse_date, use_first, Event, EventKind, Location, Lot, Qty, Source, Status};
use nookery_store::{default_data_dir, Store, SETTING_PHOTOS_AI, SETTING_RECIPES_AI};

#[derive(Parser)]
#[command(
    name = "nookery",
    version,
    about = "Kitchen inventory: what you have and what to use first"
)]
struct Cli {
    /// Data folder (default: $NOOKERY_DATA_DIR or the app's data folder).
    #[arg(long, global = true)]
    data: Option<PathBuf>,
    /// Print how long opening the database and the command took.
    #[arg(long, global = true)]
    time: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Add something you bought or found.
    Add {
        item: String,
        /// Amount: 2, 0.5, 500g, 1.5kg, 750ml, 1l.
        #[arg(short, long, default_value = "1")]
        qty: Qty,
        /// fridge, freezer or pantry.
        #[arg(short = 'i', long = "in", default_value = "fridge")]
        location: Location,
        /// Best before: 2026-11-20, 20.11., 20.11.2026 or +3.
        #[arg(short, long)]
        bb: Option<String>,
        /// Store it came from (makes it a receipt purchase).
        #[arg(short, long)]
        store: Option<String>,
        /// Price in the store's currency, e.g. 1.29.
        #[arg(short, long, requires = "store")]
        price: Option<String>,
    },
    /// What's in stock, in the order to use it.
    Ls {
        /// Only one place.
        #[arg(short = 'i', long = "in")]
        location: Option<Location>,
        /// Include used and binned lots.
        #[arg(short, long)]
        all: bool,
    },
    /// Eat or cook some (default: all) of a lot.
    Use { id: String, qty: Option<Qty> },
    /// Mark a lot opened.
    Open { id: String },
    /// Throw a lot away.
    Bin { id: String },
    /// Move a lot (to the freezer = freeze it).
    Move { id: String, to: Location },
    /// Confirm a lot is still there, optionally with what's left.
    Seen {
        id: String,
        #[arg(short, long)]
        left: Option<Qty>,
    },
    /// Every event, or one lot's history.
    Log { id: Option<String> },
    /// Read a receipt (text file or -), and with --import add its lots.
    Receipt(receipt::ReceiptArgs),
    /// Scan a photo of the fridge, freezer or pantry with the local vision model.
    Photo(photo::PhotoArgs),
    /// Recipe ideas from what's in stock (--cook N writes the full recipe).
    Recipes(recipes::RecipesArgs),
    /// List, add, edit, remove and order the saved "what I want" templates.
    Templates(recipes::TemplatesArgs),
    /// Where the AI runs (this Mac or the cloud) and which cloud model.
    Settings(settings::SettingsArgs),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("nookery: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let dir = cli.data.clone().unwrap_or_else(default_data_dir);
    let t0 = Instant::now();
    let mut store = Store::open_dir(&dir).map_err(|e| e.to_string())?;
    let opened = t0.elapsed();
    let today = Local::now().date_naive();
    let t1 = Instant::now();
    execute(&mut store, cli.cmd, today, cli.time).map_err(|e| e.to_string())?;
    if cli.time {
        eprintln!(
            "open + replay {} events: {:.2} ms · command: {:.2} ms",
            store.event_count(),
            opened.as_secs_f64() * 1e3,
            t1.elapsed().as_secs_f64() * 1e3
        );
    }
    Ok(())
}

fn execute(
    store: &mut Store,
    cmd: Cmd,
    today: NaiveDate,
    time: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let one =
        |store: &mut Store, id: &str, kind: EventKind| -> Result<(), Box<dyn std::error::Error>> {
            let lot = store.find_lot(id)?;
            store.append(&[Event { at: now, lot, kind }])?;
            let l = store.inventory().get(lot).expect("just found");
            println!("{}", line(l, nookery_core::status(l, today)));
            Ok(())
        };
    match cmd {
        Cmd::Add {
            item,
            qty,
            location,
            bb,
            store: shop,
            price,
        } => {
            let best_before = bb
                .map(|s| parse_date(&s, today).ok_or(format!("can't read the date {s:?}")))
                .transpose()?;
            let price_cents = price
                .map(|p| parse_cents(&p).ok_or(format!("can't read the price {p:?}")))
                .transpose()?;
            let source = match shop {
                Some(store) => Source::Receipt {
                    store,
                    line: None,
                    price_cents,
                    receipt: None,
                },
                None => Source::Manual,
            };
            let lot = nookery_core::LotId::new();
            store.append(&[Event {
                at: now,
                lot,
                kind: EventKind::Acquired {
                    item,
                    qty,
                    location,
                    source,
                    best_before,
                },
            }])?;
            let l = store.inventory().get(lot).expect("just added");
            println!("{}", line(l, nookery_core::status(l, today)));
        }
        Cmd::Ls { location, all } => {
            let inv = store.inventory();
            let lots: Vec<&Lot> = inv
                .all()
                .iter()
                .filter(|l| all || l.in_stock())
                .filter(|l| location.is_none_or(|p| l.location == p))
                .collect();
            if lots.is_empty() {
                println!("Nothing here yet. Add something: nookery add milk --qty 1l --bb +7");
            }
            for (l, s) in use_first(lots, today) {
                println!("{}", line(l, s));
            }
        }
        Cmd::Use { id, qty } => one(store, &id, EventKind::Used { qty })?,
        Cmd::Open { id } => one(store, &id, EventKind::Opened)?,
        Cmd::Bin { id } => one(store, &id, EventKind::Binned)?,
        Cmd::Move { id, to } => one(store, &id, EventKind::Moved { to })?,
        Cmd::Seen { id, left } => {
            let lot = store.find_lot(&id)?;
            let location = store.inventory().get(lot).expect("found").location;
            one(store, &id, EventKind::Seen { location, left })?
        }
        Cmd::Receipt(args) => receipt::run(store, args, today, time)?,
        Cmd::Photo(args) => {
            let mode = args.ai.mode(store.ai_mode(SETTING_PHOTOS_AI)?);
            let chosen = nookery_vision::Chosen::for_mode(mode, &store.cloud_model()?);
            photo::run(
                store,
                args,
                &chosen,
                &|| settings::Who {
                    by: chosen.answered_by(),
                    cloud: chosen.used_cloud(),
                },
                today,
                time,
                photo::stderr_is_terminal(),
            )?
        }
        Cmd::Recipes(args) => {
            let mode = args.ai.mode(store.ai_mode(SETTING_RECIPES_AI)?);
            let chosen = nookery_recipes::ChosenText::for_mode(mode, &store.cloud_model()?);
            recipes::run_recipes(
                store,
                args,
                &chosen,
                &|| settings::Who {
                    by: chosen.answered_by(),
                    cloud: chosen.used_cloud(),
                },
                today,
                &mut std::io::stdout(),
            )?
        }
        Cmd::Templates(args) => recipes::run_templates(store, args, &mut std::io::stdout())?,
        Cmd::Settings(args) => {
            let cloud = nookery_recipes::ChosenText::for_mode(
                nookery_core::AiMode::Cloud,
                &store.cloud_model()?,
            );
            settings::run(store, args, &cloud, &mut std::io::stdout())?
        }
        Cmd::Log { id } => {
            let events = match id {
                Some(id) => store.lot_events(store.find_lot(&id)?)?,
                None => store.events()?,
            };
            for e in events {
                println!(
                    "{}  {}  {}",
                    e.at.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
                    e.lot.short(),
                    describe(&e.kind)
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn line(l: &Lot, s: Status) -> String {
    let status = match (l.gone, s) {
        (Some(g), _) => format!("{g:?}").to_lowercase(),
        (None, Status::Expired(d)) => format!("expired {d}d ago"),
        (None, Status::UseSoon(0)) => "use today".into(),
        (None, Status::UseSoon(d)) => format!("use within {d}d"),
        (None, Status::Fresh(d)) => format!("{d}d left"),
        (None, Status::Unconfirmed(d)) => format!("not seen for {d}d"),
        (None, Status::Undated) => "".into(),
    };
    let opened = if l.opened_at.is_some() {
        " · opened"
    } else {
        ""
    };
    format!(
        "{}  {:<8} {:<24} {:>7}  {}{}",
        l.id.short(),
        l.location.as_str(),
        l.item,
        l.left.to_string(),
        status,
        opened
    )
}

fn describe(k: &EventKind) -> String {
    match k {
        EventKind::Acquired {
            item,
            qty,
            location,
            source,
            best_before,
        } => {
            let from = match source {
                Source::Receipt { store, .. } => format!(" from {store}"),
                Source::Photo => " from a photo".into(),
                Source::Manual => String::new(),
            };
            let bb = best_before
                .map(|d| format!(", best before {d}"))
                .unwrap_or_default();
            format!(
                "acquired {qty} {item} into the {}{from}{bb}",
                location.as_str()
            )
        }
        EventKind::Seen { location, left } => match left {
            Some(q) => format!("seen in the {}, {q} left", location.as_str()),
            None => format!("seen in the {}", location.as_str()),
        },
        EventKind::Moved { to } => format!("moved to the {}", to.as_str()),
        EventKind::Opened => "opened".into(),
        EventKind::Used { qty: Some(q) } => format!("used {q}"),
        EventKind::Used { qty: None } => "used up".into(),
        EventKind::Binned => "binned".into(),
    }
}

/// `1.29`, `1,29`, `2` -> cents.
fn parse_cents(s: &str) -> Option<i64> {
    let t = s
        .trim()
        .trim_start_matches(['€', '$', '£'])
        .replace(',', ".");
    let (whole, frac) = t.split_once('.').unwrap_or((&t, ""));
    if frac.len() > 2 {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let frac: i64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<2}").parse().ok()?
    };
    Some(whole * 100 + frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_prices() {
        assert_eq!(parse_cents("1.29"), Some(129));
        assert_eq!(parse_cents("1,2"), Some(120));
        assert_eq!(parse_cents("€3"), Some(300));
        assert_eq!(parse_cents("1.299"), None);
        assert_eq!(parse_cents("abc"), None);
    }

    #[test]
    fn add_use_and_list_work_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open_dir(dir.path()).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        let add = Cmd::Add {
            item: "eggs".into(),
            qty: Qty::pieces(6),
            location: Location::Fridge,
            bb: Some("+10".into()),
            store: Some("Lidl".into()),
            price: Some("2.29".into()),
        };
        execute(&mut store, add, today, false).unwrap();
        let id = store.inventory().all()[0].id.short();
        execute(
            &mut store,
            Cmd::Use {
                id,
                qty: Some(Qty::pieces(2)),
            },
            today,
            false,
        )
        .unwrap();
        let lot = &store.inventory().all()[0];
        assert_eq!(lot.left, Qty::pieces(4));
        assert!(matches!(
            lot.source,
            Source::Receipt {
                price_cents: Some(229),
                receipt: None,
                ..
            }
        ));
        execute(
            &mut store,
            Cmd::Ls {
                location: None,
                all: false,
            },
            today,
            false,
        )
        .unwrap();
    }
}
