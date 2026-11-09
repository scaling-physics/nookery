use nookery_core::Qty;
use nookery_receipt::{parse_text, Chain, LineKind, ParseError, Receipt};

fn load(name: &str) -> (String, Receipt) {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).unwrap();
    let r = parse_text(&text, None).unwrap();
    (text, r)
}

fn kinds(r: &Receipt) -> Vec<LineKind> {
    r.lines.iter().map(|l| l.kind).collect()
}

fn find<'a>(r: &'a Receipt, text: &str) -> &'a nookery_receipt::Line {
    r.lines
        .iter()
        .find(|l| l.text == text)
        .unwrap_or_else(|| panic!("no line {text}"))
}

fn common(name: &str, chain: Chain, lines: usize, total: i64) -> Receipt {
    let (text, r) = load(name);
    assert_eq!(r.chain, Some(chain));
    assert_eq!(r.lines.len(), lines);
    assert_eq!(r.total_cents, Some(total));
    assert_eq!(r.check_total(), Ok(()));
    assert_eq!(r.date, chrono::NaiveDate::from_ymd_opt(2026, 11, 8));
    assert!(r
        .lines
        .iter()
        .all(|l| l.key == nookery_receipt::alias_key(&l.text)));
    assert_eq!(r.id(), parse_text(&text, None).unwrap().id());
    assert_eq!(r.id().len(), 16);
    r
}

#[test]
fn rewe_receipt_parses_and_adds_up() {
    use LineKind::*;
    let r = common("rewe.txt", Chain::Rewe, 9, 977);
    assert_eq!(
        kinds(&r),
        [
            Item,
            Item,
            Item,
            Item,
            Deposit,
            Item,
            Discount,
            Item,
            DepositReturn
        ]
    );
    let j = find(&r, "JOGHURT NATUR");
    assert_eq!(
        (j.count, j.unit_price_cents, j.amount_cents),
        (2, Some(99), 198)
    );
    let b = find(&r, "BANANEN");
    assert_eq!(
        (b.weight, b.unit_price_cents),
        (Some(Qty::grams(512)), Some(299))
    );
    assert_eq!(find(&r, "BIO H-MILCH 1,5%").key, "BIO H-MILCH 1,5%");
}

#[test]
fn edeka_receipt_takes_counts_and_weights_from_the_line_above() {
    use LineKind::*;
    let r = common("edeka.txt", Chain::Edeka, 7, 703);
    assert_eq!(kinds(&r), [Item, Item, Item, Item, Deposit, Item, Discount]);
    let j = find(&r, "JOGHURT NATUR");
    assert_eq!((j.count, j.unit_price_cents), (2, Some(99)));
    assert_eq!(find(&r, "MILCH 3,5% 1L").count, 1);
    assert_eq!(find(&r, "BANANEN").weight, Some(Qty::grams(512)));
}

#[test]
fn netto_receipt_is_not_fooled_by_its_vat_table() {
    use LineKind::*;
    let r = common("netto.txt", Chain::Netto, 9, 859);
    assert_eq!(
        kinds(&r),
        [
            Item,
            Item,
            Item,
            Item,
            Item,
            Deposit,
            Item,
            Discount,
            DepositReturn
        ]
    );
    let t = find(&r, "TOMATEN PASSIERT");
    assert_eq!(
        (t.count, t.unit_price_cents, t.amount_cents),
        (3, Some(69), 207)
    );
    assert_eq!(find(&r, "EIER 10 STK").count, 1);
    assert_eq!(find(&r, "AEPFEL").weight, Some(Qty::grams(1036)));
    assert_eq!(find(&r, "PFANDBON").amount_cents, -25);
}

#[test]
fn denns_receipt_parses_and_adds_up() {
    use LineKind::*;
    let r = common("denns.txt", Chain::Denns, 7, 863);
    assert_eq!(
        kinds(&r),
        [Item, Item, Item, Item, Deposit, Discount, DepositReturn]
    );
    assert_eq!(find(&r, "BIO EIER 6ER").count, 2);
    assert_eq!(find(&r, "BIO BANANEN").weight, Some(Qty::grams(560)));
}

#[test]
fn a_chain_given_by_the_caller_wins_over_detection() {
    let (text, _) = load("rewe.txt");
    assert_eq!(
        parse_text(&text, Some(Chain::Netto)).unwrap().chain,
        Some(Chain::Netto)
    );
}

#[test]
fn text_without_prices_has_no_lines() {
    assert_eq!(parse_text("REWE\nKoeln\n", None), Err(ParseError::NoLines));
}
