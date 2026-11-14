//! One real art pick with the local Ollama (no database).
//! `cargo test -p nookery-recipes --release --test real_art -- --ignored --nocapture`
//! Phone model: `NOOKERY_TEXT_MODEL=qwen3:4b-instruct`.

use nookery_recipes::*;
use std::time::Instant;

#[test]
#[ignore]
fn picks_pictures_for_a_few_dishes() {
    let options: Vec<(String, String)> = [
        ("shakshuka", "Shakshuka"),
        ("ramen", "Ramen"),
        ("green-salad", "Green salad"),
        ("spaghetti-tomato", "Spaghetti with tomato sauce"),
        ("lasagne", "Lasagne"),
        ("chicken-curry", "Chicken curry with rice"),
        ("omelette", "Omelette"),
        ("pancakes", "Pancake stack"),
        ("burger", "Burger"),
        ("pizza", "Pizza"),
        ("tomato-soup", "Tomato soup"),
        ("fried-rice", "Fried rice"),
        ("veg-stir-fry", "Vegetable stir-fry"),
        ("potato-gratin", "Potato gratin"),
        ("roast-chicken", "Roast chicken"),
        ("salmon-fillet", "Salmon fillet with sides"),
        ("chili", "Chili con carne"),
        ("porridge", "Porridge"),
        ("smoothie", "Smoothie"),
        ("tacos", "Tacos"),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    let d = |t: &str, i: &[&str]| ArtDish {
        title: t.into(),
        ingredients: i.iter().map(|s| s.to_string()).collect(),
    };
    let dishes = vec![
        d(
            "Eggs poached in spiced peppers",
            &["egg", "bell pepper", "tomato", "onion", "feta", "cumin"],
        ),
        d(
            "Zucchini-Nudeln mit Hack",
            &[
                "zucchini",
                "minced beef",
                "pasta",
                "tomato sauce",
                "parmesan",
            ],
        ),
        d(
            "Quick weeknight bowl",
            &["rice", "carrot", "chickpeas", "spinach", "tahini", "lemon"],
        ),
        d("Rahmsuppe mit Brot", &["potato", "leek", "cream", "bread"]),
    ];
    let model = OllamaText::from_env();
    println!("model: {}", model.name());
    println!("--- prompt ---\n{}", art_prompt(&dishes, &options));
    let t = Instant::now();
    let answer = model
        .generate(&art_prompt(&dishes, &options), 40 * 4, &mut |_| {})
        .unwrap();
    println!(
        "--- raw ({:.0} ms) ---\n{answer}",
        t.elapsed().as_secs_f64() * 1e3
    );
    let picks = parse_art(&answer, &dishes, &options);
    for (d, p) in dishes.iter().zip(&picks) {
        println!("{} -> {:?}", d.title, p);
    }
    assert!(picks.iter().any(Option::is_some));
}
