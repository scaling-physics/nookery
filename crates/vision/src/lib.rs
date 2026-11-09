//! Photos of the fridge, freezer or pantry in, a list of what's in them out.
//!
//! The vision model (Ollama `qwen3-vl:8b-instruct` by default; the
//! "thinking" `qwen3-vl:8b` tag reasons for minutes and is too slow) runs
//! locally. Measured on the user's photos (M5, 2026-11-08/09):
//! - Whole photo in one request: the 8B model loops or invents items.
//!   JSON-schema output (`format`) made it pad for minutes: don't use it.
//! - Overlapping, zoomed tiles (fridge 5 strips, pantry 4x2), one short
//!   `name | printed text | count` list per tile: fridge 14 s / 30
//!   sightings, pantry 20 s; ~2-4 s and 40-90 tokens per tile.
//! - Prompt: no concrete food examples (the model copies them); "name the
//!   food inside, not the container" and "don't list the fridge's own
//!   labels" turned "clear jar with green contents" into "pesto" and
//!   dropped "hyper Fresh" drawer labels.
//! - Guards (`ollama.rs`): skip repeated names, cut after 3 in a row, after
//!   6 container-only names in a row, or after 12 items (a pantry spice
//!   shelf once recited 47 spices that weren't there).
//!
//! What the model saw is only a proposal: `nookery_core::reconcile` matches
//! it against the ledger and the user confirms.

mod ollama;
mod parse;
mod plan;
mod tile;

pub use plan::{decide, plan, Decided, Decision, DecisionError, Plan, PlanItem};

use nookery_core::Location;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A part of the photo, in original-image pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// One tile cut from the photo, ready for the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub index: usize,
    /// Where it is in the original photo.
    pub rect: Rect,
    /// The tile, zoomed, as JPEG.
    pub jpeg: Vec<u8>,
}

/// How to cut a photo into tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileOptions {
    pub rows: u32,
    pub cols: u32,
    /// How much neighbouring tiles overlap, in percent of a tile, so an
    /// item on a boundary is whole in at least one tile.
    pub overlap_pct: u32,
    /// Scale factor in percent applied to each tile (140 = 1.4x).
    pub zoom_pct: u32,
    /// Upper bound on the longer side of a zoomed tile, in pixels (the
    /// model's image tokens grow with area).
    pub max_side: u32,
}

impl TileOptions {
    /// Measured on the user's photos (2026-11-09, 924x2000 phone JPEGs):
    /// fridge 5 strips found pesto, ginger, mint, garlic that 4 strips
    /// missed (~18 s uncontended); the crowded pantry reads best as a 4x2
    /// grid (~30-45 s). Freezer: same as the fridge until there's a photo.
    pub fn for_place(place: Location) -> Self {
        match place {
            Location::Fridge | Location::Freezer => Self {
                rows: 5,
                cols: 1,
                overlap_pct: 25,
                zoom_pct: 140,
                max_side: 1400,
            },
            Location::Pantry => Self {
                rows: 4,
                cols: 2,
                overlap_pct: 20,
                zoom_pct: 140,
                max_side: 1400,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VisionError {
    #[error("can't read the image: {0}")]
    Image(String),
    /// Ollama isn't running or isn't reachable.
    #[error("the vision model isn't reachable at {0} (is Ollama running?)")]
    Unreachable(String),
    #[error("the vision model {0} isn't installed (ollama pull {0})")]
    ModelMissing(String),
    #[error("the vision model failed: {0}")]
    Model(String),
    /// A cloud model was asked for but Ollama isn't signed in (HTTP 401).
    #[error("cloud models need an Ollama account: run `ollama signin`")]
    SignIn,
}

/// Cuts the photo (JPEG, PNG or WebP bytes; EXIF orientation applied) into
/// overlapping, zoomed tiles, top-left first, row by row.
pub fn tiles(image: &[u8], opts: &TileOptions) -> Result<Vec<Tile>, VisionError> {
    tile::cut(image, opts)
}

/// One line of the model's answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawItem {
    /// What it is, e.g. "red bell pepper" or "clear jar with green contents".
    pub name: String,
    /// Brand or printed text, e.g. "Arla Skyr"; None for "-".
    pub text: Option<String>,
    pub count: u32,
}

/// Reads `name | printed text | count` lines; skips anything else
/// (preambles, bullets, numbering, blank lines), defaults count to 1.
pub fn parse_lines(answer: &str) -> Vec<RawItem> {
    parse::parse_lines(answer)
}

/// What the model said about one tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAnswer {
    pub items: Vec<RawItem>,
    /// Tokens generated.
    pub tokens: u32,
    /// The answer was cut off because the model began repeating itself.
    pub cut_on_repeat: bool,
}

/// A local vision model. Implementations must not send images off the
/// machine.
pub trait VisionModel: Send + Sync {
    /// e.g. "ollama qwen3-vl:8b-instruct".
    fn name(&self) -> String;
    /// Lists the food in one tile of a photo of `place`.
    fn list_items(&self, jpeg: &[u8], place: Location) -> Result<ModelAnswer, VisionError>;
}

/// The Ollama HTTP API on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ollama {
    /// Default `http://127.0.0.1:11434`, or `$NOOKERY_OLLAMA_URL`.
    pub url: String,
    /// Default `qwen3-vl:8b-instruct`, or `$NOOKERY_VISION_MODEL`.
    pub model: String,
    /// Cap on generated tokens per tile.
    pub max_tokens: u32,
}

impl Ollama {
    pub fn from_env() -> Self {
        let var = |k: &str, default: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| default.to_string())
        };
        Self {
            url: var("NOOKERY_OLLAMA_URL", "http://127.0.0.1:11434"),
            model: var("NOOKERY_VISION_MODEL", "qwen3-vl:8b-instruct"),
            max_tokens: 250,
        }
    }
}

impl VisionModel for Ollama {
    fn name(&self) -> String {
        format!("ollama {}", self.model)
    }

    fn list_items(&self, jpeg: &[u8], place: Location) -> Result<ModelAnswer, VisionError> {
        self.generate(jpeg, place)
    }
}

/// Tries `primary` (the model on this Mac); when it is missing or fails
/// on a tile, `secondary` (the cloud model) answers that tile.
/// `Unreachable` is not retried: cloud models go through the same local
/// Ollama, so it would fail too.
pub struct Fallback<A, B> {
    pub primary: A,
    pub secondary: B,
    fell_back: std::sync::atomic::AtomicBool,
}

impl<A: VisionModel, B: VisionModel> Fallback<A, B> {
    pub fn new(primary: A, secondary: B) -> Self {
        Self {
            primary,
            secondary,
            fell_back: false.into(),
        }
    }

    /// Whether `secondary` answered at least one tile.
    pub fn fell_back(&self) -> bool {
        self.fell_back.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl<A: VisionModel, B: VisionModel> VisionModel for Fallback<A, B> {
    fn name(&self) -> String {
        format!(
            "{} (cloud fallback: {})",
            self.primary.name(),
            self.secondary.name()
        )
    }

    fn list_items(&self, jpeg: &[u8], place: Location) -> Result<ModelAnswer, VisionError> {
        match self.primary.list_items(jpeg, place) {
            Err(VisionError::ModelMissing(_) | VisionError::Model(_)) => {
                self.fell_back
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.secondary.list_items(jpeg, place)
            }
            other => other,
        }
    }
}

/// The vision model for the user's setting.
pub enum Chosen {
    Local(Ollama),
    Cloud(Ollama),
    Fallback(Fallback<Ollama, Ollama>),
}

impl Chosen {
    /// `Ollama::from_env()` on this Mac, `cloud_model` (e.g.
    /// `nookery_core::CLOUD_MODEL`) through the same Ollama for the cloud.
    pub fn for_mode(mode: nookery_core::AiMode, cloud_model: &str) -> Self {
        let local = Ollama::from_env();
        let cloud = Ollama {
            model: cloud_model.to_string(),
            ..local.clone()
        };
        match mode {
            nookery_core::AiMode::Local => Chosen::Local(local),
            nookery_core::AiMode::Cloud => Chosen::Cloud(cloud),
            nookery_core::AiMode::Fallback => Chosen::Fallback(Fallback::new(local, cloud)),
        }
    }

    /// Which model answered, for the user ("ollama gemma4:31b-cloud (cloud)").
    /// Call after the scan.
    pub fn answered_by(&self) -> String {
        match self {
            Chosen::Local(m) => format!("{} (this Mac)", m.name()),
            Chosen::Cloud(m) => format!("{} (cloud)", m.name()),
            Chosen::Fallback(f) if f.fell_back() => format!(
                "{} (cloud, because the model on this Mac failed)",
                f.secondary.name()
            ),
            Chosen::Fallback(f) => format!("{} (this Mac)", f.primary.name()),
        }
    }

    /// Whether anything was sent to the cloud.
    pub fn used_cloud(&self) -> bool {
        match self {
            Chosen::Local(_) => false,
            Chosen::Cloud(_) => true,
            Chosen::Fallback(f) => f.fell_back(),
        }
    }
}

impl VisionModel for Chosen {
    fn name(&self) -> String {
        match self {
            Chosen::Local(m) | Chosen::Cloud(m) => m.name(),
            Chosen::Fallback(f) => f.name(),
        }
    }

    fn list_items(&self, jpeg: &[u8], place: Location) -> Result<ModelAnswer, VisionError> {
        match self {
            Chosen::Local(m) | Chosen::Cloud(m) => m.list_items(jpeg, place),
            Chosen::Fallback(f) => f.list_items(jpeg, place),
        }
    }
}

/// Something seen in the photo, merged across overlapping tiles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sighting {
    pub name: String,
    pub text: Option<String>,
    pub count: u32,
    /// The tiles it was seen in (indexes into `Scan::tiles`).
    pub tiles: Vec<usize>,
}

/// Merges per-tile answers: the same item seen in two overlapping tiles is
/// one sighting (count: the larger of the two, not the sum).
pub fn merge(per_tile: &[(usize, Vec<RawItem>)]) -> Vec<Sighting> {
    parse::merge(per_tile)
}

/// Timing and outcome for one tile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileReport {
    pub index: usize,
    pub rect: Rect,
    pub ms: f64,
    pub tokens: u32,
    pub items: u32,
    pub cut_on_repeat: bool,
}

/// The result of scanning one photo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scan {
    pub sightings: Vec<Sighting>,
    pub model: String,
    pub tiles: Vec<TileReport>,
    /// Decoding and tiling the image.
    pub prepare_ms: f64,
    pub total_ms: f64,
}

/// Tiles the photo, asks the model about each tile in turn (calling
/// `progress(done, total)` after each) and merges the answers.
pub fn scan(
    model: &dyn VisionModel,
    image: &[u8],
    place: Location,
    opts: &TileOptions,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Scan, VisionError> {
    let start = Instant::now();
    let tiles = tiles(image, opts)?;
    let prepare_ms = start.elapsed().as_secs_f64() * 1000.0;
    let total = tiles.len();
    let mut reports = Vec::with_capacity(total);
    let mut per_tile = Vec::with_capacity(total);
    for (done, tile) in tiles.iter().enumerate() {
        let t = Instant::now();
        let (items, tokens, cut_on_repeat) = match model.list_items(&tile.jpeg, place) {
            Ok(a) => (a.items, a.tokens, a.cut_on_repeat),
            // One bad tile shouldn't lose the other three.
            Err(VisionError::Model(_)) => (Vec::new(), 0, false),
            Err(e) => return Err(e),
        };
        reports.push(TileReport {
            index: tile.index,
            rect: tile.rect,
            ms: t.elapsed().as_secs_f64() * 1000.0,
            tokens,
            items: items.len() as u32,
            cut_on_repeat,
        });
        per_tile.push((tile.index, items));
        progress(done + 1, total);
    }
    Ok(Scan {
        sightings: merge(&per_tile),
        model: model.name(),
        tiles: reports,
        prepare_ms,
        total_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn item(name: &str, text: Option<&str>, count: u32) -> RawItem {
        RawItem {
            name: name.into(),
            text: text.map(Into::into),
            count,
        }
    }

    /// Answers tile by tile from a script.
    struct Fake(Mutex<Vec<Result<Vec<RawItem>, VisionError>>>);

    impl VisionModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn list_items(&self, _: &[u8], _: Location) -> Result<ModelAnswer, VisionError> {
            let next = self.0.lock().unwrap().remove(0)?;
            Ok(ModelAnswer {
                items: next,
                tokens: 10,
                cut_on_repeat: false,
            })
        }
    }

    /// A fixed layout, so tests don't change when the defaults are tuned.
    fn four_strips() -> TileOptions {
        TileOptions {
            rows: 4,
            cols: 1,
            overlap_pct: 25,
            zoom_pct: 140,
            max_side: 1400,
        }
    }

    fn test_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
        });
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut out)
            .encode_image(&img)
            .unwrap();
        out
    }

    fn dims(jpeg: &[u8]) -> (u32, u32) {
        image::load_from_memory(jpeg)
            .unwrap()
            .to_rgb8()
            .dimensions()
    }

    #[test]
    fn fridge_and_pantry_defaults_differ_in_columns() {
        assert_eq!(TileOptions::for_place(Location::Fridge).cols, 1);
        assert_eq!(TileOptions::for_place(Location::Freezer).rows, 5);
        let p = TileOptions::for_place(Location::Pantry);
        assert_eq!((p.rows, p.cols, p.overlap_pct), (4, 2, 20));
    }

    #[test]
    fn tiles_cover_the_image_with_overlap() {
        let t = tiles(&test_jpeg(300, 1000), &four_strips()).unwrap();
        assert_eq!(t.len(), 4);
        assert_eq!(t[0].rect.y, 0);
        let last = t[3].rect;
        assert_eq!(last.y + last.h, 1000);
        for pair in t.windows(2) {
            let (a, b) = (pair[0].rect, pair[1].rect);
            assert!(b.y < a.y + a.h, "tiles overlap");
            assert!(b.y > a.y);
            assert_eq!((a.x, a.w), (0, 300));
        }
        assert_eq!(t.iter().map(|t| t.index).collect::<Vec<_>>(), [0, 1, 2, 3]);
    }

    #[test]
    fn tiles_are_zoomed() {
        let opts = TileOptions {
            rows: 2,
            cols: 1,
            overlap_pct: 0,
            zoom_pct: 200,
            max_side: 10_000,
        };
        let t = tiles(&test_jpeg(100, 200), &opts).unwrap();
        assert_eq!(t[0].rect.h, 100);
        assert_eq!(dims(&t[0].jpeg), (200, 200));
    }

    #[test]
    fn tiles_longer_side_is_capped_at_max_side() {
        let opts = TileOptions {
            rows: 1,
            cols: 1,
            overlap_pct: 0,
            zoom_pct: 140,
            max_side: 250,
        };
        let t = tiles(&test_jpeg(200, 100), &opts).unwrap();
        assert_eq!(dims(&t[0].jpeg), (250, 125));
    }

    #[test]
    fn pantry_grid_makes_eight_tiles() {
        let t = tiles(
            &test_jpeg(200, 800),
            &TileOptions::for_place(Location::Pantry),
        )
        .unwrap();
        assert_eq!(t.len(), 8);
        assert!(t.iter().all(|t| t.rect.w > 0 && t.rect.h > 0));
    }

    #[test]
    fn degenerate_images_do_not_panic_or_yield_empty_tiles() {
        for (w, h) in [(1, 1), (2000, 3), (3, 2000)] {
            let t = tiles(&test_jpeg(w, h), &TileOptions::for_place(Location::Pantry)).unwrap();
            for tile in t {
                assert!(tile.rect.w >= 1 && tile.rect.h >= 1);
                let (zw, zh) = dims(&tile.jpeg);
                assert!(zw >= 1 && zh >= 1);
            }
        }
    }

    #[test]
    fn garbage_bytes_are_an_image_error() {
        let err = tiles(b"not an image", &four_strips()).unwrap_err();
        assert!(matches!(err, VisionError::Image(_)));
    }

    #[test]
    fn png_input_works_too() {
        let img = image::RgbaImage::new(50, 80);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let t = tiles(buf.get_ref(), &four_strips()).unwrap();
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn parse_lines_reads_clean_lines() {
        let got = parse_lines("red bell pepper | - | 2\nSkyr | Arla Skyr | 1");
        assert_eq!(
            got,
            vec![
                item("red bell pepper", None, 2),
                item("Skyr", Some("Arla Skyr"), 1)
            ]
        );
    }

    #[test]
    fn parse_lines_survives_messy_input() {
        let answer = "Here is the list:\n\
            name | printed text | count\n\
            1. milk carton |  Weihenstephan  | 1\n\
            - eggs | - | 2x\n\
            * butter | — |   x3  \n\
            2) jam jar |   \n\
            no pipe here\n\
            cheese | Gouda\n\
            \n\
            |  | 2\n\
              yoghurt  |  | \n\
            tomato | | 0";
        let got = parse_lines(answer);
        let names: Vec<_> = got.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "milk carton",
                "eggs",
                "butter",
                "jam jar",
                "cheese",
                "yoghurt",
                "tomato"
            ]
        );
        assert_eq!(got[0], item("milk carton", Some("Weihenstephan"), 1));
        assert_eq!(got[1], item("eggs", None, 2));
        assert_eq!(got[2], item("butter", None, 3));
        assert_eq!(got[3], item("jam jar", None, 1));
        assert_eq!(got[4], item("cheese", Some("Gouda"), 1));
        assert_eq!(got[5], item("yoghurt", None, 1));
        assert_eq!(got[6].count, 1);
    }

    #[test]
    fn parse_lines_of_nothing_is_empty() {
        assert!(parse_lines("").is_empty());
        assert!(parse_lines("I can't see anything.").is_empty());
    }

    #[test]
    fn merge_takes_the_larger_count_not_the_sum() {
        let got = merge(&[
            (0, vec![item("milk", None, 2)]),
            (1, vec![item("Milk", None, 3), item("eggs", None, 1)]),
        ]);
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].count, got[0].tiles.clone()), (3, vec![0, 1]));
        assert_eq!(got[1].tiles, vec![1]);
    }

    #[test]
    fn merge_joins_singular_and_plural() {
        let got = merge(&[
            (
                0,
                vec![item("tomato", None, 1), item("cherry tomatoes", None, 1)],
            ),
            (
                1,
                vec![
                    item("tomatoes", None, 4),
                    item("berries", None, 1),
                    item("berry", None, 2),
                ],
            ),
        ]);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].count, 4);
        assert_eq!(got[0].name, "tomatoes");
        assert_eq!(got[2].count, 2);
    }

    #[test]
    fn merge_keeps_the_longest_text_and_fills_a_missing_one() {
        let got = merge(&[
            (0, vec![item("skyr", None, 1)]),
            (1, vec![item("skyr", Some("Arla Skyr natural"), 1)]),
            (2, vec![item("skyr", Some("Arla"), 1)]),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].text.as_deref(), Some("Arla Skyr natural"));
        assert_eq!(got[0].tiles, vec![0, 1, 2]);
    }

    #[test]
    fn merge_keeps_same_name_with_different_brands_apart() {
        let got = merge(&[(
            0,
            vec![
                item("yoghurt", Some("Alpro"), 1),
                item("yoghurt", Some("Danone"), 1),
            ],
        )]);
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn merge_orders_by_first_tile_then_appearance() {
        let got = merge(&[
            (1, vec![item("b", None, 1)]),
            (0, vec![item("c", None, 1), item("a", None, 1)]),
        ]);
        let names: Vec<_> = got.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["c", "a", "b"]);
    }

    #[test]
    fn scan_reports_progress_and_merges_tiles() {
        let model = Fake(Mutex::new(vec![
            Ok(vec![item("milk", None, 1)]),
            Ok(vec![item("milk", Some("Arla"), 1), item("eggs", None, 6)]),
            Ok(vec![]),
            Ok(vec![item("jam", None, 1)]),
        ]));
        let mut calls = Vec::new();
        let scan = scan(
            &model,
            &test_jpeg(100, 400),
            Location::Fridge,
            &four_strips(),
            &mut |d, n| calls.push((d, n)),
        )
        .unwrap();
        assert_eq!(calls, [(1, 4), (2, 4), (3, 4), (4, 4)]);
        assert_eq!(scan.model, "fake");
        assert_eq!(scan.tiles.len(), 4);
        assert_eq!(scan.tiles[1].items, 2);
        assert_eq!(scan.tiles[1].tokens, 10);
        assert_eq!(scan.sightings.len(), 3);
        assert_eq!(scan.sightings[0].text.as_deref(), Some("Arla"));
        assert!(scan.total_ms >= scan.prepare_ms);
    }

    #[test]
    fn scan_carries_on_after_a_failed_tile() {
        let model = Fake(Mutex::new(vec![
            Ok(vec![item("milk", None, 1)]),
            Err(VisionError::Model("boom".into())),
            Ok(vec![]),
            Ok(vec![]),
        ]));
        let scan = scan(
            &model,
            &test_jpeg(100, 400),
            Location::Fridge,
            &four_strips(),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(scan.tiles[1].items, 0);
        assert_eq!(scan.sightings.len(), 1);
    }

    #[test]
    fn scan_aborts_when_the_model_is_unreachable_or_missing() {
        for err in [
            VisionError::Unreachable("u".into()),
            VisionError::ModelMissing("m".into()),
        ] {
            let model = Fake(Mutex::new(vec![Err(err)]));
            let mut calls = 0;
            let r = scan(
                &model,
                &test_jpeg(100, 400),
                Location::Fridge,
                &four_strips(),
                &mut |_, _| calls += 1,
            );
            assert!(r.is_err());
            assert_eq!(calls, 0);
        }
    }

    #[test]
    fn ollama_defaults() {
        let o = Ollama::from_env();
        assert_eq!(o.max_tokens, 250);
        assert!(!o.url.is_empty() && !o.model.is_empty());
    }

    fn call(m: &impl VisionModel) -> Result<ModelAnswer, VisionError> {
        m.list_items(&[], Location::Fridge)
    }

    #[test]
    fn fallback_leaves_the_secondary_alone_when_the_primary_answers() {
        let f = Fallback::new(
            Fake(Mutex::new(vec![Ok(vec![item("milk", None, 1)])])),
            Fake(Mutex::new(vec![Ok(vec![])])),
        );
        assert_eq!(call(&f).unwrap().items.len(), 1);
        assert!(!f.fell_back());
        assert_eq!(f.secondary.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn fallback_uses_the_secondary_when_the_primary_is_missing_or_fails() {
        for err in [
            VisionError::ModelMissing("m".into()),
            VisionError::Model("boom".into()),
        ] {
            let f = Fallback::new(
                Fake(Mutex::new(vec![Err(err)])),
                Fake(Mutex::new(vec![Ok(vec![item("eggs", None, 6)])])),
            );
            let a = call(&f).unwrap();
            assert_eq!(a.items[0].name, "eggs");
            assert!(f.fell_back());
        }
    }

    #[test]
    fn fallback_does_not_retry_unreachable_or_sign_in() {
        for err in [VisionError::Unreachable("u".into()), VisionError::SignIn] {
            let want = format!("{err:?}");
            let f = Fallback::new(
                Fake(Mutex::new(vec![Err(err)])),
                Fake(Mutex::new(vec![Ok(vec![])])),
            );
            let e = call(&f).unwrap_err();
            assert_eq!(format!("{e:?}"), want);
            assert!(!f.fell_back());
            assert_eq!(f.secondary.0.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn chosen_picks_the_model_for_the_mode() {
        use nookery_core::AiMode;
        let local = Chosen::for_mode(AiMode::Local, "some:cloud");
        assert!(matches!(local, Chosen::Local(_)));
        assert!(local.answered_by().ends_with("(this Mac)"));
        assert!(!local.used_cloud());

        let cloud = Chosen::for_mode(AiMode::Cloud, "some:cloud");
        assert_eq!(cloud.answered_by(), "ollama some:cloud (cloud)");
        assert!(cloud.used_cloud());

        let fb = Chosen::for_mode(AiMode::Fallback, "some:cloud");
        assert!(fb.name().contains("cloud fallback: ollama some:cloud"));
        assert!(!fb.used_cloud());
        assert!(fb.answered_by().ends_with("(this Mac)"));
    }
}
