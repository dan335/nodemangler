//! Reusable histogram widget for visualizing image luminance distribution.
//!
//! Provides histogram computation from FloatImage data and an egui rendering
//! function that draws a 256-bin bar chart. Used by the "visualizations" section
//! in the node settings panel for any node with image outputs.

use eframe::egui;
#[allow(unused_imports)] // Stroke/StrokeKind retained for commented-out debug drawing below.
use epaint::{vec2, Color32, Rect, Stroke, StrokeKind};
use mangler_core::float_image::FloatImage;
use mangler_core::value::Value;
use rayon::prelude::*;

use crate::graph::graph_node::{GraphNode, HistogramCache};
use crate::themes::theme::Theme;

/// Compute 256-bin histograms for luminance and per-channel R/G/B from a FloatImage.
///
/// For images with 3+ channels, luminance uses Rec. 709 coefficients:
/// `lum = 0.2126*R + 0.7152*G + 0.0722*B`, and R/G/B bins are populated independently.
/// For 1-2 channel images, only the luminance histogram is meaningful (R/G/B bins stay zeroed).
/// All four histograms share a single max_count for consistent vertical scaling.
pub fn compute_histogram(data: &FloatImage) -> HistogramCache {
    let ch = data.channels() as usize;
    let has_alpha = ch == 2 || ch == 4;
    let color_ch = if has_alpha { ch - 1 } else { ch };

    // Per-worker accumulator: (luminance, red, green, blue) bins.
    // Using `Box` keeps the 4 KiB accumulator off the stack so rayon's fold
    // identity closure stays cheap to invoke per split.
    type Bins = Box<([u32; 256], [u32; 256], [u32; 256], [u32; 256])>;
    let identity = || -> Bins { Box::new(([0u32; 256], [0u32; 256], [0u32; 256], [0u32; 256])) };

    let folded: Bins = data
        .as_slice()
        .par_chunks_exact(ch)
        .fold(identity, |mut acc, pixel| {
            // Skip fully transparent pixels. Operations like rotate fill the
            // uncovered corners with alpha=0, and counting those RGB=0 samples
            // would spike bin 0 of every channel and drown out the real content.
            if has_alpha && pixel[ch - 1] <= 0.0 {
                return acc;
            }

            if color_ch >= 3 {
                // Per-channel bins. Scale to 0..=255 and clamp via integer
                // saturation — cheaper than the branchy `f32::clamp`.
                let r_bin = (pixel[0] * 255.0) as i32;
                let g_bin = (pixel[1] * 255.0) as i32;
                let b_bin = (pixel[2] * 255.0) as i32;
                acc.1[r_bin.clamp(0, 255) as usize] += 1;
                acc.2[g_bin.clamp(0, 255) as usize] += 1;
                acc.3[b_bin.clamp(0, 255) as usize] += 1;

                // Luminance (Rec. 709)
                let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
                let lum_bin = (lum * 255.0) as i32;
                acc.0[lum_bin.clamp(0, 255) as usize] += 1;
            } else {
                // Grayscale: first channel is luminance
                let bin = (pixel[0] * 255.0) as i32;
                acc.0[bin.clamp(0, 255) as usize] += 1;
            }
            acc
        })
        .reduce(identity, |mut a, b| {
            // Merge per-worker bins. 256 adds × 4 channels is trivial
            // relative to the per-pixel work.
            for i in 0..256 {
                a.0[i] += b.0[i];
                a.1[i] += b.1[i];
                a.2[i] += b.2[i];
                a.3[i] += b.3[i];
            }
            a
        });

    let (bins, bins_r, bins_g, bins_b) = *folded;

    // Shared max across all histograms for consistent vertical scale
    let max_count = bins
        .iter()
        .chain(bins_r.iter())
        .chain(bins_g.iter())
        .chain(bins_b.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);

    HistogramCache {
        bins,
        bins_r,
        bins_g,
        bins_b,
        max_count,
        channels: data.channels(),
        image_change_id: String::new(), // caller sets this
    }
}

/// Ensure the histogram cache for a given output index is up to date.
///
/// Checks the output at `output_index` for a `Value::Image`. If the image's
/// `change_id` differs from the cached one (or no cache exists), recomputes
/// the histogram. Does nothing if the output is not an image.
pub fn ensure_histogram_cache(node: &mut GraphNode, output_index: usize) {
    // Get the image change_id and data reference
    let Some(output) = node.outputs.get(output_index) else {
        return;
    };
    let Value::Image { data, change_id } = &output.value else {
        return;
    };

    // Check if cache is already current
    if let Some(cache) = node.histogram_cache.get(&output_index) {
        if cache.image_change_id == *change_id {
            return; // cache is up to date
        }
    }

    // Recompute histogram
    let mut cache = compute_histogram(data);
    cache.image_change_id = change_id.clone();
    node.histogram_cache.insert(output_index, cache);
}

/// Draw a 256-bin histogram bar chart.
///
/// For 3+ channel images, draws luminance as a dark gray background layer,
/// then overlays R, G, B channels with semi-transparent colors. Overlapping
/// areas produce additive color mixing (yellow, magenta, cyan, white),
/// matching industry-standard histogram displays (Photoshop, Lightroom).
///
/// For 1-2 channel (grayscale) images, draws luminance only in gray.
///
/// Returns the Rect used for the histogram area (useful for
/// overlaying markers like in the levels widget).
pub fn draw_histogram(ui: &mut egui::Ui, cache: &HistogramCache, theme: &Theme) -> Rect {
    let available_width = ui.available_width();
    let height = 100.0;
    let (rect, _response) =
        ui.allocate_exact_size(vec2(available_width, height), egui::Sense::hover());

    if !ui.is_rect_visible(rect) {
        return rect;
    }

    let painter = ui.painter();
    let tv = theme.get();

    // Background
    painter.rect_filled(rect, 0.0, tv.histogram_bg);

    let bar_width = rect.width() / 256.0;

    // Determine if this is an RGB image (3+ color channels)
    let color_ch = if cache.channels == 2 || cache.channels == 4 {
        cache.channels - 1
    } else {
        cache.channels
    };
    let is_rgb = color_ch >= 3;

    if is_rgb {
        // Layer 1: Luminance as subtle background reference
        draw_bars(
            painter,
            rect,
            bar_width,
            &cache.bins,
            cache.max_count,
            tv.histogram_luminance,
        );

        // Layer 2-4: R, G, B channels with semi-transparent additive blending
        draw_bars(
            painter,
            rect,
            bar_width,
            &cache.bins_r,
            cache.max_count,
            tv.histogram_red,
        );
        draw_bars(
            painter,
            rect,
            bar_width,
            &cache.bins_g,
            cache.max_count,
            tv.histogram_green,
        );
        draw_bars(
            painter,
            rect,
            bar_width,
            &cache.bins_b,
            cache.max_count,
            tv.histogram_blue,
        );
    } else {
        // Grayscale: luminance only (use luminance color but fully opaque)
        let gray_color = Color32::from_rgb(
            tv.histogram_luminance.r(),
            tv.histogram_luminance.g(),
            tv.histogram_luminance.b(),
        );
        draw_bars(
            painter,
            rect,
            bar_width,
            &cache.bins,
            cache.max_count,
            gray_color,
        );
    }

    // Subtle border
    // painter.rect_stroke(
    //     rect,
    //     0.0,
    //     Stroke::new(1.0, tv.text_faint),
    //     StrokeKind::Outside,
    // );

    rect
}

/// Draw a set of 256 vertical bars for a single histogram channel.
fn draw_bars(
    painter: &egui::Painter,
    rect: Rect,
    bar_width: f32,
    bins: &[u32; 256],
    max_count: u32,
    color: Color32,
) {
    for (i, &count) in bins.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let normalized = count as f32 / max_count as f32;
        let bar_height = normalized * rect.height();
        let x = rect.left() + i as f32 * bar_width;
        // Add 1px overlap to prevent sub-pixel gaps between bars
        let bar_rect = Rect::from_min_max(
            egui::pos2(x, rect.bottom() - bar_height),
            egui::pos2(x + bar_width + 1.0, rect.bottom()),
        );
        painter.rect_filled(bar_rect, 0.0, color);
    }
}

#[cfg(test)]
#[path = "histogram_widget_tests.rs"]
mod tests;
