use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use nookery_core::{Event, EventKind, Location, LotId, Qty, Source};
use nookery_recipes::{RecipeError, TextModel};
use nookery_store::{Doc, Store, DOC_SHOPPING};
use nookery_sync::*;
use sha2::{Digest, Sha256};

fn acquire(item: &str) -> Event {
    Event {
        at: Utc::now(),
        lot: LotId::new(),
        kind: EventKind::Acquired {
            item: item.into(),
            qty: Qty::pieces(2),
            location: Location::Fridge,
            source: Source::Manual,
            best_before: None,
        },
    }
}

struct Fake;
impl TextModel for Fake {
    fn name(&self) -> String {
        "fake".into()
    }
    fn generate(
        &self,
        prompt: &str,
        _max: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        if prompt == "boom" {
            return Err(RecipeError::Model("boom".into()));
        }
        on_text("Hel");
        on_text("lo");
        Ok("Hello".into())
    }
}

struct Wrap {
    inner: StoreProvider,
    models: std::path::PathBuf,
}
impl Provider for Wrap {
    fn device(&self) -> String {
        self.inner.device()
    }
    fn secret(&self) -> Option<Secret> {
        self.inner.secret()
    }
    fn sync(
        &self,
        e: &[(String, Event)],
        d: &[Doc],
        s: Option<chrono::DateTime<Utc>>,
    ) -> Result<SyncReply, String> {
        self.inner.sync(e, d, s)
    }
    fn models_dir(&self) -> std::path::PathBuf {
        self.models.clone()
    }
    fn text_model(&self) -> Box<dyn TextModel> {
        Box::new(Fake)
    }
    fn scan(
        &self,
        image: &[u8],
        place: Location,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<ScanAnswer, String> {
        scan_with(
            &FakeEyes,
            "fake eyes (this Mac)".into(),
            false,
            image,
            place,
            progress,
        )
    }
}

struct FakeEyes;
impl nookery_vision::VisionModel for FakeEyes {
    fn name(&self) -> String {
        "fake eyes".into()
    }
    fn list_items(
        &self,
        _: &[u8],
        _: Location,
    ) -> Result<nookery_vision::ModelAnswer, nookery_vision::VisionError> {
        Ok(nookery_vision::ModelAnswer {
            items: vec![nookery_vision::RawItem {
                name: "milk".into(),
                text: None,
                count: 1,
            }],
            tokens: 3,
            cut_on_repeat: false,
        })
    }
}

struct Rig {
    mac: Arc<Mutex<Store>>,
    phone: Store,
    secret: Secret,
    server: Server,
    _dirs: Vec<tempfile::TempDir>,
}
impl Rig {
    fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.server.port())
    }
    fn session(&self) -> Session {
        connect(&self.addr(), &self.secret).unwrap()
    }
}

const MODEL: &str = "qwen3:4b-instruct";

fn fake_ollama(dir: &Path, content: &[u8], digest: &str) {
    let m = dir.join("manifests/registry.ollama.ai/library/qwen3");
    std::fs::create_dir_all(&m).unwrap();
    std::fs::write(
        m.join("4b-instruct"),
        format!(
            r#"{{"layers":[{{"mediaType":"application/vnd.ollama.image.license","digest":"sha256:{z}"}},{{"mediaType":"application/vnd.ollama.image.model","digest":"sha256:{digest}"}}]}}"#,
            z = "0".repeat(64)
        ),
    )
    .unwrap();
    let b = dir.join("blobs");
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(b.join(format!("sha256-{digest}")), content).unwrap();
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn rig() -> Rig {
    let secret = Secret::generate().unwrap();
    let mut mac = Store::open_in_memory().unwrap();
    mac.set_setting(SETTING_SYNC_SECRET, &secret.to_hex())
        .unwrap();
    let mac = Arc::new(Mutex::new(mac));
    let models = tempfile::tempdir().unwrap();
    let content = vec![7u8; 9_000_000]; // 3 chunks
    fake_ollama(models.path(), &content, &hex(&content));
    let provider = Wrap {
        inner: StoreProvider {
            store: mac.clone(),
            device: "mac".into(),
        },
        models: models.path().to_path_buf(),
    };
    let server = start("127.0.0.1", 0, Arc::new(provider)).unwrap();
    Rig {
        mac,
        phone: Store::open_in_memory().unwrap(),
        secret,
        server,
        _dirs: vec![models],
    }
}

fn uids(s: &Store) -> Vec<String> {
    s.synced_events_after(0)
        .unwrap()
        .into_iter()
        .map(|e| e.uid)
        .collect()
}

#[test]
fn full_round_both_ways_and_retry_duplicates_nothing() {
    let mut r = rig();
    r.mac.lock().unwrap().append(&[acquire("milk")]).unwrap();
    r.phone.append(&[acquire("eggs")]).unwrap();
    let mut peer = r.session();
    peer.hello("phone").unwrap();
    let out = sync_now(&mut r.phone, &mut peer).unwrap();
    assert_eq!(out.pushed, 1);
    assert!(out.rejected.is_empty());
    assert_eq!(out.pulled_events, 2);
    assert_eq!(r.phone.inventory().all().len(), 2);
    assert_eq!(r.mac.lock().unwrap().inventory().all().len(), 2);
    assert_eq!(uids(&r.phone), uids(&r.mac.lock().unwrap()));
    // again: nothing new
    let out = sync_now(&mut r.phone, &mut peer).unwrap();
    assert_eq!(out.pushed, 0);
    assert_eq!(r.mac.lock().unwrap().event_count(), 2);
    // A retry of the same push (as after a lost reply) is a duplicate.
    let req = prepare(&r.phone).unwrap();
    assert_eq!(req.event_count(), 0);
    let ev = r.phone.synced_events_after(0).unwrap();
    let mac_count = r.mac.lock().unwrap().event_count();
    let again = r
        .mac
        .lock()
        .unwrap()
        .merge_events(&[(ev[0].uid.clone(), ev[0].event.clone())])
        .unwrap();
    assert_eq!(again.duplicate.len(), 1);
    assert_eq!(r.mac.lock().unwrap().event_count(), mac_count);
}

#[test]
fn conflicting_phone_event_is_rejected_and_phone_matches_mac() {
    let mut r = rig();
    let milk = acquire("milk");
    r.mac
        .lock()
        .unwrap()
        .append(std::slice::from_ref(&milk))
        .unwrap();
    let mut peer = r.session();
    sync_now(&mut r.phone, &mut peer).unwrap();
    // phone uses the milk, the Mac bins it meanwhile
    r.phone
        .append(&[Event {
            at: Utc::now(),
            lot: milk.lot,
            kind: EventKind::Used { qty: None },
        }])
        .unwrap();
    r.mac
        .lock()
        .unwrap()
        .append(&[Event {
            at: Utc::now(),
            lot: milk.lot,
            kind: EventKind::Binned,
        }])
        .unwrap();
    let out = sync_now(&mut r.phone, &mut peer).unwrap();
    assert_eq!(out.pushed, 0);
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(uids(&r.phone), uids(&r.mac.lock().unwrap()));
    assert_eq!(r.phone.event_count(), 2);
}

#[test]
fn docs_sync_both_ways_including_deletion() {
    let mut r = rig();
    let t0 = Utc::now();
    let body = serde_json::json!({"name": "oats"});
    r.mac
        .lock()
        .unwrap()
        .put_doc(DOC_SHOPPING, "a", &body, t0)
        .unwrap();
    r.phone
        .put_doc(DOC_SHOPPING, "b", &body, t0 + Duration::seconds(1))
        .unwrap();
    let mut peer = r.session();
    let out = sync_now(&mut r.phone, &mut peer).unwrap();
    assert!(out.docs_changed >= 1);
    assert!(r.phone.doc(DOC_SHOPPING, "a").unwrap().is_some());
    assert!(r
        .mac
        .lock()
        .unwrap()
        .doc(DOC_SHOPPING, "b")
        .unwrap()
        .is_some());
    // the phone deletes "a" later; the Mac gets the tombstone
    r.phone
        .delete_doc(DOC_SHOPPING, "a", Utc::now() + Duration::seconds(5))
        .unwrap();
    sync_now(&mut r.phone, &mut peer).unwrap();
    assert!(r.mac.lock().unwrap().docs(DOC_SHOPPING).unwrap().len() == 1);
    assert!(
        r.mac
            .lock()
            .unwrap()
            .doc(DOC_SHOPPING, "a")
            .unwrap()
            .unwrap()
            .deleted
    );
    // the Mac deletes "b"; the phone gets it
    r.mac
        .lock()
        .unwrap()
        .delete_doc(DOC_SHOPPING, "b", Utc::now() + Duration::seconds(9))
        .unwrap();
    sync_now(&mut r.phone, &mut peer).unwrap();
    assert!(r.phone.docs(DOC_SHOPPING).unwrap().is_empty());
}

#[test]
fn wrong_secret_gets_nothing_and_rotation_locks_old_phones_out() {
    let r = rig();
    let other = Secret::generate().unwrap();
    assert!(matches!(connect(&r.addr(), &other), Err(SyncError::Auth)));
    assert!(connect(&r.addr(), &r.secret).is_ok());
    let new = Secret::generate().unwrap();
    r.mac
        .lock()
        .unwrap()
        .set_setting(SETTING_SYNC_SECRET, &new.to_hex())
        .unwrap();
    assert!(matches!(
        connect(&r.addr(), &r.secret),
        Err(SyncError::Auth)
    ));
    assert!(connect(&r.addr(), &new).is_ok());
}

#[test]
fn code_roundtrip() {
    let s = Secret::generate().unwrap();
    let c = s.to_code();
    assert_eq!(
        Secret::from_code(&c.to_lowercase().replace('-', " ")),
        Some(s)
    );
    assert!(Secret::from_code("nope").is_none());
}

#[test]
fn model_download_resume_and_hash_check() {
    let r = rig();
    let mut peer = r.session();
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("m.gguf");
    // resume from a partial file
    std::fs::write(out.path().join("m.gguf.part"), vec![7u8; 5_000_000]).unwrap();
    let mut first = None;
    let mut last = (0, 0);
    download_model(&mut peer, MODEL, &dest, &mut |d, t| {
        first.get_or_insert(d);
        last = (d, t);
    })
    .unwrap();
    assert_eq!(first, Some(5_000_000));
    assert_eq!(last, (9_000_000, 9_000_000));
    assert_eq!(std::fs::metadata(&dest).unwrap().len(), 9_000_000);
    assert!(!out.path().join("m.gguf.part").exists());

    // a damaged partial file is caught at the end
    let dest2 = out.path().join("bad.gguf");
    let mut bad = vec![7u8; 5_000_000];
    bad[10] = 9;
    std::fs::write(out.path().join("bad.gguf.part"), bad).unwrap();
    let e = download_model(&mut peer, MODEL, &dest2, &mut |_, _| {}).unwrap_err();
    assert!(matches!(e, SyncError::BadHash));
    assert!(!dest2.exists());
}

#[test]
fn model_not_on_the_allow_list_is_refused() {
    let r = rig();
    let mut peer = r.session();
    assert!(matches!(
        peer.model_info("llama3:70b"),
        Err(SyncError::NoModel)
    ));
    assert!(peer.model_info("../../etc/passwd").is_err());
    let mut peer = r.session();
    assert!(peer.model_chunk("llama3:70b", 0, 10).is_err());
    let mut peer = r.session();
    assert_eq!(peer.model_info(MODEL).unwrap().0, 9_000_000);
}

#[test]
fn generate_streams_text_from_the_macs_model() {
    let r = rig();
    let model = MacText::new(&r.addr(), r.secret.clone());
    let mut pieces = vec![];
    let all = model
        .generate("hi", 10, &mut |p| pieces.push(p.to_string()))
        .unwrap();
    assert_eq!(all, "Hello");
    assert_eq!(pieces, vec!["Hel", "lo"]);
    assert_eq!(model.name(), "your Mac (fake)");
    assert!(model.generate("boom", 10, &mut |_| {}).is_err());
}

#[test]
fn replayed_frames_are_not_answered() {
    use std::io::{Read, Write};
    let r = rig();
    let mut s = std::net::TcpStream::connect(r.addr()).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    let cn = [1u8; 24];
    s.write_all(b"NKSYNC1").unwrap();
    s.write_all(&cn).unwrap();
    let mut sn = [0u8; 24];
    s.read_exact(&mut sn).unwrap();
    let (c2s, s2c) = nookery_sync::__test::keys(&r.secret, &cn, &sn);
    let frame = |s: &mut std::net::TcpStream, bytes: &[u8]| {
        s.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
        s.write_all(bytes).unwrap();
    };
    frame(&mut s, &nookery_sync::__test::seal(&c2s, 0, b"hello"));
    let mut len = [0u8; 4];
    s.read_exact(&mut len).unwrap(); // "ok"
    let mut ok = vec![0u8; u32::from_le_bytes(len) as usize];
    s.read_exact(&mut ok).unwrap();
    let _ = s2c;
    let mut req = vec![0u8];
    req.extend(
        serde_json::to_vec(&Request::Hello {
            device: "x".into(),
            version: PROTOCOL,
        })
        .unwrap(),
    );
    let good = nookery_sync::__test::seal(&c2s, 1, &req);
    frame(&mut s, &good);
    s.read_exact(&mut len).unwrap(); // a Welcome comes back
    let mut w = vec![0u8; u32::from_le_bytes(len) as usize];
    s.read_exact(&mut w).unwrap();
    // replay the same frame: the server ends the connection, no answer
    frame(&mut s, &good);
    assert_eq!(s.read(&mut len).unwrap_or(0), 0);
}

fn tiny_jpeg() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(120, 240, image::Rgb([200, 180, 120]));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
    out.into_inner()
}

#[test]
fn scan_photo_runs_the_macs_scan_and_reports_progress() {
    let r = rig();
    let mut peer = connect(&r.addr(), &r.secret).unwrap();
    peer.hello("phone").unwrap();
    let mut ticks = vec![];
    let a = peer
        .scan_photo(Location::Fridge, &tiny_jpeg(), &mut |d, t| {
            ticks.push((d, t))
        })
        .unwrap();
    assert_eq!(a.answered_by, "fake eyes (this Mac)");
    assert!(!a.used_cloud);
    assert_eq!(a.scan.sightings.len(), 1);
    assert_eq!(a.scan.sightings[0].name, "milk");
    assert!(!ticks.is_empty());
    assert_eq!(ticks.last().unwrap().0, ticks.last().unwrap().1);
    // The same connection still works afterwards.
    assert!(peer.model_info("qwen3:4b-instruct").is_ok());
    // Garbage is refused with a message, not a crash.
    assert!(peer
        .scan_photo(Location::Fridge, b"not a jpeg", &mut |_, _| {})
        .is_err());
}
