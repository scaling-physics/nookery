//! German amounts to integer cents. No floats: receipts must add up to the
//! cent.

/// Parses `1,49`, `-0,50`, `0,50-` (trailing minus, as some tills print),
/// `1.234,56`, with an optional `EUR`/`€`. A comma and exactly two decimals
/// are required: `500` or `1,5%` are not prices, which is how a product name
/// is told apart from the price column.
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
    let (int, dec) = t.split_once(',')?;
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

/// Splits the price column off a line: `(text, cents)` for
/// `BANANE   1,49 B`. None when the line does not end in an amount.
pub(crate) fn split_price(line: &str) -> Option<(&str, i64)> {
    let line = line.trim();
    let toks: Vec<&str> = line.split_whitespace().collect();
    let mut end = toks.len();
    if end >= 2 && is_vat(toks[end - 1]) {
        end -= 1;
    }
    if end >= 2 && is_currency(toks[end - 1]) {
        end -= 1;
    }
    let last = *toks.get(end.checked_sub(1)?)?;
    let cents = parse_cents(last).or_else(|| {
        // "1,49B": the VAT letter glued to the amount.
        let glued = last.strip_suffix(['A', 'B'])?;
        parse_cents(glued)
    })?;
    let at = last.as_ptr() as usize - line.as_ptr() as usize;
    Some((line[..at].trim_end(), cents))
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
            "1,5%", "500", "0,512", "1.49", "1,2,3", "", ",50", "1.23,45", "x,12",
        ] {
            assert_eq!(parse_cents(s), None, "{s:?}");
        }
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
