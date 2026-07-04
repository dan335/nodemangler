//! ASCII art stylization.
//!
//! Tiles the image into square cells of `cell size` pixels. For each cell we
//! compute the average luminance, quantize it to one of ten glyph bins, and
//! stamp the matching 8×8 ASCII bitmap (scaled via nearest-neighbor) back
//! into the cell. Lighter cells produce sparse glyphs (space, dot), darker
//! cells produce dense glyphs (hash, @).
//!
//! Glyphs are embedded as 8×8 monochrome bitmaps. No font rendering is
//! involved; the glyph shapes approximate the familiar ASCII characters
//! `" .:-=+*#%@"` closely enough to read as the intended look while keeping
//! the filter self-contained.
//!
//! The output keeps the input's color channel count but is strictly binary
//! per color channel: glyph pixels are black (ink), the rest of the cell is
//! white (paper). Alpha is preserved.

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

/// Ten 8×8 glyph bitmaps in order of increasing ink density.
///
/// Each glyph is a `[u8; 8]`: one byte per row. Bit 7 of each byte is the
/// leftmost column. The ordered set roughly corresponds to the ASCII ramp
/// `" .:-=+*#%@"` — sparser glyphs for bright cells, denser glyphs for dark
/// cells.
static GLYPHS: [[u8; 8]; 10] = [
    // ' '  — blank (brightest)
    [0, 0, 0, 0, 0, 0, 0, 0],
    // '.'  — single pixel near the baseline
    [0, 0, 0, 0, 0, 0, 0b00011000, 0b00011000],
    // ':'  — two stacked dots
    [0, 0b00011000, 0b00011000, 0, 0, 0b00011000, 0b00011000, 0],
    // '-'  — horizontal bar at the center
    [0, 0, 0, 0b00111100, 0b00111100, 0, 0, 0],
    // '='  — double horizontal
    [0, 0, 0b00111100, 0, 0b00111100, 0, 0, 0],
    // '+'  — plus sign
    [0, 0b00011000, 0b00011000, 0b01111110, 0b01111110, 0b00011000, 0b00011000, 0],
    // '*'  — asterisk / star
    [0, 0b00011000, 0b01011010, 0b00111100, 0b00111100, 0b01011010, 0b00011000, 0],
    // '#'  — hash grid
    [0b00100100, 0b01111110, 0b00100100, 0b01111110, 0b01111110, 0b00100100, 0b01111110, 0b00100100],
    // '%'  — dense X-shape
    [0b11000011, 0b01100110, 0b00111100, 0b00011000, 0b00011000, 0b00111100, 0b01100110, 0b11000011],
    // '@'  — nearly filled block (darkest)
    [0b00111100, 0b01111110, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b01111110, 0b00111100],
];

/// ASCII-style stylization filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentAscii {}

impl OpImageAdjustmentAscii {
    /// Returns the node metadata for the ASCII filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ascii".to_string(),
            description: "Tiles the image with 10 ASCII-style glyphs chosen by cell brightness.".to_string(),
            help: "Each square cell's mean luminance picks one of ten embedded 8x8 bitmaps from the density ramp ` .:-=+*#%@`, scaled with nearest-neighbor into the cell. No font rendering is involved, so the filter stays self-contained.\n\nOutput is strictly binary per color channel (ink vs paper) and alpha is preserved. The invert toggle swaps ink and paper so glyphs render bright on a dark background. Smaller cell sizes lose glyph detail since the source bitmaps are 8x8.".to_string(),
        }
    }

    /// Creates input ports: image, cell size in pixels, and invert flag.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose brightness drives the ASCII glyph selection."),
            // Pixel size of each ASCII cell; glyphs are 8×8 so smaller cells lose detail
            Input::new("cell size".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (2.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Pixel size of each square ASCII cell; larger cells yield chunkier glyphs."),
            // Invert: false = dark glyph on bright paper (default); true = bright glyph on dark ink
            Input::new("invert".to_string(), Value::Bool(false), None, None)
                .with_description("Invert ink and paper so glyphs render bright on a dark background."),
        ]
    }

    /// Creates the output port: the ASCII-ified image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("ASCII-stylized image with glyphs stamped into brightness-ranked cells."),
        ]
    }

    /// Runs the ASCII filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let cell_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let invert_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(cell) = cell_converted.unwrap() else { unreachable!() };
        let Value::Bool(invert) = invert_converted.unwrap() else { unreachable!() };

        let cell = cell.max(2) as u32;
        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        let mut out = FloatImage::new(width, height, ch as u32);

        // Walk cell by cell so we only compute the average luminance once per cell
        let mut cy0 = 0u32;
        while cy0 < height {
            let cy1 = (cy0 + cell).min(height);
            let mut cx0 = 0u32;
            while cx0 < width {
                let cx1 = (cx0 + cell).min(width);

                // Mean luminance inside this cell (colour channels only)
                let mut sum = 0.0f32;
                let mut n = 0u32;
                for yy in cy0..cy1 {
                    for xx in cx0..cx1 {
                        let p = data.get_pixel(xx, yy);
                        let l = if ch >= 3 {
                            0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                        } else {
                            p[0]
                        };
                        sum += l;
                        n += 1;
                    }
                }
                let lum = if n > 0 { sum / n as f32 } else { 0.0 };
                // Map brightness to glyph bin. Without invert: bright cells use
                // sparse glyphs (index 0), dark cells use dense glyphs (index 9).
                let shade = if invert { lum } else { 1.0 - lum };
                let glyph_idx = (shade.clamp(0.0, 1.0) * (GLYPHS.len() - 1) as f32)
                    .round() as usize;
                let glyph = &GLYPHS[glyph_idx.min(GLYPHS.len() - 1)];

                // Stamp the 8×8 glyph into the cell via nearest-neighbor
                let cw = cx1 - cx0;
                let cht = cy1 - cy0;
                for yy in cy0..cy1 {
                    // Map cell-local y into the glyph's 0..8 row space
                    let by = (((yy - cy0) * 8) / cht.max(1)) as usize;
                    let row_bits = glyph[by.min(7)];
                    for xx in cx0..cx1 {
                        let bx = (((xx - cx0) * 8) / cw.max(1)) as usize;
                        // Bit 7 is leftmost column
                        let on = (row_bits >> (7 - bx.min(7))) & 1 == 1;

                        // Ink (glyph pixel) vs paper (rest of cell).
                        // When invert is true, ink becomes bright and paper stays dark.
                        let v = if invert {
                            if on { 1.0 } else { 0.0 }
                        } else if on { 0.0 } else { 1.0 };

                        let src = data.get_pixel(xx, yy);
                        let mut pixel = [0.0f32; 4];
                        for val in pixel.iter_mut().take(ch.min(3)) { *val = v; }
                        if ch == 2 || ch == 4 { pixel[ch - 1] = src[ch - 1]; }
                        out.put_pixel(xx, yy, &pixel[..ch]);
                    }
                }

                cx0 = cx1;
            }
            cy0 = cy1;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "ascii_tests.rs"]
mod tests;
