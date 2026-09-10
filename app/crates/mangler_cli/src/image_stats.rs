//! Image inspection utilities: statistics, pixel sampling, coordinate resolution.

use std::collections::HashSet;

use mangler_core::float_image::FloatImage;

// ── Image statistics helpers ──────────────────────────────────────────────

/// Per-channel statistics for an image.
pub(crate) struct ChannelStats {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub stddev: f32,
}

/// Expands one `FloatImage` pixel to RGBA floats, matching `FloatImage::to_rgba8`'s
/// channel rules (1ch: R=G=B=v, A=1; 2ch: R=G=B=gray, A=alpha; 3ch: A=1).
///
/// Reads the stored `f32` values as they are. Routing this through
/// `to_dynamic()` instead would clamp 1- and 2-channel images to `[0, 1]` and
/// quantize them to `u16` — and every mask, height field, distance field and
/// noise output in this app is 1-channel raw linear, so the reported figures
/// would silently disagree with what the `numbers/image` measurement nodes say
/// about the same image.
#[inline]
fn rgba_at(px: &[f32]) -> [f32; 4] {
    match px.len() {
        1 => [px[0], px[0], px[0], 1.0],
        2 => [px[0], px[0], px[0], px[1]],
        3 => [px[0], px[1], px[2], 1.0],
        _ => [px[0], px[1], px[2], px[3]],
    }
}

/// Compute per-channel (R, G, B, A) statistics for an image.
pub(crate) fn compute_image_stats(img: &FloatImage) -> Vec<(&'static str, ChannelStats)> {
    const NAMES: [&str; 4] = ["r", "g", "b", "a"];
    let n = (img.width() as usize) * (img.height() as usize);
    if n == 0 {
        return NAMES
            .iter()
            .map(|name| (*name, ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }))
            .collect();
    }

    // One pass for the extremes and the sums, a second for the variance --
    // cheaper than the four passes over a materialized RGBA copy this replaced.
    let mut min = [f32::MAX; 4];
    let mut max = [f32::MIN; 4];
    let mut sum = [0.0_f64; 4];
    for px in img.pixels() {
        let rgba = rgba_at(px);
        for ch in 0..4 {
            let v = rgba[ch];
            if v < min[ch] { min[ch] = v; }
            if v > max[ch] { max[ch] = v; }
            sum[ch] += v as f64;
        }
    }

    let mean = [sum[0] / n as f64, sum[1] / n as f64, sum[2] / n as f64, sum[3] / n as f64];
    let mut var_sum = [0.0_f64; 4];
    for px in img.pixels() {
        let rgba = rgba_at(px);
        for ch in 0..4 {
            let diff = rgba[ch] as f64 - mean[ch];
            var_sum[ch] += diff * diff;
        }
    }

    (0..4)
        .map(|ch| {
            (
                NAMES[ch],
                ChannelStats {
                    min: min[ch],
                    max: max[ch],
                    mean: mean[ch] as f32,
                    stddev: (var_sum[ch] / n as f64).sqrt() as f32,
                },
            )
        })
        .collect()
}

/// Combined stats result returned by `compute_full_image_stats`.
/// Groups per-channel statistics with transparency and unique color count
/// to avoid redundant image conversions.
pub(crate) struct FullImageStats {
    pub channels: Vec<(&'static str, ChannelStats)>,
    pub has_transparency: bool,
    pub unique_colors: usize,
}

/// Compute per-channel statistics, transparency, and unique color count in a
/// single pass over the image data. Avoids the multiple `to_dynamic()` /
/// `to_rgba8()` conversions that calling the individual helpers would incur.
pub(crate) fn compute_full_image_stats(img: &FloatImage) -> FullImageStats {
    let channels = compute_image_stats(img);

    // Convert to RGBA8 once for both transparency and unique color checks.
    let rgba8 = img.to_rgba8();
    let has_transparency = rgba8.pixels().any(|p| p.0[3] < 255);
    let unique_colors = {
        let colors: HashSet<[u8; 4]> = rgba8.pixels().map(|p| p.0).collect();
        colors.len()
    };

    FullImageStats { channels, has_transparency, unique_colors }
}

/// Check whether an image has any transparent pixels (alpha < 1.0).
#[cfg(test)]
pub(crate) fn has_transparency(img: &FloatImage) -> bool {
    let rgba = img.to_rgba8();
    rgba.pixels().any(|p| p.0[3] < 255)
}

/// Count unique colors in an image (RGBA8).
#[cfg(test)]
pub(crate) fn count_unique_colors(img: &FloatImage) -> usize {
    let rgba = img.to_rgba8();
    let colors: HashSet<[u8; 4]> = rgba.pixels().map(|p| p.0).collect();
    colors.len()
}

/// Resolve a sample coordinate string to (x, y) given image dimensions.
/// Accepts "x,y" or named positions: center, top-left, top-right, bottom-left, bottom-right.
pub(crate) fn resolve_sample_coord(s: &str, w: u32, h: u32) -> Result<(u32, u32), String> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "center" => Ok((w / 2, h / 2)),
        "top_left" => Ok((0, 0)),
        "top_right" => Ok((w.saturating_sub(1), 0)),
        "bottom_left" => Ok((0, h.saturating_sub(1))),
        "bottom_right" => Ok((w.saturating_sub(1), h.saturating_sub(1))),
        _ => {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "invalid sample '{}' — expected x,y or a named position (center, top-left, top-right, bottom-left, bottom-right)",
                    s
                ));
            }
            let x: u32 = parts[0].trim().parse().map_err(|_| format!("invalid x coordinate in '{}'", s))?;
            let y: u32 = parts[1].trim().parse().map_err(|_| format!("invalid y coordinate in '{}'", s))?;
            if x >= w || y >= h {
                return Err(format!("sample ({},{}) out of bounds for {}x{} image", x, y, w, h));
            }
            Ok((x, y))
        }
    }
}

/// Sample a pixel from an image at (x, y), returning RGBA floats.
pub(crate) fn sample_pixel(img: &FloatImage, x: u32, y: u32) -> [f32; 4] {
    rgba_at(img.get_pixel(x, y))
}

#[cfg(test)]
#[path = "image_stats_tests.rs"]
mod tests;
