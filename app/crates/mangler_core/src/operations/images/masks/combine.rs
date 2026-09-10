//! Mask combine — set-logic / arithmetic merge of two single-channel masks.
//!
//! Deliberately simpler than `blend`: no colour space, no position offset, no
//! blend-mode catalogue. Modes are the ones that matter for mask workflows
//! (AND/OR-ish, min/max, subtract, average).

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{
    OperationError, OperationResponse, OutputResponse, default_image,
};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Reduce a pixel to a scalar mask value: first channel for 1/2-ch images,
/// Rec. 709 luma for 3+ channel images.
#[inline]
fn mask_scalar(pixel: &[f32], ch: usize) -> f32 {
    if ch >= 3 {
        crate::luma::rec709(pixel[0], pixel[1], pixel[2])
    } else {
        pixel[0]
    }
}

/// Combine two masks with a chosen arithmetic / set-logic mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageMaskCombine {}

impl OpImageMaskCombine {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mask combine".to_string(),
            description: "Combine two masks with multiply, min, max, screen, subtract, or average.".to_string(),
            help: "Merges mask A with mask B into a single-channel result. Multi-channel inputs are reduced to a scalar first (Rec. 709 luma for RGB, first channel otherwise). Output size matches A; B pixels outside A's bounds contribute 0.\n\nModes:\n- multiply — a×b (AND-ish; both must be selected)\n- min / max — per-pixel minimum / maximum\n- screen — 1−(1−a)(1−b) (soft OR)\n- subtract — clamp(a−b, 0, 1)\n- average — (a+b)/2\n\n`amount` lerps the result back toward A (1 = full combine, 0 = A unchanged). Prefer this over blend when you only need set logic on masks — blend's colour-space path and position offset are unnecessary overhead for mask work.".to_string(),
        }
    }

    /// Creates inputs: a, b, mode dropdown, amount.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "a".to_string(),
                Value::Image {
                    data: default_image(),
                    change_id: get_id(),
                },
                None,
                None,
            )
            .with_description("First mask; sets the output size."),
            Input::new(
                "b".to_string(),
                Value::Image {
                    data: default_image(),
                    change_id: get_id(),
                },
                None,
                None,
            )
            .with_description("Second mask; sampled at A's coordinates (0 outside bounds)."),
            Input::new(
                "mode".to_string(),
                Value::Text("multiply".to_string()),
                Some(InputSettings::Dropdown {
                    options: vec![
                        "multiply".to_string(),
                        "min".to_string(),
                        "max".to_string(),
                        "screen".to_string(),
                        "subtract".to_string(),
                        "average".to_string(),
                    ],
                }),
                None,
            )
            .with_description("How A and B are combined (multiply/min/max/screen/subtract/average)."),
            Input::new(
                "amount".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider {
                    range: (0.0, 1.0),
                    step_by: Some(0.01),
                    clamp_to_range: true,
                }),
                None,
            )
            .with_description("Mix of the combined result vs A; 0 leaves A unchanged."),
        ]
    }

    /// Creates the single mask output.
    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "output".to_string(),
            Value::Image {
                data: default_image(),
                change_id: get_id(),
            },
            None,
        )
        .with_description("Combined single-channel mask.")]
    }

    /// Combines the two mask images.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(a) = 0,
            Image(b) = 1,
            Text(mode) = 2,
            Decimal(amount) = 3,
        }
        let amount = amount.clamp(0.0, 1.0);
        let mode = mode.to_ascii_lowercase();

        let (w, h) = a.dimensions();
        let (bw, bh) = b.dimensions();
        let a_ch = a.channels() as usize;
        let b_ch = b.channels() as usize;
        let mut output = FloatImage::new(w, h, 1);

        // The mode is a string, but it is the same string for every pixel:
        // resolve it to a function pointer once instead of running up to five
        // string comparisons per pixel. The `_` arm keeps multiply as the
        // behaviour for an unknown mode, exactly as the inline match did.
        let combine: fn(f32, f32) -> f32 = match mode.as_str() {
            "min" => |av, bv| av.min(bv),
            "max" => |av, bv| av.max(bv),
            "screen" => |av, bv| 1.0 - (1.0 - av) * (1.0 - bv),
            "subtract" => |av, bv| (av - bv).clamp(0.0, 1.0),
            "average" => |av, bv| (av + bv) * 0.5,
            // multiply (default for unknown too)
            _ => |av, bv| av * bv,
        };

        // Each output pixel depends only on the matching input pixels, so the
        // pass is order-independent and bit-identical to the serial loop.
        output.par_enumerate_pixels_mut().for_each(|(x, y, dst)| {
            let av = mask_scalar(a.get_pixel(x, y), a_ch);
            let bv = if x < bw && y < bh {
                mask_scalar(b.get_pixel(x, y), b_ch)
            } else {
                0.0
            };
            let combined = combine(av, bv);
            let out = av + amount * (combined - av);
            dst[0] = out.clamp(0.0, 1.0);
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Image {
                    data: Arc::new(output),
                    change_id: get_id(),
                },
            }],
        })
    }
}

#[cfg(test)]
#[path = "combine_tests.rs"]
mod tests;
