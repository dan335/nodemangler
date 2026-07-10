use super::*;
use image::{DynamicImage, ImageFormat};
use crate::float_image::FloatImage;
use crate::input::InputSettings;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::get_id;
use crate::value::ColorFormat;

/// Helper to create a `FloatImage` from a `DynamicImage` for test convenience.
fn float_from_dynamic(img: DynamicImage) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_dynamic(&img))
}

/// Builds the `to file` inputs with the new folder/name/format layout, forcing
/// `auto save` on so `run` actually writes (these tests call `run` directly,
/// with no engine run-context to force saving). Color format defaults to Rgba8.
fn make_file_inputs(img: Arc<FloatImage>, folder: &Path, name: &str, format: ImageFormat) -> Vec<Input> {
    make_file_inputs_with_format(img, folder, name, format, ColorFormat::Rgba8)
}

/// As [`make_file_inputs`] but with an explicit color format.
fn make_file_inputs_with_format(
    img: Arc<FloatImage>,
    folder: &Path,
    name: &str,
    format: ImageFormat,
    color_format: ColorFormat,
) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("folder".to_string(), Value::Path(folder.to_path_buf()), None, None),
        Input::new("file name".to_string(), Value::Text(name.to_string()), None, None),
        Input::new("format".to_string(), Value::ImageType(format), None, None),
        Input::new("quality".to_string(), Value::Integer(85), None, None),
        Input::new("color format".to_string(), Value::ColorFormat(color_format), None, None),
        Input::new("png compression".to_string(), Value::Text("fast".to_string()), None, None),
        Input::new("auto save".to_string(), Value::Bool(true), None, None),
        Input::new("save".to_string(), Value::Bool(false), Some(InputSettings::Button), None),
    ]
}

/// The canonical extension the given format writes.
fn ext_of(format: ImageFormat) -> &'static str {
    format.extensions_str()[0]
}

/// The path `run` will write for `folder`/`name`/`format`.
fn out_path(folder: &Path, name: &str, format: ImageFormat) -> PathBuf {
    folder.join(format!("{}.{}", name, ext_of(format)))
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

/// Assert the save succeeded and a non-empty file exists at `path`.
fn assert_save_ok(result: Result<OperationResponse, OperationError>, path: &Path) {
    assert!(result.is_ok(), "save should succeed, got: {:?}", result.err());
    let metadata = std::fs::metadata(path).unwrap();
    assert!(metadata.len() > 0, "saved file should not be 0 bytes");
}

/// Creates (or clears) a fresh temp dir for a test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nodemangler_test_file_{}", name));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
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
    assert_eq!(inputs.len(), 9);
    let names: Vec<&str> = inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec![
        "image", "folder", "file name", "format", "quality", "color format",
        "png compression", "auto save", "save",
    ]);
    assert_eq!(OpImageOutputFile::create_outputs().len(), 1);
    // Format defaults to jpg.
    match &inputs[3].value {
        Value::ImageType(f) => assert_eq!(*f, ImageFormat::Jpeg, "format should default to jpg"),
        other => panic!("expected ImageType format input, got {:?}", other),
    }
}

// --- Save-gating (auto save / save button / force) ---

#[tokio::test]
async fn test_file_output_auto_save_off_writes_nothing() {
    // Auto save off, button not pressed, no force → no file, empty path output.
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("autosave_off");
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    inputs[7].value = Value::Bool(false); // auto save off

    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Path(p) => assert!(p.as_os_str().is_empty(), "no write → empty path, got {:?}", p),
        other => panic!("expected Path output, got {:?}", other),
    }
    assert!(!out_path(&dir, "out", ImageFormat::Png).exists(), "no file should be written");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_save_button_writes_once_then_resets() {
    // Pressing the save button (a one-shot Bool(true) pulse) writes once and the
    // pulse is consumed (reset to false) so a later run doesn't re-write.
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("save_button");
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    inputs[7].value = Value::Bool(false); // auto save off
    inputs[8].value = Value::Bool(true); // save button pressed

    let path = out_path(&dir, "out", ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert_save_ok(result, &path);
    // Pulse consumed.
    assert!(matches!(inputs[8].value, Value::Bool(false)), "save pulse should reset to false");

    // A second run with the pulse now false must not write.
    std::fs::remove_file(&path).ok();
    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Path(p) => assert!(p.as_os_str().is_empty(), "second run should not write"),
        other => panic!("expected Path output, got {:?}", other),
    }
    assert!(!path.exists(), "no file should be written on the second run");
    std::fs::remove_dir_all(&dir).ok();
}

// --- Format / color-format coverage ---

#[tokio::test]
async fn test_file_output_rgba32f_saves_png() {
    // Operations like blend, levels, curves output ImageRgba32F.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("rgba32f_png");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_saves_jpeg() {
    // JPEG path uses a separate encoder; Rgb8 is the compatible color format.
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("rgba32f_jpg");
    let path = out_path(&dir, "out", ImageFormat::Jpeg);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Jpeg, ColorFormat::Rgb8);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_saves_bmp() {
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |_, _| image::Rgba([0.3, 0.6, 0.9, 1.0]));
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("rgba32f_bmp");
    let path = out_path(&dir, "out", ImageFormat::Bmp);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Bmp, ColorFormat::Rgb8);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_with_hdr_values_saves() {
    // Some ops produce values > 1.0 (contrast, brighten); must clamp/convert.
    let imgbuf = image::Rgba32FImage::from_fn(4, 4, |_, _| image::Rgba([2.5, -0.3, 1.5, 1.0]));
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("rgba32f_hdrvals");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba8_still_works() {
    let imgbuf = image::RgbaImage::from_fn(8, 8, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));
    let dir = temp_dir("rgba8_png");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_returns_path_on_success() {
    let imgbuf = image::Rgba32FImage::from_fn(4, 4, |_, _| image::Rgba([0.5, 0.5, 0.5, 1.0]));
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("returns_path");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Path(p) => {
            assert!(!p.as_os_str().is_empty(), "output path should not be empty");
            assert!(p.exists(), "output file should exist at the returned path");
            assert_eq!(p, &path, "output path should be folder/name.ext");
        }
        other => panic!("Expected Path output, got {:?}", other),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_defaults_name_to_graph_name() {
    // An empty file name falls back to the graph name from the run context.
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("default_name");
    let mut inputs = make_file_inputs(img, &dir, "", ImageFormat::Png);

    // Install a run context supplying the graph name (as the engine would).
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        graph_dir: Some(dir.clone()),
        graph_name: "my graph".to_string(),
        force_save: false,
    });
    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    drop(_guard);

    match &result.responses[0].value {
        Value::Path(p) => assert_eq!(p, &dir.join("my graph.png"), "empty name → graph name"),
        other => panic!("Expected Path output, got {:?}", other),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_relative_folder_resolves_against_graph_dir() {
    // A relative folder resolves against the graph's directory from the context.
    let base = temp_dir("relative_folder");
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let mut inputs = make_file_inputs(img, Path::new("renders"), "out", ImageFormat::Png);

    let _guard = crate::run_context::set(crate::run_context::RunContext {
        graph_dir: Some(base.clone()),
        graph_name: "g".to_string(),
        force_save: false,
    });
    let result = OpImageOutputFile::run(&mut inputs).await.unwrap();
    drop(_guard);

    let expected = base.join("renders").join("out.png");
    match &result.responses[0].value {
        Value::Path(p) => assert_eq!(p, &expected, "relative folder should join graph dir"),
        other => panic!("Expected Path output, got {:?}", other),
    }
    assert!(expected.exists(), "file should be written under the created subfolder");
    std::fs::remove_dir_all(&base).ok();
}

#[tokio::test]
async fn test_file_output_luma8_saves_png() {
    let imgbuf = image::GrayImage::from_fn(8, 8, |x, _| image::Luma([(x * 32) as u8]));
    let img = float_from_dynamic(DynamicImage::ImageLuma8(imgbuf));
    let dir = temp_dir("luma8_png");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Png, ColorFormat::Gray8);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba16_saves_png() {
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("rgba16_png");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Png, ColorFormat::Rgba16);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba32f_saves_exr() {
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as f32 / 8.0, y as f32 / 8.0, 0.5, 1.0])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("rgba32f_exr");
    let path = out_path(&dir, "out", ImageFormat::OpenExr);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::OpenExr, ColorFormat::Rgba32F);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_gray8_saves_jpeg() {
    let imgbuf = image::GrayImage::from_fn(8, 8, |x, _| image::Luma([(x * 32) as u8]));
    let img = float_from_dynamic(DynamicImage::ImageLuma8(imgbuf));
    let dir = temp_dir("gray8_jpg");
    let path = out_path(&dir, "out", ImageFormat::Jpeg);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Jpeg, ColorFormat::Gray8);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_gray16_saves_png() {
    let imgbuf = image::Rgba32FImage::from_fn(8, 8, |x, _| image::Rgba([x as f32 / 8.0, 0.0, 0.0, 1.0]));
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("gray16_png");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Png, ColorFormat::Gray16);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_incompatible_rgba32f_png_errors() {
    let imgbuf = image::Rgba32FImage::from_fn(4, 4, |_, _| image::Rgba([0.5, 0.5, 0.5, 1.0]));
    let img = float_from_dynamic(DynamicImage::ImageRgba32F(imgbuf));
    let dir = temp_dir("incompat_rgba32f_png");
    let path = out_path(&dir, "out", ImageFormat::Png);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Png, ColorFormat::Rgba32F);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgba32F + PNG should be rejected");
    assert!(result.unwrap_err().node_error.is_some(), "should have a node-level error");
    assert!(!path.exists(), "no file should be created on error");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_incompatible_rgb16_jpeg_errors() {
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("incompat_rgb16_jpg");
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Jpeg, ColorFormat::Rgb16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgb16 + JPEG should be rejected");
    assert!(result.unwrap_err().node_error.is_some(), "should have a node-level error");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_farbfeld_rgba16() {
    let imgbuf = image::RgbaImage::from_fn(8, 8, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));
    let dir = temp_dir("farbfeld_rgba16");
    let path = out_path(&dir, "out", ImageFormat::Farbfeld);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Farbfeld, ColorFormat::Rgba16);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_farbfeld_wrong_format_errors() {
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("farbfeld_wrong");
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Farbfeld, ColorFormat::Rgba8);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "Rgba8 + Farbfeld should be rejected");
    assert!(result.unwrap_err().node_error.is_some(), "should have a node-level error");
    std::fs::remove_dir_all(&dir).ok();
}

// --- Path / name handling ---

#[tokio::test]
async fn test_file_output_dotted_filename_keeps_full_name() {
    // A dotted stem must be preserved: "render.v2" + png → "render.v2.png".
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("dotted_name");
    let path = out_path(&dir, "render.v2", ImageFormat::Png);
    let mut inputs = make_file_inputs(img, &dir, "render.v2", ImageFormat::Png);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_empty_folder_without_graph_dir_errors() {
    // Empty folder + no run context (no graph dir) → nowhere to write.
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let mut inputs = make_file_inputs(img, Path::new(""), "out", ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "empty folder with no graph dir should be rejected");
}

#[tokio::test]
async fn test_file_output_empty_name_and_no_graph_name_errors() {
    // Empty name + no run context (no graph name) → no file name to write.
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("empty_name_err");
    let mut inputs = make_file_inputs(img, &dir, "", ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "empty name with no graph name should be rejected");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_folder_is_file_errors() {
    // A folder input that points at an existing file can't be created as a dir.
    let dir = temp_dir("folder_is_file");
    let file_as_folder = dir.join("not_a_folder.txt");
    std::fs::write(&file_as_folder, "x").unwrap();
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let mut inputs = make_file_inputs(img, &file_as_folder, "out", ImageFormat::Png);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "file used as folder should be rejected");
    std::fs::remove_dir_all(&dir).ok();
}

// --- Encoder settings ---

#[tokio::test]
async fn test_file_output_jpg_quality_affects_size() {
    let imgbuf = image::RgbImage::from_fn(64, 64, |x, y| {
        image::Rgb([(x * 4) as u8, (y * 4) as u8, ((x * y) % 256) as u8])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgb8(imgbuf));
    let dir = temp_dir("jpg_quality");
    let mut sizes = vec![];
    for (name, quality) in [("q10", 10), ("q95", 95)] {
        let path = out_path(&dir, name, ImageFormat::Jpeg);
        let mut inputs = make_file_inputs_with_format(img.clone(), &dir, name, ImageFormat::Jpeg, ColorFormat::Rgb8);
        inputs[4].value = Value::Integer(quality);
        assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
        sizes.push(std::fs::metadata(&path).unwrap().len());
    }
    assert!(sizes[0] < sizes[1], "q10 ({}) should be smaller than q95 ({})", sizes[0], sizes[1]);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_png_compression_levels() {
    let imgbuf = image::RgbaImage::from_fn(32, 32, |x, y| {
        image::Rgba([(x * 8) as u8, (y * 8) as u8, 128, 255])
    });
    let reference = imgbuf.clone();
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));
    let dir = temp_dir("png_compression");
    let mut sizes = std::collections::HashMap::new();
    for level in ["fast", "default", "best", "uncompressed"] {
        let path = out_path(&dir, level, ImageFormat::Png);
        let mut inputs = make_file_inputs(img.clone(), &dir, level, ImageFormat::Png);
        inputs[6].value = Value::Text(level.to_string());
        assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
        let decoded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(decoded.as_raw(), reference.as_raw(), "{} PNG should decode to identical pixels", level);
        sizes.insert(level, std::fs::metadata(&path).unwrap().len());
    }
    assert!(sizes["best"] < sizes["uncompressed"], "best ({}) < uncompressed ({})", sizes["best"], sizes["uncompressed"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_invalid_png_compression_errors() {
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("bad_png_compression");
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Png);
    inputs[6].value = Value::Text("banana".to_string());
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "unknown png compression should be rejected");
    let err = result.unwrap_err();
    assert_eq!(err.input_errors.first().map(|(i, _)| *i), Some(6), "error should point at the png compression input");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgb32f_saves_hdr() {
    let imgbuf = image::Rgb32FImage::from_fn(8, 8, |x, y| image::Rgb([x as f32 / 4.0, y as f32 / 8.0, 2.5]));
    let img = float_from_dynamic(DynamicImage::ImageRgb32F(imgbuf));
    let dir = temp_dir("rgb32f_hdr");
    let path = out_path(&dir, "out", ImageFormat::Hdr);
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Hdr, ColorFormat::Rgb32F);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    let decoded = image::open(&path).unwrap().to_rgb32f();
    let px = decoded.get_pixel(7, 0);
    assert!((px[0] - 1.75).abs() < 0.02, "HDR value above 1.0 should survive, got {}", px[0]);
    assert!((px[2] - 2.5).abs() < 0.02, "HDR value above 1.0 should survive, got {}", px[2]);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_hdr_wrong_color_format_errors() {
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("hdr_wrong_cf");
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Hdr);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "HDR + Rgba8 should be rejected");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_rgba8_saves_avif() {
    let imgbuf = image::RgbaImage::from_fn(16, 16, |x, y| {
        image::Rgba([(x * 16) as u8, (y * 16) as u8, 128, 255])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgba8(imgbuf));
    let dir = temp_dir("rgba8_avif");
    let path = out_path(&dir, "out", ImageFormat::Avif);
    let mut inputs = make_file_inputs(img, &dir, "out", ImageFormat::Avif);
    assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_avif_quality_affects_size() {
    let imgbuf = image::RgbImage::from_fn(64, 64, |x, y| {
        image::Rgb([(x * 4) as u8, (y * 4) as u8, ((x * y) % 256) as u8])
    });
    let img = float_from_dynamic(DynamicImage::ImageRgb8(imgbuf));
    let dir = temp_dir("avif_quality");
    let mut sizes = vec![];
    for (name, quality) in [("q10", 10), ("q95", 95)] {
        let path = out_path(&dir, name, ImageFormat::Avif);
        let mut inputs = make_file_inputs_with_format(img.clone(), &dir, name, ImageFormat::Avif, ColorFormat::Rgb8);
        inputs[4].value = Value::Integer(quality);
        assert_save_ok(OpImageOutputFile::run(&mut inputs).await, &path);
        sizes.push(std::fs::metadata(&path).unwrap().len());
    }
    assert!(sizes[0] < sizes[1], "q10 ({}) should be smaller than q95 ({})", sizes[0], sizes[1]);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_file_output_avif_wrong_color_format_errors() {
    let img = float_from_dynamic(DynamicImage::ImageRgba8(image::RgbaImage::new(4, 4)));
    let dir = temp_dir("avif_wrong_cf");
    let mut inputs = make_file_inputs_with_format(img, &dir, "out", ImageFormat::Avif, ColorFormat::Rgba16);
    let result = OpImageOutputFile::run(&mut inputs).await;
    assert!(result.is_err(), "AVIF + Rgba16 should be rejected");
    std::fs::remove_dir_all(&dir).ok();
}
