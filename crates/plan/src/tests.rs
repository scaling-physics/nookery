use chrono::NaiveDate;
use nookery_core::{FoodCategory, LotId};
use nookery_recipes::{Idea, StockItem, Suggestion, Urgency};

use super::*;

fn item(name: &str, urgency: Urgency) -> StockItem {
    StockItem {
        lot: LotId::new(),
        item: name.into(),
        left: "1".into(),
        location: nookery_core::Location::Fridge,
        urgency,
        opened: false,
    }
}

fn idea(name: &str, have: &[&StockItem], need: &[&str], score: i32) -> Suggestion {
    Suggestion {
        idea: Idea {
            name: name.into(),
            why: format!("why {name}"),
            minutes: Some(20),
            uses: vec![],
            needs: need.iter().map(|s| s.to_string()).collect(),
        },
        have: have.iter().map(|s| (s.item.clone(), s.lot)).collect(),
        need: need.iter().map(|s| s.to_string()).collect(),
        uses_soon: have
            .iter()
            .filter(|s| s.urgency == Urgency::UseSoon)
            .count() as u32,
        expired: vec![],
        score,
    }
}

fn opts() -> PlanOptions {
    PlanOptions::new(
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap(),
        "2026-11-08T10:00:00Z",
    )
}

fn names(p: &PlanDoc) -> Vec<&str> {
    p.days.iter().map(|d| d.name.as_str()).collect()
}

#[test]
fn use_soon_coverage_beats_raw_score() {
    let spinach = item("spinach", Urgency::UseSoon);
    let rice = item("rice", Urgency::Normal);
    let stock = vec![spinach.clone(), rice.clone()];
    // One slot. A scores higher alone, but B covers the use-soon lot.
    let pool = vec![
        idea("A rice bowl", &[&rice], &[], 9),
        idea("B spinach pie", &[&spinach], &["flour", "eggs", "x"], 3),
    ];
    let p = plan_week(&pool, &stock, 1, &opts());
    assert_eq!(names(&p), ["B spinach pie"]);
    // With one more day both are used.
    let p = plan_week(&pool, &stock, 2, &opts());
    assert_eq!(p.days.len(), 2);
    assert!(p.spares.is_empty());
}

#[test]
fn a_lot_counts_once() {
    let spinach = item("spinach", Urgency::UseSoon);
    let kale = item("kale", Urgency::UseSoon);
    let pasta = item("spaghetti", Urgency::Normal);
    let stock = vec![spinach.clone(), kale.clone(), pasta.clone()];
    // Two dinners with the same lot are worth less than one with a second.
    let pool = vec![
        idea("spinach soup", &[&spinach], &[], 4),
        idea("spinach salad", &[&spinach], &[], 4),
        idea("kale pasta", &[&kale, &pasta], &[], 4),
    ];
    let p = plan_week(&pool, &stock, 2, &opts());
    assert!(names(&p).contains(&"kale pasta"));
    assert_eq!(p.spares.len(), 1);
}

#[test]
fn variety_allows_two_of_a_protein() {
    let beef = item("beef", Urgency::Normal);
    let carrot = item("carrots", Urgency::Normal);
    let stock = vec![beef.clone(), carrot.clone()];
    let pool = vec![
        idea("beef 1", &[&beef], &[], 9),
        idea("beef 2", &[&beef], &[], 9),
        idea("beef 3", &[&beef], &[], 9),
        idea("carrot soup", &[&carrot], &[], 1),
    ];
    let p = plan_week(&pool, &stock, 3, &opts());
    let beefs = names(&p).iter().filter(|n| n.starts_with("beef")).count();
    assert_eq!(beefs, 2);
    assert!(names(&p).contains(&"carrot soup"));
}

#[test]
fn half_the_dinners_have_a_vegetable() {
    let chicken = item("chicken breast", Urgency::Normal);
    let rice = item("rice", Urgency::Normal);
    let beans = item("lentils", Urgency::Normal);
    let carrot = item("carrots", Urgency::Normal);
    let leek = item("leek", Urgency::Normal);
    let stock = vec![
        chicken.clone(),
        rice.clone(),
        beans.clone(),
        carrot.clone(),
        leek.clone(),
    ];
    let pool = vec![
        idea("rice 1", &[&rice], &[], 9),
        idea("rice 2", &[&rice], &[], 9),
        idea("rice 3", &[&rice], &[], 9),
        idea("rice 4", &[&rice], &[], 9),
        idea("carrot bake", &[&carrot], &[], 1),
        idea("leek soup", &[&leek], &[], 1),
    ];
    // 4 days: at least 2 with a vegetable.
    let p = plan_week(&pool, &stock, 4, &opts());
    let veg = p
        .days
        .iter()
        .filter(|d| d.name.contains("carrot") || d.name.contains("leek"))
        .count();
    assert!(veg >= 2, "{:?}", names(&p));
}

#[test]
fn short_pool_and_empty_pool() {
    let rice = item("rice", Urgency::Normal);
    let stock = vec![rice.clone()];
    let pool = vec![
        idea("rice", &[&rice], &[], 2),
        idea("rice 2", &[&rice], &[], 1),
    ];
    let p = plan_week(&pool, &stock, 5, &opts());
    assert_eq!(p.days.len(), 2);
    assert_eq!(p.days[0].date, "2026-11-09");
    assert_eq!(p.days[1].date, "2026-11-10");
    assert!(plan_week(&[], &stock, 5, &opts()).days.is_empty());
}

#[test]
fn most_urgent_first_and_dates() {
    let milk = item("milk", Urgency::UseSoon);
    let spinach = item("spinach", Urgency::UseSoon);
    let rice = item("rice", Urgency::Normal);
    // Stock is in use-first order: spinach before milk.
    let stock = vec![spinach.clone(), milk.clone(), rice.clone()];
    let pool = vec![
        idea("rice", &[&rice], &[], 5),
        idea("milk pudding", &[&milk], &[], 4),
        idea("spinach pie", &[&spinach], &[], 3),
    ];
    let mut o = opts();
    o.start = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
    let p = plan_week(&pool, &stock, 3, &o);
    assert_eq!(names(&p), ["spinach pie", "milk pudding", "rice"]);
    assert_eq!(p.week, "2026-11-09");
    assert_eq!(p.days[0].date, "2026-11-11");
    assert_eq!(p.days[0].uses_soon, 1);
}

#[test]
fn swap_takes_the_next_spare_and_keeps_cooked_days() {
    let rice = item("rice", Urgency::Normal);
    let stock = vec![rice.clone()];
    let pool = vec![
        idea("one", &[&rice], &[], 5),
        idea("two", &[&rice], &[], 4),
        idea("three", &[&rice], &[], 3),
    ];
    let mut p = plan_week(&pool, &stock, 2, &opts());
    assert_eq!(names(&p), ["one", "two"]);
    assert!(swap_day(&mut p, "2026-11-10"));
    assert_eq!(names(&p), ["one", "three"]);
    assert_eq!(p.days[1].date, "2026-11-10");
    assert_eq!(p.spares[0].name, "two");
    p.days[0].cooked = true;
    assert!(!swap_day(&mut p, "2026-11-09"));
    assert!(!swap_day(&mut p, "2026-12-01"));
    p.spares.clear();
    assert!(!swap_day(&mut p, "2026-11-10"));
}

#[test]
fn start_dates() {
    let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
    assert_eq!(default_start(d(2026, 11, 9)), d(2026, 11, 9)); // Monday
    assert_eq!(default_start(d(2026, 11, 11)), d(2026, 11, 16));
    assert_eq!(default_start(d(2026, 11, 15)), d(2026, 11, 16)); // Sunday
    assert_eq!(monday_of(d(2026, 11, 15)), d(2026, 11, 9));
}

#[test]
fn doc_round_trips_as_camel_case() {
    let rice = item("rice", Urgency::Normal);
    let pool = vec![idea("one", &[&rice], &["lemon"], 5)];
    let p = plan_week(&pool, &[rice], 1, &opts());
    let v = serde_json::to_value(&p).unwrap();
    assert!(v["days"][0]["needCategories"].is_array());
    assert_eq!(v["createdAt"], "2026-11-08T10:00:00Z");
    assert_eq!(serde_json::from_value::<PlanDoc>(v).unwrap(), p);
}

fn plan_with_needs(needs: &[(&str, bool)]) -> PlanDoc {
    PlanDoc {
        week: "2026-11-09".into(),
        days: needs
            .iter()
            .enumerate()
            .map(|(i, (n, cooked))| Day {
                date: format!("2026-11-{:02}", 9 + i),
                name: "d".into(),
                why: String::new(),
                minutes: None,
                have: vec![],
                need: vec![n.to_string(), "salt shaker".into()],
                need_categories: vec![FoodCategory::Vegetables, FoodCategory::Spices],
                uses_soon: 0,
                cooked: *cooked,
            })
            .collect(),
        spares: vec![],
        request: PlanRequest::default(),
        created_at: String::new(),
    }
}

#[test]
fn needs_dedup_against_list_and_stock() {
    let plan = plan_with_needs(&[
        ("onion", false),
        ("red onions", false),
        ("leek", true),
        ("lemon", false),
    ]);
    let needs = plan_needs(&plan);
    let names: Vec<&str> = needs.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["onion", "salt shaker", "lemon"]);
    let left = new_to_buy(needs, &["Lemons".to_string()], &["salt shaker".to_string()]);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].0, "onion");
}

#[test]
fn aisles_and_places() {
    assert!(aisle_rank(FoodCategory::Vegetables) < aisle_rank(FoodCategory::Dairy));
    assert!(aisle_rank(FoodCategory::Leftovers) < aisle_rank(FoodCategory::Other));
    assert_eq!(
        put_away_place(FoodCategory::Meat),
        nookery_core::Location::Fridge
    );
    assert_eq!(
        put_away_place(FoodCategory::Grains),
        nookery_core::Location::Pantry
    );
    assert_eq!(
        put_away_place(FoodCategory::Fruit),
        nookery_core::Location::Fridge
    );
    assert_eq!(
        put_away_place(FoodCategory::Other),
        nookery_core::Location::Pantry
    );
}

struct Script(std::sync::Mutex<Vec<String>>);

impl nookery_recipes::TextModel for Script {
    fn name(&self) -> String {
        "fake".into()
    }
    fn generate(
        &self,
        _: &str,
        _: u32,
        _: &mut dyn FnMut(&str),
    ) -> Result<String, nookery_recipes::RecipeError> {
        Ok(self.0.lock().unwrap().remove(0))
    }
}

#[test]
fn pool_asks_again_only_when_short() {
    let rice = item("rice", Urgency::Normal);
    let stock = vec![rice];
    let a = "### Rice bowl\nwhy: x\ntime: 10\nuses: rice\nneeds: none\n";
    let b = "### Rice bowl\nwhy: x\n### Fried rice\nwhy: y\nuses: rice\n";
    let req = nookery_recipes::Request {
        template: "t".into(),
        note: String::new(),
        count: 1,
    };
    let m = Script(std::sync::Mutex::new(vec![a.into(), b.into()]));
    let (pool, _) = gather_pool(&m, &stock, &req, 2).unwrap();
    assert_eq!(pool.len(), 2);
    assert!(m.0.lock().unwrap().is_empty());
    // Enough on the first call: the second answer is never asked for.
    let m = Script(std::sync::Mutex::new(vec![b.into(), a.into()]));
    let (pool, _) = gather_pool(&m, &stock, &req, 2).unwrap();
    assert_eq!(pool.len(), 2);
    assert_eq!(m.0.lock().unwrap().len(), 1);
}
