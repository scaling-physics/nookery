//! Shrinks a receipt photo before the phone sends it to the Mac.
//!
//! Receipts are long and thin and OCR needs the resolution, so the limit is
//! 3000 px on the long side (ScanPhoto uses 2000). Only PNG / JPEG larger
//! than 4 MB or 3000 px are re-encoded (JPEG, quality 90, EXIF orientation
//! applied); everything else (PDF, HEIC, small images) is sent as it is.

use image::{DynamicImage, ImageDecoder, ImageReader};
use std::io::Cursor;

pub const MAX_LONG_SIDE: u32 = 3000;
pub const SHRINK_ABOVE_BYTES: usize = 4 * 1024 * 1024;
const JPEG_QUALITY: u8 = 90;

fn is_png(b: &[u8]) -> bool {
    b.starts_with(&[0x89, b'P', b'N', b'G'])
}
fn is_jpeg(b: &[u8]) -> bool {
    b.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// The bytes to send. Returns the input when nothing needs shrinking or the
/// image can't be decoded (the Mac then says what is wrong with it).
pub fn shrink_for_ocr(bytes: &[u8]) -> Vec<u8> {
    if !(is_png(bytes) || is_jpeg(bytes)) {
        return bytes.to_vec();
    }
    shrink(bytes).unwrap_or_else(|| bytes.to_vec())
}

fn shrink(bytes: &[u8]) -> Option<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let (w, h) = decoder.dimensions();
    if bytes.len() <= SHRINK_ABOVE_BYTES && w.max(h) <= MAX_LONG_SIDE {
        return None;
    }
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).ok()?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    if img.width().max(img.height()) > MAX_LONG_SIDE {
        img = img.resize(
            MAX_LONG_SIDE,
            MAX_LONG_SIDE,
            image::imageops::FilterType::Triangle,
        );
    }
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .ok()?;
    // Never send more than we were given.
    (out.len() < bytes.len()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img =
            image::RgbImage::from_fn(w, h, |x, y| image::Rgb([x as u8, y as u8, (x ^ y) as u8]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn small_images_and_other_files_pass_through() {
        let small = png(100, 300);
        assert_eq!(shrink_for_ocr(&small), small);
        assert_eq!(shrink_for_ocr(b"%PDF-1.7 ..."), b"%PDF-1.7 ...");
        assert_eq!(shrink_for_ocr(b"garbage"), b"garbage");
    }

    #[test]
    fn long_thin_images_shrink_to_3000_px_jpeg() {
        let big = png(1200, 4000);
        let out = shrink_for_ocr(&big);
        assert!(is_jpeg(&out));
        let img = image::load_from_memory(&out).unwrap();
        assert_eq!(img.height(), MAX_LONG_SIDE);
        assert_eq!(img.width(), 900);
    }
}
