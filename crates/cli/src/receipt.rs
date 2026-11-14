//! `nookery receipt FILE`: parse a receipt, show what the ledger would learn
//! from it, and (with `--import`) write it in one transaction.

use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;

use chrono::{NaiveDate, Utc};
use clap::Args;
use nookery_core::{parse_date, Location, Qty};
use nookery_extract as extract;
use nookery_extract::Method;
use nookery_receipt::{
    parse_text, propose, to_events, Chain, CheckError, Confirmed, Line, LineKind, Proposal, Receipt,
};
use nookery_store::{ReceiptRecord, Store, StoreError};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Args)]
pub struct ReceiptArgs {
    /// Receipt file (text, PDF, PNG, JPEG or HEIC photo), or - for stdin.
    pub file: PathBuf,
    /// Print the text read from the file and exit (for debugging parsing).
    #[arg(long)]
    pub print_text: bool,
    /// rewe, edeka, netto or denns (default: detected from the header).
    #[arg(long)]
    pub store: Option<Chain>,
    /// Write the lots (and learn aliases) instead of only showing them.
    #[arg(long)]
    pub import: bool,
    /// Set line N (as numbered in the table): N=item[,qty][,place][,bb].
    #[arg(long = "set", value_name = "N=ITEM[,QTY][,PLACE][,BB]")]
    pub sets: Vec<SetSpec>,
    /// Leave line N untracked (toilet paper, bags).
    #[arg(long = "skip", value_name = "N")]
    pub skips: Vec<usize>,
    /// Import even if the lines don't add up to the printed total.
    #[arg(long)]
    pub ignore_total: bool,
}

/// One `--set` value. `line` is 1-based, as printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetSpec {
    pub line: usize,
    pub item: String,
    pub qty: Option<Qty>,
    pub location: Option<Location>,
    pub best_before: Option<String>,
}

impl std::str::FromStr for SetSpec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (n, rest) = s
            .split_once('=')
            .ok_or_else(|| format!("{s:?}: expected N=item[,qty][,place][,bb]"))?;
        let line: usize = n
            .trim()
            .parse()
            .ok()
            .filter(|&n| n >= 1)
            .ok_or_else(|| format!("{s:?}: {n:?} is not a line number"))?;
        let mut parts = rest.split(',').map(str::trim);
        let item = parts.next().unwrap_or_default();
        if item.is_empty() {
            return Err(format!("{s:?}: the item name is missing"));
        }
        // Empty fields mean "default", so `3=milk,,,+5` sets only the date.
        let mut field = || parts.next().filter(|p| !p.is_empty());
        let qty = field()
            .map(|q| {
                q.parse::<Qty>()
                    .map_err(|_| format!("{s:?}: can't read the amount {q:?}"))
            })
            .transpose()?;
        let location = field()
            .map(|p| {
                p.parse::<Location>()
                    .map_err(|_| format!("{s:?}: {p:?} is not fridge, freezer or pantry"))
            })
            .transpose()?;
        let best_before = field().map(String::from);
        if parts.next().is_some() {
            return Err(format!("{s:?}: too many fields (item,qty,place,bb)"));
        }
        Ok(SetSpec {
            line,
            item: item.into(),
            qty,
            location,
            best_before,
        })
    }
}

/// Lines that need an answer before the receipt can be imported.
#[derive(Debug, PartialEq, Eq)]
pub struct Unresolved(pub Vec<usize>);

/// Proposals + `--set`s + `--skip`s -> what to import. Lines are 1-based here.
pub fn build_confirmed(
    receipt: &Receipt,
    proposals: &[Proposal],
    sets: &[SetSpec],
    skips: &[usize],
    today: NaiveDate,
) -> Res<Result<Vec<Confirmed>, Unresolved>> {
    let is_item = |n: usize| {
        receipt
            .lines
            .get(n.wrapping_sub(1))
            .is_some_and(|l| l.kind == LineKind::Item)
    };
    let mut touched = HashSet::new();
    for n in sets.iter().map(|s| s.line).chain(skips.iter().copied()) {
        if !is_item(n) {
            return Err(format!("line {n} is not an item line of this receipt").into());
        }
        if !touched.insert(n) {
            return Err(format!("line {n} is named more than once in --set/--skip").into());
        }
    }
    let mut out = Vec::new();
    let mut unresolved = Vec::new();
    for p in proposals {
        let n = p.line + 1;
        if skips.contains(&n) {
            continue;
        }
        let set = sets.iter().find(|s| s.line == n);
        let fallback_qty = p
            .qty
            .unwrap_or(Qty::pieces(receipt.lines[p.line].count as i64));
        let (item, qty, location, best_before) = match (set, &p.alias) {
            (Some(s), alias) => {
                let bb = s
                    .best_before
                    .as_deref()
                    .map(|b| parse_date(b, today).ok_or(format!("can't read the date {b:?}")))
                    .transpose()?;
                (
                    s.item.clone(),
                    s.qty.unwrap_or(fallback_qty),
                    s.location
                        .or(alias.as_ref().map(|a| a.location))
                        .unwrap_or(Location::Fridge),
                    bb,
                )
            }
            (None, Some(a)) => (a.item.clone(), fallback_qty, a.location, None),
            (None, None) => {
                unresolved.push(n);
                continue;
            }
        };
        out.push(Confirmed {
            line: p.line,
            item,
            qty,
            location,
            best_before,
        });
    }
    Ok(if unresolved.is_empty() {
        Ok(out)
    } else {
        Err(Unresolved(unresolved))
    })
}

fn euros(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}{},{:02}", cents.abs() / 100, cents.abs() % 100)
}

fn marker(k: LineKind) -> &'static str {
    match k {
        LineKind::Item => "item",
        LineKind::Deposit => "Pfand",
        LineKind::DepositReturn => "Leergut",
        LineKind::Discount => "discount",
    }
}

fn amount_note(l: &Line) -> String {
    match l.weight {
        Some(w) => w.to_string(),
        None if l.count > 1 => format!("{}x", l.count),
        None => String::new(),
    }
}

fn proposal_note(p: &Proposal) -> String {
    match &p.alias {
        Some(a) => {
            let qty = p.qty.map(|q| format!(" {q}")).unwrap_or_default();
            format!("-> {}{qty}, {}", a.item, a.location.as_str())
        }
        None => "-> ?".into(),
    }
}

/// What was read from the file, and a one-line note on how.
fn read_input(file: &PathBuf) -> Res<(String, Option<String>)> {
    let bytes = if file.as_os_str() == "-" {
        let mut b = Vec::new();
        std::io::stdin().read_to_end(&mut b)?;
        b
    } else {
        std::fs::read(file).map_err(|e| format!("can't read {}: {e}", file.display()))?
    };
    read_bytes(&bytes)
}

/// Detects the kind by content (never the extension) and extracts the text.
fn read_bytes(bytes: &[u8]) -> Res<(String, Option<String>)> {
    if extract::kind_of(bytes).is_none() {
        return Err("not a text, PDF, PNG, JPEG or HEIC file".into());
    }
    let got = extract::extract(bytes)?;
    let note = match got.method {
        Method::Text => None,
        Method::PdfText => Some(format!("read from PDF text ({} ms)", got.ms)),
        Method::Ocr => Some(format!(
            "read from photo (OCR, {} ms): check the lines",
            got.ms
        )),
    };
    Ok((got.text, note))
}

pub fn run(store: &mut Store, args: ReceiptArgs, today: NaiveDate, time: bool) -> Res<()> {
    let (text, note) = read_input(&args.file)?;
    if args.print_text {
        if let Some(n) = &note {
            eprintln!("{n}");
        }
        print!("{text}");
        if !text.ends_with('\n') {
            println!();
        }
        return Ok(());
    }
    if let Some(n) = &note {
        println!("{n}");
    }
    let t = Instant::now();
    let receipt = parse_text(&text, args.store)?;
    let parse_time = t.elapsed();
    if time {
        eprintln!(
            "parse {} lines: {:.2} ms",
            receipt.lines.len(),
            parse_time.as_secs_f64() * 1e3
        );
    }

    let aliases = match receipt.chain {
        Some(c) => store.aliases(c.name())?,
        None => Default::default(),
    };
    let proposals = propose(&receipt, |k| aliases.get(k).cloned());
    let by_line: BTreeMap<usize, &Proposal> = proposals.iter().map(|p| (p.line, p)).collect();
    let id = receipt.id();
    let already = store.has_receipt(&id)?;

    match receipt.chain {
        Some(c) => println!("store: {}", c.name()),
        None => println!("store unknown: pass --store"),
    }
    match receipt.date {
        Some(d) => println!("date:  {d}"),
        None => println!("date:  unknown"),
    }
    for (i, l) in receipt.lines.iter().enumerate() {
        let note = by_line
            .get(&i)
            .map(|p| proposal_note(p))
            .unwrap_or_default();
        println!(
            "{:>3}  {:<8} {:<30} {:>9} {:>8}  {}",
            i + 1,
            marker(l.kind),
            l.text,
            amount_note(l),
            euros(l.amount_cents),
            note
        )
    }
    let check = receipt.check_total();
    match &check {
        Ok(()) => println!(
            "lines {} = total {} ✓",
            euros(receipt.lines_cents()),
            euros(receipt.total_cents.unwrap_or_default())
        ),
        Err(CheckError::NoTotal) => println!(
            "lines {}, but no total was found on the receipt",
            euros(receipt.lines_cents())
        ),
        Err(CheckError::Mismatch {
            lines_cents,
            total_cents,
        }) => println!(
            "MISMATCH: lines {} != total {} (difference {})",
            euros(*lines_cents),
            euros(*total_cents),
            euros(lines_cents - total_cents)
        ),
    }
    if already {
        println!("already imported (receipt {id})");
    }
    if !args.import {
        return Ok(());
    }

    let chain = receipt
        .chain
        .ok_or("store unknown: pass --store rewe|edeka|netto|denns")?;
    if already {
        return Err(StoreError::DuplicateReceipt(id).into());
    }
    if let Err(e) = check {
        if !args.ignore_total {
            return Err(format!("{e}; fix the receipt text or pass --ignore-total").into());
        }
        eprintln!("warning: {e}; importing anyway");
    }
    let confirmed = match build_confirmed(&receipt, &proposals, &args.sets, &args.skips, today)? {
        Ok(c) => c,
        Err(Unresolved(lines)) => {
            let mut msg = String::from("these lines need an answer (--set or --skip):");
            for n in lines {
                let l = &receipt.lines[n - 1];
                let qty = by_line[&(n - 1)].qty.unwrap_or(Qty::pieces(1));
                msg += &format!(
                    "\n  {n}  {}   e.g. --set '{n}=<item>,{qty},fridge'   or --skip {n}",
                    l.text
                );
            }
            return Err(msg.into());
        }
    };
    let now = Utc::now();
    let import = to_events(&receipt, &confirmed, now)?;
    let record = ReceiptRecord {
        id,
        store: chain.name().into(),
        date: receipt.date,
        total_cents: receipt.total_cents,
        text,
    };
    store.import_receipt(&record, &import.events, &import.learned, now)?;
    for e in &import.events {
        let l = store.inventory().get(e.lot).expect("just imported");
        println!("{}", crate::line(l, nookery_core::status(l, today)));
    }
    println!(
        "imported {} lots; learned {} aliases",
        import.events.len(),
        import.learned.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::Alias;

    const REWE: &str = include_str!("../../receipt/tests/fixtures/rewe.txt");

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
    }

    fn set(s: &str) -> SetSpec {
        s.parse().unwrap()
    }

    #[test]
    fn set_values_parse_with_optional_fields() {
        assert_eq!(
            set("2=milk"),
            SetSpec {
                line: 2,
                item: "milk".into(),
                qty: None,
                location: None,
                best_before: None
            }
        );
        let s = set("3=eggs, 6, pantry, +5");
        assert_eq!(s.qty, Some(Qty::pieces(6)));
        assert_eq!(s.location, Some(Location::Pantry));
        assert_eq!(s.best_before.as_deref(), Some("+5"));
        let s = set("3=milk,,,+5");
        assert_eq!((s.qty, s.location), (None, None));
        assert_eq!(s.best_before.as_deref(), Some("+5"));
        assert_eq!(set("1=milk,1l").qty, Some(Qty::millilitres(1000)));
    }

    #[test]
    fn bad_set_values_are_rejected() {
        for bad in [
            "milk",
            "0=milk",
            "x=milk",
            "1=",
            "1=milk,lots",
            "1=milk,1,shed",
            "1=a,1,fridge,+1,x",
        ] {
            assert!(bad.parse::<SetSpec>().is_err(), "{bad}");
        }
    }

    fn rewe() -> (Receipt, Vec<Proposal>) {
        let r = parse_text(REWE, None).unwrap();
        let milk = Alias {
            item: "milk".into(),
            each: Some(Qty::millilitres(1000)),
            location: Location::Fridge,
        };
        let key = r.lines[0].key.clone();
        let p = propose(&r, |k| (k == key).then(|| milk.clone()));
        (r, p)
    }

    #[test]
    fn unknown_lines_are_listed_not_guessed() {
        let (r, p) = rewe();
        let n_items = p.len();
        let got = build_confirmed(&r, &p, &[], &[], today()).unwrap();
        let Err(Unresolved(lines)) = got else {
            panic!("expected unresolved")
        };
        assert_eq!(lines.len(), n_items - 1);
        assert!(!lines.contains(&1));
    }

    #[test]
    fn sets_and_skips_resolve_everything() {
        let (r, p) = rewe();
        let item_lines: Vec<usize> = p.iter().map(|p| p.line + 1).collect();
        let sets = vec![
            set(&format!("{}=yogurt,500g,fridge,+4", item_lines[1])),
            set(&format!("{}=milk,2l", item_lines[0])),
        ];
        let skips: Vec<usize> = item_lines[2..].to_vec();
        let c = build_confirmed(&r, &p, &sets, &skips, today())
            .unwrap()
            .unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(
            (c[0].item.as_str(), c[0].qty),
            ("milk", Qty::millilitres(2000))
        );
        assert_eq!(c[1].item, "yogurt");
        assert_eq!(c[1].best_before, NaiveDate::from_ymd_opt(2026, 11, 12));
    }

    #[test]
    fn alias_defaults_apply_and_bad_targets_error() {
        let (r, p) = rewe();
        // Alias line imported untouched: proposal qty and alias place.
        let skips: Vec<usize> = p[1..].iter().map(|p| p.line + 1).collect();
        let c = build_confirmed(&r, &p, &[], &skips, today())
            .unwrap()
            .unwrap();
        assert_eq!(c[0].qty, Qty::millilitres(1000));
        assert_eq!(c[0].location, Location::Fridge);
        // A deposit line, an unknown line, and a double mention.
        let deposit = r
            .lines
            .iter()
            .position(|l| l.kind == LineKind::Deposit)
            .unwrap()
            + 1;
        assert!(build_confirmed(&r, &p, &[], &[deposit], today()).is_err());
        assert!(build_confirmed(&r, &p, &[], &[99], today()).is_err());
        assert!(build_confirmed(&r, &p, &[set("1=milk")], &[1], today()).is_err());
        assert!(build_confirmed(&r, &p, &[set("1=milk,,,nonsense")], &[], today()).is_err());
    }

    #[test]
    fn text_passes_through_and_garbage_is_refused() {
        let (t, note) = read_bytes(REWE.as_bytes()).unwrap();
        assert!(t.contains("REWE") && note.is_none());
        assert!(read_bytes(&[0, 1, 2, 255, 254]).is_err());
    }

    #[test]
    fn pdf_text_is_noted() {
        let pdf = include_bytes!("../../extract/tests/fixtures/rewe-text.pdf");
        let (t, note) = read_bytes(pdf).unwrap();
        assert!(parse_text(&t, None).is_ok());
        assert!(note.unwrap().starts_with("read from PDF text ("));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn photo_is_noted_as_ocr() {
        let png = include_bytes!("../../extract/tests/fixtures/rewe-clean.png");
        let (t, note) = read_bytes(png).unwrap();
        assert!(!t.is_empty());
        assert!(note.unwrap().contains("(OCR,"));
    }

    #[test]
    fn euros_format_like_the_receipt() {
        assert_eq!(euros(2345), "23,45");
        assert_eq!(euros(-5), "-0,05");
        assert_eq!(euros(0), "0,00");
    }
}
