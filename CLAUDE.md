# CLAUDE.md

Development instructions for agent-chime-rs.

## Project Overview

Native Rust rewrite of
[agent-chime](https://github.com/kevinmichaelchen/agent-chime) for audible
notifications in agentic CLI workflows.

## Key Documentation

- `README.md` — User-facing documentation
- `design.md` — Architecture and technical design
- `requirements.md` — Functional and non-functional requirements

## Build Commands

```bash
# Build
cargo build
cargo build --release
cargo build --release --features metal

# Test
cargo test
cargo test -- --nocapture

# Run
cargo run -- --help
cargo run -- system-info
cargo run -- test-tts --text "Hello"

# Lint
cargo clippy
cargo fmt --check
```

## Architecture

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library root
├── cli.rs            # clap argument parsing
├── config.rs         # Config loading
├── events.rs         # Event types
├── adapters/         # CLI-specific parsers
├── tts/              # TTS backends
├── audio/            # Playback and cache
└── system/           # System detection
```

## TTS Backends

- **pocket-tts**: Default, CPU-friendly, ~0.3x RT
- **qwen3-tts-rs**: Optional, VoiceDesign with emotion control

## Coding Conventions

- Use `thiserror` for error types
- Use `anyhow` for error propagation in main
- Use `tracing` for logging (not `println!`)
- Prefer `&str` over `String` in function params
- All public types need doc comments
- Tests go in `tests/` directory (integration) or inline `#[cfg(test)]` (unit)

## Dependencies

Reference implementations:

- https://github.com/babybirdprd/pocket-tts
- https://github.com/TrevorS/qwen3-tts-rs

## Parity Checklist

Must match Python agent-chime:

- [ ] `notify` command with --source and --event flags
- [ ] `system-info` command with --json flag
- [ ] `models` command with --json flag
- [ ] `test-tts` command
- [ ] `config` command (--show, --init, --validate)
- [ ] Config file at ~/.config/agent-chime/config.json
- [ ] Audio cache at ~/.cache/agent-chime/
- [ ] Earcon files bundled (yield.wav, decision.wav, error.wav)
