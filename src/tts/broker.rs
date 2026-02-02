use crate::config::{Config, Mode};
use crate::events::Event;

pub fn get_text_for_event(event: &Event, config: &Config) -> Option<String> {
    let event_config = config.events.get(&event.event_type)?;
    if !event_config.enabled {
        return None;
    }

    match event_config.mode {
        Mode::Tts => event_config
            .template
            .clone()
            .or_else(|| Some(event.event_type.default_template().to_string())),
        Mode::Earcon | Mode::Silent => None,
    }
}
