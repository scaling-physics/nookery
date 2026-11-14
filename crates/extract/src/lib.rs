//! Receipt files -> receipt text, for `nookery_receipt::parse_text`.
//!
//! - PDF with a text layer (REWE eBon and other digital receipts): the
//!   exact text, rebuilt into rows from glyph positions (`rows`). No OCR.
//! - PNG / JPEG / HEIC photos and scanned PDFs (no text layer): OCR with
//!   Apple's Vision framework, on the Mac only (fast after the first call,
//!   ~0.1-0.2 s a photo, and far better than `ocrs` on receipts: that one
//!   dropped minus signs and split prices from their items). Elsewhere
//!   `OcrUnavailable`; the phone sends the file to the Mac instead.
//!
//! Everything runs on this device. Nothing here talks to the network.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
mod apple;
mod pdf;
mod rows;

use std::time::Instant;

/// What a file is, from its first bytes (never from its name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    Pdf,
    Png,
    Jpeg,
    Heic,
    /// UTF-8 text: passed through unchanged.
    Text,
}

/// How the text was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    /// Plain text file.
    Text,
    /// A PDF's own text layer: exact.
    PdfText,
    /// OCR of a photo, or of a PDF page without a text layer: check it.
    Ocr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extracted {
    /// One receipt line per text line, columns joined with spaces
    /// (`BANANEN   1,53 B`), top to bottom.
    pub text: String,
    pub kind: FileKind,
    pub method: Method,
    pub pages: u32,
    pub ms: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExtractError {
    #[error("not a PDF, PNG, JPEG, HEIC or text file")]
    UnknownFormat,
    #[error("no text found in the file")]
    NoText,
    #[error("reading photos needs the Mac (Apple Vision)")]
    OcrUnavailable,
    #[error("can't read the PDF: {0}")]
    Pdf(String),
    #[error("can't read the image: {0}")]
    Image(String),
    #[error("the file is too large (over {max_mb} MB)")]
    TooLarge { max_mb: u32 },
}

/// Files larger than this are refused before any work.
pub const MAX_BYTES: usize = 20 * 1024 * 1024;

/// A piece of positioned text: a glyph run from a PDF or a recognised
/// observation from OCR. Coordinates are in any unit, origin top-left,
/// y growing downwards; `x0 < x1`, `y0 < y1`.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// Detects the file kind from magic bytes (`%PDF`, PNG signature, JPEG
/// SOI, ISO-BMFF `ftyp` heic/heix/mif1); valid UTF-8 without NULs = Text.
pub fn kind_of(bytes: &[u8]) -> Option<FileKind> {
    if bytes.starts_with(b"%PDF-") {
        return Some(FileKind::Pdf);
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(FileKind::Png);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(FileKind::Jpeg);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if [
            &b"heic"[..],
            b"heix",
            b"hevc",
            b"hevx",
            b"heim",
            b"heis",
            b"mif1",
            b"msf1",
        ]
        .contains(&brand)
        {
            return Some(FileKind::Heic);
        }
    }
    if !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok() {
        return Some(FileKind::Text);
    }
    None
}

/// Reads a receipt file into text. Blocking (OCR can take 0.1-1 s, the
/// first OCR call after start up to ~25 s while macOS loads its model):
/// call it off the UI thread.
pub fn extract(bytes: &[u8]) -> Result<Extracted, ExtractError> {
    let start = Instant::now();
    if bytes.len() > MAX_BYTES {
        return Err(ExtractError::TooLarge {
            max_mb: (MAX_BYTES / (1024 * 1024)) as u32,
        });
    }
    let kind = kind_of(bytes).ok_or(ExtractError::UnknownFormat)?;
    let (text, method, pages) = match kind {
        FileKind::Text => {
            let s = std::str::from_utf8(bytes).map_err(|_| ExtractError::UnknownFormat)?;
            let s = s.strip_prefix('\u{feff}').unwrap_or(s);
            (s.replace("\r\n", "\n").replace('\r', "\n"), Method::Text, 1)
        }
        FileKind::Pdf => match pdf::text_layer(bytes) {
            Ok((text, pages)) if !text.trim().is_empty() => (text, Method::PdfText, pages),
            // No text layer (a scan) or a file the text reader can't parse:
            // render and read it.
            other => match pdf_ocr(bytes) {
                Ok((text, pages)) => (text, Method::Ocr, pages),
                Err(ExtractError::OcrUnavailable) => match other {
                    Err(e) => return Err(ExtractError::Pdf(e)),
                    Ok(_) => return Err(ExtractError::OcrUnavailable),
                },
                Err(e) => return Err(e),
            },
        },
        FileKind::Png | FileKind::Jpeg | FileKind::Heic => (image_ocr(bytes)?, Method::Ocr, 1),
    };
    if text.trim().is_empty() {
        return Err(ExtractError::NoText);
    }
    Ok(Extracted {
        text,
        kind,
        method,
        pages,
        ms: start.elapsed().as_millis() as u64,
    })
}

#[cfg(target_os = "macos")]
fn image_ocr(bytes: &[u8]) -> Result<String, ExtractError> {
    apple::ocr_image(bytes)
}
#[cfg(not(target_os = "macos"))]
fn image_ocr(_bytes: &[u8]) -> Result<String, ExtractError> {
    Err(ExtractError::OcrUnavailable)
}
#[cfg(target_os = "macos")]
fn pdf_ocr(bytes: &[u8]) -> Result<(String, u32), ExtractError> {
    apple::ocr_pdf(bytes)
}
#[cfg(not(target_os = "macos"))]
fn pdf_ocr(_bytes: &[u8]) -> Result<(String, u32), ExtractError> {
    Err(ExtractError::OcrUnavailable)
}

/// Groups positioned words into text rows: words whose vertical centres
/// overlap form one row (tolerant of a few degrees of skew), sorted left
/// to right and joined with 3 spaces; rows top to bottom, one per line.
///
/// The skew is estimated first (the angle, within +-8 degrees, at which
/// the most text mass lines up in horizontal bands), the words are
/// de-skewed by it, then grouped.
pub fn rows(words: &[Word]) -> String {
    rows::rows(words)
}
