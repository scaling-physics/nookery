//! Where the model runs, per feature, as the user chose. Local is the
//! default and nothing leaves the Mac; cloud is opt-in.
//!
//! Cloud models (Ollama Cloud) go through the local Ollama app after
//! `ollama signin`, so the code path is the same; only the model differs.
//! That also means cloud doesn't help when Ollama itself isn't running.

use std::fmt;
use std::str::FromStr;

/// The default cloud model: Gemma 4 31B, which does images and text, so
/// one model serves photos and recipes (checked 2026-11-09:
/// `ollama show gemma4:31b-cloud` lists completion + vision).
pub const CLOUD_MODEL: &str = "gemma4:31b-cloud";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AiMode {
    /// Only the model on this Mac.
    #[default]
    Local,
    /// This Mac first; the cloud model when the local one is missing or
    /// fails.
    Fallback,
    /// The cloud model.
    Cloud,
}

impl AiMode {
    pub const ALL: [AiMode; 3] = [AiMode::Local, AiMode::Fallback, AiMode::Cloud];

    pub fn as_str(&self) -> &'static str {
        match self {
            AiMode::Local => "local",
            AiMode::Fallback => "fallback",
            AiMode::Cloud => "cloud",
        }
    }
}

impl fmt::Display for AiMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AiMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "local" => Ok(AiMode::Local),
            "fallback" => Ok(AiMode::Fallback),
            "cloud" => Ok(AiMode::Cloud),
            _ => Err(format!("unknown mode {s:?} (local, fallback, cloud)")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_round_trip_and_default_to_local() {
        for m in AiMode::ALL {
            assert_eq!(m.as_str().parse::<AiMode>(), Ok(m));
        }
        assert_eq!(AiMode::default(), AiMode::Local);
        assert!("sky".parse::<AiMode>().is_err());
    }
}
