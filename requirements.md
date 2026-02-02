# Requirements

## 1. Overview

Provide audible feedback for agentic CLI workflows on macOS when the agent
yields or requires a decision. This is a native Rust rewrite of `agent-chime`
for faster startup, single-binary distribution, and lower memory usage.

## 2. Functional Requirements

### 2.1 Event Detection

- **FR-1.1**: Detect when the agent finishes and yields control (`AGENT_YIELD`).
- **FR-1.2**: Detect when the agent needs a user decision (`DECISION_REQUIRED`).
- **FR-1.3**: Detect recoverable errors when the CLI exposes error events
  (`ERROR_RETRY`).
- **FR-1.4**: If a CLI does not expose error events, do not synthesize
  `ERROR_RETRY`.
- **FR-1.5**: Currently, `ERROR_RETRY` is OpenCode-only via `session.error`.
- **FR-1.6**: Support integration with `claude`, `codex`, and `opencode` CLI
  tools.
- **FR-1.7**: Provide a pluggable adapter interface for adding new CLI tools.

### 2.2 TTS Broker

- **FR-2.1**: Convert events into short spoken prompts using templates.
- **FR-2.2**: Support per-event templates configurable by user.
- **FR-2.3**: Allow per-event mode selection: `tts`, `earcon`, or `silent`.
- **FR-2.4**: Truncate long summaries and append "Check the screen."

### 2.3 TTS Provider

- **FR-3.1**: Support multiple TTS backends via a unified interface.
- **FR-3.2**: Include PocketTTS backend (CPU-friendly, fast).
- **FR-3.3**: Include Qwen3-TTS backend (VoiceDesign, emotion control).
- **FR-3.4**: All TTS runs locally — no network requests for synthesis.
- **FR-3.5**: Support streaming synthesis for low first-audio latency.
- **FR-3.6**: Support emotion/style control via `instruct` parameter
  (Qwen3-TTS).
- **FR-3.7**: Allow optional model downloads when explicitly enabled; support
  local model paths for offline use.

### 2.4 Audio Rendering

- **FR-4.1**: Play audio immediately using `afplay` (macOS built-in).
- **FR-4.2**: Support volume control (0.0 - 1.0 range).
- **FR-4.3**: Support streaming playback when available.
- **FR-4.4**: Provide earcon fallback for each event type.

### 2.5 Caching

- **FR-5.1**: Cache synthesized audio for repeated prompts.
- **FR-5.2**: Cache key includes: `(text, voice, backend)`.
- **FR-5.3**: Implement LRU eviction policy.
- **FR-5.4**: Make cache size and entry limits configurable.

### 2.6 Configuration

- **FR-6.1**: Load config from `~/.config/agent-chime/config.json`.
- **FR-6.2**: Support project-local config at `./agent-chime.json`.
- **FR-6.3**: Fall back to built-in defaults when no config exists.
- **FR-6.4**: Allow configuration of: backend, voice, volume, instruct.
- **FR-6.5**: Allow per-event configuration: enabled, mode, template.

### 2.7 CLI Interface

- **FR-7.1**: Provide `notify` command for processing events.
- **FR-7.2**: Provide `system-info` command for system detection.
- **FR-7.3**: Provide `models` command for listing backends and cache status.
- **FR-7.4**: Provide `test-tts` command for testing synthesis.
- **FR-7.5**: Provide `config` command for configuration management.
- **FR-7.6**: Support `--json` flag for machine-readable output.
- **FR-7.7**: Support `--verbose` flag for debug logging.

### 2.8 Compatibility

- **FR-8.1**: CLI interface matches Python `agent-chime` for drop-in
  replacement.
- **FR-8.2**: Config file format compatible with Python version.
- **FR-8.3**: Same hook commands work for both implementations.

## 3. Non-Functional Requirements

### 3.1 Performance

- **NFR-1.1**: Binary startup time < 50ms.
- **NFR-1.2**: Time-to-first-audio < 500ms for cached prompts.
- **NFR-1.3**: Time-to-first-audio < 2s for uncached prompts.
- **NFR-1.4**: Memory usage < 500MB during synthesis (PocketTTS).
- **NFR-1.5**: Binary size < 50MB (without bundled models).

### 3.2 Reliability

- **NFR-2.1**: If TTS fails, fall back to earcon.
- **NFR-2.2**: Errors must never block agent output.
- **NFR-2.3**: Graceful degradation: TTS → Earcon → Silent → Continue.
- **NFR-2.4**: Log warnings on failures but do not exit with error.
- **NFR-2.5**: Enforce a synthesis circuit breaker (default 10s) that aborts
  long-running TTS and falls back to earcons.

### 3.3 Privacy

- **NFR-3.1**: All TTS runs locally on-device.
- **NFR-3.2**: No network requests for audio synthesis (downloads only for model
  assets).
- **NFR-3.3**: No telemetry or analytics.
- **NFR-3.4**: Model asset downloads must be opt-in and configurable.

### 3.4 Portability

- **NFR-4.1**: macOS only for initial release.
- **NFR-4.2**: Apple Silicon (M1/M2/M3/M4) optimized.
- **NFR-4.3**: Intel Mac support via CPU inference.
- **NFR-4.4**: Single static binary, no runtime dependencies.

### 3.5 Accessibility

- **NFR-5.1**: Distinct earcons for each event type.
- **NFR-5.2**: Volume normalization to prevent startling.
- **NFR-5.3**: Configurable volume level.

### 3.6 Maintainability

- **NFR-6.1**: Modular architecture with clear separation of concerns.
- **NFR-6.2**: Comprehensive error types with context.
- **NFR-6.3**: Unit tests for all core components.
- **NFR-6.4**: Integration tests for end-to-end flows.

## 4. Constraints

- **C-1**: Must integrate with `claude`, `codex`, and `opencode` CLI workflows.
- **C-2**: Must run locally on macOS without external services.
- **C-3**: Adapter interface must remain stable across releases.
- **C-4**: Must use `afplay` for audio playback (macOS built-in).
- **C-5**: TTS backends must be pure Rust (no Python runtime).

## 5. Dependencies

### 5.1 Required

| Crate                  | Purpose                    |
| ---------------------- | -------------------------- |
| `clap`                 | CLI argument parsing       |
| `serde` / `serde_json` | Config and payload parsing |
| `anyhow` / `thiserror` | Error handling             |
| `tracing`              | Logging                    |
| `directories`          | XDG config paths           |
| `hound`                | WAV I/O                    |

### 5.2 TTS Backends

| Crate        | Repository                    | Models              |
| ------------ | ----------------------------- | ------------------- |
| `pocket-tts` | kevinmichaelchen/pocket-tts   | PocketTTS 0.5B      |
| `qwen3-tts`  | kevinmichaelchen/qwen3-tts-rs | Qwen3-TTS 0.6B/1.7B |

## 6. Acceptance Criteria

### 6.1 Minimum Viable Product

- [ ] `agent-chime notify --source claude` processes stdin JSON
- [ ] `agent-chime notify --source codex` processes argv JSON
- [ ] `agent-chime notify --source opencode --event AGENT_YIELD` works
- [ ] PocketTTS backend synthesizes "Ready." and plays via afplay
- [ ] Earcon plays when TTS fails
- [ ] Config loads from `~/.config/agent-chime/config.json`
- [ ] `system-info` command shows system details
- [ ] `models` command lists available backends
- [ ] `test-tts` command synthesizes and plays audio

### 6.2 Feature Parity with Python

- [ ] All CLI commands match Python version
- [ ] Config file format compatible
- [ ] Same earcon files included
- [ ] Audio cache with LRU eviction
- [ ] Per-event mode configuration
- [ ] Volume control

### 6.3 Rust-Specific Improvements

- [ ] Startup time < 50ms (vs ~500ms Python)
- [ ] Single binary distribution
- [ ] Optional Metal acceleration
- [ ] Qwen3-TTS with emotion control

## 7. Open Questions

_None at this time. All CLI hook points are documented in design.md._
