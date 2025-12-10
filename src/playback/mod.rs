pub mod events;
pub mod lyrics;
pub mod mpv;
pub mod queue;
pub mod spotify;

pub use lyrics::{fetch_lyrics, Lyrics};
pub use mpv::{fetch_audio_url, MpvPlayer};
pub use queue::Queue;
pub use spotify::SpotifyPlayer;
