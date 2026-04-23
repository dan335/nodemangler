//! Video encoder wrapper.
//!
//! [`VideoEncoder`] wraps [`video_rs::encode::Encoder`] with a simpler API:
//! `open` → repeated `push_frame` → `finalize`. Each `VideoType` variant maps
//! to an H.264 yuv420p preset; the container is chosen by file extension.
//!
//! Frame conversion takes a 1–4 channel [`FloatImage`] and produces an RGB24
//! `Array3<u8>` frame suitable for `Encoder::encode`. Conversion and encoding
//! both run via `tokio::task::spawn_blocking` since `video-rs` is synchronous.

use std::path::{Path, PathBuf};

use ndarray::Array3;
use video_rs::encode::{Encoder, Settings};
use video_rs::time::Time;

use crate::float_image::FloatImage;
use crate::value::VideoType;
use crate::video::decoder_cache::VideoError;

/// Encode frames produced by the graph into a video file.
///
/// Hold-once, push-many: `open` configures the encoder, `push_frame` is
/// called once per rendered frame in order, and `finalize` flushes and closes
/// the output.
pub struct VideoEncoder {
    /// Held in `Option` so `finalize` can move it out into the blocking task.
    inner: Option<Encoder>,
    path: PathBuf,
    fps: f32,
    frame_count: u32,
    /// Dimensions the encoder was opened with; frames that don't match are
    /// resized (with a single warning log).
    expected_size: (u32, u32),
    warned_size_mismatch: bool,
}

impl VideoEncoder {
    /// Open a new encoder writing to `path` with the given dimensions, fps,
    /// and container type. Uses h264 yuv420p for all `VideoType` variants;
    /// the container is selected by the file extension on `path` (which the
    /// caller should set from `format.extension()`).
    pub fn open(
        path: &Path,
        width: u32,
        height: u32,
        fps: f32,
        _format: VideoType,
    ) -> Result<Self, VideoError> {
        crate::video::ensure_init();
        let settings = Settings::preset_h264_yuv420p(width as usize, height as usize, false);
        let encoder = Encoder::new(path.to_path_buf(), settings).map_err(VideoError::from)?;
        Ok(Self {
            inner: Some(encoder),
            path: path.to_path_buf(),
            fps: fps.max(1.0),
            frame_count: 0,
            expected_size: (width, height),
            warned_size_mismatch: false,
        })
    }

    /// Push a single frame to the encoder. Runs the actual encode on a
    /// blocking thread so the async runtime isn't stalled.
    pub async fn push_frame(&mut self, img: &FloatImage) -> Result<(), VideoError> {
        let (ew, eh) = self.expected_size;
        if img.width() != ew || img.height() != eh {
            if !self.warned_size_mismatch {
                eprintln!(
                    "video encoder: frame size {}x{} does not match output size {}x{} — resizing",
                    img.width(),
                    img.height(),
                    ew,
                    eh
                );
                self.warned_size_mismatch = true;
            }
        }

        // Resample to the expected size if necessary. Cheap when sizes match.
        let resized = if img.width() == ew && img.height() == eh {
            img.clone()
        } else {
            img.resize(ew, eh)
        };

        let frame_array = float_image_to_rgb24(&resized);
        let timestamp = Time::from_secs_f64(self.frame_count as f64 / self.fps as f64);

        // Move the encoder out, encode, move back. This lets us use
        // spawn_blocking without holding a mutable borrow across await.
        let mut encoder = self
            .inner
            .take()
            .ok_or_else(|| VideoError("encoder already finalized".to_string()))?;
        let encoder_and_frame = tokio::task::spawn_blocking(move || -> Result<Encoder, VideoError> {
            encoder.encode(&frame_array, timestamp).map_err(VideoError::from)?;
            Ok(encoder)
        })
        .await
        .map_err(|e| VideoError(format!("encode task join error: {e}")))?;
        self.inner = Some(encoder_and_frame?);
        self.frame_count += 1;
        Ok(())
    }

    /// Flush remaining packets, write the container trailer, and return the
    /// output path. Consumes `self` so further pushes are impossible.
    pub async fn finalize(mut self) -> Result<PathBuf, VideoError> {
        let mut encoder = self
            .inner
            .take()
            .ok_or_else(|| VideoError("encoder already finalized".to_string()))?;
        tokio::task::spawn_blocking(move || -> Result<(), VideoError> {
            encoder.finish().map_err(VideoError::from)?;
            Ok(())
        })
        .await
        .map_err(|e| VideoError(format!("finalize task join error: {e}")))??;
        Ok(self.path.clone())
    }
}

/// Convert a 1–4 channel `FloatImage` to an `Array3<u8>` with shape
/// `(height, width, 3)` in RGB24 order.
///
/// Channel handling:
/// - 1ch (grayscale): replicate into R, G, B.
/// - 2ch (grayscale + alpha): replicate gray; alpha discarded.
/// - 3ch (RGB): copy directly.
/// - 4ch (RGBA): copy RGB; alpha discarded.
fn float_image_to_rgb24(img: &FloatImage) -> Array3<u8> {
    use rayon::prelude::*;

    let w = img.width() as usize;
    let h = img.height() as usize;
    let c = img.channels() as usize;
    let src = img.as_raw();

    // Allocate the output ndarray. We fill row by row so that the layout is
    // contiguous (C-order) which is what video-rs expects.
    let mut out = Array3::<u8>::zeros((h, w, 3));
    // Flat view so we can pipe through rayon cleanly.
    let out_slice = out
        .as_slice_mut()
        .expect("Array3 zeros returns contiguous storage");

    out_slice
        .par_chunks_mut(3)
        .enumerate()
        .for_each(|(pixel_idx, dst)| {
            let src_off = pixel_idx * c;
            let (r, g, b) = match c {
                1 => {
                    let v = src[src_off];
                    (v, v, v)
                }
                2 => {
                    let v = src[src_off];
                    (v, v, v)
                }
                3 | 4 => (src[src_off], src[src_off + 1], src[src_off + 2]),
                _ => (0.0, 0.0, 0.0),
            };
            dst[0] = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
            dst[1] = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
            dst[2] = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
        });

    out
}
