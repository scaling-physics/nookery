//! PDF text layer (pure Rust): positioned glyphs -> runs -> `rows`.

use crate::{rows::rows_exact, Word};
use pdf_extract::{MediaBox, OutputDev, OutputError, Transform};

struct Run {
    text: String,
    x0: f32,
    end: f32,
    base: f32,
    fs: f32,
}

#[derive(Default)]
struct Collector {
    pages: Vec<Vec<Word>>,
    cur: Option<Run>,
    flip_h: f64,
}

impl Collector {
    fn flush(&mut self) {
        if let (Some(r), Some(page)) = (self.cur.take(), self.pages.last_mut()) {
            if !r.text.trim().is_empty() {
                page.push(Word {
                    text: r.text,
                    x0: r.x0,
                    x1: r.end.max(r.x0 + 0.1),
                    y0: r.base - 0.8 * r.fs,
                    y1: r.base + 0.2 * r.fs,
                });
            }
        }
    }
}

impl OutputDev for Collector {
    fn begin_page(
        &mut self,
        _n: u32,
        mb: &MediaBox,
        _art: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        self.flush();
        self.flip_h = mb.ury - mb.lly;
        self.pages.push(Vec::new());
        Ok(())
    }
    fn end_page(&mut self) -> Result<(), OutputError> {
        self.flush();
        Ok(())
    }
    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        ch: &str,
    ) -> Result<(), OutputError> {
        let vx = font_size * (trm.m11 + trm.m21);
        let vy = font_size * (trm.m12 + trm.m22);
        let fs = (vx * vy).abs().sqrt() as f32;
        let x = trm.m31 as f32;
        let y = (self.flip_h - trm.m32) as f32;
        let end = x + (width * fs as f64) as f32;
        if ch.trim().is_empty() {
            // Spaces are recovered from the gap to the next glyph.
            return Ok(());
        }
        if let Some(r) = self.cur.as_mut() {
            let gap = x - r.end;
            if (y - r.base).abs() < 0.3 * r.fs.max(fs) && gap > -0.5 * fs && gap < 1.5 * fs {
                if gap > 0.1 * fs {
                    r.text.push(' ');
                }
                r.text.push_str(ch);
                r.end = end;
                return Ok(());
            }
        }
        self.flush();
        self.cur = Some(Run {
            text: ch.to_string(),
            x0: x,
            end,
            base: y,
            fs,
        });
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

/// Text of every page (rows rebuilt from glyph positions, pages separated
/// by a blank line) and the page count. Errors are messages for
/// `ExtractError::Pdf`.
pub fn text_layer(bytes: &[u8]) -> Result<(String, u32), String> {
    // The parser can panic on odd files: contain it.
    let res = std::panic::catch_unwind(|| -> Result<Vec<Vec<Word>>, String> {
        let mut doc = pdf_extract::Document::load_mem(bytes).map_err(|e| e.to_string())?;
        if doc.is_encrypted() {
            doc.decrypt("").map_err(|e| e.to_string())?;
        }
        let mut c = Collector::default();
        pdf_extract::output_doc(&doc, &mut c).map_err(|e| e.to_string())?;
        c.flush();
        Ok(c.pages)
    });
    let pages = match res {
        Ok(r) => r?,
        Err(_) => return Err("unreadable PDF".into()),
    };
    let n = pages.len() as u32;
    let texts: Vec<String> = pages
        .iter()
        .map(|p| rows_exact(p))
        .filter(|t| !t.trim().is_empty())
        .collect();
    Ok((texts.join("\n\n"), n))
}
