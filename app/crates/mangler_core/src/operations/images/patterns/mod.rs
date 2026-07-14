//! Pattern generation operations.
//!
//! This module provides operations that generate repeating tiled patterns as
//! grayscale images. Available patterns include brick, hexagonal, weave, and
//! a tile sampler that scatters an input pattern across a grid with randomization.

use crate::float_image::FloatImage;

pub mod brick;
pub mod flood_fill;
pub mod flood_fill_mapper;
pub mod hexagonal;
pub mod scatter_on_curve;
pub mod splatter;
pub mod tile_generator;
pub mod tile_sampler;
pub mod weave;

/// Precomputed placement of a single rotated/scaled pattern stamp, shared by
/// the `splatter` and `scatter on curve` ops.
///
/// The bounding box (`sx..ex`, `sy..ey`) is in output pixels and is the
/// caller's responsibility to clamp to the output bounds — [`draw_stamp`]
/// iterates it verbatim.
pub(crate) struct StampPlacement {
    /// Stamp center X in output pixels.
    pub center_x: f64,
    /// Stamp center Y in output pixels.
    pub center_y: f64,
    /// Cosine of the stamp's rotation angle.
    pub cos_a: f64,
    /// Sine of the stamp's rotation angle.
    pub sin_a: f64,
    /// Full drawn size in output pixels (post per-instance scaling).
    pub draw: f64,
    /// Per-RGB-channel tint multiplier (alpha and beyond are untinted).
    pub tint: [f64; 3],
    /// Inclusive-left bounding-box X in output pixels (clamped by the caller).
    pub sx: i32,
    /// Exclusive-right bounding-box X in output pixels (clamped by the caller).
    pub ex: i32,
    /// Inclusive-top bounding-box Y in output pixels (clamped by the caller).
    pub sy: i32,
    /// Exclusive-bottom bounding-box Y in output pixels (clamped by the caller).
    pub ey: i32,
}

/// Composite one stamp into output row `py` (a `width * channels` slice `row`),
/// max-blending each output channel with the tinted, nearest-sampled pattern
/// texel.
///
/// Uses the inverse transform (output px -> stamp-local coords -> pattern UV)
/// with the stamp's rotation and scale, mirroring the pattern outward. A no-op
/// when `py` is outside the stamp's vertical span. The X iteration is exactly
/// `stamp.sx..stamp.ex`, so the caller must have clamped that range into
/// `[0, width)`.
pub(crate) fn draw_stamp(
    row: &mut [f32],
    py: i32,
    stamp: &StampPlacement,
    pattern: &FloatImage,
    channels: usize,
) {
    if py < stamp.sy || py >= stamp.ey {
        return;
    }
    let pat_w = pattern.width() as f64;
    let pat_h = pattern.height() as f64;
    for px in stamp.sx..stamp.ex {
        // Inverse transform: output px -> local coords -> pattern UV.
        let dx = px as f64 - stamp.center_x;
        let dy = py as f64 - stamp.center_y;
        let lx = stamp.cos_a * dx + stamp.sin_a * dy;
        let ly = -stamp.sin_a * dx + stamp.cos_a * dy;
        let u = (lx / stamp.draw + 0.5) * pat_w;
        let v = (ly / stamp.draw + 0.5) * pat_h;
        if u < 0.0 || u >= pat_w || v < 0.0 || v >= pat_h {
            continue;
        }
        let src = pattern.get_pixel(u as u32, v as u32);
        let base = px as usize * channels;
        // Max composite with per-channel tint applied first.
        for c in 0..channels {
            let t = if c < 3 { stamp.tint[c] as f32 } else { 1.0 };
            row[base + c] = row[base + c].max(src[c] * t);
        }
    }
}
