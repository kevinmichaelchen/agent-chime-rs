use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    #[value(alias = "AGENT_YIELD", alias = "agent_yield")]
    AgentYield,
    #[value(alias = "DECISION_REQUIRED", alias = "decision_required")]
    DecisionRequired,
    #[value(alias = "ERROR_RETRY", alias = "error_retry")]
    ErrorRetry,
}

impl EventType {
    pub fn default_template(self) -> &'static str {
        match self {
            EventType::AgentYield => "Ready.",
            EventType::DecisionRequired => "I need your input.",
            EventType::ErrorRetry => "I hit an error. Please review.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Claude,
    Codex,
    #[value(name = "opencode", alias = "open-code")]
    OpenCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: EventType,
    pub source: Source,
    pub timestamp: DateTime<Utc>,
    pub summary: Option<String>,
    pub context: Option<serde_json::Value>,
    pub priority: Priority,
}

impl Event {
    pub fn new(event_type: EventType, source: Source) -> Self {
        let priority = match event_type {
            EventType::DecisionRequired | EventType::ErrorRetry => Priority::High,
            EventType::AgentYield => Priority::Normal,
        };

        Self {
            event_type,
            source,
            timestamp: Utc::now(),
            summary: None,
            context: None,
            priority,
        }
    }
}
