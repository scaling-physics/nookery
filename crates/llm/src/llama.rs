//! `LlamaText`: a GGUF model run in this process.

use crate::{LlamaOptions, LlmError, Utf8Stream, TEMPERATURE};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use nookery_recipes::{RecipeError, TextModel};
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Timings of the last `generate` call.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GenStats {
    pub prompt_tokens: u32,
    pub generated_tokens: u32,
    pub prompt_ms: f64,
    pub generate_ms: f64,
}

impl GenStats {
    pub fn prompt_tps(&self) -> f64 {
        self.prompt_tokens as f64 / (self.prompt_ms / 1e3).max(1e-9)
    }
    pub fn generate_tps(&self) -> f64 {
        self.generated_tokens as f64 / (self.generate_ms / 1e3).max(1e-9)
    }
}

struct Inner {
    backend: LlamaBackend,
    model: LlamaModel,
    opts: LlamaOptions,
    label: String,
    stats: Mutex<GenStats>,
}

/// A text model held in memory. Cheap to clone (shared); `generate` makes
/// a fresh context each time, so calls don't see each other. Loading takes
/// a second or two (the file is memory-mapped), so do it off the UI thread.
#[derive(Clone)]
pub struct LlamaText {
    inner: Arc<Inner>,
}

impl LlamaText {
    pub fn load(path: &Path, opts: LlamaOptions) -> Result<Self, LlmError> {
        if !path.exists() {
            return Err(LlmError::Missing(path.to_path_buf()));
        }
        let mut backend = LlamaBackend::init().map_err(|e| LlmError::Backend(e.to_string()))?;
        backend.void_logs();
        let params = LlamaModelParams::default().with_n_gpu_layers(opts.gpu_layers);
        let model = LlamaModel::load_from_file(&backend, path, &params)
            .map_err(|e| LlmError::Load(e.to_string()))?;
        let label = opts.label.clone().unwrap_or_else(|| {
            let n = model.meta_val_str("general.basename").unwrap_or_default();
            let s = model.meta_val_str("general.size_label").unwrap_or_default();
            let t = format!("{n} {s}").trim().to_lowercase();
            if t.is_empty() {
                "model".into()
            } else {
                t
            }
        });
        Ok(Self {
            inner: Arc::new(Inner {
                backend,
                model,
                opts,
                label,
                stats: Mutex::new(GenStats::default()),
            }),
        })
    }

    /// Timings of the most recent `generate`.
    pub fn last_stats(&self) -> GenStats {
        *self.inner.stats.lock().unwrap()
    }

    /// Size of the model file's tensors in bytes (roughly the RAM it needs).
    pub fn model_bytes(&self) -> u64 {
        self.inner.model.size()
    }
}

fn err(what: &str, e: impl std::fmt::Display) -> RecipeError {
    RecipeError::Model(format!("{what}: {e}"))
}

impl TextModel for LlamaText {
    fn name(&self) -> String {
        format!("{} (on this device)", self.inner.label)
    }

    fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<String, RecipeError> {
        let Inner {
            backend,
            model,
            opts,
            stats,
            ..
        } = &*self.inner;
        // The model's own chat template (Qwen3 instruct: no thinking block).
        let msg =
            LlamaChatMessage::new("user".into(), prompt.into()).map_err(|e| err("prompt", e))?;
        let text = match model.chat_template(None) {
            Ok(t) => model
                .apply_chat_template(&t, &[msg], true)
                .map_err(|e| err("chat template", e))?,
            // No template in the file: ChatML, which Qwen uses.
            Err(_) => format!("<|im_start|>user\n{prompt}<|im_end|>\n<|im_start|>assistant\n"),
        };
        let vocab = model.vocab();
        // The template already contains the special tokens as text.
        let tokens = vocab.tokenize(text.as_bytes(), false, true);
        if tokens.is_empty() {
            return Err(err("prompt", "no tokens"));
        }
        let n_ctx = opts.n_ctx;
        if tokens.len() as u32 + 1 >= n_ctx {
            return Err(err(
                "prompt",
                format!("{} tokens don't fit in a {n_ctx} window", tokens.len()),
            ));
        }
        let n_batch = 512u32;
        let cparams = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(n_batch)
            .with_n_threads(opts.threads)
            .with_n_threads_batch(opts.threads);
        let mut ctx = model
            .new_context(backend, cparams)
            .map_err(|e| err("context", e))?;

        let t0 = Instant::now();
        let mut batch = LlamaBatch::new(n_batch as usize, 1);
        let last = tokens.len() - 1;
        for (c, chunk) in tokens.chunks(n_batch as usize).enumerate() {
            batch.clear();
            for (i, t) in chunk.iter().enumerate() {
                let pos = c * n_batch as usize + i;
                batch
                    .add(*t, pos as i32, &[0], pos == last)
                    .map_err(|e| err("batch", e))?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| err("decoding prompt", e))?;
        }
        ctx.synchronize(); // GPU work is async: time the prompt honestly
        let prompt_ms = t0.elapsed().as_secs_f64() * 1e3;

        // Qwen's recommended sampling for instruct models (temperature from
        // the recipes crate; top-k 20, top-p 0.8) plus a light repeat penalty.
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(model.vocab().n_tokens(), 64, 1.05, 0.0, 0.0),
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.8, 1),
            LlamaSampler::temp(TEMPERATURE),
            LlamaSampler::dist(1234),
        ]);
        sampler.accept_many(&tokens);

        let t1 = Instant::now();
        let mut out = String::new();
        let mut utf8 = Utf8Stream::default();
        let mut pos = tokens.len() as i32;
        let mut n_gen = 0u32;
        let mut sample_at = batch.n_tokens() - 1;
        while n_gen < max_tokens && (pos as u32) < n_ctx {
            let tok = sampler.sample(&ctx, sample_at);
            sampler.accept(tok);
            if vocab.is_eog(tok) {
                break;
            }
            let bytes = vocab.token_to_piece(tok, false, None);
            let piece = utf8.push(&bytes);
            if !piece.is_empty() {
                on_text(&piece);
                out.push_str(&piece);
            }
            n_gen += 1;
            batch.clear();
            batch
                .add(tok, pos, &[0], true)
                .map_err(|e| err("batch", e))?;
            pos += 1;
            sample_at = 0;
            ctx.decode(&mut batch).map_err(|e| err("decoding", e))?;
        }
        let rest = utf8.finish();
        if !rest.is_empty() {
            on_text(&rest);
            out.push_str(&rest);
        }
        *stats.lock().unwrap() = GenStats {
            prompt_tokens: tokens.len() as u32,
            generated_tokens: n_gen,
            prompt_ms,
            generate_ms: t1.elapsed().as_secs_f64() * 1e3,
        };
        Ok(out)
    }
}
