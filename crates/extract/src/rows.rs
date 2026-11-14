//! Words -> text rows, tolerant of skew.

use crate::Word;

const MAX_ANGLE_DEG: f32 = 8.0;
const STEP_DEG: f32 = 0.25;

struct W<'a> {
    text: &'a str,
    x0: f32,
    cx: f32,
    cy: f32,
    h: f32,
    weight: f32,
}

pub fn rows(words: &[Word]) -> String {
    group(words, true)
}

/// Like `rows` without skew correction (PDF glyph positions are exact; a
/// monospaced two-column receipt would otherwise let the estimator pair a
/// name with the price one row down).
pub fn rows_exact(words: &[Word]) -> String {
    group(words, false)
}

fn group(words: &[Word], deskew: bool) -> String {
    let ws: Vec<W> = words
        .iter()
        .filter(|w| !w.text.trim().is_empty() && w.y0.is_finite() && w.y1.is_finite())
        .map(|w| W {
            text: w.text.trim(),
            x0: w.x0,
            cx: (w.x0 + w.x1) / 2.0,
            cy: (w.y0 + w.y1) / 2.0,
            h: (w.y1 - w.y0).abs(),
            weight: (w.x1 - w.x0).abs().max(1e-3),
        })
        .collect();
    if ws.is_empty() {
        return String::new();
    }
    let mut hs: Vec<f32> = ws.iter().map(|w| w.h).filter(|h| *h > 0.0).collect();
    hs.sort_by(|a, b| a.total_cmp(b));
    let h_med = hs.get(hs.len() / 2).copied().unwrap_or(1.0);

    let slope = if deskew {
        estimate_slope(&ws, h_med)
    } else {
        0.0
    };
    let ys: Vec<f32> = ws.iter().map(|w| w.cy - slope * w.cx).collect();

    let mut order: Vec<usize> = (0..ws.len()).collect();
    order.sort_by(|&a, &b| ys[a].total_cmp(&ys[b]));

    struct Row {
        members: Vec<usize>,
        sum_y: f32,
        sum_h: f32,
    }
    let mut rows: Vec<Row> = Vec::new();
    for i in order {
        let joined = match rows.last_mut() {
            Some(r) => {
                let n = r.members.len() as f32;
                let mean_y = r.sum_y / n;
                let h = (ws[i].h).min(r.sum_h / n).max(0.6 * h_med);
                if (ys[i] - mean_y).abs() <= 0.5 * h {
                    r.members.push(i);
                    r.sum_y += ys[i];
                    r.sum_h += ws[i].h;
                    true
                } else {
                    false
                }
            }
            None => false,
        };
        if !joined {
            rows.push(Row {
                members: vec![i],
                sum_y: ys[i],
                sum_h: ws[i].h,
            });
        }
    }
    rows.sort_by(|a, b| {
        (a.sum_y / a.members.len() as f32).total_cmp(&(b.sum_y / b.members.len() as f32))
    });
    let mut lines = Vec::with_capacity(rows.len());
    for mut r in rows {
        r.members.sort_by(|&a, &b| ws[a].x0.total_cmp(&ws[b].x0));
        let parts: Vec<&str> = r.members.iter().map(|&i| ws[i].text).collect();
        lines.push(parts.join("   "));
    }
    lines.join("\n")
}

/// Slope (dy/dx) at which the weighted words fall into the sharpest
/// horizontal bands. Prefers 0 when nothing is clearly better.
fn estimate_slope(ws: &[W], h_med: f32) -> f32 {
    if ws.len() < 4 {
        return 0.0;
    }
    let win = h_med / 3.0;
    let score = |slope: f32| -> f32 {
        let mut v: Vec<(f32, f32)> = ws.iter().map(|w| (w.cy - slope * w.cx, w.weight)).collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut prefix = Vec::with_capacity(v.len() + 1);
        prefix.push(0.0f32);
        for (_, w) in &v {
            prefix.push(prefix.last().unwrap() + w);
        }
        let (mut lo, mut hi) = (0usize, 0usize);
        let mut total = 0.0;
        for (i, (y, w)) in v.iter().enumerate() {
            while v[lo].0 < y - win {
                lo += 1;
            }
            while hi < v.len() && v[hi].0 <= y + win {
                hi += 1;
            }
            let _ = i;
            total += w * (prefix[hi] - prefix[lo]);
        }
        total
    };
    let mut best = (0.0f32, score(0.0));
    let n = (MAX_ANGLE_DEG / STEP_DEG) as i32;
    for k in 1..=n {
        for sign in [1.0f32, -1.0] {
            let a = sign * k as f32 * STEP_DEG;
            let s = (a.to_radians()).tan();
            let sc = score(s);
            if sc > best.1 * 1.01 {
                best = (s, sc);
            }
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(text: &str, x0: f32, x1: f32, cy: f32, h: f32) -> Word {
        Word {
            text: text.into(),
            x0,
            x1,
            y0: cy - h / 2.0,
            y1: cy + h / 2.0,
        }
    }

    #[test]
    fn empty() {
        assert_eq!(rows(&[]), "");
        assert_eq!(rows(&[w("  ", 0.0, 1.0, 5.0, 10.0)]), "");
    }

    #[test]
    fn same_row_far_apart_and_ordered() {
        let words = [
            w("2,49 B", 500.0, 560.0, 100.0, 20.0),
            w("KAESE GOUDA", 40.0, 200.0, 101.0, 20.0),
            w("BROT", 40.0, 100.0, 130.0, 20.0),
        ];
        assert_eq!(rows(&words), "KAESE GOUDA   2,49 B\nBROT");
    }

    #[test]
    fn close_rows_not_merged() {
        let words = [
            w("A", 40.0, 60.0, 100.0, 20.0),
            w("B", 40.0, 60.0, 115.0, 20.0),
            w("1,00", 500.0, 540.0, 100.0, 20.0),
            w("2,00", 500.0, 540.0, 115.0, 20.0),
        ];
        assert_eq!(rows(&words), "A   1,00\nB   2,00");
    }

    #[test]
    fn rows_come_out_top_to_bottom() {
        let words = [
            w("third", 0.0, 50.0, 300.0, 20.0),
            w("first", 0.0, 50.0, 100.0, 20.0),
            w("second", 0.0, 50.0, 200.0, 20.0),
        ];
        assert_eq!(rows(&words), "first\nsecond\nthird");
    }

    fn skewed(deg: f32) -> Vec<Word> {
        // 12 rows, pitch 26, name at the left, price at the right.
        let t = deg.to_radians().tan();
        let mut v = Vec::new();
        for r in 0..12 {
            let y = 50.0 + r as f32 * 26.0;
            for (txt, x0, x1) in [
                (format!("ITEM{r}"), 40.0, 160.0),
                (format!("{r},99 B"), 500.0, 580.0),
            ] {
                let cx = (x0 + x1) / 2.0;
                v.push(w(&txt, x0, x1, y + t * cx, 20.0));
            }
        }
        v
    }

    #[test]
    fn skew_of_three_degrees() {
        for deg in [3.0, -3.0, 2.5, 5.0] {
            let out = rows(&skewed(deg));
            let lines: Vec<&str> = out.lines().collect();
            assert_eq!(lines.len(), 12, "deg {deg}: {out}");
            for (r, l) in lines.iter().enumerate() {
                assert_eq!(*l, format!("ITEM{r}   {r},99 B"), "deg {deg}");
            }
        }
    }

    #[test]
    fn no_skew_stays_straight() {
        let out = rows(&skewed(0.0));
        assert_eq!(out.lines().count(), 12);
    }
}
