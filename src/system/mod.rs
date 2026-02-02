use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: Option<usize>,
    pub recommended_backends: Option<Vec<String>>,
}

pub fn detect() -> SystemInfo {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let cpu_cores = std::thread::available_parallelism().ok().map(|n| n.get());

    let recommended_backends = cpu_cores.map(|cores| {
        if cores >= 8 {
            vec!["pocket-tts".to_string(), "qwen3-tts".to_string()]
        } else {
            vec!["pocket-tts".to_string()]
        }
    });

    SystemInfo {
        os,
        arch,
        cpu_cores,
        recommended_backends,
    }
}
