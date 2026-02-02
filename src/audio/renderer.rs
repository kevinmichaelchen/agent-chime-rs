use anyhow::{bail, Context};
use std::path::Path;
use std::process::Command;

pub fn play_file(path: &Path, volume: f32) -> anyhow::Result<()> {
    let status = Command::new("afplay")
        .arg("-v")
        .arg(volume.to_string())
        .arg(path)
        .status()
        .with_context(|| format!("play audio with afplay: {}", path.display()))?;

    if !status.success() {
        bail!("afplay exited with status {status}");
    }

    Ok(())
}

pub fn play_bytes(bytes: &[u8], volume: f32) -> anyhow::Result<()> {
    let mut temp = tempfile::NamedTempFile::new().context("create temp file")?;
    std::io::Write::write_all(&mut temp, bytes).context("write audio bytes")?;
    play_file(temp.path(), volume)
}
