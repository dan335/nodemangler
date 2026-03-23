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

/// Compute per-channel (R, G, B, A) statistics for an image.
///
/// Converts the FloatImage to RGBA f32 for uniform 4-channel analysis.
pub(crate) fn compute_image_stats(img: &FloatImage) -> Vec<(&'static str, ChannelStats)> {
    let dynamic = img.to_dynamic();
    let rgba = dynamic.to_rgba32f();
    let pixels: Vec<&[f32]> = rgba.as_raw().chunks(4).collect();
    let n = pixels.len() as f64;
    if n == 0.0 {
        return vec![
            ("r", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
            ("g", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
            ("b", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
            ("a", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
        ];
    }

    let mut result = Vec::with_capacity(4);
    for (ch, name) in ["r", "g", "b", "a"].iter().enumerate() {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut sum = 0.0_f64;
        for px in &pixels {
            let v = px[ch];
            if v < min { min = v; }
            if v > max { max = v; }
            sum += v as f64;
        }
        let mean = sum / n;
        let mut var_sum = 0.0_f64;
        for px in &pixels {
            let diff = px[ch] as f64 - mean;
            var_sum += diff * diff;
        }
        let stddev = (var_sum / n).sqrt();
        result.push((*name, ChannelStats {
            min,
            max,
            mean: mean as f32,
            stddev: stddev as f32,
        }));
    }
    result
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
    let dynamic = img.to_dynamic();
    let rgba = dynamic.to_rgba32f();
    let px = rgba.get_pixel(x, y);
    [px.0[0], px.0[1], px.0[2], px.0[3]]
}

#[cfg(test)]
#[path = "image_stats_tests.rs"]
mod tests;
