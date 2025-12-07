pub mod spotify;
pub mod youtube;
mod traits;
mod types;

pub use spotify::SpotifyProvider;
pub use youtube::YoutubeProvider;
pub use traits::Provider;
pub use types::*;
