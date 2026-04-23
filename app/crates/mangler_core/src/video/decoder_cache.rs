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

/// Number of recently decoded frames kept per file.
const RING_CAPACITY: usize = 8;

/// If the target frame is more than this many frames past `last_decoded_index`,
/// or is before it, fall through to a `seek_to_frame` call rather than
/// decoding forward.
const SEEK_THRESHOLD: u32 = 30;

/// Immutable metadata extracted from a video file on first open.
#[derive(Debug, Clone, Copy)]
pub struct VideoMeta {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub duration_seconds: f64,
    pub total_frames: u32,
}

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
        let meta = VideoMeta {
            width,
            height,
            fps,
            duration_seconds,
            total_frames,
        };
        let entry = Self {
            decoder,
            ring: VecDeque::with_capacity(RING_CAPACITY),
            last_decoded_index: None,
        };
        Ok((meta, entry))
    }

    fn push_ring(&mut self, index: u32, frame: Arc<FloatImage>) {
        if self.ring.len() == RING_CAPACITY {
            self.ring.pop_front();
        }
        self.ring.push_back((index, frame));
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
            self.ring.clear();
            self.last_decoded_index = None;
        }

        // Decode forward until a frame with PTS-index >= frame_index arrives.
        // Protect against infinite loops in pathological streams: bound the
        // number of decodes to a generous multiple of total_frames.
        let max_decodes = (meta.total_frames as usize).saturating_mul(2).max(256);
        for _ in 0..max_decodes {
            let (pts, mut raw) = self.decoder.decode().map_err(VideoError::from)?;
            let idx = pts_to_frame_index(&pts, meta.fps, meta.total_frames);
            let float = convert_frame_to_float_image(&mut raw)?;
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
fn convert_frame_to_float_image(frame: &mut video_rs::Frame) -> Result<FloatImage, VideoError> {
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
    let mut data = vec![0.0f32; pixel_count * 4];
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
