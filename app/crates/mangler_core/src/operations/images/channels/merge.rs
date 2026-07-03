//! Channel merge operation.
//!
//! Recombines four separate images (one per channel) into a single
//! 4-channel RGBA FloatImage. Each input's first channel is used as
//! the channel value.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Minimum pixel count before the merge is parallelized over rows.
const PARALLEL_PIXELS: usize = 1 << 16;

/// Per-row view of one source image: the raw row slice (empty when the row is
/// out of bounds), how many leading pixels are in bounds, the channel count,
/// whether luminance reduction applies, and the out-of-bounds fill value.
struct SourceRow<'a> {
    row: &'a [f32],
    in_w: usize,
    ch: usize,
    luma: bool,
    fill: f32,
}

impl<'a> SourceRow<'a> {
    /// Builds the row view for source `img` at output row `y`, clipped to the
    /// output width `out_w`. `fill` is used for out-of-bounds pixels.
    fn new(img: &'a FloatImage, y: u32, out_w: usize, fill: f32) -> Self {
        if y < img.height() && img.width() > 0 {
            let ch = img.channels() as usize;
            let iw = img.width() as usize;
            let start = y as usize * iw * ch;
            Self { row: &img.as_raw()[start..start + iw * ch], in_w: iw.min(out_w), ch, luma: ch >= 3, fill }
        } else {
            Self { row: &[], in_w: 0, ch: 1, luma: false, fill }
        }
    }

    /// Scalar value at column `x`: Rec. 601 luminance for RGB(A) sources,
    /// first channel otherwise, or the fill value out of bounds.
    #[inline]
    fn value(&self, x: usize) -> f32 {
        if x < self.in_w {
            let px = &self.row[x * self.ch..];
            if self.luma { 0.299 * px[0] + 0.587 * px[1] + 0.114 * px[2] } else { px[0] }
        } else {
            self.fill
        }
    }
}

/// Operation that merges four channel images into a single RGBA image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelMerge {}

impl OpImageChannelMerge {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel merge".to_string(),
            description: "Merges R, G, B, A channel images into one RGBA image.".to_string(),
            help: "For every pixel of the red input, samples the red, green, blue, and alpha source images at the same coordinate and composes a new RGBA pixel from their luminance values. Each source image is reduced to a single scalar per pixel: if it has three or more channels the Rec. 601 weighted luminance 0.299 R + 0.587 G + 0.114 B is used, otherwise the first channel is taken directly.\n\nThe output size is taken from the red input; pixels outside the other sources' bounds are zero for RGB and 1 for alpha. No resizing is performed, so mismatched inputs simply get cropped/extended. Useful for round-tripping through `channels split` or building masks from arbitrary sources.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("red".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance becomes the red channel of the output."),
            Input::new("green".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance becomes the green channel of the output."),
            Input::new("blue".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance becomes the blue channel of the output."),
            Input::new("alpha".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance becomes the alpha channel of the output."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
            .with_description("RGBA image assembled from the four per-channel source images.")]
    }

    /// Merges four images by taking each one's first channel (or luminance) as an RGBA component.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let red_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let green_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let blue_converted = convert_input(inputs, 2, ValueType::Image, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data:red_data, change_id:_} = red_converted.unwrap() else { unreachable!() };
        let Value::Image{data:green_data, change_id:_} = green_converted.unwrap() else { unreachable!() };
        let Value::Image{data:blue_data, change_id:_} = blue_converted.unwrap() else { unreachable!() };
        let Value::Image{data:alpha_data, change_id:_} = alpha_converted.unwrap() else { unreachable!() };

        // Use the red channel's dimensions as the output size
        let (width, height) = red_data.dimensions();
        let w = width as usize;
        let mut out_data = vec![0.0f32; w * height as usize * 4];

        // Fill one output row from per-source row views; the source bounds
        // checks and channel dispatch are hoisted to once per row.
        let process_row = |(y, dst_row): (usize, &mut [f32])| {
            let y = y as u32;
            let red = SourceRow::new(&red_data, y, w, 0.0);
            let green = SourceRow::new(&green_data, y, w, 0.0);
            let blue = SourceRow::new(&blue_data, y, w, 0.0);
            // Alpha defaults to 1.0 for out-of-bounds pixels
            let alpha = SourceRow::new(&alpha_data, y, w, 1.0);
            for (x, dst) in dst_row.chunks_exact_mut(4).enumerate() {
                dst[0] = red.value(x);
                dst[1] = green.value(x);
                dst[2] = blue.value(x);
                dst[3] = alpha.value(x);
            }
        };

        if w > 0 {
            if w * height as usize >= PARALLEL_PIXELS {
                out_data.par_chunks_exact_mut(w * 4).enumerate().for_each(process_row);
            } else {
                out_data.chunks_exact_mut(w * 4).enumerate().for_each(process_row);
            }
        }

        let output = FloatImage::from_raw(width, height, 4, out_data).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
