use crate::config::TtsConfig;
use anyhow::bail;

use super::{pocket::PocketTtsBackend, qwen3::Qwen3TtsBackend};

pub trait TtsBackend: Send + Sync {
    fn name(&self) -> &str;
    fn synthesize(&self, text: &str, config: &TtsConfig) -> anyhow::Result<Vec<u8>>;
    fn supports_instruct(&self) -> bool;
}

pub fn select_backend(name: &str) -> anyhow::Result<Box<dyn TtsBackend>> {
    match name {
        "pocket-tts" => {
            #[cfg(feature = "pocket-tts-backend")]
            {
                Ok(Box::new(PocketTtsBackend::new()))
            }
            #[cfg(not(feature = "pocket-tts-backend"))]
            {
                bail!("pocket-tts backend not enabled; rebuild with --features pocket-tts-backend")
            }
        }
        "qwen3-tts" => {
            #[cfg(feature = "qwen3-tts-backend")]
            {
                Ok(Box::new(Qwen3TtsBackend::new()))
            }
            #[cfg(not(feature = "qwen3-tts-backend"))]
            {
                bail!("qwen3-tts backend not enabled; rebuild with --features qwen3-tts-backend")
            }
        }
        _ => bail!("unknown backend: {name}"),
    }
}
