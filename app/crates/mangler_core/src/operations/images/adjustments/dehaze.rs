//! Dehaze adjustment operation for images.
//!
//! Heuristic haze removal based on the dark-channel prior (He, Sun & Tang 2009),
//! simplified with a global atmospheric-light estimate and no soft-matting
//! refinement. It reduces the milky, low-contrast veil of atmospheric haze by
//! estimating how much airlight has been mixed into each pixel and inverting it.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Dehaze operation that removes atmospheric haze using a simplified dark-channel prior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDehaze{}

impl OpImageAdjustmentDehaze {
    /// Returns the node metadata (name, description, help) for the dehaze operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "dehaze".to_string(),
            description: "Removes atmospheric haze to restore contrast and colour.".to_string(),
            help: "Heuristic haze removal based on the dark-channel prior (He, Sun & Tang 2009), \
simplified with a global atmospheric-light estimate and no soft-matting refinement — not a \
calibrated physical model.\n\n\
The dark-channel prior observes that in a haze-free outdoor image most small patches contain at \
least one colour channel with a very low value; haze lifts that minimum, so the size of the \
per-patch minimum estimates how much airlight (the bright atmospheric veil) has been blended into \
each pixel. This node computes the dark channel (per-pixel minimum across the red, green and blue \
channels), applies a square min-filter over it, estimates a single global atmospheric light A, \
derives a transmission map t (how much of the original scene survives the haze), and inverts the \
haze model J = (I - A) / t + A to recover the clearer image.\n\n\
'amount' (0..1) scales how aggressively the haze is removed: 0 leaves the image untouched (early \
identity), 1 removes almost all of the estimated haze. Because it is a heuristic global estimate, \
high amounts can over-darken skies or introduce colour casts.\n\n\
'radius' is the half-width in pixels (authored at a 1024px reference and scaled to the actual \
image resolution) of the min-filter window used to build the dark channel. Larger radii give a \
smoother, more robust haze estimate but blur fine transmission detail and can leave halos around \
sharp depth edges (there is no matting refinement to fix them). Grayscale images (fewer than 3 \
channels) have no chroma dark channel and pass through unchanged.".to_string(),
        }
    }

    /// Creates the input ports: source image, haze-removal amount, and dark-channel filter radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to remove haze from (needs 3+ channels for an effect)."),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How aggressively to remove estimated haze; 0 is identity, 1 is maximal."),
            Input::new("radius".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None)
                .with_description("Dark-channel min-filter radius in pixels (at a 1024px reference), scaled to resolution."),
        ]
    }

    /// Creates the output port: the dehazed image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with estimated atmospheric haze removed, alpha preserved."),
        ]
    }

    /// Executes the dehaze operation using a simplified dark-channel prior.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let image_converted  = convert_input(inputs, 0, ValueType::Image,   &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // Return if any input failed to convert.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Extract values.
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let mut result = (*data).clone();
        let ch = result.channels() as usize;

        // Early identity: no removal requested, or not enough colour channels for a dark channel.
        if amount == 0.0 || ch < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![ OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } } ],
            });
        }

        let (w, h) = result.dimensions();
        let wu = w as usize;
        let hu = h as usize;
        // Which channel index (if any) is alpha — never touched.
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Convert the authored px@1024 radius to this image's resolution; floor at 1 pixel.
        let r = crate::operations::scale_to_resolution(radius as f32, w, h).round().max(1.0) as i32;

        // --- Dark channel: per-pixel minimum over the first three colour channels. ---
        let mut dark = vec![0.0f32; wu * hu];
        for y in 0..hu {
            for x in 0..wu {
                let px = result.get_pixel(x as u32, y as u32);
                let m = px[0].min(px[1]).min(px[2]);
                dark[y * wu + x] = m;
            }
        }

        // --- Min-filter the dark channel over a (2r+1) square window, edges clamped. ---
        let mut darkmin = vec![0.0f32; wu * hu];
        for y in 0..hu {
            for x in 0..wu {
                let mut m = f32::INFINITY;
                let y0 = (y as i32 - r).max(0) as usize;
                let y1 = ((y as i32 + r) as usize).min(hu - 1);
                let x0 = (x as i32 - r).max(0) as usize;
                let x1 = ((x as i32 + r) as usize).min(wu - 1);
                for yy in y0..=y1 {
                    let row = yy * wu;
                    for xx in x0..=x1 {
                        let v = dark[row + xx];
                        if v < m { m = v; }
                    }
                }
                darkmin[y * wu + x] = m;
            }
        }

        // --- Atmospheric light A (global estimate). ---
        // Prefer averaging the RGB of the brightest-darkmin pixels (the haziest, most airlight-dominated
        // region) — the top ~0.1% by darkmin. Fall back to the per-channel image maximum if empty.
        let total = wu * hu;
        // Number of pixels forming the top 0.1% (at least 1).
        let top_n = ((total as f32 * 0.001).round() as usize).max(1);
        // Collect (darkmin, index) and select the largest `top_n` by darkmin.
        let mut order: Vec<usize> = (0..total).collect();
        // Partial ordering: sort indices by descending darkmin. `sort_unstable_by` is fine here.
        order.sort_unstable_by(|&a, &b| darkmin[b].partial_cmp(&darkmin[a]).unwrap_or(std::cmp::Ordering::Equal));

        let mut a_r;
        let mut a_g;
        let mut a_b;
        {
            // Average the colour of the top_n brightest-darkmin pixels.
            let mut sr = 0.0f32;
            let mut sg = 0.0f32;
            let mut sb = 0.0f32;
            for &idx in order.iter().take(top_n) {
                let x = (idx % wu) as u32;
                let y = (idx / wu) as u32;
                let px = result.get_pixel(x, y);
                sr += px[0];
                sg += px[1];
                sb += px[2];
            }
            let n = top_n as f32;
            a_r = sr / n;
            a_g = sg / n;
            a_b = sb / n;
        }
        // Per-channel floor to avoid division by (near-)zero atmospheric light.
        a_r = a_r.max(0.1);
        a_g = a_g.max(0.1);
        a_b = a_b.max(0.1);
        let a_channels = [a_r, a_g, a_b];

        // Luma of the atmospheric light drives the transmission normalisation.
        let a_luma = (0.2126 * a_r + 0.7152 * a_g + 0.0722 * a_b).max(0.1);

        // --- Recover the scene per pixel: J_c = (I_c - A_c)/t + A_c. ---
        let amount_f = amount as f32;
        for y in 0..hu {
            for x in 0..wu {
                // Transmission: how much of the original scene survives the haze at this pixel.
                let t = (1.0 - amount_f * 0.95 * (darkmin[y * wu + x] / a_luma)).clamp(0.1, 1.0);
                let px = result.get_pixel(x as u32, y as u32).to_vec();
                let mut out = px.clone();
                for c in 0..color_ch.min(3) {
                    // Invert the haze model; J is intentionally left unclamped.
                    out[c] = (px[c] - a_channels[c]) / t + a_channels[c];
                }
                result.put_pixel(x as u32, y as u32, &out);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![ OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } } ],
        })
    }
}

#[cfg(test)]
#[path = "dehaze_tests.rs"]
mod tests;
