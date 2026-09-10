//! The crate's two luma (perceived-brightness) weightings, in one place.
//!
//! Collapsing an RGB pixel to a single brightness number is one of the most
//! repeated lines in the operation library — thresholds, masks, histograms,
//! edge detectors, tone mapping and every "preserve the colour, change the
//! brightness" adjustment all need it. It was written out longhand at 81 sites
//! across ~50 files, which made two things easy to miss:
//!
//! 1. **There are two conventions in use, not one.** Most of the image
//!    operations use Rec. 709 / sRGB weights; the statistical `numbers/image/`
//!    nodes, `grayscale`, `channels/mixer` and a handful of others use the
//!    older Rec. 601 weights. Both are defensible, and changing either would
//!    change what those nodes output, so both live here under names that say
//!    which is which. Pick deliberately when writing a new operation:
//!    [`rec709`] for anything modelling display brightness, [`rec601`] only to
//!    stay consistent with an existing Rec. 601 node.
//! 2. **A typo in a coefficient is invisible.** `0.7152` written as `0.7125`
//!    still compiles, still looks plausible, and shifts every pixel slightly.
//!
//! The slice-taking variants ([`rec709_px`] / [`rec601_px`]) additionally fold
//! in the channel-count rule the operations were all repeating by hand: an
//! image with fewer than three channels is already luminance, so channel 0 is
//! used directly rather than being weighted.

/// Rec. 709 / sRGB luma coefficients (red, green, blue).
///
/// The weighting used by HDTV and sRGB, and the default for new operations.
pub const REC709: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// Rec. 601 luma coefficients (red, green, blue).
///
/// The older SDTV weighting. Kept because a number of nodes already ship it and
/// their output is part of their contract — see the module docs.
pub const REC601: [f32; 3] = [0.299, 0.587, 0.114];

/// Rec. 709 coefficients at `f64` precision.
///
/// A separate constant rather than `REC709[i] as f64`: widening the `f32`
/// literal gives a *different* number from the `f64` literal, so a caller
/// working in double precision must start from double-precision coefficients
/// or its results shift.
pub const REC709_F64: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// Rec. 709 luma of a linear RGB triple.
#[inline]
pub fn rec709(r: f32, g: f32, b: f32) -> f32 {
    REC709[0] * r + REC709[1] * g + REC709[2] * b
}

/// Rec. 709 luma of a linear RGB triple at `f64` precision. See [`REC709_F64`].
#[inline]
pub fn rec709_f64(r: f64, g: f64, b: f64) -> f64 {
    REC709_F64[0] * r + REC709_F64[1] * g + REC709_F64[2] * b
}

/// Rec. 601 luma of an RGB triple.
#[inline]
pub fn rec601(r: f32, g: f32, b: f32) -> f32 {
    REC601[0] * r + REC601[1] * g + REC601[2] * b
}

/// Rec. 709 luma of a pixel slice of any channel count.
///
/// 1- and 2-channel pixels are already luminance (grey, grey+alpha), so
/// channel 0 is returned unweighted. An empty slice yields 0.
#[inline]
pub fn rec709_px(px: &[f32]) -> f32 {
    match px.len() {
        0 => 0.0,
        1 | 2 => px[0],
        _ => rec709(px[0], px[1], px[2]),
    }
}

/// Rec. 601 luma of a pixel slice of any channel count. See [`rec709_px`].
#[inline]
pub fn rec601_px(px: &[f32]) -> f32 {
    match px.len() {
        0 => 0.0,
        1 | 2 => px[0],
        _ => rec601(px[0], px[1], px[2]),
    }
}

#[cfg(test)]
#[path = "luma_tests.rs"]
mod tests;
