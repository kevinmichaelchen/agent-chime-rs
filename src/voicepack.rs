use crate::config::{Config, VoicePackRoute};
use crate::events::{Event, EventType};
use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use regex::{Regex, RegexBuilder};
use std::fs;
use std::path::{Path, PathBuf};
use voicepack_spec::Manifest;

pub fn select_audio(event: &Event, config: &Config) -> Result<Option<Vec<u8>>> {
    if !config.voicepack.enabled {
        return Ok(None);
    }

    let manifest_path = match config.voicepack_manifest_path() {
        Some(path) => path,
        None => return Ok(None),
    };

    let pack = VoicePack::load(&manifest_path, &config.voicepack.routes)
        .with_context(|| format!("load voicepack manifest at {}", manifest_path.display()))?;

    Ok(pack.select_audio(event))
}

struct VoicePack {
    root: PathBuf,
    manifest: Manifest,
    routes: Vec<RouteRule>,
}

struct RouteRule {
    events: Vec<EventType>,
    regex: Regex,
    phrases: Vec<String>,
}

impl VoicePack {
    fn load(manifest_path: &Path, routes: &[VoicePackRoute]) -> Result<Self> {
        let raw = fs::read_to_string(manifest_path)
            .with_context(|| format!("read manifest {}", manifest_path.display()))?;
        let manifest: Manifest = serde_json::from_str(&raw)
            .with_context(|| format!("parse manifest {}", manifest_path.display()))?;
        let root = manifest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let mut compiled_routes = Vec::new();
        for route in routes {
            if route.phrases.is_empty() {
                continue;
            }
            let mut builder = RegexBuilder::new(&route.pattern);
            builder.case_insensitive(!route.case_sensitive);
            let regex = builder
                .build()
                .with_context(|| format!("compile voicepack route regex: {}", route.pattern))?;
            compiled_routes.push(RouteRule {
                events: route.events.clone(),
                regex,
                phrases: route.phrases.clone(),
            });
        }

        Ok(Self {
            root,
            manifest,
            routes: compiled_routes,
        })
    }

    fn select_audio(&self, event: &Event) -> Option<Vec<u8>> {
        let mut phrase_keys = Vec::new();
        if let Some(summary) = event.summary.as_deref() {
            for route in &self.routes {
                if !route.events.is_empty() && !route.events.contains(&event.event_type) {
                    continue;
                }
                if route.regex.is_match(summary) {
                    phrase_keys.extend(route.phrases.iter().cloned());
                    break;
                }
            }
        }

        if phrase_keys.is_empty() {
            if let Some(keys) = self.manifest.events.get(event_key(event.event_type)) {
                phrase_keys.extend(keys.iter().cloned());
            }
        }

        if phrase_keys.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let phrase_key = phrase_keys.choose(&mut rng)?;
        let phrase = self.manifest.phrases.get(phrase_key)?;
        let variant = phrase.variants.choose(&mut rng)?;

        let path = self.resolve_audio_path(&variant.file)?;
        fs::read(&path).ok()
    }

    fn resolve_audio_path(&self, file: &str) -> Option<PathBuf> {
        let candidate = if Path::new(file).is_absolute() {
            PathBuf::from(file)
        } else {
            self.root.join(file)
        };

        let root = self.root.canonicalize().ok()?;
        let resolved = candidate.canonicalize().ok()?;
        if !resolved.starts_with(&root) {
            return None;
        }
        Some(resolved)
    }
}

fn event_key(event: EventType) -> &'static str {
    match event {
        EventType::AgentYield => "agent_yield",
        EventType::DecisionRequired => "decision_required",
        EventType::ErrorRetry => "error_retry",
    }
}
