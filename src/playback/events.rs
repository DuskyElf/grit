use crate::provider::Track;

#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    Play(Track),
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    Seek(i64),  //negative for revwind
    Volume(u8), //0-100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    None,
    One,
    All,
}

#[derive(Debug, Clone)]
pub enum PlaybackState {
    Playing { track: Track, position_secs: f64 },
    Paused { track: Track, position_secs: f64 },
    Stopped,
    Loading,
}
