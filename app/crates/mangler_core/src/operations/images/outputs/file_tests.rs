use super::*;
use image::DynamicImage;
use crate::float_image::FloatImage;
use std::sync::Arc;
use crate::get_id;
use crate::value::ColorFormat;

/// Helper to create a `FloatImage` from a `DynamicImage` for test convenience.
fn float_from_dynamic(img: DynamicImage) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_dynamic(&img))
}

/// Helper to build file output inputs with default Rgba8 color format.
/// `path` is the full destination path (extension drives the format).
fn make_file_inputs(img: Arc<FloatImage>, path: std::path::PathBuf) -> Vec<Input> {
    make_file_inputs_with_format(img, path, ColorFormat::Rgba8)
}

/// Helper to build file output inputs with a specific color format.
fn make_file_inputs_with_format(
    img: Arc<FloatImage>,
    path: std::path::PathBuf,
    color_format: ColorFormat,
) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("file path".to_string(), Value::Path(path), None, None),
        Input::new("quality".to_string(), Value::Integer(85), None, None),
        Input::new("color format".to_string(), Value::ColorFormat(color_format), None, None),
        Input::new("png compression".to_string(), Value::Text("fast".to_string()), None, None),
    ]
}

/// Reference conversion: the historical two-step path (`FloatImage::to_dynamic`
/// followed by the `image` crate's whole-buffer conversions). The optimized
/// single-pass `convert_from_float` must stay byte-identical to this.
fn reference_convert(data: &FloatImage, format: &ColorFormat) -> DynamicImage {
    let img = data.to_dynamic();
    match format {
        ColorFormat::Rgba32F => DynamicImage::ImageRgba32F(img.to_rgba32f()),
        ColorFormat::Rgb32F => DynamicImage::ImageRgb32F(img.to_rgb32f()),
        ColorFormat::Rgba16 => DynamicImage::ImageRgba16(img.to_rgba16()),
        ColorFormat::Rgb16 => DynamicImage::ImageRgb16(img.to_rgb16()),
        ColorFormat::GrayA16 => DynamicImage::ImageLumaA16(img.to_luma_alpha16()),
        ColorFormat::Gray16 => DynamicImage::ImageLuma16(img.to_luma16()),
        ColorFormat::Rgba8 => DynamicImage::ImageRgba8(img.to_rgba8()),
        ColorFormat::Rgb8 => DynamicImage::ImageRgb8(img.to_rgb8()),
        ColorFormat::GrayA8 => DynamicImage::ImageLumaA8(img.to_luma_alpha8()),
        ColorFormat::Gray8 => DynamicImage::ImageLuma8(img.to_luma8()),
    }
}

#[test]
fn test_convert_from_float_matches_reference() {
    let formats = [
        ColorFormat::Gray8, ColorFormat::Gray16, ColorFormat::GrayA8, ColorFormat::GrayA16,
        ColorFormat::Rgb8, ColorFormat::Rgb16, ColorFormat::Rgb32F,
        ColorFormat::Rgba8, ColorFormat::Rgba16, ColorFormat::Rgba32F,
    ];
    for ch in 1..=4u32 {
        // Deterministic pseudo-random pixels including edge/out-of-range values.
        let mut img = FloatImage::new(7, 5, ch);
        let mut state = 0x1234_5678u32;
        for (i, v) in img.as_raw_mut().iter_mut().enumerate() {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *v = match i % 7 {
                0 => 0.0,
                1 => 1.0,
                2 => -0.25,
                3 => 1.5,
                4 => 0.4999,
                _ => (state >> 8) as f32 / 16_777_216.0,
            };
        }
        for format in &formats {
            let got = convert_from_float(&img, format);
            let want = reference_convert(&img, format);
            assert_eq!(got.color(), want.color(), "layout mismatch for {:?} from {}ch", format, ch);
            assert_eq!(got.as_bytes(), want.as_bytes(), "bytes mismatch for {:?} from {}ch", format, ch);
        }
    }
}

/// Helper to create a temp dir, run the operation, and assert success.
/// Returns the output file path.
fn assert_save_ok(result: Result<OperationResponse, OperationError>, path: &std::path::Path) {
    assert!(result.is_ok(), "save should succeed, got: {:?}", result.err());
    let metadata = std::fs::metadata(path).unwrap();
    assert!(metadata.len() > 0, "saved file should not be 0 bytes");
}

#[tokio::test]
async fn test_file_output_settings() {
    let s = OpImageOutputFile::settings();
    assert!(!s.name.is_empty());
    assert!(!OpImageOutputFile::create_inputs().is_empty());
    assert!(!OpImageOutputFile::create_outputs().is_empty());
}

#[tokio::test]
async fn test_file_output_exact_settings() {
    let s = OpImageOutputFile::settings();
    assert_eq!(s.name, "to file");
    let inputs = OpImageOutputFile::create_inputs();
    assert_eq!(inputs.len(), 5);
    let names: Vec<&str> = inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["image", "file path", "quality", "color format", "png compression"]);
    assert_eq!(OpImageOutputFile::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_file_output_nonexistent_folder_returns_error() {
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_file_inputs(img, std::path::PathBuf::from("/this/path/does/not/exist/at/all/out.png"));
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "saving to nonexistent folder should fail");
}

// --- Bug #1 regression: ImageRgba32F must save successfully ---

#[tokio::test]
async fn test_file_output_rgba32f_saves_png() {
    // Operations like blend, levels, curves output ImageRgba32F.
    // The file output node must handle this without producing 0-byte files.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba32f_png");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_saves_jpeg() {
    // JPEG path uses a separate encoder; verify Rgba32F works there too.
    // Default color format is Rgba8, which is incompatible with JPEG.
    // Use Rgb8 for JPEG.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba32f_jpg");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.jpg");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Rgb8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_saves_bmp() {
    // BMP uses the rgb-only path; verify Rgba32F works there too.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |_, _| {
        image::Rgba([0.3, 0.6, 0.9, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba32f_bmp");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.bmp");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Rgb8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_with_hdr_values_saves() {
    // Some operations produce float values > 1.0 (e.g. contrast, brighten).
    // File output must clamp/convert these without failing.
    let imgbuf = image::Rgba32FImage::from_fn(4, 4, |_, _| {
        image::Rgba([2.5, -0.3, 1.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba32f_hdr");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgba8_still_works() {
    // Sanity check: standard Rgba8 images must still save correctly.
    let imgbuf = image::RgbaImage::from_fn(8, 8, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba8_png");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_returns_path_on_success() {
    // Verify the output value contains the actual file path, not an empty path.
    let imgbuf = image::Rgba32FImage::from_fn(4, 4, |_, _| {
        image::Rgba([0.5, 0.5, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba32f_path");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Path(p) => {
            assert!(!p.as_os_str().is_empty(), "output path should not be empty");
            assert!(p.exists(), "output file should exist at the returned path");
            assert_eq!(p, &path, "output path should be the requested path exactly");
        }
        other => panic!("Expected Path output, got {:?}", other),
    }

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_luma8_saves_png() {
    // Noise generators output ImageLuma8; verify it still saves correctly.
    let imgbuf = image::GrayImage::from_fn(8, 8, |x, _| {
        image::Luma([(x * 32) as u8])
    });
    let img = float_from_dynamic(DynamicImage::ImageLuma8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_luma8_png");
    std::fs::create_dir_all(&tmp).unwrap();

    // Use Gray8 to match the luma input
    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Gray8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

// --- Color format tests ---

#[tokio::test]
async fn test_file_output_rgba16_saves_png() {
    // 16-bit RGBA to PNG should preserve higher bit depth.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba16_png");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Rgba16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_saves_exr() {
    // 32-bit float to OpenEXR — the only format that supports full float precision.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba32f_exr");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.exr");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Rgba32F);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_gray8_saves_jpeg() {
    // Grayscale JPEG output.
    let imgbuf = image::GrayImage::from_fn(8, 8, |x, _| {
        image::Luma([(x * 32) as u8])
    });
    let img = float_from_dynamic(DynamicImage::ImageLuma8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_gray8_jpg");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.jpg");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Gray8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_gray16_saves_png() {
    // 16-bit grayscale to PNG.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, _| {
        image::Rgba([x as f32 / 8.0, 0.0, 0.0, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_gray16_png");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Gray16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_incompatible_rgba32f_png_errors() {
    // Rgba32F is not compatible with PNG — should return an error.
    let imgbuf = image::Rgba32FImage::from_fn(4, 4, |_, _| {
        image::Rgba([0.5, 0.5, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_incompat_rgba32f_png");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs_with_format(img, path, ColorFormat::Rgba32F);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgba32F + PNG should be rejected");
    let err = result.unwrap_err();
    assert!(err.node_error.is_some(), "should have a node-level error message");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_incompatible_rgb16_jpeg_errors() {
    // Rgb16 is not compatible with JPEG — should return an error.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_incompat_rgb16_jpg");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.jpg");
    let mut inputs = make_file_inputs_with_format(img, path, ColorFormat::Rgb16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgb16 + JPEG should be rejected");
    let err = result.unwrap_err();
    assert!(err.node_error.is_some(), "should have a node-level error message");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_farbfeld_rgba16() {
    // Farbfeld with Rgba16 — the only compatible format.
    let imgbuf = image::RgbaImage::from_fn(8, 8, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_farbfeld_rgba16");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.ff");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Rgba16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

// --- Path handling ---

#[tokio::test]
async fn test_file_output_dotted_filename_keeps_full_name() {
    // A dot in the file stem must not be treated as an extension boundary:
    // "render.v2.png" must save exactly as given, not as "render.png".
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_dotted_name");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("render.v2.png");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_empty_path_errors() {
    // An empty path must be rejected.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let mut inputs = make_file_inputs(img, std::path::PathBuf::new());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "empty path should be rejected");
}

#[tokio::test]
async fn test_file_output_missing_extension_errors_before_writing() {
    // A path with no extension can't select a format; must error, and no
    // file should be created.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_missing_ext");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("no_extension");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "missing extension should be rejected");
    assert!(!path.exists(), "no file should be created on error");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_unknown_extension_errors_before_writing() {
    // An unrecognized extension must be rejected before any file is written.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_unknown_ext");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("out.dds");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "unknown extension should be rejected");
    let err = result.unwrap_err();
    assert!(err.node_error.as_ref().unwrap().contains("dds"), "error should mention the bad extension: {:?}", err.node_error);
    assert!(!path.exists(), "no file should be created on error");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_missing_parent_folder_errors() {
    // The path's parent directory must exist; a nonexistent parent is rejected.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let path = std::path::PathBuf::from("/this/path/does/not/exist/at/all/out.png");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "missing parent folder should be rejected");
    assert!(!path.exists(), "no file should be created on error");
}

#[tokio::test]
async fn test_file_output_folder_is_file_errors() {
    // A destination whose parent path points at an existing file must be rejected.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_folder_is_file");
    std::fs::create_dir_all(&tmp).unwrap();
    let file_as_folder = tmp.join("not_a_folder.txt");
    std::fs::write(&file_as_folder, "x").unwrap();

    let path = file_as_folder.join("out.png");
    let mut inputs = make_file_inputs(img, path);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "file used as folder should be rejected");

    std::fs::remove_file(&file_as_folder).ok();
    std::fs::remove_dir(&tmp).ok();
}

// --- Encoder settings ---

#[tokio::test]
async fn test_file_output_jpg_quality_affects_size() {
    // The quality input must actually reach the JPEG encoder: a low-quality
    // save of a non-trivial image must be smaller than a high-quality one.
    let imgbuf = image::RgbImage::from_fn(64, 64, |x, y| {
        image::Rgb([(x * 4) as u8, (y * 4) as u8, ((x * y) % 256) as u8])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgb8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_jpg_quality");
    std::fs::create_dir_all(&tmp).unwrap();

    let mut sizes = vec![];
    for (name, quality) in [("q10", 10), ("q95", 95)] {
        let path = tmp.join(format!("{}.jpg", name));
        let mut inputs = make_file_inputs_with_format(img.clone(), path.clone(), ColorFormat::Rgb8);
        inputs[2].value = Value::Integer(quality);
        let result = OpImageOutputFile::run(&mut inputs).await;
        assert_save_ok(result, &path);
        sizes.push(std::fs::metadata(&path).unwrap().len());
        std::fs::remove_file(&path).ok();
    }
    assert!(sizes[0] < sizes[1], "quality 10 ({} bytes) should be smaller than quality 95 ({} bytes)", sizes[0], sizes[1]);

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_png_compression_levels() {
    // Every compression level must produce a decodable PNG with identical
    // pixels; "uncompressed" must be larger than "best".
    let imgbuf = image::RgbaImage::from_fn(32, 32, |x, y| {
        image::Rgba([(x * 8) as u8, (y * 8) as u8, 128, 255])
    });
    let reference = imgbuf.clone();
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_png_compression");
    std::fs::create_dir_all(&tmp).unwrap();

    let mut sizes = std::collections::HashMap::new();
    for level in ["fast", "default", "best", "uncompressed"] {
        let path = tmp.join(format!("{}.png", level));
        let mut inputs = make_file_inputs(img.clone(), path.clone());
        inputs[4].value = Value::Text(level.to_string());
        let result = OpImageOutputFile::run(&mut inputs).await;
        assert_save_ok(result, &path);

        let decoded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(decoded.as_raw(), reference.as_raw(), "{} PNG should decode to identical pixels", level);
        sizes.insert(level, std::fs::metadata(&path).unwrap().len());
        std::fs::remove_file(&path).ok();
    }
    assert!(sizes["best"] < sizes["uncompressed"], "best ({}) should be smaller than uncompressed ({})", sizes["best"], sizes["uncompressed"]);

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_invalid_png_compression_errors() {
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_bad_png_compression");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.png");
    let mut inputs = make_file_inputs(img, path);
    inputs[4].value = Value::Text("banana".to_string());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "unknown png compression should be rejected");
    let err = result.unwrap_err();
    assert_eq!(err.input_errors.first().map(|(i, _)| *i), Some(4), "error should point at the png compression input");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgb32f_saves_hdr() {
    // Radiance HDR writes RGBE from Rgb32F; values above 1.0 must survive
    // the round trip (within RGBE's shared-exponent precision).
    let imgbuf = image::Rgb32FImage::from_fn(8, 8, |x, y| {
        image::Rgb([x as f32 / 4.0, y as f32 / 8.0, 2.5])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgb32F(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgb32f_hdr");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.hdr");
    let mut inputs = make_file_inputs_with_format(img, path.clone(), ColorFormat::Rgb32F);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    let decoded = image::open(&path).unwrap().to_rgb32f();
    let px = decoded.get_pixel(7, 0);
    assert!((px[0] - 1.75).abs() < 0.02, "HDR value above 1.0 should survive, got {}", px[0]);
    assert!((px[2] - 2.5).abs() < 0.02, "HDR value above 1.0 should survive, got {}", px[2]);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_hdr_wrong_color_format_errors() {
    // HDR only accepts Rgb32F; 8-bit layouts must be rejected up front.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_hdr_wrong_cf");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.hdr");
    let mut inputs = make_file_inputs(img, path);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "HDR + Rgba8 should be rejected");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_rgba8_saves_avif() {
    // AVIF encodes through ravif (pure Rust); verify a decodable, non-empty
    // file is produced. (Decoding AVIF needs a C library, so only check size.)
    let imgbuf = image::RgbaImage::from_fn(16, 16, |x, y| {
        image::Rgba([(x * 16) as u8, (y * 16) as u8, 128, 255])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_rgba8_avif");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.avif");
    let mut inputs = make_file_inputs(img, path.clone());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_avif_quality_affects_size() {
    // The shared quality input must reach the AVIF encoder.
    let imgbuf = image::RgbImage::from_fn(64, 64, |x, y| {
        image::Rgb([(x * 4) as u8, (y * 4) as u8, ((x * y) % 256) as u8])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgb8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_avif_quality");
    std::fs::create_dir_all(&tmp).unwrap();

    let mut sizes = vec![];
    for (name, quality) in [("q10", 10), ("q95", 95)] {
        let path = tmp.join(format!("{}.avif", name));
        let mut inputs = make_file_inputs_with_format(img.clone(), path.clone(), ColorFormat::Rgb8);
        inputs[2].value = Value::Integer(quality);
        let result = OpImageOutputFile::run(&mut inputs).await;
        assert_save_ok(result, &path);
        sizes.push(std::fs::metadata(&path).unwrap().len());
        std::fs::remove_file(&path).ok();
    }
    assert!(sizes[0] < sizes[1], "quality 10 ({} bytes) should be smaller than quality 95 ({} bytes)", sizes[0], sizes[1]);

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_avif_wrong_color_format_errors() {
    // AVIF always encodes 8-bit, so 16-bit/float layouts are rejected.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_avif_wrong_cf");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.avif");
    let mut inputs = make_file_inputs_with_format(img, path, ColorFormat::Rgba16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "AVIF + Rgba16 should be rejected");

    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_farbfeld_wrong_format_errors() {
    // Farbfeld with Rgba8 — should be rejected.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_farbfeld_wrong");
    std::fs::create_dir_all(&tmp).unwrap();

    let path = tmp.join("test_output.ff");
    let mut inputs = make_file_inputs_with_format(img, path, ColorFormat::Rgba8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgba8 + Farbfeld should be rejected");
    let err = result.unwrap_err();
    assert!(err.node_error.is_some(), "should have a node-level error message");

    std::fs::remove_dir(&tmp).ok();
}
