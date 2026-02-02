use agent_chime::config::Config;
use agent_chime::tts;
use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.tts.backend = Some("pocket-tts".to_string());
    config.tts.timeout_seconds = 0;

    let phrases = [
        ("Ready.", "voicepack/audio/agent_ready.wav"),
        ("All set.", "voicepack/audio/agent_all_set.wav"),
        ("Your turn.", "voicepack/audio/agent_your_turn.wav"),
        ("Next step?", "voicepack/audio/agent_next_step.wav"),
        ("I'm done.", "voicepack/audio/agent_done.wav"),
        ("I need your input.", "voicepack/audio/decision_input.wav"),
        ("Question for you.", "voicepack/audio/decision_question.wav"),
        ("Your call.", "voicepack/audio/decision_call.wav"),
        ("Please choose.", "voicepack/audio/decision_choose.wav"),
        ("Something failed.", "voicepack/audio/error_failed.wav"),
        ("I hit an error.", "voicepack/audio/error_hit.wav"),
        ("Retry needed.", "voicepack/audio/error_retry.wav"),
        ("That timed out.", "voicepack/audio/error_timeout.wav"),
        ("Build complete.", "voicepack/audio/build_complete.wav"),
        ("Tests failed.", "voicepack/audio/tests_failed.wav"),
        ("Deploy complete.", "voicepack/audio/deploy_complete.wav"),
    ];

    for (text, file) in phrases {
        let path = Path::new(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if path.exists() {
            println!("Skipping {}", file);
            continue;
        }
        println!("Generating {}", file);
        let audio = tts::synthesize_in_process(text, &config, &config.tts.backend)?;
        fs::write(path, audio)?;
    }

    Ok(())
}
