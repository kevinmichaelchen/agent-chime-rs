use crate::events::{EventType, Source};
use serde_json::Value;

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

pub fn extract_summary(source: Source, payload: &str) -> Option<String> {
    let value: Value = serde_json::from_str(payload).ok()?;
    match source {
        Source::Claude => claude::extract_summary(&value),
        Source::Codex => codex::extract_summary(&value),
        Source::OpenCode => None,
    }
}

fn extract_text_field(value: &Value, pointers: &[&str]) -> Option<String> {
    for pointer in pointers {
        if let Some(text) = value.pointer(pointer).and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_text_from_content(value: &Value) -> Option<String> {
    let content = value.get("content")?.as_array()?;
    for item in content {
        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if let Some(text) = item.get("content").and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub(crate) fn extract_summary_common(value: &Value) -> Option<String> {
    extract_text_field(
        value,
        &[
            "/summary",
            "/message",
            "/text",
            "/prompt",
            "/question",
            "/last_message",
        ],
    )
    .or_else(|| extract_text_from_content(value))
}
