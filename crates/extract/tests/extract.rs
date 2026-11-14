use nookery_extract::*;
use std::time::Instant;

fn fx(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// rewe.txt without its `#` comment line.
fn expected_lines() -> Vec<String> {
    let p = format!(
        "{}/../receipt/tests/fixtures/rewe.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn collapsed(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn kinds() {
    assert_eq!(kind_of(&fx("rewe-text.pdf")), Some(FileKind::Pdf));
    assert_eq!(kind_of(&fx("rewe-scan.pdf")), Some(FileKind::Pdf));
    assert_eq!(kind_of(&fx("rewe-clean.png")), Some(FileKind::Png));
    assert_eq!(kind_of(&fx("rewe-photo.jpg")), Some(FileKind::Jpeg));
    assert_eq!(kind_of(b"REWE\n1,00 A"), Some(FileKind::Text));
    assert_eq!(kind_of(b"\x00\x01\x02garbage"), None);
    assert_eq!(kind_of(&[0xff, 0xfe, 0x00, 0x80]), None);
    for brand in [b"heic", b"heix", b"mif1", b"hevc", b"msf1"] {
        let mut v = vec![0, 0, 0, 24];
        v.extend_from_slice(b"ftyp");
        v.extend_from_slice(brand);
        v.extend_from_slice(&[0; 12]);
        assert_eq!(kind_of(&v), Some(FileKind::Heic), "{brand:?}");
    }
    let mut mp4 = vec![0, 0, 0, 24];
    mp4.extend_from_slice(b"ftypisom");
    mp4.extend_from_slice(&[0; 12]);
    assert_eq!(kind_of(&mp4), None);
}

#[test]
fn text_passes_through() {
    let e = extract("\u{feff}A  1,00\r\nB  2,00\r\n".as_bytes()).unwrap();
    assert_eq!(e.text, "A  1,00\nB  2,00\n");
    assert_eq!((e.kind, e.method), (FileKind::Text, Method::Text));
}

#[test]
fn errors() {
    assert_eq!(extract(b"\0\0\0garbage"), Err(ExtractError::UnknownFormat));
    let big = vec![b'a'; MAX_BYTES + 1];
    assert_eq!(extract(&big), Err(ExtractError::TooLarge { max_mb: 20 }));
    assert_eq!(extract(b"  \n "), Err(ExtractError::NoText));
    assert!(matches!(
        extract(b"%PDF-1.4\nnot really"),
        Err(ExtractError::Pdf(_)) | Err(ExtractError::Image(_))
    ));
}

#[test]
fn pdf_text_layer() {
    let t = Instant::now();
    let e = extract(&fx("rewe-text.pdf")).unwrap();
    println!(
        "pdf text layer: {} ms (inner {} ms)",
        t.elapsed().as_millis(),
        e.ms
    );
    assert_eq!(
        (e.kind, e.method, e.pages),
        (FileKind::Pdf, Method::PdfText, 1)
    );
    println!("{}", e.text);
    assert_eq!(collapsed(&e.text), expected_lines());
    let parsed = nookery_receipt::parse_text(&e.text, None).expect("parses");
    assert!(parsed.check_total().is_ok(), "{:?}", parsed.check_total());
    assert_eq!(parsed.total_cents, Some(977));
}

#[cfg(target_os = "macos")]
mod ocr {
    use super::*;

    fn check_common(e: &Extracted) {
        assert_eq!(e.method, Method::Ocr);
        for needle in ["BANANEN", "SUMME", "9,77"] {
            assert!(e.text.contains(needle), "missing {needle} in\n{}", e.text);
        }
    }

    #[test]
    fn clean_png() {
        let t = Instant::now();
        let e = extract(&fx("rewe-clean.png")).unwrap();
        println!("OCR first call (png): {} ms", t.elapsed().as_millis());
        check_common(&e);
        println!("{}", e.text);
        let parsed = nookery_receipt::parse_text(&e.text, None).expect("parses");
        assert!(
            parsed.check_total().is_ok(),
            "{:?}\n{}",
            parsed.check_total(),
            e.text
        );
        let t = Instant::now();
        let e2 = extract(&fx("rewe-clean.png")).unwrap();
        println!("OCR warm (png): {} ms", t.elapsed().as_millis());
        assert_eq!(e.text, e2.text);
    }

    #[test]
    fn photo_jpg() {
        let t = Instant::now();
        let e = extract(&fx("rewe-photo.jpg")).unwrap();
        println!("OCR photo: {} ms", t.elapsed().as_millis());
        check_common(&e);
        assert_eq!(e.kind, FileKind::Jpeg);
        println!("----- photo text -----\n{}\n----- end -----", e.text);
        // the skew must not split name from price
        assert!(e
            .text
            .lines()
            .any(|l| l.starts_with("KAESE GOUDA") && l.contains("2,49")));
    }

    #[test]
    fn scanned_pdf() {
        let t = Instant::now();
        let e = extract(&fx("rewe-scan.pdf")).unwrap();
        println!("OCR scanned pdf: {} ms", t.elapsed().as_millis());
        check_common(&e);
        assert_eq!((e.kind, e.pages), (FileKind::Pdf, 1));
    }

    #[test]
    fn blank_png_has_no_text() {
        // 8x8 white PNG
        let png = fx("blank.png");
        assert_eq!(extract(&png), Err(ExtractError::NoText));
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn photos_need_the_mac() {
    assert_eq!(
        extract(&fx("rewe-clean.png")),
        Err(ExtractError::OcrUnavailable)
    );
    assert_eq!(
        extract(&fx("rewe-scan.pdf")),
        Err(ExtractError::OcrUnavailable)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn heic_photo() {
    // Convert the photo with the system's sips (skip if unavailable).
    let dir = std::env::temp_dir().join(format!("nookery-extract-heic-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = format!(
        "{}/tests/fixtures/rewe-photo.jpg",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = dir.join("p.heic");
    let ok = std::process::Command::new("sips")
        .args(["-s", "format", "heic", &src, "--out"])
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("sips can't write HEIC here: skipped");
        return;
    }
    let bytes = std::fs::read(&out).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(kind_of(&bytes), Some(FileKind::Heic));
    let e = extract(&bytes).unwrap();
    assert_eq!(e.method, Method::Ocr);
    assert!(
        e.text.contains("SUMME") && e.text.contains("9,77"),
        "{}",
        e.text
    );
}
