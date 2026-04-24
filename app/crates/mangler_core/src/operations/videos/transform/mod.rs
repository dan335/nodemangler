//! Video transform operations.
//!
//! * extract-frame ops consume a `Value::Video` handle and emit a `Value::Image`.
//! * `trim` / `speed` / `reverse` / `loop_video` transform the `VideoRef`
//!   itself (metadata only — no decode) and forward a new handle.

pub mod extract_frame_by_index;
pub mod extract_frame_by_time;
pub mod trim;
pub mod speed;
pub mod reverse;
pub mod loop_video;

#[cfg(feature = "video")]
pub(super) mod decode_helper {
    use crate::float_image::FloatImage;
    use crate::value::VideoRef;
    use crate::video::{VideoDecoderCache, VideoError};
    use std::sync::Arc;

    /// Decode the source frame at `source_index` from the video file backing
    /// `video`. The caller is responsible for mapping effective-timeline
    /// queries to a source frame index via [`VideoRef::source_frame_for_effective_time`]
    /// or [`VideoRef::source_frame_for_effective_frame`]. Clamps to
    /// `[0, source_total_frames - 1]` as a final safety net.
    pub async fn decode_source_frame(
        video: &VideoRef,
        source_index: u32,
    ) -> Result<Arc<FloatImage>, VideoError> {
        let idx = source_index.min(video.source_meta.total_frames.saturating_sub(1));
        VideoDecoderCache::global().frame(&video.path, idx).await
    }
}
