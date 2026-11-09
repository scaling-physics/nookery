//! `nookery photo FILE --in <place>`: scan a photo of the fridge, freezer or
//! pantry with the local vision model, show what it saw against the ledger,
//! and (with `--import`) mark matches seen and add what the user names.
//! The logic lives in `nookery_vision::{plan, decide}`; this only parses
//! flags and prints.

use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use clap::Args;
use nookery_core::{parse_date, Location, Qty};
use nookery_store::{Store, PHOTO_ALIASES};

use crate::settings::{cloud_note, sign_in_message, AiOverride, Who, PHOTOS_SENT};
use nookery_vision::{decide, plan, scan, Decision, Plan, TileOptions, VisionError, VisionModel};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Args)]
pub struct PhotoArgs {
    /// Photo (jpg, png or webp).
    pub file: PathBuf,
    /// Where the photo was taken: fridge, freezer or pantry.
    #[arg(short = 'i', long = "in")]
    pub place: Location,
    /// Write the result (mark matches seen, add --add sightings, learn names).
    #[arg(long)]
    pub import: bool,
    /// Add sighting N as a new lot: N[=item[,qty][,bb]].
    #[arg(long = "add", value_name = "N[=ITEM[,QTY][,BB]]")]
    pub adds: Vec<AddSpec>,
    /// Ignore sighting N (a wrong guess).
    #[arg(long = "skip", value_name = "N")]
    pub skips: Vec<usize>,
    #[command(flatten)]
    pub ai: AiOverride,
}

/// One `--add` value. `sighting` is 1-based, as printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddSpec {
    pub sighting: usize,
    pub item: Option<String>,
    pub qty: Option<Qty>,
    pub best_before: Option<String>,
}

impl std::str::FromStr for AddSpec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (n, rest) = s.split_once('=').unwrap_or((s, ""));
        let sighting: usize = n
            .trim()
            .parse()
            .ok()
            .filter(|&n| n >= 1)
            .ok_or_else(|| format!("{s:?}: {n:?} is not a sighting number"))?;
        let mut parts = rest.split(',').map(str::trim);
        // Empty fields mean "default", so `3=,,+5` sets only the date.
        let mut field = || parts.next().filter(|p| !p.is_empty());
        let item = field().map(String::from);
        let qty = field()
            .map(|q| {
                q.parse::<Qty>()
                    .map_err(|_| format!("{s:?}: can't read the amount {q:?}"))
            })
            .transpose()?;
        let best_before = field().map(String::from);
        if parts.next().is_some() {
            return Err(format!("{s:?}: too many fields (item,qty,bb)"));
        }
        Ok(AddSpec {
            sighting,
            item,
            qty,
            best_before,
        })
    }
}

/// Decisions for `decide` plus the (1-based) unmatched sightings left out.
#[derive(Debug, PartialEq, Eq)]
pub struct Decisions {
    pub decisions: Vec<(usize, Decision)>,
    pub left_out: Vec<usize>,
}

/// Plan + flags -> decisions. Matched sightings are marked seen unless
/// `--add`ed (a second one) or `--skip`ped; unmatched ones need `--add`.
pub fn build_decisions(
    plan: &Plan,
    adds: &[AddSpec],
    skips: &[usize],
    today: NaiveDate,
) -> Res<Decisions> {
    let mut touched = HashSet::new();
    for n in adds.iter().map(|a| a.sighting).chain(skips.iter().copied()) {
        if n < 1 || n > plan.items.len() {
            return Err(format!(
                "sighting {n} doesn't exist (the photo has {})",
                plan.items.len()
            )
            .into());
        }
        if !touched.insert(n) {
            return Err(format!("sighting {n} is named more than once in --add/--skip").into());
        }
    }
    let mut decisions = Vec::new();
    let mut left_out = Vec::new();
    for (i, p) in plan.items.iter().enumerate() {
        let n = i + 1;
        if skips.contains(&n) {
            decisions.push((i, Decision::Ignore));
        } else if let Some(a) = adds.iter().find(|a| a.sighting == n) {
            let best_before = a
                .best_before
                .as_deref()
                .map(|b| parse_date(b, today).ok_or(format!("can't read the date {b:?}")))
                .transpose()?;
            decisions.push((
                i,
                Decision::Add {
                    item: a.item.clone().unwrap_or_else(|| p.item.clone()),
                    qty: a.qty.unwrap_or(Qty::pieces(p.count as i64)),
                    best_before,
                },
            ));
        } else if !p.lots.is_empty() {
            decisions.push((i, Decision::Seen));
        } else {
            left_out.push(n);
        }
    }
    Ok(Decisions {
        decisions,
        left_out,
    })
}

fn vision_error(e: VisionError, model: &str) -> Box<dyn std::error::Error> {
    match e {
        VisionError::Unreachable(url) => format!(
            "the vision model isn't reachable at {url}: start Ollama (open the app or run `ollama serve`) and try again"
        )
        .into(),
        VisionError::ModelMissing(m) => format!(
            "the vision model {m} isn't installed: run `ollama pull {m}` (scanning with {model})"
        )
        .into(),
        VisionError::SignIn => sign_in_message("photos").into(),
        VisionError::NoCredits => format!("{e}: `nookery settings photos local`").into(),
        e => e.into(),
    }
}

pub fn run(
    store: &mut Store,
    args: PhotoArgs,
    model: &dyn VisionModel,
    who: &dyn Fn() -> Who,
    today: NaiveDate,
    time: bool,
    show_progress: bool,
) -> Res<()> {
    let image = std::fs::read(&args.file)
        .map_err(|e| format!("can't read {}: {e}", args.file.display()))?;
    let opts = TileOptions::for_place(args.place);
    let mut progress = |done: usize, total: usize| {
        if show_progress {
            eprint!("\rtile {done}/{total}…");
            if done == total {
                eprintln!();
            }
        }
    };
    let result = scan(model, &image, args.place, &opts, &mut progress)
        .map_err(|e| vision_error(e, &model.name()))?;
    let who = who();
    if who.cloud {
        eprintln!("{}", cloud_note(PHOTOS_SENT));
    }
    let aliases = store.aliases(PHOTO_ALIASES)?;
    let plan = plan(&result, &aliases, store.inventory().all(), args.place);
    let inv = store.inventory();

    println!("{} in the {}", args.file.display(), args.place.as_str());
    if plan.items.is_empty() {
        println!("nothing recognised in the photo");
    }
    for (i, p) in plan.items.iter().enumerate() {
        let s = &result.sightings[p.sighting];
        let mut name = if p.learned {
            format!("{} (learned)", p.item)
        } else {
            p.item.clone()
        };
        if p.learned || p.seen_as != p.item {
            name += &format!(" seen as \"{}\"", p.seen_as);
        }
        let text = s
            .text
            .as_deref()
            .map(|t| format!(" [{t}]"))
            .unwrap_or_default();
        let matched = if p.lots.is_empty() {
            "new".to_string()
        } else {
            p.lots
                .iter()
                .filter_map(|id| inv.get(*id))
                .map(|l| format!("= {} {} [{}]", l.item, l.left, l.id.short()))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("{:>3}  {name}{text}  x{}  {matched}", i + 1, p.count);
    }
    if !plan.missing.is_empty() {
        println!("\nNot in the photo (still there?):");
        for id in &plan.missing {
            if let Some(l) = inv.get(*id) {
                println!(
                    "  {}  {:<24} {:>7}",
                    l.id.short(),
                    l.item,
                    l.left.to_string()
                );
            }
        }
        println!("  -> nookery use|bin <id>  if it's gone, nookery seen <id>  if it's still there");
    }
    let per_tile = result
        .tiles
        .iter()
        .map(|t| format!("{:.1}s", t.ms / 1e3))
        .collect::<Vec<_>>()
        .join(" ");
    println!(
        "\n{}: prepare {:.0} ms, tiles {per_tile}, total {:.1} s",
        who.by,
        result.prepare_ms,
        result.total_ms / 1e3
    );
    if time {
        eprintln!("{} events in the ledger", store.event_count());
    }
    if !args.import {
        if !args.adds.is_empty() || !args.skips.is_empty() {
            eprintln!("(nothing written: add --import to apply --add/--skip)");
        }
        return Ok(());
    }

    let d = build_decisions(&plan, &args.adds, &args.skips, today)?;
    let (events, learned) = decide(&plan, &d.decisions, Utc::now())?;
    store.append_learning(&events, PHOTO_ALIASES, &learned, Utc::now())?;
    let mut seen = 0;
    for e in &events {
        let l = store.inventory().get(e.lot).expect("just written");
        let line = crate::line(l, nookery_core::status(l, today));
        match e.kind {
            nookery_core::EventKind::Seen { .. } => {
                seen += 1;
                println!("seen   {line}");
            }
            _ => println!("added  {line}"),
        }
    }
    println!(
        "marked {seen} seen, added {}, learned {} names",
        events.len() - seen,
        learned.len()
    );
    if !d.left_out.is_empty() {
        let list = d.left_out.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        println!(
            "left out {} unmatched (photo guesses are noisy): {}; add with --add N[=item,qty,bb]",
            d.left_out.len(),
            list.join(", ")
        );
    }
    Ok(())
}

pub fn stderr_is_terminal() -> bool {
    std::io::stderr().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Event, EventKind, LotId, Source};
    use nookery_vision::{ModelAnswer, RawItem};
    use std::sync::Mutex;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    fn add(s: &str) -> AddSpec {
        s.parse().unwrap()
    }

    #[test]
    fn add_values_parse_with_optional_fields() {
        assert_eq!(
            add("2"),
            AddSpec {
                sighting: 2,
                item: None,
                qty: None,
                best_before: None
            }
        );
        let a = add("3=pesto, 2, +5");
        assert_eq!(a.item.as_deref(), Some("pesto"));
        assert_eq!(a.qty, Some(Qty::pieces(2)));
        assert_eq!(a.best_before.as_deref(), Some("+5"));
        let a = add("1=,,+5");
        assert_eq!((a.item, a.qty), (None, None));
        assert_eq!(add("1=milk,1l").qty, Some(Qty::millilitres(1000)));
    }

    #[test]
    fn bad_add_values_are_rejected() {
        for bad in ["", "0", "x=milk", "1=milk,lots", "1=a,1,+1,x"] {
            assert!(bad.parse::<AddSpec>().is_err(), "{bad}");
        }
    }

    fn acquire(item: &str) -> Event {
        Event {
            at: "2026-11-01T10:00:00Z".parse().unwrap(),
            lot: LotId::new(),
            kind: EventKind::Acquired {
                item: item.into(),
                qty: Qty::pieces(1),
                location: Location::Fridge,
                source: Source::Manual,
                best_before: None,
            },
        }
    }

    /// Answers every tile with the same list.
    struct Fake(Mutex<Vec<RawItem>>);

    impl VisionModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn list_items(&self, _: &[u8], _: Location) -> Result<ModelAnswer, VisionError> {
            Ok(ModelAnswer {
                items: self.0.lock().unwrap().drain(..).collect(),
                tokens: 1,
                cut_on_repeat: false,
            })
        }
    }

    fn fake() -> Fake {
        let it = |n: &str, c| RawItem {
            name: n.into(),
            text: None,
            count: c,
        };
        Fake(Mutex::new(vec![
            it("skyr", 1),
            it("jar with green contents", 2),
            it("fish label", 1),
        ]))
    }

    fn photo(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("fridge.png");
        image::RgbImage::new(40, 80).save(&path).unwrap();
        path
    }

    fn fake_who() -> Who {
        Who {
            by: "fake".into(),
            cloud: false,
        }
    }

    fn args(file: PathBuf, import: bool, adds: &[&str], skips: &[usize]) -> PhotoArgs {
        PhotoArgs {
            file,
            place: Location::Fridge,
            import,
            adds: adds.iter().map(|a| add(a)).collect(),
            skips: skips.to_vec(),
            ai: AiOverride::default(),
        }
    }

    fn scanned_plan(store: &Store) -> Plan {
        let mut sc = nookery_vision::Scan {
            sightings: Vec::new(),
            model: "fake".into(),
            tiles: vec![],
            prepare_ms: 0.0,
            total_ms: 0.0,
        };
        for n in ["skyr", "jar with green contents", "fish label"] {
            sc.sightings.push(nookery_vision::Sighting {
                name: n.into(),
                text: None,
                count: 1,
                tiles: vec![0],
            });
        }
        plan(
            &sc,
            &Default::default(),
            store.inventory().all(),
            Location::Fridge,
        )
    }

    #[test]
    fn decisions_follow_defaults_and_flags() {
        let mut store = Store::open_in_memory().unwrap();
        store.append(&[acquire("skyr")]).unwrap();
        let p = scanned_plan(&store);
        let d = build_decisions(&p, &[], &[], today()).unwrap();
        assert_eq!(d.decisions, vec![(0, Decision::Seen)]);
        assert_eq!(d.left_out, vec![2, 3]);
        // --add on a matched sighting adds a second lot; --skip ignores.
        let d = build_decisions(&p, &[add("1"), add("2=pesto,2,+3")], &[3], today()).unwrap();
        assert_eq!(
            d.decisions[0],
            (
                0,
                Decision::Add {
                    item: "skyr".into(),
                    qty: Qty::pieces(1),
                    best_before: None
                }
            )
        );
        assert!(
            matches!(&d.decisions[1], (1, Decision::Add { item, best_before: Some(_), .. }) if item == "pesto")
        );
        assert_eq!(d.decisions[2], (2, Decision::Ignore));
        assert!(d.left_out.is_empty());
        // Skipping a matched one.
        let d = build_decisions(&p, &[], &[1], today()).unwrap();
        assert_eq!(d.decisions, vec![(0, Decision::Ignore)]);
    }

    #[test]
    fn bad_indexes_and_dates_are_errors() {
        let store = Store::open_in_memory().unwrap();
        let p = scanned_plan(&store);
        assert!(build_decisions(&p, &[add("4")], &[], today()).is_err());
        assert!(build_decisions(&p, &[], &[9], today()).is_err());
        assert!(build_decisions(&p, &[], &[0], today()).is_err());
        assert!(build_decisions(&p, &[add("1")], &[1], today()).is_err());
        assert!(build_decisions(&p, &[add("1"), add("1=x")], &[], today()).is_err());
        assert!(build_decisions(&p, &[add("1=x,,nonsense")], &[], today()).is_err());
    }

    #[test]
    fn whole_flow_with_a_fake_model() {
        let dir = tempfile::tempdir().unwrap();
        let file = photo(dir.path());
        let mut store = Store::open_in_memory().unwrap();
        store
            .append(&[acquire("skyr"), acquire("old ham")])
            .unwrap();
        let before = store.event_count();

        // Read-only: nothing written.
        run(
            &mut store,
            args(file.clone(), false, &["2=pesto"], &[]),
            &fake(),
            &fake_who,
            today(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(store.event_count(), before);

        // Import: skyr seen, jar added as pesto (learned), fish left out.
        // The fake answers once, on the first tile only.
        run(
            &mut store,
            args(file.clone(), true, &["2=pesto"], &[]),
            &fake(),
            &fake_who,
            today(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(store.event_count(), before + 2);
        assert!(store.inventory().all().iter().any(|l| l.item == "pesto"));
        let aliases = store.aliases(PHOTO_ALIASES).unwrap();
        assert_eq!(aliases.len(), 1);

        // A second scan now shows pesto as learned and matched.
        run(
            &mut store,
            args(file, true, &[], &[]),
            &fake(),
            &fake_who,
            today(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(store.event_count(), before + 4);
    }

    #[test]
    fn unreachable_model_is_a_clear_error() {
        struct Down;
        impl VisionModel for Down {
            fn name(&self) -> String {
                "down".into()
            }
            fn list_items(&self, _: &[u8], _: Location) -> Result<ModelAnswer, VisionError> {
                Err(VisionError::Unreachable("http://x".into()))
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open_in_memory().unwrap();
        let err = run(
            &mut store,
            args(photo(dir.path()), false, &[], &[]),
            &Down,
            &fake_who,
            today(),
            false,
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("start Ollama"), "{err}");
    }

    #[test]
    fn sign_in_points_at_the_photos_setting() {
        struct Out;
        impl VisionModel for Out {
            fn name(&self) -> String {
                "out".into()
            }
            fn list_items(&self, _: &[u8], _: Location) -> Result<ModelAnswer, VisionError> {
                Err(VisionError::SignIn)
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open_in_memory().unwrap();
        let err = run(
            &mut store,
            args(photo(dir.path()), false, &[], &[]),
            &Out,
            &fake_who,
            today(),
            false,
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("ollama signin"), "{err}");
        assert!(err.contains("nookery settings photos local"), "{err}");
    }
}
