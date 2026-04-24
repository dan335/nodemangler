//! Combines two normal maps (typically base + detail) into one.
//!
//! Offers four blend modes commonly used in PBR authoring tools:
//! - **Whiteout** — the default; preserves overhang detail best.
//! - **RNM (Reoriented Normal Mapping)** — physically motivated; matches
//!   Unreal/Unity conventions.
//! - **Partial Derivative** — converts each normal to its (dx, dy) partial
//!   derivatives, sums them, and re-normalises. Cheap and predictable.
//! - **Linear** — naive componentwise average with re-normalisation. Worst
//!   fidelity but occasionally useful for soft blends.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::pbr::{normalize, pack_normal, unpack_normal};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Normal-map combining operation (detail-over-base blending).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrNormalCombine {}

impl OpImagePbrNormalCombine {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "normal combine".to_string(),
            description: "Combines two normal maps (e.g. base + detail) using Whiteout / RNM / partial-derivative / linear blending.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("base".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("detail".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // 0 = Whiteout (default), 1 = RNM, 2 = Partial Derivative, 3 = Linear
            Input::new("mode".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let base_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let detail_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let mode_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: base, change_id: _ } = base_converted.unwrap() else { unreachable!() };
        let Value::Image { data: detail, change_id: _ } = detail_converted.unwrap() else { unreachable!() };
        let Value::Integer(mode) = mode_converted.unwrap() else { unreachable!() };

        let (width, height) = base.dimensions();
        let mut output = FloatImage::new(width, height, 4);

        // Scale factor maps output (x, y) into detail-image UVs — handles mismatched sizes.
        let sx = if detail.width() > 0 { detail.width() as f32 / width.max(1) as f32 } else { 1.0 };
        let sy = if detail.height() > 0 { detail.height() as f32 / height.max(1) as f32 } else { 1.0 };
        let mut detail_buf = [0.0f32; 4];
        let detail_ch = detail.channels() as usize;

        for y in 0..height {
            for x in 0..width {
                let base_px = base.get_pixel(x, y);
                // Bilinear sample the detail map so it stretches over the base if sizes differ.
                detail.bilinear_sample(x as f32 * sx, y as f32 * sy, &mut detail_buf[..detail_ch]);
                let n1 = unpack_normal(base_px);
                let n2 = unpack_normal(&detail_buf[..detail_ch]);

                let combined = match mode {
                    1 => rnm(n1, n2),
                    2 => partial_derivative(n1, n2),
                    3 => linear(n1, n2),
                    _ => whiteout(n1, n2),
                };

                output.put_pixel(x, y, &pack_normal(combined));
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

/// Whiteout blend — simple, robust, preserves overhang detail.
/// See Colin Barré-Brisebois, "Blending in Detail" (2012).
fn whiteout(n1: [f32; 3], n2: [f32; 3]) -> [f32; 3] {
    normalize([n1[0] + n2[0], n1[1] + n2[1], n1[2] * n2[2]])
}

/// Reoriented Normal Mapping (Barré-Brisebois & Hill 2012). The de-facto
/// "correct" normal-map blend.
fn rnm(n1: [f32; 3], n2: [f32; 3]) -> [f32; 3] {
    // Shift base into the basis so that (0,0,1) maps to n1
    let t = [n1[0], n1[1], n1[2] + 1.0];
    let u = [-n2[0], -n2[1], n2[2]];
    let dot = t[0] * u[0] + t[1] * u[1] + t[2] * u[2];
    normalize([
        t[0] * dot - u[0] * t[2],
        t[1] * dot - u[1] * t[2],
        t[2] * dot - u[2] * t[2],
    ])
}

/// Partial-derivative blend — sum the (dx, dy) partial derivatives of each
/// normal and re-derive z from the result.
///
/// For a tangent-space normal `(n_x, n_y, n_z)` the corresponding height-map
/// partial derivatives are `(n_x / n_z, n_y / n_z)` (ignoring the overall
/// sign convention since we only care about matching it between inputs). Sum
/// the pairs, then the combined normal is `normalize(sum_dx, sum_dy, 1)`.
fn partial_derivative(n1: [f32; 3], n2: [f32; 3]) -> [f32; 3] {
    let d1 = deriv(n1);
    let d2 = deriv(n2);
    normalize([d1[0] + d2[0], d1[1] + d2[1], 1.0])
}

fn deriv(n: [f32; 3]) -> [f32; 2] {
    // Clamp z so we never divide by zero on a sideways normal.
    let nz = n[2].abs().max(1e-4);
    [n[0] / nz, n[1] / nz]
}

/// Naive componentwise average with re-normalisation.
fn linear(n1: [f32; 3], n2: [f32; 3]) -> [f32; 3] {
    normalize([
        (n1[0] + n2[0]) * 0.5,
        (n1[1] + n2[1]) * 0.5,
        (n1[2] + n2[2]) * 0.5,
    ])
}

#[cfg(test)]
#[path = "normal_combine_tests.rs"]
mod tests;
