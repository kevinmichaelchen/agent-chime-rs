use agent_chime::adapters::{claude, codex};
use agent_chime::events::EventType;

#[test]
fn claude_stop_maps_to_yield() {
    let payload = r#"{"hook_event_name":"Stop"}"#;
    let event = claude::parse_event(payload).unwrap();
    assert_eq!(event, Some(EventType::AgentYield));
}

#[test]
fn claude_pretooluse_ask_user_maps_to_decision() {
    let payload = r#"{"hook_event_name":"PreToolUse","tool_name":"AskUserQuestion"}"#;
    let event = claude::parse_event(payload).unwrap();
    assert_eq!(event, Some(EventType::DecisionRequired));
}

#[test]
fn codex_turn_complete_maps_to_yield() {
    let payload = r#"{"type":"agent-turn-complete"}"#;
    let event = codex::parse_event(payload).unwrap();
    assert_eq!(event, Some(EventType::AgentYield));
}
