//! Recipe pictures: the model picks the closest drawn dish kind and which
//! ingredients to show. Decoration only: the caller falls back to word
//! matching when anything here fails. The model writes ids and names; every
//! answer is checked against the option list and the dish's own ingredients.

use serde::{Deserialize, Serialize};

/// Most ingredients shown per picture.
pub const MAX_SHOWN: usize = 4;
/// Tokens to allow per dish in the answer.
pub const TOKENS_PER_DISH: u32 = 40;

/// One dish to draw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtDish {
    pub title: String,
    pub ingredients: Vec<String>,
}

/// The model's choice for one dish.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtPick {
    pub kind: String,
    pub show: Vec<String>,
}

/// One prompt for a batch of dishes. `options` are `(id, label)`.
pub fn art_prompt(dishes: &[ArtDish], options: &[(String, String)]) -> String {
    let mut p = String::from(
        "You choose a picture for each dish from a fixed list of dish kinds.\n\nDISH KINDS (id - label):\n",
    );
    for (id, label) in options {
        p.push_str(&format!("{id} \u{2014} {label}\n"));
    }
    p.push_str("\nDISHES:\n");
    for (i, d) in dishes.iter().enumerate() {
        p.push_str(&format!(
            "{}. {} (ingredients: {})\n",
            i + 1,
            d.title.trim(),
            d.ingredients.join(", ")
        ));
    }
    p.push_str(&format!(
        "\nFor each dish pick the id of the closest dish kind, and up to {MAX_SHOWN} ingredients \
         that are clearly visible on the plate, copied exactly from that dish's list. \
         Answer with exactly one line per dish and nothing else:\n\
         N | id | ingredient, ingredient, ...\n"
    ));
    p
}

fn matches_ingredient(shown: &str, ingredient: &str) -> bool {
    let (a, b) = (shown.to_lowercase(), ingredient.to_lowercase());
    !a.is_empty() && (b.contains(&a) || a.contains(&b))
}

/// Reads the answer: one entry per dish, `None` where the line is missing or
/// the id is not in `options`. Tolerates spacing, case, `-`/`\u{2014}`
/// separators, `1.` / `1)` numbering and junk lines.
pub fn parse_art(
    answer: &str,
    dishes: &[ArtDish],
    options: &[(String, String)],
) -> Vec<Option<ArtPick>> {
    let mut out: Vec<Option<ArtPick>> = vec![None; dishes.len()];
    for line in answer.lines() {
        let line = line.trim().trim_start_matches(['*', '`', '-', ' ']);
        // Leading number, optionally followed by . ) : or the separator.
        let digits: String = line.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        let Ok(n) = digits.parse::<usize>() else {
            continue;
        };
        if n == 0 || n > dishes.len() || out[n - 1].is_some() {
            continue;
        }
        let rest = line[digits.len()..].trim_start_matches(['.', ')', ':', ' ']);
        let rest = rest.trim_start_matches(['|', '-', '\u{2014}', '\u{2013}', ' ']);
        let rest = rest.replace(" - ", " | ");
        let mut parts = rest.splitn(2, ['|', '\u{2014}', '\u{2013}']);
        let id = parts.next().unwrap_or("").trim();
        let list = parts.next().unwrap_or("");
        let id = id.trim_matches(['`', '*', ' ']).to_lowercase();
        let Some((kind, _)) = options.iter().find(|(o, _)| o.to_lowercase() == id) else {
            continue;
        };
        let mut show: Vec<String> = Vec::new();
        for name in list.split(',') {
            let name = name.trim().trim_matches(['`', '*', '.', ' ']);
            if let Some(real) = dishes[n - 1]
                .ingredients
                .iter()
                .find(|i| matches_ingredient(name, i))
            {
                if !show.contains(real) && show.len() < MAX_SHOWN {
                    show.push(real.clone());
                }
            }
        }
        out[n - 1] = Some(ArtPick {
            kind: kind.clone(),
            show,
        });
    }
    out
}

/// Asks `model` about a batch (callers keep it to about 8 dishes).
pub fn pick_art(
    model: &dyn crate::TextModel,
    dishes: &[ArtDish],
    options: &[(String, String)],
) -> Result<Vec<Option<ArtPick>>, crate::RecipeError> {
    if dishes.is_empty() {
        return Ok(Vec::new());
    }
    let prompt = art_prompt(dishes, options);
    let max = TOKENS_PER_DISH * dishes.len() as u32;
    let answer = model.generate(&prompt, max, &mut |_| {})?;
    Ok(parse_art(&answer, dishes, options))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dish(t: &str, i: &[&str]) -> ArtDish {
        ArtDish {
            title: t.into(),
            ingredients: i.iter().map(|s| s.to_string()).collect(),
        }
    }
    fn opts() -> Vec<(String, String)> {
        [
            ("shakshuka", "Shakshuka"),
            ("ramen", "Ramen"),
            ("green-salad", "Green salad"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
    }

    #[test]
    fn prompt_lists_options_and_dishes() {
        let p = art_prompt(&[dish("Egg pan", &["egg", "tomato"])], &opts());
        assert!(p.contains("ramen \u{2014} Ramen"));
        assert!(p.contains("1. Egg pan (ingredients: egg, tomato)"));
        assert!(p.contains("N | id | ingredient"));
    }

    #[test]
    fn parses_tolerantly() {
        let dishes = vec![
            dish(
                "A",
                &["Egg", "Tomatoes", "feta", "onion", "parsley", "chili"],
            ),
            dish("B", &["noodles", "egg"]),
            dish("C", &["lettuce"]),
            dish("D", &["x"]),
        ];
        let answer = "Sure!\n1 |  Shakshuka | egg, tomato, FETA, onion, parsley, chili\n\
                      2) RAMEN \u{2014} noodles, pork\n\
                      **3.** nonsense | lettuce\n\
                      junk line\n\
                      4 - green-salad - x";
        let r = parse_art(answer, &dishes, &opts());
        let a = r[0].as_ref().unwrap();
        assert_eq!(a.kind, "shakshuka");
        assert_eq!(a.show, ["Egg", "Tomatoes", "feta", "onion"]);
        let b = r[1].as_ref().unwrap();
        assert_eq!(
            (b.kind.as_str(), b.show.as_slice()),
            ("ramen", &["noodles".to_string()][..])
        );
        assert!(r[2].is_none(), "unknown id");
        assert_eq!(r[3].as_ref().unwrap().kind, "green-salad");
    }

    #[test]
    fn missing_lines_and_out_of_range_are_none() {
        let dishes = vec![dish("A", &["egg"]), dish("B", &["egg"])];
        let r = parse_art("2 | ramen | egg\n7 | ramen | egg", &dishes, &opts());
        assert!(r[0].is_none());
        assert_eq!(r[1].as_ref().unwrap().show, ["egg"]);
    }
}
