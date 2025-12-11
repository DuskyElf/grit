use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub fn config_path(grit_dir: &Path) -> PathBuf {
    grit_dir.join("working_playlist")
}

pub fn load(grit_dir: &Path) -> Result<String> {
    let path = config_path(grit_dir);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read working playlist from {:?}", path))?;
    Ok(content.trim().to_string())
}

pub fn save(grit_dir: &Path, playlist_id: &str) -> Result<()> {
    let path = config_path(grit_dir);
    fs::write(&path, playlist_id)
        .with_context(|| format!("Failed to write working playlist to {:?}", path))
}
