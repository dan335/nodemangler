//! Video decode and encode support for the node graph engine.
//!
//! This module is gated behind the `video` cargo feature. Enabling it adds
//! dependencies on `video-rs` (and transitively FFmpeg development libraries).
//! See `docs/video-setup.md` for install instructions.

#[cfg(feature = "video")]
pub mod decoder_cache;
#[cfg(feature = "video")]
pub mod encoder;

#[cfg(feature = "video")]
pub use crate::value::VideoMeta;
#[cfg(feature = "video")]
pub use decoder_cache::{VideoDecoderCache, VideoError};
#[cfg(feature = "video")]
pub use encoder::VideoEncoder;

/// Initialise `video-rs` once. Safe to call many times. Called implicitly
/// by the decoder cache and the encoder, so callers normally don't need to.
#[cfg(feature = "video")]
pub fn ensure_init() {
    use std::sync::OnceLock;
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // `video_rs::init` initialises ffmpeg logging / network subsystems.
        // Returns Err only if already initialised — ignore.
        let _ = video_rs::init();
    });
}
