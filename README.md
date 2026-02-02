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

We use pure Rust TTS implementations built on
[Candle](https://github.com/huggingface/candle):

| Backend                                                 | Model          | Speed        | Use Case                         |
| ------------------------------------------------------- | -------------- | ------------ | -------------------------------- |
| [pocket-tts](https://github.com/babybirdprd/pocket-tts) | PocketTTS 0.5B | ~0.3x RT     | Default — fast, CPU-friendly     |
| [qwen3-tts-rs](https://github.com/TrevorS/qwen3-tts-rs) | Qwen3-TTS 1.7B | ~0.5-0.7x RT | VoiceDesign with emotion control |

Both support:

- CPU inference (no GPU required)
- Optional Metal acceleration on macOS
- Streaming synthesis for low latency
- Voice cloning

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
    ]
  }
}
```

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
```

### Configuration

Create `~/.config/agent-chime/config.json`:

```json
{
  "tts": {
    "backend": "pocket-tts",
    "voice": null,
    "instruct": null
  },
  "volume": 0.8,
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

### TTS Backend Selection

| Backend      | Best For                        | Emotion Control        |
| ------------ | ------------------------------- | ---------------------- |
| `pocket-tts` | Fast notifications, low memory  | No                     |
| `qwen3-tts`  | High quality, expressive speech | Yes (via `--instruct`) |

For emotion-controlled speech with Qwen3-TTS:

```bash
agent-chime test-tts \
  --backend qwen3-tts \
  --text "I need your input on this decision." \
  --instruct "A calm, professional voice with slight urgency"
```

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

```bash
# Debug build
cargo build

# Release build
cargo build --release

# With Metal acceleration (macOS)
cargo build --release --features metal

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
