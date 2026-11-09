//! One real run against the user's stock and the local Ollama. Reads a COPY
//! of the real database; never writes to the original.
//! `cargo test -p nookery-recipes --release --test real_stock -- --ignored --nocapture`

use nookery_recipes::*;
use std::time::Instant;

#[test]
#[ignore]
fn suggestions_and_a_recipe_from_the_real_stock() {
    let src = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join("Library/Application Support/dev.nookery.app");
    let tmp = tempfile::tempdir().unwrap();
    for f in ["nookery.db", "nookery.db-wal", "nookery.db-shm"] {
        if src.join(f).exists() {
            std::fs::copy(src.join(f), tmp.path().join(f)).unwrap();
        }
    }
    let store = nookery_store::Store::open_dir(tmp.path()).unwrap();
    let today = chrono::Local::now().date_naive();
    let stock = stock_from(store.inventory().all(), today);
    println!("{} lots in stock", stock.len());

    let request = Request {
        template: "Dinner for two, ready in 30 minutes".into(),
        note: String::new(),
        count: 4,
    };
    println!("--- prompt ---\n{}", ideas_prompt(&stock, &request));
    let model = OllamaText::from_env();
    let s = suggest(&model, &stock, &request).unwrap();
    println!("{} ideas in {:.0} ms ({})", s.items.len(), s.ms, s.model);
    for (i, x) in s.items.iter().enumerate() {
        println!(
            "{}. {} ({:?} min) score {}\n   why: {}\n   have: {:?}\n   need: {:?}\n   expired: {:?}",
            i + 1,
            x.idea.name,
            x.idea.minutes,
            x.score,
            x.idea.why,
            x.have.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
            x.need,
            x.expired
        );
    }
    let top = s.items.first().expect("at least one idea");
    let t = Instant::now();
    let r = write_recipe(&model, top, &stock, &request, &mut |_| {}).unwrap();
    println!(
        "--- recipe in {:.0} ms ---",
        t.elapsed().as_secs_f64() * 1e3
    );
    println!("# {} (serves {:?}, {:?} min)", r.title, r.serves, r.minutes);
    println!("ingredients: {:#?}\nsteps: {:#?}", r.ingredients, r.steps);
    println!("--- raw ---\n{}", r.raw);
}
