//! Line-by-line parsing of receipt text. One generic parser; the chains
//! differ in where the count/weight line sits (see `Cont`), and that is
//! handled by looking at the amounts rather than by chain.

use chrono::NaiveDate;
use nookery_core::Qty;

use crate::money::{parse_cents, split_price};
use crate::{alias_key, Chain, Line, LineKind};

/// Lines that start the sum; everything after is footer.
const TOTAL_WORDS: [&str; 4] = ["SUMME", "ZU ZAHLEN", "GESAMT", "TOTAL"];
/// Payment words that end the item list when a till prints no SUMME.
const PAYMENT_WORDS: [&str; 7] = [
    "BAR",
    "EC-CASH",
    "KARTE",
    "GEGEBEN",
    "RÜCKGELD",
    "GIROCARD",
    "VISA",
];

/// A "count x unit price" or "weight x price per kg" line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cont {
    Multi { count: u32, unit_cents: i64 },
    Weight { grams: i64, per_kg_cents: i64 },
}

impl Cont {
    /// What the item line's amount should be if this line belongs to it.
    fn expected_cents(&self) -> i64 {
        match *self {
            Cont::Multi { count, unit_cents } => count as i64 * unit_cents,
            // grams * cents/kg / 1000, rounded half up.
            Cont::Weight {
                grams,
                per_kg_cents,
            } => (grams * per_kg_cents + 500) / 1000,
        }
    }

    fn apply(&self, line: &mut Line) {
        match *self {
            Cont::Multi { count, unit_cents } => {
                line.count = count;
                line.unit_price_cents = Some(unit_cents);
            }
            Cont::Weight {
                grams,
                per_kg_cents,
            } => {
                line.weight = Some(Qty::grams(grams));
                line.unit_price_cents = Some(per_kg_cents);
            }
        }
    }
}

/// A unit tail after the price: `EUR/kg`, `FUR/kg` (OCR), `€/kg`, `/kg`, `EUR`.
fn is_unit_tail(tok: &str) -> bool {
    match tok.strip_suffix("/kg") {
        Some(pre) => pre.chars().count() <= 3 && !pre.contains(|c: char| c.is_ascii_digit()),
        None => matches!(tok, "eur" | "fur" | "€"),
    }
}

/// `2 Stk x 0,99`, `2 St x 0,99`, `2 x 0,99`, `2x 0,99`,
/// `0,512 kg x 2,99 EUR/kg`. OCR variants: `X`, `×`, `*` for the x, a
/// decimal point in the weight or price, `FUR/kg`, and (weight lines only)
/// one misread character instead of the x: `0,512 kg ? 2,99`.
pub(crate) fn parse_cont(line: &str) -> Option<Cont> {
    let lower = line.to_lowercase();
    let mut toks: Vec<&str> = lower.split_whitespace().collect();
    while toks.last().is_some_and(|t| is_unit_tail(t)) {
        toks.pop();
    }
    let unit_cents = parse_cents(toks.pop()?)?;
    let is_x = |t: &str| matches!(t, "x" | "×" | "*");
    match toks.as_slice() {
        [w, "kg", op] if is_x(op) || is_misread_x(op) => {
            let (kg, frac) = w.split_once([',', '.']).unwrap_or((w, ""));
            if frac.len() > 3 || !kg.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
                return None;
            }
            let frac = format!("{frac:0<3}");
            let grams = kg.parse::<i64>().ok()? * 1000 + frac.parse::<i64>().ok()?;
            (grams > 0).then_some(Cont::Weight {
                grams,
                per_kg_cents: unit_cents,
            })
        }
        [n, op] if is_x(op) => multi(n, unit_cents),
        [n, "stk" | "stk." | "st" | "st." | "stück", op] if is_x(op) => multi(n, unit_cents),
        [n] => multi(n.strip_suffix(['x', '×', '*'])?, unit_cents),
        _ => None,
    }
}

/// One stray character where the multiplication sign should be.
fn is_misread_x(tok: &str) -> bool {
    tok.chars().count() == 1 && !tok.contains(|c: char| c.is_ascii_digit() || c == '-')
}

fn multi(n: &str, unit_cents: i64) -> Option<Cont> {
    let count: u32 = n.parse().ok()?;
    (1..=999)
        .contains(&count)
        .then_some(Cont::Multi { count, unit_cents })
}

fn classify(text: &str, cents: i64) -> (LineKind, i64) {
    let up = text.to_uppercase();
    let returned = up.contains("LEERGUT")
        || up.contains("PFANDBON")
        || (up.contains("PFAND")
            && (up.contains("RÜCKGABE") || up.contains("RUECKGABE") || up.contains("RUCKGABE")));
    if returned {
        (LineKind::DepositReturn, -cents.abs())
    } else if up.contains("PFAND") {
        let kind = if cents < 0 {
            LineKind::DepositReturn
        } else {
            LineKind::Deposit
        };
        (kind, cents)
    } else if cents < 0 {
        // RABATT, PREISVORTEIL, COUPON, AKTION, -20%: any negative amount.
        (LineKind::Discount, cents)
    } else {
        (LineKind::Item, cents)
    }
}

/// True when the text is `word` or starts with it as a whole word
/// ("BAR" matches "BAR 20,00" but not "BARILLA").
fn starts_with_word(text: &str, words: &[&str]) -> bool {
    let up = text.trim().to_uppercase();
    words.iter().any(|w| {
        up.strip_prefix(w)
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric()))
    })
}

pub(crate) struct Parsed {
    pub lines: Vec<Line>,
    pub total_cents: Option<i64>,
    pub date: Option<NaiveDate>,
}

/// Assumptions: an item is `text <price> [A|B]`. A count/weight line
/// belongs to the item above it when its product matches that item's
/// amount (REWE, denn's style), otherwise to the next item (Netto, EDEKA
/// style). Everything after SUMME is footer.
pub(crate) fn parse_lines(text: &str, _chain: Option<Chain>) -> Parsed {
    let mut lines: Vec<Line> = Vec::new();
    let mut total = None;
    let mut pending: Option<Cont> = None;
    let mut open_item: Option<usize> = None;
    let mut done = false; // reached SUMME or the payment block
    let mut date = None;

    for raw in text.lines() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        if date.is_none() {
            date = find_date(raw);
        }
        if done {
            continue;
        }
        if let Some(cont) = parse_cont(raw) {
            let below = open_item.filter(|&i| lines[i].amount_cents == cont.expected_cents());
            match below {
                Some(i) => cont.apply(&mut lines[i]),
                None => pending = Some(cont),
            }
            open_item = None;
            continue;
        }
        let Some((label, cents)) = split_price(raw) else {
            done = starts_with_word(raw, &PAYMENT_WORDS) && !lines.is_empty();
            continue;
        };
        if label.is_empty() {
            continue;
        }
        if starts_with_word(label, &TOTAL_WORDS) {
            total = Some(cents);
            done = true;
            continue;
        }
        if starts_with_word(label, &PAYMENT_WORDS) {
            done = true;
            continue;
        }
        let (kind, amount_cents) = classify(label, cents);
        let mut line = Line {
            text: label.to_string(),
            key: alias_key(label),
            kind,
            count: 1,
            weight: None,
            unit_price_cents: None,
            amount_cents,
        };
        open_item = None;
        if kind == LineKind::Item {
            if let Some(c) = pending.take() {
                c.apply(&mut line);
            }
            open_item = Some(lines.len());
        }
        lines.push(line);
    }
    Parsed {
        lines,
        total_cents: total,
        date,
    }
}

/// The first `dd.mm.yy` or `dd.mm.yyyy` token in a line. OCR variants with a
/// comma for a point (`08.11,26`) count only when they form a valid date.
pub(crate) fn find_date(line: &str) -> Option<NaiveDate> {
    line.split_whitespace().find_map(|tok| {
        let tok = tok.trim_matches(|c: char| !c.is_ascii_digit());
        let mut p = tok.split(['.', ',']);
        let (d, m, y) = (p.next()?, p.next()?, p.next()?);
        if p.next().is_some() || !(y.len() == 2 || y.len() == 4) {
            return None;
        }
        // All commas (`1,19,05`) are prices and amounts, not dates.
        if !tok.contains('.') {
            return None;
        }
        let year: i32 = y.parse().ok()?;
        let year = if y.len() == 2 { 2000 + year } else { year };
        NaiveDate::from_ymd_opt(year, m.parse().ok()?, d.parse().ok()?)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_and_weight_lines_are_recognised() {
        assert_eq!(
            parse_cont("  2 Stk x   0,99"),
            Some(Cont::Multi {
                count: 2,
                unit_cents: 99
            })
        );
        assert_eq!(
            parse_cont("2 x 0,99"),
            Some(Cont::Multi {
                count: 2,
                unit_cents: 99
            })
        );
        assert_eq!(
            parse_cont("3 St x 0,69"),
            Some(Cont::Multi {
                count: 3,
                unit_cents: 69
            })
        );
        assert_eq!(
            parse_cont("0,512 kg x   2,99 EUR/kg"),
            Some(Cont::Weight {
                grams: 512,
                per_kg_cents: 299
            })
        );
        assert_eq!(
            parse_cont("1,5 kg x 1,99"),
            Some(Cont::Weight {
                grams: 1500,
                per_kg_cents: 199
            })
        );
        assert_eq!(parse_cont("BANANE 1,49 B"), None);
    }

    #[test]
    fn ocr_variants_of_count_and_weight_lines() {
        let two = Some(Cont::Multi {
            count: 2,
            unit_cents: 99,
        });
        for l in [
            "2 Stk X   0,99",
            "2 Stk × 0,99",
            "2 Stk * 0,99",
            "2X 0,99",
            "2 Stk x 0.99",
        ] {
            assert_eq!(parse_cont(l), two, "{l}");
        }
        let w = Some(Cont::Weight {
            grams: 512,
            per_kg_cents: 299,
        });
        for l in [
            "0,512 kg ? 2,99 FUR/kg",
            "0,512 kg X 2,99 EUR/kg",
            "0,512 kg × 2,99 €/kg",
            "0,512 kg x 2,99 /kg",
            "0.512 kg x 2.99 eur/kg",
            "0,512 kg , 2,99",
        ] {
            assert_eq!(parse_cont(l), w, "{l}");
        }
        // A misread x is only tolerated on weight lines.
        assert_eq!(parse_cont("2 Stk ? 0,99"), None);
        assert_eq!(parse_cont("0,512 kg 2,99"), None);
        assert_eq!(parse_cont("0,512 kg xx 2,99"), None);
    }

    #[test]
    fn dates_with_two_or_four_digit_years_parse() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 8);
        assert_eq!(find_date("08.11.26 14:32"), d);
        assert_eq!(find_date("Datum: 08.11.2026"), d);
        assert_eq!(find_date("Tel 0221.000"), None);
        assert_eq!(find_date("32.13.26"), None);
        assert_eq!(find_date("08.11,26 14:32"), d);
        assert_eq!(find_date("08,11.26"), d);
        assert_eq!(find_date("32.11,26"), None);
        assert_eq!(find_date("1,19,05"), None);
        assert_eq!(find_date("1.234,56"), None);
    }

    #[test]
    fn deposits_returns_and_discounts_are_classified() {
        assert_eq!(classify("PFAND EINWEG", 25).0, LineKind::Deposit);
        assert_eq!(classify("LEERGUT", -25), (LineKind::DepositReturn, -25));
        assert_eq!(classify("Pfandbon", 25), (LineKind::DepositReturn, -25));
        assert_eq!(classify("PFAND RUECKGABE", -25).0, LineKind::DepositReturn);
        assert_eq!(classify("Sofort-Rabatt 20%", -30).0, LineKind::Discount);
        assert_eq!(classify("BANANE", 149).0, LineKind::Item);
    }
}
