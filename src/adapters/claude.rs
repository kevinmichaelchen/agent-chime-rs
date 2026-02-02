use crate::events::EventType;
use anyhow::Context;
use serde_json::Value;

pub fn parse_event(payload: &str) -> anyhow::Result<Option<EventType>> {
    let value: Value = serde_json::from_str(payload).context("parse claude payload")?;
    let hook = value
        .get("hook_event_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match hook {
        "Stop" | "Notification" => Ok(Some(EventType::AgentYield)),
        "PreToolUse" => {
            let tool_name = value
                .get("tool_name")
                .and_then(|v| v.as_str())
                .or_else(|| value.get("tool").and_then(|v| v.as_str()))
                .or_else(|| value.pointer("/tool/name").and_then(|v| v.as_str()))
                .unwrap_or("");

            if tool_name == "AskUserQuestion" {
                Ok(Some(EventType::DecisionRequired))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}
