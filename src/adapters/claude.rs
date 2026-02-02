use crate::events::EventType;
use anyhow::Context;
use serde_json::Value;

use super::extract_summary_common;

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

pub fn extract_summary(value: &Value) -> Option<String> {
    extract_summary_common(value)
        .or_else(|| {
            value
                .pointer("/tool_input/question")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            value
                .pointer("/tool_input/prompt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}
