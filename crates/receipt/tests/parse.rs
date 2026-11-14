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

fn ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn ocr_text_parses_to_the_same_lines_as_the_clean_receipt() {
    let (_, clean) = load("rewe.txt");
    let (_, ocr) = load("rewe-ocr.txt");
    assert_eq!(ocr.check_total(), Ok(()));
    assert_eq!(ocr.total_cents, clean.total_cents);
    assert_eq!(ocr.date, clean.date);
    assert_eq!(ocr.chain, clean.chain);
    assert_eq!(ocr.lines.len(), clean.lines.len());
    for (a, b) in ocr.lines.iter().zip(&clean.lines) {
        assert_eq!(ws(&a.text), ws(&b.text));
        assert_eq!(
            (
                a.kind,
                a.count,
                a.weight,
                a.unit_price_cents,
                a.amount_cents
            ),
            (
                b.kind,
                b.count,
                b.weight,
                b.unit_price_cents,
                b.amount_cents
            )
        );
    }
}

fn receipt_with(extra: &str, total: &str) -> String {
    format!("REWE Markt\nBANANEN   1,00 B\n{extra}\nSUMME   EUR   {total}\n08.11.26\n")
}

#[test]
fn point_decimals_glued_vat_and_x_variants_parse_in_a_receipt() {
    let t = "REWE Markt\nMINERALWASSER   0.79 A\nPFAND   0.25A\nJOGHURT   1,98B\n2 Stk X   0,99\n\
             BANANEN   1,53 B\n0,512 kg ? 2,99 FUR/kg\nSUMME   EUR   4,55\n08.11,26\n";
    let r = parse_text(t, None).unwrap();
    assert_eq!(r.check_total(), Ok(()));
    assert_eq!(r.date, chrono::NaiveDate::from_ymd_opt(2026, 11, 8));
    assert_eq!(find(&r, "JOGHURT").count, 2);
    assert_eq!(find(&r, "BANANEN").weight, Some(Qty::grams(512)));
    assert_eq!(find(&r, "PFAND").amount_cents, 25);
}

#[test]
fn names_with_numbers_are_not_mistaken_for_prices() {
    let t = "REWE Markt\nMILCH 3,5% 1L   1,19 B\nSPEZI 0.33   1,29 A\nEIER 10ER   2,99 B\n\
             SUMME   EUR   5,47\n";
    let r = parse_text(t, None).unwrap();
    assert_eq!(r.check_total(), Ok(()));
    let names: Vec<&str> = r.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(names, ["MILCH 3,5% 1L", "SPEZI 0.33", "EIER 10ER"]);
    // A name with a point number and no price column is not a line at all.
    let r = parse_text("REWE\nSPEZI 0.33\nBANANEN   1,00 B\nSUMME   1,00\n", None).unwrap();
    assert_eq!(r.lines.len(), 1);
    // 1.5 is not a price.
    let r = parse_text("REWE\nMILCH 1.5\nBANANEN   1,00 B\nSUMME   1,00\n", None).unwrap();
    assert_eq!(r.lines.len(), 1);
}

#[test]
fn a_lost_minus_is_never_invented() {
    let ok = parse_text(&receipt_with("Sofort-Rabatt   -0,50", "0,50"), None).unwrap();
    assert_eq!(ok.check_total(), Ok(()));
    let lost = parse_text(&receipt_with("Sofort-Rabatt   0,50", "0,50"), None).unwrap();
    assert!(lost.check_total().is_err());
    assert_eq!(find(&lost, "Sofort-Rabatt").amount_cents, 50);
}

#[test]
fn junk_after_the_price_only_counts_when_it_cannot_be_a_name() {
    let r = parse_text(&receipt_with("KAESE   2,49 Bîšźš", "3,49"), None).unwrap();
    assert_eq!(r.check_total(), Ok(()));
    let r = parse_text(&receipt_with("KAESE   2,49 Brot", "3,49"), None).unwrap();
    assert_eq!(r.lines.len(), 1);
}
