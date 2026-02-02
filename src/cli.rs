use clap::{Args, Parser, Subcommand};

use crate::events::{EventType, Source};

#[derive(Parser, Debug)]
#[command(name = "agent-chime", version, about = "Audible notifications for agentic CLI workflows")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Enable verbose logging")]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Notify(NotifyArgs),
    SystemInfo(SystemInfoArgs),
    Models(ModelsArgs),
    TestTts(TestTtsArgs),
    Config(ConfigArgs),
    #[command(name = "__synthesize", hide = true)]
    InternalSynthesize(InternalSynthesizeArgs),
}

#[derive(Args, Debug)]
pub struct NotifyArgs {
    #[arg(long, value_enum, help = "Source CLI")]
    pub source: Source,

    #[arg(long, value_enum, help = "Explicit event type")]
    pub event: Option<EventType>,

    #[arg(long, help = "Override TTS backend")]
    pub backend: Option<String>,

    #[arg(value_name = "JSON", help = "Event payload JSON (for claude/codex)")]
    pub payload: Option<String>,
}

#[derive(Args, Debug)]
pub struct SystemInfoArgs {
    #[arg(long, help = "Output as JSON")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ModelsArgs {
    #[arg(long, help = "Output as JSON")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct TestTtsArgs {
    #[arg(long, help = "Text to synthesize")]
    pub text: Option<String>,

    #[arg(long, help = "TTS backend")]
    pub backend: Option<String>,

    #[arg(long, help = "Voice name")]
    pub voice: Option<String>,

    #[arg(long, help = "Emotion/style instruction (qwen3-tts)")]
    pub instruct: Option<String>,

    #[arg(long, value_name = "PATH", help = "Save audio to file")]
    pub output: Option<std::path::PathBuf>,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[arg(long, help = "Show current config as JSON")]
    pub show: bool,

    #[arg(long, help = "Create default config file")]
    pub init: bool,

    #[arg(long, help = "Validate configuration")]
    pub validate: bool,
}

#[derive(Args, Debug)]
pub struct InternalSynthesizeArgs {
    #[arg(long, help = "Text to synthesize")]
    pub text: String,

    #[arg(long, help = "TTS backend")]
    pub backend: Option<String>,
}
