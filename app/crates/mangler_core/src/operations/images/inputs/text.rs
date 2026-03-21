//! Text-to-image rendering operation.
//!
//! Renders one or more lines of text onto a grayscale image using the embedded
//! Manrope Regular font via `ab_glyph`. Supports:
//! - Multi-line text via `\n` and optional automatic word-wrapping
//! - Horizontal alignment (Left, Center, Right) relative to the anchor point
//! - Vertical alignment (Top, Middle, Bottom) of the text block relative to the anchor
//! - Adjustable letter spacing and line spacing multiplier
//! - Arbitrary rotation of the text block around the anchor point

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{DynamicImage, GrayImage};
use imageproc::geometric_transformations::{rotate_about_center, Interpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType, TextHAlign, TextVAlign};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Bytes of the embedded Manrope Regular font used for text rendering.
static FONT_BYTES: &[u8] = include_bytes!("../../../../assets/Manrope-Regular.ttf");

/// Operation that renders a text string onto a grayscale image.
///
/// Text is drawn as white pixels on a black background, making the output
/// suitable as a mask or stencil in downstream blend/composite nodes.
/// Multi-line layout is driven by `\n` in the text and optional word-wrapping;
/// alignment and spacing are fully controllable. An optional rotation is applied
/// around the anchor point after rasterisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputText {}

impl OpImageInputText {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from text".to_string(),
            description: "Renders a text string to a grayscale image.".to_string(),
        }
    }

    /// Creates the input definitions:
    /// - `text` — the string to render (supports `\n` for explicit line breaks)
    /// - `font_size` — size in pixels (default 64)
    /// - `image_width` / `image_height` — output canvas size in pixels
    /// - `x_position` / `y_position` — normalised (0–1) anchor for text placement
    /// - `letter_spacing` — extra pixels added between glyphs (can be negative)
    /// - `line_spacing` — multiplier on line height between stacked lines (1.0 = tight)
    /// - `wrap_width` — word-wrap column in pixels (0 = no wrapping)
    /// - `h_align` — horizontal alignment of lines relative to the anchor x
    /// - `v_align` — vertical alignment of the text block relative to the anchor y
    /// - `rotation` — clockwise rotation in degrees around the anchor point
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text("Hello".to_string()), Some(InputSettings::MultiLineText), None),
            Input::new(
                "font_size".to_string(),
                Value::Decimal(64.0),
                Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }),
                None,
            ),
            Input::new(
                "image_width".to_string(),
                Value::Integer(512),
                Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }),
                None,
            ),
            Input::new(
                "image_height".to_string(),
                Value::Integer(512),
                Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }),
                None,
            ),
            Input::new(
                "x_position".to_string(),
                Value::Decimal(0.5),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }),
                None,
            ),
            Input::new(
                "y_position".to_string(),
                Value::Decimal(0.5),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }),
                None,
            ),
            Input::new(
                "letter_spacing".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::DragValue { clamp: Some((-100.0, 500.0)), speed: None }),
                None,
            ),
            Input::new(
                "line_spacing".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::DragValue { clamp: Some((0.0, 10.0)), speed: None }),
                None,
            ),
            Input::new(
                "wrap_width".to_string(),
                Value::Integer(0),
                Some(InputSettings::DragValue { clamp: Some((0.0, 10000.0)), speed: None }),
                None,
            ),
            Input::new("h_align".to_string(), Value::TextHAlign(TextHAlign::Center), None, None),
            Input::new("v_align".to_string(), Value::TextVAlign(TextVAlign::Middle), None, None),
            Input::new(
                "rotation".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (-180.0, 180.0), step_by: None, clamp_to_range: true }),
                None,
            ),
        ]
    }

    /// Creates the output definitions: the rendered grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new(
                "output".to_string(),
                Value::Image { data: default_image(), change_id: get_id() },
                None,
            ),
        ]
    }

    /// Executes the operation:
    ///
    /// 1. Splits the input text on `\n` and optionally word-wraps each segment.
    /// 2. Measures each line's pixel width and stacks lines with the requested spacing.
    /// 3. Positions the text block at the anchor using the chosen alignment.
    /// 4. Rasterises every glyph into a `GrayImage`.
    /// 5. If `rotation` is non-zero, renders into a padded temp image, rotates it
    ///    around the text block's centre via `rotate_about_center`, then blits onto
    ///    the final canvas at the anchor position.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_c        = convert_input(inputs, 0,  ValueType::Text,      &mut input_errors);
        let font_size_c   = convert_input(inputs, 1,  ValueType::Decimal,   &mut input_errors);
        let width_c       = convert_input(inputs, 2,  ValueType::Integer,   &mut input_errors);
        let height_c      = convert_input(inputs, 3,  ValueType::Integer,   &mut input_errors);
        let x_pos_c       = convert_input(inputs, 4,  ValueType::Decimal,   &mut input_errors);
        let y_pos_c       = convert_input(inputs, 5,  ValueType::Decimal,   &mut input_errors);
        let letter_sp_c   = convert_input(inputs, 6,  ValueType::Decimal,   &mut input_errors);
        let line_sp_c     = convert_input(inputs, 7,  ValueType::Decimal,   &mut input_errors);
        let wrap_w_c      = convert_input(inputs, 8,  ValueType::Integer,   &mut input_errors);
        let h_align_c     = convert_input(inputs, 9,  ValueType::TextHAlign, &mut input_errors);
        let v_align_c     = convert_input(inputs, 10, ValueType::TextVAlign, &mut input_errors);
        let rotation_c    = convert_input(inputs, 11, ValueType::Decimal,   &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Text(text)             = text_c.unwrap()       else { unreachable!() };
        let Value::Decimal(font_size)     = font_size_c.unwrap()   else { unreachable!() };
        let Value::Integer(img_width)     = width_c.unwrap()       else { unreachable!() };
        let Value::Integer(img_height)    = height_c.unwrap()      else { unreachable!() };
        let Value::Decimal(x_pos)         = x_pos_c.unwrap()       else { unreachable!() };
        let Value::Decimal(y_pos)         = y_pos_c.unwrap()       else { unreachable!() };
        let Value::Decimal(letter_sp)     = letter_sp_c.unwrap()   else { unreachable!() };
        let Value::Decimal(line_sp)       = line_sp_c.unwrap()     else { unreachable!() };
        let Value::Integer(wrap_width)    = wrap_w_c.unwrap()      else { unreachable!() };
        let Value::TextHAlign(h_align)    = h_align_c.unwrap()     else { unreachable!() };
        let Value::TextVAlign(v_align)    = v_align_c.unwrap()     else { unreachable!() };
        let Value::Decimal(rotation_deg)  = rotation_c.unwrap()    else { unreachable!() };

        let img_width   = img_width.max(1) as u32;
        let img_height  = img_height.max(1) as u32;
        let font_size   = font_size.max(1.0);
        let letter_sp   = letter_sp;
        let line_sp     = line_sp.max(0.0);
        let wrap_px     = wrap_width.max(0) as f32;

        // Load the embedded font and create a scaled instance.
        let font = FontArc::try_from_slice(FONT_BYTES).map_err(|e| OperationError {
            input_errors: vec![],
            node_error: Some(format!("Failed to load embedded font: {e}")),
        })?;
        let scale  = PxScale::from(font_size);
        let scaled = font.as_scaled(scale);

        let ascent      = scaled.ascent();
        let descent     = scaled.descent(); // negative in ab_glyph
        let line_height = ascent - descent;
        // Vertical distance from one baseline to the next.
        let step        = line_height * line_sp;

        // --- Parse text into display lines (respecting \n and optional word-wrap) ---
        let display_lines: Vec<String> = text
            .lines()
            .flat_map(|seg| wrap_line(seg, &scaled, letter_sp, wrap_px))
            .collect();

        let n_lines = display_lines.len();

        // Measure each line's pixel width.
        let line_widths: Vec<f32> = display_lines
            .iter()
            .map(|l| measure_line_width(l, &scaled, letter_sp))
            .collect();

        // Total height of the text block (top of ascent on line 0 to bottom of descent on last line).
        let total_block_height = if n_lines == 0 {
            0.0
        } else {
            line_height + (n_lines as f32 - 1.0) * step
        };

        let anchor_x = x_pos * img_width as f32;
        let anchor_y = y_pos * img_height as f32;

        let use_rotation = rotation_deg.abs() > f32::EPSILON;

        // When rotating we render into a padded square temp image centred at the anchor,
        // then rotate it about its own centre (= the anchor) and blit back.
        // Without rotation we render directly into the full canvas-sized image.
        let (temp_w, temp_h, temp_cx, temp_cy): (u32, u32, f32, f32) = if use_rotation {
            let max_line_w = line_widths.iter().cloned().fold(0.0f32, f32::max);

            // Compute the text block's extent relative to the anchor for each axis.
            // This accounts for alignment: Left-aligned text extends rightward from the
            // anchor, so the rightmost corner is much farther than half the text width.
            let (x_min, x_max) = match h_align {
                TextHAlign::Left   => (0.0_f32, max_line_w),
                TextHAlign::Center => (-max_line_w / 2.0, max_line_w / 2.0),
                TextHAlign::Right  => (-max_line_w, 0.0_f32),
            };
            let (y_min, y_max) = match v_align {
                TextVAlign::Top    => (0.0_f32, total_block_height),
                TextVAlign::Middle => (-total_block_height / 2.0, total_block_height / 2.0),
                TextVAlign::Bottom => (-total_block_height, 0.0_f32),
            };

            // Max distance from the anchor to any corner of the text block.
            // After rotation by any angle, no pixel can exceed this radius from the anchor.
            let corners = [(x_min, y_min), (x_max, y_min), (x_min, y_max), (x_max, y_max)];
            let max_radius = corners
                .iter()
                .map(|(x, y)| (x * x + y * y).sqrt())
                .fold(1.0_f32, f32::max);

            let sz = (max_radius * 2.0).ceil() as u32 + 8;
            let sz = sz.max(4);
            (sz, sz, sz as f32 / 2.0, sz as f32 / 2.0)
        } else {
            (img_width, img_height, anchor_x, anchor_y)
        };

        // Top of the text block in temp-image coordinates.
        let temp_y_block_top = match v_align {
            TextVAlign::Top    => temp_cy,
            TextVAlign::Middle => temp_cy - total_block_height / 2.0,
            TextVAlign::Bottom => temp_cy - total_block_height,
        };

        // --- Rasterise all lines into the temp image ---
        let mut temp = GrayImage::new(temp_w, temp_h);

        for (line_idx, (line_text, line_width)) in
            display_lines.iter().zip(line_widths.iter()).enumerate()
        {
            // Baseline Y for this line.
            let baseline_y = temp_y_block_top + ascent + line_idx as f32 * step;

            // Left edge of this line, based on horizontal alignment.
            let x_start = match h_align {
                TextHAlign::Left   => temp_cx,
                TextHAlign::Center => temp_cx - line_width / 2.0,
                TextHAlign::Right  => temp_cx - line_width,
            };

            // Walk glyphs for this line.
            let mut cursor_x = 0.0f32;
            let mut prev_id: Option<_> = None;

            for c in line_text.chars() {
                let id = scaled.glyph_id(c);
                if let Some(prev) = prev_id {
                    cursor_x += scaled.kern(prev, id);
                }

                let glyph = id.with_scale_and_position(
                    scale,
                    ab_glyph::point(x_start + cursor_x, baseline_y),
                );

                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|dx, dy, coverage| {
                        let ix = bounds.min.x as i32 + dx as i32;
                        let iy = bounds.min.y as i32 + dy as i32;
                        if ix >= 0 && iy >= 0 && ix < temp_w as i32 && iy < temp_h as i32 {
                            // Max-blend so overlapping subpixels merge cleanly.
                            let existing = temp.get_pixel(ix as u32, iy as u32).0[0];
                            let new_val  = (coverage * 255.0) as u8;
                            temp.put_pixel(ix as u32, iy as u32, image::Luma([existing.max(new_val)]));
                        }
                    });
                }

                cursor_x += scaled.h_advance(id) + letter_sp;
                prev_id = Some(id);
            }
        }

        // --- Apply rotation (if requested) and produce final image ---
        let dynamic_image = if use_rotation {
            let rotation_rad = rotation_deg.to_radians();
            // Rotate the temp image around its own centre, which maps to the anchor point.
            let rotated = rotate_about_center(
                &temp,
                rotation_rad,
                Interpolation::Bilinear,
                image::Luma([0u8]),
            );

            // Blit the rotated temp image onto the main canvas, centred at the anchor.
            let mut canvas = GrayImage::new(img_width, img_height);
            let blit_x = (anchor_x - temp_w as f32 / 2.0).round() as i32;
            let blit_y = (anchor_y - temp_h as f32 / 2.0).round() as i32;

            for y in 0..temp_h {
                for x in 0..temp_w {
                    let cx = blit_x + x as i32;
                    let cy = blit_y + y as i32;
                    if cx >= 0 && cy >= 0 && cx < img_width as i32 && cy < img_height as i32 {
                        let src = rotated.get_pixel(x, y).0[0];
                        if src > 0 {
                            let dst = canvas.get_pixel(cx as u32, cy as u32).0[0];
                            canvas.put_pixel(cx as u32, cy as u32, image::Luma([src.max(dst)]));
                        }
                    }
                }
            }

            DynamicImage::ImageLuma8(canvas)
        } else {
            // No rotation: temp is already the full canvas-sized image.
            DynamicImage::ImageLuma8(temp)
        };

        // Convert the grayscale DynamicImage to a 1-channel FloatImage
        let float_img = FloatImage::from_dynamic(&dynamic_image);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {
                    value: Value::Image { data: Arc::new(float_img), change_id: get_id() },
                },
            ],
        })
    }
}

/// Measures the pixel width of a text string at the current scale.
///
/// `letter_sp` is added to every glyph's natural horizontal advance, so even
/// the last glyph contributes a small trailing gap. This is intentional: it
/// keeps the measure consistent with the actual advance used when rasterising,
/// which matters for right-alignment and center-alignment accuracy.
fn measure_line_width<F: Font, SF: ScaleFont<F>>(text: &str, scaled: &SF, letter_sp: f32) -> f32 {
    let mut width = 0.0f32;
    let mut prev_id: Option<_> = None;

    for c in text.chars() {
        let id = scaled.glyph_id(c);
        if let Some(prev) = prev_id {
            width += scaled.kern(prev, id);
        }
        width += scaled.h_advance(id) + letter_sp;
        prev_id = Some(id);
    }
    width
}

/// Word-wraps a single text segment (already split on `\n`) into display lines.
///
/// Returns the segment unchanged when `wrap_px <= 0.0`. Words are delimited by
/// spaces. A single word wider than `wrap_px` is kept on its own line without
/// further splitting — it will extend beyond the wrap boundary rather than be
/// truncated or cause a panic.
fn wrap_line<F: Font, SF: ScaleFont<F>>(
    text: &str,
    scaled: &SF,
    letter_sp: f32,
    wrap_px: f32,
) -> Vec<String> {
    if wrap_px <= 0.0 {
        return vec![text.to_string()];
    }

    let mut result: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0f32;
    // Width of a space character (plus letter spacing).
    let space_w = scaled.h_advance(scaled.glyph_id(' ')) + letter_sp;

    for word in text.split(' ') {
        if word.is_empty() {
            // Consecutive spaces: count the gap but don't add a visible word.
            if !current.is_empty() {
                current.push(' ');
                current_w += space_w;
            }
            continue;
        }

        let word_w = measure_line_width(word, scaled, letter_sp);

        if current.is_empty() {
            // Always accept the first word on a line, even if oversized.
            current.push_str(word);
            current_w = word_w;
        } else {
            let new_w = current_w + space_w + word_w;
            if new_w <= wrap_px {
                current.push(' ');
                current.push_str(word);
                current_w = new_w;
            } else {
                result.push(current.clone());
                current = word.to_string();
                current_w = word_w;
            }
        }
    }

    result.push(current);
    result
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
