//! Cutting a photo into overlapping, zoomed tiles.

use crate::{Rect, Tile, TileOptions, VisionError};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use std::io::Cursor;

fn img_err(e: impl std::fmt::Display) -> VisionError {
    VisionError::Image(e.to_string())
}

/// Decodes the bytes and applies the EXIF orientation (phones store
/// portrait photos rotated).
pub(crate) fn decode(bytes: &[u8]) -> Result<DynamicImage, VisionError> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(img_err)?;
    let mut decoder = reader.into_decoder().map_err(img_err)?;
    let orientation = decoder.orientation().map_err(img_err)?;
    let mut img = DynamicImage::from_decoder(decoder).map_err(img_err)?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// `n` spans along a length of `total` pixels that overlap by `overlap_pct`
/// percent of a span. Each is at least 1 px and inside `0..total`.
fn spans(total: u32, n: u32, overlap_pct: u32) -> Vec<(u32, u32)> {
    let n = n.max(1);
    let keep = 1.0 - f64::from(overlap_pct.min(90)) / 100.0;
    let len = f64::from(total) / (1.0 + f64::from(n - 1) * keep);
    let len = (len.round() as u32).clamp(1, total.max(1));
    let step = len as f64 * keep;
    (0..n)
        .map(|i| {
            let start = ((f64::from(i) * step).round() as u32).min(total.saturating_sub(len));
            (start, len)
        })
        .collect()
}

fn zoomed_size(w: u32, h: u32, opts: &TileOptions) -> (u32, u32) {
    let mut scale = f64::from(opts.zoom_pct) / 100.0;
    let longer = f64::from(w.max(h));
    if longer * scale > f64::from(opts.max_side) {
        scale = f64::from(opts.max_side) / longer;
    }
    let s = |v: u32| ((f64::from(v) * scale).round() as u32).max(1);
    (s(w), s(h))
}

pub(crate) fn cut(image: &[u8], opts: &TileOptions) -> Result<Vec<Tile>, VisionError> {
    let img = decode(image)?.to_rgb8();
    let (iw, ih) = img.dimensions();
    let img = DynamicImage::ImageRgb8(img);
    let mut out = Vec::new();
    for (ry, rh) in spans(ih, opts.rows, opts.overlap_pct) {
        for (cx, cw) in spans(iw, opts.cols, opts.overlap_pct) {
            let rect = Rect {
                x: cx,
                y: ry,
                w: cw,
                h: rh,
            };
            let (zw, zh) = zoomed_size(cw, rh, opts);
            let tile = img
                .crop_imm(cx, ry, cw, rh)
                .resize_exact(zw, zh, FilterType::Lanczos3)
                .to_rgb8();
            let mut jpeg = Vec::new();
            JpegEncoder::new_with_quality(&mut jpeg, 90)
                .encode_image(&tile)
                .map_err(img_err)?;
            out.push(Tile {
                index: out.len(),
                rect,
                jpeg,
            });
        }
    }
    Ok(out)
}
