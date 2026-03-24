//! Content-aware image resizing via seam carving.
//!
//! Implements the Avidan-Shamir seam carving algorithm from scratch on [`FloatImage`].
//! The algorithm removes or inserts low-energy seams (connected paths of pixels from
//! top to bottom or left to right) to resize an image while preserving visually
//! important content. Horizontal seams are handled by transposing the image, applying
//! vertical seam operations, and transposing back.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{
    convert_input, default_image, OperationError, OperationResponse, OutputResponse,
};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Content-aware resize using the seam carving algorithm.
///
/// Shrinks or enlarges an image to the target dimensions by removing or duplicating
/// the lowest-energy connected pixel paths (seams). This preserves visually important
/// regions better than uniform scaling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformSeamCarve {}

impl OpImageTransformSeamCarve {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "seam carve".to_string(),
            description: "Content-aware resize via seam carving. Removes or inserts low-energy pixel seams to change dimensions while preserving important content.".to_string(),
        }
    }

    /// Creates the default inputs: source image, target width, and target height.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "image".to_string(),
                Value::Image {
                    data: default_image(),
                    change_id: get_id(),
                },
                None,
                None,
            ),
            Input::new(
                "width".to_string(),
                Value::Integer(512),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 10000.0)),
                    speed: None,
                }),
                None,
            ),
            Input::new(
                "height".to_string(),
                Value::Integer(512),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 10000.0)),
                    speed: None,
                }),
                None,
            ),
        ]
    }

    /// Creates the default outputs: resized image, and its actual width and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new(
                "output".to_string(),
                Value::Image {
                    data: default_image(),
                    change_id: get_id(),
                },
                None,
            ),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the seam carving operation.
    ///
    /// Converts the inputs, clamps dimensions, runs the seam carving algorithm,
    /// and returns the resized image along with its actual dimensions.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs to expected types
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        // Return early if any input failed conversion
        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        // Extract values (safe to unwrap after error check)
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else {
            unreachable!()
        };
        let Value::Integer(mut width) = width_converted.unwrap() else {
            unreachable!()
        };
        let Value::Integer(mut height) = height_converted.unwrap() else {
            unreachable!()
        };

        // Ensure minimum dimensions of 1x1
        width = width.max(1);
        height = height.max(1);

        // Run the seam carving algorithm
        let output = seam_carve(&data, width as u32, height as u32);

        let value_width = Value::Integer(output.width() as i32);
        let value_height = Value::Integer(output.height() as i32);

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {
                    value: Value::Image {
                        data: Arc::new(output),
                        change_id: get_id(),
                    },
                },
                OutputResponse {
                    value: value_width,
                },
                OutputResponse {
                    value: value_height,
                },
            ],
        })
    }
}

// ---------------------------------------------------------------------------
// Seam carving algorithm implementation
// ---------------------------------------------------------------------------

/// Top-level orchestrator: resize `img` to `target_w` x `target_h` using seam carving.
///
/// Width is adjusted first (vertical seams), then height (horizontal seams via transpose).
fn seam_carve(img: &FloatImage, target_w: u32, target_h: u32) -> FloatImage {
    let mut result = img.clone();

    // --- Adjust width (vertical seams) ---
    let delta_w = target_w as i32 - result.width() as i32;
    if delta_w < 0 {
        // Remove vertical seams to shrink width
        for _ in 0..delta_w.unsigned_abs() {
            let energy = compute_energy(&result);
            let cumulative = compute_cumulative_energy(&energy, result.width() as usize, result.height() as usize);
            let seam = find_seam(&cumulative, result.width() as usize, result.height() as usize);
            result = remove_seam(&result, &seam);
        }
    } else if delta_w > 0 {
        // Insert vertical seams to enlarge width. Each pass can insert at most w-1
        // seams (doubling the image), so we may need multiple passes for large enlargements.
        result = enlarge_width(result, target_w);
    }

    // --- Adjust height (horizontal seams via transpose) ---
    let delta_h = target_h as i32 - result.height() as i32;
    if delta_h < 0 {
        // Transpose, remove vertical seams, transpose back
        result = transpose(&result);
        for _ in 0..delta_h.unsigned_abs() {
            let energy = compute_energy(&result);
            let cumulative = compute_cumulative_energy(&energy, result.width() as usize, result.height() as usize);
            let seam = find_seam(&cumulative, result.width() as usize, result.height() as usize);
            result = remove_seam(&result, &seam);
        }
        result = transpose(&result);
    } else if delta_h > 0 {
        // Transpose, enlarge via vertical seams, transpose back
        result = transpose(&result);
        result = enlarge_width(result, target_h);
        result = transpose(&result);
    }

    result
}

/// Enlarges the image width to `target_w` using one or more passes of seam insertion.
///
/// Each pass can insert at most `current_width - 1` seams (roughly doubling the image).
/// If the target requires more, multiple passes are performed on the progressively
/// enlarged result.
fn enlarge_width(mut img: FloatImage, target_w: u32) -> FloatImage {
    while img.width() < target_w {
        let remaining = target_w as usize - img.width() as usize;
        // Each pass can insert at most w-1 seams
        let max_insert = (img.width() as usize).saturating_sub(1).max(1);
        let n = remaining.min(max_insert);
        let seams = find_n_seams(&img, n);
        if seams.is_empty() {
            break;
        }
        img = insert_seams(&img, &seams);
    }
    img
}

/// Computes per-pixel energy as the gradient magnitude using forward differences.
///
/// For each pixel, sums the absolute differences to the right and bottom neighbors
/// across all channels. Edge pixels use backward differences. The energy map is
/// computed in parallel across rows using rayon.
fn compute_energy(img: &FloatImage) -> Vec<f32> {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let ch = img.channels() as usize;
    let raw = img.as_raw();
    let mut energy = vec![0.0f32; w * h];

    // Parallel energy computation across rows
    energy
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row_energy)| {
            for x in 0..w {
                let base = (y * w + x) * ch;
                let mut e = 0.0f32;

                // Horizontal gradient: difference to right neighbor (or left at right edge)
                let nx = if x + 1 < w { x + 1 } else { x.saturating_sub(1) };
                let neighbor_h = (y * w + nx) * ch;
                for c in 0..ch {
                    e += (raw[base + c] - raw[neighbor_h + c]).abs();
                }

                // Vertical gradient: difference to bottom neighbor (or top at bottom edge)
                let ny = if y + 1 < h { y + 1 } else { y.saturating_sub(1) };
                let neighbor_v = (ny * w + x) * ch;
                for c in 0..ch {
                    e += (raw[base + c] - raw[neighbor_v + c]).abs();
                }

                row_energy[x] = e;
            }
        });

    energy
}

/// Builds the cumulative energy map using dynamic programming (top to bottom).
///
/// Each pixel's cumulative energy is its own energy plus the minimum of the three
/// pixels above it (top-left, top-center, top-right). The first row is copied
/// directly from the energy map.
fn compute_cumulative_energy(energy: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut cumulative = energy.to_vec();

    // Sequential DP: each row depends on the previous row
    for y in 1..h {
        for x in 0..w {
            let idx = y * w + x;
            // Minimum of three neighbors in the row above
            let mut min_above = cumulative[(y - 1) * w + x];
            if x > 0 {
                min_above = min_above.min(cumulative[(y - 1) * w + x - 1]);
            }
            if x + 1 < w {
                min_above = min_above.min(cumulative[(y - 1) * w + x + 1]);
            }
            cumulative[idx] += min_above;
        }
    }

    cumulative
}

/// Finds the minimum-energy vertical seam by backtracking from the last row.
///
/// Returns a vector of length `h` where `seam[y]` is the x-coordinate of the
/// seam pixel in row `y`.
fn find_seam(cumulative: &[f32], w: usize, h: usize) -> Vec<usize> {
    let mut seam = vec![0usize; h];

    // Find the x with minimum cumulative energy in the last row
    let last_row_start = (h - 1) * w;
    let mut min_x = 0;
    let mut min_val = cumulative[last_row_start];
    for x in 1..w {
        if cumulative[last_row_start + x] < min_val {
            min_val = cumulative[last_row_start + x];
            min_x = x;
        }
    }
    seam[h - 1] = min_x;

    // Backtrack from bottom to top, choosing the minimum neighbor above
    for y in (0..h - 1).rev() {
        let prev_x = seam[y + 1];
        let mut best_x = prev_x;
        let mut best_val = cumulative[y * w + prev_x];

        if prev_x > 0 && cumulative[y * w + prev_x - 1] < best_val {
            best_val = cumulative[y * w + prev_x - 1];
            best_x = prev_x - 1;
        }
        if prev_x + 1 < w && cumulative[y * w + prev_x + 1] < best_val {
            best_x = prev_x + 1;
        }

        seam[y] = best_x;
    }

    seam
}

/// Removes a single vertical seam from the image, producing an image one pixel narrower.
///
/// For each row, copies all pixels except the one at the seam's x-coordinate.
/// Uses raw slice operations for performance.
fn remove_seam(img: &FloatImage, seam: &[usize]) -> FloatImage {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let ch = img.channels() as usize;
    let new_w = w - 1;
    let raw = img.as_raw();
    let mut new_data = vec![0.0f32; new_w * h * ch];

    for y in 0..h {
        let seam_x = seam[y];
        let src_row = y * w * ch;
        let dst_row = y * new_w * ch;

        // Copy pixels before the seam
        let before_len = seam_x * ch;
        new_data[dst_row..dst_row + before_len].copy_from_slice(&raw[src_row..src_row + before_len]);

        // Copy pixels after the seam
        let after_src = src_row + (seam_x + 1) * ch;
        let after_dst = dst_row + before_len;
        let after_len = (w - seam_x - 1) * ch;
        new_data[after_dst..after_dst + after_len].copy_from_slice(&raw[after_src..after_src + after_len]);
    }

    FloatImage::from_raw(new_w as u32, h as u32, ch as u32, new_data).unwrap()
}

/// Finds `n` seams for insertion by progressively removing them from a working copy.
///
/// Each seam is found on the shrunk image, then its x-coordinates are mapped back
/// to the original image's coordinate space by accounting for previously removed seams.
/// Returns seams in original-image coordinates.
fn find_n_seams(img: &FloatImage, n: usize) -> Vec<Vec<usize>> {
    let h = img.height() as usize;
    let mut working = img.clone();
    let mut seams_original: Vec<Vec<usize>> = Vec::with_capacity(n);

    for _ in 0..n {
        if working.width() <= 1 {
            break;
        }

        let energy = compute_energy(&working);
        let cumulative = compute_cumulative_energy(&energy, working.width() as usize, working.height() as usize);
        let seam = find_seam(&cumulative, working.width() as usize, working.height() as usize);

        // Map the seam back to original coordinates by adjusting for all previously
        // removed seams. For each row, any previously removed seam at an x <= the
        // current seam's x shifts the current coordinate right by one.
        let mut original_seam = seam.clone();
        for prev_seam in &seams_original {
            for y in 0..h {
                if prev_seam[y] <= original_seam[y] {
                    original_seam[y] += 1;
                }
            }
        }

        seams_original.push(original_seam);
        working = remove_seam(&working, &seam);
    }

    seams_original
}

/// Inserts all given seams into the image simultaneously, producing a wider image.
///
/// For each row, the seam x-coordinates are sorted, then the row is rebuilt by
/// copying original pixels and inserting an averaged duplicate at each seam position.
/// The inserted pixel is the average of the seam pixel and its right neighbor.
fn insert_seams(img: &FloatImage, seams: &[Vec<usize>]) -> FloatImage {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let ch = img.channels() as usize;
    let n = seams.len();
    let new_w = w + n;
    let raw = img.as_raw();
    let mut new_data = vec![0.0f32; new_w * h * ch];

    for y in 0..h {
        // Collect and sort seam x-coordinates for this row
        let mut seam_xs: Vec<usize> = seams.iter().map(|s| s[y]).collect();
        seam_xs.sort_unstable();

        let src_row = y * w * ch;
        let dst_row = y * new_w * ch;
        let mut dst_x = 0;

        // Index into sorted seam_xs
        let mut seam_idx = 0;

        for src_x in 0..w {
            // Copy the original pixel
            let src_start = src_row + src_x * ch;
            let dst_start = dst_row + dst_x * ch;
            new_data[dst_start..dst_start + ch].copy_from_slice(&raw[src_start..src_start + ch]);
            dst_x += 1;

            // If this x is a seam position, insert an averaged pixel after it
            while seam_idx < seam_xs.len() && seam_xs[seam_idx] == src_x {
                let dst_start = dst_row + dst_x * ch;
                // Average with right neighbor (or duplicate if at right edge)
                let right_x = (src_x + 1).min(w - 1);
                let right_start = src_row + right_x * ch;
                for c in 0..ch {
                    new_data[dst_start + c] = (raw[src_start + c] + raw[right_start + c]) * 0.5;
                }
                dst_x += 1;
                seam_idx += 1;
            }
        }
    }

    FloatImage::from_raw(new_w as u32, h as u32, ch as u32, new_data).unwrap()
}

/// Transposes the image (swaps width and height).
///
/// Used to convert horizontal seam operations into vertical seam operations.
/// Pixel `(x, y)` in the input maps to `(y, x)` in the output.
fn transpose(img: &FloatImage) -> FloatImage {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let ch = img.channels() as usize;
    let raw = img.as_raw();
    // Transposed image has dimensions (h, w)
    let mut new_data = vec![0.0f32; w * h * ch];

    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * ch;
            let dst = (x * h + y) * ch; // (x, y) -> pixel at column=y, row=x in new image
            new_data[dst..dst + ch].copy_from_slice(&raw[src..src + ch]);
        }
    }

    FloatImage::from_raw(h as u32, w as u32, ch as u32, new_data).unwrap()
}

#[cfg(test)]
#[path = "seam_carve_tests.rs"]
mod tests;
