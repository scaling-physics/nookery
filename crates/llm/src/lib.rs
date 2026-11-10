//! An in-process text model for the phone (llama.cpp via `llama-cpp-2`).
//! See AGENTS.md. The model code is behind feature `llama` (it builds
//! llama.cpp from C++); the helpers here need nothing.

use std::path::{Path, PathBuf};

#[cfg(feature = "llama")]
mod llama;
#[cfg(feature = "llama")]
pub use llama::{GenStats, LlamaText};

/// Sampling temperature: the same as the recipes crate (`TEMPERATURE`).
pub const TEMPERATURE: f32 = 0.7;
/// Context window in tokens (prompt + answer).
pub const N_CTX: u32 = 4096;

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("the model file {0} doesn't exist")]
    Missing(PathBuf),
    #[error("couldn't start the language model runtime: {0}")]
    Backend(String),
    #[error("couldn't load the model: {0}")]
    Load(String),
}

/// How to load and run the model.
#[derive(Clone, Debug, PartialEq)]
pub struct LlamaOptions {
    /// Threads for decoding and prompt processing. Default
    /// `min(available_parallelism, 8)`.
    pub threads: i32,
    /// Context window. Default [`N_CTX`].
    pub n_ctx: u32,
    /// Layers to offload to the GPU (Metal on macOS). `0` = CPU only.
    /// Default: all on macOS, none elsewhere.
    pub gpu_layers: u32,
    /// Label for `name()`, e.g. "qwen3 4b". Default: from the GGUF file.
    pub label: Option<String>,
}

impl Default for LlamaOptions {
    fn default() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            threads: cores.min(8) as i32,
            n_ctx: N_CTX,
            gpu_layers: if cfg!(target_os = "macos") { 999 } else { 0 },
            label: None,
        }
    }
}

/// Finds the GGUF blob of an Ollama model (`"qwen3:4b-instruct"`) under
/// `models_dir` (usually `~/.ollama/models`). For tests and development
/// only: the phone gets its file from the Mac.
pub fn ollama_blob(models_dir: &Path, model: &str) -> Option<PathBuf> {
    let (name, tag) = model.split_once(':').unwrap_or((model, "latest"));
    let manifest = models_dir
        .join("manifests/registry.ollama.ai/library")
        .join(name)
        .join(tag);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest).ok()?).ok()?;
    let digest = json
        .get("layers")?
        .as_array()?
        .iter()
        .find(|l| {
            l.get("mediaType").and_then(|m| m.as_str())
                == Some("application/vnd.ollama.image.model")
        })?
        .get("digest")?
        .as_str()?;
    let path = models_dir.join("blobs").join(digest.replacen(':', "-", 1));
    path.exists().then_some(path)
}

/// Turns streamed token bytes into text, holding back an incomplete UTF-8
/// character until its remaining bytes arrive.
#[derive(Default, Debug)]
pub struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    /// Adds bytes; returns the text that is complete now (maybe empty).
    pub fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        match std::str::from_utf8(&self.pending) {
            Ok(s) => {
                let out = s.to_string();
                self.pending.clear();
                out
            }
            Err(e) => {
                let good = e.valid_up_to();
                let mut out = String::from_utf8_lossy(&self.pending[..good]).into_owned();
                match e.error_len() {
                    // A truly invalid byte sequence: replace it, keep going.
                    Some(n) => {
                        out.push('\u{FFFD}');
                        let rest = self.pending.split_off(good + n);
                        self.pending = rest;
                        out.push_str(&self.push(&[]));
                    }
                    // Just cut off at the end: wait for more bytes.
                    None => {
                        self.pending.drain(..good);
                    }
                }
                out
            }
        }
    }

    /// Whatever is left at the end (an unfinished character becomes U+FFFD).
    pub fn finish(&mut self) -> String {
        let s = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        s
    }
}

/// Whether `[start, end)` of a Qwen-style answer is a stray end marker we
/// should never show (some templates leak `<|im_end|>` as text).
pub fn strip_end_marker(s: &str) -> &str {
    s.strip_suffix("<|im_end|>").unwrap_or(s).trim_end()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_split_across_tokens() {
        let mut u = Utf8Stream::default();
        let bytes = "Crème brûlée 🍳".as_bytes();
        let mut out = String::new();
        for b in bytes {
            out.push_str(&u.push(&[*b]));
        }
        out.push_str(&u.finish());
        assert_eq!(out, "Crème brûlée 🍳");
    }

    #[test]
    fn utf8_holds_back_partial_char() {
        let mut u = Utf8Stream::default();
        let e = "é".as_bytes();
        assert_eq!(u.push(&[b'a', e[0]]), "a");
        assert_eq!(u.push(&[e[1]]), "é");
        assert_eq!(u.finish(), "");
    }

    #[test]
    fn utf8_invalid_byte_is_replaced() {
        let mut u = Utf8Stream::default();
        assert_eq!(u.push(&[b'a', 0xFF, b'b']), "a\u{FFFD}b");
        u.push(&[0xE2, 0x82]);
        assert_eq!(u.finish(), "\u{FFFD}");
    }

    #[test]
    fn end_marker() {
        assert_eq!(strip_end_marker("hi \n<|im_end|>"), "hi");
        assert_eq!(strip_end_marker("hi"), "hi");
    }

    #[test]
    fn blob_lookup() {
        let d = tempfile::tempdir().unwrap();
        let m = d.path().join("manifests/registry.ollama.ai/library/qwen3");
        std::fs::create_dir_all(&m).unwrap();
        std::fs::create_dir_all(d.path().join("blobs")).unwrap();
        std::fs::write(
            m.join("4b-instruct"),
            r#"{"layers":[{"mediaType":"application/vnd.ollama.image.template","digest":"sha256:aa"},
               {"mediaType":"application/vnd.ollama.image.model","digest":"sha256:bb"}]}"#,
        )
        .unwrap();
        assert_eq!(ollama_blob(d.path(), "qwen3:4b-instruct"), None);
        std::fs::write(d.path().join("blobs/sha256-bb"), b"x").unwrap();
        assert_eq!(
            ollama_blob(d.path(), "qwen3:4b-instruct"),
            Some(d.path().join("blobs/sha256-bb"))
        );
        assert_eq!(ollama_blob(d.path(), "nope:1"), None);
    }
}
