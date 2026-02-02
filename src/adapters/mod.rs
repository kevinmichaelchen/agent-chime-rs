use crate::events::{EventType, Source};

pub mod claude;
pub mod codex;
pub mod opencode;

pub fn parse_event(source: Source, payload: &str) -> anyhow::Result<Option<EventType>> {
    match source {
        Source::Claude => claude::parse_event(payload),
        Source::Codex => codex::parse_event(payload),
        Source::OpenCode => Ok(None),
    }
}
