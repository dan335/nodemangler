//! Colour lookup (LUT) adjustment operation for images.
//!
//! Loads an Adobe `.cube` colour LUT (1D or 3D) from disk and applies it to an
//! image. 3D LUTs are evaluated with trilinear interpolation over the RGB cube;
//! 1D LUTs map each channel independently by lerping along the table. A
//! `strength` slider blends between the original and graded pixel so partial
//! grades are possible. An empty path is a pass-through (the source image is
//! returned unchanged).

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// A parsed Adobe `.cube` colour LUT.
///
/// Holds either a 1D LUT (`dims == 1`, `size` entries, applied per channel) or a
/// 3D LUT (`dims == 3`, `size*size*size` entries, applied as a trilinear lookup
/// over the RGB cube). The input domain is `[domain_min, domain_max]` per
/// channel; values are normalised into `[0,1]` against this domain before the
/// lookup.
pub struct CubeLut {
    /// The grid size `n`: a 3D LUT holds `n*n*n` entries, a 1D LUT holds `n`.
    pub size: usize,
    /// LUT dimensionality: `1` for a 1D LUT, `3` for a 3D LUT.
    pub dims: u8,
    /// The LUT table entries as `[r, g, b]` triples.
    ///
    /// For a 3D LUT the flat index is `red + green*n + blue*n*n` (red varies
    /// fastest), matching the `.cube` file layout.
    pub data: Vec<[f32; 3]>,
    /// Per-channel lower bound of the input domain (defaults to `[0,0,0]`).
    pub domain_min: [f32; 3],
    /// Per-channel upper bound of the input domain (defaults to `[1,1,1]`).
    pub domain_max: [f32; 3],
}

/// Colour lookup operation applying a `.cube` LUT to an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentColorLookup {}

impl OpImageAdjustmentColorLookup {
    /// Returns the node metadata (name, description and help) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "color lookup".to_string(),
            description: "Applies a 3D or 1D colour LUT from an Adobe .cube file.".to_string(),
            help: "Loads an Adobe .cube colour lookup table from disk and applies it to every pixel of the image.\n\nA 3D LUT (LUT_3D_SIZE n) samples an n\u{00d7}n\u{00d7}n cube of output colours and is evaluated with trilinear interpolation across the eight surrounding grid points, so smooth grades stay smooth even for small cubes. A 1D LUT (LUT_1D_SIZE n) instead holds n entries and is applied to each channel independently by lerping along the table using that channel's normalised value \u{2014} useful for pure tone/contrast curves.\n\nEach channel is first normalised into [0,1] against the file's DOMAIN_MIN / DOMAIN_MAX (defaulting to 0 and 1) and clamped, then looked up. The strength slider blends between the original pixel and the graded result (0 = original, 1 = fully graded); the blended output is not clamped. Alpha is always preserved and images with fewer than three channels pass through unchanged.\n\nNote the .cube ordering convention: the RED component varies fastest, then green, then blue. An empty path is treated as a pass-through so the node is harmless until a LUT is chosen. Parse errors (missing size, wrong row count, malformed numbers) surface as a node error.".to_string(),
        }
    }

    /// Creates the input ports: the source image, the `.cube` LUT path, and a strength slider.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to grade through the LUT."),
            Input::new("lut".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: vec!["cube".to_string()],
                set_directory: None,
                set_file_name: None,
                set_title: Some("LUT (.cube)".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }), None)
                .with_description("Path to an Adobe .cube LUT file (1D or 3D). Empty = pass-through."),
            Input::new("strength".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Blend between the original (0) and the fully graded result (1)."),
        ]
    }

    /// Creates the output port: the graded image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with the LUT applied, blended by strength; alpha preserved."),
        ]
    }

    /// Executes the colour lookup: loads and parses the LUT, then applies it per pixel.
    ///
    /// Returns the input image unchanged when the path is empty. Returns a node
    /// error if the file cannot be read or the `.cube` content is malformed.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let lut_converted = convert_input(inputs, 1, ValueType::Path, &mut input_errors);
        let strength_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Path(path) = lut_converted.unwrap() else { unreachable!() };
        let Value::Decimal(strength) = strength_converted.unwrap() else { unreachable!() };

        // Empty path is a pass-through: hand the source image straight back.
        if path.as_os_str().is_empty() {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                ],
            });
        }

        // Read the LUT file from disk.
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => return Err(OperationError { input_errors: vec![], node_error: Some(format!("Failed to read LUT file '{}': {}", path.display(), e)) }),
        };

        // Parse the .cube content into a CubeLut.
        let lut = match parse_cube(&text) {
            Ok(l) => l,
            Err(e) => return Err(OperationError { input_errors: vec![], node_error: Some(format!("Failed to parse .cube LUT: {}", e)) }),
        };

        // Clone the source image and apply the LUT to each pixel's colour channels.
        let mut result = (*data).clone();
        let ch = result.channels() as usize;

        // Only images with at least three colour channels carry RGB; anything
        // smaller (grayscale, gray+alpha) has no chroma to grade, so pass through.
        if ch >= 3 {
            for pixel in result.pixels_mut() {
                // Sample the LUT for the pixel's RGB triple.
                let rgb = [pixel[0], pixel[1], pixel[2]];
                let looked_up = sample(&lut, rgb);
                // Blend each colour channel toward the graded value by strength.
                for c in 0..3 {
                    pixel[c] = rgb[c] + (looked_up[c] - rgb[c]) * strength as f32;
                }
                // Channels 3.. (alpha) are left untouched.
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}

/// Parses the text of an Adobe `.cube` LUT into a [`CubeLut`].
///
/// Supported directives:
/// - `TITLE "..."` — ignored.
/// - `DOMAIN_MIN r g b` / `DOMAIN_MAX r g b` — input domain (defaults 0 and 1).
/// - `LUT_1D_SIZE n` — declares an `n`-entry 1D LUT.
/// - `LUT_3D_SIZE n` — declares an `n*n*n`-entry 3D LUT (red varies fastest).
///
/// Blank lines and lines starting with `#` are ignored. Every other non-directive
/// line is treated as an `r g b` data row. Returns `Err` with a descriptive
/// message if the size is missing, a number is malformed, or the row count does
/// not match the declared size.
pub fn parse_cube(text: &str) -> Result<CubeLut, String> {
    let mut dims: Option<u8> = None;
    let mut size: usize = 0;
    let mut domain_min = [0.0f32, 0.0, 0.0];
    let mut domain_max = [1.0f32, 1.0, 1.0];
    let mut data: Vec<[f32; 3]> = Vec::new();

    /// Parses exactly three whitespace-separated floats from an iterator of tokens.
    fn parse_triple<'a, I: Iterator<Item = &'a str>>(mut it: I, ctx: &str) -> Result<[f32; 3], String> {
        let mut out = [0.0f32; 3];
        for (i, slot) in out.iter_mut().enumerate() {
            let tok = it.next().ok_or_else(|| format!("{}: expected 3 numbers, found {}", ctx, i))?;
            *slot = tok.parse::<f32>().map_err(|_| format!("{}: '{}' is not a valid number", ctx, tok))?;
        }
        Ok(out)
    }

    for raw_line in text.lines() {
        let line = raw_line.trim();
        // Skip blank lines and comments.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut tokens = line.split_whitespace();
        let keyword = tokens.next().unwrap_or("");
        let upper = keyword.to_ascii_uppercase();

        match upper.as_str() {
            "TITLE" => {
                // Title metadata is not used.
                continue;
            }
            "DOMAIN_MIN" => {
                domain_min = parse_triple(tokens, "DOMAIN_MIN")?;
            }
            "DOMAIN_MAX" => {
                domain_max = parse_triple(tokens, "DOMAIN_MAX")?;
            }
            "LUT_1D_SIZE" => {
                let n = tokens.next().ok_or("LUT_1D_SIZE: missing size")?;
                let n: usize = n.parse().map_err(|_| format!("LUT_1D_SIZE: '{}' is not a valid integer", n))?;
                if n < 2 { return Err("LUT_1D_SIZE must be at least 2".to_string()); }
                dims = Some(1);
                size = n;
            }
            "LUT_3D_SIZE" => {
                let n = tokens.next().ok_or("LUT_3D_SIZE: missing size")?;
                let n: usize = n.parse().map_err(|_| format!("LUT_3D_SIZE: '{}' is not a valid integer", n))?;
                if n < 2 { return Err("LUT_3D_SIZE must be at least 2".to_string()); }
                dims = Some(3);
                size = n;
            }
            _ => {
                // Any other non-directive line must be a data row: r g b.
                // Re-parse the whole line (keyword token was actually the first number).
                let triple = parse_triple(line.split_whitespace(), "LUT data row")?;
                data.push(triple);
            }
        }
    }

    let dims = dims.ok_or("no LUT_1D_SIZE or LUT_3D_SIZE directive found")?;

    // Verify the number of data rows matches the declared LUT size.
    let expected = match dims {
        1 => size,
        3 => size * size * size,
        _ => unreachable!(),
    };
    if data.len() != expected {
        return Err(format!(
            "expected {} LUT data rows for {}D LUT of size {}, found {}",
            expected, dims, size, data.len()
        ));
    }

    Ok(CubeLut { size, dims, data, domain_min, domain_max })
}

/// Samples a [`CubeLut`] for an input RGB triple, returning the graded RGB.
///
/// Each channel is normalised into `[0,1]` against the LUT's domain and clamped.
/// A 3D LUT is evaluated with trilinear interpolation over the eight surrounding
/// grid points; a 1D LUT lerps each channel independently along the table.
pub fn sample(lut: &CubeLut, rgb: [f32; 3]) -> [f32; 3] {
    // Normalise each channel into [0,1] against the input domain, clamped.
    let mut t = [0.0f32; 3];
    for c in 0..3 {
        let span = lut.domain_max[c] - lut.domain_min[c];
        let n = if span.abs() > f32::EPSILON {
            (rgb[c] - lut.domain_min[c]) / span
        } else {
            0.0
        };
        t[c] = n.clamp(0.0, 1.0);
    }

    match lut.dims {
        1 => {
            // 1D LUT: lerp each channel independently along the table.
            let mut out = [0.0f32; 3];
            let last = (lut.size - 1) as f32;
            for c in 0..3 {
                let pos = t[c] * last;
                let i0 = pos.floor() as usize;
                let i1 = (i0 + 1).min(lut.size - 1);
                let frac = pos - i0 as f32;
                out[c] = lut.data[i0][c] * (1.0 - frac) + lut.data[i1][c] * frac;
            }
            out
        }
        _ => {
            // 3D LUT: trilinear interpolation over the RGB cube.
            let n = lut.size;
            let last = (n - 1) as f32;

            // Scale normalized values to grid coordinates.
            let fr = t[0] * last;
            let fg = t[1] * last;
            let fb = t[2] * last;

            // Lower corner indices and fractional offsets.
            let r0 = fr.floor() as usize;
            let g0 = fg.floor() as usize;
            let b0 = fb.floor() as usize;
            let r1 = (r0 + 1).min(n - 1);
            let g1 = (g0 + 1).min(n - 1);
            let b1 = (b0 + 1).min(n - 1);
            let dr = fr - r0 as f32;
            let dg = fg - g0 as f32;
            let db = fb - b0 as f32;

            // Fetch an entry by (red, green, blue) grid indices; red varies fastest.
            let fetch = |r: usize, g: usize, b: usize| -> [f32; 3] {
                lut.data[r + g * n + b * n * n]
            };

            // Eight cube corners.
            let c000 = fetch(r0, g0, b0);
            let c100 = fetch(r1, g0, b0);
            let c010 = fetch(r0, g1, b0);
            let c110 = fetch(r1, g1, b0);
            let c001 = fetch(r0, g0, b1);
            let c101 = fetch(r1, g0, b1);
            let c011 = fetch(r0, g1, b1);
            let c111 = fetch(r1, g1, b1);

            // Linear interpolation helper for two RGB triples.
            let lerp = |a: [f32; 3], b: [f32; 3], f: f32| -> [f32; 3] {
                [
                    a[0] * (1.0 - f) + b[0] * f,
                    a[1] * (1.0 - f) + b[1] * f,
                    a[2] * (1.0 - f) + b[2] * f,
                ]
            };

            // Interpolate along red, then green, then blue.
            let c00 = lerp(c000, c100, dr);
            let c10 = lerp(c010, c110, dr);
            let c01 = lerp(c001, c101, dr);
            let c11 = lerp(c011, c111, dr);

            let c0 = lerp(c00, c10, dg);
            let c1 = lerp(c01, c11, dg);

            lerp(c0, c1, db)
        }
    }
}

#[cfg(test)]
#[path = "color_lookup_tests.rs"]
mod tests;
