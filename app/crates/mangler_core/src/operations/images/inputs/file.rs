//! Image-from-file input operation.
//!
//! Reads an image from a local file path and outputs the decoded image
//! along with its width and height. Most formats decode through the image
//! crate into a `DynamicImage`, converted to a `FloatImage` via
//! [`FloatImage::from_dynamic`], preserving the original channel count
//! (grayscale stays 1ch, RGB 3ch, etc.). JPEG XL (via jxl-oxide), PSD
//! (via psd, flattened composite), HEIC/HEIF (via heif-oxide) and camera
//! RAW (via rawler) are decoded by dedicated pure-Rust crates.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use image::ImageReader;
use super::raw_decode;

/// Operation that loads an image from a file on disk.
///
/// Accepts a file path input with an extension filter matching supported image
/// formats, and produces the decoded image plus its dimensions as outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputFile {}

impl OpImageInputFile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from file".to_string(),
            description: "Grabs an image from a file.".to_string(),
            help: "Decodes an image file from disk and converts it into a FloatImage, preserving the source channel count (grayscale stays 1ch, RGB 3ch, RGBA 4ch). The path input uses a picker filtered to the supported image extensions. JPEG XL files are decoded with jxl-oxide, PSD files with the psd crate (the flattened composite image; individual layers are not exposed), and HEIC/HEIF files (iPhone photos) with heif-oxide — grid tiles, rotation, and Display P3 color are handled; output is sRGB.\n\nCamera RAW files (Canon CR3/CR2, Nikon NEF, Sony ARW, Fujifilm RAF, Adobe DNG and more) are developed with rawler using the camera's own as-shot settings — demosaic, white balance, colour matrix, sRGB gamma, and EXIF orientation — which looks roughly like the camera's own JPEG. Use the 'from raw' node instead when you want control over white balance, linear output, or resolution — dragging a raw file in from the Libraries panel creates that node for you.\n\nThe 'path' output echoes the loaded path, so downstream text nodes can name an export after the source file.\n\nErrors if the file cannot be opened or the format is unsupported. Note that pixel values are interpreted as sRGB by default; connect a linear-RGB conversion downstream if the file holds linear data like a normal or height map.".to_string(),
        }
    }

    /// Creates the input definitions: a single file path input with image extension filtering.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("path".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path{
                extension_filter: ValueType::file_extensions(&ValueType::Image),
                set_directory: None,
                set_file_name: None,
                set_title: Some("image".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }), None)
                .with_description("Path to an image file to load from disk."),
        ]
    }

    /// Creates the output definitions: the decoded image, its width and height,
    /// and the path it was loaded from.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Image decoded from the file on disk."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Width of the loaded image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Height of the loaded image in pixels."),
            Output::new("path".to_string(), Value::Path(PathBuf::new()), None)
                .with_description("The file path that was loaded, echoed for downstream use (e.g. naming an exported file after the source)."),
        ]
    }

    /// Executes the operation: reads and decodes the image file at the given path.
    ///
    /// Returns an error if the file cannot be opened or the image format is unsupported.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let path_converted = convert_input(inputs, 0, ValueType::Path, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Path(path) = path_converted.unwrap() else { unreachable!() };

        // run node — decoding is shared with the GUI's library image preview.
        let decode_result = load_image_from_path(&path);

        match decode_result {
            Ok(float_img) => {
                let width = float_img.width();
                let height = float_img.height();
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![
                        OutputResponse { value: Value::Image { data: Arc::new(float_img), change_id: get_id() } },
                        OutputResponse { value: Value::Integer(width as i32) },
                        OutputResponse { value: Value::Integer(height as i32) },
                        OutputResponse { value: Value::Path(path) },
                    ],
                })
            }
            Err(e) => Err(OperationError { input_errors, node_error: Some(format!("Error opening image: {}", e)) }),
        }
    }

    /// Decodes a JPEG XL file with jxl-oxide (first frame for animations).
    ///
    /// The stream API yields interleaved f32 color + alpha channels, which map
    /// directly onto `FloatImage` semantics (1ch gray … 4ch RGBA).
    pub(crate) fn decode_jxl(path: &std::path::Path) -> Result<FloatImage, String> {
        let image = jxl_oxide::JxlImage::open_with_defaults(path).map_err(|e| e.to_string())?;
        let render = image.render_frame(0).map_err(|e| e.to_string())?;
        let mut stream = render.stream();
        let (width, height, channels) = (stream.width(), stream.height(), stream.channels());
        if channels == 0 || channels > 4 {
            return Err(format!("Unsupported JPEG XL channel count: {}", channels));
        }
        let mut buf = vec![0f32; width as usize * height as usize * channels as usize];
        stream.write_to_buffer(&mut buf);
        FloatImage::from_raw(width, height, channels, buf)
            .ok_or_else(|| "JPEG XL decode produced a mismatched buffer size.".to_string())
    }

    /// Decodes a PSD file with the psd crate.
    ///
    /// Uses the flattened composite image (individual layers are not exposed),
    /// which the crate always returns as 8-bit RGBA.
    pub(crate) fn decode_psd(path: &std::path::Path) -> Result<FloatImage, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let parsed = psd::Psd::from_bytes(&bytes).map_err(|e| e.to_string())?;
        let (width, height) = (parsed.width(), parsed.height());
        let data: Vec<f32> = parsed.rgba().iter().map(|&v| v as f32 / 255.0).collect();
        FloatImage::from_raw(width, height, 4, data)
            .ok_or_else(|| "PSD decode produced a mismatched buffer size.".to_string())
    }

    /// Decodes a HEIC/HEIF file with heif-oxide (pure Rust: HEIF container +
    /// rust_h265 HEVC decode).
    ///
    /// iPhone-style grid images, irot/imir/clap orientation, and Display P3 →
    /// sRGB conversion are handled by the library; output arrives as
    /// interleaved 0..1 floats in the source's channel count (3ch RGB, or
    /// 4ch when the file carries a decodable alpha auxiliary image).
    pub(crate) fn decode_heif(path: &std::path::Path) -> Result<FloatImage, String> {
        let decoded = heif_oxide::decode_file(path).map_err(|e| e.to_string())?;
        let (width, height, channels) = (decoded.width, decoded.height, decoded.channels());
        FloatImage::from_raw(width, height, channels, decoded.to_f32_interleaved())
            .ok_or_else(|| "HEIC decode produced a mismatched buffer size.".to_string())
    }
}

/// The set of camera RAW extensions, built once.
///
/// `load_image_from_path` is called once per file when listing a folder, so
/// this must not rebuild and allocate the list on every call.
#[cfg(feature = "raw")]
fn raw_extension_set() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| ValueType::raw_file_extensions().into_iter().collect())
}

/// Whether `path`'s extension names a camera RAW format handled by rawler.
///
/// Shared with the GUI so that adding a `.CR3` from the Libraries panel or a
/// file drop creates a `from raw` node — which exposes the development
/// controls — instead of a `from file` node that can only develop it with the
/// camera's as-shot settings. Always false when the `raw` feature is disabled,
/// since nothing in the build could decode it.
pub fn is_raw_file(path: &std::path::Path) -> bool {
    #[cfg(feature = "raw")]
    {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| raw_extension_set().contains(&ext.to_ascii_lowercase()))
    }
    #[cfg(not(feature = "raw"))]
    {
        let _ = path;
        false
    }
}

/// Decodes an image file at `path` into a [`FloatImage`], preserving its
/// channel count. JPEG XL, PSD, HEIC/HEIF and camera RAW use dedicated
/// decoders; everything else goes through the `image` crate. Shared by the
/// image-from-file node and the GUI's library image preview so both accept
/// exactly the same formats.
///
/// RAW files develop with the camera's own settings
/// ([`raw_decode::RawOptions::default`]). Prefer
/// [`load_image_from_path_with_raw_options`] when the caller has develop
/// controls (the `from folder` node); this path stays parameter-free so
/// drag-and-drop and library previews stay simple.
pub fn load_image_from_path(path: &std::path::Path) -> Result<FloatImage, String> {
    #[cfg(feature = "raw")]
    {
        load_image_from_path_with_raw_options(path, &raw_decode::RawOptions::default())
    }
    #[cfg(not(feature = "raw"))]
    {
        load_image_non_raw(path)
    }
}

/// Like [`load_image_from_path`], but develops camera RAW files with `raw_options`
/// instead of the as-shot defaults.
///
/// Non-raw formats ignore `raw_options` entirely. Used by `from folder` so a
/// batch/watch run can apply the same develop recipe (white balance, exposure,
/// max size, linear/sRGB) to every raw in a shoot folder.
pub fn load_image_from_path_with_raw_options(
    path: &std::path::Path,
    raw_options: &raw_decode::RawOptions,
) -> Result<FloatImage, String> {
    #[cfg(feature = "raw")]
    {
        if is_raw_file(path) {
            // Must precede the `image`-crate fallback: several RAW containers
            // are TIFF-based, and `image` would happily decode a DNG's embedded
            // preview thumbnail from IFD0 instead of the actual photograph.
            return raw_decode::decode_raw(path, raw_options);
        }
    }
    #[cfg(not(feature = "raw"))]
    {
        let _ = raw_options;
    }
    load_image_non_raw(path)
}

/// Decode path for every non-raw format (and the only path when the `raw`
/// feature is off).
fn load_image_non_raw(path: &std::path::Path) -> Result<FloatImage, String> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match extension.as_deref() {
        Some("jxl") => OpImageInputFile::decode_jxl(path),
        Some("psd") => OpImageInputFile::decode_psd(path),
        Some("heic") | Some("heif") => OpImageInputFile::decode_heif(path),
        _ => ImageReader::open(path)
            .map_err(|e| e.to_string())
            .and_then(|reader| reader.decode().map_err(|e| e.to_string()))
            .map(|dynamic_image| FloatImage::from_dynamic(&dynamic_image)),
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
