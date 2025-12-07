use std::{fs, path::Path};

use anyhow::{Context, Ok};
use sha2::{Digest, Sha256};

use crate::provider::PlaylistSnapshot;

pub fn compute_hash(snapshot : &PlaylistSnapshot) -> anyhow::Result<String>{
    let yaml = serde_yaml::to_string(snapshot)
        .with_context(||"Failed to serialize snapshot for hashing")?;

    let mut hasher = Sha256::new();
    hasher.update(yaml.as_bytes());
    let result = hasher.finalize();

    let hex = result.iter()
        .take(6) //6 bytes = 12 hex chars
        .map(|b| format!("{:02x}", b))
        .collect();

    Ok(hex)
}

pub fn save(snapshot: &PlaylistSnapshot, path: &Path) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(snapshot)
        .with_context(|| "Failed to serialize snapshot")?;
    
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }
    
    fs::write(path, yaml)
        .with_context(|| format!("Failed to write snapshot to {:?}", path))
}

pub fn load(path: &Path) -> anyhow::Result<PlaylistSnapshot> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read snapshot from {:?}", path))?;
    
    serde_yaml::from_str(&content)
        .with_context(|| "Failed to parse snapshot YAML")
}

pub fn snapshot_path(plr_dir: &Path, playlist_id: &str) -> std::path::PathBuf {
    plr_dir.join("playlists").join(playlist_id).join("playlist.yaml")
}