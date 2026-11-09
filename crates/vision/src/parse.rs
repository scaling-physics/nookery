//! The model's text answer: parsing lines and merging tiles.

use crate::{RawItem, Sighting};

fn parse_count(s: &str) -> Option<u32> {
    let s = s
        .trim()
        .trim_matches(|c: char| matches!(c, 'x' | 'X' | '×' | '*'));
    s.trim().parse::<u32>().ok()
}

/// Drops list decoration: "- ", "* ", "• ", "1. ", "2) ".
fn strip_marker(line: &str) -> &str {
    let l = line.trim();
    let l = l.trim_start_matches(['-', '*', '•', '–']).trim_start();
    let digits = l.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &l[digits..];
        if let Some(r) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
            if r.starts_with(' ') {
                return r.trim_start();
            }
        }
    }
    l
}

/// One line; None when it isn't an item (no `|`, header, empty name).
pub(crate) fn parse_line(line: &str) -> Option<RawItem> {
    let line = strip_marker(line);
    if !line.contains('|') {
        return None;
    }
    let parts: Vec<&str> = line.split('|').map(str::trim).collect();
    let name = parts[0].trim_matches('*').trim();
    if name.is_empty() || name.eq_ignore_ascii_case("name") {
        return None;
    }
    let (text, count) = match parts.len() {
        2 => match parse_count(parts[1]) {
            Some(c) => (String::new(), c),
            None => (parts[1].to_string(), 1),
        },
        n => match parse_count(parts[n - 1]) {
            Some(c) => (parts[1..n - 1].join(" | "), c),
            None => (
                parts[1..].join(" | ").trim_matches(['|', ' ']).to_string(),
                1,
            ),
        },
    };
    let text = text.trim();
    let text = if matches!(text, "" | "-" | "—" | "–") {
        None
    } else {
        Some(text.to_string())
    };
    Some(RawItem {
        name: name.to_string(),
        text,
        count: count_or_one(count),
    })
}

fn count_or_one(c: u32) -> u32 {
    c.clamp(1, 999)
}

pub fn parse_lines(answer: &str) -> Vec<RawItem> {
    answer.lines().filter_map(parse_line).collect()
}

/// Lower case, single spaces: how names are compared for repeats.
pub(crate) fn norm_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// `norm_name` plus a naive singular for the last word, so "tomato" and
/// "tomatoes" meet. Wrong for some words, which only costs a missed merge.
fn merge_key(name: &str) -> String {
    let mut n = norm_name(name);
    let (head, last) = match n.rfind(' ') {
        Some(i) => (n[..=i].to_string(), n[i + 1..].to_string()),
        None => (String::new(), n.clone()),
    };
    let sing = if let Some(s) = last.strip_suffix("ies").filter(|s| !s.is_empty()) {
        format!("{s}y")
    } else if let Some(s) = ["oes", "ches", "shes", "sses", "xes"]
        .iter()
        .find_map(|suf| last.strip_suffix(suf).map(|s| (s, suf)))
        .map(|(s, suf)| format!("{s}{}", &suf[..suf.len() - 2]))
    {
        s
    } else if last.len() > 3 && last.ends_with('s') && !last.ends_with("ss") {
        last[..last.len() - 1].to_string()
    } else {
        last
    };
    n = format!("{head}{sing}");
    n
}

fn texts_compatible(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => {
            // "Arla" read in one tile and "Arla Skyr" in another: same thing.
            let (x, y) = (norm_name(x), norm_name(y));
            x.contains(&y) || y.contains(&x)
        }
        _ => true,
    }
}

pub fn merge(per_tile: &[(usize, Vec<RawItem>)]) -> Vec<Sighting> {
    let mut order: Vec<&(usize, Vec<RawItem>)> = per_tile.iter().collect();
    order.sort_by_key(|(i, _)| *i); // stable: keeps call order within a tile
    let mut out: Vec<(String, Sighting)> = Vec::new();
    for (tile, items) in order {
        for item in items {
            let key = merge_key(&item.name);
            let hit = out
                .iter_mut()
                .find(|(k, s)| *k == key && texts_compatible(&s.text, &item.text));
            match hit {
                Some((_, s)) => {
                    s.count = s.count.max(item.count);
                    if !s.tiles.contains(tile) {
                        s.tiles.push(*tile);
                    }
                    if item.text.as_ref().map_or(0, |t| t.len())
                        > s.text.as_ref().map_or(0, |t| t.len())
                    {
                        s.text = item.text.clone();
                    }
                    if item.name.len() > s.name.len() {
                        s.name = item.name.clone();
                    }
                }
                None => out.push((
                    key,
                    Sighting {
                        name: item.name.clone(),
                        text: item.text.clone(),
                        count: item.count,
                        tiles: vec![*tile],
                    },
                )),
            }
        }
    }
    out.into_iter()
        .map(|(_, mut s)| {
            s.tiles.sort_unstable();
            s
        })
        .collect()
}
