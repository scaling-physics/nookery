use super::*;
use std::sync::Mutex;

fn item(name: &str, left: &str, loc: Location, urgency: Urgency) -> StockItem {
    StockItem {
        lot: LotId::new(),
        item: name.into(),
        left: left.into(),
        location: loc,
        urgency,
        opened: false,
    }
}

fn req() -> Request {
    Request {
        template: "Dinner for two".into(),
        note: "no chilli".into(),
        count: 4,
    }
}

fn idea(name: &str, uses: &[&str], needs: &[&str], minutes: Option<u32>) -> Idea {
    Idea {
        name: name.into(),
        why: String::new(),
        minutes,
        uses: uses.iter().map(|s| s.to_string()).collect(),
        needs: needs.iter().map(|s| s.to_string()).collect(),
    }
}

struct Fake(Mutex<Vec<String>>, Mutex<Vec<String>>);
impl Fake {
    fn new(answers: &[&str]) -> Self {
        Fake(
            Mutex::new(answers.iter().rev().map(|s| s.to_string()).collect()),
            Mutex::default(),
        )
    }
}
impl TextModel for Fake {
    fn name(&self) -> String {
        "fake".into()
    }
    fn generate(
        &self,
        prompt: &str,
        _: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        self.1.lock().unwrap().push(prompt.to_string());
        let a = self.0.lock().unwrap().pop().expect("scripted answer");
        on_text(&a);
        Ok(a)
    }
}

fn kitchen() -> Vec<StockItem> {
    vec![
        item("spinach", "1", Location::Fridge, Urgency::UseSoon),
        item("zucchini", "2", Location::Fridge, Urgency::UseSoon),
        item("skyr", "500g", Location::Fridge, Urgency::Normal),
        item("old milk", "1l", Location::Fridge, Urgency::Expired),
        item("peas", "300g", Location::Freezer, Urgency::Normal),
        item("buckwheat", "1kg", Location::Pantry, Urgency::Normal),
        item("buckwheat", "500g", Location::Pantry, Urgency::Normal),
    ]
}

#[test]
fn the_prompt_groups_stock_and_leaves_expired_out() {
    let mut s = kitchen();
    s[2].opened = true;
    let p = ideas_prompt(&s, &req());
    assert!(p.contains("USE SOON: spinach, zucchini x2"), "{p}");
    assert!(p.contains("FRIDGE: skyr 500g (opened)"), "{p}");
    assert!(p.contains("FREEZER: peas 300g"), "{p}");
    assert!(p.contains("PANTRY: buckwheat x2 packs"), "{p}");
    assert!(!p.contains("old milk"));
    assert!(p.contains("What I want: Dinner for two — no chilli"));
    assert!(p.contains("Suggest 4 recipes"));
}

#[test]
fn empty_groups_are_skipped_in_the_prompt() {
    let s = vec![item("rice", "1", Location::Pantry, Urgency::Normal)];
    let p = ideas_prompt(&s, &req());
    assert!(p.contains("PANTRY: rice\n"));
    assert!(!p.contains("FRIDGE:") && !p.contains("USE SOON:") && !p.contains("FREEZER:"));
}

#[test]
fn messy_idea_answers_are_read() {
    let answer = "Sure! Here you go:\n\n\
**1. Spinach Frittata**\n\
**why:** Uses the spinach and zucchini.\n\
**Time:** 25 min\n\
**uses:** spinach, zucchini x2, eggs.\n\
**needs:** none\n\n\
## Skyr bowl\n\
why: quick\n\
time: 1 hour\n\
uses: skyr 500g\n\
needs: -\n\n\
### 3) Pea soup\n\
Time: about 20-30 minutes\n\
uses: peas\n";
    let ideas = parse_ideas(answer);
    assert_eq!(ideas.len(), 3);
    assert_eq!(ideas[0].name, "Spinach Frittata");
    assert_eq!(ideas[0].why, "Uses the spinach and zucchini.");
    assert_eq!(ideas[0].minutes, Some(25));
    assert_eq!(ideas[0].uses, ["spinach", "zucchini x2", "eggs"]);
    assert!(ideas[0].needs.is_empty());
    assert_eq!(ideas[1].name, "Skyr bowl");
    assert_eq!(ideas[1].minutes, Some(60));
    assert_eq!(ideas[1].uses, ["skyr 500g"]);
    assert!(ideas[1].needs.is_empty());
    assert_eq!(ideas[2].name, "Pea soup");
    assert_eq!(ideas[2].minutes, Some(20));
    assert!(ideas[2].why.is_empty());
}

#[test]
fn fields_before_any_header_are_ignored() {
    assert!(parse_ideas("why: nothing\nuses: a").is_empty());
}

#[test]
fn a_hallucinated_egg_moves_to_need_and_staples_are_dropped() {
    let stock = kitchen();
    let s = check(
        vec![idea(
            "Frittata",
            &["spinach", "egg", "salt", "olive oil", "black pepper"],
            &["parmesan"],
            Some(20),
        )],
        &stock,
    );
    let s = &s[0];
    assert_eq!(s.have.len(), 1);
    assert_eq!(s.have[0], ("spinach".to_string(), stock[0].lot));
    assert_eq!(s.need, ["egg", "parmesan"]);
    assert_eq!(s.uses_soon, 1);
}

#[test]
fn quantities_are_ignored_when_matching() {
    let stock = kitchen();
    let s = check(
        vec![idea("x", &["500g skyr", "zucchini x2"], &[], None)],
        &stock,
    );
    assert_eq!(s[0].have.len(), 2);
    assert!(s[0].need.is_empty());
}

#[test]
fn an_expired_only_match_warns_and_needs_it() {
    let stock = kitchen();
    let s = check(
        vec![idea("Porridge", &["old milk", "buckwheat"], &[], None)],
        &stock,
    );
    assert_eq!(s[0].expired, [stock[3].lot]);
    assert_eq!(s[0].need, ["old milk"]);
    assert_eq!(s[0].have.len(), 1);
}

#[test]
fn a_needs_item_that_is_in_stock_moves_to_have() {
    let stock = kitchen();
    let s = check(
        vec![idea("Bowl", &["skyr"], &["Peas", "feta"], None)],
        &stock,
    );
    let names: Vec<_> = s[0].have.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["skyr", "Peas"]);
    assert_eq!(s[0].need, ["feta"]);
}

#[test]
fn need_is_deduplicated_ignoring_case() {
    let s = check(vec![idea("x", &["Egg", "egg"], &["EGG"], None)], &kitchen());
    assert_eq!(s[0].need.len(), 1);
}

#[test]
fn the_most_urgent_lot_is_used_and_each_lot_serves_one_ingredient() {
    let stock = vec![
        item("red bell pepper", "1", Location::Fridge, Urgency::Normal),
        item("red bell pepper", "1", Location::Fridge, Urgency::UseSoon),
    ];
    let s = check(
        vec![idea("x", &["red bell pepper", "chili pepper"], &[], None)],
        &stock,
    );
    assert_eq!(s[0].have[0].1, stock[1].lot);
    assert_eq!(s[0].uses_soon, 1);
}

#[test]
fn suggestions_are_sorted_by_score_then_minutes_then_name() {
    let stock = kitchen();
    let s = check(
        vec![
            idea("Plain", &["skyr"], &["a", "b"], Some(10)),
            idea("Soon", &["spinach", "zucchini"], &[], Some(30)),
            idea("B tie", &["skyr"], &[], Some(20)),
            idea("A tie", &["peas"], &[], Some(20)),
            idea("Fast tie", &["peas"], &[], Some(5)),
        ],
        &stock,
    );
    let names: Vec<_> = s.iter().map(|s| s.idea.name.as_str()).collect();
    assert_eq!(names, ["Soon", "Fast tie", "A tie", "B tie", "Plain"]);
    assert_eq!(s[0].score, 3 * 2 + 2);
    assert_eq!(s[4].score, 1 - 4);
}

#[test]
fn messy_recipes_are_read() {
    let r = parse_recipe(
        "# **Spinach Frittata**\nServes: 2 people\n**Time:** 25 minutes\n\n## Ingredients\n- 200 g spinach\n* 4 eggs\n• 1 zucchini\n\n## Steps\n1. Chop.\n2) Fry.\nStep 3: Eat.\n",
    );
    assert_eq!(r.title, "Spinach Frittata");
    assert_eq!((r.serves, r.minutes), (Some(2), Some(25)));
    assert_eq!(r.ingredients, ["200 g spinach", "4 eggs", "1 zucchini"]);
    assert_eq!(r.steps, ["Chop.", "Fry.", "Eat."]);
    assert!(r.raw.contains("Frittata"));
}

#[test]
fn a_recipe_without_headers_gets_a_best_effort() {
    let r = parse_recipe("Quick Toast\n- bread\n- butter\n1. Toast it.\n2. Butter it.");
    assert_eq!(r.title, "Quick Toast");
    assert_eq!(r.ingredients, ["bread", "butter"]);
    assert_eq!(r.steps.len(), 2);
}

#[test]
fn the_recipe_prompt_lists_amounts_left_and_what_to_buy() {
    let stock = kitchen();
    let s = check(vec![idea("Bowl", &["skyr"], &["feta"], Some(5))], &stock);
    let p = recipe_prompt(&s[0], &stock, &req());
    assert!(p.contains("- skyr: 500g available"));
    assert!(p.contains("- feta"));
    assert!(p.contains("10 steps"));
}

#[test]
fn suggest_with_nothing_in_stock_says_so() {
    let m = Fake::new(&[]);
    assert!(matches!(
        suggest(&m, &[], &req()),
        Err(RecipeError::NoStock)
    ));
    let only_expired = [item("milk", "1", Location::Fridge, Urgency::Expired)];
    assert!(matches!(
        suggest(&m, &only_expired, &req()),
        Err(RecipeError::NoStock)
    ));
}

#[test]
fn suggest_and_write_recipe_work_with_a_scripted_model() {
    let m = Fake::new(&[
        "### Skyr bowl\nwhy: easy\ntime: 5\nuses: skyr, egg\nneeds: none\n",
        "# Skyr bowl\nserves: 1\n## Ingredients\n- 200g skyr\n## Steps\n1. Eat.",
    ]);
    let stock = kitchen();
    let s = suggest(&m, &stock, &req()).unwrap();
    assert_eq!(s.model, "fake");
    assert_eq!(s.items[0].need, ["egg"]);
    let mut streamed = String::new();
    let r = write_recipe(&m, &s.items[0], &stock, &req(), &mut |t| {
        streamed.push_str(t)
    })
    .unwrap();
    assert_eq!(r.steps, ["Eat."]);
    assert!(streamed.contains("Skyr bowl"));
}

#[test]
fn a_closed_port_is_reported_as_unreachable() {
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let m = OllamaText {
        url: format!("http://127.0.0.1:{port}"),
        model: "x".into(),
    };
    let e = m.generate("hi", 10, &mut |_| {}).unwrap_err();
    assert!(matches!(e, RecipeError::Unreachable(_)), "{e:?}");
}

#[test]
fn ollama_errors_are_mapped() {
    assert!(matches!(
        helpers::model_error("m", "model 'm' not found"),
        RecipeError::ModelMissing(_)
    ));
    assert!(matches!(
        helpers::model_error("m", "boom"),
        RecipeError::Model(_)
    ));
}

#[test]
fn stock_from_orders_by_use_first_and_maps_urgency() {
    use chrono::NaiveDate;
    use nookery_core::{Event, EventKind, Inventory, Qty, Source};
    let today = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
    let at = today.and_hms_opt(12, 0, 0).unwrap().and_utc();
    let mut inv = Inventory::default();
    let mut ids = vec![];
    for (name, bb) in [
        ("rice", None),
        ("cream", Some(-2)),
        ("spinach", Some(1)),
        ("cheese", Some(20)),
    ] {
        let lot = LotId::new();
        ids.push(lot);
        inv.apply(&Event {
            at,
            lot,
            kind: EventKind::Acquired {
                item: name.into(),
                qty: Qty::pieces(1),
                location: Location::Fridge,
                source: Source::Manual,
                best_before: bb.map(|d| today + chrono::Duration::days(d)),
            },
        })
        .unwrap();
    }
    let s = stock_from(inv.all(), today);
    let order: Vec<_> = s.iter().map(|s| (s.item.as_str(), s.urgency)).collect();
    assert_eq!(
        order,
        [
            ("cream", Urgency::Expired),
            ("spinach", Urgency::UseSoon),
            ("cheese", Urgency::Normal),
            ("rice", Urgency::Normal)
        ]
    );
}

/// Streams `text` (if any) then returns `result`; counts calls.
struct Script {
    text: Option<&'static str>,
    result: fn() -> Result<String, RecipeError>,
    calls: Mutex<u32>,
}
impl Script {
    fn new(text: Option<&'static str>, result: fn() -> Result<String, RecipeError>) -> Self {
        Script {
            text,
            result,
            calls: Mutex::new(0),
        }
    }
    fn calls(&self) -> u32 {
        *self.calls.lock().unwrap()
    }
}
impl TextModel for Script {
    fn name(&self) -> String {
        "script".into()
    }
    fn generate(
        &self,
        _: &str,
        _: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        *self.calls.lock().unwrap() += 1;
        if let Some(t) = self.text {
            on_text(t);
        }
        (self.result)()
    }
}

#[test]
fn text_fallback_skips_the_secondary_when_the_primary_answers() {
    let f = FallbackText::new(
        Script::new(None, || Ok("local".into())),
        Script::new(None, || Ok("cloud".into())),
    );
    assert_eq!(f.generate("p", 10, &mut |_| {}).unwrap(), "local");
    assert!(!f.fell_back());
    assert_eq!(f.secondary.calls(), 0);
}

#[test]
fn text_fallback_uses_the_secondary_when_the_primary_is_missing_or_fails() {
    let fails: [fn() -> Result<String, RecipeError>; 2] = [
        || Err(RecipeError::ModelMissing("m".into())),
        || Err(RecipeError::Model("boom".into())),
    ];
    for fail in fails {
        let f = FallbackText::new(
            Script::new(None, fail),
            Script::new(None, || Ok("cloud".into())),
        );
        assert_eq!(f.generate("p", 10, &mut |_| {}).unwrap(), "cloud");
        assert!(f.fell_back());
    }
}

#[test]
fn text_fallback_does_not_retry_unreachable_or_sign_in() {
    let fails: [fn() -> Result<String, RecipeError>; 2] = [
        || Err(RecipeError::Unreachable("u".into())),
        || Err(RecipeError::SignIn),
    ];
    for fail in fails {
        let f = FallbackText::new(
            Script::new(None, fail),
            Script::new(None, || Ok("cloud".into())),
        );
        assert!(f.generate("p", 10, &mut |_| {}).is_err());
        assert!(!f.fell_back());
        assert_eq!(f.secondary.calls(), 0);
    }
}

#[test]
fn text_fallback_is_not_retried_after_text_was_streamed() {
    let f = FallbackText::new(
        Script::new(Some("half an ans"), || {
            Err(RecipeError::Model("cut".into()))
        }),
        Script::new(None, || Ok("cloud".into())),
    );
    let mut got = String::new();
    let r = f.generate("p", 10, &mut |t| got.push_str(t));
    assert!(matches!(r, Err(RecipeError::Model(_))));
    assert_eq!(got, "half an ans");
    assert!(!f.fell_back());
    assert_eq!(f.secondary.calls(), 0);
}

#[test]
fn chosen_text_picks_the_model_for_the_mode() {
    use nookery_core::AiMode;
    let local = ChosenText::for_mode(AiMode::Local, "some:cloud");
    assert!(matches!(local, ChosenText::Local(_)));
    assert!(local.answered_by().ends_with("(this Mac)"));
    assert!(!local.used_cloud());

    let cloud = ChosenText::for_mode(AiMode::Cloud, "some:cloud");
    assert_eq!(cloud.answered_by(), "ollama some:cloud (cloud)");
    assert!(cloud.used_cloud());

    let fb = ChosenText::for_mode(AiMode::Fallback, "some:cloud");
    assert!(fb.name().contains("cloud fallback: ollama some:cloud"));
    assert!(!fb.used_cloud());
}

#[test]
fn a_402_means_no_cloud_credits_not_a_sign_in_problem() {
    let e = crate::helpers::model_error(
        "gemma4:31b-cloud",
        "HTTP 402: {\"error\":\"Add usage credits or subscribe and save up to 66%: https://ollama.com/plans\"}",
    );
    assert!(matches!(e, RecipeError::NoCredits), "{e:?}");
    assert!(e.to_string().contains("ollama.com/plans"));
}

// ---- chat ----

fn sample_recipe() -> Recipe {
    parse_recipe(
        "# Spinach omelette\nServes: 2\nTime: 15 min\n## Ingredients\n- 100 g spinach\n- 3 eggs\n## Steps\n1. Wilt the spinach.\n2. Add the eggs.\n",
    )
}

#[test]
fn chat_prompt_has_recipe_stock_and_bounded_history() {
    let stock = vec![
        item("zucchini", "2", Location::Fridge, Urgency::UseSoon),
        item("old cream", "1", Location::Fridge, Urgency::Expired),
    ];
    let history: Vec<ChatTurn> = (0..10)
        .map(|i| ChatTurn {
            role: if i % 2 == 0 {
                ChatRole::User
            } else {
                ChatRole::Cook
            },
            text: format!("turn number {i}"),
        })
        .collect();
    let p = chat_prompt(&sample_recipe(), &stock, &history, "make it spicier");
    assert!(p.contains("# Spinach omelette"));
    assert!(p.contains("- 3 eggs"));
    assert!(p.contains("USE SOON: zucchini x2"));
    assert!(!p.contains("old cream"));
    assert!(!p.contains("turn number 3"));
    assert!(p.contains("turn number 4") && p.contains("turn number 9"));
    assert!(p.contains("Me: turn number 4") && p.contains("You: turn number 5"));
    assert!(p.contains("My message: make it spicier"));
    // The recipe survives its own text format.
    assert_eq!(parse_recipe(&sample_recipe().to_text()).steps.len(), 2);
}

#[test]
fn chat_question_only() {
    let a = parse_chat("Yes, you can freeze the leftovers for up to 2 months.\n");
    assert_eq!(
        a.reply,
        "Yes, you can freeze the leftovers for up to 2 months."
    );
    assert!(a.recipe.is_none());
}

#[test]
fn chat_reply_and_revised_recipe() {
    let a = parse_chat(
        "I swapped the halloumi for feta and added zucchini.\n---\n# Spinach feta omelette\nServes: 4\nTime: 20 min\n## Ingredients\n- 200 g spinach\n- 6 eggs\n## Steps\n1. Wilt.\n2. Set.\n",
    );
    assert_eq!(
        a.reply,
        "I swapped the halloumi for feta and added zucchini."
    );
    let r = a.recipe.unwrap();
    assert_eq!(r.title, "Spinach feta omelette");
    assert_eq!((r.serves, r.minutes), (Some(4), Some(20)));
    assert_eq!((r.ingredients.len(), r.steps.len()), (2, 2));
}

#[test]
fn chat_sloppy_formatting() {
    let a = parse_chat(
        "**Reply:** Done, no oven needed.\n\nHere is the revised recipe:\n\n**# Pan frittata**\n\n**Servings:** 2\n**Ingredients**\n* 4 eggs\n* 1 zucchini\n**Steps**\nStep 1: Fry the zucchini.\nStep 2: Pour in the eggs.\n",
    );
    assert!(
        a.reply.starts_with("Done, no oven needed."),
        "{:?}",
        a.reply
    );
    let r = a.recipe.unwrap();
    assert_eq!(r.title, "Pan frittata");
    assert_eq!(r.serves, Some(2));
    assert_eq!(r.ingredients, ["4 eggs", "1 zucchini"]);
    assert_eq!(r.steps, ["Fry the zucchini.", "Pour in the eggs."]);
}

#[test]
fn chat_recipe_without_reply_and_half_recipes() {
    let a =
        parse_chat("# Veg curry\nServes: 2\n## Ingredients\n- chickpeas\n## Steps\n1. Simmer.\n");
    assert_eq!(a.reply, "");
    assert_eq!(a.recipe.unwrap().title, "Veg curry");
    // Headerless recipe still found.
    let b = parse_chat("## Ingredients\n- rice\n## Steps\n1. Boil.\n");
    assert!(b.recipe.is_some());
    // Ingredients only: not a recipe, the text stays the reply.
    let c = parse_chat("You could add:\n## Ingredients\n- rice\n");
    assert!(c.recipe.is_none());
    assert!(c.reply.contains("You could add"));
}

#[test]
fn chat_runs_the_model() {
    let model = Fake::new(&["Sure.\n---\n# T\n## Ingredients\n- a\n## Steps\n1. b\n"]);
    let mut seen = String::new();
    let a = chat(&model, &sample_recipe(), &[], &[], "hi", &mut |t| {
        seen.push_str(t)
    })
    .unwrap();
    assert_eq!(a.reply, "Sure.");
    assert!(a.recipe.is_some());
}
