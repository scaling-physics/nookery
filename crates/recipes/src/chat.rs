//! Talking to the cook about a recipe: questions ("Can I freeze it?") and
//! changes ("make it vegetarian"). The model answers in plain words and, only
//! when the recipe should change, adds the whole revised recipe in the same
//! format `parse_recipe` reads.

use crate::helpers::stock_lines;
use crate::*;

/// Turns of history kept in the prompt (a 4096-token phone model must fit
/// recipe + stock + history + answer).
pub const MAX_HISTORY: usize = 6;
/// Longest history turn kept, in characters.
const MAX_TURN_CHARS: usize = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    Cook,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: ChatRole,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatAnswer {
    /// What the cook said (empty if the model only sent a recipe).
    pub reply: String,
    /// The whole revised recipe, when the model changed it.
    pub recipe: Option<Recipe>,
}

impl Recipe {
    /// The recipe in the format `parse_recipe` reads.
    pub fn to_text(&self) -> String {
        let mut s = format!("# {}\n", self.title);
        if let Some(n) = self.serves {
            s.push_str(&format!("Serves: {n}\n"));
        }
        if let Some(m) = self.minutes {
            s.push_str(&format!("Time: {m} min\n"));
        }
        s.push_str("## Ingredients\n");
        for i in &self.ingredients {
            s.push_str(&format!("- {i}\n"));
        }
        s.push_str("## Steps\n");
        for (n, st) in self.steps.iter().enumerate() {
            s.push_str(&format!("{}. {st}\n", n + 1));
        }
        s
    }
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

/// The prompt for one chat turn. Expired lots are left out of the stock
/// (as in `ideas_prompt`); only the last `MAX_HISTORY` turns are kept.
pub fn chat_prompt(
    recipe: &Recipe,
    stock: &[StockItem],
    history: &[ChatTurn],
    message: &str,
) -> String {
    let mut p = String::from(
        "You are a friendly, practical home cook helping me with the recipe below. \
Answer in plain, short sentences.\n\n",
    );
    p.push_str("The recipe:\n");
    p.push_str(&recipe.to_text());
    p.push_str("\nWhat I have in the kitchen (suggest swaps from this when it helps):\n");
    let lines = stock_lines(stock);
    if lines.is_empty() {
        p.push_str("(nothing recorded)\n");
    } else {
        p.push_str(&lines);
    }
    p.push_str("I always have water, oil, salt and pepper.\n");
    let start = history.len().saturating_sub(MAX_HISTORY);
    if history.len() > start {
        p.push_str("\nOur conversation so far:\n");
        for t in &history[start..] {
            let who = match t.role {
                ChatRole::User => "Me",
                ChatRole::Cook => "You",
            };
            p.push_str(&format!("{who}: {}\n", clip(&t.text, MAX_TURN_CHARS)));
        }
    }
    p.push_str(&format!("\nMy message: {}\n\n", clip(message, 1000)));
    p.push_str(
        "If I only asked a question, just answer it. Do NOT write the recipe again.\n\
If I asked for a change (vegetarian, no oven, spicier, more people, a swap, using something else), \
write one or two sentences saying what you changed, then a line with only ---, \
then the WHOLE revised recipe in exactly this format:\n\
# <title>\n\
serves: <number of people>\n\
time: <minutes>\n\
## Ingredients\n\
- <amount> <ingredient>\n\
## Steps\n\
1. <step>\n",
    );
    p
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    t.len() >= 3
        && t.chars()
            .all(|c| c == '-' || c == '*' || c == '_' || c == '=')
}

/// Splits the model's answer into the reply and an optional revised recipe.
/// A recipe counts only if it has ingredients and steps; otherwise the whole
/// answer is the reply.
pub fn parse_chat(answer: &str) -> ChatAnswer {
    let lines: Vec<&str> = answer.lines().collect();
    let plain = |l: &str| l.replace(['*', '_'], "").trim().to_string();
    // Where the recipe starts: a one-# title line, else the first section header.
    let title_at = lines.iter().position(|l| {
        let p = plain(l);
        p.starts_with('#') && !p.starts_with("##")
    });
    let section_at = lines.iter().position(|l| {
        let p = plain(l).trim_start_matches('#').trim().to_lowercase();
        matches!(
            p.trim_end_matches(':'),
            "ingredients" | "steps" | "method" | "directions" | "instructions"
        )
    });
    let start = match (title_at, section_at) {
        (Some(t), Some(s)) => Some(t.min(s)),
        (t, s) => t.or(s),
    };
    let reply_only = || ChatAnswer {
        reply: clean_reply(answer),
        recipe: None,
    };
    let Some(start) = start else {
        return reply_only();
    };
    let recipe_text = lines[start..].join("\n");
    let recipe = parse_recipe(&recipe_text);
    if recipe.ingredients.is_empty() || recipe.steps.is_empty() {
        return reply_only();
    }
    ChatAnswer {
        reply: clean_reply(&lines[..start].join("\n")),
        recipe: Some(recipe),
    }
}

/// Trims, drops trailing rules ("---") and a leading "reply:" label.
fn clean_reply(s: &str) -> String {
    let s = s.replace("**", "");
    let mut lines: Vec<&str> = s.trim().lines().collect();
    loop {
        match lines.last() {
            Some(l) if l.trim().is_empty() || is_rule(l) => {}
            // "Here is the revised recipe:" introduces what was cut off.
            Some(l) if l.trim_end().ends_with(':') && l.to_lowercase().contains("recipe") => {}
            _ => break,
        }
        lines.pop();
    }
    let joined = lines.join("\n");
    let t = joined.trim();
    let lower = t.to_lowercase();
    for label in ["reply:", "answer:"] {
        if lower.starts_with(label) {
            return t[label.len()..].trim().to_string();
        }
    }
    t.to_string()
}

/// One chat turn: asks the model (streaming to `on_text`) and splits the
/// answer.
pub fn chat(
    model: &dyn TextModel,
    recipe: &Recipe,
    stock: &[StockItem],
    history: &[ChatTurn],
    message: &str,
    on_text: &mut dyn FnMut(&str),
) -> Result<ChatAnswer, RecipeError> {
    let answer = model.generate(&chat_prompt(recipe, stock, history, message), 1000, on_text)?;
    Ok(parse_chat(&answer))
}
