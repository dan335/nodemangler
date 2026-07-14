use super::*;
use crate::input::InputSettings;
use std::path::PathBuf;

#[test]
fn test_image_format_from_path_common_extensions() {
    assert_eq!(image_format_from_path(&PathBuf::from("out.png")), Ok(ImageFormat::Png));
    assert_eq!(image_format_from_path(&PathBuf::from("out.jpg")), Ok(ImageFormat::Jpeg));
    assert_eq!(image_format_from_path(&PathBuf::from("out.jpeg")), Ok(ImageFormat::Jpeg));
    assert_eq!(image_format_from_path(&PathBuf::from("out.bmp")), Ok(ImageFormat::Bmp));
    assert_eq!(image_format_from_path(&PathBuf::from("out.avif")), Ok(ImageFormat::Avif));
    assert_eq!(image_format_from_path(&PathBuf::from("out.exr")), Ok(ImageFormat::OpenExr));
    assert_eq!(image_format_from_path(&PathBuf::from("out.hdr")), Ok(ImageFormat::Hdr));
    assert_eq!(image_format_from_path(&PathBuf::from("out.ff")), Ok(ImageFormat::Farbfeld));
}

#[test]
fn test_image_format_from_path_is_case_insensitive() {
    assert_eq!(image_format_from_path(&PathBuf::from("OUT.PNG")), Ok(ImageFormat::Png));
    assert_eq!(image_format_from_path(&PathBuf::from("out.Jpg")), Ok(ImageFormat::Jpeg));
}

#[test]
fn test_image_format_from_path_with_dotted_stem() {
    // A dot in the stem must not be mistaken for the extension boundary; the
    // real extension (last component) still drives format selection.
    assert_eq!(image_format_from_path(&PathBuf::from("render.v2.png")), Ok(ImageFormat::Png));
}

#[test]
fn test_image_format_from_path_missing_extension_errors() {
    let err = image_format_from_path(&PathBuf::from("out")).unwrap_err();
    assert!(err.contains("no extension"), "error should mention missing extension: {}", err);
    assert!(err.contains("png"), "error should list supported extensions: {}", err);
}

#[test]
fn test_image_format_from_path_unknown_extension_errors() {
    let err = image_format_from_path(&PathBuf::from("out.dds")).unwrap_err();
    assert!(err.contains("dds"), "error should mention the bad extension: {}", err);
    assert!(err.contains("png"), "error should list supported extensions: {}", err);
}

// --- should_save_and_consume: honor_force -----------------------------------
//
// `honor_force` was added so a batch run can drive `to file`/`material`
// (which always honor the engine's force-save flag, `honor_force = true`)
// differently from `to clipboard` (which opts out mid-batch so hundreds of
// iterations don't rewrite the clipboard hundreds of times).

/// Build the two save-gate inputs (`auto save`, `save`) at the indices
/// `should_save_and_consume` expects (0, 1) for direct unit tests.
fn gate_inputs(auto: bool, pulse: bool) -> Vec<Input> {
    vec![
        Input::new("auto save".to_string(), Value::Bool(auto), None, None),
        Input::new("save".to_string(), Value::Bool(pulse), Some(InputSettings::Button), None),
    ]
}

#[test]
fn test_should_save_honor_force_false_ignores_forced_flag() {
    // Neither the auto-save toggle nor the save pulse is set, so the only
    // thing that could make this write is the engine's force-save flag —
    // which `honor_force = false` (as `to clipboard` passes while a batch
    // item is in flight) must ignore.
    let mut inputs = gate_inputs(false, false);
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        force_save: true,
        ..Default::default()
    });
    assert!(!should_save_and_consume(&mut inputs, 0, 1, false));
}

#[test]
fn test_should_save_honor_force_true_respects_forced_flag() {
    // Same setup, but `honor_force = true` (as `to file`/`material` always
    // pass) means the forced flag counts.
    let mut inputs = gate_inputs(false, false);
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        force_save: true,
        ..Default::default()
    });
    assert!(should_save_and_consume(&mut inputs, 0, 1, true));
}

#[test]
fn test_should_save_pulse_consumed_regardless_of_honor_force() {
    // A manual click (the `save` pulse) forces a write independent of
    // `honor_force`/the engine's force flag, and the pulse is still reset to
    // `false` afterwards so a later reactive run doesn't re-save.
    let mut inputs = gate_inputs(false, true);
    assert!(should_save_and_consume(&mut inputs, 0, 1, false));
    assert!(matches!(inputs[1].value, Value::Bool(false)));
}

// --- resolve_output_dir_and_stem: batch-aware file name ----------------------
//
// `batch_item_stem` was added so an output node can name its file after the
// source item driving the current batch iteration instead of overwriting one
// file on every iteration.

/// An absolute stand-in output folder. `resolve_output_dir_and_stem` never
/// touches the filesystem (see its doc comment), so this doesn't need to
/// exist on disk — it just needs to be non-empty and absolute so folder
/// resolution itself isn't what's under test here.
fn out_dir() -> PathBuf {
    std::env::temp_dir().join("nodemangler_test_resolve_stem")
}

#[test]
fn test_resolve_stem_no_batch_empty_name_falls_back_to_graph_name() {
    // Unchanged pre-batch behavior: no batch item, empty file name -> the
    // graph's name.
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        graph_name: "my graph".to_string(),
        ..Default::default()
    });
    let (_, stem) = resolve_output_dir_and_stem(&out_dir(), "", 0, 1, false).unwrap();
    assert_eq!(stem, "my graph");
}

#[test]
fn test_resolve_stem_no_batch_literal_name_is_verbatim() {
    // Unchanged pre-batch behavior: no batch item, a literal file name is
    // used as-is (whether or not it happens to be wired).
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        graph_name: "my graph".to_string(),
        ..Default::default()
    });
    let (_, stem) = resolve_output_dir_and_stem(&out_dir(), "out_1", 0, 1, false).unwrap();
    assert_eq!(stem, "out_1");
}

#[test]
fn test_resolve_stem_batch_empty_name_uses_item_stem() {
    // Mid-batch, an empty file name resolves to the current source item's
    // stem instead of the graph's name.
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        graph_name: "my graph".to_string(),
        batch_item_stem: Some("photo".to_string()),
        ..Default::default()
    });
    let (_, stem) = resolve_output_dir_and_stem(&out_dir(), "", 0, 1, false).unwrap();
    assert_eq!(stem, "photo");
}

#[test]
fn test_resolve_stem_batch_unwired_literal_gets_item_suffix() {
    // Mid-batch, an unwired literal file name is a template reused by every
    // iteration (fresh output nodes are pre-filled with a literal
    // `{graph}_{N}` stem) — decorate it with the item stem so iterations
    // don't overwrite each other.
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        batch_item_stem: Some("photo".to_string()),
        ..Default::default()
    });
    let (_, stem) = resolve_output_dir_and_stem(&out_dir(), "out_1", 0, 1, false).unwrap();
    assert_eq!(stem, "out_1_photo");
}

#[test]
fn test_resolve_stem_batch_wired_literal_stays_verbatim() {
    // Mid-batch, a *connected* file name (e.g. wired from a from-folder
    // node's `file name` output) is the user's explicit per-item choice —
    // never decorated.
    let _guard = crate::run_context::set(crate::run_context::RunContext {
        batch_item_stem: Some("photo".to_string()),
        ..Default::default()
    });
    let (_, stem) = resolve_output_dir_and_stem(&out_dir(), "out_1", 0, 1, true).unwrap();
    assert_eq!(stem, "out_1");
}
