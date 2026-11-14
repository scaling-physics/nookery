//! Recipes tab: prompt templates and recipe suggestions.
//!
//! `suggest_recipes` locks the store only to read the stock and the template,
//! runs the local text model on a blocking thread with the store unlocked,
//! then keeps the checked suggestions, the request and the stock in
//! `AppState` under an id. `write_recipe` looks them up by that id and streams
//! the model's text through a channel, so the model is never asked twice for
//! the same ideas.
//!
//! Template ids are `u32` here (specta can't export `i64` as a number).
//! Errors: `BadInput("name" | "body" | "ids" | "stock" | "index")`.

use chrono::{Local, NaiveDate, Utc};
use nookery_core::{category, status, AiMode, LotId};
use nookery_recipes::{
    chat as chat_text, pick_art, stock_from, suggest, write_recipe as write_recipe_text, ArtDish,
    ArtPick, ChatRole, ChatTurn, Recipe, RecipeError, Request, StockItem, Suggestion, Suggestions,
    TextModel,
};
use nookery_store::{Store, StoreError, SETTING_RECIPES_AI};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::Channel;
use tauri::State;

use super::phone::Writer;
use super::{bad, view, AppState, CommandError, FoodCategoryView, LotView};

/// Used when no template is chosen.
const DEFAULT_TEMPLATE: &str = "Dinner for two, mostly from what I have.";

// ---- templates ----

#[derive(Debug, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateView {
    pub id: u32,
    pub name: String,
    pub body: String,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveTemplateInput {
    /// None: a new template (added at the end).
    pub id: Option<u32>,
    pub name: String,
    pub body: String,
}

fn template_view(t: nookery_store::Template) -> TemplateView {
    TemplateView {
        id: t.id as u32,
        name: t.name,
        body: t.body,
    }
}

fn template_error(e: StoreError, name: &str) -> CommandError {
    match e {
        StoreError::DuplicateTemplate(_) => bad("name"),
        StoreError::EmptyTemplate => bad(if name.trim().is_empty() {
            "name"
        } else {
            "body"
        }),
        StoreError::NoTemplate(_) => CommandError::NotFound,
        other => other.into(),
    }
}

pub fn list_templates_in(store: &Store) -> Result<Vec<TemplateView>, CommandError> {
    Ok(store.templates()?.into_iter().map(template_view).collect())
}

pub fn save_template_in(
    store: &mut Store,
    input: &SaveTemplateInput,
) -> Result<TemplateView, CommandError> {
    store
        .save_template(
            input.id.map(i64::from),
            &input.name,
            &input.body,
            Utc::now(),
        )
        .map(template_view)
        .map_err(|e| template_error(e, &input.name))
}

pub fn delete_template_in(store: &mut Store, id: u32) -> Result<(), CommandError> {
    store
        .delete_template(i64::from(id))
        .map_err(|e| template_error(e, "x"))
}

pub fn order_templates_in(store: &mut Store, ids: &[u32]) -> Result<(), CommandError> {
    let ids: Vec<i64> = ids.iter().map(|i| i64::from(*i)).collect();
    store.order_templates(&ids).map_err(|e| match e {
        // Not exactly the current set of templates.
        StoreError::NoTemplate(_) => CommandError::NotFound,
        other => template_error(other, "x"),
    })
}

#[tauri::command]
#[specta::specta]
pub fn list_templates(state: State<'_, AppState>) -> Result<Vec<TemplateView>, CommandError> {
    let store = state.store()?;
    list_templates_in(&store)
}

#[tauri::command]
#[specta::specta]
pub fn save_template(
    state: State<'_, AppState>,
    input: SaveTemplateInput,
) -> Result<TemplateView, CommandError> {
    let mut store = state.store()?;
    save_template_in(&mut store, &input)
}

#[tauri::command]
#[specta::specta]
pub fn delete_template(state: State<'_, AppState>, id: u32) -> Result<(), CommandError> {
    let mut store = state.store()?;
    delete_template_in(&mut store, id)
}

#[tauri::command]
#[specta::specta]
pub fn order_templates(state: State<'_, AppState>, ids: Vec<u32>) -> Result<(), CommandError> {
    let mut store = state.store()?;
    order_templates_in(&mut store, &ids)
}

// ---- suggestions ----

/// The last suggestions, kept until replaced (`write_recipe` reads them).
pub struct StoredSuggestions {
    id: String,
    suggestions: Suggestions,
    request: Request,
    stock: Vec<StockItem>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HaveView {
    /// The ingredient as the model wrote it.
    pub ingredient: String,
    pub lot: LotView,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionView {
    /// The id to answer with in `write_recipe`.
    pub index: u32,
    pub name: String,
    pub why: String,
    pub minutes: Option<u32>,
    pub have: Vec<HaveView>,
    pub need: Vec<String>,
    /// The food category of each `need` entry (same order), for icons and
    /// illustrations.
    pub need_categories: Vec<FoodCategoryView>,
    /// Expired lots the idea wanted: a warning, not an ingredient.
    pub expired: Vec<LotView>,
    pub uses_soon: u32,
    pub score: i32,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionsView {
    pub id: String,
    pub model: String,
    pub used_cloud: bool,
    pub ms: f64,
    pub items: Vec<SuggestionView>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SuggestInput {
    pub template_id: Option<u32>,
    pub note: String,
    pub count: u32,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WriteRecipeInput {
    pub suggestions_id: String,
    pub index: u32,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeView {
    pub title: String,
    pub serves: Option<u32>,
    pub minutes: Option<u32>,
    pub ingredients: Vec<String>,
    /// The food category of each ingredient line (same order).
    pub ingredient_categories: Vec<FoodCategoryView>,
    pub steps: Vec<String>,
    /// The model's whole answer, for display if parsing missed something.
    pub raw: String,
    /// Which model answered, e.g. "ollama x (cloud)".
    pub model: String,
    pub used_cloud: bool,
    pub ms: f64,
}

impl From<RecipeError> for CommandError {
    fn from(e: RecipeError) -> Self {
        match e {
            RecipeError::Unreachable(url) => Self::RecipeModelUnavailable(url),
            RecipeError::ModelMissing(model) => Self::RecipeModelMissing(model),
            RecipeError::NoStock => bad("stock"),
            RecipeError::SignIn => Self::CloudSignIn,
            RecipeError::NoCredits => Self::CloudCredits,
            RecipeError::Model(m) => {
                eprintln!("nookery: recipe model: {m}");
                Self::Internal
            }
        }
    }
}

/// Reads the stock and the request (the only part that needs the store).
pub fn prepare_in(
    store: &Store,
    input: &SuggestInput,
    today: NaiveDate,
) -> Result<(Vec<StockItem>, Request), CommandError> {
    let template = match input.template_id {
        None => DEFAULT_TEMPLATE.to_string(),
        Some(id) => {
            store
                .templates()?
                .into_iter()
                .find(|t| t.id == i64::from(id))
                .ok_or(CommandError::NotFound)?
                .body
        }
    };
    let stock = stock_from(store.inventory().in_stock(), today);
    let request = Request {
        template,
        note: input.note.trim().to_string(),
        count: input.count.clamp(1, 8),
    };
    Ok((stock, request))
}

/// Turns checked suggestions into what the UI shows, looking lots up in
/// the ledger (a lot that has since gone is left out).
pub fn suggestions_view(
    store: &Store,
    s: &Suggestions,
    today: NaiveDate,
    id: &str,
) -> SuggestionsView {
    let inv = store.inventory();
    let lot_view = |id: &LotId| inv.get(*id).map(|l| view(l, status(l, today)));
    let items = s
        .items
        .iter()
        .enumerate()
        .map(|(i, sg)| SuggestionView {
            index: i as u32,
            name: sg.idea.name.clone(),
            why: sg.idea.why.clone(),
            minutes: sg.idea.minutes,
            have: sg
                .have
                .iter()
                .filter_map(|(ing, lot)| {
                    lot_view(lot).map(|lot| HaveView {
                        ingredient: ing.clone(),
                        lot,
                    })
                })
                .collect(),
            need: sg.need.clone(),
            need_categories: sg.need.iter().map(|n| category(n).into()).collect(),
            expired: sg.expired.iter().filter_map(lot_view).collect(),
            uses_soon: sg.uses_soon,
            score: sg.score,
        })
        .collect();
    SuggestionsView {
        id: id.into(),
        model: s.model.clone(),
        used_cloud: false,
        ms: s.ms,
        items,
    }
}

/// Writes one recipe, streaming text to `on_text`. Touches no store.
pub fn write_in(
    model: &dyn TextModel,
    stored: &StoredSuggestions,
    index: u32,
    on_text: &mut dyn FnMut(&str),
) -> Result<RecipeView, CommandError> {
    let suggestion: &Suggestion = stored
        .suggestions
        .items
        .get(index as usize)
        .ok_or_else(|| bad("index"))?;
    let t = std::time::Instant::now();
    let r = write_recipe_text(model, suggestion, &stored.stock, &stored.request, on_text)?;
    Ok(RecipeView {
        title: r.title,
        serves: r.serves,
        minutes: r.minutes,
        ingredient_categories: r.ingredients.iter().map(|n| category(n).into()).collect(),
        ingredients: r.ingredients,
        steps: r.steps,
        raw: r.raw,
        model: model.name(),
        used_cloud: false,
        ms: t.elapsed().as_secs_f64() * 1e3,
    })
}

#[cfg(test)]
/// Prepare, suggest and review in one go (the command does the steps
/// separately so the store isn't locked while the model runs).
pub fn suggest_in(
    model: &dyn TextModel,
    store: &Store,
    input: &SuggestInput,
    today: NaiveDate,
) -> Result<(StoredSuggestions, SuggestionsView), CommandError> {
    let (stock, request) = prepare_in(store, input, today)?;
    let suggestions = suggest(model, &stock, &request)?;
    let id = LotId::new().to_string();
    let v = suggestions_view(store, &suggestions, today, &id);
    Ok((
        StoredSuggestions {
            id,
            suggestions,
            request,
            stock,
        },
        v,
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn suggest_recipes(
    state: State<'_, AppState>,
    input: SuggestInput,
) -> Result<SuggestionsView, CommandError> {
    let today = Local::now().date_naive();
    let (stock, request, mode, cloud_model) = {
        let store = state.store()?;
        let (stock, request) = prepare_in(&store, &input, today)?;
        (
            stock,
            request,
            store.ai_mode(SETTING_RECIPES_AI)?,
            store.cloud_model()?,
        )
    };
    // The model runs on a blocking thread and with the store unlocked.
    let model = Writer::for_state(&state, mode, &cloud_model)?;
    let (mut suggestions, stock, request, answered_by, used_cloud) =
        tauri::async_runtime::spawn_blocking(move || {
            let s = suggest(&model, &stock, &request)?;
            Ok::<_, RecipeError>((s, stock, request, model.answered_by(), model.used_cloud()))
        })
        .await
        .map_err(|_| CommandError::Internal)??;
    let id = LotId::new().to_string();
    suggestions.model = answered_by;
    let mut v = {
        let store = state.store()?;
        suggestions_view(&store, &suggestions, today, &id)
    };
    v.used_cloud = used_cloud;
    *state
        .last_suggestions
        .lock()
        .map_err(|_| CommandError::Internal)? = Some(StoredSuggestions {
        id,
        suggestions,
        request,
        stock,
    });
    Ok(v)
}

#[tauri::command]
#[specta::specta]
pub async fn write_recipe(
    state: State<'_, AppState>,
    input: WriteRecipeInput,
    on_text: Channel<String>,
) -> Result<RecipeView, CommandError> {
    let stored = {
        let last = state
            .last_suggestions
            .lock()
            .map_err(|_| CommandError::Internal)?;
        let s = last
            .as_ref()
            .filter(|s| s.id == input.suggestions_id)
            .ok_or(CommandError::NotFound)?;
        // Cloned so the lock isn't held while the model runs.
        StoredSuggestions {
            id: s.id.clone(),
            suggestions: s.suggestions.clone(),
            request: s.request.clone(),
            stock: s.stock.clone(),
        }
    };
    let (mode, cloud_model) = {
        let store = state.store()?;
        (store.ai_mode(SETTING_RECIPES_AI)?, store.cloud_model()?)
    };
    let model = Writer::for_state(&state, mode, &cloud_model)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut r = write_in(&model, &stored, input.index, &mut |t| {
            let _ = on_text.send(t.to_string());
        })?;
        r.model = model.answered_by();
        r.used_cloud = model.used_cloud();
        Ok(r)
    })
    .await
    .map_err(|_| CommandError::Internal)?
}

// ---- chat about a recipe ----

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatRecipeInput {
    pub title: String,
    pub serves: Option<u32>,
    pub minutes: Option<u32>,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ChatRoleView {
    User,
    Cook,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurnInput {
    pub role: ChatRoleView,
    pub text: String,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeChatInput {
    /// The recipe as it is now (a suggestion's or a saved favourite's).
    pub recipe: ChatRecipeInput,
    pub history: Vec<ChatTurnInput>,
    pub message: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeChatView {
    pub reply: String,
    /// The whole revised recipe, when the cook changed it.
    pub recipe: Option<RecipeView>,
    pub model: String,
    pub used_cloud: bool,
    pub ms: f64,
}

/// One chat turn against `model`. Touches no store.
pub fn chat_in(
    model: &dyn TextModel,
    stock: &[StockItem],
    input: &RecipeChatInput,
    on_text: &mut dyn FnMut(&str),
) -> Result<RecipeChatView, CommandError> {
    let message = input.message.trim();
    if message.is_empty() {
        return Err(bad("message"));
    }
    let recipe = Recipe {
        title: input.recipe.title.clone(),
        serves: input.recipe.serves,
        minutes: input.recipe.minutes,
        ingredients: input.recipe.ingredients.clone(),
        steps: input.recipe.steps.clone(),
        raw: String::new(),
    };
    let history: Vec<ChatTurn> = input
        .history
        .iter()
        .map(|t| ChatTurn {
            role: match t.role {
                ChatRoleView::User => ChatRole::User,
                ChatRoleView::Cook => ChatRole::Cook,
            },
            text: t.text.clone(),
        })
        .collect();
    let t = std::time::Instant::now();
    let a = chat_text(model, &recipe, stock, &history, message, on_text)?;
    let revised = a.recipe.is_some();
    let recipe = a.recipe.map(|mut r| {
        if r.title.trim().is_empty() {
            r.title = recipe.title.clone();
        }
        RecipeView {
            title: r.title,
            serves: r.serves,
            minutes: r.minutes,
            ingredient_categories: r.ingredients.iter().map(|n| category(n).into()).collect(),
            ingredients: r.ingredients,
            steps: r.steps,
            raw: r.raw,
            model: model.name(),
            used_cloud: false,
            ms: 0.0,
        }
    });
    let reply = if a.reply.is_empty() && revised {
        "Done, I updated the recipe.".to_string()
    } else {
        a.reply
    };
    Ok(RecipeChatView {
        reply,
        recipe,
        model: model.name(),
        used_cloud: false,
        ms: t.elapsed().as_secs_f64() * 1e3,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn recipe_chat(
    state: State<'_, AppState>,
    input: RecipeChatInput,
    on_text: Channel<String>,
) -> Result<RecipeChatView, CommandError> {
    let today = Local::now().date_naive();
    // Same model choice as `write_recipe`; the store is unlocked while it runs.
    let (stock, mode, cloud_model) = {
        let store = state.store()?;
        (
            stock_from(store.inventory().in_stock(), today),
            store.ai_mode(SETTING_RECIPES_AI)?,
            store.cloud_model()?,
        )
    };
    let model = Writer::for_state(&state, mode, &cloud_model)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut v = chat_in(&model, &stock, &input, &mut |t| {
            let _ = on_text.send(t.to_string());
        })?;
        v.model = model.answered_by();
        v.used_cloud = model.used_cloud();
        if let Some(r) = v.recipe.as_mut() {
            r.model = v.model.clone();
            r.used_cloud = v.used_cloud;
            r.ms = v.ms;
        }
        Ok(v)
    })
    .await
    .map_err(|_| CommandError::Internal)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Event, EventKind, Location, Qty, Source};
    use std::sync::Mutex;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    fn field(e: CommandError) -> String {
        match e {
            CommandError::BadInput(f) => f,
            other => panic!("{other:?}"),
        }
    }

    fn input(id: Option<u32>, name: &str, body: &str) -> SaveTemplateInput {
        SaveTemplateInput {
            id,
            name: name.into(),
            body: body.into(),
        }
    }

    #[test]
    fn templates_round_trip_and_errors_map() {
        let mut s = Store::open_in_memory().unwrap();
        let all = list_templates_in(&s).unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].name, "Use it up");

        let t = save_template_in(&mut s, &input(None, " Indian ", " Curry night ")).unwrap();
        assert_eq!(
            (t.name.as_str(), t.body.as_str()),
            ("Indian", "Curry night")
        );
        assert_eq!(list_templates_in(&s).unwrap().last().unwrap(), &t);
        let edited = save_template_in(&mut s, &input(Some(t.id), "Indian", "Dal")).unwrap();
        assert_eq!((edited.id, edited.body.as_str()), (t.id, "Dal"));

        let dup = save_template_in(&mut s, &input(None, "Indian", "x")).unwrap_err();
        assert_eq!(field(dup), "name");
        let no_name = save_template_in(&mut s, &input(None, " ", "x")).unwrap_err();
        assert_eq!(field(no_name), "name");
        let no_body = save_template_in(&mut s, &input(None, "Pasta", "  ")).unwrap_err();
        assert_eq!(field(no_body), "body");
        assert!(matches!(
            save_template_in(&mut s, &input(Some(999), "Ghost", "x")),
            Err(CommandError::NotFound)
        ));

        let mut ids: Vec<u32> = list_templates_in(&s)
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        ids.reverse();
        order_templates_in(&mut s, &ids).unwrap();
        let after: Vec<u32> = list_templates_in(&s)
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(after, ids);
        assert!(matches!(
            order_templates_in(&mut s, &ids[1..]),
            Err(CommandError::NotFound)
        ));

        delete_template_in(&mut s, t.id).unwrap();
        assert!(matches!(
            delete_template_in(&mut s, t.id),
            Err(CommandError::NotFound)
        ));
        assert_eq!(list_templates_in(&s).unwrap().len(), 5);
    }

    /// Answers the ideas prompt, then the recipe prompt, from a script.
    struct Fake(Mutex<Vec<Result<String, RecipeError>>>);

    impl TextModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn generate(
            &self,
            _: &str,
            _: u32,
            on_text: &mut dyn FnMut(&str),
        ) -> Result<String, RecipeError> {
            let answer = self.0.lock().unwrap().remove(0)?;
            for chunk in answer.as_bytes().chunks(10) {
                on_text(std::str::from_utf8(chunk).unwrap());
            }
            Ok(answer)
        }
    }

    fn stocked() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        for (item, bb) in [
            ("spinach", "2026-11-09"),
            ("eggs", "2026-11-20"),
            ("cream", "2026-11-08"),
        ] {
            s.append(&[Event {
                at: Utc::now(),
                lot: LotId::new(),
                kind: EventKind::Acquired {
                    item: item.into(),
                    qty: Qty::pieces(1),
                    location: Location::Fridge,
                    source: Source::Manual,
                    best_before: Some(bb.parse().unwrap()),
                },
            }])
            .unwrap();
        }
        s
    }

    const IDEAS: &str = "### Spinach omelette\nwhy: Uses the spinach.\ntime: 15\nuses: spinach, eggs, cream\nneeds: cheese, salt\n";
    const RECIPE: &str = "# Spinach omelette\nServes: 2\nTime: 15 min\n\n## Ingredients\n- 100 g spinach\n- 3 eggs\n\n## Steps\n1. Wilt the spinach.\n2. Add the eggs.\n";

    fn ask(count: u32) -> SuggestInput {
        SuggestInput {
            template_id: Some(1),
            note: " no chilli ".into(),
            count,
        }
    }

    #[test]
    fn suggests_and_writes_with_a_fake_model() {
        let store = stocked();
        let model = Fake(Mutex::new(vec![Ok(IDEAS.into()), Ok(RECIPE.into())]));
        let (stored, v) = suggest_in(&model, &store, &ask(99), today()).unwrap();
        assert_eq!(stored.request.count, 8);
        assert_eq!(stored.request.note, "no chilli");
        assert!(stored.request.template.contains("use-soon"));
        assert_eq!(v.model, "fake");
        assert!(!v.used_cloud);
        assert_eq!(v.items.len(), 1);
        let it = &v.items[0];
        assert_eq!(
            (it.index, it.name.as_str(), it.minutes),
            (0, "Spinach omelette", Some(15))
        );
        let have: Vec<&str> = it.have.iter().map(|h| h.lot.item.as_str()).collect();
        assert_eq!(have, ["spinach", "eggs"]);
        assert_eq!(it.uses_soon, 1);
        assert_eq!(it.expired.len(), 1);
        assert_eq!(it.expired[0].item, "cream");
        assert!(it.need.contains(&"cheese".to_string()));
        assert!(!it.need.contains(&"salt".to_string()));

        let mut streamed = String::new();
        let r = write_in(&model, &stored, 0, &mut |t| streamed.push_str(t)).unwrap();
        assert_eq!((r.model.as_str(), r.used_cloud), ("fake", false));
        assert_eq!(streamed, RECIPE);
        assert_eq!(r.raw, RECIPE);
        assert_eq!(r.title, "Spinach omelette");
        assert_eq!((r.serves, r.minutes), (Some(2), Some(15)));
        assert_eq!(r.steps.len(), 2);
        assert_eq!(r.ingredients.len(), 2);
        assert!(matches!(
            write_in(&model, &stored, 5, &mut |_| {}),
            Err(CommandError::BadInput(f)) if f == "index"
        ));
    }

    #[test]
    fn model_and_stock_errors_map() {
        let go = |store: &Store, model: Fake, template_id: Option<u32>| {
            let i = SuggestInput {
                template_id,
                ..ask(4)
            };
            suggest_in(&model, store, &i, today())
                .map(|_| ())
                .unwrap_err()
        };
        let none = |r: Result<String, RecipeError>| Fake(Mutex::new(vec![r]));
        let store = stocked();
        assert!(matches!(
            go(&store, none(Err(RecipeError::Unreachable("http://x".into()))), None),
            CommandError::RecipeModelUnavailable(u) if u == "http://x"
        ));
        assert!(matches!(
            go(&store, none(Err(RecipeError::ModelMissing("m:1".into()))), None),
            CommandError::RecipeModelMissing(m) if m == "m:1"
        ));
        assert!(matches!(
            go(&store, none(Err(RecipeError::Model("boom".into()))), None),
            CommandError::Internal
        ));
        assert!(matches!(
            go(&store, none(Ok(IDEAS.into())), Some(999)),
            CommandError::NotFound
        ));
        let empty = Store::open_in_memory().unwrap();
        let e = go(&empty, none(Ok(IDEAS.into())), None);
        assert_eq!(field(e), "stock");
    }

    #[test]
    fn chat_answers_questions_and_revises_recipes() {
        let input = |msg: &str| RecipeChatInput {
            recipe: ChatRecipeInput {
                title: "Spinach omelette".into(),
                serves: Some(2),
                minutes: Some(15),
                ingredients: vec!["3 eggs".into()],
                steps: vec!["Cook.".into()],
            },
            history: vec![ChatTurnInput {
                role: ChatRoleView::User,
                text: "hi".into(),
            }],
            message: msg.into(),
        };
        let stock = stock_from(stocked().inventory().in_stock(), today());
        let q = Fake(Mutex::new(vec![Ok("Yes, it freezes well.".into())]));
        let mut streamed = String::new();
        let v = chat_in(&q, &stock, &input("Can I freeze it?"), &mut |t| {
            streamed.push_str(t)
        })
        .unwrap();
        assert_eq!(v.reply, "Yes, it freezes well.");
        assert!(v.recipe.is_none());
        assert_eq!(streamed, "Yes, it freezes well.");

        let r = Fake(Mutex::new(vec![Ok(
            "# \n## Ingredients\n- 3 eggs\n- 1 zucchini\n## Steps\n1. Cook.\n".into(),
        )]));
        let v = chat_in(&r, &stock, &input("use the zucchini too"), &mut |_| {}).unwrap();
        let rec = v.recipe.unwrap();
        assert_eq!(rec.title, "Spinach omelette");
        assert_eq!(rec.ingredient_categories.len(), 2);
        assert!(v.reply.contains("updated"), "{:?}", v.reply);

        assert!(matches!(
            chat_in(&q, &stock, &input("  "), &mut |_| {}),
            Err(CommandError::BadInput(f)) if f == "message"
        ));
    }
}

// ---- recipe pictures ----

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtDishInput {
    pub title: String,
    pub ingredients: Vec<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtPickView {
    pub kind: String,
    pub show: Vec<String>,
}

/// Most dishes per request.
const MAX_ART_DISHES: usize = 8;

/// The picture choice with any model: pure, so it is tested without Tauri.
pub fn pick_dish_art_with(
    model: &dyn TextModel,
    dishes: &[ArtDishInput],
    options: &[ArtOption],
) -> Result<Vec<Option<ArtPickView>>, CommandError> {
    if dishes.len() > MAX_ART_DISHES {
        return Err(bad("dishes"));
    }
    let dishes: Vec<ArtDish> = dishes
        .iter()
        .map(|d| ArtDish {
            title: d.title.clone(),
            ingredients: d.ingredients.clone(),
        })
        .collect();
    let options: Vec<(String, String)> = options
        .iter()
        .map(|o| (o.id.clone(), o.label.clone()))
        .collect();
    Ok(pick_art(model, &dishes, &options)?
        .into_iter()
        .map(|p| p.map(|ArtPick { kind, show }| ArtPickView { kind, show }))
        .collect())
}

/// The recipe model picks the closest dish kind and the ingredients to show.
/// Always local (this Mac's Ollama, or on the phone the phone model / the
/// paired Mac), even when recipes are set to cloud: it is decoration, and
/// nothing is sent to the cloud without the user's choice. The UI falls back
/// to word matching on any error.
#[tauri::command]
#[specta::specta]
pub async fn pick_dish_art(
    state: State<'_, AppState>,
    dishes: Vec<ArtDishInput>,
    options: Vec<ArtOption>,
) -> Result<Vec<Option<ArtPickView>>, CommandError> {
    if dishes.len() > MAX_ART_DISHES {
        return Err(bad("dishes"));
    }
    let model = Writer::for_state(&state, AiMode::Local, "")?;
    tauri::async_runtime::spawn_blocking(move || pick_dish_art_with(&model, &dishes, &options))
        .await
        .map_err(|_| CommandError::Internal)?
}

#[cfg(test)]
mod art_tests {
    use super::*;

    struct Fake(&'static str);
    impl TextModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn generate(
            &self,
            _: &str,
            max: u32,
            _: &mut dyn FnMut(&str),
        ) -> Result<String, RecipeError> {
            assert_eq!(max, 40);
            Ok(self.0.into())
        }
    }

    #[test]
    fn picks_are_checked_against_the_options_and_ingredients() {
        let dishes = vec![ArtDishInput {
            title: "Eggs".into(),
            ingredients: vec!["egg".into(), "tomato".into()],
        }];
        let options = vec![ArtOption {
            id: "shakshuka".into(),
            label: "Shakshuka".into(),
        }];
        let r =
            pick_dish_art_with(&Fake("1 | shakshuka | egg, unicorn"), &dishes, &options).unwrap();
        assert_eq!(
            r,
            vec![Some(ArtPickView {
                kind: "shakshuka".into(),
                show: vec!["egg".into()]
            })]
        );
        let r = pick_dish_art_with(&Fake("1 | nope | egg"), &dishes, &options).unwrap();
        assert_eq!(r, vec![None]);
        let many: Vec<ArtDishInput> = (0..9)
            .map(|_| ArtDishInput {
                title: "x".into(),
                ingredients: vec![],
            })
            .collect();
        assert!(matches!(
            pick_dish_art_with(&Fake(""), &many, &options),
            Err(CommandError::BadInput(f)) if f == "dishes"
        ));
    }
}
