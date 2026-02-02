use crate::events::EventType;
use anyhow::Context;
use serde_json::Value;

use super::extract_summary_common;

pub fn parse_event(payload: &str) -> anyhow::Result<Option<EventType>> {
    let value: Value = serde_json::from_str(payload).context("parse codex payload")?;
    let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "agent-turn-complete" => Ok(Some(EventType::AgentYield)),
        _ => Ok(None),
    }
}

pub fn extract_summary(value: &Value) -> Option<String> {
    extract_summary_common(value)
        .or_else(|| {
            value
                .pointer("/message/content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            value
                .pointer("/output")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}
