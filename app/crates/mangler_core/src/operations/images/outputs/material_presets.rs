//! Channel-packing specs, parser, and engine for the material export node.
//!
//! Pure logic, no I/O: both the built-in engine presets (Godot/Unity/Unreal)
//! and the parsed Custom slots produce a common [`TextureSpec`]; the single
//! [`pack_texture`] consumer turns a spec plus the eight source maps into one
//! packed `FloatImage`. `material.rs` owns file names, resolution, and the
//! color-format/degrade policy.

use crate::float_image::FloatImage;
use crate::value::{ColorFormat, ExportPreset};
use rayon::prelude::*;
use std::sync::Arc;

/// Minimum pixel count before packing is parallelized over rows (matches
/// `channels/merge.rs`).
const PARALLEL_PIXELS: usize = 1 << 16;

/// Number of PBR map inputs (albedo … emission), in input-index order.
pub(crate) const MAP_COUNT: usize = 8;

/// One of the eight PBR map inputs. Discriminant order matches the node's
/// input indices exactly (`SourceMap::Albedo as usize == 0`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceMap {
    Albedo,
    Opacity,
    Normal,
    Roughness,
    Metallic,
    AmbientOcclusion,
    Height,
    Emission,
}

/// Which scalar to read out of a source map's pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceChannel {
    R,
    G,
    B,
    A,
    /// Rec. 601 luminance for ≥3-channel sources, first channel otherwise.
    Luma,
}

/// One output channel of a packed texture.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PackedChannel {
    /// A fixed value written to every pixel (e.g. an unused custom R/G/B slot).
    Constant(f32),
    /// A value sampled from a source map, optionally inverted (`1 - x`), used
    /// for the Unreal DirectX normal G-flip and the Unity smoothness channel.
    Source {
        map: SourceMap,
        channel: SourceChannel,
        invert: bool,
    },
}

/// A single output texture: file suffix, per-channel packing recipe (length
/// 1, 3, or 4), and the preferred color format before any degrade.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextureSpec {
    pub suffix: String,
    pub channels: Vec<PackedChannel>,
    pub preferred_format: ColorFormat,
}

/// Error from parsing the four Custom slots. `offset` maps to the input widget:
/// 0 = suffix, 1..=4 = the r/g/b/a source dropdowns, so the caller can point an
/// `input_errors` entry at `12 + slot*5 + offset`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CustomSlotError {
    pub slot: usize,
    pub offset: usize,
    pub message: String,
}

/// The 24 channel-source options offered by every Custom slot dropdown, shared
/// verbatim between `create_inputs()` and [`parse_channel_source`].
pub(crate) const CHANNEL_SOURCE_OPTIONS: &[&str] = &[
    "none",
    "roughness",
    "1 - roughness",
    "metallic",
    "1 - metallic",
    "ao",
    "1 - ao",
    "height",
    "1 - height",
    "opacity",
    "1 - opacity",
    "albedo",
    "albedo.r",
    "albedo.g",
    "albedo.b",
    "albedo.a",
    "normal",
    "normal.r",
    "normal.g",
    "normal.b",
    "emission",
    "emission.r",
    "emission.g",
    "emission.b",
];

/// Shorthand for a non-inverted source channel.
fn src(map: SourceMap, channel: SourceChannel) -> PackedChannel {
    PackedChannel::Source { map, channel, invert: false }
}

/// Shorthand for an inverted (`1 - x`) source channel.
fn src_inv(map: SourceMap, channel: SourceChannel) -> PackedChannel {
    PackedChannel::Source { map, channel, invert: true }
}

/// Build the texture specs for a built-in engine preset. `connected` marks which
/// maps have real data; it only controls whether the base color texture gains an
/// alpha channel (opacity). Whether each spec is actually written is decided
/// separately by [`spec_is_writable`].
pub(crate) fn builtin_specs(preset: ExportPreset, connected: &[bool; MAP_COUNT]) -> Vec<TextureSpec> {
    use SourceChannel::*;
    use SourceMap::*;

    let has_opacity = connected[Opacity as usize];
    // Base color: RGB from albedo, plus A = opacity only when opacity is wired.
    let base_color = |suffix: &str| -> TextureSpec {
        let mut channels = vec![src(Albedo, R), src(Albedo, G), src(Albedo, B)];
        let preferred_format = if has_opacity {
            channels.push(src(Opacity, Luma));
            ColorFormat::Rgba8
        } else {
            ColorFormat::Rgb8
        };
        TextureSpec { suffix: suffix.to_string(), channels, preferred_format }
    };
    // ORM: R = AO, G = roughness, B = metallic (glTF / ORMMaterial3D layout).
    let orm = || TextureSpec {
        suffix: "orm".to_string(),
        channels: vec![src(AmbientOcclusion, Luma), src(Roughness, Luma), src(Metallic, Luma)],
        preferred_format: ColorFormat::Rgb8,
    };
    let emission = |suffix: &str| TextureSpec {
        suffix: suffix.to_string(),
        channels: vec![src(Emission, R), src(Emission, G), src(Emission, B)],
        preferred_format: ColorFormat::Rgb8,
    };
    let height = || TextureSpec {
        suffix: "height".to_string(),
        channels: vec![src(Height, Luma)],
        preferred_format: ColorFormat::Gray16,
    };
    // OpenGL (Y+) normal — Godot/Unity.
    let normal_gl = || TextureSpec {
        suffix: "normal".to_string(),
        channels: vec![src(Normal, R), src(Normal, G), src(Normal, B)],
        preferred_format: ColorFormat::Rgb16,
    };
    // DirectX (Y−) normal — Unreal: green channel inverted.
    let normal_dx = || TextureSpec {
        suffix: "normal".to_string(),
        channels: vec![src(Normal, R), src_inv(Normal, G), src(Normal, B)],
        preferred_format: ColorFormat::Rgb16,
    };

    match preset {
        ExportPreset::Godot => vec![base_color("albedo"), orm(), normal_gl(), emission("emission"), height()],
        ExportPreset::Unity => vec![
            base_color("albedo"),
            // Metallic-smoothness: RGB = metallic, A = 1 − roughness. Always Rgba8.
            TextureSpec {
                suffix: "metallic".to_string(),
                channels: vec![src(Metallic, Luma), src(Metallic, Luma), src(Metallic, Luma), src_inv(Roughness, Luma)],
                preferred_format: ColorFormat::Rgba8,
            },
            normal_gl(),
            TextureSpec { suffix: "ao".to_string(), channels: vec![src(AmbientOcclusion, Luma)], preferred_format: ColorFormat::Gray8 },
            emission("emission"),
            height(),
        ],
        ExportPreset::Unreal => vec![base_color("basecolor"), orm(), normal_dx(), emission("emissive"), height()],
        // Custom is driven by the slot dropdowns, not this table.
        ExportPreset::Custom => vec![],
    }
}

/// Parse one channel-source dropdown string into a [`PackedChannel`]. `Ok(None)`
/// means "none" (an empty string is treated the same). Trims and lowercases so
/// connected Text inputs work regardless of case/whitespace.
pub(crate) fn parse_channel_source(text: &str) -> Result<Option<PackedChannel>, String> {
    use SourceChannel::*;
    use SourceMap::*;
    Ok(match text.trim().to_lowercase().as_str() {
        "" | "none" => None,
        "roughness" => Some(src(Roughness, Luma)),
        "1 - roughness" => Some(src_inv(Roughness, Luma)),
        "metallic" => Some(src(Metallic, Luma)),
        "1 - metallic" => Some(src_inv(Metallic, Luma)),
        "ao" => Some(src(AmbientOcclusion, Luma)),
        "1 - ao" => Some(src_inv(AmbientOcclusion, Luma)),
        "height" => Some(src(Height, Luma)),
        "1 - height" => Some(src_inv(Height, Luma)),
        "opacity" => Some(src(Opacity, Luma)),
        "1 - opacity" => Some(src_inv(Opacity, Luma)),
        "albedo" => Some(src(Albedo, Luma)),
        "albedo.r" => Some(src(Albedo, R)),
        "albedo.g" => Some(src(Albedo, G)),
        "albedo.b" => Some(src(Albedo, B)),
        "albedo.a" => Some(src(Albedo, A)),
        "normal" => Some(src(Normal, Luma)),
        "normal.r" => Some(src(Normal, R)),
        "normal.g" => Some(src(Normal, G)),
        "normal.b" => Some(src(Normal, B)),
        "emission" => Some(src(Emission, Luma)),
        "emission.r" => Some(src(Emission, R)),
        "emission.g" => Some(src(Emission, G)),
        "emission.b" => Some(src(Emission, B)),
        other => return Err(format!("Unknown channel source '{}'.", other)),
    })
}

/// Build texture specs from the four Custom slots. A slot with an empty (trimmed)
/// suffix is unused and skipped. An r/g/b source of "none" becomes `Constant(0.0)`;
/// an alpha of "none" drops the alpha channel (3-channel file). Duplicate suffixes
/// are an error.
pub(crate) fn custom_specs(slots: &[(String, [String; 4]); 4]) -> Result<Vec<TextureSpec>, CustomSlotError> {
    let mut specs = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for (slot, (suffix, chans)) in slots.iter().enumerate() {
        let suffix_trim = suffix.trim();
        if suffix_trim.is_empty() {
            continue;
        }
        let key = suffix_trim.to_lowercase();
        if seen.contains(&key) {
            return Err(CustomSlotError { slot, offset: 0, message: format!("Duplicate texture suffix '{}'.", suffix_trim) });
        }
        seen.push(key);

        // offset 1..=4 == r/g/b/a source dropdowns.
        let mut parsed: [Option<PackedChannel>; 4] = [None, None, None, None];
        for (i, source) in chans.iter().enumerate() {
            match parse_channel_source(source) {
                Ok(pc) => parsed[i] = pc,
                Err(message) => return Err(CustomSlotError { slot, offset: i + 1, message }),
            }
        }

        let mut channels = vec![
            parsed[0].clone().unwrap_or(PackedChannel::Constant(0.0)),
            parsed[1].clone().unwrap_or(PackedChannel::Constant(0.0)),
            parsed[2].clone().unwrap_or(PackedChannel::Constant(0.0)),
        ];
        let preferred_format = if let Some(alpha) = parsed[3].clone() {
            channels.push(alpha);
            ColorFormat::Rgba8
        } else {
            ColorFormat::Rgb8
        };
        specs.push(TextureSpec { suffix: suffix_trim.to_string(), channels, preferred_format });
    }

    Ok(specs)
}

/// A spec is written only if at least one of its channels reads a source map
/// that is actually connected; specs made entirely of constants/unconnected maps
/// are skipped (a texture with no real data is not exported).
pub(crate) fn spec_is_writable(spec: &TextureSpec, connected: &[bool; MAP_COUNT]) -> bool {
    spec.channels.iter().any(|c| matches!(c, PackedChannel::Source { map, .. } if connected[*map as usize]))
}

/// Fallback value for an unconnected map's channel: albedo 1, opacity 1, normal
/// (0.5, 0.5, 1), roughness 1 (glTF default), metallic 0, AO 1, height 0.5,
/// emission 0. Scalar maps use the same value on every channel; Luma is the Rec.
/// 601 weighting of the default RGB.
pub(crate) fn map_default(map: SourceMap, channel: SourceChannel) -> f32 {
    let rgba = match map {
        SourceMap::Albedo => [1.0, 1.0, 1.0, 1.0],
        SourceMap::Opacity => [1.0, 1.0, 1.0, 1.0],
        SourceMap::Normal => [0.5, 0.5, 1.0, 1.0],
        SourceMap::Roughness => [1.0, 1.0, 1.0, 1.0],
        SourceMap::Metallic => [0.0, 0.0, 0.0, 0.0],
        SourceMap::AmbientOcclusion => [1.0, 1.0, 1.0, 1.0],
        SourceMap::Height => [0.5, 0.5, 0.5, 0.5],
        SourceMap::Emission => [0.0, 0.0, 0.0, 1.0],
    };
    match channel {
        SourceChannel::R => rgba[0],
        SourceChannel::G => rgba[1],
        SourceChannel::B => rgba[2],
        SourceChannel::A => rgba[3],
        SourceChannel::Luma => 0.299 * rgba[0] + 0.587 * rgba[1] + 0.114 * rgba[2],
    }
}

/// Sample a channel out of a provided map's pixel at `(x, y)`. The map is assumed
/// to already be `w × h` (the caller resizes once before packing). Gray sources
/// answer R/G/B with the first channel; A is the last channel for 2/4-channel
/// sources else 1.0; Luma is Rec. 601 for ≥3-channel sources else the first channel.
fn sample_channel(img: &FloatImage, channel: SourceChannel, x: usize, y: usize, w: usize) -> f32 {
    let ch = img.channels() as usize;
    let idx = (y * w + x) * ch;
    let px = &img.as_raw()[idx..idx + ch];
    match channel {
        SourceChannel::R => px[0],
        SourceChannel::G => if ch >= 3 { px[1] } else { px[0] },
        SourceChannel::B => if ch >= 3 { px[2] } else { px[0] },
        SourceChannel::A => if ch == 2 || ch == 4 { px[ch - 1] } else { 1.0 },
        SourceChannel::Luma => if ch >= 3 { 0.299 * px[0] + 0.587 * px[1] + 0.114 * px[2] } else { px[0] },
    }
}

/// Evaluate one packed channel at `(x, y)`.
fn eval_channel(pc: &PackedChannel, maps: &[Option<Arc<FloatImage>>; MAP_COUNT], x: usize, y: usize, w: usize) -> f32 {
    match pc {
        PackedChannel::Constant(c) => *c,
        PackedChannel::Source { map, channel, invert } => {
            let v = match &maps[*map as usize] {
                Some(img) => sample_channel(img, *channel, x, y, w),
                None => map_default(*map, *channel),
            };
            if *invert { 1.0 - v } else { v }
        }
    }
}

/// Pack a spec into a `FloatImage` of size `w × h` with `spec.channels.len()`
/// channels. Provided maps in `maps` must already be `w × h`; unconnected maps
/// (`None`) resolve to their [`map_default`] constants.
pub(crate) fn pack_texture(spec: &TextureSpec, maps: &[Option<Arc<FloatImage>>; MAP_COUNT], w: u32, h: u32) -> FloatImage {
    let out_ch = spec.channels.len();
    let wu = w as usize;
    let hu = h as usize;
    let mut out = vec![0.0f32; wu * hu * out_ch];

    let process_row = |(y, dst_row): (usize, &mut [f32])| {
        for (x, dst) in dst_row.chunks_exact_mut(out_ch).enumerate() {
            for (c, pc) in spec.channels.iter().enumerate() {
                dst[c] = eval_channel(pc, maps, x, y, wu);
            }
        }
    };

    if wu > 0 && hu > 0 {
        if wu * hu >= PARALLEL_PIXELS {
            out.par_chunks_exact_mut(wu * out_ch).enumerate().for_each(process_row);
        } else {
            out.chunks_exact_mut(wu * out_ch).enumerate().for_each(process_row);
        }
    }

    FloatImage::from_raw(w, h, out_ch as u32, out).unwrap()
}

#[cfg(test)]
#[path = "material_presets_tests.rs"]
mod tests;
