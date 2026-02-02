use crate::config::{Config, Mode};
use crate::events::EventType;
use anyhow::Context;

use super::renderer;

pub fn should_play(event_type: EventType, config: &Config) -> bool {
    config
        .events
        .get(&event_type)
        .map(|cfg| cfg.enabled && cfg.mode == Mode::Earcon)
        .unwrap_or(false)
}

pub fn play_for_event(event_type: EventType, config: &Config) -> anyhow::Result<()> {
    if !should_play(event_type, config) {
        return Ok(());
    }

    let dir = match config.default_earcons_dir() {
        Some(dir) => dir,
        None => {
            tracing::warn!("earcons directory not found; skipping earcon");
            return Ok(());
        }
    };

    let filename = match event_type {
        EventType::AgentYield => "yield.wav",
        EventType::DecisionRequired => "decision.wav",
        EventType::ErrorRetry => "error.wav",
    };

    let path = dir.join(filename);
    if !path.exists() {
        tracing::warn!(path = %path.display(), "earcon file missing; skipping");
        return Ok(());
    }

    renderer::play_file(&path, config.volume).context("play earcon")
}
