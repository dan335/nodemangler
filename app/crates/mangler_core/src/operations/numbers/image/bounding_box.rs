//! Axis-aligned bounding box of an image's content.
//!
//! Finds the smallest rectangle that contains every pixel whose significance
//! exceeds a threshold, reporting its top-left corner and size as integers.
//! Significance is the alpha channel when the image has one, otherwise
//! luminance — so a shape on transparency and a shape on black both measure.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports the bounding box of content above a threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageBoundingBox {}

impl OpNumberImageBoundingBox {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bounding box".to_string(),
            description: "Finds the box enclosing content above a threshold.".to_string(),
            help: "Scans the image for pixels whose significance exceeds the threshold and outputs the top-left corner (x, y) and the width and height of the smallest axis-aligned rectangle that contains them all.\n\nSignificance is the alpha channel when the image has one (2 or 4 channels), otherwise Rec. 601 luminance. If no pixel qualifies, every output is 0. Use it to auto-crop a shape, or to drive a crop/resize node so the graph trims to whatever content is present.".to_string(),
        }
    }

    /// Creates the input ports: the image and a significance threshold.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Image whose content is measured."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("A pixel counts as content when its significance (alpha, or luminance) exceeds this."),
        ]
    }

    /// Creates the output ports: corner and size of the bounding box.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Integer(0), None)
                .with_description("Left edge of the bounding box in pixels."),
            Output::new("y".to_string(), Value::Integer(0), None)
                .with_description("Top edge of the bounding box in pixels."),
            Output::new("width".to_string(), Value::Integer(0), None)
                .with_description("Bounding box width in pixels (0 if no content)."),
            Output::new("height".to_string(), Value::Integer(0), None)
                .with_description("Bounding box height in pixels (0 if no content)."),
        ]
    }

    /// Executes the bounding-box measurement.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(threshold) = 1,
        }

        let (w, h) = data.dimensions();
        let ch = data.channels();

        let (mut minx, mut miny) = (u32::MAX, u32::MAX);
        let (mut maxx, mut maxy) = (0u32, 0u32);
        let mut found = false;

        for y in 0..h {
            for x in 0..w {
                let px = data.get_pixel(x, y);
                let sig = if ch == 2 { px[1] } else if ch == 4 { px[3] } else { super::pixel_luma(px) };
                if sig > threshold {
                    found = true;
                    minx = minx.min(x);
                    miny = miny.min(y);
                    maxx = maxx.max(x);
                    maxy = maxy.max(y);
                }
            }
        }

        let (ox, oy, ow, oh) = if found {
            (minx as i32, miny as i32, (maxx - minx + 1) as i32, (maxy - miny + 1) as i32)
        } else {
            (0, 0, 0, 0)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Integer(ox) },
                OutputResponse { value: Value::Integer(oy) },
                OutputResponse { value: Value::Integer(ow) },
                OutputResponse { value: Value::Integer(oh) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "bounding_box_tests.rs"]
mod tests;
