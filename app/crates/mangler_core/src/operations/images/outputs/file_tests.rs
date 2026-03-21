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
fn make_file_inputs(img: Arc<FloatImage>, folder: std::path::PathBuf, format: image::ImageFormat) -> Vec<Input> {
    make_file_inputs_with_format(img, folder, format, ColorFormat::Rgba8)
}

/// Helper to build file output inputs with a specific color format.
fn make_file_inputs_with_format(
    img: Arc<FloatImage>,
    folder: std::path::PathBuf,
    format: image::ImageFormat,
    color_format: ColorFormat,
) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("file name".to_string(), Value::Text("test_output".to_string()), None, None),
        Input::new("folder".to_string(), Value::Path(folder), None, None),
        Input::new("image format".to_string(), Value::ImageType(format), None, None),
        Input::new("jpg quality".to_string(), Value::Integer(85), None, None),
        Input::new("color format".to_string(), Value::ColorFormat(color_format), None, None),
    ]
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
    assert_eq!(OpImageOutputFile::create_inputs().len(), 6);
    assert_eq!(OpImageOutputFile::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_file_output_nonexistent_folder_returns_error() {
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_file_inputs(img, std::path::PathBuf::from("/this/path/does/not/exist/at/all"), image::ImageFormat::Png);
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

    let mut inputs = make_file_inputs(img, tmp.clone(), image::ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.png");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Jpeg, ColorFormat::Rgb8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.jpg");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Bmp, ColorFormat::Rgb8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.bmp");
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

    let mut inputs = make_file_inputs(img, tmp.clone(), image::ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.png");
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

    let mut inputs = make_file_inputs(img, tmp.clone(), image::ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.png");
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

    let mut inputs = make_file_inputs(img, tmp.clone(), image::ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Path(p) => {
            assert!(!p.as_os_str().is_empty(), "output path should not be empty");
            assert!(p.exists(), "output file should exist at the returned path");
        }
        other => panic!("Expected Path output, got {:?}", other),
    }

    let path = tmp.join("test_output.png");
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
    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Png, ColorFormat::Gray8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.png");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Png, ColorFormat::Rgba16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.png");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::OpenExr, ColorFormat::Rgba32F);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.exr");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Jpeg, ColorFormat::Gray8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.jpg");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Png, ColorFormat::Gray16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.png");
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Png, ColorFormat::Rgba32F);
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Jpeg, ColorFormat::Rgb16);
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

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Farbfeld, ColorFormat::Rgba16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    let path = tmp.join("test_output.ff");
    assert_save_ok(result, &path);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&tmp).ok();
}

#[tokio::test]
async fn test_file_output_farbfeld_wrong_format_errors() {
    // Farbfeld with Rgba8 — should be rejected.
    let imgbuf = image::RgbaImage::new(4, 4);
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));

    let tmp = std::env::temp_dir().join("nodemangler_test_farbfeld_wrong");
    std::fs::create_dir_all(&tmp).unwrap();

    let mut inputs = make_file_inputs_with_format(img, tmp.clone(), image::ImageFormat::Farbfeld, ColorFormat::Rgba8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgba8 + Farbfeld should be rejected");
    let err = result.unwrap_err();
    assert!(err.node_error.is_some(), "should have a node-level error message");

    std::fs::remove_dir(&tmp).ok();
}
