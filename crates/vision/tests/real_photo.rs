//! Runs the real model on the user's private photos. Slow (~35 s), needs
//! Ollama: `cargo test -p nookery-vision --test real_photo -- --ignored --nocapture`.

use nookery_core::Location;
use nookery_vision::{scan, Ollama, TileOptions};
use std::path::PathBuf;

fn run(file: &str, place: Location) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/private/photos")
        .join(file);
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let model = Ollama::from_env();
    let scan = scan(
        &model,
        &bytes,
        place,
        &TileOptions::for_place(place),
        &mut |d, n| eprintln!("tile {d}/{n}"),
    )
    .unwrap();
    for s in &scan.sightings {
        println!(
            "{} x{} [{}] tiles {:?}",
            s.name,
            s.count,
            s.text.as_deref().unwrap_or("-"),
            s.tiles
        );
    }
    for t in &scan.tiles {
        println!(
            "tile {}: {:.0} ms, {} tokens, {} items, cut_on_repeat={}",
            t.index, t.ms, t.tokens, t.items, t.cut_on_repeat
        );
    }
    println!(
        "prepare {:.0} ms, total {:.0} ms",
        scan.prepare_ms, scan.total_ms
    );
    assert!(scan.sightings.len() >= 5);
}

#[test]
#[ignore = "needs Ollama and a private photo"]
fn fridge_photo_gives_at_least_five_sightings() {
    run("fridge-1.jpg", Location::Fridge);
}

#[test]
#[ignore = "needs Ollama and a private photo"]
fn pantry_photo_gives_at_least_five_sightings() {
    run("pantry-1.jpg", Location::Pantry);
}
