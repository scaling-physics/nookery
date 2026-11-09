//! Recipes from what's in stock, written by a local model (Ollama; default
//! `qwen3-vl:8b-instruct`, the model photos already keep loaded, so no
//! second model has to load).
//!
//! The model only writes ideas and steps. What you HAVE and what you'd
//! NEED is decided here, against the ledger: the model's "uses" list is
//! checked item by item (`nookery_core::names_match`), and anything not in
//! stock (it claimed "egg" for a frittata when there were none) moves to
//! "needs". Expired lots are never offered as ingredients; they come back
//! as a warning.
//!
//! Measured (M5, 2026-11-09, real stock of ~35 items): 4 ideas in ~14 s
//! (275 prompt tokens, ~310 generated).

use chrono::NaiveDate;
use nookery_core::{use_first, Location, Lot, LotId, Status};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::time::{Duration, Instant};

/// How urgent a lot is, from `nookery_core::status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// Past its date: not offered as an ingredient.
    Expired,
    /// Use within `USE_SOON_DAYS`.
    UseSoon,
    Normal,
}

/// One in-stock lot as the recipe side sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockItem {
    pub lot: LotId,
    pub item: String,
    /// e.g. "500g", "2".
    pub left: String,
    pub location: Location,
    pub urgency: Urgency,
    pub opened: bool,
}

/// In-stock lots, most urgent first (the `use_first` order).
pub fn stock_from<'a>(lots: impl IntoIterator<Item = &'a Lot>, today: NaiveDate) -> Vec<StockItem> {
    use_first(lots.into_iter().filter(|l| l.in_stock()), today)
        .into_iter()
        .map(|(l, st)| StockItem {
            lot: l.id,
            item: l.item.clone(),
            left: l.left.to_string(),
            location: l.location,
            urgency: match st {
                Status::Expired(_) => Urgency::Expired,
                Status::UseSoon(_) => Urgency::UseSoon,
                _ => Urgency::Normal,
            },
            opened: l.opened_at.is_some(),
        })
        .collect()
}

/// Assumed in every kitchen; never listed as missing.
pub const STAPLES: &[&str] = &[
    "water",
    "oil",
    "olive oil",
    "salt",
    "pepper",
    "black pepper",
];

/// What the user asked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// The template's text ("Dinner for two, ready in 30 minutes...").
    pub template: String,
    /// Anything typed on top ("something Indian", "no chilli").
    pub note: String,
    /// How many ideas (1-8).
    pub count: u32,
}

/// The prompt for `count` ideas: stock grouped as USE SOON / FRIDGE /
/// FREEZER / PANTRY (expired lots left out), the request, and the exact
/// answer format `parse_ideas` reads. No concrete example recipes (local
/// models copy examples).
pub fn ideas_prompt(stock: &[StockItem], request: &Request) -> String {
    let count = request.count.clamp(1, 8);
    // Without use-soon items the model still claimed to use some (real run,
    // 2026-11-09), and with a pantry-only stock it put a KitKat in a
    // stir-fry: say both plainly.
    let any_soon = stock.iter().any(|s| s.urgency == Urgency::UseSoon);
    let prefer = if any_soon {
        "Prefer recipes that use the USE SOON items."
    } else {
        "Nothing needs using up urgently."
    };
    let why = if any_soon {
        "what makes it good, which use-soon items it uses"
    } else {
        "what makes it good"
    };
    let mut p = format!(
        "You are a creative home cook. Suggest {count} recipes I can cook mostly from what I have.\n\
{prefer} Each recipe may need at most 3 things I don't have.\n\
Only combine ingredients that really go together; you don't have to use everything.\n\
Assume I have water, oil, salt and pepper.\n\nWhat I have:\n"
    );
    p.push_str(&stock_lines(stock));
    let (t, n) = (request.template.trim(), request.note.trim());
    let want = match (t.is_empty(), n.is_empty()) {
        (false, false) => format!("{t} — {n}"),
        (false, true) => t.to_string(),
        (true, false) => n.to_string(),
        (true, true) => "Something tasty and simple".to_string(),
    };
    p.push_str(&format!(
        "\nWhat I want: {want}\n\n\
Answer in exactly this format for each recipe, nothing else:\n\
### <recipe name>\n\
why: <one sentence: {why}>\n\
time: <minutes>\n\
uses: <comma-separated ingredients I have>\n\
needs: <comma-separated ingredients I don't have, or none>\n"
    ));
    p
}

/// One idea as the model wrote it (unchecked).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Idea {
    pub name: String,
    pub why: String,
    pub minutes: Option<u32>,
    /// Ingredients the model says we have.
    pub uses: Vec<String>,
    /// Ingredients the model says we'd need.
    pub needs: Vec<String>,
}

/// Reads `### name` / `why:` / `time:` / `uses:` / `needs:` blocks; robust
/// to bold markers, numbering, "25 min", "none", missing fields.
pub fn parse_ideas(answer: &str) -> Vec<Idea> {
    parse_ideas_impl(answer)
}

/// An idea checked against the stock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub idea: Idea,
    /// `(ingredient as written, lot)` for ingredients really in stock.
    pub have: Vec<(String, LotId)>,
    /// Ingredients not in stock (the model's `needs` plus `uses` it got
    /// wrong), staples removed, de-duplicated.
    pub need: Vec<String>,
    /// How many `have` lots are use-soon.
    pub uses_soon: u32,
    /// Expired lots the model wanted to use: warn, don't count.
    pub expired: Vec<LotId>,
    /// Higher is better: use-soon items used, little to buy.
    pub score: i32,
}

/// Checks each idea against the stock and sorts best first:
/// `score = 3 * uses_soon + have.len() - 2 * need.len()` (ties: fewer
/// minutes, then name).
pub fn check(ideas: Vec<Idea>, stock: &[StockItem]) -> Vec<Suggestion> {
    let mut out: Vec<Suggestion> = ideas.into_iter().map(|i| check_one(i, stock)).collect();
    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| {
                a.idea
                    .minutes
                    .unwrap_or(u32::MAX)
                    .cmp(&b.idea.minutes.unwrap_or(u32::MAX))
            })
            .then_with(|| a.idea.name.cmp(&b.idea.name))
    });
    out
}

/// The prompt for the full recipe of one suggestion: ingredients with
/// amounts (using what's in stock, amounts within what's left), numbered
/// steps; format `parse_recipe` reads.
pub fn recipe_prompt(suggestion: &Suggestion, stock: &[StockItem], request: &Request) -> String {
    let mut p = String::from("You are a careful home cook. Write one complete recipe.\n\n");
    p.push_str(&format!("Recipe: {}\n", suggestion.idea.name));
    if !suggestion.idea.why.is_empty() {
        p.push_str(&format!("Idea: {}\n", suggestion.idea.why));
    }
    let (t, n) = (request.template.trim(), request.note.trim());
    if !t.is_empty() || !n.is_empty() {
        p.push_str(&format!("What I want: {t}"));
        if !n.is_empty() {
            p.push_str(&format!("{}{n}", if t.is_empty() { "" } else { " — " }));
        }
        p.push('\n');
    }
    p.push_str("\nIngredients I have, and how much is left (do not use more than this):\n");
    let mut any = false;
    for (name, lot) in &suggestion.have {
        any = true;
        match stock.iter().find(|s| s.lot == *lot) {
            Some(s) => p.push_str(&format!("- {name}: {} available\n", s.left)),
            None => p.push_str(&format!("- {name}\n")),
        }
    }
    if !any {
        p.push_str("- (nothing specific)\n");
    }
    p.push_str("\nIngredients I will buy (use them freely):\n");
    if suggestion.need.is_empty() {
        p.push_str("- none\n");
    }
    for n in &suggestion.need {
        p.push_str(&format!("- {n}\n"));
    }
    p.push_str(
        "\nAssume I have water, oil, salt and pepper. Use at most about 10 steps.\n\
Answer in exactly this format, nothing else:\n\
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub title: String,
    pub serves: Option<u32>,
    pub minutes: Option<u32>,
    /// "200 g spinach", one per line.
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    /// The model's whole answer, for display if parsing missed something.
    pub raw: String,
}

/// Reads the full-recipe format; anything unparsed stays in `raw`.
pub fn parse_recipe(answer: &str) -> Recipe {
    parse_recipe_impl(answer)
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("the recipe model isn't reachable at {0} (is Ollama running?)")]
    Unreachable(String),
    #[error("the recipe model {0} isn't installed (ollama pull {0})")]
    ModelMissing(String),
    #[error("the recipe model failed: {0}")]
    Model(String),
    #[error("nothing in stock to cook with")]
    NoStock,
    /// A cloud model was asked for but Ollama isn't signed in (HTTP 401).
    #[error("cloud models need an Ollama account: run `ollama signin`")]
    SignIn,
    /// Signed in, but the account can't pay for this cloud model (HTTP 402).
    #[error("your Ollama account has no credits for this cloud model (signed in, but it isn't free): add credits at https://ollama.com/plans, or use the model on this Mac")]
    NoCredits,
}

/// A local text model. Implementations must not send anything off the
/// machine.
pub trait TextModel: Send + Sync {
    /// e.g. "ollama qwen3-vl:8b-instruct".
    fn name(&self) -> String;
    /// Generates up to `max_tokens`, calling `on_text` with each streamed
    /// piece; returns the whole answer.
    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError>;
}

/// Ollama on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OllamaText {
    /// Default `http://127.0.0.1:11434`, or `$NOOKERY_OLLAMA_URL`.
    pub url: String,
    /// Default `qwen3-vl:8b-instruct`, or `$NOOKERY_TEXT_MODEL`.
    pub model: String,
}

impl OllamaText {
    pub fn from_env() -> Self {
        let get = |k: &str, d: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| d.to_string())
        };
        OllamaText {
            url: get("NOOKERY_OLLAMA_URL", "http://127.0.0.1:11434"),
            model: get("NOOKERY_TEXT_MODEL", "qwen3-vl:8b-instruct"),
        }
    }
}

impl TextModel for OllamaText {
    fn name(&self) -> String {
        format!("ollama {}", self.model)
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(2)))
            .timeout_per_read(Some(Duration::from_secs(120)))
            .http_status_as_error(false)
            .build()
            .into();
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": true,
            "think": false,
            "options": {"temperature": TEMPERATURE, "num_ctx": 4096, "num_predict": max_tokens},
        });
        let url = self.url.trim_end_matches('/');
        let mut resp = agent
            .post(&format!("{url}/api/generate"))
            .send_json(&body)
            .map_err(|e| transport_error(&self.url, e))?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.body_mut().read_to_string().unwrap_or_default();
            return Err(model_error(&self.model, &format!("HTTP {status}: {text}")));
        }
        let mut reader = BufReader::new(resp.into_body().into_reader());
        let mut all = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .map_err(|e| RecipeError::Model(format!("reading the answer: {e}")))?;
            if n == 0 {
                break;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if let Some(err) = v["error"].as_str() {
                return Err(model_error(&self.model, err));
            }
            if let Some(t) = v["response"].as_str().filter(|t| !t.is_empty()) {
                all.push_str(t);
                on_text(t);
            }
            if v["done"].as_bool() == Some(true) {
                break;
            }
        }
        Ok(all)
    }
}

/// Tries `primary` (this Mac); when it is missing or fails, `secondary`
/// (the cloud model) answers. A failure after text was already streamed
/// is not retried (the user has seen half an answer). `Unreachable` is not
/// retried either: cloud models go through the same local Ollama.
pub struct FallbackText<A, B> {
    pub primary: A,
    pub secondary: B,
    fell_back: std::sync::atomic::AtomicBool,
}

impl<A: TextModel, B: TextModel> FallbackText<A, B> {
    pub fn new(primary: A, secondary: B) -> Self {
        Self {
            primary,
            secondary,
            fell_back: false.into(),
        }
    }

    /// Whether `secondary` answered.
    pub fn fell_back(&self) -> bool {
        self.fell_back.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl<A: TextModel, B: TextModel> TextModel for FallbackText<A, B> {
    fn name(&self) -> String {
        format!(
            "{} (cloud fallback: {})",
            self.primary.name(),
            self.secondary.name()
        )
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        let mut streamed = false;
        let first = self.primary.generate(prompt, max_tokens, &mut |t| {
            streamed = true;
            on_text(t)
        });
        match first {
            Err(RecipeError::ModelMissing(_) | RecipeError::Model(_)) if !streamed => {
                self.fell_back
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.secondary.generate(prompt, max_tokens, on_text)
            }
            other => other,
        }
    }
}

/// The text model for the user's setting.
pub enum ChosenText {
    Local(OllamaText),
    Cloud(OllamaText),
    Fallback(FallbackText<OllamaText, OllamaText>),
}

impl ChosenText {
    /// `OllamaText::from_env()` on this Mac, `cloud_model` through the same
    /// Ollama for the cloud.
    pub fn for_mode(mode: nookery_core::AiMode, cloud_model: &str) -> Self {
        let local = OllamaText::from_env();
        let cloud = OllamaText {
            model: cloud_model.to_string(),
            ..local.clone()
        };
        match mode {
            nookery_core::AiMode::Local => ChosenText::Local(local),
            nookery_core::AiMode::Cloud => ChosenText::Cloud(cloud),
            nookery_core::AiMode::Fallback => ChosenText::Fallback(FallbackText::new(local, cloud)),
        }
    }

    /// Which model answered, for the user. Call after generating.
    pub fn answered_by(&self) -> String {
        match self {
            ChosenText::Local(m) => format!("{} (this Mac)", m.name()),
            ChosenText::Cloud(m) => format!("{} (cloud)", m.name()),
            ChosenText::Fallback(f) if f.fell_back() => format!(
                "{} (cloud, because the model on this Mac failed)",
                f.secondary.name()
            ),
            ChosenText::Fallback(f) => format!("{} (this Mac)", f.primary.name()),
        }
    }

    pub fn used_cloud(&self) -> bool {
        match self {
            ChosenText::Local(_) => false,
            ChosenText::Cloud(_) => true,
            ChosenText::Fallback(f) => f.fell_back(),
        }
    }
}

impl TextModel for ChosenText {
    fn name(&self) -> String {
        match self {
            ChosenText::Local(m) | ChosenText::Cloud(m) => m.name(),
            ChosenText::Fallback(f) => f.name(),
        }
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        match self {
            ChosenText::Local(m) | ChosenText::Cloud(m) => m.generate(prompt, max_tokens, on_text),
            ChosenText::Fallback(f) => f.generate(prompt, max_tokens, on_text),
        }
    }
}

/// Checked, sorted suggestions and how long the model took.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Suggestions {
    pub items: Vec<Suggestion>,
    pub model: String,
    pub ms: f64,
}

/// Asks for ideas and checks them. `NoStock` if nothing usable is in stock.
pub fn suggest(
    model: &dyn TextModel,
    stock: &[StockItem],
    request: &Request,
) -> Result<Suggestions, RecipeError> {
    if !stock.iter().any(|s| s.urgency != Urgency::Expired) {
        return Err(RecipeError::NoStock);
    }
    let t0 = Instant::now();
    let answer = model.generate(&ideas_prompt(stock, request), 700, &mut |_| {})?;
    let items = check(parse_ideas(&answer), stock);
    Ok(Suggestions {
        items,
        model: model.name(),
        ms: t0.elapsed().as_secs_f64() * 1e3,
    })
}

/// Writes the full recipe, streaming the text to `on_text` as it comes.
pub fn write_recipe(
    model: &dyn TextModel,
    suggestion: &Suggestion,
    stock: &[StockItem],
    request: &Request,
    on_text: &mut dyn FnMut(&str),
) -> Result<Recipe, RecipeError> {
    let answer = model.generate(&recipe_prompt(suggestion, stock, request), 900, on_text)?;
    Ok(parse_recipe(&answer))
}

mod helpers;
use helpers::*;

#[cfg(test)]
mod tests;
