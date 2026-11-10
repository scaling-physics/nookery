#![cfg(feature = "llama")]
//! One real run against the user's stock with the in-process model (the Ollama
//! blob of qwen3:4b-instruct, or $NOOKERY_GGUF). Reads a COPY
//! of the real database; never writes to the original.
//! `cargo test -p nookery-llm --features llama --release --test real_model -- --ignored --nocapture`

use nookery_llm::{ollama_blob, LlamaOptions, LlamaText};
use nookery_recipes::*;
use std::time::Instant;

#[test]
#[ignore]
fn llama_suggestions_and_a_recipe_from_the_real_stock() {
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
    let path = std::env::var_os("NOOKERY_GGUF")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            ollama_blob(
                &std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".ollama/models"),
                "qwen3:4b-instruct",
            )
        })
        .expect("no GGUF: pull qwen3:4b-instruct in Ollama or set NOOKERY_GGUF");
    let t = Instant::now();
    let model = LlamaText::load(&path, LlamaOptions::default()).unwrap();
    println!(
        "loaded {} ({} MB) in {:.0} ms",
        model.name(),
        model.model_bytes() >> 20,
        t.elapsed().as_secs_f64() * 1e3
    );
    let s = suggest(&model, &stock, &request).unwrap();
    let st = model.last_stats();
    println!(
        "prompt {} tok at {:.1} tok/s, generated {} tok at {:.1} tok/s",
        st.prompt_tokens,
        st.prompt_tps(),
        st.generated_tokens,
        st.generate_tps()
    );
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
    let st = model.last_stats();
    println!(
        "recipe: prompt {} tok at {:.1} tok/s, generated {} tok at {:.1} tok/s",
        st.prompt_tokens,
        st.prompt_tps(),
        st.generated_tokens,
        st.generate_tps()
    );
    println!("--- raw ---\n{}", r.raw);
}
