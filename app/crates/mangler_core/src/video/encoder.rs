//! Video encoder wrapper.
//!
//! [`VideoEncoder`] wraps [`video_rs::encode::Encoder`] with a simpler API:
//! `open` → repeated `push_frame` → `finalize`.
//!
//! `open` takes an explicit `(VideoContainer, VideoCodec)` pair and validates
//! it against the static compatibility matrix before touching FFmpeg. Only a
//! subset of the matrix has encoder support wired up today (see match arm in
//! [`VideoEncoder::open`]); pairs that are legal but not-yet-implemented
//! surface as a distinct error so we can expand coverage independently.
//!
//! Frame conversion takes a 1–4 channel [`FloatImage`] and produces an RGB24
//! `Array3<u8>` frame suitable for `Encoder::encode`. Conversion and encoding
//! both run via `tokio::task::spawn_blocking` since `video-rs` is synchronous.

use std::path::{Path, PathBuf};

use ndarray::Array3;
use video_rs::encode::{Encoder, Settings};
use video_rs::time::Time;

use crate::float_image::FloatImage;
use crate::value::{VideoCodec, VideoContainer};
use crate::video::decoder_cache::VideoError;

#[cfg(test)]
#[path = "encoder_tests.rs"]
mod tests;

/// The concrete encoder preset chosen for a `(container, codec)` pair.
///
/// Single source of truth for which combos have encoder support wired up.
/// `encoder_preset_for` is the only way to obtain one; `to_settings` is the
/// only way to turn it into FFmpeg settings. Adding a new preset means adding
/// a variant here, an arm to `encoder_preset_for`, and an arm to
/// `to_settings`.
enum EncoderPreset {
    H264Yuv420p,
    // NOTE: video-rs 0.11 does not expose a VP9 preset; `WebM + Vp9` is not
    // yet implemented. Add a `Vp9` variant here and wire it up when it is.
}

impl EncoderPreset {
    fn to_settings(&self, width: u32, height: u32) -> Settings {
        match self {
            EncoderPreset::H264Yuv420p => {
                Settings::preset_h264_yuv420p(width as usize, height as usize, false)
            }
        }
    }
}

/// Map a `(container, codec)` pair to the implemented encoder preset, if any.
/// `None` means the combo is legal in the compatibility matrix but not yet
/// wired up — callers should surface that as "encoder not yet implemented".
fn encoder_preset_for(container: VideoContainer, codec: VideoCodec) -> Option<EncoderPreset> {
    use EncoderPreset::*;
    match (container, codec) {
        (VideoContainer::Mp4,  VideoCodec::H264)
        | (VideoContainer::Mov, VideoCodec::H264)
        | (VideoContainer::Mkv, VideoCodec::H264) => Some(H264Yuv420p),
        _ => None,
    }
}

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
    /// container, and codec.
    ///
    /// Validates `(container, codec)` against the compatibility matrix first
    /// (cheap — no FFmpeg calls). Then dispatches to the right encoder preset.
    /// Only the combos we actually wire up today succeed; any other legal
    /// combo returns an "encoder not yet implemented" error.
    pub fn open(
        path: &Path,
        width: u32,
        height: u32,
        fps: f32,
        container: VideoContainer,
        codec: VideoCodec,
    ) -> Result<Self, VideoError> {
        if !codec.is_supported_in(container) {
            return Err(VideoError(format!(
                "codec {:?} is not valid in a {:?} container",
                codec, container
            )));
        }

        let preset = encoder_preset_for(container, codec).ok_or_else(|| {
            VideoError(format!(
                "encoder not yet implemented for {:?} + {:?}",
                container, codec
            ))
        })?;

        crate::video::ensure_init();
        let settings = preset.to_settings(width, height);
        let encoder = Encoder::new(path.to_path_buf(), settings).map_err(|e| {
            // EINVAL at encoder open time almost always means the underlying
            // ffmpeg build has no encoder registered for the requested codec
            // (vcpkg's default `ffmpeg` port disables libx264 under GPL, and
            // hardware encoders are disabled too). Enrich the error so users
            // don't have to chase "Invalid argument" alone.
            let raw = VideoError::from(e);
            VideoError(format!(
                "{} — this usually means the ffmpeg build in use has no {:?} \
                 encoder. On Windows, reinstall vcpkg ffmpeg with \
                 `vcpkg install ffmpeg[x264,gpl]:x64-windows --recurse` (libx264 \
                 is GPL and not in the default feature set), or use a BtbN \
                 `gpl-shared` prebuilt pack. See docs/video-setup.md.",
                raw.0,
                codec,
            ))
        })?;
        Ok(Self {
            inner: Some(encoder),
            path: path.to_path_buf(),
            fps: fps.max(1.0),
            frame_count: 0,
            expected_size: (width, height),
            warned_size_mismatch: false,
        })
    }

    /// Whether an encoder preset is wired up for the given combo. The combo
    /// must also pass [`VideoCodec::is_supported_in`] — this only reflects
    /// what we've implemented, not what's legal in the container.
    ///
    /// UI uses this to warn the user *before* they click Render that a combo
    /// they've selected won't actually encode.
    pub fn has_encoder_preset(container: VideoContainer, codec: VideoCodec) -> bool {
        encoder_preset_for(container, codec).is_some()
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
