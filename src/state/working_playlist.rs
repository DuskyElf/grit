use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct WorkingState {
    pub playlist_id: String,
    #[serde(default)]
    pub last_track_index: Option<usize>,
}

pub fn config_path(grit_dir: &Path) -> PathBuf {
    grit_dir.join("working_playlist.json")
}

pub fn load(grit_dir: &Path) -> Result<String> {
    let state = load_state(grit_dir)?;
    Ok(state.playlist_id)
}

pub fn load_state(grit_dir: &Path) -> Result<WorkingState> {
    let path = config_path(grit_dir);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read working state from {:?}", path))?;
    let state: WorkingState =
        serde_json::from_str(&content).with_context(|| "Failed to parse working state")?;
    Ok(state)
}

pub fn save(grit_dir: &Path, playlist_id: &str) -> Result<()> {
    let state = WorkingState {
        playlist_id: playlist_id.to_string(),
        last_track_index: None,
    };
    save_state(grit_dir, &state)
}

pub fn save_state(grit_dir: &Path, state: &WorkingState) -> Result<()> {
    let path = config_path(grit_dir);
    let content = serde_json::to_string_pretty(state)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write working state to {:?}", path))
}

pub fn save_last_track(grit_dir: &Path, track_index: usize) -> Result<()> {
    let mut state = load_state(grit_dir).unwrap_or_default();
    state.last_track_index = Some(track_index);
    save_state(grit_dir, &state)
}
