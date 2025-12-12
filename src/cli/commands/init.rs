use crate::provider::{Provider, ProviderKind, SpotifyProvider, YoutubeProvider};
use crate::state::{
    clear_staged, credentials, snapshot, working_playlist, JournalEntry, Operation,
};
use anyhow::{Context, Result};
use std::path::Path;

/// Extract playlist/album ID from URL or return as-is if already an ID
fn extract_id(input: &str) -> String {
    // Handle Spotify playlist URLs
    if input.contains("spotify.com/playlist/") {
        return input
            .split("playlist/")
            .nth(1)
            .and_then(|s| s.split('?').next())
            .unwrap_or(input)
            .to_string();
    }

    // Handle Spotify album URLs
    if input.contains("spotify.com/album/") {
        return input
            .split("album/")
            .nth(1)
            .and_then(|s| s.split('?').next())
            .unwrap_or(input)
            .to_string();
    }

    // Handle YouTube URLs
    if input.contains("youtube.com") || input.contains("youtu.be") {
        if let Some(list_start) = input.find("list=") {
            let id_part = &input[list_start + 5..];
            return id_part.split('&').next().unwrap_or(input).to_string();
        }
    }

    input.to_string()
}

fn is_album_url(input: &str) -> bool {
    input.contains("spotify.com/album/")
}

/// Detect provider from playlist URL
pub fn detect_provider(input: &str) -> Option<ProviderKind> {
    if input.contains("spotify.com") {
        Some(ProviderKind::Spotify)
    } else if input.contains("youtube.com") || input.contains("youtu.be") {
        Some(ProviderKind::Youtube)
    } else {
        None
    }
}

pub async fn run(provider: ProviderKind, input: &str, grit_dir: &Path) -> Result<()> {
    let id = extract_id(input);
    let is_album = is_album_url(input);

    let snapshot_path = snapshot::snapshot_path(grit_dir, &id);
    if snapshot_path.exists() {
        anyhow::bail!(
            "{} {} already initialized. Use 'grit pull' to update.",
            if is_album { "Album" } else { "Playlist" },
            id
        );
    }

    let token = credentials::load(grit_dir, provider)?.context(format!(
        "No credentials found. Please run 'grit auth {provider}' first."
    ))?;

    let playlist = match provider {
        ProviderKind::Spotify => {
            let client_id =
                std::env::var("SPOTIFY_CLIENT_ID").context("SPOTIFY_CLIENT_ID not set")?;
            let client_secret =
                std::env::var("SPOTIFY_CLIENT_SECRET").context("SPOTIFY_CLIENT_SECRET not set")?;

            let spotify =
                SpotifyProvider::new(client_id, client_secret).with_token(&token, grit_dir);

            if is_album {
                println!("Fetching album {}...", id);
                spotify.fetch_album(&id).await?
            } else {
                println!("Fetching playlist {}...", id);
                spotify.fetch(&id).await?
            }
        }
        ProviderKind::Youtube => {
            let client_id =
                std::env::var("YOUTUBE_CLIENT_ID").context("YOUTUBE_CLIENT_ID not set")?;
            let client_secret =
                std::env::var("YOUTUBE_CLIENT_SECRET").context("YOUTUBE_CLIENT_SECRET not set")?;

            let youtube =
                YoutubeProvider::new(client_id, client_secret).with_token(&token, grit_dir);
            println!("Fetching playlist {}...", id);
            youtube.fetch(&id).await?
        }
    };

    println!("  Name: {}", playlist.name);
    println!("  Tracks: {}", playlist.tracks.len());

    snapshot::save(&playlist, &snapshot_path)?;
    let hash = snapshot::compute_hash(&playlist)?;

    snapshot::save_by_hash(&playlist, &hash, grit_dir, &id)?;

    let journal_path = JournalEntry::journal_path(grit_dir, &id);
    let entry = JournalEntry::new(Operation::Init, hash, playlist.tracks.len(), 0, 0);
    JournalEntry::append(&journal_path, &entry)?;

    clear_staged(grit_dir, &id)?;

    println!(
        "\n{} initialized!",
        if is_album { "Album" } else { "Playlist" }
    );
    println!("  Snapshot: {:?}", snapshot_path);
    println!("  Journal: {:?}", journal_path);

    working_playlist::save(grit_dir, &id)?;

    Ok(())
}
