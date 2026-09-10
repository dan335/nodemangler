//! Opacity blend between two normal maps.
//!
//! Linearly interpolates between two unit normals and re-normalises the
//! result. Simpler than `normal_combine` — no detail-preservation heuristics.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::pbr::{normalize, pack_normal, unpack_normal};
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Opacity-weighted blend between two normal maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrNormalBlend {}

impl OpImagePbrNormalBlend {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "normal blend".to_string(),
            description: "Linearly interpolates between two normal maps with an opacity slider; re-normalises the result.".to_string(),
            help: "Unpacks both normal maps from their 0-1 storage back to -1..1 vectors, mixes them componentwise by opacity, and re-normalises before re-packing. Produces a smooth fade between two normal fields without any detail-preservation heuristics.\n\nIf map B is a different size than A it is bilinearly sampled so it stretches to fit. For combining a detail layer over a base, prefer the normal_combine node which offers the Whiteout or RNM operator and retains overhang detail; this node is better for opacity-style fades.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("a")
                .with_description("First normal map, used fully when opacity is 0."),
            image_input("b")
                .with_description("Second normal map, used fully when opacity is 1."),
            Input::new("opacity".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Interpolation factor between normal map A and B."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Re-normalised normal map blended from A and B by opacity."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(a) = 0,
            Image(b) = 1,
            Decimal(opacity) = 2,
        }

        let opacity = opacity.clamp(0.0, 1.0);
        let (width, height) = a.dimensions();

        let sx = if b.width() > 0 { b.width() as f32 / width.max(1) as f32 } else { 1.0 };
        let sy = if b.height() > 0 { b.height() as f32 / height.max(1) as f32 } else { 1.0 };
        let mut b_buf = [0.0f32; 4];
        let b_ch = b.channels() as usize;

        let mut output = FloatImage::new(width, height, 4);
        for y in 0..height {
            for x in 0..width {
                let na = unpack_normal(a.get_pixel(x, y));
                b.bilinear_sample(x as f32 * sx, y as f32 * sy, &mut b_buf[..b_ch]);
                let nb = unpack_normal(&b_buf[..b_ch]);

                let mixed = normalize([
                    na[0] * (1.0 - opacity) + nb[0] * opacity,
                    na[1] * (1.0 - opacity) + nb[1] * opacity,
                    na[2] * (1.0 - opacity) + nb[2] * opacity,
                ]);

                output.put_pixel(x, y, &pack_normal(mixed));
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
#[path = "normal_blend_tests.rs"]
mod tests;
