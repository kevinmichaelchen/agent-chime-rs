#[cfg(feature = "qwen3-tts-backend")]
mod imp {
    use anyhow::{Context, bail};
    use qwen3_tts::{AudioBuffer, Language, Qwen3TTS, Speaker};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::path::Path;
    use std::rc::Rc;
    use std::str::FromStr;

    use crate::config::TtsConfig;

    use crate::tts::provider::TtsBackend;

    thread_local! {
        static MODEL_CACHE: RefCell<HashMap<String, Rc<Qwen3TTS>>> = RefCell::new(HashMap::new());
    }

    pub struct Qwen3TtsBackend;

    impl Qwen3TtsBackend {
        pub fn new() -> Self {
            Self
        }

        fn model_key(model: &str, tokenizer: Option<&str>, device: &str) -> String {
            let tok = tokenizer.unwrap_or("default");
            format!("{model}:{tok}:{device}")
        }

        fn load_model(
            model_id: &str,
            tokenizer_id: Option<&str>,
            device_str: &str,
            allow_downloads: bool,
        ) -> anyhow::Result<Rc<Qwen3TTS>> {
            let is_local = Path::new(model_id).exists();
            if !allow_downloads && !is_local {
                bail!("qwen3-tts model must be a local path when downloads are disabled");
            }
            if let Some(tokenizer) = tokenizer_id {
                let token_local = Path::new(tokenizer).exists();
                if !allow_downloads && !token_local {
                    bail!("qwen3-tts tokenizer must be local when downloads are disabled");
                }
            }

            if !allow_downloads {
                std::env::set_var("HF_HUB_OFFLINE", "1");
            }

            let key = Self::model_key(model_id, tokenizer_id, device_str);
            if let Some(model) = MODEL_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
                return Ok(model);
            }

            let device = if device_str.eq_ignore_ascii_case("auto") {
                qwen3_tts::auto_device().context("select device")?
            } else {
                qwen3_tts::parse_device(device_str).context("parse device")?
            };
            if device.is_cpu() {
                tracing::warn!(
                    "qwen3-tts is running on CPU; expect high latency. Consider setting tts.qwen3_tts.device to metal/cuda and enabling the matching build feature."
                );
            }

            let model = if !is_local && allow_downloads {
                #[cfg(feature = "hub")]
                {
                    if tokenizer_id.is_some() {
                        tracing::warn!("tokenizer override ignored when downloading model via hub");
                    }
                    let paths = qwen3_tts::ModelPaths::download(Some(model_id))
                        .with_context(|| format!("download Qwen3-TTS model {model_id}"))?;
                    Qwen3TTS::from_paths(&paths, device)
                        .with_context(|| format!("load Qwen3-TTS model {model_id}"))?
                }
                #[cfg(not(feature = "hub"))]
                {
                    bail!("qwen3-tts hub downloads are disabled; rebuild with --features hub");
                }
            } else {
                Qwen3TTS::from_pretrained_with_tokenizer(model_id, tokenizer_id, device)
                    .with_context(|| format!("load Qwen3-TTS model {model_id}"))?
            };

            let model = Rc::new(model);
            MODEL_CACHE.with(|cache| cache.borrow_mut().insert(key, model.clone()));
            Ok(model)
        }

        fn parse_language(config: &TtsConfig) -> anyhow::Result<Language> {
            let lang = config
                .qwen3_tts
                .language
                .as_deref()
                .unwrap_or("English");
            Language::from_str(lang)
        }

        fn parse_speaker(config: &TtsConfig) -> anyhow::Result<Speaker> {
            let speaker = config
                .voice
                .as_deref()
                .or(config.qwen3_tts.speaker.as_deref())
                .unwrap_or("Ryan");
            Speaker::from_str(speaker)
        }

        fn audio_to_wav_bytes(audio: &AudioBuffer) -> anyhow::Result<Vec<u8>> {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: audio.sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut cursor = Cursor::new(Vec::new());
            let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
            for sample in &audio.samples {
                let clamped = sample.clamp(-1.0, 1.0);
                let scaled = (clamped * 32767.0) as i16;
                writer.write_sample(scaled)?;
            }
            writer.finalize()?;
            Ok(cursor.into_inner())
        }
    }

    impl TtsBackend for Qwen3TtsBackend {
        fn name(&self) -> &str {
            "qwen3-tts"
        }

        fn synthesize(&self, text: &str, config: &TtsConfig) -> anyhow::Result<Vec<u8>> {
            let model_id = config
                .qwen3_tts
                .model
                .as_deref()
                .context("qwen3-tts requires tts.qwen3_tts.model")?;
            let tokenizer_id = config.qwen3_tts.tokenizer.as_deref();
            let device = config
                .qwen3_tts
                .device
                .as_deref()
                .unwrap_or("auto");

            let model = Self::load_model(model_id, tokenizer_id, device, config.allow_downloads)?;
            let language = Self::parse_language(config)?;

            let audio = if let Some(instruct) = config.instruct.as_deref() {
                model
                    .synthesize_voice_design(text, instruct, language, None)
                    .context("synthesize voice design")?
            } else if let Some(ref_audio_path) = config.qwen3_tts.ref_audio.as_deref() {
                let ref_audio = AudioBuffer::load(ref_audio_path)
                    .with_context(|| format!("load ref audio {ref_audio_path}"))?;
                let prompt = model.create_voice_clone_prompt(&ref_audio, config.qwen3_tts.ref_text.as_deref())?;
                model
                    .synthesize_voice_clone(text, &prompt, language, None)
                    .context("synthesize voice clone")?
            } else {
                let speaker = Self::parse_speaker(config)?;
                model
                    .synthesize_with_voice(text, speaker, language, None)
                    .context("synthesize with speaker")?
            };

            Self::audio_to_wav_bytes(&audio)
        }

        fn supports_instruct(&self) -> bool {
            true
        }
    }
}

#[cfg(feature = "qwen3-tts-backend")]
pub use imp::Qwen3TtsBackend;

#[cfg(not(feature = "qwen3-tts-backend"))]
pub struct Qwen3TtsBackend;

#[cfg(not(feature = "qwen3-tts-backend"))]
impl Qwen3TtsBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "qwen3-tts-backend"))]
impl super::provider::TtsBackend for Qwen3TtsBackend {
    fn name(&self) -> &str {
        "qwen3-tts"
    }

    fn synthesize(&self, _text: &str, _config: &crate::config::TtsConfig) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("qwen3-tts backend not enabled")
    }

    fn supports_instruct(&self) -> bool {
        true
    }
}
