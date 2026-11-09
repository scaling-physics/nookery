//! A food category from an item's name, for the little icons in lists.
//! Derived, never stored: it's a guess from words (English and the German
//! printed on receipts and packs), and a better source (Open Food Facts,
//! M2) can replace it without touching the ledger.

use serde::{Deserialize, Serialize};

use crate::reconcile::name_key;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoodCategory {
    Vegetables,
    Fruit,
    Herbs,
    Dairy,
    Cheese,
    Eggs,
    Meat,
    Fish,
    Bread,
    /// Pasta, rice, oats, flour, quinoa, buckwheat, cereal.
    Grains,
    /// Beans, lentils, chickpeas, canned peas.
    Legumes,
    /// Salt, pepper, spice jars, dried herbs.
    Spices,
    /// Pesto, sauces, condiments, pickles, oil, jam, honey.
    Sauces,
    /// Chocolate, biscuits, crisps, sugar.
    Sweets,
    /// Coffee, tea, juice, water, wine.
    Drinks,
    Nuts,
    /// Cooked at home: leftovers, curry, soup.
    Leftovers,
    Other,
}

/// Phrases checked first (they would be misread word by word).
const PHRASES: &[(&str, FoodCategory)] = &[
    ("bell pepper", FoodCategory::Vegetables),
    ("chili pepper", FoodCategory::Vegetables),
    ("chilli pepper", FoodCategory::Vegetables),
    ("black pepper", FoodCategory::Spices),
    ("white pepper", FoodCategory::Spices),
    ("peanut butter", FoodCategory::Sauces),
    ("coconut milk", FoodCategory::Sauces),
    ("oat milk", FoodCategory::Drinks),
    ("soy sauce", FoodCategory::Sauces),
    ("ice cream", FoodCategory::Sweets),
    ("canned peas", FoodCategory::Legumes),
    ("green bean", FoodCategory::Vegetables),
    ("kidney bean", FoodCategory::Legumes),
    ("spring onion", FoodCategory::Vegetables),
];

/// Single words (after `name_key`: lower case, singular). First match in
/// the name wins, so more specific words come before generic ones.
const WORDS: &[(&[&str], FoodCategory)] = &[
    (
        &[
            "leftover", "curry", "soup", "stew", "dal", "dhal", "lasagne", "lasagna", "reste",
        ],
        FoodCategory::Leftovers,
    ),
    (
        &[
            "parsley",
            "petersilie",
            "basil",
            "basilikum",
            "mint",
            "minze",
            "coriander",
            "cilantro",
            "koriander",
            "dill",
            "chive",
            "schnittlauch",
            "rosemary",
            "thyme",
            "sage",
            "oregano",
        ],
        FoodCategory::Herbs,
    ),
    (
        &[
            "cheese",
            "käse",
            "kaese",
            "halloumi",
            "feta",
            "mozzarella",
            "parmesan",
            "gouda",
            "cheddar",
            "paneer",
            "ricotta",
            "mascarpone",
            "camembert",
            "brie",
        ],
        FoodCategory::Cheese,
    ),
    (&["egg", "eier", "ei"], FoodCategory::Eggs),
    (
        &[
            "milk",
            "milch",
            "weidemilch",
            "skyr",
            "yoghurt",
            "yogurt",
            "joghurt",
            "quark",
            "cream",
            "sahne",
            "butter",
            "kefir",
            "buttermilk",
            "schmand",
        ],
        FoodCategory::Dairy,
    ),
    (
        &[
            "chicken",
            "hähnchen",
            "haehnchen",
            "beef",
            "rind",
            "pork",
            "schwein",
            "mince",
            "minced",
            "hack",
            "hackfleisch",
            "ham",
            "schinken",
            "bacon",
            "speck",
            "sausage",
            "wurst",
            "salami",
            "lamb",
            "turkey",
            "pute",
            "meat",
            "fleisch",
        ],
        FoodCategory::Meat,
    ),
    (
        &[
            "fish",
            "fisch",
            "salmon",
            "lachs",
            "tuna",
            "thunfisch",
            "cod",
            "shrimp",
            "prawn",
            "garnele",
            "sardine",
        ],
        FoodCategory::Fish,
    ),
    (
        &[
            "bread",
            "brot",
            "brötchen",
            "broetchen",
            "toast",
            "baguette",
            "roll",
            "bagel",
            "wrap",
            "tortilla",
            "pita",
            "naan",
        ],
        FoodCategory::Bread,
    ),
    (
        &[
            "pasta",
            "nudel",
            "nudeln",
            "spaghetti",
            "rigatoni",
            "penne",
            "fusilli",
            "macaroni",
            "noodle",
            "rice",
            "reis",
            "basmati",
            "oat",
            "hafer",
            "haferflocken",
            "flour",
            "mehl",
            "quinoa",
            "buckwheat",
            "buchweizen",
            "couscous",
            "bulgur",
            "cereal",
            "muesli",
            "müsli",
            "granola",
            "semolina",
            "polenta",
        ],
        FoodCategory::Grains,
    ),
    (
        &[
            "bean",
            "bohne",
            "bohnen",
            "lentil",
            "linse",
            "linsen",
            "chickpea",
            "kichererbse",
            "kichererbsen",
            "tofu",
        ],
        FoodCategory::Legumes,
    ),
    (
        &[
            "salt",
            "salz",
            "pepper",
            "pfeffer",
            "spice",
            "gewürz",
            "gewuerz",
            "cumin",
            "kreuzkümmel",
            "turmeric",
            "kurkuma",
            "cinnamon",
            "zimt",
            "cardamom",
            "nutmeg",
            "masala",
            "chilli",
            "cayenne",
        ],
        FoodCategory::Spices,
    ),
    (
        &[
            "pesto",
            "sauce",
            "soße",
            "sosse",
            "ketchup",
            "mustard",
            "senf",
            "mayo",
            "mayonnaise",
            "pickle",
            "gurken",
            "gherkin",
            "oil",
            "öl",
            "oel",
            "vinegar",
            "essig",
            "jam",
            "marmelade",
            "honey",
            "honig",
            "tahini",
            "hummus",
            "dressing",
            "paste",
            "stock",
            "brühe",
            "bruehe",
        ],
        FoodCategory::Sauces,
    ),
    (
        &[
            "chocolate",
            "schokolade",
            "kitkat",
            "biscuit",
            "keks",
            "cookie",
            "chip",
            "crisp",
            "candy",
            "sweet",
            "sugar",
            "zucker",
            "cake",
            "kuchen",
            "snack",
        ],
        FoodCategory::Sweets,
    ),
    (
        &[
            "coffee",
            "kaffee",
            "espresso",
            "tea",
            "tee",
            "juice",
            "saft",
            "water",
            "wasser",
            "wine",
            "wein",
            "beer",
            "bier",
            "soda",
            "lemonade",
            "limonade",
            "kaffeeweiß",
        ],
        FoodCategory::Drinks,
    ),
    (
        &[
            "nut",
            "nuss",
            "nüsse",
            "almond",
            "mandel",
            "walnut",
            "cashew",
            "peanut",
            "hazelnut",
            "seed",
            "pistachio",
        ],
        FoodCategory::Nuts,
    ),
    (
        &[
            "apple",
            "apfel",
            "banana",
            "banane",
            "lemon",
            "zitrone",
            "lime",
            "limette",
            "orange",
            "pear",
            "birne",
            "berry",
            "beere",
            "strawberry",
            "erdbeere",
            "blueberry",
            "grape",
            "traube",
            "mango",
            "pineapple",
            "kiwi",
            "peach",
            "plum",
            "pflaume",
            "melon",
            "avocado",
            "cherry",
            "kirsche",
            "fig",
            "date",
        ],
        FoodCategory::Fruit,
    ),
    (
        &[
            "vegetable",
            "gemüse",
            "tomato",
            "tomate",
            "cucumber",
            "gurke",
            "zucchini",
            "courgette",
            "eggplant",
            "aubergine",
            "paprika",
            "carrot",
            "karotte",
            "möhre",
            "onion",
            "zwiebel",
            "garlic",
            "knoblauch",
            "ginger",
            "ingwer",
            "potato",
            "kartoffel",
            "spinach",
            "spinat",
            "lettuce",
            "salat",
            "broccoli",
            "brokkoli",
            "cauliflower",
            "blumenkohl",
            "cabbage",
            "kohl",
            "kale",
            "leek",
            "lauch",
            "celery",
            "sellerie",
            "mushroom",
            "pilz",
            "champignon",
            "corn",
            "mais",
            "pea",
            "erbse",
            "erbsen",
            "pumpkin",
            "kürbis",
            "squash",
            "radish",
            "beet",
            "rote",
            "asparagus",
            "spargel",
            "chili",
        ],
        FoodCategory::Vegetables,
    ),
];

/// The category of an item, from its name (`name_key` words, so plurals,
/// case and container words don't matter). `Other` when nothing matches.
pub fn category(item: &str) -> FoodCategory {
    let key = name_key(item);
    let lower = item.to_lowercase();
    for (phrase, c) in PHRASES {
        if lower.contains(phrase) || key.contains(phrase) {
            return *c;
        }
    }
    let words: Vec<&str> = key.split(' ').collect();
    for (list, c) in WORDS {
        if words.iter().any(|w| list.contains(w)) {
            return *c;
        }
    }
    // German compounds, last resort: a word that ends with a known word of
    // 4+ letters ("Vollkornbrot", "Erdbeerjoghurt") or starts with one of
    // 5+ ("Hähnchenbrust"; 5 so "pea" doesn't claim "peanut").
    for (list, c) in WORDS {
        let compound = |w: &&str| {
            list.iter().any(|k| {
                let n = k.chars().count();
                (n >= 4 && w.ends_with(k)) || (n >= 5 && w.starts_with(k))
            })
        };
        if words.iter().any(compound) {
            return *c;
        }
    }
    FoodCategory::Other
}

#[cfg(test)]
mod tests {
    use super::*;
    use FoodCategory::*;

    #[test]
    fn the_users_kitchen_sorts_sensibly() {
        let cases = [
            ("skyr", Dairy),
            ("pesto", Sauces),
            ("pickles", Sauces),
            ("green limes", Fruit),
            ("ginger", Vegetables),
            ("garlic", Vegetables),
            ("mint", Herbs),
            ("zucchini", Vegetables),
            ("red bell pepper", Vegetables),
            ("paprika", Vegetables),
            ("tomato", Vegetables),
            ("chicken", Meat),
            ("cheese", Cheese),
            ("nuts", Nuts),
            ("Buckwheat", Grains),
            ("High Protein Pasta", Grains),
            ("Canned beans", Legumes),
            ("peas", Vegetables),
            ("canned peas", Legumes),
            ("KitKat mini", Sweets),
            ("coffee", Drinks),
            ("milk", Dairy),
            ("Pfeffer", Spices),
            ("Pepper", Spices),
            ("black pepper", Spices),
            ("Salt", Spices),
            ("Sugar", Sweets),
            ("leftover yellow curry", Leftovers),
            ("yellow box", Other),
        ];
        for (name, want) in cases {
            assert_eq!(category(name), want, "{name}");
        }
    }

    #[test]
    fn german_names_and_compounds_work() {
        assert_eq!(category("Vollkornbrot"), Bread);
        assert_eq!(category("H-Milch 1,5%"), Dairy);
        assert_eq!(category("Hähnchenbrust"), Meat);
        assert_eq!(category("Kichererbsen"), Legumes);
        assert_eq!(category("Zwiebeln"), Vegetables);
    }
}
