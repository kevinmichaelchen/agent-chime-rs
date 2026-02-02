# agent-chime-rs

Audible notifications for agentic CLI workflows on macOS. Native Rust rewrite of
[agent-chime](https://github.com/kevinmichaelchen/agent-chime) for faster
startup and single-binary distribution.

When your agent yields or asks for a decision, you hear a short, clear audio cue
(TTS or earcon) while still seeing the full text output.

## Why Rust?

| Aspect           | Python (agent-chime)    | Rust (agent-chime-rs)              |
| ---------------- | ----------------------- | ---------------------------------- |
| **Startup**      | ~500ms                  | ~10ms                              |
| **Distribution** | Requires Python + venv  | Single binary                      |
| **TTS Backend**  | mlx-audio               | pocket-tts / qwen3-tts-rs          |
| **Memory**       | Higher (Python runtime) | Lower                              |
| **Install**      | `uv sync`               | `cargo install` or download binary |

## Features

- **Event hooks**: Detect `yield` and `decision` moments from agent tools
- **CLI adapters**: Pluggable adapters for `claude`, `codex`, `opencode`
- **TTS broker**: Normalize messages into compact, spoken prompts
- **Multiple TTS backends**: PocketTTS (CPU, fast) and Qwen3-TTS (quality,
  emotion)
- **Earcon fallback**: Short audio cues when TTS fails or for specific events
- **Audio caching**: LRU cache for repeated prompts

## Supported CLI Tools

- `claude` — Claude Code CLI
- `codex` — OpenAI Codex CLI
- `opencode` — OpenCode CLI

## TTS Backends

We integrate community Rust ports built on
[Candle](https://github.com/huggingface/candle):

| Backend                                                          | Model Class         | Speed        | Use Case                              |
| ---------------------------------------------------------------- | ------------------- | ------------ | ------------------------------------- |
| [pocket-tts](https://github.com/kevinmichaelchen/pocket-tts)     | PocketTTS 0.5B      | ~0.3x RT     | Default — fast, CPU-friendly          |
| [qwen3-tts-rs](https://github.com/kevinmichaelchen/qwen3-tts-rs) | Qwen3-TTS 0.6B/1.7B | ~0.5-0.7x RT | VoiceDesign and higher‑quality voices |

Both are local-first and support CPU inference. Qwen3-TTS is heavy; CPU-only
runs are not recommended (expect high latency). Prefer Metal/CUDA acceleration
and build with `--features metal`. Repeated prompts are cached, so common event
phrases become near-instant after the first run.

## Quick Start

### Installation

```bash
# From crates.io (once published)
cargo install agent-chime

# Or build from source
git clone https://github.com/kevinmichaelchen/agent-chime-rs
cd agent-chime-rs
cargo build --release
```

### Configure Your CLI Tool

#### Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": [
      { "type": "command", "command": "agent-chime notify --source claude" }
    ],
    "Notification": [
      { "type": "command", "command": "agent-chime notify --source claude" }
    ],
    "PreToolUse": [
      { "type": "command", "command": "agent-chime notify --source claude" }
    ]
  }
}
```

Note: `PreToolUse` is only used for `AskUserQuestion` events; configure your
Claude hooks to forward those to `agent-chime` so decision prompts are audible.

#### Codex

Add to `~/.codex/config.toml`:

```toml
notify = ["agent-chime", "notify", "--source", "codex"]
```

#### OpenCode

Create `.opencode/plugin/agent-chime.js`:

```javascript
export const AgentChimePlugin = async ({ $ }) => ({
  event: async ({ event }) => {
    if (event.type === "session.idle")
      await $`agent-chime notify --source opencode --event AGENT_YIELD`;
    if (event.type === "session.error")
      await $`agent-chime notify --source opencode --event ERROR_RETRY`;
    if (event.type === "permission.asked")
      await $`agent-chime notify --source opencode --event DECISION_REQUIRED`;
  },
});
```

## Usage

### CLI Commands

```bash
# Show system info and recommended TTS backend
agent-chime system-info
agent-chime system-info --json

# List available TTS models and cache status
agent-chime models
agent-chime models --json

# Test TTS synthesis
agent-chime test-tts
agent-chime test-tts --text "Hello world"
agent-chime test-tts --backend pocket-tts
agent-chime test-tts --backend qwen3-tts --instruct "A cheerful voice"

# Manage configuration
agent-chime config              # Show config path
agent-chime config --show       # Show current config as JSON
agent-chime config --init       # Create default config file
agent-chime config --validate   # Validate configuration

# Process notifications (called by hooks)
agent-chime notify --source claude    # Reads JSON from stdin
agent-chime notify --source codex     # Reads JSON from argv
agent-chime notify --source opencode --event AGENT_YIELD
agent-chime notify --source opencode --event AGENT_YIELD --summary "Build complete"
```

### Configuration

Create `~/.config/agent-chime/config.json`:

```json
{
  "tts": {
    "backend": "pocket-tts",
    "voice": null,
    "instruct": null,
    "timeout_seconds": 10,
    "allow_downloads": true,
    "pocket_tts": {
      "variant": "b6369a24",
      "voice": "alba",
      "use_metal": false
    },
    "qwen3_tts": {
      "model": null,
      "tokenizer": null,
      "speaker": "Ryan",
      "language": "English",
      "ref_audio": null,
      "ref_text": null,
      "device": "auto"
    }
  },
  "volume": 0.8,
  "cache_max_mb": 100,
  "cache_max_entries": 1000,
  "voicepack": {
    "enabled": false,
    "manifest_path": "./voicepack/manifest.json",
    "routes": [
      {
        "pattern": "error|failed|timeout",
        "phrases": ["system.timeout_fallback"],
        "events": ["ERROR_RETRY"]
      }
    ]
  },
  "events": {
    "AGENT_YIELD": {
      "enabled": true,
      "mode": "tts",
      "template": "Ready."
    },
    "DECISION_REQUIRED": {
      "enabled": true,
      "mode": "tts",
      "template": "I need your input."
    },
    "ERROR_RETRY": {
      "enabled": true,
      "mode": "earcon"
    }
  }
}
```

Audio is cached on disk (LRU by modification time) to speed up repeated prompts.
Tune `cache_max_mb` and `cache_max_entries` to fit your system. If synthesis
exceeds `tts.timeout_seconds`, the process is terminated and earcons are used
instead. Set `timeout_seconds` to `0` to disable the circuit breaker.

### Voice Packs (Pre-Generated Audio)

Enable `voicepack.enabled` to play pre-generated audio files instead of doing
on-device TTS. `manifest_path` should point at a voice pack manifest (see
`agent-chime-voicepack-spec`). When `routes` match the last message, agent-chime
will choose a phrase from those keys; otherwise it falls back to the event
mapping in the manifest.

If `voicepack.enabled` is true and a pack can be selected, playback happens
before any TTS or earcon fallback.

This repo ships a small dev voice pack under `./voicepack`. Regenerate the
audio files with:

```bash
cargo run --example voicepack_gen
```

#### Selection Order (Voice Packs)

1. Extract a `summary` from the CLI payload (Claude/Codex). For OpenCode, pass
   `--summary` to enable routing by message text.
2. If `routes` are configured, scan in order and pick the first regex match
   that also matches the current event (if `events` is set).
3. If a route matches, select a random phrase from `route.phrases`. Otherwise,
   fall back to `manifest.events[<event>]`.
4. Select a random variant and play it. If playback fails, fall back to TTS
   and then earcons.

Enable debug logs to see which phrase/file was selected:

```bash
agent-chime -v notify --source opencode --event AGENT_YIELD --summary "Build complete"
```

### TTS Backend Selection

| Backend      | Best For                        | Voice Options                             |
| ------------ | ------------------------------- | ----------------------------------------- |
| `pocket-tts` | Fast notifications, low memory  | Predefined voices (e.g. `alba`)           |
| `qwen3-tts`  | High quality, expressive speech | Speakers, voice cloning, VoiceDesign text |

For VoiceDesign speech with Qwen3-TTS:

```bash
agent-chime test-tts \
  --backend qwen3-tts \
  --text "I need your input on this decision." \
  --instruct "A calm, professional voice with slight urgency"
```

### Model Downloads

Both backends can download model assets from Hugging Face when `allow_downloads`
is `true`. Set it to `false` if you require fully offline operation and provide
local paths for `qwen3_tts.model` (and optional `qwen3_tts.tokenizer`) and
`pocket_tts.voice` where needed. If the model repo requires authentication, set
`HF_TOKEN` in your environment.

### Qwen3-TTS Setup (Optional)

To use Qwen3-TTS, point `tts.qwen3_tts.model` at a local model directory or a
Hugging Face model ID:

```json
{
  "tts": {
    "backend": "qwen3-tts",
    "allow_downloads": true,
    "qwen3_tts": {
      "model": "Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice",
      "speaker": "Ryan",
      "language": "English"
    }
  }
}
```

For Base models that support voice cloning, set `qwen3_tts.ref_audio` (WAV) and
optionally `qwen3_tts.ref_text` to enable ICL-style cloning.

## Event Types

| Event               | Description                      | Default Mode              |
| ------------------- | -------------------------------- | ------------------------- |
| `AGENT_YIELD`       | Agent finished, waiting for user | TTS: "Ready."             |
| `DECISION_REQUIRED` | Agent needs explicit input       | TTS: "I need your input." |
| `ERROR_RETRY`       | Recoverable error occurred       | Earcon                    |

## Documentation

- [`requirements.md`](requirements.md) — Functional and non-functional
  requirements
- [`design.md`](design.md) — Architecture, event model, and data flow

## Scope

### In Scope

- macOS only (Apple Silicon optimized)
- English-only prompts
- Short spoken messages (1-2 sentences max)
- Minimal setup and config
- Adapter system for `claude`, `codex`, `opencode`

### Out of Scope (initial release)

- Cross-platform support (Windows, Linux)
- Long-form narration
- GUI configuration

## Building

Prereqs:

- `cmake` (required by PocketTTS dependency)

```bash
# Debug build
cargo build

# Release build
cargo build --release

# With Metal acceleration (macOS)
cargo build --release --features metal

# Minimal build (disable heavy TTS backends)
cargo build --release --no-default-features

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- system-info
```

## License

MIT

## Credits

- [pocket-tts](https://github.com/babybirdprd/pocket-tts) — Rust PocketTTS
  implementation
- [qwen3-tts-rs](https://github.com/TrevorS/qwen3-tts-rs) — Rust Qwen3-TTS
  implementation
- [candle](https://github.com/huggingface/candle) — Rust ML framework
- [agent-chime](https://github.com/kevinmichaelchen/agent-chime) — Original
  Python implementation
