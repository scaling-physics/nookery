//! Reading the dates people type and labels print.

use chrono::{Datelike, Duration, NaiveDate};

/// `2026-11-20`, `20.11.2026`, `20.11.26`, `20.11.` / `20.11` (the next such
/// day from `today`, as a label without a year means), `20/11/2026`, and
/// `+3` (days from today).
pub fn parse_date(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let t = s.trim();
    if let Some(days) = t.strip_prefix('+') {
        return days.parse::<i64>().ok().map(|d| today + Duration::days(d));
    }
    if let Ok(d) = NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return Some(d);
    }
    let parts: Vec<&str> = t.trim_end_matches(['.', '/']).split(['.', '/']).collect();
    let num = |p: &str| p.trim().parse::<u32>().ok();
    match parts.as_slice() {
        [d, m] => {
            let (d, m) = (num(d)?, num(m)?);
            let this = NaiveDate::from_ymd_opt(today.year(), m, d)?;
            if this >= today - Duration::days(7) {
                Some(this)
            } else {
                NaiveDate::from_ymd_opt(today.year() + 1, m, d)
            }
        }
        [d, m, y] => {
            let y = num(y)? as i32;
            let y = if y < 100 { 2000 + y } else { y };
            NaiveDate::from_ymd_opt(y, num(m)?, num(d)?)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn reads_common_forms() {
        let today = d(2026, 11, 8);
        assert_eq!(parse_date("2026-11-20", today), Some(d(2026, 11, 20)));
        assert_eq!(parse_date("20.11.2026", today), Some(d(2026, 11, 20)));
        assert_eq!(parse_date("20.11.26", today), Some(d(2026, 11, 20)));
        assert_eq!(parse_date("20/11/2026", today), Some(d(2026, 11, 20)));
        assert_eq!(parse_date("+3", today), Some(d(2026, 11, 11)));
    }

    #[test]
    fn a_label_without_a_year_means_the_next_one() {
        let today = d(2026, 11, 8);
        assert_eq!(parse_date("20.11.", today), Some(d(2026, 11, 20)));
        // A few days past still means this year (an expired label).
        assert_eq!(parse_date("05.11", today), Some(d(2026, 11, 5)));
        assert_eq!(parse_date("15.01.", today), Some(d(2027, 1, 15)));
    }

    #[test]
    fn rejects_impossible_dates() {
        let today = d(2026, 11, 8);
        assert_eq!(parse_date("31.02.2026", today), None);
        assert_eq!(parse_date("soon", today), None);
        assert_eq!(parse_date("", today), None);
    }
}
