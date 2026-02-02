use crate::events::EventType;
use anyhow::{bail, Context};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tts: TtsConfig,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub events: HashMap<EventType, EventConfig>,
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub cache_max_mb: Option<u64>,
    #[serde(default)]
    pub cache_max_entries: Option<usize>,
    #[serde(default)]
    pub earcons_dir: Option<PathBuf>,
    #[serde(default)]
    pub voicepack: VoicePackConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub instruct: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_allow_downloads")]
    pub allow_downloads: bool,
    #[serde(default)]
    pub pocket_tts: PocketTtsConfig,
    #[serde(default)]
    pub qwen3_tts: Qwen3TtsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PocketTtsConfig {
    pub variant: Option<String>,
    pub voice: Option<String>,
    pub use_metal: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Qwen3TtsConfig {
    pub model: Option<String>,
    pub tokenizer: Option<String>,
    pub speaker: Option<String>,
    pub language: Option<String>,
    pub ref_audio: Option<String>,
    pub ref_text: Option<String>,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoicePackConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub manifest_path: Option<PathBuf>,
    #[serde(default)]
    pub routes: Vec<VoicePackRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePackRoute {
    pub pattern: String,
    pub phrases: Vec<String>,
    #[serde(default)]
    pub events: Vec<EventType>,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Tts,
    Earcon,
    Silent,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        if let Some(path) = Self::project_path() {
            if path.exists() {
                return Self::load_from_path(&path);
            }
        }

        if let Ok(path) = Self::default_path() {
            if path.exists() {
                return Self::load_from_path(&path);
            }
        }

        Ok(Self::default())
    }

    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read config at {}", path.display()))?;
        let mut config: Config = serde_json::from_str(&raw)
            .with_context(|| format!("parse config at {}", path.display()))?;
        config.apply_defaults();
        Ok(config)
    }

    pub fn init_default() -> anyhow::Result<PathBuf> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let config = Self::default();
        fs::write(&path, serde_json::to_string_pretty(&config)?)?;
        Ok(path)
    }

    pub fn default_path() -> anyhow::Result<PathBuf> {
        let base = BaseDirs::new().context("unable to resolve home directory")?;
        Ok(base.config_dir().join("agent-chime").join("config.json"))
    }

    pub fn default_cache_dir(&self) -> anyhow::Result<PathBuf> {
        if let Some(dir) = &self.cache_dir {
            return Ok(dir.clone());
        }
        let base = BaseDirs::new().context("unable to resolve home directory")?;
        Ok(base.cache_dir().join("agent-chime"))
    }

    pub fn cache_limits(&self) -> (u64, usize) {
        let max_mb = self.cache_max_mb.unwrap_or(100);
        let max_entries = self.cache_max_entries.unwrap_or(1000);
        (max_mb * 1024 * 1024, max_entries)
    }

    pub fn default_earcons_dir(&self) -> Option<PathBuf> {
        if let Some(dir) = &self.earcons_dir {
            return Some(dir.clone());
        }

        let local = PathBuf::from("earcons");
        if local.exists() {
            return Some(local);
        }

        None
    }

    pub fn voicepack_manifest_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.voicepack.manifest_path {
            return Some(path.clone());
        }

        let local = PathBuf::from("voicepack").join("manifest.json");
        if local.exists() {
            return Some(local);
        }

        None
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if !(0.0..=1.0).contains(&self.volume) {
            bail!("volume must be between 0.0 and 1.0");
        }

        if let Some(backend) = &self.tts.backend {
            match backend.as_str() {
                "pocket-tts" | "qwen3-tts" => {}
                _ => bail!("unsupported backend: {backend}"),
            }
        }

        if let Some(backend) = &self.tts.backend {
            if backend == "qwen3-tts" && self.tts.qwen3_tts.model.is_none() {
                bail!("qwen3-tts backend requires tts.qwen3_tts.model to be set");
            }
        }

        if let Some(max_mb) = self.cache_max_mb {
            if max_mb == 0 {
                bail!("cache_max_mb must be greater than 0");
            }
        }

        if let Some(max_entries) = self.cache_max_entries {
            if max_entries == 0 {
                bail!("cache_max_entries must be greater than 0");
            }
        }

        if self.voicepack.enabled {
            if let Some(path) = self.voicepack_manifest_path() {
                if !path.exists() {
                    bail!("voicepack manifest not found: {}", path.display());
                }
            } else {
                bail!("voicepack enabled but no manifest_path configured");
            }
        }

        Ok(())
    }

    fn apply_defaults(&mut self) {
        for event_type in [
            EventType::AgentYield,
            EventType::DecisionRequired,
            EventType::ErrorRetry,
        ] {
            match self.events.get_mut(&event_type) {
                Some(event_config) => {
                    if event_config.template.is_none() {
                        event_config.template = Some(event_type.default_template().to_string());
                    }
                }
                None => {
                    self.events
                        .insert(event_type, EventConfig::default_for(event_type));
                }
            }
        }

        if self.tts.pocket_tts.variant.is_none() {
            self.tts.pocket_tts.variant = Some("b6369a24".to_string());
        }

        if self.tts.pocket_tts.voice.is_none() {
            self.tts.pocket_tts.voice = Some("alba".to_string());
        }

        if self.tts.pocket_tts.use_metal.is_none() {
            self.tts.pocket_tts.use_metal = Some(false);
        }

        if self.tts.qwen3_tts.speaker.is_none() {
            self.tts.qwen3_tts.speaker = Some("Ryan".to_string());
        }

        if self.tts.qwen3_tts.language.is_none() {
            self.tts.qwen3_tts.language = Some("English".to_string());
        }

        if self.tts.qwen3_tts.device.is_none() {
            self.tts.qwen3_tts.device = Some("auto".to_string());
        }

        if self.cache_max_mb.is_none() {
            self.cache_max_mb = Some(100);
        }

        if self.cache_max_entries.is_none() {
            self.cache_max_entries = Some(1000);
        }

        if self.voicepack.routes.is_empty() {
            self.voicepack.routes = Vec::new();
        }
    }

    fn project_path() -> Option<PathBuf> {
        Some(PathBuf::from("agent-chime.json"))
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut events = HashMap::new();
        events.insert(
            EventType::AgentYield,
            EventConfig::default_for(EventType::AgentYield),
        );
        events.insert(
            EventType::DecisionRequired,
            EventConfig::default_for(EventType::DecisionRequired),
        );
        events.insert(
            EventType::ErrorRetry,
            EventConfig::default_for(EventType::ErrorRetry),
        );

        Self {
            tts: TtsConfig::default(),
            volume: default_volume(),
            events,
            cache_dir: None,
            cache_max_mb: Some(100),
            cache_max_entries: Some(1000),
            earcons_dir: None,
            voicepack: VoicePackConfig::default(),
        }
    }
}

impl EventConfig {
    pub fn default_for(event_type: EventType) -> Self {
        let mode = match event_type {
            EventType::ErrorRetry => Mode::Earcon,
            _ => Mode::Tts,
        };

        Self {
            enabled: true,
            mode,
            template: Some(event_type.default_template().to_string()),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_allow_downloads() -> bool {
    true
}

fn default_timeout_seconds() -> u64 {
    10
}

fn default_volume() -> f32 {
    0.8
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            backend: Some("pocket-tts".to_string()),
            voice: None,
            instruct: None,
            timeout_seconds: default_timeout_seconds(),
            allow_downloads: default_allow_downloads(),
            pocket_tts: PocketTtsConfig {
                variant: Some("b6369a24".to_string()),
                voice: Some("alba".to_string()),
                use_metal: Some(false),
            },
            qwen3_tts: Qwen3TtsConfig {
                model: None,
                tokenizer: None,
                speaker: Some("Ryan".to_string()),
                language: Some("English".to_string()),
                ref_audio: None,
                ref_text: None,
                device: Some("auto".to_string()),
            },
        }
    }
}
