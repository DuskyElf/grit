use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time_secs: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub plain: Option<String>,
}

#[derive(Deserialize)]
struct LrcLibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

impl Lyrics {
    pub fn current_line_index(&self, position_secs: f64) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }

        let mut current = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if line.time_secs <= position_secs {
                current = i;
            } else {
                break;
            }
        }
        Some(current)
    }
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();

    for line in lrc.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('[') {
            continue;
        }

        if let Some(bracket_end) = line.find(']') {
            let timestamp = &line[1..bracket_end];
            let text = line[bracket_end + 1..].trim().to_string();

            if let Some(time_secs) = parse_timestamp(timestamp) {
                if !text.is_empty() {
                    lines.push(LyricLine { time_secs, text });
                }
            }
        }
    }

    lines.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
    lines
}

fn parse_timestamp(ts: &str) -> Option<f64> {
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let minutes: f64 = parts[0].parse().ok()?;
    let seconds: f64 = parts[1].parse().ok()?;

    Some(minutes * 60.0 + seconds)
}

pub async fn fetch_lyrics(track_name: &str, artist_name: &str, duration_secs: u64) -> Result<Lyrics> {
    let client = Client::new();

    let url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}&duration={}",
        urlencoding::encode(track_name),
        urlencoding::encode(artist_name),
        duration_secs
    );

    let response = client
        .get(&url)
        .header("User-Agent", "grit/1.0")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(Lyrics::default());
    }

    let data: LrcLibResponse = response.json().await?;

    let lines = data.synced_lyrics
        .as_ref()
        .map(|s| parse_lrc(s))
        .unwrap_or_default();

    Ok(Lyrics {
        lines,
        plain: data.plain_lyrics,
    })
}