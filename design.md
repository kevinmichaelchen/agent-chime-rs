# Design

## 1. Goals

- Provide audible cues for `yield` and `decision` events in agentic CLI
  workflows.
- Stay local-first on macOS with low latency.
- Single binary distribution with fast startup (~10ms vs ~500ms for Python).
- API-compatible with Python `agent-chime` for drop-in replacement.

## 2. System Overview

### 2.1 Components

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              agent-chime-rs                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   Adapters  │    │  TTS Broker │    │TTS Provider │    │   Renderer  │  │
│  │             │───▶│             │───▶│             │───▶│             │  │
│  │ - Claude    │    │ - Templates │    │ - PocketTTS │    │ - afplay    │  │
│  │ - Codex     │    │ - Policy    │    │ - Qwen3TTS  │    │ - Cache     │  │
│  │ - OpenCode  │    │ - Routing   │    │ - Fallback  │    │ - Earcons   │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Adapters**: Parse CLI-specific event formats into unified `Event` struct.
- **TTS Broker**: Select template, apply policy, determine TTS vs earcon mode.
- **TTS Provider**: Interface to Rust TTS backends (pocket-tts, qwen3-tts-rs).
- **Renderer**: Play audio via `afplay`, manage cache, handle earcon fallback.

### 2.2 Data Flow

```
┌─────────────────┐         ┌─────────────┐         ┌─────────────┐
│  Agent CLI      │  stdin/ │  Adapter    │  Event  │  TTS Broker │
│  (claude/codex/ │──argv──▶│  (parse)    │────────▶│  (template) │
│  opencode)      │         │             │         │             │
└─────────────────┘         └─────────────┘         └─────────────┘
                                                           │
                                                           ▼
┌─────────────────┐         ┌─────────────┐         ┌─────────────┐
│  Audio Output   │◀────────│  Renderer   │◀────────│TTS Provider │
│  (speakers)     │  audio  │  (afplay)   │  bytes  │  (candle)   │
└─────────────────┘         └─────────────┘         └─────────────┘
```

## 3. Event Model

### 3.1 Event Types

```rust
pub enum EventType {
    AgentYield,       // Agent is done and waiting
    DecisionRequired, // Agent needs explicit user input
    ErrorRetry,       // Recoverable error or interruption
}
```

### 3.2 Event Struct

```rust
pub struct Event {
    pub event_type: EventType,
    pub source: Source,
    pub timestamp: DateTime<Utc>,
    pub summary: Option<String>,
    pub context: Option<serde_json::Value>,
    pub priority: Priority,
}

pub enum Source {
    Claude,
    Codex,
    OpenCode,
}

pub enum Priority {
    Low,
    Normal,
    High,
}
```

### 3.3 Priority Rules

- `DecisionRequired`: Always `High` priority
- `AgentYield`: `Normal` priority, can be coalesced if multiple events occur
- `ErrorRetry`: `High` priority, preempts lower priority playback

## 4. CLI Integration

Each CLI uses its native hook/event system. The adapter parses tool-specific
formats into the unified `Event` struct.

### 4.1 Claude Code

**Input**: JSON via stdin

```json
{
  "hook_event_name": "Stop",
  "session_id": "abc123",
  "reason": "Task appears complete"
}
```

**Event Mapping**:

| Hook Event                     | Maps To            |
| ------------------------------ | ------------------ |
| `Stop`                         | `AgentYield`       |
| `Notification`                 | `AgentYield`       |
| `PreToolUse` (AskUserQuestion) | `DecisionRequired` |

### 4.2 Codex

**Input**: JSON as CLI argument (argv)

```json
{
  "type": "agent-turn-complete",
  "thread-id": "uuid",
  "last-assistant-message": "Done."
}
```

**Event Mapping**:

| Event Type            | Maps To      |
| --------------------- | ------------ |
| `agent-turn-complete` | `AgentYield` |

### 4.3 OpenCode

**Input**: Explicit `--event` flag

```bash
agent-chime notify --source opencode --event AGENT_YIELD
```

**Event Mapping**:

| Plugin Event       | Maps To            |
| ------------------ | ------------------ |
| `session.idle`     | `AgentYield`       |
| `permission.asked` | `DecisionRequired` |
| `session.error`    | `ErrorRetry`       |

### 4.4 Event Mapping Summary

| Internal Event     | Claude                         | Codex                 | OpenCode           |
| ------------------ | ------------------------------ | --------------------- | ------------------ |
| `AgentYield`       | `Stop`, `Notification`         | `agent-turn-complete` | `session.idle`     |
| `DecisionRequired` | `PreToolUse` (AskUserQuestion) | —                     | `permission.asked` |
| `ErrorRetry`       | —                              | —                     | `session.error`    |

## 5. TTS Broker

### 5.1 Templates

Default templates for each event type:

```rust
const TEMPLATES: &[(&EventType, &str)] = &[
    (EventType::AgentYield, "Ready."),
    (EventType::DecisionRequired, "I need your input."),
    (EventType::ErrorRetry, "I hit an error. Please review."),
];
```

### 5.2 Policies

- Max spoken length: 1-2 sentences
- If summary exceeds limit, truncate and append "Check the screen."
- Respect per-event `mode` config (tts / earcon / silent)

### 5.3 Routing Logic

```rust
pub fn get_text_for_event(event: &Event, config: &Config) -> Option<String> {
    let event_config = config.events.get(&event.event_type)?;

    if !event_config.enabled {
        return None;
    }

    match event_config.mode {
        Mode::Tts => Some(event_config.template.clone()),
        Mode::Earcon => None, // Handled separately
        Mode::Silent => None,
    }
}
```

## 6. TTS Provider

### 6.1 Backend Abstraction

```rust
pub trait TtsBackend: Send + Sync {
    fn name(&self) -> &str;
    fn synthesize(&self, text: &str, config: &TtsConfig) -> Result<Audio>;
    fn synthesize_stream(&self, text: &str, config: &TtsConfig)
        -> Box<dyn Iterator<Item = Result<AudioChunk>> + '_>;
    fn supports_instruct(&self) -> bool;
}
```

### 6.2 Available Backends

| Backend   | Crate          | Features                                 |
| --------- | -------------- | ---------------------------------------- |
| PocketTTS | `pocket-tts`   | CPU default, Metal optional, streaming   |
| Qwen3TTS  | `qwen3-tts-rs` | VoiceDesign, emotion control, CUDA/Metal |

### 6.3 Backend Selection

```rust
pub fn select_backend(config: &Config) -> Box<dyn TtsBackend> {
    match config.tts.backend.as_deref() {
        Some("qwen3-tts") => Box::new(Qwen3TtsBackend::new()),
        _ => Box::new(PocketTtsBackend::new()), // Default
    }
}
```

### 6.4 Fallback Chain

```
Primary Backend → Fallback Backend → Earcon → Silent
```

If the primary backend fails:

1. Try fallback backend (pocket-tts if qwen3-tts failed)
2. Play earcon for the event type
3. Log warning and continue (never block agent output)

## 7. Audio Rendering

### 7.1 Playback

- Use `afplay` on macOS (built-in, no dependencies)
- Volume control via `-v` flag (0.0 - 1.0)
- Streaming: Write chunks to temp file, start playback immediately

```rust
pub struct Renderer {
    volume: f32,
    earcons_dir: PathBuf,
    cache: AudioCache,
}

impl Renderer {
    pub fn play(&self, audio: &[u8]) -> Result<()> {
        let temp = NamedTempFile::new()?;
        temp.write_all(audio)?;
        Command::new("afplay")
            .arg("-v").arg(self.volume.to_string())
            .arg(temp.path())
            .status()?;
        Ok(())
    }
}
```

### 7.2 Earcons

Bundled WAV files for each event type:

| Event              | File           | Characteristics                  |
| ------------------ | -------------- | -------------------------------- |
| `AgentYield`       | `yield.wav`    | Ascending major fifth (positive) |
| `DecisionRequired` | `decision.wav` | Rising triad (attention)         |
| `ErrorRetry`       | `error.wav`    | Descending sequence (alert)      |

### 7.3 Caching

- LRU cache at `~/.cache/agent-chime/`
- Key: `(text, voice, backend)`
- Max size: 100MB (configurable)
- Max entries: 1000 (configurable)

```rust
pub struct AudioCache {
    dir: PathBuf,
    max_size_bytes: u64,
    max_entries: usize,
}

impl AudioCache {
    pub fn get(&self, text: &str, voice: &str, backend: &str) -> Option<Vec<u8>>;
    pub fn put(&mut self, text: &str, voice: &str, backend: &str, audio: &[u8]);
}
```

## 8. Configuration

### 8.1 Config File

Location: `~/.config/agent-chime/config.json`

```rust
#[derive(Deserialize, Serialize)]
pub struct Config {
    pub tts: TtsConfig,
    pub volume: f32,
    pub events: HashMap<EventType, EventConfig>,
    pub cache_dir: Option<PathBuf>,
    pub cache_max_mb: Option<u64>,
    pub cache_max_entries: Option<usize>,
    pub earcons_dir: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
pub struct TtsConfig {
    pub backend: Option<String>,  // "pocket-tts" | "qwen3-tts"
    pub voice: Option<String>,
    pub instruct: Option<String>, // For qwen3-tts VoiceDesign
    pub timeout_seconds: u64,     // Circuit breaker for synthesis
    pub allow_downloads: bool,
    pub pocket_tts: PocketTtsConfig,
    pub qwen3_tts: Qwen3TtsConfig,
}

#[derive(Deserialize, Serialize)]
pub struct PocketTtsConfig {
    pub variant: Option<String>,  // e.g. "b6369a24"
    pub voice: Option<String>,    // e.g. "alba" or path
    pub use_metal: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct Qwen3TtsConfig {
    pub model: Option<String>,     // local path or HF ID
    pub tokenizer: Option<String>, // local path or HF ID
    pub speaker: Option<String>,   // e.g. "Ryan"
    pub language: Option<String>,  // e.g. "English"
    pub ref_audio: Option<String>, // voice clone WAV
    pub ref_text: Option<String>,  // reference transcript
    pub device: Option<String>,    // "auto" | "cpu" | "metal" | "cuda:0"
}

#[derive(Deserialize, Serialize)]
pub struct EventConfig {
    pub enabled: bool,
    pub mode: Mode,  // "tts" | "earcon" | "silent"
    pub template: String,
}
```

### 8.2 Config Locations (priority order)

1. `./agent-chime.json` (project-local)
2. `~/.config/agent-chime/config.json`
3. Built-in defaults

## 9. CLI Interface

### 9.1 Commands

```
agent-chime <COMMAND>

Commands:
  notify       Process a notification event
  system-info  Show system information (--json supported)
  models       List available TTS backends and models (--json supported)
  test-tts     Test TTS synthesis
  config       Manage configuration (--show/--init/--validate)
  help         Print help

Global Options:
  -v, --verbose  Enable verbose logging
  -h, --help     Print help
  -V, --version  Print version
```

### 9.2 Notify Command

```
agent-chime notify [OPTIONS]

Options:
  --source <SOURCE>  Source CLI [claude|codex|opencode]
  --event <EVENT>    Explicit event type [AGENT_YIELD|DECISION_REQUIRED|ERROR_RETRY]
  --backend <NAME>   Override TTS backend
```

### 9.3 Test-TTS Command

```
agent-chime test-tts [OPTIONS]

Options:
  --text <TEXT>        Text to synthesize
  --backend <NAME>     TTS backend [pocket-tts|qwen3-tts]
  --voice <VOICE>      Voice name
  --instruct <TEXT>    Emotion/style instruction (qwen3-tts only)
  --output <PATH>      Save audio to file
```

## 10. Project Structure

```
agent-chime-rs/
├── Cargo.toml
├── README.md
├── design.md
├── requirements.md
├── earcons/
│   ├── yield.wav
│   ├── decision.wav
│   └── error.wav
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library root
│   ├── cli.rs            # Argument parsing
│   ├── config.rs         # Configuration loading
│   ├── events.rs         # Event types and structs
│   ├── adapters/
│   │   ├── mod.rs
│   │   ├── claude.rs
│   │   ├── codex.rs
│   │   └── opencode.rs
│   ├── tts/
│   │   ├── mod.rs
│   │   ├── broker.rs     # Template routing
│   │   ├── provider.rs   # Backend abstraction
│   │   ├── pocket.rs     # PocketTTS backend
│   │   └── qwen3.rs      # Qwen3TTS backend
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── renderer.rs   # afplay wrapper
│   │   └── cache.rs      # LRU audio cache
│   └── system/
│       ├── mod.rs
│       └── detector.rs   # System info detection
└── tests/
    ├── adapters_test.rs
    ├── broker_test.rs
    └── integration_test.rs
```

## 11. Dependencies

### 11.1 Core Dependencies

```toml
[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# TTS backends (community Rust ports)
pocket-tts = { git = "https://github.com/kevinmichaelchen/pocket-tts" }
qwen3-tts = { git = "https://github.com/kevinmichaelchen/qwen3-tts-rs", optional = true }

# Audio
hound = "3"  # WAV I/O

# Utilities
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
directories = "5"  # XDG paths
```

### 11.2 Optional Features

```toml
[features]
default = ["pocket-tts-backend", "qwen3-tts-backend", "hub"]
pocket-tts-backend = ["dep:pocket-tts"]
qwen3-tts-backend = ["dep:qwen3-tts"]
metal = ["pocket-tts/metal", "qwen3-tts/metal"]
hub = ["qwen3-tts/hub"]
```

## 12. Error Handling

### 12.1 Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Adapter error: {0}")]
    Adapter(String),

    #[error("TTS error: {0}")]
    Tts(String),

    #[error("Playback error: {0}")]
    Playback(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 12.2 Failure Modes

| Failure                 | Recovery                  |
| ----------------------- | ------------------------- |
| Config parse error      | Use defaults, log warning |
| TTS backend unavailable | Try fallback backend      |
| TTS synthesis fails     | Play earcon               |
| Playback fails          | Log warning, continue     |
| Adapter parse error     | Log warning, no audio     |

**Critical principle**: Never block the agent output. All failures degrade
gracefully.

## 13. Testing Strategy

### 13.1 Unit Tests

- Adapter parsing (each CLI format)
- Broker template selection
- Config loading and validation
- Cache key generation

### 13.2 Integration Tests

- End-to-end notify flow (mock TTS)
- Config file precedence
- Earcon fallback behavior

### 13.3 Manual Testing

- Latency measurement (time to first audio)
- Audio quality verification
- CLI hook integration with actual tools

## 14. Future Work

- Cross-platform support (Windows, Linux)
- Additional TTS backends (Coqui, Piper)
- Voice cloning from user samples
- GUI configuration app
- Streaming synthesis for lower latency
