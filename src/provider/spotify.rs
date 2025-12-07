use crate::provider::{
    DiffPatch, OAuthToken, PlaylistSnapshot, Provider, ProviderKind, Track, TrackChange,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API_BASE: &str = "https://api.spotify.com/v1";

pub struct SpotifyProvider {
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct SpotifyTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct SpotifyPlaylist {
    id: String,
    name: String,
    description: Option<String>,
    snapshot_id: String,
    tracks: SpotifyTracks,
}

#[derive(Deserialize)]
struct SpotifyTracks {
    items: Vec<SpotifyTrackItem>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct SpotifyTrackItem {
    track: Option<SpotifyTrackObject>,
}

#[derive(Deserialize)]
struct SpotifyTrackObject {
    id: String,
    name: String,
    duration_ms: u64,
    artists: Vec<SpotifyArtist>,
}

#[derive(Deserialize)]
struct SpotifyArtist {
    name: String,
}

#[derive(Deserialize)]
struct SpotifySearchResponse {
    tracks: SpotifySearchTracks,
}

#[derive(Deserialize)]
struct SpotifySearchTracks {
    items: Vec<SpotifyTrackObject>,
}

impl SpotifyTokenResponse {
    fn into_oauth_token(self) -> OAuthToken {
        use std::time::{SystemTime, UNIX_EPOCH};

        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + self.expires_in;

        OAuthToken {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: Some(expires_at),
            token_type: self.token_type,
            scope: self.scope,
        }
    }
}

impl SpotifyProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            access_token: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn with_token(mut self, token: &OAuthToken) -> Self {
        self.access_token = Some(token.access_token.clone());
        self
    }

    fn get_token(&self) -> Result<&str> {
        self.access_token
            .as_deref()
            .context("Not authenticated with Spotify")
    }

    fn basic_auth_header(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        base64::engine::general_purpose::STANDARD.encode(credentials)
    }

    async fn token_request(&self, params: &[(&str, &str)]) -> Result<SpotifyTokenResponse> {
        let response = self
            .http
            .post(TOKEN_URL)
            .header(
                "Authorization",
                format!("Basic {}", self.basic_auth_header()),
            )
            .form(params)
            .send()
            .await
            .context("Failed to send token request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Token request failed: {}", error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse token response")
    }

    async fn api_get<T: serde::de::DeserializeOwned>(&self, url: &str, token: &str) -> Result<T> {
        let response = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to send API request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Spotify API error {}: {}", status, error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse API response")
    }
}

#[async_trait]
impl Provider for SpotifyProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Spotify
    }

    fn oauth_url(&self, redirect_uri: &str, state: &str) -> String {
        let scopes = [
            "playlist-read-private",
            "playlist-read-collaborative",
            "playlist-modify-public",
            "playlist-modify-private",
        ]
        .join(" ");

        format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
            AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state),
        )
    }

    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<OAuthToken> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        self.token_request(&params)
            .await
            .map(|r| r.into_oauth_token())
    }

    async fn refresh_token(&self, token: &OAuthToken) -> Result<OAuthToken> {
        let refresh = token
            .refresh_token
            .as_ref()
            .context("No refresh token available")?;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
        ];

        let mut new_token = self.token_request(&params).await?.into_oauth_token();

        // Spotify doesn't always return a new refresh_token
        if new_token.refresh_token.is_none() {
            new_token.refresh_token = token.refresh_token.clone();
        }

        Ok(new_token)
    }

    async fn fetch(&self, playlist_id: &str) -> Result<PlaylistSnapshot> {
        let token = self.get_token()?;
        let url = format!("{}/playlists/{}", API_BASE, playlist_id);

        let playlist: SpotifyPlaylist = self.api_get(&url, token).await?;

        let mut all_tracks = Vec::new();

        for item in playlist.tracks.items {
            if let Some(track) = item.track {
                all_tracks.push(Track {
                    id: track.id,
                    name: track.name,
                    artists: track.artists.into_iter().map(|a| a.name).collect(),
                    duration_ms: track.duration_ms,
                    provider: ProviderKind::Spotify,
                    metadata: None,
                });
            }
        }

        let mut next_url = playlist.tracks.next;
        while let Some(url) = next_url {
            let page: SpotifyTracks = self.api_get(&url, token).await?;

            for item in page.items {
                if let Some(track) = item.track {
                    all_tracks.push(Track {
                        id: track.id,
                        name: track.name,
                        artists: track.artists.into_iter().map(|a| a.name).collect(),
                        duration_ms: track.duration_ms,
                        provider: ProviderKind::Spotify,
                        metadata: None,
                    });
                }
            }

            next_url = page.next;
        }

        Ok(PlaylistSnapshot {
            id: playlist.id,
            name: playlist.name,
            description: playlist.description,
            tracks: all_tracks,
            provider: ProviderKind::Spotify,
            snapshot_hash: playlist.snapshot_id,
            metadata: None,
        })
    }

    async fn apply(&self, playlist_id: &str, patch: &DiffPatch) -> Result<()> {
        let token = self.get_token()?;

        // Process in order: removals, additions, then moves
        // (Processing removals first prevents index shifting issues)

        for change in &patch.changes {
            if let TrackChange::Removed { track, .. } = change {
                let uri = format!("spotify:track:{}", track.id);
                let body = serde_json::json!({
                    "tracks": [{"uri": uri}]
                });

                let url = format!("{}/playlists/{}/tracks", API_BASE, playlist_id);

                self.http
                    .delete(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&body)
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }

        for change in &patch.changes {
            if let TrackChange::Added { track, index } = change {
                let uri = format!("spotify:track:{}", track.id);
                let body = serde_json::json!({
                    "uris": [uri],
                    "position": index
                });

                self.http
                    .post(format!("{}/playlists/{}/tracks", API_BASE, playlist_id))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&body)
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }

        for change in &patch.changes {
            if let TrackChange::Moved { from, to, .. } = change {
                // Spotify's reorder API uses insert_before semantics:
                // - When moving forward (from < to): insert_before = to + 1 (account for removal)
                // - When moving backward (from > to): insert_before = to
                let insert_before = if from < to { to + 1 } else { *to };

                let body = serde_json::json!({
                    "range_start": from,
                    "insert_before": insert_before,
                    "range_length": 1
                });

                self.http
                    .put(format!("{}/playlists/{}/tracks", API_BASE, playlist_id))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&body)
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }

        Ok(())
    }

    async fn playable_url(&self, track: &Track) -> Result<String> {
        // Spotify URI format for librespot
        Ok(format!("spotify:track:{}", track.id))
    }

    async fn search_by_query(&self, query: &str) -> Result<Vec<Track>> {
        let token = self.get_token()?;
        let url = format!(
            "{}/search?q={}&type=track&limit=10",
            API_BASE,
            urlencoding::encode(query)
        );

        let resp: SpotifySearchResponse = self.api_get(&url, token).await?;

        let tracks = resp
            .tracks
            .items
            .into_iter()
            .map(|track| Track {
                id: track.id,
                name: track.name,
                artists: track.artists.into_iter().map(|a| a.name).collect(),
                duration_ms: track.duration_ms,
                provider: ProviderKind::Spotify,
                metadata: None,
            })
            .collect();

        Ok(tracks)
    }
}
