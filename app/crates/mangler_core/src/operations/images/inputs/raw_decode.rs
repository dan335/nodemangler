//! Camera RAW decode via rawler (Canon CR3/CR2, Nikon NEF, Sony ARW, DNG, …).
//!
//! rawler ships a full development pipeline — rescale, demosaic, crop, white
//! balance, camera→sRGB matrix, sRGB gamma — whose default output is
//! sRGB-encoded floats, exactly the convention the rest of the pipeline uses.
//! That means a decoded RAW drops straight into the adjustment nodes with no
//! adapter layer.
//!
//! Shared by: the `from file` node's extension dispatch (with
//! [`RawOptions::default`]), the dedicated `from raw` node (with user values),
//! the GUI's 2D library image preview, and library-panel thumbnails via
//! [`decode_raw_preview_rgba8`] (embedded camera JPEG; falls back to
//! [`decode_raw`] with a small `max_dimension`).
//!
//! The whole module compiles without the `raw` feature; only [`decode_raw`]'s
//! body is gated, so the `from raw` node exists in every build and the node
//! menu stays stable.

use crate::float_image::FloatImage;

#[cfg(feature = "raw")]
use rawler::decoders::Orientation;
#[cfg(feature = "raw")]
use rawler::imgop::develop::{Intermediate, ProcessingStep, RawDevelop};

/// How the raw's white-balance multipliers are chosen before development.
///
/// These multipliers are applied per-CFA-channel *before* demosaic and *before*
/// the camera→sRGB matrix, so the choice cannot be reproduced downstream. This
/// is deliberately not a temperature control: `adjustments/white_balance.rs`
/// already implements Kelvin + tint with Bradford adaptation, and a second
/// temperature model here would disagree with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawWhiteBalance {
    /// The multipliers the camera recorded at capture time. Matches the
    /// camera's own JPEG.
    #[default]
    AsShot,
    /// The camera's calibrated neutral — "no creative white-balance decision",
    /// the right starting point when the user intends to set temperature with a
    /// downstream `white balance` node.
    CameraNeutral,
    /// Drop the white-balance step entirely: raw sensor ratios, which are
    /// green-dominant and never directly what a photographer wants. Exposed for
    /// diagnostics and custom calibration pipelines.
    None,
}

impl RawWhiteBalance {
    /// Parses the node's dropdown label. Unknown strings fall back to the
    /// default, matching how the curve nodes handle their dropdowns.
    pub fn from_label(label: &str) -> Self {
        match label {
            "camera neutral" => Self::CameraNeutral,
            "none" => Self::None,
            _ => Self::AsShot,
        }
    }
}

/// Development settings shared by the plain extension dispatch and the
/// dedicated `from raw` node.
#[derive(Debug, Clone, PartialEq)]
pub struct RawOptions {
    /// Which white-balance multipliers to develop with.
    pub white_balance: RawWhiteBalance,
    /// `false` drops the demosaic step, yielding the 1-channel CFA mosaic at
    /// full sensor resolution (rawler skips the colour steps too).
    pub demosaic: bool,
    /// `true` drops the sRGB gamma step, yielding scene-linear output.
    pub linear_output: bool,
    /// Longest-edge cap applied after development. `None` = full resolution.
    pub max_dimension: Option<u32>,
    /// Apply the file's EXIF orientation. Off only for diagnostics.
    pub apply_orientation: bool,
    /// Exposure adjustment in stops, applied to the developed buffer. `0.0` = off.
    pub exposure_stops: f32,
}

impl Default for RawOptions {
    fn default() -> Self {
        Self {
            white_balance: RawWhiteBalance::AsShot,
            demosaic: true,
            // Matches the pipeline's sRGB-encoded convention and the other loaders.
            linear_output: false,
            // The dispatch path stays lossless; the `from raw` node defaults to 4096.
            max_dimension: None,
            apply_orientation: true,
            exposure_stops: 0.0,
        }
    }
}

/// Builds rawler's processing-step vector for `opts`.
///
/// Always emitted in rawler's canonical `RawDevelop::default()` order, so the
/// result is correct whether rawler iterates the vector or tests membership.
/// `Rescale`, `CropActiveArea` and `CropDefault` are never optional — without
/// them the output carries the masked black-border columns and the uncropped
/// sensor area.
#[cfg(feature = "raw")]
pub(crate) fn steps_for(opts: &RawOptions) -> Vec<ProcessingStep> {
    let mut steps = vec![ProcessingStep::Rescale];
    if opts.demosaic {
        steps.push(ProcessingStep::Demosaic);
    }
    steps.push(ProcessingStep::CropActiveArea);
    if opts.white_balance != RawWhiteBalance::None {
        steps.push(ProcessingStep::WhiteBalance);
    }
    if opts.demosaic {
        // Calibrate is a no-op on monochrome data, but keep the pairing explicit.
        steps.push(ProcessingStep::Calibrate);
    }
    steps.push(ProcessingStep::CropDefault);
    if !opts.linear_output {
        steps.push(ProcessingStep::SRgb);
    }
    steps
}

/// Rewrites `src` into a new [`FloatImage`] with `orientation` applied.
///
/// `src` is a flat interleaved buffer of `channels_in` components per pixel;
/// only the first `channels_out` are kept (used to drop the 4th layer of a
/// 4-colour CFA, which the pipeline would otherwise read as alpha).
///
/// The transform is fused into this single source→destination copy rather than
/// done as a separate pass, because at 26–45 MP an extra full-image copy costs
/// hundreds of megabytes.
#[cfg(feature = "raw")]
pub(crate) fn orient_into(
    src: &[f32],
    width: usize,
    height: usize,
    channels_in: usize,
    channels_out: usize,
    orientation: Orientation,
) -> Option<FloatImage> {
    // `Unknown` means the camera didn't record an orientation — treat it as
    // Normal rather than rotating on a guess.
    let (transpose, hflip, vflip) = if matches!(orientation, Orientation::Unknown) {
        (false, false, false)
    } else {
        orientation.to_flips()
    };

    // Fast path for the common landscape case: a straight strided copy.
    if !transpose && !hflip && !vflip && channels_in == channels_out {
        return FloatImage::from_raw(
            width as u32,
            height as u32,
            channels_out as u32,
            src.to_vec(),
        );
    }

    let (dst_w, dst_h) = if transpose { (height, width) } else { (width, height) };
    let mut out = Vec::with_capacity(dst_w * dst_h * channels_out);
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            // Invert the forward transform, which flips before transposing.
            let (mut sx, mut sy) = if transpose { (dy, dx) } else { (dx, dy) };
            if hflip {
                sx = width - 1 - sx;
            }
            if vflip {
                sy = height - 1 - sy;
            }
            let base = (sy * width + sx) * channels_in;
            out.extend_from_slice(&src[base..base + channels_out]);
        }
    }

    FloatImage::from_raw(dst_w as u32, dst_h as u32, channels_out as u32, out)
}

/// Scales `img` by `stops` of exposure.
///
/// Exposure is a linear-light operation, so sRGB-encoded data is decoded,
/// scaled and re-encoded. Values are left unclamped in both paths, matching the
/// unbounded-float contract the rest of the pipeline relies on.
#[cfg(feature = "raw")]
fn apply_exposure(img: &mut FloatImage, stops: f32, already_linear: bool) {
    use crate::color::color_spaces::rgb_linear::{linear_to_nonlinear_srgb, nonlinear_to_linear_rgb};

    let gain = 2f32.powf(stops);
    let channels = img.channels() as usize;
    // Never scale alpha.
    let colour_channels = if channels == 2 || channels == 4 { channels - 1 } else { channels };

    for pixel in img.pixels_mut() {
        for component in pixel.iter_mut().take(colour_channels) {
            *component = if already_linear {
                *component * gain
            } else {
                linear_to_nonlinear_srgb(nonlinear_to_linear_rgb(*component) * gain)
            };
        }
    }
}

/// Decodes a camera RAW file into a [`FloatImage`].
///
/// With [`RawOptions::default`] this reproduces rawler's stock development
/// pipeline plus EXIF orientation: sRGB primaries, sRGB gamma, 3 channels —
/// roughly what the camera's own JPEG looks like.
#[cfg(feature = "raw")]
pub fn decode_raw(path: &std::path::Path, opts: &RawOptions) -> Result<FloatImage, String> {
    // rawler catches panics inside its decoders and reports them as errors, so
    // a corrupt file cannot unwind out of the blocking pool.
    let mut raw = rawler::decode_file(path).map_err(|e| e.to_string())?;

    if opts.white_balance == RawWhiteBalance::CameraNeutral {
        raw.wb_coeffs = raw.neutralwb();
    }
    let orientation = if opts.apply_orientation { raw.orientation } else { Orientation::Normal };

    let developed = RawDevelop { steps: steps_for(opts) }
        .develop_intermediate(&raw)
        .map_err(|e| e.to_string())?;
    // Release the u16 sensor buffer before the large float pass.
    drop(raw);

    let mut image = match &developed {
        Intermediate::Monochrome(pixels) => {
            orient_into(&pixels.data, pixels.width, pixels.height, 1, 1, orientation)
        }
        Intermediate::ThreeColor(pixels) => orient_into(
            pixels.data.as_flattened(),
            pixels.width,
            pixels.height,
            3,
            3,
            orientation,
        ),
        // A 4-colour CFA's fourth layer is not alpha; keeping it would make the
        // whole pipeline treat it as transparency. Drop it.
        Intermediate::FourColor(pixels) => orient_into(
            pixels.data.as_flattened(),
            pixels.width,
            pixels.height,
            4,
            3,
            orientation,
        ),
    }
    .ok_or_else(|| "RAW decode produced a mismatched buffer size.".to_string())?;
    drop(developed);

    if opts.exposure_stops != 0.0 {
        apply_exposure(&mut image, opts.exposure_stops, opts.linear_output);
    }

    if let Some(max) = opts.max_dimension.filter(|&m| m > 0) {
        if image.width().max(image.height()) > max {
            image = image.resize_fit(max, max);
        }
    }

    Ok(image)
}

/// Stub used when the `raw` feature is disabled, so the `from raw` node still
/// compiles and the node menu is identical in every build.
#[cfg(not(feature = "raw"))]
pub fn decode_raw(_path: &std::path::Path, _opts: &RawOptions) -> Result<FloatImage, String> {
    Err("this build was compiled without camera RAW support (cargo feature \"raw\")".to_string())
}

/// Minimum long-edge for an embedded preview to be considered useful as a
/// library thumbnail. Some bodies only store a tiny ~160×120 IFD thumb;
/// a blurry postage stamp is worse than falling back to a reduced develop.
pub const RAW_PREVIEW_MIN_EDGE: u32 = 160;

/// Extracts the camera's embedded full/preview JPEG (rawler `full_image`) and
/// returns an RGBA8 buffer whose longest edge is ≤ `max_edge`.
///
/// Orders of magnitude cheaper than [`decode_raw`]: no demosaic, no float
/// develop pass. The trade-off is that this is the **camera's** rendering
/// (picture style / in-camera WB), not what the `from raw` node produces —
/// every DAM accepts that for library browsing.
///
/// Returns `Err` when the file has no usable preview (caller should fall back
/// to [`decode_raw`] with a small `max_dimension`). Previews smaller than
/// [`RAW_PREVIEW_MIN_EDGE`] are rejected for the same reason.
///
/// Orientation is left as the camera stored it — embedded JPEGs are often
/// already upright, and blindly applying EXIF orientation double-rotates some
/// bodies. Validate with real portrait fixtures if a specific brand looks wrong.
#[cfg(feature = "raw")]
pub fn decode_raw_preview_rgba8(
    path: &std::path::Path,
    max_edge: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    use image::GenericImageView;
    use rawler::analyze::extract_full_pixels;
    use rawler::decoders::RawDecodeParams;

    let max_edge = max_edge.max(1);
    let img = extract_full_pixels(path, &RawDecodeParams::default()).map_err(|e| e.to_string())?;
    let (w, h) = img.dimensions();
    if w.max(h) < RAW_PREVIEW_MIN_EDGE {
        return Err(format!(
            "embedded RAW preview too small ({w}×{h}; need ≥ {RAW_PREVIEW_MIN_EDGE})"
        ));
    }

    let thumb = if w.max(h) > max_edge {
        img.thumbnail(max_edge, max_edge)
    } else {
        img
    };
    let rgba = thumb.to_rgba8();
    let (rw, rh) = (rgba.width(), rgba.height());
    Ok((rgba.into_raw(), rw, rh))
}

/// Stub when the `raw` feature is off — library thumbs fall back cleanly.
#[cfg(not(feature = "raw"))]
pub fn decode_raw_preview_rgba8(
    _path: &std::path::Path,
    _max_edge: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    Err("this build was compiled without camera RAW support (cargo feature \"raw\")".to_string())
}

#[cfg(test)]
#[path = "raw_decode_tests.rs"]
mod tests;
