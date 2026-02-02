#[cfg(feature = "pocket-tts-backend")]
mod imp {
    use anyhow::{bail, Context};
    use pocket_tts::config::defaults;
    use pocket_tts::weights::download_if_necessary;
    use pocket_tts::{ModelState, TTSModel};
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::config::TtsConfig;

    use crate::tts::provider::TtsBackend;

    static MODEL_CACHE: OnceLock<Mutex<HashMap<String, Arc<TTSModel>>>> = OnceLock::new();

    pub struct PocketTtsBackend;

    impl PocketTtsBackend {
        pub fn new() -> Self {
            Self
        }

        fn model_key(variant: &str, use_metal: bool) -> String {
            format!("{variant}:metal={use_metal}")
        }

        fn load_model(
            variant: &str,
            use_metal: bool,
            allow_downloads: bool,
        ) -> anyhow::Result<Arc<TTSModel>> {
            let cache = MODEL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
            let key = Self::model_key(variant, use_metal);
            if let Some(model) = cache.lock().unwrap().get(&key) {
                return Ok(model.clone());
            }

            if !allow_downloads {
                std::env::set_var("HF_HUB_OFFLINE", "1");
            }

            let device = if use_metal {
                #[cfg(feature = "metal")]
                {
                    candle_core::Device::new_metal(0).context("init metal device")?
                }
                #[cfg(not(feature = "metal"))]
                {
                    bail!("metal device requested but metal feature is not enabled");
                }
            } else {
                candle_core::Device::Cpu
            };

            let model = TTSModel::load_with_params_device(
                variant,
                defaults::TEMPERATURE,
                defaults::LSD_DECODE_STEPS,
                defaults::EOS_THRESHOLD,
                None,
                &device,
            )
            .with_context(|| format!("load PocketTTS variant {variant}"))?;

            let model = Arc::new(model);
            cache.lock().unwrap().insert(key, model.clone());
            Ok(model)
        }

        fn resolve_voice_state(
            model: &TTSModel,
            voice_spec: Option<&str>,
            allow_downloads: bool,
        ) -> anyhow::Result<ModelState> {
            let spec = voice_spec.unwrap_or("alba").trim();
            if spec.is_empty() {
                bail!("voice spec is empty");
            }

            if spec.starts_with("hf://") {
                if !allow_downloads {
                    bail!("hf:// voice spec requires downloads; set tts.allow_downloads=true or use a local file");
                }
                let path = download_if_necessary(spec)?;
                return Self::voice_from_path(model, &path);
            }

            let path = PathBuf::from(spec);
            if path.exists() {
                return Self::voice_from_path(model, &path);
            }

            if !allow_downloads {
                bail!("voice '{spec}' requires download; set tts.allow_downloads=true or provide a local file path");
            }

            let hf_path = format!(
                "hf://kyutai/pocket-tts-without-voice-cloning/embeddings/{spec}.safetensors"
            );
            let path = download_if_necessary(&hf_path)?;
            Self::voice_from_path(model, &path)
        }

        fn voice_from_path(model: &TTSModel, path: &PathBuf) -> anyhow::Result<ModelState> {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match ext.as_str() {
                "safetensors" => model
                    .get_voice_state_from_prompt_file(path)
                    .context("load voice prompt"),
                "wav" | "wave" => model.get_voice_state(path).context("load voice wav"),
                _ => bail!("unsupported voice file extension: {ext}"),
            }
        }
    }

    impl Default for PocketTtsBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TtsBackend for PocketTtsBackend {
        fn name(&self) -> &str {
            "pocket-tts"
        }

        fn synthesize(&self, text: &str, config: &TtsConfig) -> anyhow::Result<Vec<u8>> {
            let variant = config.pocket_tts.variant.as_deref().unwrap_or("b6369a24");
            let use_metal = config.pocket_tts.use_metal.unwrap_or(false);
            let model = Self::load_model(variant, use_metal, config.allow_downloads)?;

            let voice_spec = config
                .voice
                .as_deref()
                .or(config.pocket_tts.voice.as_deref());
            let voice_state =
                Self::resolve_voice_state(&model, voice_spec, config.allow_downloads)?;

            let audio = model.generate(text, &voice_state)?;
            let mut cursor = Cursor::new(Vec::new());
            pocket_tts::audio::write_wav_to_writer(&mut cursor, &audio, model.sample_rate as u32)?;
            Ok(cursor.into_inner())
        }

        fn supports_instruct(&self) -> bool {
            false
        }
    }
}

#[cfg(feature = "pocket-tts-backend")]
pub use imp::PocketTtsBackend;

#[cfg(not(feature = "pocket-tts-backend"))]
pub struct PocketTtsBackend;

#[cfg(not(feature = "pocket-tts-backend"))]
impl PocketTtsBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "pocket-tts-backend"))]
impl Default for PocketTtsBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "pocket-tts-backend"))]
impl super::provider::TtsBackend for PocketTtsBackend {
    fn name(&self) -> &str {
        "pocket-tts"
    }

    fn synthesize(
        &self,
        _text: &str,
        _config: &crate::config::TtsConfig,
    ) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("pocket-tts backend not enabled")
    }

    fn supports_instruct(&self) -> bool {
        false
    }
}
