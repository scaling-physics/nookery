//! Private helpers: prompt stock lines, answer parsing, checking, Ollama
//! error mapping.

use crate::*;
use nookery_core::names_match;

/// Some variety between runs; ideas are meant to differ.
pub(crate) const TEMPERATURE: f64 = 0.7;

// ---- Ollama errors (same mapping as the vision crate) ----

pub(crate) fn model_error(model: &str, msg: &str) -> RecipeError {
    if msg.to_lowercase().contains("unauthorized") {
        RecipeError::SignIn
    } else if msg.to_lowercase().contains("not found") {
        RecipeError::ModelMissing(model.to_string())
    } else {
        RecipeError::Model(msg.trim().to_string())
    }
}

pub(crate) fn transport_error(url: &str, e: ureq::Error) -> RecipeError {
    match e {
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            RecipeError::Unreachable(url.to_string())
        }
        ureq::Error::Io(ref io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
            RecipeError::Unreachable(url.to_string())
        }
        other => RecipeError::Model(other.to_string()),
    }
}

// ---- prompt: stock ----

/// One line per non-empty group, lots with the same name grouped.
pub(crate) fn stock_lines(stock: &[StockItem]) -> String {
    let sections: [(&str, Option<Location>); 4] = [
        ("USE SOON", None),
        ("FRIDGE", Some(Location::Fridge)),
        ("FREEZER", Some(Location::Freezer)),
        ("PANTRY", Some(Location::Pantry)),
    ];
    let pick = |s: &StockItem, place: Option<Location>| match place {
        None => s.urgency == Urgency::UseSoon,
        Some(l) => s.urgency == Urgency::Normal && s.location == l,
    };
    let mut out = String::new();
    for (title, place) in sections {
        let mut groups: Vec<(String, Vec<&StockItem>)> = Vec::new();
        for s in stock.iter().filter(|s| pick(s, place)) {
            let key = s.item.trim().to_lowercase();
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, v)) => v.push(s),
                None => groups.push((key, vec![s])),
            }
        }
        let entries: Vec<String> = groups.iter().map(|(_, v)| entry(v)).collect();
        if !entries.is_empty() {
            out.push_str(&format!("{title}: {}\n", entries.join(", ")));
        }
    }
    out
}

fn entry(lots: &[&StockItem]) -> String {
    let first = lots[0];
    let name = first.item.trim();
    if lots.len() > 1 {
        return format!("{name} x{} packs", lots.len());
    }
    let mut e = name.to_string();
    // "1" is just "one of it": the name says enough.
    if first.left != "1" && !first.left.is_empty() {
        if first.left.chars().all(|c| c.is_ascii_digit() || c == '.') {
            e.push_str(&format!(" x{}", first.left));
        } else {
            e.push_str(&format!(" {}", first.left));
        }
    }
    if first.opened {
        e.push_str(" (opened)");
    }
    e
}

// ---- parse_ideas ----

/// Strips markdown noise: `**`, leading `#`, `1.`/`1)`, bullets.
fn clean_line(line: &str) -> String {
    let l = line.replace("**", "").replace("__", "");
    let l = l.trim().trim_start_matches('#').trim();
    let l = l.trim_start_matches(['-', '*', '•']).trim();
    strip_number(l).trim().to_string()
}

fn strip_number(s: &str) -> &str {
    let digits = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        let rest = &s[digits..];
        if let Some(r) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
            return r.trim_start();
        }
    }
    s
}

fn field(line: &str) -> Option<(&'static str, String)> {
    let (k, v) = line.split_once(':')?;
    let key = match k.trim().to_lowercase().as_str() {
        "why" => "why",
        "time" | "minutes" | "duration" => "time",
        "uses" | "use" => "uses",
        "needs" | "need" => "needs",
        _ => return None,
    };
    Some((key, v.trim().to_string()))
}

pub(crate) fn split_list(v: &str) -> Vec<String> {
    v.split(',')
        .map(|p| {
            p.trim()
                .trim_end_matches('.')
                .trim_matches(['"', '*'])
                .trim()
        })
        .filter(|p| {
            let l = p.to_lowercase();
            !p.is_empty() && !matches!(l.as_str(), "none" | "-" | "n/a" | "nothing" | "no" | "–")
        })
        .map(String::from)
        .collect()
}

pub(crate) fn parse_minutes(v: &str) -> Option<u32> {
    let start = v.find(|c: char| c.is_ascii_digit())?;
    let rest = &v[start..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n: u32 = digits.parse().ok()?;
    let after = rest[digits.len()..].trim_start().to_lowercase();
    Some(if after.starts_with('h') { n * 60 } else { n })
}

pub(crate) fn parse_ideas_impl(answer: &str) -> Vec<Idea> {
    let mut ideas: Vec<Idea> = Vec::new();
    let new = |name: &str| Idea {
        name: name.trim().trim_end_matches([':', '.']).trim().to_string(),
        why: String::new(),
        minutes: None,
        uses: vec![],
        needs: vec![],
    };
    for raw in answer.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        let c = clean_line(t);
        if let Some((key, v)) = field(&c) {
            let Some(idea) = ideas.last_mut() else {
                continue;
            };
            match key {
                "why" => idea.why = v,
                "time" => idea.minutes = parse_minutes(&v),
                "uses" => idea.uses = split_list(&v),
                _ => idea.needs = split_list(&v),
            }
            continue;
        }
        let is_header = t.starts_with('#');
        let numbered = !t.starts_with('#')
            && strip_number(t.trim_start_matches("**")) != t.trim_start_matches("**");
        let bold = t.starts_with("**") && t.trim_end().ends_with("**");
        if (is_header || numbered || bold) && !c.is_empty() {
            ideas.push(new(&c));
        }
    }
    ideas.retain(|i| !i.name.is_empty());
    ideas
}

// ---- check ----

/// Drops parentheses and quantities at either end: "2 eggs", "skyr 500g",
/// "pesto x2", "200 g spinach" -> the bare name, lower case.
pub(crate) fn strip_qty(name: &str) -> String {
    let mut no_paren = String::new();
    let mut depth = 0;
    for ch in name.chars() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => no_paren.push(ch),
            _ => {}
        }
    }
    let mut words: Vec<&str> = no_paren.split_whitespace().collect();
    const UNITS: [&str; 14] = [
        "g", "kg", "ml", "l", "cl", "dl", "tbsp", "tsp", "cup", "cups", "pcs", "piece", "pieces",
        "packs",
    ];
    let qty_like = |w: &str| {
        let w = w.to_lowercase();
        w.starts_with(|c: char| c.is_ascii_digit())
            || (w.starts_with('x') && w.len() > 1 && w[1..].chars().all(|c| c.is_ascii_digit()))
    };
    while words.last().is_some_and(|w| qty_like(w)) {
        words.pop();
        if words
            .last()
            .is_some_and(|w| UNITS.contains(&w.to_lowercase().as_str()))
        {
            words.pop();
        }
    }
    while words.first().is_some_and(|w| qty_like(w)) {
        words.remove(0);
        if words
            .first()
            .is_some_and(|w| UNITS.contains(&w.to_lowercase().as_str()))
        {
            words.remove(0);
        }
    }
    words.join(" ").to_lowercase()
}

fn is_staple(clean: &str) -> bool {
    // "pepper" matches "bell pepper" in names_match; a vegetable is not a staple.
    let words: Vec<&str> = clean.split_whitespace().collect();
    if words.len() > 1
        && words.last() == Some(&"pepper")
        && !matches!(words[0], "black" | "white" | "ground" | "cracked")
    {
        return false;
    }
    STAPLES.iter().any(|s| names_match(clean, s))
}

fn rank(s: &StockItem) -> u8 {
    match s.urgency {
        Urgency::UseSoon => 0,
        Urgency::Normal => 1,
        Urgency::Expired => 2,
    }
}

pub(crate) fn check_one(idea: Idea, stock: &[StockItem]) -> Suggestion {
    let mut have: Vec<(String, LotId)> = Vec::new();
    let mut need: Vec<String> = Vec::new();
    let mut expired: Vec<LotId> = Vec::new();
    let mut assigned: Vec<(LotId, String)> = Vec::new();

    let mut resolve = |written: &str, from_uses: bool| {
        let written = written.trim();
        let clean = strip_qty(written);
        if clean.is_empty() || is_staple(&clean) {
            return;
        }
        if have.iter().any(|(w, _)| strip_qty(w) == clean)
            || need.iter().any(|n| strip_qty(n) == clean)
        {
            return;
        }
        let mut cands: Vec<(usize, &StockItem)> = stock
            .iter()
            .enumerate()
            .filter(|(_, s)| s.urgency != Urgency::Expired && names_match(&clean, &s.item))
            .collect();
        cands.sort_by_key(|(i, s)| (rank(s), *i));
        let free = cands.iter().find(|(_, s)| {
            assigned
                .iter()
                .find(|(l, _)| *l == s.lot)
                .is_none_or(|(_, n)| names_match(n, &clean))
        });
        if let Some((_, s)) = free {
            assigned.push((s.lot, clean));
            have.push((written.to_string(), s.lot));
            return;
        }
        if cands.is_empty() && from_uses {
            if let Some(s) = stock
                .iter()
                .find(|s| s.urgency == Urgency::Expired && names_match(&clean, &s.item))
            {
                if !expired.contains(&s.lot) {
                    expired.push(s.lot);
                }
            }
        }
        if !need.iter().any(|n| {
            let nc = strip_qty(n);
            nc == clean || names_match(&nc, &clean)
        }) {
            need.push(written.to_string());
        }
    };
    for u in &idea.uses {
        resolve(u, true);
    }
    for n in &idea.needs {
        resolve(n, false);
    }

    let mut soon: Vec<LotId> = Vec::new();
    for (_, l) in &have {
        if !soon.contains(l)
            && stock
                .iter()
                .any(|s| s.lot == *l && s.urgency == Urgency::UseSoon)
        {
            soon.push(*l);
        }
    }
    let uses_soon = soon.len() as u32;
    let score = 3 * uses_soon as i32 + have.len() as i32 - 2 * need.len() as i32;
    Suggestion {
        idea,
        have,
        need,
        uses_soon,
        expired,
        score,
    }
}

// ---- parse_recipe ----

#[derive(PartialEq, Clone, Copy)]
enum Section {
    None,
    Ingredients,
    Steps,
}

fn section_of(c: &str) -> Option<Section> {
    let w = c.trim_end_matches(':').trim().to_lowercase();
    match w.as_str() {
        "ingredients" => Some(Section::Ingredients),
        "steps" | "instructions" | "method" | "directions" | "preparation" => Some(Section::Steps),
        _ => None,
    }
}

fn strip_step_label(s: &str) -> String {
    let s = s.trim();
    let lower = s.to_lowercase();
    let s = if lower.starts_with("step ") {
        let rest = s[5..].trim_start();
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits > 0 {
            rest[digits..].trim_start_matches([':', '.', ')', ' '])
        } else {
            s
        }
    } else {
        strip_number(s)
    };
    s.trim().to_string()
}

pub(crate) fn parse_recipe_impl(answer: &str) -> Recipe {
    let mut r = Recipe {
        title: String::new(),
        serves: None,
        minutes: None,
        ingredients: vec![],
        steps: vec![],
        raw: answer.to_string(),
    };
    let mut section = Section::None;
    for raw in answer.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        let plain = t.replace("**", "").replace("__", "");
        let plain = plain.trim();
        let hashes = plain.chars().take_while(|&c| c == '#').count();
        let body = plain.trim_start_matches('#').trim();
        if let Some(sec) = section_of(body) {
            section = sec;
            continue;
        }
        if hashes == 1 && r.title.is_empty() {
            r.title = body.to_string();
            continue;
        }
        let bullet = body.starts_with(['-', '*', '•']);
        let b = body.trim_start_matches(['-', '*', '•']).trim();
        if let Some((k, v)) = b.split_once(':') {
            match k.trim().to_lowercase().as_str() {
                "serves" | "servings" | "serving" | "yield" => {
                    r.serves = parse_minutes_plain(v);
                    continue;
                }
                "time" | "total time" | "minutes" => {
                    r.minutes = parse_minutes(v);
                    continue;
                }
                _ => {}
            }
        }
        let numbered = strip_number(b) != b || b.to_lowercase().starts_with("step ");
        let target = match section {
            Section::None if numbered => Section::Steps,
            Section::None if bullet => Section::Ingredients,
            s => s,
        };
        match target {
            Section::Ingredients => r.ingredients.push(b.to_string()),
            Section::Steps => {
                let s = strip_step_label(b);
                if !s.is_empty() {
                    r.steps.push(s)
                }
            }
            Section::None => {
                if r.title.is_empty() {
                    r.title = body.to_string();
                }
            }
        }
    }
    r
}

fn parse_minutes_plain(v: &str) -> Option<u32> {
    let digits: String = v
        .trim()
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}
