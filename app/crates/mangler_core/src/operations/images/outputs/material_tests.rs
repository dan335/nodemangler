use super::*;
use crate::float_image::FloatImage;
use crate::operations::OperationResponse;

/// Builds a `w × h` image with the given per-pixel channel `values`.
fn solid(w: u32, h: u32, values: &[f32]) -> Arc<FloatImage> {
    let ch = values.len() as u32;
    let mut data = Vec::with_capacity((w * h) as usize * values.len());
    for _ in 0..(w * h) {
        data.extend_from_slice(values);
    }
    Arc::new(FloatImage::from_raw(w, h, ch, data).unwrap())
}

/// The 30-input vec with defaults; helpers below wire specific maps/settings.
fn base_inputs() -> Vec<Input> {
    OpImageOutputMaterial::create_inputs()
}

/// Marks a map input as connected with a real image.
fn set_map(inputs: &mut [Input], map: usize, img: Arc<FloatImage>) {
    inputs[map].value = Value::Image { data: img, change_id: get_id() };
    inputs[map].connection = Some(("upstream".to_string(), 0));
}

fn set_preset(inputs: &mut [Input], preset: ExportPreset) {
    inputs[8].value = Value::ExportPreset(preset);
}

/// Sets the base file path (index 9); its extension drives the format and
/// its stem is reused as the base name for every exported file.
fn set_path(inputs: &mut [Input], path: &std::path::Path) {
    inputs[9].value = Value::Path(path.to_path_buf());
}

/// Convenience: set the base path from a folder + base name + format,
/// mirroring the old `set_name`/`set_folder`/`set_format` triple.
fn set_name_folder_format(inputs: &mut [Input], folder: &std::path::Path, name: &str, format: ImageFormat) {
    let ext = format.extensions_str()[0];
    set_path(inputs, &folder.join(format!("{}.{}", name, ext)));
}

/// Creates a fresh temp dir for a test.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nodemangler_test_material_{}", name));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &std::path::Path) {
    std::fs::remove_dir_all(dir).ok();
}

fn ok(result: Result<OperationResponse, OperationError>) -> OperationResponse {
    match result {
        Ok(r) => r,
        Err(e) => panic!("material export should succeed, got {:?}", e),
    }
}

// ① Settings freeze — the input order is a forever-frozen contract.
#[tokio::test]
async fn test_material_settings_freeze() {
    let s = OpImageOutputMaterial::settings();
    assert_eq!(s.name, "material");
    let inputs = OpImageOutputMaterial::create_inputs();
    assert_eq!(inputs.len(), 30, "input count is frozen at 30");
    let names: Vec<&str> = inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(&names[0..10], &[
        "albedo", "opacity", "normal", "roughness", "metallic", "ambient occlusion",
        "height", "emission", "preset", "file path",
    ]);
    // Custom slots at 10 + slot*5 + offset.
    assert_eq!(names[10], "texture 1 suffix");
    assert_eq!(names[11], "texture 1 r");
    assert_eq!(names[14], "texture 1 a");
    assert_eq!(names[15], "texture 2 suffix");
    assert_eq!(names[25], "texture 4 suffix");
    assert_eq!(names[29], "texture 4 a");
    let outputs = OpImageOutputMaterial::create_outputs();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].name, "folder");
}

// ② Godot ORM channel values.
#[tokio::test]
async fn test_material_godot_orm_channels() {
    let dir = temp_dir("godot_orm");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "mat", ImageFormat::Png);
    set_map(&mut inputs, 5, solid(4, 4, &[1.0])); // ao = 1.0
    set_map(&mut inputs, 3, solid(4, 4, &[0.5])); // roughness = 0.5
    set_map(&mut inputs, 4, solid(4, 4, &[0.25])); // metallic = 0.25
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    let orm = image::open(dir.join("mat_orm.png")).unwrap().to_rgb8();
    let px = orm.get_pixel(0, 0);
    assert!((px[0] as i32 - 255).abs() <= 1, "R = ao ~ 255, got {}", px[0]);
    assert!((px[1] as i32 - 128).abs() <= 1, "G = roughness ~ 128, got {}", px[1]);
    assert!((px[2] as i32 - 64).abs() <= 1, "B = metallic ~ 64, got {}", px[2]);
    cleanup(&dir);
}

// ③ Albedo is 3-channel without opacity, RGBA with it.
#[tokio::test]
async fn test_material_albedo_alpha_with_opacity() {
    let dir = temp_dir("albedo_alpha");
    // Without opacity: 3-channel file.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "a", ImageFormat::Png);
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.6, 0.7]));
    ok(OpImageOutputMaterial::run(&mut inputs).await);
    let img = image::open(dir.join("a_albedo.png")).unwrap();
    assert_eq!(img.color().channel_count(), 3, "no opacity -> RGB");

    // With opacity: 4-channel file.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "b", ImageFormat::Png);
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.6, 0.7]));
    set_map(&mut inputs, 1, solid(4, 4, &[0.4]));
    ok(OpImageOutputMaterial::run(&mut inputs).await);
    let img = image::open(dir.join("b_albedo.png")).unwrap();
    assert_eq!(img.color().channel_count(), 4, "opacity -> RGBA");
    let a = img.to_rgba8().get_pixel(0, 0)[3];
    assert!((a as i32 - 102).abs() <= 1, "alpha = opacity 0.4 ~ 102, got {}", a);
    cleanup(&dir);
}

// ④ Unreal normal G-flip vs Godot no-flip (16-bit).
#[tokio::test]
async fn test_material_normal_green_flip() {
    let dir = temp_dir("normal_flip");
    let normal = solid(4, 4, &[0.2, 0.7, 1.0]);

    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "g", ImageFormat::Png);
    set_map(&mut inputs, 2, normal.clone());
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Unreal);
    set_name_folder_format(&mut inputs, &dir, "u", ImageFormat::Png);
    set_map(&mut inputs, 2, normal);
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    let godot = image::open(dir.join("g_normal.png")).unwrap().to_rgb16();
    let unreal = image::open(dir.join("u_normal.png")).unwrap().to_rgb16();
    let gg = godot.get_pixel(0, 0)[1];
    let ug = unreal.get_pixel(0, 0)[1];
    assert!((gg as i32 - 45875).abs() <= 2, "Godot G ~ 0.7*65535, got {}", gg);
    assert!((ug as i32 - 19660).abs() <= 2, "Unreal G ~ 0.3*65535, got {}", ug);
    // R and B match between the two (no flip there).
    assert_eq!(godot.get_pixel(0, 0)[0], unreal.get_pixel(0, 0)[0]);
    assert_eq!(godot.get_pixel(0, 0)[2], unreal.get_pixel(0, 0)[2]);
    cleanup(&dir);
}

// ⑤ Unity smoothness A ≈ 1 − roughness.
#[tokio::test]
async fn test_material_unity_smoothness() {
    let dir = temp_dir("unity_smooth");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Unity);
    set_name_folder_format(&mut inputs, &dir, "m", ImageFormat::Png);
    set_map(&mut inputs, 4, solid(4, 4, &[0.9])); // metallic
    set_map(&mut inputs, 3, solid(4, 4, &[0.25])); // roughness
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    let metallic = image::open(dir.join("m_metallic.png")).unwrap().to_rgba8();
    let px = metallic.get_pixel(0, 0);
    assert!((px[0] as i32 - 230).abs() <= 1, "RGB = metallic 0.9 ~ 230, got {}", px[0]);
    assert!((px[3] as i32 - 191).abs() <= 1, "A = 1 - 0.25 = 0.75 ~ 191, got {}", px[3]);
    cleanup(&dir);
}

// ⑥ Only textures referencing a connected map are written.
#[tokio::test]
async fn test_material_skips_unconnected() {
    let dir = temp_dir("skip_unconnected");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "s", ImageFormat::Png);
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    assert!(dir.join("s_albedo.png").exists());
    assert!(!dir.join("s_orm.png").exists());
    assert!(!dir.join("s_normal.png").exists());
    assert!(!dir.join("s_emission.png").exists());
    assert!(!dir.join("s_height.png").exists());
    cleanup(&dir);
}

// ⑦ Unconnected maps fall back to their neutral constants inside ORM.
#[tokio::test]
async fn test_material_fallback_constants_in_orm() {
    let dir = temp_dir("fallback_orm");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "f", ImageFormat::Png);
    set_map(&mut inputs, 4, solid(4, 4, &[0.5])); // only metallic connected
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    let orm = image::open(dir.join("f_orm.png")).unwrap().to_rgb8();
    let px = orm.get_pixel(0, 0);
    assert_eq!(px[0], 255, "ao default 1.0");
    assert_eq!(px[1], 255, "roughness default 1.0");
    assert!((px[2] as i32 - 128).abs() <= 1, "metallic 0.5 ~ 128, got {}", px[2]);
    cleanup(&dir);
}

// ⑧ Custom slots end-to-end; empty-suffix ignored; inert under Godot.
#[tokio::test]
async fn test_material_custom_slot() {
    let dir = temp_dir("custom_slot");

    // Custom preset: slot 1 builds a "mask" texture using 1 - roughness in R;
    // slot 2 has an empty suffix and is ignored.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Custom);
    set_name_folder_format(&mut inputs, &dir, "c", ImageFormat::Png);
    set_map(&mut inputs, 3, solid(4, 4, &[0.25])); // roughness
    inputs[10].value = Value::Text("mask".to_string());
    inputs[11].value = Value::Text("1 - roughness".to_string()); // r
    inputs[12].value = Value::Text("none".to_string()); // g
    inputs[13].value = Value::Text("none".to_string()); // b
    inputs[14].value = Value::Text("none".to_string()); // a
    // slot 2 left with empty suffix.
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    assert!(dir.join("c_mask.png").exists());
    let mask = image::open(dir.join("c_mask.png")).unwrap().to_rgb8();
    let px = mask.get_pixel(0, 0);
    assert!((px[0] as i32 - 191).abs() <= 1, "R = 1 - 0.25 = 0.75 ~ 191, got {}", px[0]);
    assert_eq!(px[1], 0, "G none -> 0");

    // The same slots are inert under Godot (built-in specs used instead).
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "d", ImageFormat::Png);
    set_map(&mut inputs, 3, solid(4, 4, &[0.25]));
    inputs[10].value = Value::Text("mask".to_string());
    inputs[11].value = Value::Text("1 - roughness".to_string());
    ok(OpImageOutputMaterial::run(&mut inputs).await);
    assert!(!dir.join("d_mask.png").exists(), "custom slots inert under Godot");
    assert!(dir.join("d_orm.png").exists(), "Godot orm written (roughness connected)");
    cleanup(&dir);
}

// ⑨ Custom parse error maps to the exact input index 10 + slot*5 + offset.
#[tokio::test]
async fn test_material_custom_error_index() {
    let dir = temp_dir("custom_error");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Custom);
    set_name_folder_format(&mut inputs, &dir, "e", ImageFormat::Png);
    set_map(&mut inputs, 3, solid(4, 4, &[0.5]));
    // slot 1 (second slot), green channel (offset 2): garbage.
    inputs[15].value = Value::Text("tex".to_string()); // slot 1 suffix
    inputs[16].value = Value::Text("albedo.r".to_string()); // r
    inputs[17].value = Value::Text("banana".to_string()); // g -> error
    let err = OpImageOutputMaterial::run(&mut inputs).await.unwrap_err();
    let expected = 10 + 1 * 5 + 2; // = 17
    assert_eq!(err.input_errors.first().map(|(i, _)| *i), Some(expected));
    cleanup(&dir);
}

// ⑩ Mixed input sizes -> output uses the first provided map's size.
#[tokio::test]
async fn test_material_mixed_sizes() {
    let dir = temp_dir("mixed_sizes");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "sz", ImageFormat::Png);
    set_map(&mut inputs, 0, solid(8, 8, &[0.5, 0.5, 0.5])); // albedo first, 8x8
    set_map(&mut inputs, 3, solid(4, 4, &[0.5])); // roughness 4x4
    ok(OpImageOutputMaterial::run(&mut inputs).await);

    let albedo = image::open(dir.join("sz_albedo.png")).unwrap();
    assert_eq!(albedo.width(), 8);
    assert_eq!(albedo.height(), 8);
    let orm = image::open(dir.join("sz_orm.png")).unwrap();
    assert_eq!((orm.width(), orm.height()), (8, 8), "resized to first map size");
    cleanup(&dir);
}

// ⑪ JPEG + alpha texture errors at input 9 (the file path); JPEG + height
// degrades to 8-bit.
#[tokio::test]
async fn test_material_format_policy() {
    let dir = temp_dir("format_policy");

    // JPEG cannot hold the RGBA albedo (opacity connected) -> hard error at 9.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "j", ImageFormat::Jpeg);
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    set_map(&mut inputs, 1, solid(4, 4, &[0.5]));
    let err = OpImageOutputMaterial::run(&mut inputs).await.unwrap_err();
    assert_eq!(err.input_errors.first().map(|(i, _)| *i), Some(9));

    // JPEG + height silently degrades Gray16 -> Gray8 and succeeds.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "h", ImageFormat::Jpeg);
    set_map(&mut inputs, 6, solid(4, 4, &[0.5]));
    ok(OpImageOutputMaterial::run(&mut inputs).await);
    assert!(dir.join("h_height.jpg").exists());
    cleanup(&dir);
}

// ⑫ Empty path, unknown extension, missing parent folder, and no-maps are rejected.
#[tokio::test]
async fn test_material_path_and_folder_errors() {
    let dir = temp_dir("name_folder");

    // Empty path -> error at input 9.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    let err = OpImageOutputMaterial::run(&mut inputs).await.unwrap_err();
    assert_eq!(err.input_errors.first().map(|(i, _)| *i), Some(9), "empty path -> input 9");

    // Unrecognized extension -> error at input 9, before any file is written.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_path(&mut inputs, &dir.join("ok.dds"));
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    let err = OpImageOutputMaterial::run(&mut inputs).await.unwrap_err();
    assert_eq!(err.input_errors.first().map(|(i, _)| *i), Some(9), "unknown extension -> input 9");
    assert!(!dir.join("ok_albedo.dds").exists(), "no file should be created on error");

    // Missing parent folder -> error.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_path(&mut inputs, &std::path::Path::new("/this/does/not/exist/at/all").join("ok.png"));
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    assert!(OpImageOutputMaterial::run(&mut inputs).await.is_err(), "bad folder rejected");

    // No maps connected -> error.
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "empty", ImageFormat::Png);
    assert!(OpImageOutputMaterial::run(&mut inputs).await.is_err(), "no maps -> error");
    cleanup(&dir);
}

// ⑬ Returns the destination folder (the path's parent) as a Path output.
#[tokio::test]
async fn test_material_returns_folder() {
    let dir = temp_dir("returns_folder");
    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_name_folder_format(&mut inputs, &dir, "r", ImageFormat::Png);
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    let response = ok(OpImageOutputMaterial::run(&mut inputs).await);
    match &response.responses[0].value {
        Value::Path(p) => assert_eq!(p, &dir),
        other => panic!("expected Path output, got {:?}", other),
    }
    cleanup(&dir);
}

// ⑭ File path's extension drives the format (.png -> PNG, .jpg -> JPEG).
#[tokio::test]
async fn test_material_extension_drives_format() {
    let dir = temp_dir("extension_drives_format");

    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_path(&mut inputs, &dir.join("mat.png"));
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    ok(OpImageOutputMaterial::run(&mut inputs).await);
    assert!(dir.join("mat_albedo.png").exists());

    let mut inputs = base_inputs();
    set_preset(&mut inputs, ExportPreset::Godot);
    set_path(&mut inputs, &dir.join("mat.jpg"));
    set_map(&mut inputs, 0, solid(4, 4, &[0.5, 0.5, 0.5]));
    ok(OpImageOutputMaterial::run(&mut inputs).await);
    assert!(dir.join("mat_albedo.jpg").exists());

    cleanup(&dir);
}
