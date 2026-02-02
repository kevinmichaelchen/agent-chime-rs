pub mod broker;
pub mod pocket;
pub mod provider;
pub mod qwen3;

use crate::audio::{cache::AudioCache, renderer};
use crate::config::Config;
use anyhow::Context;
use serde::Serialize;
use serde_json;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub struct BackendInfo {
    pub name: String,
    pub available: bool,
    pub supports_instruct: bool,
}

#[derive(Debug, Serialize)]
pub struct ModelsInfo {
    pub backends: Vec<BackendInfo>,
    pub cache_dir: Option<PathBuf>,
}

pub fn synthesize(
    text: &str,
    config: &Config,
    backend_override: &Option<String>,
) -> anyhow::Result<Vec<u8>> {
    let backend_name = backend_override
        .clone()
        .or_else(|| config.tts.backend.clone())
        .unwrap_or_else(|| "pocket-tts".to_string());

    let cache_dir = config.default_cache_dir()?;
    let (max_size_bytes, max_entries) = config.cache_limits();
    let cache = AudioCache::new(cache_dir, max_size_bytes, max_entries);
    let config_json = serde_json::to_string(&config.tts).context("serialize tts config")?;
    let cache_key = AudioCache::key(&backend_name, text, &config_json);
    if let Some(bytes) = cache.get(&cache_key) {
        return Ok(bytes);
    }

    let timeout_seconds = config.tts.timeout_seconds;
    let internal = std::env::var("AGENT_CHIME_INTERNAL_TTS").is_ok();

    let audio = if timeout_seconds == 0 || internal {
        synthesize_uncached(text, config, &backend_name)?
    } else {
        synthesize_with_timeout(text, config, &backend_name, timeout_seconds)?
    };

    if let Err(err) = cache.put(&cache_key, &audio) {
        tracing::debug!(error = ?err, "cache write failed");
    }

    Ok(audio)
}

pub fn synthesize_in_process(
    text: &str,
    config: &Config,
    backend_override: &Option<String>,
) -> anyhow::Result<Vec<u8>> {
    let backend_name = backend_override
        .clone()
        .or_else(|| config.tts.backend.clone())
        .unwrap_or_else(|| "pocket-tts".to_string());
    synthesize_uncached(text, config, &backend_name)
}

fn synthesize_uncached(text: &str, config: &Config, backend_name: &str) -> anyhow::Result<Vec<u8>> {
    let backend = provider::select_backend(backend_name)?;
    backend
        .synthesize(text, &config.tts)
        .with_context(|| format!("synthesize with {backend_name}"))
}

fn synthesize_with_timeout(
    text: &str,
    config: &Config,
    backend_name: &str,
    timeout_seconds: u64,
) -> anyhow::Result<Vec<u8>> {
    let exe = std::env::current_exe().context("resolve current executable")?;
    let mut cmd = Command::new(exe);
    cmd.arg("__synthesize")
        .arg("--text")
        .arg(text)
        .arg("--backend")
        .arg(backend_name)
        .env("AGENT_CHIME_INTERNAL_TTS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = cmd.spawn().context("spawn internal tts worker")?;

    let config_bytes = serde_json::to_vec(config).context("serialize config JSON")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&config_bytes)
            .context("write config to tts worker")?;
    }

    let stdout = child.stdout.take().context("capture tts worker stdout")?;
    let reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let mut handle = stdout;
        let _ = handle.read_to_end(&mut buf);
        buf
    });

    let deadline = Duration::from_secs(timeout_seconds);
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().context("poll tts worker")? {
            if !status.success() {
                let _ = reader.join();
                anyhow::bail!("tts worker exited with status {status}");
            }
            break;
        }

        if start.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            anyhow::bail!("tts timed out after {timeout_seconds}s");
        }

        thread::sleep(Duration::from_millis(25));
    }

    match reader.join() {
        Ok(audio) => Ok(audio),
        Err(_) => anyhow::bail!("tts worker panicked while streaming audio"),
    }
}

pub fn play_audio(audio: &[u8], volume: f32) -> anyhow::Result<()> {
    renderer::play_bytes(audio, volume)
}

pub fn synthesize_and_play(
    text: &str,
    config: &Config,
    backend_override: &Option<String>,
) -> anyhow::Result<()> {
    let audio = synthesize(text, config, backend_override)?;
    play_audio(&audio, config.volume)
}

pub fn models_info() -> anyhow::Result<ModelsInfo> {
    let config = Config::load()?;
    let cache_dir = config.default_cache_dir().ok();

    let mut backends = Vec::new();
    backends.push(BackendInfo {
        name: "pocket-tts".to_string(),
        available: true,
        supports_instruct: false,
    });

    backends.push(BackendInfo {
        name: "qwen3-tts".to_string(),
        available: cfg!(feature = "qwen3-tts-backend"),
        supports_instruct: true,
    });

    Ok(ModelsInfo { backends, cache_dir })
}
