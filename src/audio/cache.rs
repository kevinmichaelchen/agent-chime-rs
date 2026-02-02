use anyhow::Context;
use filetime::{set_file_mtime, FileTime};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AudioCache {
    pub dir: PathBuf,
    pub max_size_bytes: u64,
    pub max_entries: usize,
}

impl AudioCache {
    pub fn new(dir: PathBuf, max_size_bytes: u64, max_entries: usize) -> Self {
        Self {
            dir,
            max_size_bytes,
            max_entries,
        }
    }

    pub fn key(backend: &str, text: &str, config_json: &str) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(backend.as_bytes());
        hasher.update(b"\0");
        hasher.update(text.as_bytes());
        hasher.update(b"\0");
        hasher.update(config_json.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.path_for_key(key);
        let data = fs::read(&path).ok()?;
        let _ = set_file_mtime(&path, FileTime::now());
        Some(data)
    }

    pub fn put(&self, key: &str, audio: &[u8]) -> anyhow::Result<()> {
        if audio.is_empty() {
            return Ok(());
        }

        if audio.len() as u64 > self.max_size_bytes {
            return Ok(());
        }

        fs::create_dir_all(&self.dir).context("create cache dir")?;
        let path = self.path_for_key(key);
        let tmp = self.dir.join(format!("{}.tmp", key));

        fs::write(&tmp, audio).context("write cache temp")?;
        fs::rename(&tmp, &path).context("rename cache file")?;

        self.prune()?;
        Ok(())
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{}.wav", key))
    }

    fn prune(&self) -> anyhow::Result<()> {
        if !_dir_exists(&self.dir) {
            return Ok(());
        }

        let mut entries = Vec::new();
        let mut total_size = 0u64;

        for entry in fs::read_dir(&self.dir).context("read cache dir")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("wav") {
                continue;
            }
            let meta = entry.metadata()?;
            let size = meta.len();
            total_size += size;
            let mtime = FileTime::from_last_modification_time(&meta);
            entries.push((path, mtime, size));
        }

        // Sort oldest first for eviction
        entries.sort_by_key(|(_, mtime, _)| mtime.seconds());

        let mut current_entries = entries.len();
        let mut current_size = total_size;

        for (path, _mtime, size) in entries {
            if current_size <= self.max_size_bytes && current_entries <= self.max_entries {
                break;
            }
            let _ = fs::remove_file(&path);
            current_size = current_size.saturating_sub(size);
            current_entries = current_entries.saturating_sub(1);
        }

        Ok(())
    }
}

fn _dir_exists(path: &Path) -> bool {
    path.is_dir()
}
