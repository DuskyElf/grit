use std::io::{self, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::cli::commands::utils::create_provider;
use crate::playback::{MpvPlayer, Queue};
use crate::state::snapshot;

pub async fn run(playlist: Option<&str>, shuffle: bool, grit_dir: &Path) -> Result<()> {
    let playlist_id = playlist.context("Playlist required (use --playlist or -l)")?;

    let snapshot_path = snapshot::snapshot_path(grit_dir, playlist_id);
    if !snapshot_path.exists() {
        bail!("Playlist not tracked. Run 'grit init <playlist>' first.");
    }

    let snap = snapshot::load(&snapshot_path)?;
    if snap.tracks.is_empty() {
        bail!("Playlist is empty");
    }

    println!("Playing: {} ({} tracks)", snap.name, snap.tracks.len());

    let provider = create_provider(snap.provider, grit_dir)?;

    let mut queue = Queue::new(snap.tracks.clone());
    if shuffle {
        queue.toggle_shuffle();
        println!("Shuffle: ON");
    }

    let mut player = MpvPlayer::spawn().await?;

    if let Some(track) = queue.current_track() {
        let url = provider.playable_url(track).await?;
        println!("\n▶ {} - {}", track.name, track.artists.join(", "));
        player.load(&url).await?;
    }

    println!("\nControls: [space] pause  [n] next  [p] prev  [s] shuffle  [q] quit");

    let mut is_paused = false;
    enable_raw_mode()?;

    loop {
        // Check for keyboard input (non-blocking)
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(' ') => {
                        is_paused = !is_paused;
                        if is_paused {
                            player.pause().await?;
                        } else {
                            player.resume().await?;
                        }
                    }
                    KeyCode::Char('n') => {
                        if let Some(track) = queue.next() {
                            let url = provider.playable_url(track).await?;
                            print!(
                                "\r▶ {} - {}                    ",
                                track.name,
                                track.artists.join(", ")
                            );
                            io::stdout().flush()?;
                            player.load(&url).await?;
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Some(track) = queue.previous() {
                            let url = provider.playable_url(track).await?;
                            print!(
                                "\r▶ {} - {}                    ",
                                track.name,
                                track.artists.join(", ")
                            );
                            io::stdout().flush()?;
                            player.load(&url).await?;
                        }
                    }
                    KeyCode::Char('s') => {
                        queue.toggle_shuffle();
                    }
                    _ => {}
                }
            }
        }

        // Check for mpv events (track ended)
        if let Some(event) = player.try_recv_event() {
            if MpvPlayer::is_track_finished(&event) {
                // Auto-advance to next track
                if let Some(track) = queue.next() {
                    let url = provider.playable_url(track).await?;
                    print!(
                        "\r▶ {} - {}                    ",
                        track.name,
                        track.artists.join(", ")
                    );
                    io::stdout().flush()?;
                    player.load(&url).await?;
                } else {
                    println!("\nPlaylist finished");
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    player.quit().await?;

    Ok(())
}
