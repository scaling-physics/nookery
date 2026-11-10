//! Shopping-list rules: what to add from a plan, aisle order, where bought
//! things go when put away.

use nookery_core::{category, names_match, FoodCategory, Location};

use crate::PlanDoc;

/// Aisle-like order of the list.
pub const AISLE_ORDER: [FoodCategory; 18] = [
    FoodCategory::Vegetables,
    FoodCategory::Fruit,
    FoodCategory::Herbs,
    FoodCategory::Bread,
    FoodCategory::Dairy,
    FoodCategory::Cheese,
    FoodCategory::Eggs,
    FoodCategory::Meat,
    FoodCategory::Fish,
    FoodCategory::Grains,
    FoodCategory::Legumes,
    FoodCategory::Sauces,
    FoodCategory::Spices,
    FoodCategory::Nuts,
    FoodCategory::Sweets,
    FoodCategory::Drinks,
    FoodCategory::Leftovers,
    FoodCategory::Other,
];

/// Position of a category in the list (smaller = earlier).
pub fn aisle_rank(c: FoodCategory) -> usize {
    AISLE_ORDER
        .iter()
        .position(|x| *x == c)
        .unwrap_or(usize::MAX)
}

/// Fridge for fresh things, else the pantry.
pub fn put_away_place(c: FoodCategory) -> Location {
    use FoodCategory as C;
    match c {
        C::Dairy
        | C::Cheese
        | C::Eggs
        | C::Meat
        | C::Fish
        | C::Vegetables
        | C::Herbs
        | C::Fruit => Location::Fridge,
        _ => Location::Pantry,
    }
}

/// The `from` marker of items added from a plan.
pub fn from_plan(week: &str) -> String {
    format!("plan:{week}")
}

/// What the plan still needs to buy: the `need` of every day not yet cooked,
/// one entry per distinct ingredient (`names_match`), with its category.
pub fn plan_needs(plan: &PlanDoc) -> Vec<(String, FoodCategory)> {
    let mut out: Vec<(String, FoodCategory)> = Vec::new();
    for d in plan.days.iter().filter(|d| !d.cooked) {
        for (i, n) in d.need.iter().enumerate() {
            let n = n.trim();
            if n.is_empty() || out.iter().any(|(o, _)| names_match(o, n)) {
                continue;
            }
            out.push((
                n.to_string(),
                d.need_categories
                    .get(i)
                    .copied()
                    .unwrap_or_else(|| category(n)),
            ));
        }
    }
    out
}

/// The needs that are not already on the list (undone items) and not in
/// stock.
pub fn new_to_buy(
    needs: Vec<(String, FoodCategory)>,
    undone: &[String],
    in_stock: &[String],
) -> Vec<(String, FoodCategory)> {
    needs
        .into_iter()
        .filter(|(n, _)| {
            !undone
                .iter()
                .chain(in_stock)
                .any(|have| names_match(have, n))
        })
        .collect()
}
