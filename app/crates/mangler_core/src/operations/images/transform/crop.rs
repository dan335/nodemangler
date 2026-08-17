//! Crop operation for extracting a rectangular sub-region from an image.

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

/// A crop region resolved to integer source pixels.
///
/// `x`/`y` are the inclusive origin; `w`/`h` are the size. Always at least
/// 1×1 and always inside an `iw`×`ih` image (a 0×0 source is treated as 1×1
/// so the arithmetic stays well-formed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelCrop {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl PixelCrop {
    /// Convert this pixel region back to origin-size fractions of `iw`×`ih`.
    pub fn to_norm(self, iw: u32, ih: u32) -> [f32; 4] {
        let iw = iw.max(1) as f32;
        let ih = ih.max(1) as f32;
        [
            self.x as f32 / iw,
            self.y as f32 / ih,
            self.w as f32 / iw,
            self.h as f32 / ih,
        ]
    }
}

/// Resolve a fractional origin/size crop — and an optional integer W:H lock —
/// to the pixel rectangle `run()` actually copies.
///
/// Far edges are rounded from `origin + size` (so abutting crops share an
/// edge). `ratio_w`/`ratio_h` both `> 0` lock the result to that pixel
/// aspect: the largest same-ratio rectangle that fits inside the requested
/// region, centered when the request is off-ratio. Either side `<= 0` is
/// free, and the unconstrained path is the historical rounding.
pub fn resolve_crop(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    ratio_w: i32,
    ratio_h: i32,
    iw: u32,
    ih: u32,
) -> PixelCrop {
    let iw = iw.max(1) as i64;
    let ih = ih.max(1) as i64;
    // NaN casts to 0 in Rust, so a garbage fraction degrades to the origin.
    let x0 = ((x * iw as f32).round() as i64).clamp(0, iw - 1);
    let y0 = ((y * ih as f32).round() as i64).clamp(0, ih - 1);
    // Clamp the far edge to at least one pixel past the origin and at most
    // the image edge, so an off-origin crop clips at the right/bottom edge
    // instead of edge-replicating past-the-edge pixels.
    let x1 = ((((x + width) * iw as f32).round()) as i64).clamp(x0 + 1, iw);
    let y1 = ((((y + height) * ih as f32).round()) as i64).clamp(y0 + 1, ih);

    if ratio_w <= 0 || ratio_h <= 0 {
        return PixelCrop { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }

    let avail_w = x1 - x0;
    let avail_h = y1 - y0;
    let rw = ratio_w as i64;
    let rh = ratio_h as i64;
    let (cw, ch) = fit_aspect_size(avail_w, avail_h, rw, rh);
    // Center the fitted rect in the request. When the request is already
    // on-ratio, `cw == avail_w` (and same for h) so this is a no-op — a
    // body-drag that wrote this exact box is not recentered next tick.
    let x = x0 + (avail_w - cw) / 2;
    let y = y0 + (avail_h - ch) / 2;
    PixelCrop { x, y, w: cw, h: ch }
}

/// Largest `cw`×`ch` that fits in `avail_w`×`avail_h` and matches `rw`:`rh`
/// as closely as integer pixels allow. Prefers the candidate with the
/// smaller cross-multiply error; ties go to the larger area, then to
/// keeping the available width.
fn fit_aspect_size(avail_w: i64, avail_h: i64, rw: i64, rh: i64) -> (i64, i64) {
    let avail_w = avail_w.max(1);
    let avail_h = avail_h.max(1);
    let rw = rw.max(1);
    let rh = rh.max(1);

    // Full-width candidate: pick the height that best matches the ratio.
    let ch_from_w = ((avail_w as f64 * rh as f64) / rw as f64).round() as i64;
    let a = (avail_w, ch_from_w.clamp(1, avail_h));
    // Full-height candidate: pick the width that best matches the ratio.
    let cw_from_h = ((avail_h as f64 * rw as f64) / rh as f64).round() as i64;
    let b = (cw_from_h.clamp(1, avail_w), avail_h);

    let err = |c: (i64, i64)| (c.0 * rh - c.1 * rw).abs();
    let ea = err(a);
    let eb = err(b);
    if ea < eb {
        a
    } else if eb < ea {
        b
    } else if a.0 * a.1 >= b.0 * b.1 {
        a
    } else {
        b
    }
}

/// Crops an image to a rectangular sub-region defined by position (x, y) and size (width, height),
/// all expressed as 0-1 fractions of the source image's dimensions.
///
/// Working in fractions makes the node resolution-independent: the same values keep framing the
/// same part of the picture whether the source is 512px or 6000px wide.
///
/// Inputs are clamped so the region always keeps at least one pixel and never extends past the
/// right or bottom edge. An optional integer W:H pair (`aspect w` / `aspect h`) locks the
/// crop's pixel aspect; both 0 leaves it free. Outputs the cropped image along with its
/// actual pixel width and height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformCrop {}

impl OpImageTransformCrop {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "crop".to_string(),
            description: "Extracts a rectangular region using fractional position and size, with an optional pixel aspect lock.".to_string(),
            help: "Copies a rectangular sub-region of the source image starting at (x, y) with the requested width and height; the result is a new image whose pixel dimensions are emitted on the width/height outputs.\n\nAll four parameters are 0-1 fractions of the source image's size, not pixels: x = 0.25 starts a quarter of the way across, width = 0.5 keeps half the image's width. That makes the crop resolution-independent — the same values frame the same part of the picture at any input size — so swapping a 1024px source for a 6000px one needs no re-tuning.\n\n`aspect w` and `aspect h` optionally lock the crop to a pixel W:H (16 and 9 = widescreen, 1 and 1 = square). Both 0 (the default) leaves the crop free. When locked, the largest same-ratio rectangle that fits inside the requested region is used, centered if the request itself is off-ratio. Wire the `dimensions` node's width/height here to lock to the source's own shape.\n\nFractions are converted to pixel edges by rounding, then clamped to the source's valid range: the region always keeps at least one pixel and never extends past the right or bottom edge, so an off-origin crop clips instead of running off the image. No resampling is performed; channel count is preserved exactly.".to_string(),
        }
    }

    /// Creates the default inputs: source image, x/y position, width/height of the crop
    /// region (0-1 fractions of the source), and an optional integer W:H aspect lock.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to crop."),
            Input::new("x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Left edge of the crop region as a 0-1 fraction of image width (0.25 = a quarter across). Resolution-independent."),
            Input::new("y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Top edge of the crop region as a 0-1 fraction of image height. Resolution-independent."),
            Input::new("width".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Width of the crop region as a 0-1 fraction of image width (1.0 = full image); clipped at the right edge."),
            Input::new("height".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Height of the crop region as a 0-1 fraction of image height (1.0 = full image); clipped at the bottom edge."),
            Input::new("aspect w".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 100000.0)), speed: None }), None)
                .with_description("Width half of an optional pixel aspect lock (16 of 16:9). 0 = free. Both sides must be > 0 to lock."),
            Input::new("aspect h".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 100000.0)), speed: None }), None)
                .with_description("Height half of an optional pixel aspect lock (9 of 16:9). 0 = free. Both sides must be > 0 to lock."),
        ]
    }

    /// Creates the default outputs: cropped image, and its width and height as integers.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Cropped sub-region of the source image."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Actual cropped image width in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Actual cropped image height in pixels."),
        ]
    }

    /// Executes the crop operation.
    ///
    /// Converts the fractional x, y, width, and height into pixel edges, applies
    /// the optional aspect lock, and clamps them to the source image bounds before cropping.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let width_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let height_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let ratio_w_converted = convert_input(inputs, 5, ValueType::Integer, &mut input_errors);
        let ratio_h_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(width) = width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(ratio_w) = ratio_w_converted.unwrap() else { unreachable!() };
        let Value::Integer(ratio_h) = ratio_h_converted.unwrap() else { unreachable!() };

        // run node
        // The parameters are 0-1 fractions of the source size, so resolve them
        // against the actual image to get pixel edges. Rounding the far edge
        // from (origin + size) rather than rounding the size on its own means
        // abutting crops share an edge exactly instead of gapping or
        // overlapping by a pixel. A positive aspect w/h pair then fits the
        // largest same-ratio rectangle inside that region.
        let crop = resolve_crop(x, y, width, height, ratio_w, ratio_h, data.width(), data.height());

        let cx = crop.x as u32;
        let cy = crop.y as u32;
        let cw = crop.w as u32;
        let ch = crop.h as u32;

        // Copy the crop region into a new FloatImage, preserving channel count
        let mut output = crate::float_image::FloatImage::new(cw, ch, data.channels());
        for oy in 0..ch {
            for ox in 0..cw {
                let sx = (cx + ox).min(data.width() - 1);
                let sy = (cy + oy).min(data.height() - 1);
                output.put_pixel(ox, oy, data.get_pixel(sx, sy));
            }
        }

        let value_width = Value::Integer(output.width() as i32);
        let value_height = Value::Integer(output.height() as i32);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(output), change_id:get_id() }},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}

#[cfg(test)]
#[path = "crop_tests.rs"]
mod tests;
