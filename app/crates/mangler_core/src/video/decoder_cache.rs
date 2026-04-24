//! Cached video frame decoder.
//!
//! [`VideoDecoderCache`] is a process-global cache keyed on file path. Each
//! entry owns a [`video_rs::decode::Decoder`], the clip's [`VideoMeta`], and
//! a small ring buffer of recently decoded frames to make forward scrubbing
//! cheap. Backward seeks or large forward jumps fall through to a
//! `seek_to_frame` call.
//!
//! All decode work runs on `tokio::task::spawn_blocking` because `video-rs`
//! is synchronous.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::float_image::FloatImage;
use crate::value::{VideoCodec, VideoContainer, VideoMeta};

#[cfg(test)]
#[path = "decoder_cache_tests.rs"]
mod tests;

/// Number of recently decoded frames kept per file.
///
/// Sized for interactive scrubbing: while dragging `current_frame` back and
/// forth, small excursions should hit cache instead of re-decoding. At
/// 1080p RGBA f32 that's ~32 MB per frame × 64 = ~2 GB worst case per open
/// clip; acceptable for one clip open at a time, re-evaluate if users open
/// many clips simultaneously.
const RING_CAPACITY: usize = 64;

/// If the target frame is more than this many frames past `last_decoded_index`,
/// or is before it, fall through to a `seek_to_frame` call rather than
/// decoding forward.
///
/// H.264 GOPs are commonly 30-120 frames between keyframes. Setting this
/// higher than typical GOP length means forward-decoding (within GOP) is
/// preferred over seeking (which always lands at or before a keyframe and
/// re-decodes to the target anyway).
const SEEK_THRESHOLD: u32 = 60;

// `VideoMeta` moved to `value.rs` so it can be carried inside `Value::Video`
// without gating that variant on the `video` feature. Re-exported via
// `video/mod.rs` for call sites that previously imported it from here.

/// Errors returned by the decoder cache.
#[derive(Debug, Clone)]
pub struct VideoError(pub String);

impl std::fmt::Display for VideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for VideoError {}

impl From<video_rs::Error> for VideoError {
    fn from(e: video_rs::Error) -> Self {
        VideoError(format!("video-rs: {e}"))
    }
}

/// Per-file decoder state. Held behind a `Mutex` so concurrent calls against
/// the same file serialize; different files decode in parallel.
///
/// Meta is cached separately on `VideoCacheEntry` so `meta_blocking` can
/// return it without touching the mutex (which matters because
/// `tokio::sync::Mutex::blocking_lock` panics if called from inside the
/// async runtime).
struct VideoEntry {
    decoder: video_rs::decode::Decoder,
    ring: VecDeque<(u32, Arc<FloatImage>)>,
    last_decoded_index: Option<u32>,
    /// Reclaimed pixel buffer from an evicted ring entry — handed back to
    /// `convert_frame_to_float_image` so successive decodes on the same clip
    /// avoid allocating a fresh `Vec<f32>` (~33 MB at 4K) every time.
    /// `None` after the buffer has been taken by an in-flight decode;
    /// `Some` after a ring eviction where we were the only holder of the
    /// outgoing Arc.
    scratch_buf: Option<Vec<f32>>,
}

/// Public-facing cache entry: the immutable meta is duplicated out here so it
/// can be read without grabbing the decoder mutex.
struct VideoCacheEntry {
    meta: VideoMeta,
    state: Arc<Mutex<VideoEntry>>,
}

impl Clone for VideoCacheEntry {
    fn clone(&self) -> Self {
        Self {
            meta: self.meta,
            state: Arc::clone(&self.state),
        }
    }
}

impl VideoEntry {
    fn open(path: &Path) -> Result<(VideoMeta, Self), VideoError> {
        crate::video::ensure_init();
        let decoder = video_rs::decode::Decoder::new(path.to_path_buf())
            .map_err(VideoError::from)?;
        let (width, height) = decoder.size_out();
        let fps = decoder.frame_rate();
        // `frames()` can return 0 on streams that don't store a count in their
        // metadata. Fall back to duration * fps for a reasonable estimate.
        let frames_reported = decoder.frames().unwrap_or(0) as u32;
        let duration_seconds = decoder
            .duration()
            .ok()
            .map(|t| t.as_secs_f64())
            .unwrap_or(0.0);
        let total_frames = if frames_reported > 0 {
            frames_reported
        } else if fps > 0.0 && duration_seconds > 0.0 {
            (duration_seconds * fps as f64).round().max(1.0) as u32
        } else {
            1
        };

        // Read container + codec via a parallel ffmpeg input context. Cheap —
        // header parse only, dropped after metadata extraction. video-rs owns
        // its Reader privately so we can't borrow it for this.
        let (container, codec) = read_container_and_codec(path)?;

        let meta = VideoMeta {
            width,
            height,
            fps,
            duration_seconds,
            total_frames,
            container,
            codec,
        };
        let entry = Self {
            decoder,
            ring: VecDeque::with_capacity(RING_CAPACITY),
            last_decoded_index: None,
            scratch_buf: None,
        };
        Ok((meta, entry))
    }

    fn push_ring(&mut self, index: u32, frame: Arc<FloatImage>) {
        if self.ring.len() == RING_CAPACITY {
            if let Some((_, evicted)) = self.ring.pop_front() {
                // If we're the only holder, reclaim the pixel buffer for the
                // next decode. Fails (drops) when a downstream node is still
                // holding the Arc — common during active graph runs. That's
                // fine; we just allocate a fresh Vec in that case and try
                // again on the next eviction.
                self.try_reclaim_scratch(evicted);
            }
        }
        self.ring.push_back((index, frame));
    }

    /// Evict helper: reclaim the Vec<f32> from `frame` into `scratch_buf`
    /// if we're the only holder. Never overwrites an existing scratch (first
    /// reclaim wins until it's consumed).
    fn try_reclaim_scratch(&mut self, frame: Arc<FloatImage>) {
        if self.scratch_buf.is_some() {
            return;
        }
        if let Ok(float_image) = Arc::try_unwrap(frame) {
            self.scratch_buf = Some(float_image.into_data());
        }
    }

    fn find_ring(&self, index: u32) -> Option<Arc<FloatImage>> {
        self.ring
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, f)| f.clone())
    }

    /// Decode the frame at `frame_index`. Seeks if the target is far from the
    /// last decoded position; otherwise decodes forward through the stream.
    ///
    /// The frame index for each decoded frame is derived from the frame's PTS
    /// (time stamp) times fps, not a manual counter. `seek_to_frame` lands on
    /// the nearest keyframe at or before the target, so after a seek the next
    /// decoded frame may be several frames earlier than requested — we keep
    /// decoding forward until the PTS-derived index reaches the target.
    fn get_frame(&mut self, meta: VideoMeta, frame_index: u32) -> Result<Arc<FloatImage>, VideoError> {
        let frame_index = frame_index.min(meta.total_frames.saturating_sub(1));

        if let Some(hit) = self.find_ring(frame_index) {
            return Ok(hit);
        }

        let needs_seek = match self.last_decoded_index {
            Some(last) => frame_index < last || frame_index.saturating_sub(last) > SEEK_THRESHOLD,
            None => frame_index > 0,
        };

        if needs_seek {
            self.decoder
                .seek_to_frame(frame_index as i64)
                .map_err(VideoError::from)?;
            // Drain the ring, trying to reclaim the oldest buffer on the way.
            // First uniquely-held one wins; the rest drop normally.
            while let Some((_, evicted)) = self.ring.pop_front() {
                self.try_reclaim_scratch(evicted);
            }
            self.last_decoded_index = None;
        }

        // Decode forward until a frame with PTS-index >= frame_index arrives.
        // Protect against infinite loops in pathological streams: bound the
        // number of decodes to a generous multiple of total_frames.
        let max_decodes = (meta.total_frames as usize).saturating_mul(2).max(256);
        for _ in 0..max_decodes {
            let (pts, mut raw) = self.decoder.decode().map_err(VideoError::from)?;
            let idx = pts_to_frame_index(&pts, meta.fps, meta.total_frames);
            let float = convert_frame_to_float_image(&mut raw, self.scratch_buf.take())?;
            let arc = Arc::new(float);
            self.push_ring(idx, arc.clone());
            self.last_decoded_index = Some(idx);

            if idx >= frame_index {
                return Ok(arc);
            }
        }

        Err(VideoError(format!(
            "decode loop exceeded max iterations while seeking to frame {}",
            frame_index
        )))
    }
}

/// Map a frame's timestamp to a frame index using the clip's fps. Clamps to
/// `[0, total_frames - 1]` so a PTS just beyond the last frame doesn't
/// overflow the ring-buffer key space.
fn pts_to_frame_index(pts: &video_rs::Time, fps: f32, total_frames: u32) -> u32 {
    let seconds = pts.as_secs_f64();
    let raw = (seconds * fps as f64).round() as i64;
    raw.clamp(0, total_frames.saturating_sub(1) as i64) as u32
}

/// Convert an `ndarray::Array3<u8>` RGB24 frame into a 4-channel `FloatImage`.
///
/// Divides by 255 to get an sRGB float, and sets alpha to 1.0. Uses rayon for
/// the per-pixel loop on large frames.
///
/// `scratch` is an optional reclaimed pixel buffer from an evicted ring
/// entry. If its capacity fits the needed size we skip the allocation
/// entirely; otherwise (or when `None`) we allocate fresh. Every decoded
/// frame's `Vec<f32>` is ~33 MB at 4K — reuse across a clip's worth of
/// frames is a meaningful chunk of allocator pressure removed.
fn convert_frame_to_float_image(
    frame: &mut video_rs::Frame,
    scratch: Option<Vec<f32>>,
) -> Result<FloatImage, VideoError> {
    use rayon::prelude::*;

    let (h, w, c) = {
        let shape = frame.shape();
        if shape.len() != 3 || shape[2] != 3 {
            return Err(VideoError(format!(
                "unexpected frame shape {:?}, expected [h, w, 3]",
                shape
            )));
        }
        (shape[0] as u32, shape[1] as u32, 3u32)
    };

    let slice = frame
        .as_slice()
        .ok_or_else(|| VideoError("frame was not contiguous".into()))?;

    let pixel_count = (w * h) as usize;
    let needed = pixel_count * 4;

    // Reuse the scratch buffer when big enough; fall back to fresh alloc.
    // par_chunks_mut below overwrites every element, so a memset to zero
    // here would be wasted — `resize` with 1.0 (the alpha default) is cheap
    // and keeps the vec initialized in case any future consumer reads
    // without overwriting.
    let mut data = match scratch {
        Some(mut buf) if buf.capacity() >= needed => {
            buf.clear();
            buf.resize(needed, 0.0);
            buf
        }
        _ => vec![0.0f32; needed],
    };

    data.par_chunks_mut(4).enumerate().for_each(|(i, out)| {
        let src = i * c as usize;
        out[0] = slice[src] as f32 / 255.0;
        out[1] = slice[src + 1] as f32 / 255.0;
        out[2] = slice[src + 2] as f32 / 255.0;
        out[3] = 1.0;
    });

    FloatImage::from_raw(w, h, 4, data)
        .ok_or_else(|| VideoError("from_raw length mismatch".into()))
}

/// Open a short-lived ffmpeg input context, read container + best-video-stream
/// codec, drop it. Errors with a user-facing message if either is outside our
/// supported enum variants.
fn read_container_and_codec(path: &Path) -> Result<(VideoContainer, VideoCodec), VideoError> {
    use ffmpeg_next as ffmpeg;
    let input = ffmpeg::format::input(&path.to_path_buf()).map_err(|e| {
        VideoError(format!("ffmpeg could not open {}: {}", path.display(), e))
    })?;
    let format_name = input.format().name().to_string();
    let container = classify_container(&format_name, path)?;

    let stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or_else(|| VideoError(format!("no video stream in {}", path.display())))?;
    let codec_id = stream.parameters().id();
    let codec = classify_codec(codec_id)?;

    Ok((container, codec))
}

/// Map an FFmpeg demuxer short-name (comma-separated, e.g. `"mov,mp4,m4a,3gp,3g2,mj2"`)
/// to a [`VideoContainer`].
///
/// FFmpeg groups structurally-similar containers under a single demuxer, so the
/// returned name can't always disambiguate MP4 from MOV. When the path's
/// extension matches one of the name tokens, we use the extension as a
/// tiebreak *within the set ffmpeg already confirmed*. Otherwise we pick the
/// first token that matches a known variant.
fn classify_container(format_name: &str, path: &Path) -> Result<VideoContainer, VideoError> {
    let tokens: Vec<&str> = format_name.split(',').map(str::trim).collect();

    // Extension-as-tiebreak within ffmpeg's reported family.
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lc = ext.to_ascii_lowercase();
        if tokens.iter().any(|t| t.eq_ignore_ascii_case(&ext_lc)) {
            if let Some(c) = match_container_token(&ext_lc) {
                return Ok(c);
            }
        }
    }

    for token in &tokens {
        if let Some(c) = match_container_token(token) {
            return Ok(c);
        }
    }

    Err(VideoError(format!(
        "Unsupported container format '{}' (mangler supports: mp4, mov, mkv, webm, avi).",
        format_name
    )))
}

/// Matches a single short-name token against known containers. Case-insensitive.
/// Returns `None` for unrecognized tokens (caller decides whether that's an error).
fn match_container_token(token: &str) -> Option<VideoContainer> {
    match token.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => Some(VideoContainer::Mp4),
        "mov" => Some(VideoContainer::Mov),
        "matroska" | "mkv" => Some(VideoContainer::Mkv),
        "webm" => Some(VideoContainer::WebM),
        "avi" => Some(VideoContainer::Avi),
        _ => None,
    }
}

/// Map an FFmpeg `codec::Id` to a [`VideoCodec`]. Unsupported codecs (Theora,
/// DNxHD, MJPEG, etc.) surface as an error rather than silently decoding.
fn classify_codec(id: ffmpeg_next::codec::Id) -> Result<VideoCodec, VideoError> {
    use ffmpeg_next::codec::Id;
    match id {
        Id::H264 => Ok(VideoCodec::H264),
        Id::HEVC | Id::H265 => Ok(VideoCodec::H265),
        Id::VP8 => Ok(VideoCodec::Vp8),
        Id::VP9 => Ok(VideoCodec::Vp9),
        Id::AV1 => Ok(VideoCodec::Av1),
        Id::MPEG4 => Ok(VideoCodec::Mpeg4),
        Id::PRORES => Ok(VideoCodec::ProRes),
        other => Err(VideoError(format!(
            "Unsupported codec '{:?}' (mangler supports: H264, H265, VP8, VP9, AV1, MPEG-4, ProRes).",
            other
        ))),
    }
}

/// Process-global cache of open video decoders.
pub struct VideoDecoderCache {
    entries: DashMap<PathBuf, VideoCacheEntry>,
}

impl VideoDecoderCache {
    /// The shared cache instance.
    pub fn global() -> &'static VideoDecoderCache {
        static INSTANCE: OnceLock<VideoDecoderCache> = OnceLock::new();
        INSTANCE.get_or_init(|| VideoDecoderCache {
            entries: DashMap::new(),
        })
    }

    /// Get or open the entry for a given path. Cheap after the first open.
    fn entry(&self, path: &Path) -> Result<VideoCacheEntry, VideoError> {
        if let Some(existing) = self.entries.get(path) {
            return Ok(existing.clone());
        }
        let (meta, entry) = VideoEntry::open(path)?;
        let cache_entry = VideoCacheEntry {
            meta,
            state: Arc::new(Mutex::new(entry)),
        };
        self.entries.insert(path.to_path_buf(), cache_entry.clone());
        Ok(cache_entry)
    }

    /// Fetch metadata for a video file. Synchronous and does not touch the
    /// decoder mutex — safe to call from anywhere, including inside an async
    /// runtime where `tokio::sync::Mutex::blocking_lock` would panic.
    pub fn meta_blocking(&self, path: &Path) -> Result<VideoMeta, VideoError> {
        let entry = self.entry(path)?;
        Ok(entry.meta)
    }

    /// Fetch metadata asynchronously. Equivalent to `meta_blocking`; the
    /// async flavour is here for API symmetry.
    pub async fn meta(&self, path: &Path) -> Result<VideoMeta, VideoError> {
        let entry = self.entry(path)?;
        Ok(entry.meta)
    }

    /// Fetch the frame at `frame_index` for the given path.
    ///
    /// Runs the blocking decode work on a tokio blocking thread so the async
    /// runtime isn't stalled.
    pub async fn frame(
        &self,
        path: &Path,
        frame_index: u32,
    ) -> Result<Arc<FloatImage>, VideoError> {
        let entry = self.entry(path)?;
        let state = entry.state.clone();
        let meta = entry.meta;
        let result = tokio::task::spawn_blocking(move || {
            let mut guard = state.blocking_lock();
            guard.get_frame(meta, frame_index)
        })
        .await
        .map_err(|e| VideoError(format!("decode task join error: {e}")))?;
        result
    }
}
