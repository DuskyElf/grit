mod cli;
mod playback;
mod provider;
mod state;
mod tui;
mod utils;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Commands};
use provider::ProviderKind;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present (ignores if missing)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let grit_dir = PathBuf::from(".grit");

    match cli.command {
        Commands::Auth { provider } => {
            cli::commands::auth::run(provider, &grit_dir).await?;
        }
        Commands::Init { playlist, provider } => {
            let provider = provider
                .or(cli.provider)
                .or_else(|| cli::commands::init::detect_provider(&playlist))
                .unwrap_or(ProviderKind::Spotify);
            cli::commands::init::run(provider, &playlist, &grit_dir).await?;
        }
        Commands::Search { query } => {
            cli::commands::staging::search(&query, cli.provider, &grit_dir).await?;
        }
        Commands::Add { track_id } => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::staging::add(&track_id, Some(&playlist), &grit_dir).await?;
        }
        Commands::Remove { track_id } => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::staging::remove(&track_id, Some(&playlist), &grit_dir).await?;
        }
        Commands::Move {
            track_id,
            new_index,
        } => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::staging::move_track(&track_id, new_index, Some(&playlist), &grit_dir)
                .await?;
        }
        Commands::Status { playlist } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::staging::status(Some(&playlist), &grit_dir).await?;
        }
        Commands::Reset { playlist } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::staging::reset(Some(&playlist), &grit_dir).await?;
        }
        Commands::List { playlist } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::misc::list(Some(&playlist), &grit_dir).await?;
        }
        Commands::Find { query, playlist } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::misc::find(&query, Some(&playlist), &grit_dir).await?;
        }
        Commands::Logout { provider } => {
            cli::commands::auth::logout(provider, &grit_dir).await?;
        }
        Commands::Whoami { provider } => {
            cli::commands::auth::whoami(provider, &grit_dir).await?;
        }
        Commands::Commit { message } => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::staging::commit(&message, Some(&playlist), &grit_dir).await?;
        }
        Commands::Push { playlist } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::vcs::push(Some(&playlist), &grit_dir).await?;
        }
        Commands::Log => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::vcs::log(Some(&playlist), &grit_dir).await?;
        }
        Commands::Pull => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::vcs::pull(Some(&playlist), &grit_dir).await?;
        }
        Commands::Diff { staged, remote } => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::vcs::diff_cmd(Some(&playlist), &grit_dir, staged, remote).await?;
        }
        Commands::Playlists { query } => {
            cli::commands::misc::playlists(query.as_deref(), &grit_dir).await?;
        }
        Commands::Switch { playlist } => {
            cli::commands::misc::switch(&playlist, &grit_dir).await?;
        }
        Commands::Revert { hash, playlist } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::vcs::revert(hash.as_deref(), Some(&playlist), &grit_dir).await?;
        }
        Commands::Apply { file } => {
            let playlist = resolve_playlist(None, cli.playlist.clone(), &grit_dir)?;
            cli::commands::vcs::apply(&file, Some(&playlist), &grit_dir).await?;
        }
        Commands::Play { playlist, shuffle } => {
            let playlist = resolve_playlist(playlist, cli.playlist.clone(), &grit_dir)?;
            cli::commands::play::run(Some(&playlist), shuffle, &grit_dir).await?;
        }
    }

    Ok(())
}

/// Resolves the playlist ID to use based on command-line argument,
/// global option, or working playlist in config.
fn resolve_playlist(
    command_playlist: Option<String>,
    global_playlist: Option<String>,
    grit_dir: &Path,
) -> anyhow::Result<String> {
    command_playlist
        .or(global_playlist)
        .or_else(|| crate::state::working_playlist::load(grit_dir).ok())
        .context("Playlist required (use --playlist, 'grit switch <id>', or run 'grit init' to set working playlist)")
}
