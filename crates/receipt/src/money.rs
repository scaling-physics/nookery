//! German amounts to integer cents. No floats: receipts must add up to the
//! cent.

/// Parses `1,49`, `-0,50`, `0,50-` (trailing minus, as some tills print),
/// `1.234,56`, with an optional `EUR`/`€`. Exactly two decimals are required
/// after a comma (or, for OCR, after a lone point: `0.79`): `500`, `1.5` or
/// `1,5%` are not prices, which is how a product name is told apart from the
/// price column.
pub(crate) fn parse_cents(s: &str) -> Option<i64> {
    let mut t = s.trim();
    for cur in ["EUR", "€"] {
        t = t.strip_prefix(cur).unwrap_or(t).trim_start();
        t = t.strip_suffix(cur).unwrap_or(t).trim_end();
    }
    let mut negative = false;
    if let Some(rest) = t.strip_prefix(['-', '−']) {
        negative = true;
        t = rest;
    }
    if let Some(rest) = t.strip_suffix(['-', '−']) {
        negative = !negative;
        t = rest;
    }
    // A decimal point is a decimal separator only in the unambiguous OCR
    // case `d{1,3}.dd` with no comma anywhere (`0.79`); `1.5`, `500` and
    // `1.234` are not prices.
    let (int, dec) = match t.split_once(',') {
        Some(parts) => parts,
        None => {
            let (int, dec) = t.split_once('.')?;
            if int.is_empty() || int.len() > 3 || !int.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            (int, dec)
        }
    };
    if dec.len() != 2 || !dec.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut groups = int.split('.');
    let first = groups.next()?;
    if first.is_empty() || first.len() > 3 && int.contains('.') {
        return None;
    }
    let mut euros = String::from(first);
    for g in groups {
        if g.len() != 3 {
            return None;
        }
        euros.push_str(g);
    }
    if !euros.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let cents = euros.parse::<i64>().ok()?.checked_mul(100)? + dec.parse::<i64>().ok()?;
    Some(if negative { -cents } else { cents })
}

fn is_vat(tok: &str) -> bool {
    matches!(tok, "A" | "B" | "*A" | "*B")
}

fn is_currency(tok: &str) -> bool {
    tok.eq_ignore_ascii_case("EUR") || tok == "€"
}

/// OCR junk after the price column where the VAT letter was misread:
/// `Bîšźš`, `îŘîę`, `*`. At most 6 chars, no digits or signs, and nothing
/// ASCII-alphanumeric except an optional leading `A`/`B`, so it can never be
/// part of a product name (`Bäcker` is not junk).
fn is_junk(tok: &str) -> bool {
    if tok.chars().count() > 6 || tok.contains(|c: char| c.is_ascii_digit() || "-−+,".contains(c))
    {
        return false;
    }
    let rest = tok.strip_prefix(['A', 'B']).unwrap_or(tok);
    !rest.is_empty() && !rest.contains(|c: char| c.is_ascii_alphanumeric())
}

/// Splits the price column off a line: `(text, cents)` for
/// `BANANE   1,49 B`. None when the line does not end in an amount.
/// Tolerated (OCR): a VAT letter glued to the price (`0,25A`, `1,53*B`),
/// junk after it (`2,49 Bîšźš`), `0.79` with a point. A point price needs a
/// marker (VAT letter or junk) or a gap of 2+ spaces before it, so a name
/// like `SPEZI 0.33` is not split.
pub(crate) fn split_price(line: &str) -> Option<(&str, i64)> {
    let line = line.trim();
    let toks: Vec<&str> = line.split_whitespace().collect();
    let mut end = toks.len();
    let mut marker = false;
    if end >= 2 && is_junk(toks[end - 1]) {
        end -= 1;
        marker = true;
    }
    if end >= 2 && is_vat(toks[end - 1]) {
        end -= 1;
        marker = true;
    }
    if end >= 2 && is_currency(toks[end - 1]) {
        end -= 1;
    }
    let last = *toks.get(end.checked_sub(1)?)?;
    let (core, cents) = match parse_cents(last) {
        Some(c) => (last, c),
        None => {
            // "1,49B", "1,49*B": the VAT letter glued to the amount.
            let glued = ["*A", "*B", "A", "B"]
                .iter()
                .find_map(|suf| last.strip_suffix(suf))?;
            marker = true;
            (glued, parse_cents(glued)?)
        }
    };
    let at = last.as_ptr() as usize - line.as_ptr() as usize;
    let before = &line[..at];
    if core.contains('.') && !core.contains(',') {
        let gap = before.len() - before.trim_end().len() >= 2;
        if !marker && !gap {
            return None;
        }
    }
    Some((before.trim_end(), cents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_signed_amounts_parse() {
        assert_eq!(parse_cents("1,49"), Some(149));
        assert_eq!(parse_cents("-0,50"), Some(-50));
        assert_eq!(parse_cents("0,50-"), Some(-50));
        assert_eq!(parse_cents("0,05"), Some(5));
    }

    #[test]
    fn thousands_separators_and_currency_are_accepted() {
        assert_eq!(parse_cents("1.234,56"), Some(123_456));
        assert_eq!(parse_cents("12,00 EUR"), Some(1200));
        assert_eq!(parse_cents("€ 3,10"), Some(310));
    }

    #[test]
    fn things_that_are_not_prices_are_rejected() {
        for s in [
            "1,5%", "500", "0,512", "1.5", "1.234", "1,2,3", "1.2345", "1234.56", ".50", "", ",50",
            "1.23,45", "x,12",
        ] {
            assert_eq!(parse_cents(s), None, "{s:?}");
        }
    }

    #[test]
    fn point_decimals_need_exactly_two_digits() {
        assert_eq!(parse_cents("0.79"), Some(79));
        assert_eq!(parse_cents("12.50"), Some(1250));
        assert_eq!(parse_cents("-0.50"), Some(-50));
        assert_eq!(parse_cents("1.234,56"), Some(123_456));
    }

    #[test]
    fn ocr_noise_around_the_price_column() {
        assert_eq!(
            split_price("MINERALWASSER   0.79 A"),
            Some(("MINERALWASSER", 79))
        );
        assert_eq!(split_price("PFAND   0.25A"), Some(("PFAND", 25)));
        assert_eq!(split_price("BANANEN   1,53B"), Some(("BANANEN", 153)));
        assert_eq!(split_price("BANANEN   1,53*B"), Some(("BANANEN", 153)));
        assert_eq!(split_price("KAESE   2,49 Bîšźš"), Some(("KAESE", 249)));
        assert_eq!(split_price("BROT   2,29 îŘîę"), Some(("BROT", 229)));
        assert_eq!(
            split_price("SUMME   EUR   9,77"),
            Some(("SUMME   EUR", 977))
        );
        // Names with numbers stay names.
        assert_eq!(
            split_price("MILCH 3,5% 1L   1,19 B"),
            Some(("MILCH 3,5% 1L", 119))
        );
        assert_eq!(
            split_price("SPEZI 0.33   1,29 A"),
            Some(("SPEZI 0.33", 129))
        );
        assert_eq!(split_price("EIER 10ER   2,99 B"), Some(("EIER 10ER", 299)));
        assert_eq!(split_price("BARILLA 1,99"), Some(("BARILLA", 199)));
        assert_eq!(split_price("SPEZI 0.33"), None);
        assert_eq!(split_price("MILCH 1.5"), None);
        assert_eq!(split_price("KAESE 2,49 Bäcker"), None);
        assert_eq!(split_price("KAESE 2,49 Brot"), None);
    }

    #[test]
    fn price_column_is_split_from_the_text() {
        assert_eq!(split_price("BANANE   1,49 B"), Some(("BANANE", 149)));
        assert_eq!(
            split_price("MILCH 1,5%   1,19 B"),
            Some(("MILCH 1,5%", 119))
        );
        assert_eq!(split_price("PFAND 0,25 A"), Some(("PFAND", 25)));
        assert_eq!(split_price("SUMME EUR 9,77"), Some(("SUMME EUR", 977)));
        assert_eq!(split_price("KAESE 2,49B"), Some(("KAESE", 249)));
        assert_eq!(split_price("MILCH 1,5%"), None);
        assert_eq!(split_price("Musterweg 7"), None);
    }
}
