//! Channel-packed PBR material export operation.
//!
//! Takes the eight standard PBR maps (albedo, opacity, normal, roughness,
//! metallic, ambient occlusion, height, emission) and an engine preset, and
//! writes a set of channel-packed textures into the chosen folder — one node
//! instead of hand-wiring `channels merge` + several `to file` nodes. The
//! Godot/Unity/Unreal presets pick the file set, packing, normal-space, and bit
//! depth; the Custom preset exposes four free-form texture slots with per-channel
//! source dropdowns. See [`material_presets`] for the specs and packing engine.

use image::ImageFormat;
use image::codecs::png::CompressionType as PngCompression;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType, ColorFormat, ExportPreset};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use super::{image_format_from_path, save_image};
use super::material_presets::{
    builtin_specs, custom_specs, pack_texture, spec_is_writable, TextureSpec, CHANNEL_SOURCE_OPTIONS, MAP_COUNT,
};

/// Operation that exports channel-packed PBR textures for a target engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputMaterial {}

impl OpImageOutputMaterial {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "material".to_string(),
            description: "Exports channel-packed PBR textures for a game engine.".to_string(),
            help: "Packs the standard PBR maps into the file set a target engine expects and writes them next to the chosen base path as `{base}_{suffix}.{ext}` (e.g. choosing `material.png` writes `material_orm.png` in the same folder). A map input counts as connected when it is wired or holds a real (non-1×1) image; only textures that reference at least one connected map are written, and every unconnected map falls back to a neutral constant: albedo 1, opacity 1, normal (0.5, 0.5, 1), roughness 1, metallic 0, ao 1, height 0.5, emission 0. Pixel values are written exactly as stored (sRGB floats, same as the `to file` node); no color-space conversion is applied.\n\nPresets:\n• Godot — albedo (+A=opacity), orm (R=ao, G=roughness, B=metallic), normal (OpenGL Y+, 16-bit), emission, height (16-bit gray).\n• Unity — albedo (+A=opacity), metallic (RGB=metallic, A=1−roughness smoothness, always RGBA), normal (OpenGL Y+, 16-bit), ao (8-bit gray), emission, height (16-bit gray).\n• Unreal — basecolor (+A=opacity), orm, normal (DirectX Y−: green channel inverted, 16-bit), emissive, height (16-bit gray).\n• Custom — four free-form slots: a suffix plus R/G/B/A source dropdowns. An R/G/B source of \"none\" writes 0; an alpha of \"none\" makes a 3-channel file; a \"1 - x\" option inverts. Empty-suffix slots are ignored and duplicate suffixes are an error. The slot inputs are inert under the built-in presets.\n\nThe base path's extension chooses the image format shared by every exported texture — supported: png, jpg/jpeg, gif, webp, pnm, tiff, tga, bmp, ico, hdr, exr, ff (farbfeld), avif, qoi. A texture's preferred 16-bit depth is silently degraded to 8-bit when the chosen format can't hold it; a still-incompatible format (e.g. an alpha texture into JPEG) is rejected before any file is written. Encoding uses fixed defaults (quality 85, PNG fast). Files are (re)written whenever an input changes. The destination folder is returned as an output for chaining.".to_string(),
        }
    }

    /// Creates the 30 inputs. The order is a frozen contract (positional zip
    /// reconcile in graph.rs); future additions must append.
    pub fn create_inputs() -> Vec<Input> {
        let image_input = |name: &str, desc: &str| {
            Input::new(name.to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description(desc)
        };
        let mut inputs = vec![
            // 0..=7 — the eight PBR maps (indices match SourceMap discriminants).
            image_input("albedo", "Base color / diffuse map. Alpha is taken from opacity when connected."),
            image_input("opacity", "Opacity map; supplies the alpha channel of the base color texture when connected."),
            image_input("normal", "Tangent-space normal map (OpenGL Y+ convention; Unreal flips green on export)."),
            image_input("roughness", "Roughness map; also drives Unity smoothness as 1 − roughness."),
            image_input("metallic", "Metallic map."),
            image_input("ambient occlusion", "Ambient occlusion map."),
            image_input("height", "Height / displacement map (exported as 16-bit grayscale)."),
            image_input("emission", "Emissive color map."),
            // 8 — engine preset.
            Input::new("preset".to_string(), Value::ExportPreset(ExportPreset::Godot), None, None)
                .with_description("Target engine convention: chooses the file set, channel packing, normal space, and bit depth."),
            // 9 — base file path. Its extension chooses the image format
            // shared by every exported texture; its stem (extension
            // stripped) is the base for each `{base}_{suffix}.{ext}` file.
            Input::new("file path".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: ValueType::file_extensions(&ValueType::Image),
                set_directory: None,
                set_file_name: None,
                set_title: Some("save material".to_string()),
                file_dialog_type: crate::input::FileDialogType::SaveFile,
            }), None)
                .with_description("Base destination path; its extension selects the image format and its stem is reused as the base of every exported file's name."),
        ];

        // 10..=29 — four Custom slots. Slot `s` channel `offset` is at index
        // `10 + s*5 + offset` (offset 0 = suffix, 1..=4 = r/g/b/a). Only used
        // when preset = Custom.
        let dropdown = || Some(InputSettings::Dropdown {
            options: CHANNEL_SOURCE_OPTIONS.iter().map(|s| s.to_string()).collect(),
        });
        for slot in 0..4 {
            let n = slot + 1;
            inputs.push(
                Input::new(format!("texture {} suffix", n), Value::Text(String::new()), Some(InputSettings::SingleLineText), None)
                    .with_description("Custom texture file suffix; leave empty to disable this slot (Custom preset only)."),
            );
            for chan in ["r", "g", "b", "a"] {
                inputs.push(
                    Input::new(format!("texture {} {}", n, chan), Value::Text("none".to_string()), dropdown(), None)
                        .with_description("Source for this channel of the custom texture (Custom preset only)."),
                );
            }
        }

        inputs
    }

    /// Creates the single output: the folder the textures were written to.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("folder".to_string(), Value::Path(PathBuf::new()), None)
                .with_description("Folder the packed textures were written to."),
        ]
    }

    /// Executes the operation: packs and writes the preset's texture set.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert the core inputs (maps 0..8, preset, file path).
        let mut map_values: Vec<Option<Value>> = Vec::with_capacity(MAP_COUNT);
        for m in 0..MAP_COUNT {
            map_values.push(convert_input(inputs, m, ValueType::Image, &mut input_errors));
        }
        let preset_converted = convert_input(inputs, 8, ValueType::ExportPreset, &mut input_errors);
        let path_converted = convert_input(inputs, 9, ValueType::Path, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Extract map data and the connected/provided flags (connection wired or
        // a real non-1×1 image — mirrors simulation::is_unconnected, inverted).
        let mut map_data: Vec<Arc<crate::float_image::FloatImage>> = Vec::with_capacity(MAP_COUNT);
        let mut provided = [false; MAP_COUNT];
        for (m, value) in map_values.into_iter().enumerate() {
            let Value::Image { data, change_id: _ } = value.unwrap() else { unreachable!() };
            let (w, h) = data.dimensions();
            provided[m] = inputs[m].connection.is_some() || !(w <= 1 && h <= 1);
            map_data.push(data);
        }
        let Value::ExportPreset(preset) = preset_converted.unwrap() else { unreachable!() };
        let Value::Path(path) = path_converted.unwrap() else { unreachable!() };

        // Validation: at least one map, a non-empty path with a recognized
        // extension, and an existing destination folder.
        let Some(first) = provided.iter().position(|&p| p) else {
            return Err(OperationError { input_errors: vec![], node_error: Some("No map inputs are connected; nothing to export.".to_string()) });
        };
        if path.as_os_str().is_empty() {
            let msg = "File path is empty.".to_string();
            return Err(OperationError { input_errors: vec![(9, msg.clone())], node_error: Some(msg) });
        }
        let image_format = match image_format_from_path(&path) {
            Ok(f) => f,
            Err(msg) => {
                return Err(OperationError { input_errors: vec![(9, msg.clone())], node_error: Some(msg) });
            }
        };
        let base_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("material").to_string();
        let folder = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        if !folder.as_os_str().is_empty() && !folder.is_dir() {
            return Err(OperationError { input_errors: vec![(9, "Folder does not exist or is not a directory.".to_string())], node_error: Some("Folder does not exist or is not a directory.".to_string()) });
        }

        // Export size = first provided map; resize the other provided maps once
        // and cache them. Unconnected maps stay None (resolved to constants).
        let (out_w, out_h) = map_data[first].dimensions();
        let mut maps: [Option<Arc<crate::float_image::FloatImage>>; MAP_COUNT] = Default::default();
        for m in 0..MAP_COUNT {
            if !provided[m] { continue; }
            let data = &map_data[m];
            maps[m] = if data.dimensions() == (out_w, out_h) {
                Some(Arc::clone(data))
            } else {
                Some(Arc::new(data.resize(out_w, out_h)))
            };
        }

        // Build the texture specs.
        let specs: Vec<TextureSpec> = if preset == ExportPreset::Custom {
            let slots = Self::read_custom_slots(inputs);
            match custom_specs(&slots) {
                Ok(s) => s,
                Err(e) => {
                    let index = 10 + e.slot * 5 + e.offset;
                    return Err(OperationError { input_errors: vec![(index, e.message.clone())], node_error: Some(e.message) });
                }
            }
        } else {
            builtin_specs(preset, &provided)
        };

        // Write each writable spec, resolving/degrading the color format per file.
        for spec in specs.iter().filter(|s| spec_is_writable(s, &provided)) {
            let color_format = match resolve_format(spec.preferred_format, image_format) {
                Some(f) => f,
                None => {
                    let msg = format!(
                        "{:?} cannot store the '{}' texture ({:?}).",
                        image_format, spec.suffix, spec.preferred_format
                    );
                    return Err(OperationError { input_errors: vec![(9, msg.clone())], node_error: Some(msg) });
                }
            };
            let packed = pack_texture(spec, &maps, out_w, out_h);
            let ext = image_format.extensions_str()[0];
            let out_path = path.with_file_name(format!("{}_{}.{}", base_stem, spec.suffix, ext));
            if let Err(e) = save_image(&out_path, &packed, &color_format, image_format, 85, PngCompression::Fast) {
                return Err(OperationError { input_errors: vec![], node_error: Some(format!("Failed to save '{}': {}", spec.suffix, e)) });
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Path(folder) }],
        })
    }

    /// Reads the four Custom slots (suffix + r/g/b/a source strings) from the
    /// inputs at their frozen indices, tolerating a short input vec.
    fn read_custom_slots(inputs: &[Input]) -> [(String, [String; 4]); 4] {
        let text_at = |idx: usize| -> String {
            match inputs.get(idx).map(|i| &i.value) {
                Some(Value::Text(t)) => t.clone(),
                _ => String::new(),
            }
        };
        let mut slots: [(String, [String; 4]); 4] = Default::default();
        for slot in 0..4 {
            let base = 10 + slot * 5;
            slots[slot] = (
                text_at(base),
                [text_at(base + 1), text_at(base + 2), text_at(base + 3), text_at(base + 4)],
            );
        }
        slots
    }
}

/// Resolve a spec's preferred color format against the chosen image format:
/// return it as-is if compatible, silently degrade 16-bit → 8-bit and retry, or
/// `None` if still incompatible.
fn resolve_format(preferred: ColorFormat, image_format: ImageFormat) -> Option<ColorFormat> {
    if preferred.is_compatible_with_image_format(&image_format) {
        return Some(preferred);
    }
    let degraded = match preferred {
        ColorFormat::Rgba16 => ColorFormat::Rgba8,
        ColorFormat::Rgb16 => ColorFormat::Rgb8,
        ColorFormat::GrayA16 => ColorFormat::GrayA8,
        ColorFormat::Gray16 => ColorFormat::Gray8,
        other => other,
    };
    if degraded != preferred && degraded.is_compatible_with_image_format(&image_format) {
        Some(degraded)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "material_tests.rs"]
mod tests;
