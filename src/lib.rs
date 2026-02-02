pub mod adapters;
pub mod audio;
pub mod cli;
pub mod config;
pub mod events;
pub mod system;
pub mod tts;

use anyhow::Context;
use cli::{Cli, Commands};
use events::{Event, Source};
use std::io::Read;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    setup_tracing(cli.verbose);

    match cli.command {
        Commands::Notify(args) => notify(args),
        Commands::SystemInfo(args) => system_info(args),
        Commands::Models(args) => models(args),
        Commands::TestTts(args) => test_tts(args),
        Commands::Config(args) => config_cmd(args),
        Commands::InternalSynthesize(args) => internal_synthesize(args),
    }
}

fn setup_tracing(verbose: bool) {
    let filter = if verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn notify(args: cli::NotifyArgs) -> anyhow::Result<()> {
    let config = config::Config::load().context("load config")?;

    let event_type = match args.event {
        Some(event) => Some(event),
        None => {
            if args.source == Source::OpenCode {
                anyhow::bail!("--event is required when --source opencode is used");
            }

            let payload = args.payload.or_else(read_stdin_json);
            let payload = match payload {
                Some(payload) => payload,
                None => {
                    tracing::warn!("no payload provided; skipping");
                    return Ok(());
                }
            };

            adapters::parse_event(args.source, &payload).context("parse event payload")?
        }
    };

    let event_type = match event_type {
        Some(event) => event,
        None => {
            tracing::warn!("event not recognized; skipping");
            return Ok(());
        }
    };

    let event = Event::new(event_type, args.source);

    let text = tts::broker::get_text_for_event(&event, &config);

    if let Some(text) = text {
        if let Err(err) = tts::synthesize_and_play(&text, &config, &args.backend) {
            tracing::warn!(error = ?err, "tts failed; trying earcon");
            audio::earcon::play_for_event(event.event_type, &config)?;
        }
        return Ok(());
    }

    if audio::earcon::should_play(event.event_type, &config) {
        audio::earcon::play_for_event(event.event_type, &config)?;
    }

    Ok(())
}

fn system_info(args: cli::SystemInfoArgs) -> anyhow::Result<()> {
    let info = system::detect();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!("OS: {}", info.os);
    println!("Arch: {}", info.arch);
    if let Some(cores) = info.cpu_cores {
        println!("CPU cores: {}", cores);
    }
    if let Some(recommended) = info.recommended_backends {
        println!("Recommended backends: {}", recommended.join(", "));
    }

    Ok(())
}

fn models(args: cli::ModelsArgs) -> anyhow::Result<()> {
    let info = tts::models_info()?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!("Available backends:");
    for backend in info.backends {
        let status = if backend.available { "available" } else { "unavailable" };
        println!("- {} ({})", backend.name, status);
    }

    if let Some(cache_dir) = info.cache_dir {
        println!("Cache dir: {}", cache_dir.display());
    }

    Ok(())
}

fn test_tts(args: cli::TestTtsArgs) -> anyhow::Result<()> {
    let mut config = config::Config::load().context("load config")?;
    if let Some(voice) = args.voice {
        config.tts.voice = Some(voice);
    }
    if let Some(instruct) = args.instruct {
        config.tts.instruct = Some(instruct);
    }
    let text = args.text.unwrap_or_else(|| "Hello world".to_string());

    let audio = tts::synthesize(&text, &config, &args.backend).context("tts synthesis")?;

    if let Some(path) = args.output {
        std::fs::write(path, &audio).context("write output")?;
    }

    tts::play_audio(&audio, config.volume)?;

    Ok(())
}

fn config_cmd(args: cli::ConfigArgs) -> anyhow::Result<()> {
    if args.init {
        let path = config::Config::init_default()?;
        println!("Initialized config at {}", path.display());
        return Ok(());
    }

    if args.show {
        let config = config::Config::load()?;
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }

    if args.validate {
        let config = config::Config::load()?;
        config.validate()?;
        println!("Config OK");
        return Ok(());
    }

    let path = config::Config::default_path()?;
    println!("{}", path.display());
    Ok(())
}

fn internal_synthesize(args: cli::InternalSynthesizeArgs) -> anyhow::Result<()> {
    let raw = read_stdin_bytes().context("read config from stdin")?;
    if raw.is_empty() {
        anyhow::bail!("internal synth expects config JSON on stdin");
    }
    let config: config::Config =
        serde_json::from_slice(&raw).context("parse config JSON")?;
    let audio = tts::synthesize_in_process(&args.text, &config, &args.backend)?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, &audio).context("write audio to stdout")?;
    std::io::Write::flush(&mut stdout)?;
    Ok(())
}

fn read_stdin_json() -> Option<String> {
    let mut input = String::new();
    let mut stdin = std::io::stdin();
    if stdin.read_to_string(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    } else {
        None
    }
}

fn read_stdin_bytes() -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut stdin = std::io::stdin();
    stdin.read_to_end(&mut buf)?;
    Ok(buf)
}
