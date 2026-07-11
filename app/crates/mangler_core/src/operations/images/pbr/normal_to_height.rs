//! Height reconstruction from a tangent-space normal map.
//!
//! Inverts `normal from height` by integrating the slope field implied by
//! every normal vector. For a tangent-space normal `n = (nx, ny, nz)` the
//! local surface slopes are `dh/dx = -nx/nz` and `dh/dy = -ny/nz`. The
//! reconstructed height is defined only up to an additive constant and
//! suffers from drift along any single integration path, so we average
//! two cumulative sweeps (top-down, left-right) and renormalize the
//! result into [0, 1] for display.
//!
//! This is not a true Poisson solver — it is path-dependent and will
//! show seams if the input normals are not integrable (not derived from
//! an actual height field). For well-behaved inputs (round-trip from
//! `normal from height`) the error is low and the low-frequency shape
//! is recovered. For arbitrary authored normals, the output should be
//! treated as approximate.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Reconstructs a grayscale height field from a tangent-space normal map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrNormalToHeight {}

impl OpImagePbrNormalToHeight {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "normal to height".to_string(),
            description: "Reconstructs a grayscale height field from a tangent-space normal map.".to_string(),
            help: "For each pixel, unpacks the RGB-encoded normal into `[-1, 1]` and computes local slopes `dh/dx = -nx/nz` and `dh/dy = -ny/nz`. The height is integrated via two orthogonal cumulative sweeps (row then column) and the results are averaged, then the full field is renormalised to [0, 1] so the output is displayable and lossless under a final normal-from-height round-trip up to a linear rescale.\n\nIntegration is path-dependent: authored normals that do not come from a real height field (for example, decorative normals mixed between two independent layers) will show a low-frequency tilt or seam in the reconstruction. Scale rescales the reconstructed height about mid-grey AFTER normalisation (applying it to the slopes would be cancelled by the min/max stretch): larger scale compresses the relief toward 0.5 (gentler), smaller scale expands it (steeper), and scale 1 leaves it unchanged. Output is single-channel regardless of input channels.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Tangent-space normal map to integrate into a height field."),
            Input::new("scale".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 20.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Rescales the reconstructed height about mid-grey after normalisation; larger values flatten the relief, smaller values exaggerate it."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Reconstructed 1-channel height map normalised to [0, 1]."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let scale_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };

        let scale = scale.max(0.01);
        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let width = w as usize;
        let height = h as usize;

        // First pass: compute the slope field. `slope_x[y][x]` is dh/dx at
        // pixel (x,y); `slope_y[y][x]` is dh/dy.
        let mut slope_x = vec![0.0f32; width * height];
        let mut slope_y = vec![0.0f32; width * height];

        for y in 0..h {
            for x in 0..w {
                let px = data.get_pixel(x, y);
                // Unpack the normal. Grayscale inputs are treated as a flat up
                // vector (trivially zero slopes everywhere).
                let (nx, ny, nz) = if ch >= 3 {
                    (px[0] * 2.0 - 1.0, px[1] * 2.0 - 1.0, px[2] * 2.0 - 1.0)
                } else {
                    (0.0, 0.0, 1.0)
                };
                // Clamp nz to avoid division by ~0 on extreme normals.
                // NOTE: `scale` is deliberately NOT applied here. Dividing every
                // slope by a constant is exactly undone by the final min/max
                // renormalisation, so it would be a no-op. Scale is applied after
                // normalisation instead (see below).
                let nz_safe = if nz.abs() < 1e-4 { 1e-4_f32.copysign(nz) } else { nz };
                let sx = -nx / nz_safe;
                let sy = -ny / nz_safe;
                let idx = (y as usize) * width + (x as usize);
                slope_x[idx] = sx;
                slope_y[idx] = sy;
            }
        }

        // Second pass: two cumulative integrations.
        //
        // Path A: integrate along y first (down the first column), then along x
        //         for each row. Drift is along x within each row.
        // Path B: integrate along x first (across the first row), then along y
        //         for each column. Drift is along y within each column.
        // Averaging the two distributes drift in both axes.
        let mut h_a = vec![0.0f32; width * height];
        let mut h_b = vec![0.0f32; width * height];

        // Path A: first column by y, then each row by x.
        for y in 1..height {
            h_a[y * width] = h_a[(y - 1) * width] + slope_y[(y - 1) * width];
        }
        for y in 0..height {
            for x in 1..width {
                h_a[y * width + x] = h_a[y * width + (x - 1)] + slope_x[y * width + (x - 1)];
            }
        }

        // Path B: first row by x, then each column by y.
        for x in 1..width {
            h_b[x] = h_b[x - 1] + slope_x[x - 1];
        }
        for x in 0..width {
            for y in 1..height {
                h_b[y * width + x] = h_b[(y - 1) * width + x] + slope_y[(y - 1) * width + x];
            }
        }

        // Average the two paths into a single height field and track min/max.
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        let mut combined = vec![0.0f32; width * height];
        for i in 0..(width * height) {
            let v = 0.5 * (h_a[i] + h_b[i]);
            combined[i] = v;
            if v < min { min = v; }
            if v > max { max = v; }
        }

        // Normalise to [0, 1]. If the image is flat, emit mid-grey.
        let range = max - min;
        let mut output = FloatImage::new(w, h, 1);
        if range < 1e-8 {
            for y in 0..h {
                for x in 0..w {
                    output.put_pixel(x, y, &[0.5]);
                }
            }
        } else {
            let inv = 1.0 / range;
            for y in 0..h {
                for x in 0..w {
                    let v = (combined[(y as usize) * width + (x as usize)] - min) * inv;
                    // Apply `scale` about mid-grey AFTER normalisation, where it
                    // is not cancelled by the stretch. Larger scale compresses
                    // the relief toward 0.5 (gentler); smaller scale expands it
                    // (steeper). At scale 1 this is the identity.
                    let v = (0.5 + (v - 0.5) / scale).clamp(0.0, 1.0);
                    output.put_pixel(x, y, &[v]);
                }
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "normal_to_height_tests.rs"]
mod tests;
