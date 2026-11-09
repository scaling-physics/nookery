//! Store receipts in, ledger events out. Pure: no I/O, no clock.
//!
//! Text (from a digital receipt, or OCR of a paper one) is parsed into
//! lines with exact cent amounts; the lines must add up to the printed
//! total. Each food line is then matched against the aliases learned from
//! earlier confirmations, and what the user confirms becomes `Acquired`
//! events (plus new aliases, so the next receipt asks less).
//!
//! Store chains: REWE, EDEKA, Netto, denn's Biomarkt. Their exact layouts
//! are tuned against real receipts in the git-ignored `fixtures/private/`;
//! the fixtures in `tests/fixtures/` are SYNTHETIC.

mod money;
mod parse;

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use nookery_core::{Alias, Event, EventKind, Location, LotId, Qty, Source};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Rewe,
    Edeka,
    Netto,
    Denns,
}

impl Chain {
    pub const ALL: [Chain; 4] = [Chain::Rewe, Chain::Edeka, Chain::Netto, Chain::Denns];

    /// The store name stored on events and aliases ("REWE", "EDEKA",
    /// "Netto", "denn's").
    pub fn name(&self) -> &'static str {
        match self {
            Chain::Rewe => "REWE",
            Chain::Edeka => "EDEKA",
            Chain::Netto => "Netto",
            Chain::Denns => "denn's",
        }
    }

    /// Recognises the chain from the receipt header.
    pub fn detect(text: &str) -> Option<Chain> {
        let header: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .take(8)
            // A VAT table ("Netto  Steuer  Brutto") is not a header.
            .filter(|l| {
                let up = l.to_uppercase();
                !(up.contains("STEUER") || up.contains("BRUTTO"))
            })
            .map(|l| l.to_uppercase().replace(['’', '`', '´'], "'"))
            .collect();
        let has = |words: &[&str]| header.iter().any(|l| words.iter().any(|w| l.contains(w)));
        if has(&["REWE"]) {
            Some(Chain::Rewe)
        } else if has(&["EDEKA", "E CENTER", "E-CENTER"]) {
            Some(Chain::Edeka)
        } else if has(&["DENN'S", "DENNS"]) {
            Some(Chain::Denns)
        } else if has(&["NETTO", "MARKEN-DISCOUNT"]) {
            Some(Chain::Netto)
        } else {
            None
        }
    }
}

impl std::str::FromStr for Chain {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let t = s.trim().to_lowercase().replace(['\'', '’'], "");
        match t.as_str() {
            "rewe" => Ok(Chain::Rewe),
            "edeka" => Ok(Chain::Edeka),
            "netto" => Ok(Chain::Netto),
            "denns" | "denns biomarkt" => Ok(Chain::Denns),
            _ => Err(format!("unknown store {s:?} (rewe, edeka, netto, denns)")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    /// Something bought (food or not; the user decides what to track).
    Item,
    /// Pfand charged on a bottle or crate.
    Deposit,
    /// Pfand returned (Leergut, Pfandbon): negative.
    DepositReturn,
    /// A discount or coupon, negative, belonging to the item line above it.
    Discount,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// As printed, e.g. "BIO H-MILCH 1,5%".
    pub text: String,
    /// `alias_key(text)`: what aliases are looked up by.
    pub key: String,
    pub kind: LineKind,
    /// Multi-buy count ("2 Stk x 0,99"), else 1.
    pub count: u32,
    /// Weighed goods ("0,512 kg x 2,99 EUR/kg").
    pub weight: Option<Qty>,
    /// Price per piece or per kg when printed.
    pub unit_price_cents: Option<i64>,
    /// The line's amount, signed (discounts and returns negative).
    pub amount_cents: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub chain: Option<Chain>,
    pub date: Option<NaiveDate>,
    pub lines: Vec<Line>,
    /// The printed total ("SUMME EUR 23,45").
    pub total_cents: Option<i64>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("no receipt lines found")]
    NoLines,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CheckError {
    #[error("no total found on the receipt")]
    NoTotal,
    #[error("lines add up to {} but the total says {}", euros(*lines_cents), euros(*total_cents))]
    Mismatch { lines_cents: i64, total_cents: i64 },
}

/// `977` -> `"9,77 €"`, as German receipts print it.
fn euros(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let c = cents.unsigned_abs();
    format!("{sign}{},{:02} €", c / 100, c % 100)
}

/// Parses receipt text. `chain` overrides detection.
pub fn parse_text(text: &str, chain: Option<Chain>) -> Result<Receipt, ParseError> {
    let chain = chain.or_else(|| Chain::detect(text));
    let parsed = parse::parse_lines(text, chain);
    if parsed.lines.is_empty() {
        return Err(ParseError::NoLines);
    }
    Ok(Receipt {
        chain,
        date: parsed.date,
        lines: parsed.lines,
        total_cents: parsed.total_cents,
    })
}

impl Receipt {
    /// The sum of all lines, in cents.
    pub fn lines_cents(&self) -> i64 {
        self.lines.iter().map(|l| l.amount_cents).sum()
    }

    /// The lines must add up to the printed total, to the cent.
    pub fn check_total(&self) -> Result<(), CheckError> {
        let total_cents = self.total_cents.ok_or(CheckError::NoTotal)?;
        let lines_cents = self.lines_cents();
        if lines_cents == total_cents {
            Ok(())
        } else {
            Err(CheckError::Mismatch {
                lines_cents,
                total_cents,
            })
        }
    }

    /// Stable id: the same receipt (chain, date, lines, total) always gives
    /// the same id, so importing it twice is caught. 16 hex characters.
    pub fn id(&self) -> String {
        use std::fmt::Write;
        let mut canon = format!(
            "{}|{}|{}",
            self.chain.map_or("", |c| c.name()),
            self.date.map(|d| d.to_string()).unwrap_or_default(),
            self.total_cents.map(|t| t.to_string()).unwrap_or_default(),
        );
        for l in &self.lines {
            let _ = write!(
                canon,
                "\n{}\t{:?}\t{}\t{}",
                l.text, l.kind, l.count, l.amount_cents
            );
        }
        format!("{:016x}", fnv1a(canon.as_bytes()))
    }
}

/// The lookup key for a printed line: upper case, whitespace collapsed, a
/// trailing VAT letter ("A"/"B") and amounts removed.
pub fn alias_key(text: &str) -> String {
    // Only a real price column is stripped, so "1,5%" stays and so does a
    // name that happens to end in "B".
    let name = money::split_price(text).map_or(text, |(name, _)| name);
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

/// FNV-1a 64: tiny, and stable across runs and platforms (unlike the std
/// hasher), which the receipt id needs.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The item's amount plus the discounts printed directly below it.
fn price_with_discounts(receipt: &Receipt, i: usize) -> i64 {
    let discounts = receipt.lines[i + 1..]
        .iter()
        .take_while(|l| l.kind == LineKind::Discount)
        .map(|l| l.amount_cents)
        .sum::<i64>();
    receipt.lines[i].amount_cents + discounts
}

/// A guess for one `Item` line, for the user to confirm or correct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// Index into `Receipt::lines`.
    pub line: usize,
    /// The learned alias, if this text was confirmed before.
    pub alias: Option<Alias>,
    /// The amount, when known: weight, or count x `alias.each`, or count
    /// pieces when there is no alias.
    pub qty: Option<Qty>,
    /// The item's price with its discounts applied, in cents.
    pub price_cents: i64,
}

/// One proposal per `Item` line, in receipt order.
pub fn propose(receipt: &Receipt, lookup: impl Fn(&str) -> Option<Alias>) -> Vec<Proposal> {
    let mut out = Vec::new();
    for (i, l) in receipt.lines.iter().enumerate() {
        if l.kind != LineKind::Item {
            continue;
        }
        let alias = lookup(&l.key);
        let qty = match (l.weight, alias.as_ref().and_then(|a| a.each)) {
            (Some(w), _) => Qty {
                milli: w.milli,
                unit: w.unit,
            },
            (None, Some(each)) => Qty {
                milli: each.milli * l.count as i64,
                unit: each.unit,
            },
            (None, None) => Qty::pieces(l.count as i64),
        };
        out.push(Proposal {
            line: i,
            alias,
            qty: Some(qty),
            price_cents: price_with_discounts(receipt, i),
        });
    }
    out
}

/// What the user confirmed for one line; lines left out are not tracked
/// (toilet paper, deposits).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confirmed {
    pub line: usize,
    pub item: String,
    pub qty: Qty,
    pub location: Location,
    pub best_before: Option<NaiveDate>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("which store is this receipt from?")]
    NoChain,
    #[error("line {0} is not on the receipt or is not an item")]
    BadLine(usize),
    #[error("line {0} is confirmed twice")]
    Duplicate(usize),
}

/// What importing a receipt writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    /// One `Acquired` per confirmed line; source `Receipt { store:
    /// chain.name(), line: text, price_cents, receipt: Some(id) }`, in the
    /// confirmed order.
    pub events: Vec<Event>,
    /// `(Line::key, Alias)` to remember for this chain.
    pub learned: Vec<(String, Alias)>,
}

/// The events and aliases for the confirmed lines.
pub fn to_events(
    receipt: &Receipt,
    confirmed: &[Confirmed],
    at: DateTime<Utc>,
) -> Result<Import, ImportError> {
    let chain = receipt.chain.ok_or(ImportError::NoChain)?;
    let id = receipt.id();
    let mut seen = HashSet::new();
    let mut import = Import {
        events: Vec::new(),
        learned: Vec::new(),
    };
    for c in confirmed {
        let line = match receipt.lines.get(c.line) {
            Some(l) if l.kind == LineKind::Item => l,
            _ => return Err(ImportError::BadLine(c.line)),
        };
        if !seen.insert(c.line) {
            return Err(ImportError::Duplicate(c.line));
        }
        import.events.push(Event {
            at,
            lot: LotId::new(),
            kind: EventKind::Acquired {
                item: c.item.clone(),
                qty: c.qty,
                location: c.location,
                source: Source::Receipt {
                    store: chain.name().into(),
                    line: Some(line.text.clone()),
                    price_cents: Some(price_with_discounts(receipt, c.line)),
                    receipt: Some(id.clone()),
                },
                best_before: c.best_before,
            },
        });
        // A weighed line has no fixed unit size; otherwise one bought unit
        // is the confirmed amount divided by the multi-buy count.
        let each = line.weight.is_none().then(|| Qty {
            milli: c.qty.milli / line.count.max(1) as i64,
            unit: c.qty.unit,
        });
        import.learned.push((
            line.key.clone(),
            Alias {
                item: c.item.clone(),
                each,
                location: c.location,
            },
        ));
    }
    Ok(import)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, kind: LineKind, amount: i64) -> Line {
        Line {
            text: text.into(),
            key: alias_key(text),
            kind,
            count: 1,
            weight: None,
            unit_price_cents: None,
            amount_cents: amount,
        }
    }

    fn receipt(chain: Option<Chain>, lines: Vec<Line>, total: Option<i64>) -> Receipt {
        Receipt {
            chain,
            date: NaiveDate::from_ymd_opt(2026, 11, 8),
            lines,
            total_cents: total,
        }
    }

    fn at() -> DateTime<Utc> {
        "2026-11-08T14:32:00Z".parse().unwrap()
    }

    fn confirm(line: usize, item: &str, qty: Qty) -> Confirmed {
        Confirmed {
            line,
            item: item.into(),
            qty,
            location: Location::Fridge,
            best_before: None,
        }
    }

    #[test]
    fn alias_keys_drop_only_the_price_column() {
        assert_eq!(alias_key("Bio H-Milch 1,5%   1,19 B"), "BIO H-MILCH 1,5%");
        assert_eq!(alias_key("  bio  h-milch 1,5% "), "BIO H-MILCH 1,5%");
        assert_eq!(alias_key("Äpfel Braeburn 2,49 *A"), "ÄPFEL BRAEBURN");
        assert_eq!(alias_key("VITAMIN B"), "VITAMIN B");
    }

    #[test]
    fn detect_reads_the_header_and_ignores_a_vat_table() {
        assert_eq!(Chain::detect("REWE Markt GmbH\nKoeln"), Some(Chain::Rewe));
        assert_eq!(Chain::detect("E center Muster\nStr 1"), Some(Chain::Edeka));
        assert_eq!(Chain::detect("denn’s Biomarkt"), Some(Chain::Denns));
        assert_eq!(Chain::detect("DENNS BIOMARKT"), Some(Chain::Denns));
        assert_eq!(Chain::detect("Netto Marken-Discount"), Some(Chain::Netto));
        let unknown = "Kiosk\nA 1\nB 2\nC 3\nD 4\nE 5\nF 6\nG 7\nNetto Steuer Brutto";
        assert_eq!(Chain::detect(unknown), None);
        assert_eq!(Chain::detect("Kiosk\nNetto  Steuer  Brutto"), None);
    }

    #[test]
    fn check_total_reports_missing_and_mismatching_totals() {
        let l = vec![
            line("A", LineKind::Item, 100),
            line("B", LineKind::Item, 50),
        ];
        assert_eq!(
            receipt(None, l.clone(), None).check_total(),
            Err(CheckError::NoTotal)
        );
        assert_eq!(receipt(None, l.clone(), Some(150)).check_total(), Ok(()));
        assert_eq!(
            receipt(None, l, Some(140)).check_total(),
            Err(CheckError::Mismatch {
                lines_cents: 150,
                total_cents: 140
            })
        );
    }

    #[test]
    fn ids_are_sixteen_hex_chars_and_depend_on_content() {
        let a = receipt(
            Some(Chain::Rewe),
            vec![line("A", LineKind::Item, 100)],
            Some(100),
        );
        let b = receipt(
            Some(Chain::Rewe),
            vec![line("A", LineKind::Item, 101)],
            Some(101),
        );
        assert_eq!(a.id().len(), 16);
        assert!(a
            .id()
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(a.id(), a.clone().id());
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn fnv1a_matches_the_reference_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn proposals_apply_discounts_and_use_known_unit_sizes() {
        let mut milk = line("MILCH", LineKind::Item, 198);
        milk.count = 2;
        let mut bananas = line("BANANEN", LineKind::Item, 153);
        bananas.weight = Some(Qty::grams(512));
        let r = receipt(
            Some(Chain::Rewe),
            vec![
                milk,
                line("RABATT", LineKind::Discount, -30),
                bananas,
                line("PFAND", LineKind::Deposit, 25),
                line("BROT", LineKind::Item, 229),
            ],
            None,
        );
        let alias = Alias {
            item: "milk".into(),
            each: Some(Qty::millilitres(1000)),
            location: Location::Fridge,
        };
        let p = propose(&r, |k| (k == "MILCH").then(|| alias.clone()));
        assert_eq!(p.iter().map(|p| p.line).collect::<Vec<_>>(), [0, 2, 4]);
        assert_eq!(p[0].price_cents, 168);
        assert_eq!(p[0].qty, Some(Qty::millilitres(2000)));
        assert_eq!(p[0].alias.as_ref(), Some(&alias));
        assert_eq!(p[1].qty, Some(Qty::grams(512)));
        assert_eq!(p[2].qty, Some(Qty::pieces(1)));
        assert_eq!(p[2].alias, None);
    }

    #[test]
    fn importing_needs_a_chain_and_valid_unique_item_lines() {
        let lines = vec![
            line("A", LineKind::Item, 100),
            line("PFAND", LineKind::Deposit, 25),
        ];
        let c = |i| confirm(i, "a", Qty::pieces(1));
        let no_chain = receipt(None, lines.clone(), None);
        assert_eq!(
            to_events(&no_chain, &[c(0)], at()).unwrap_err(),
            ImportError::NoChain
        );
        let r = receipt(Some(Chain::Netto), lines, None);
        assert_eq!(
            to_events(&r, &[c(1)], at()).unwrap_err(),
            ImportError::BadLine(1)
        );
        assert_eq!(
            to_events(&r, &[c(5)], at()).unwrap_err(),
            ImportError::BadLine(5)
        );
        assert_eq!(
            to_events(&r, &[c(0), c(0)], at()).unwrap_err(),
            ImportError::Duplicate(0)
        );
    }

    #[test]
    fn importing_writes_acquired_events_and_learns_unit_sizes() {
        let mut eggs = line("BIO EIER", LineKind::Item, 298);
        eggs.count = 2;
        let mut bananas = line("BANANEN", LineKind::Item, 153);
        bananas.weight = Some(Qty::grams(512));
        let r = receipt(
            Some(Chain::Denns),
            vec![eggs, line("COUPON", LineKind::Discount, -50), bananas],
            None,
        );
        let out = to_events(
            &r,
            &[
                confirm(2, "bananas", Qty::grams(512)),
                confirm(0, "eggs", Qty::pieces(12)),
            ],
            at(),
        )
        .unwrap();
        assert_eq!(out.events.len(), 2);
        match &out.events[1].kind {
            EventKind::Acquired {
                item, qty, source, ..
            } => {
                assert_eq!((item.as_str(), *qty), ("eggs", Qty::pieces(12)));
                assert_eq!(
                    source,
                    &Source::Receipt {
                        store: "denn's".into(),
                        line: Some("BIO EIER".into()),
                        price_cents: Some(248),
                        receipt: Some(r.id()),
                    }
                );
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(out.learned[0].0, "BANANEN");
        assert_eq!(out.learned[0].1.each, None);
        assert_eq!(out.learned[1].0, "BIO EIER");
        assert_eq!(out.learned[1].1.each, Some(Qty::pieces(6)));
    }

    #[test]
    fn mismatches_are_explained_in_euros() {
        let e = CheckError::Mismatch {
            lines_cents: 977,
            total_cents: -5,
        };
        assert_eq!(
            e.to_string(),
            "lines add up to 9,77 € but the total says -0,05 €"
        );
    }
}
