//! Talking to Ollama, and the repetition rules for its streamed answer.

use crate::parse::{norm_name, parse_line};
use crate::{ModelAnswer, Ollama, RawItem, VisionError};
use base64::Engine;
use nookery_core::Location;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::time::Duration;

/// The model starts looping after a while. A line whose name was already
/// listed in this tile is skipped (a single repeat is harmless); after this
/// many skipped lines in a row the answer is cut off.
const MAX_CONSECUTIVE_REPEATS: u32 = 3;
/// A run of this many lines that all name just a container ("jar with ...",
/// "plastic bag with ...") means the model gave up identifying things and is
/// listing every package it sees: cut it off. (4 was too strict: a fridge
/// shelf really is a row of boxes, measured 2026-11-09.)
const MAX_CONTAINER_RUN: u32 = 6;
/// A tile holds a shelf section: real answers had 2-10 items. Past this the
/// model is reciting a generic list (a pantry spice shelf produced 47 spices
/// and sauces, no repeats, mostly not there): cut it off.
const MAX_ITEMS_PER_TILE: usize = 12;

const CONTAINER_WORDS: [&str; 7] = [
    "jar",
    "plastic bag",
    "bag",
    "container",
    "bottle",
    "box",
    "package",
];
const ADJECTIVES: [&str; 8] = [
    "clear", "small", "large", "glass", "plastic", "white", "open", "closed",
];

fn is_container_only(name: &str) -> bool {
    let mut n = norm_name(name);
    while let Some(rest) = ADJECTIVES
        .iter()
        .find_map(|a| n.strip_prefix(a).and_then(|r| r.strip_prefix(' ')))
    {
        n = rest.to_string();
    }
    CONTAINER_WORDS
        .iter()
        .any(|w| n.strip_prefix(w).is_some_and(|r| r.starts_with(" with")))
}

/// Builds items from streamed text chunks and applies the repetition rules.
/// Pure (no I/O) so it can be tested with plain strings.
#[derive(Default)]
pub(crate) struct Assembler {
    pending: String,
    items: Vec<RawItem>,
    seen: HashSet<String>,
    repeats: u32,
    containers: u32,
    cut: bool,
}

impl Assembler {
    /// Feeds a chunk. Returns false once the answer should be cut off.
    pub fn push(&mut self, chunk: &str) -> bool {
        self.pending.push_str(chunk);
        while !self.cut {
            let Some(nl) = self.pending.find('\n') else {
                break;
            };
            let line: String = self.pending.drain(..=nl).collect();
            self.line(&line);
        }
        !self.cut
    }

    fn line(&mut self, line: &str) {
        let Some(item) = parse_line(line) else {
            return;
        };
        if !self.seen.insert(norm_name(&item.name)) {
            self.repeats += 1;
            self.cut = self.repeats >= MAX_CONSECUTIVE_REPEATS;
            return;
        }
        self.repeats = 0;
        self.containers = if is_container_only(&item.name) {
            self.containers + 1
        } else {
            0
        };
        self.items.push(item);
        self.cut = self.containers >= MAX_CONTAINER_RUN || self.items.len() >= MAX_ITEMS_PER_TILE;
    }

    /// Handles the unterminated last line; returns items and whether cut.
    pub fn finish(mut self) -> (Vec<RawItem>, bool) {
        if !self.cut {
            let rest = std::mem::take(&mut self.pending);
            self.line(&rest);
        }
        (self.items, self.cut)
    }
}

fn prompt(place: Location) -> String {
    let place = match place {
        Location::Fridge => "fridge",
        Location::Freezer => "freezer",
        Location::Pantry => "pantry cupboard",
    };
    // No concrete food examples here: the model copies them into its answer.
    format!(
        "This picture is one part of a photo of a {place}.\n\
List each food or drink item you can see in it, one per line, in this format:\n\
name | printed text | count\n\
name: the food, in plain English, as specific as you can (the kind of vegetable, cheese, sauce, leftover dish...). For food in a jar, box or bag, name the food inside, not the container. If you are not sure, give your best guess and end the name with ?\n\
printed text: brand or words readable on the package, else -\n\
count: how many.\n\
Each item once. Do not list the shelves, drawers, labels printed on the {place} itself, paper, or empty bags. Do not guess items you cannot see. No other text."
    )
}

fn model_error(model: &str, msg: &str) -> VisionError {
    let lower = msg.to_lowercase();
    if lower.contains("unauthorized") {
        VisionError::SignIn
    } else if lower.contains("not found") {
        VisionError::ModelMissing(model.to_string())
    } else {
        VisionError::Model(msg.trim().to_string())
    }
}

fn transport_error(url: &str, e: ureq::Error) -> VisionError {
    match e {
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            VisionError::Unreachable(url.to_string())
        }
        ureq::Error::Io(ref io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
            VisionError::Unreachable(url.to_string())
        }
        other => VisionError::Model(other.to_string()),
    }
}

impl Ollama {
    pub(crate) fn generate(
        &self,
        jpeg: &[u8],
        place: Location,
    ) -> Result<ModelAnswer, VisionError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(2)))
            .timeout_per_read(Some(Duration::from_secs(120)))
            .http_status_as_error(false)
            .build()
            .into();
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt(place),
            "images": [base64::engine::general_purpose::STANDARD.encode(jpeg)],
            "stream": true,
            "think": false,
            "options": {"temperature": 0, "num_ctx": 8192, "num_predict": self.max_tokens},
        });
        let url = self.url.trim_end_matches('/');
        let mut resp = agent
            .post(&format!("{url}/api/generate"))
            .send_json(&body)
            .map_err(|e| transport_error(&self.url, e))?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.body_mut().read_to_string().unwrap_or_default();
            // A 404 whose body says "not found" is a missing model.
            return Err(model_error(&self.model, &format!("HTTP {status}: {text}")));
        }

        let mut lines = BufReader::new(resp.into_body().into_reader());
        let mut asm = Assembler::default();
        let (mut chunks, mut eval_count) = (0u32, None);
        let mut line = String::new();
        loop {
            line.clear();
            let n = lines
                .read_line(&mut line)
                .map_err(|e| VisionError::Model(format!("reading the answer: {e}")))?;
            if n == 0 {
                break;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if let Some(err) = v["error"].as_str() {
                return Err(model_error(&self.model, err));
            }
            chunks += 1;
            if let Some(n) = v["eval_count"].as_u64() {
                eval_count = Some(n as u32);
            }
            if !asm.push(v["response"].as_str().unwrap_or("")) {
                break; // dropping the reader closes the connection: Ollama stops
            }
            if v["done"].as_bool() == Some(true) {
                break;
            }
        }
        let (items, cut_on_repeat) = asm.finish();
        Ok(ModelAnswer {
            items,
            tokens: eval_count.unwrap_or(chunks),
            cut_on_repeat,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chunks: &[&str]) -> (Vec<RawItem>, bool) {
        let mut a = Assembler::default();
        for c in chunks {
            if !a.push(c) {
                break;
            }
        }
        a.finish()
    }

    #[test]
    fn lines_split_across_chunks_are_reassembled() {
        let (items, cut) = run(&["milk | Ar", "la | 2\negg", "s | - | 6"]);
        assert!(!cut);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text.as_deref(), Some("Arla"));
        assert_eq!(items[1].count, 6);
    }

    #[test]
    fn a_single_repeat_is_skipped_without_cutting() {
        let (items, cut) = run(&["milk | - | 1\nMilk | - | 1\neggs | - | 1\n"]);
        assert!(!cut);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn three_repeats_in_a_row_cut_the_answer() {
        let (items, cut) =
            run(&["a | - | 1\nb | - | 1\na | - | 1\nb | - | 1\na | - | 1\nc | - | 1\n"]);
        assert!(cut);
        assert_eq!(items.len(), 2, "c is never read");
    }

    #[test]
    fn a_new_item_resets_the_repeat_count() {
        let (items, cut) =
            run(&["a | - | 1\na | - | 1\na | - | 1\nb | - | 1\na | - | 1\na | - | 1\n"]);
        assert!(!cut);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn a_run_of_container_only_names_cuts_the_answer() {
        let text = "milk | - | 1\n\
            jar with red paste | - | 1\n\
            Clear jar with green contents | - | 1\n\
            plastic bag with leaves | - | 1\n\
            clear container with sauce | - | 1\n\
            jar with something | - | 1\n\
            box with eggs | - | 1\n\
            bottle with oil | - | 1\n";
        let (items, cut) = run(&[text]);
        assert!(cut);
        assert_eq!(items.len(), 7);
    }

    #[test]
    fn a_runaway_list_is_cut_after_twelve_items() {
        let text: String = (0..40).map(|i| format!("spice {i} | - | 1\n")).collect();
        let (items, cut) = run(&[&text]);
        assert!(cut);
        assert_eq!(items.len(), MAX_ITEMS_PER_TILE);
    }

    #[test]
    fn container_names_broken_up_by_real_items_do_not_cut() {
        let text = "jar with a | - | 1\njar with b | - | 1\nbutter | - | 1\njar with c | - | 1\njar with d | - | 1\n";
        let (items, cut) = run(&[text]);
        assert!(!cut);
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn a_named_jar_is_not_container_only() {
        assert!(!is_container_only("jar of pickles"));
        assert!(!is_container_only("pickle jar"));
        assert!(is_container_only("small glass jar with lid"));
    }

    #[test]
    fn missing_models_are_told_apart_from_other_failures() {
        assert!(matches!(
            model_error("m", "HTTP 404: {\"error\":\"model 'm' not found\"}"),
            VisionError::ModelMissing(_)
        ));
        assert!(matches!(
            model_error("m", "HTTP 500: oops"),
            VisionError::Model(_)
        ));
    }

    #[test]
    fn a_closed_port_is_unreachable() {
        let o = Ollama {
            url: "http://127.0.0.1:1".into(),
            model: "m".into(),
            max_tokens: 10,
        };
        assert!(matches!(
            o.generate(b"x", Location::Fridge),
            Err(VisionError::Unreachable(_))
        ));
    }
}
