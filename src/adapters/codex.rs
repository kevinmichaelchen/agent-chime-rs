use crate::events::EventType;
use anyhow::Context;
use serde_json::Value;

pub fn parse_event(payload: &str) -> anyhow::Result<Option<EventType>> {
    let value: Value = serde_json::from_str(payload).context("parse codex payload")?;
    let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "agent-turn-complete" => Ok(Some(EventType::AgentYield)),
        _ => Ok(None),
    }
}
