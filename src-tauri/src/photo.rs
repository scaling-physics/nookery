//! Photo scanning: thin commands over `nookery-vision` and the store.
//!
//! `scan_photo` runs the local vision model (15-60 s) WITHOUT holding the
//! store lock, keeps the resulting plan in `AppState` under a scan id, and
//! returns what to review. `apply_photo` looks the plan up by that id, so the
//! model is never run twice.
//!
//! `apply_photo` reports a bad field as `BadInput("<field>:<index>")`
//! (`action:2`, `item:2`, `qty:2`, `bestBefore:2`) or `BadInput("index:<i>")`
//! for an index that is not in the scan or is decided twice.

use base64::Engine;
use chrono::{DateTime, Local, NaiveDate, Utc};
use nookery_core::{parse_date, status, Location, LotId, Qty};
use nookery_store::{Store, PHOTO_ALIASES, SETTING_PHOTOS_AI};
use nookery_vision::{
    decide, plan, scan, Chosen, Decision, DecisionError, Plan, Scan, TileOptions, VisionError,
    VisionModel,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::Channel;
use tauri::State;

use super::{bad, phone, view, AppState, CommandError, LotView};

/// The last scan, kept until it is applied or replaced.
pub struct StoredScan {
    id: String,
    plan: Plan,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TileView {
    pub index: u32,
    pub ms: f64,
    pub items: u32,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PhotoItemView {
    /// The id to answer with in `apply_photo`.
    pub index: u32,
    /// What the model said.
    pub seen_as: String,
    /// The learned name if there is one, else what the model said.
    pub item: String,
    pub learned: bool,
    /// Text read off the packaging, if any.
    pub text: Option<String>,
    pub count: u32,
    /// In-stock lots it matched; empty means new.
    pub matches: Vec<LotView>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PhotoReviewView {
    pub scan_id: String,
    pub location: String,
    /// Which model answered, e.g. "ollama x (cloud)".
    pub model: String,
    pub used_cloud: bool,
    pub total_ms: f64,
    pub tiles: Vec<TileView>,
    pub items: Vec<PhotoItemView>,
    /// In stock in this place but not in the photo (nothing is removed).
    pub missing: Vec<LotView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PhotoAction {
    Seen,
    Add,
    Ignore,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PhotoDecisionInput {
    pub index: u32,
    pub action: PhotoAction,
    pub item: Option<String>,
    pub qty: Option<String>,
    pub best_before: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPhotoInput {
    pub scan_id: String,
    pub decisions: Vec<PhotoDecisionInput>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPhotoResult {
    pub seen: u32,
    pub added: u32,
    pub learned: u32,
}

impl From<VisionError> for CommandError {
    fn from(e: VisionError) -> Self {
        match e {
            VisionError::Image(_) => bad("image"),
            VisionError::Unreachable(url) => Self::VisionUnavailable(url),
            VisionError::ModelMissing(model) => Self::VisionModelMissing(model),
            VisionError::SignIn => Self::CloudSignIn,
            VisionError::NoCredits => Self::CloudCredits,
            VisionError::Model(m) => {
                eprintln!("nookery: vision model: {m}");
                Self::Internal
            }
        }
    }
}

/// Runs the model over the photo. Touches no store.
pub fn run_scan(
    model: &dyn VisionModel,
    image: &[u8],
    place: Location,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Scan, CommandError> {
    Ok(scan(
        model,
        image,
        place,
        &TileOptions::for_place(place),
        progress,
    )?)
}

/// Matches a scan against the ledger and builds what the user reviews.
pub fn review_in(
    store: &Store,
    scan: &Scan,
    place: Location,
    today: NaiveDate,
    scan_id: &str,
) -> Result<(Plan, PhotoReviewView), CommandError> {
    let aliases = store.aliases(PHOTO_ALIASES)?;
    let p = plan(scan, &aliases, store.inventory().in_stock(), place);
    let inv = store.inventory();
    let lot_view = |id: &LotId| inv.get(*id).map(|l| view(l, status(l, today)));
    let items = p
        .items
        .iter()
        .map(|it| {
            let s = &scan.sightings[it.sighting];
            PhotoItemView {
                index: it.sighting as u32,
                seen_as: it.seen_as.clone(),
                item: it.item.clone(),
                learned: it.learned,
                text: s.text.clone(),
                count: it.count,
                matches: it.lots.iter().filter_map(lot_view).collect(),
            }
        })
        .collect();
    let missing = p.missing.iter().filter_map(lot_view).collect();
    let review = PhotoReviewView {
        scan_id: scan_id.into(),
        location: place.as_str().into(),
        model: scan.model.clone(),
        used_cloud: false,
        total_ms: scan.total_ms,
        tiles: scan
            .tiles
            .iter()
            .map(|t| TileView {
                index: t.index as u32,
                ms: t.ms,
                items: t.items,
            })
            .collect(),
        items,
        missing,
    };
    Ok((p, review))
}

#[cfg(test)]
/// Scan, then review against `store` (the command does the two steps
/// separately so the store isn't locked during the scan).
pub fn scan_photo_in(
    model: &dyn VisionModel,
    image: &[u8],
    place: Location,
    store: &Store,
    today: NaiveDate,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<(Plan, PhotoReviewView), CommandError> {
    let s = run_scan(model, image, place, progress)?;
    review_in(store, &s, place, today, &LotId::new().to_string())
}

pub fn apply_photo_in(
    store: &mut Store,
    plan: &Plan,
    decisions: &[PhotoDecisionInput],
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<ApplyPhotoResult, CommandError> {
    let mut decided = Vec::with_capacity(decisions.len());
    for d in decisions {
        let n = d.index;
        let it = plan
            .items
            .get(n as usize)
            .ok_or_else(|| bad(&format!("index:{n}")))?;
        let decision = match d.action {
            PhotoAction::Seen => Decision::Seen,
            PhotoAction::Ignore => Decision::Ignore,
            PhotoAction::Add => {
                let item = d.item.as_deref().unwrap_or(&it.item).trim().to_string();
                if item.is_empty() {
                    return Err(bad(&format!("item:{n}")));
                }
                let qty = match d.qty.as_deref().map(str::trim) {
                    None | Some("") => Qty::pieces(it.count.max(1) as i64),
                    Some(q) => q.parse().map_err(|_| bad(&format!("qty:{n}")))?,
                };
                let best_before = match d.best_before.as_deref().map(str::trim) {
                    None | Some("") => None,
                    Some(s) => {
                        Some(parse_date(s, today).ok_or_else(|| bad(&format!("bestBefore:{n}")))?)
                    }
                };
                Decision::Add {
                    item,
                    qty,
                    best_before,
                }
            }
        };
        decided.push((n as usize, decision));
    }
    let (events, learned) = decide(plan, &decided, now).map_err(|e| match e {
        DecisionError::BadSighting(i) | DecisionError::Duplicate(i) => bad(&format!("index:{i}")),
        DecisionError::NothingMatched(i) => bad(&format!("action:{i}")),
        DecisionError::NoName(i) => bad(&format!("item:{i}")),
    })?;
    let added = events
        .iter()
        .filter(|e| matches!(e.kind, nookery_core::EventKind::Acquired { .. }))
        .count() as u32;
    store
        .append_learning(&events, PHOTO_ALIASES, &learned, now)
        .map_err(|e| match e {
            nookery_store::StoreError::Ledger(_) => bad("decisions"),
            other => other.into(),
        })?;
    Ok(ApplyPhotoResult {
        seen: events.len() as u32 - added,
        added,
        learned: learned.len() as u32,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn scan_photo(
    state: State<'_, AppState>,
    image_base64: String,
    location: String,
    on_progress: Channel<ScanProgress>,
) -> Result<PhotoReviewView, CommandError> {
    let place: Location = location.parse().map_err(|_| bad("location"))?;
    let image = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|_| bad("image"))?;
    let (scan, answered_by, used_cloud) = if cfg!(mobile) {
        // The phone has no vision model: the paired Mac reads the photo
        // (with its own AI setting) and the review is built here.
        let target = phone::mac_target(&state)?;
        let a = tauri::async_runtime::spawn_blocking(move || {
            phone::scan_via_mac(&target, &image, place, &mut |done, total| {
                let _ = on_progress.send(ScanProgress { done, total });
            })
        })
        .await
        .map_err(|_| CommandError::Internal)??;
        (a.scan, a.answered_by, a.used_cloud)
    } else {
        let (mode, cloud_model) = {
            let store = state.store()?;
            (store.ai_mode(SETTING_PHOTOS_AI)?, store.cloud_model()?)
        };
        // The model runs on a blocking thread and with the store unlocked.
        tauri::async_runtime::spawn_blocking(move || {
            let model = Chosen::for_mode(mode, &cloud_model);
            let s = run_scan(&model, &image, place, &mut |done, total| {
                let _ = on_progress.send(ScanProgress {
                    done: done as u32,
                    total: total as u32,
                });
            })?;
            Ok::<_, CommandError>((s, model.answered_by(), model.used_cloud()))
        })
        .await
        .map_err(|_| CommandError::Internal)??
    };
    let id = LotId::new().to_string();
    let (p, mut review) = {
        let store = state.store()?;
        review_in(&store, &scan, place, Local::now().date_naive(), &id)?
    };
    review.model = answered_by;
    review.used_cloud = used_cloud;
    *state.last_scan.lock().map_err(|_| CommandError::Internal)? = Some(StoredScan { id, plan: p });
    Ok(review)
}

#[tauri::command]
#[specta::specta]
pub fn apply_photo(
    state: State<'_, AppState>,
    input: ApplyPhotoInput,
) -> Result<ApplyPhotoResult, CommandError> {
    let mut last = state.last_scan.lock().map_err(|_| CommandError::Internal)?;
    let stored = last
        .as_ref()
        .filter(|s| s.id == input.scan_id)
        .ok_or(CommandError::NotFound)?;
    let mut store = state.store()?;
    let r = apply_photo_in(
        &mut store,
        &stored.plan,
        &input.decisions,
        Local::now().date_naive(),
        Utc::now(),
    )?;
    // Applied: the plan is spent (applying twice would double-add).
    *last = None;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nookery_core::{Event, EventKind, Source};
    use nookery_vision::{ModelAnswer, RawItem};
    use std::sync::Mutex;

    struct Fake(Mutex<Vec<Result<Vec<RawItem>, VisionError>>>);

    impl VisionModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn list_items(&self, _: &[u8], _: Location) -> Result<ModelAnswer, VisionError> {
            let mut script = self.0.lock().unwrap();
            // Tiles beyond the script see nothing (the tiling may change).
            let items = if script.is_empty() {
                Vec::new()
            } else {
                script.remove(0)?
            };
            Ok(ModelAnswer {
                items,
                tokens: 5,
                cut_on_repeat: false,
            })
        }
    }

    fn raw(name: &str, count: u32) -> RawItem {
        RawItem {
            name: name.into(),
            text: None,
            count,
        }
    }

    fn png() -> Vec<u8> {
        let img = image::RgbImage::from_fn(40, 160, |x, y| image::Rgb([x as u8, y as u8, 80]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    fn store_with_milk_and_eggs() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        for item in ["milk", "eggs"] {
            s.append(&[Event {
                at: Utc::now(),
                lot: LotId::new(),
                kind: EventKind::Acquired {
                    item: item.into(),
                    qty: Qty::pieces(1),
                    location: Location::Fridge,
                    source: Source::Manual,
                    best_before: None,
                },
            }])
            .unwrap();
        }
        s
    }

    fn scripted() -> Fake {
        Fake(Mutex::new(vec![
            Ok(vec![raw("milk", 1)]),
            Ok(vec![raw("clear jar with green contents", 1)]),
            Ok(vec![]),
            Ok(vec![]),
        ]))
    }

    fn decision(index: u32, action: PhotoAction) -> PhotoDecisionInput {
        PhotoDecisionInput {
            index,
            action,
            item: None,
            qty: None,
            best_before: None,
        }
    }

    #[test]
    fn scans_matches_and_applies_with_learning() {
        let mut store = store_with_milk_and_eggs();
        let mut ticks = Vec::new();
        let (p, review) = scan_photo_in(
            &scripted(),
            &png(),
            Location::Fridge,
            &store,
            today(),
            &mut |d, t| ticks.push((d, t)),
        )
        .unwrap();
        let total = ticks.len();
        assert!(total >= 2);
        assert_eq!(ticks.last(), Some(&(total, total)));
        assert_eq!(review.tiles.len(), total);
        assert_eq!(review.model, "fake");
        assert!(!review.used_cloud);
        assert_eq!(review.items.len(), 2);
        let milk = &review.items[0];
        assert_eq!((milk.seen_as.as_str(), milk.matches.len()), ("milk", 1));
        assert_eq!(milk.matches[0].item, "milk");
        let jar = &review.items[1];
        assert!(jar.matches.is_empty() && !jar.learned);
        assert_eq!(review.missing.len(), 1);
        assert_eq!(review.missing[0].item, "eggs");

        let mut add = decision(1, PhotoAction::Add);
        add.item = Some("pesto".into());
        add.best_before = Some("+10".into());
        let r = apply_photo_in(
            &mut store,
            &p,
            &[decision(0, PhotoAction::Seen), add],
            today(),
            Utc::now(),
        )
        .unwrap();
        assert_eq!((r.seen, r.added, r.learned), (1, 1, 1));
        assert!(store.inventory().in_stock().any(|l| l.item == "pesto"));

        // The next scan of the same jar already says "pesto".
        let (_, again) = scan_photo_in(
            &scripted(),
            &png(),
            Location::Fridge,
            &store,
            today(),
            &mut |_, _| {},
        )
        .unwrap();
        assert!(again.items[1].learned && again.items[1].item == "pesto");
        assert_eq!(again.items[1].matches.len(), 1);
    }

    #[test]
    fn bad_decisions_name_the_field_and_index() {
        let mut store = store_with_milk_and_eggs();
        let (p, _) = scan_photo_in(
            &scripted(),
            &png(),
            Location::Fridge,
            &store,
            today(),
            &mut |_, _| {},
        )
        .unwrap();
        let field = |e: CommandError| match e {
            CommandError::BadInput(f) => f,
            other => panic!("{other:?}"),
        };
        let mut run = |d: PhotoDecisionInput| {
            field(apply_photo_in(&mut store, &p, &[d], today(), Utc::now()).unwrap_err())
        };
        assert_eq!(run(decision(1, PhotoAction::Seen)), "action:1");
        assert_eq!(run(decision(9, PhotoAction::Ignore)), "index:9");
        let mut d = decision(1, PhotoAction::Add);
        d.qty = Some("lots".into());
        assert_eq!(run(d), "qty:1");
        let mut d = decision(1, PhotoAction::Add);
        d.best_before = Some("x".into());
        assert_eq!(run(d), "bestBefore:1");
        let mut d = decision(1, PhotoAction::Add);
        d.item = Some(" ".into());
        assert_eq!(run(d), "item:1");
        assert_eq!(store.event_count(), 2);
    }

    #[test]
    fn vision_errors_map_to_command_errors() {
        let store = Store::open_in_memory().unwrap();
        let go = |model: Fake, image: &[u8]| {
            scan_photo_in(
                &model,
                image,
                Location::Fridge,
                &store,
                today(),
                &mut |_, _| {},
            )
            .unwrap_err()
        };
        let down = Fake(Mutex::new(vec![Err(VisionError::Unreachable(
            "http://x".into(),
        ))]));
        assert!(matches!(go(down, &png()), CommandError::VisionUnavailable(u) if u == "http://x"));
        let missing = Fake(Mutex::new(vec![Err(VisionError::ModelMissing(
            "m:1".into(),
        ))]));
        assert!(matches!(go(missing, &png()), CommandError::VisionModelMissing(m) if m == "m:1"));
        assert!(
            matches!(go(scripted(), b"not an image"), CommandError::BadInput(f) if f == "image")
        );
    }
}
