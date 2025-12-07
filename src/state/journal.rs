use std::{fs::{self, OpenOptions}, io::Write, path::Path};

use anyhow::{Context, Ok};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Init,
    Pull,
    Push,
    Apply,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub operation: Operation,
    pub snapshot_hash: String,
    pub added: usize,
    pub removed: usize,
    pub moved: usize,
    pub message: Option<String>,
}

impl JournalEntry{
    pub fn new(op: Operation, hash: String, added: usize, removed: usize, moved: usize) -> Self{
        JournalEntry{
            timestamp : Utc::now(),
            operation : op,
            snapshot_hash : hash,
            added,
            removed,
            moved,
            message : None
        }
    }

    pub fn append(path : &Path, entry : &JournalEntry) -> anyhow::Result<()>{
        if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    let mut file = OpenOptions::new()
    .create(true).append(true).open(path)
    .with_context(|| format!("Failed to open journal {:?}", path))?;

    let line = serde_json::to_string(entry)
        .with_context(||"Failed to serialize journal entry")?;

    writeln!(file, "{}", line)
        .with_context(|| "Failed to write to journal")
    }

    pub fn read_all(path : &Path) -> anyhow::Result<Vec<JournalEntry>>{
        if !path.exists(){
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read journal {:?}", path))?;

        content
            .lines()
            .filter(|ln| !ln.trim().is_empty())
            .map(|ln| {
                serde_json::from_str(ln)
                    .with_context(|| format!("Failed to parse journal line: {}", ln))
            })
            .collect()
    }

    pub fn journal_path(plr_dir: &Path, playlist_id: &str) -> std::path::PathBuf {
    plr_dir.join("playlists").join(playlist_id).join("journal.log")
}
}

