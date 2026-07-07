use super::*;

/// Builds a `w × h` image with `ch` channels; each pixel repeats `values`.
fn solid(w: u32, h: u32, values: &[f32]) -> Arc<FloatImage> {
    let ch = values.len() as u32;
    let mut data = Vec::with_capacity((w * h) as usize * values.len());
    for _ in 0..(w * h) {
        data.extend_from_slice(values);
    }
    Arc::new(FloatImage::from_raw(w, h, ch, data).unwrap())
}

// --- parser -----------------------------------------------------------------

#[test]
fn test_parser_accepts_all_options() {
    for opt in CHANNEL_SOURCE_OPTIONS {
        let parsed = parse_channel_source(opt);
        assert!(parsed.is_ok(), "option {:?} should parse", opt);
        if *opt == "none" {
            assert_eq!(parsed.unwrap(), None, "'none' must parse to None");
        } else {
            assert!(parsed.unwrap().is_some(), "option {:?} should be a Some", opt);
        }
    }
}

#[test]
fn test_parser_is_tolerant() {
    // Empty is treated as none; case and surrounding whitespace are ignored.
    assert_eq!(parse_channel_source("").unwrap(), None);
    assert_eq!(parse_channel_source("  NONE  ").unwrap(), None);
    assert_eq!(
        parse_channel_source("  Albedo.R  ").unwrap(),
        Some(PackedChannel::Source { map: SourceMap::Albedo, channel: SourceChannel::R, invert: false })
    );
}

#[test]
fn test_parser_rejects_garbage() {
    assert!(parse_channel_source("banana").is_err());
    assert!(parse_channel_source("albedo.z").is_err());
    assert!(parse_channel_source("2 - roughness").is_err());
}

#[test]
fn test_parser_invert_flag() {
    assert_eq!(
        parse_channel_source("1 - roughness").unwrap(),
        Some(PackedChannel::Source { map: SourceMap::Roughness, channel: SourceChannel::Luma, invert: true })
    );
    assert_eq!(
        parse_channel_source("roughness").unwrap(),
        Some(PackedChannel::Source { map: SourceMap::Roughness, channel: SourceChannel::Luma, invert: false })
    );
}

#[test]
fn test_channel_source_options_count() {
    assert_eq!(CHANNEL_SOURCE_OPTIONS.len(), 24);
}

#[test]
fn test_source_map_indices_match_input_order() {
    assert_eq!(SourceMap::Albedo as usize, 0);
    assert_eq!(SourceMap::Opacity as usize, 1);
    assert_eq!(SourceMap::Normal as usize, 2);
    assert_eq!(SourceMap::Roughness as usize, 3);
    assert_eq!(SourceMap::Metallic as usize, 4);
    assert_eq!(SourceMap::AmbientOcclusion as usize, 5);
    assert_eq!(SourceMap::Height as usize, 6);
    assert_eq!(SourceMap::Emission as usize, 7);
}

// --- builtin spec shapes ----------------------------------------------------

#[test]
fn test_godot_specs_shape() {
    let specs = builtin_specs(ExportPreset::Godot, &[false; MAP_COUNT]);
    let suffixes: Vec<&str> = specs.iter().map(|s| s.suffix.as_str()).collect();
    assert_eq!(suffixes, ["albedo", "orm", "normal", "emission", "height"]);
    // albedo without opacity is 3-channel Rgb8.
    assert_eq!(specs[0].channels.len(), 3);
    assert_eq!(specs[0].preferred_format, ColorFormat::Rgb8);
    // orm layout: R = AO, G = roughness, B = metallic.
    assert_eq!(specs[1].channels, vec![
        PackedChannel::Source { map: SourceMap::AmbientOcclusion, channel: SourceChannel::Luma, invert: false },
        PackedChannel::Source { map: SourceMap::Roughness, channel: SourceChannel::Luma, invert: false },
        PackedChannel::Source { map: SourceMap::Metallic, channel: SourceChannel::Luma, invert: false },
    ]);
    // normal is 16-bit, no inversion (OpenGL Y+).
    assert_eq!(specs[2].preferred_format, ColorFormat::Rgb16);
    assert!(specs[2].channels.iter().all(|c| matches!(c, PackedChannel::Source { invert: false, .. })));
    // height is 16-bit gray, single channel.
    assert_eq!(specs[4].channels.len(), 1);
    assert_eq!(specs[4].preferred_format, ColorFormat::Gray16);
}

#[test]
fn test_albedo_gains_alpha_with_opacity() {
    let mut connected = [false; MAP_COUNT];
    connected[SourceMap::Opacity as usize] = true;
    let specs = builtin_specs(ExportPreset::Godot, &connected);
    assert_eq!(specs[0].channels.len(), 4);
    assert_eq!(specs[0].preferred_format, ColorFormat::Rgba8);
    assert_eq!(specs[0].channels[3], PackedChannel::Source { map: SourceMap::Opacity, channel: SourceChannel::Luma, invert: false });
}

#[test]
fn test_unity_metallic_always_rgba() {
    // Metallic-smoothness is always 4-channel with A = 1 − roughness.
    let specs = builtin_specs(ExportPreset::Unity, &[false; MAP_COUNT]);
    let metallic = specs.iter().find(|s| s.suffix == "metallic").unwrap();
    assert_eq!(metallic.channels.len(), 4);
    assert_eq!(metallic.preferred_format, ColorFormat::Rgba8);
    assert_eq!(metallic.channels[3], PackedChannel::Source { map: SourceMap::Roughness, channel: SourceChannel::Luma, invert: true });
    // ao is a separate 8-bit gray texture.
    let ao = specs.iter().find(|s| s.suffix == "ao").unwrap();
    assert_eq!(ao.preferred_format, ColorFormat::Gray8);
}

#[test]
fn test_unreal_normal_green_inverted() {
    let specs = builtin_specs(ExportPreset::Unreal, &[false; MAP_COUNT]);
    let suffixes: Vec<&str> = specs.iter().map(|s| s.suffix.as_str()).collect();
    assert_eq!(suffixes, ["basecolor", "orm", "normal", "emissive", "height"]);
    let normal = specs.iter().find(|s| s.suffix == "normal").unwrap();
    assert_eq!(normal.channels[0], PackedChannel::Source { map: SourceMap::Normal, channel: SourceChannel::R, invert: false });
    assert_eq!(normal.channels[1], PackedChannel::Source { map: SourceMap::Normal, channel: SourceChannel::G, invert: true });
    assert_eq!(normal.channels[2], PackedChannel::Source { map: SourceMap::Normal, channel: SourceChannel::B, invert: false });
}

#[test]
fn test_custom_specs_empty_and_duplicate() {
    // Empty suffix slots are skipped.
    let slots: [(String, [String; 4]); 4] = Default::default();
    assert!(custom_specs(&slots).unwrap().is_empty());

    // A used slot: r/g/b none -> constants, a none -> 3-channel.
    let mut slots: [(String, [String; 4]); 4] = Default::default();
    slots[0] = ("mask".to_string(), ["roughness".to_string(), "none".to_string(), "1 - metallic".to_string(), "none".to_string()]);
    let specs = custom_specs(&slots).unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].suffix, "mask");
    assert_eq!(specs[0].channels.len(), 3);
    assert_eq!(specs[0].channels[1], PackedChannel::Constant(0.0));
    assert_eq!(specs[0].preferred_format, ColorFormat::Rgb8);

    // Alpha source -> 4-channel Rgba8.
    slots[0].1[3] = "opacity".to_string();
    let specs = custom_specs(&slots).unwrap();
    assert_eq!(specs[0].channels.len(), 4);
    assert_eq!(specs[0].preferred_format, ColorFormat::Rgba8);

    // Duplicate suffix -> error pointing at the suffix widget (offset 0).
    let mut slots: [(String, [String; 4]); 4] = Default::default();
    slots[0].0 = "mask".to_string();
    slots[2].0 = "mask".to_string();
    let err = custom_specs(&slots).unwrap_err();
    assert_eq!(err.slot, 2);
    assert_eq!(err.offset, 0);
}

#[test]
fn test_custom_specs_channel_error_offset() {
    let mut slots: [(String, [String; 4]); 4] = Default::default();
    // slot 1, green channel (offset 2) is garbage.
    slots[1] = ("tex".to_string(), ["albedo.r".to_string(), "banana".to_string(), "none".to_string(), "none".to_string()]);
    let err = custom_specs(&slots).unwrap_err();
    assert_eq!(err.slot, 1);
    assert_eq!(err.offset, 2);
}

// --- spec_is_writable -------------------------------------------------------

#[test]
fn test_spec_is_writable() {
    let specs = builtin_specs(ExportPreset::Godot, &[false; MAP_COUNT]);
    let orm = specs.iter().find(|s| s.suffix == "orm").unwrap();
    let normal = specs.iter().find(|s| s.suffix == "normal").unwrap();

    // No maps connected -> nothing writable.
    assert!(!spec_is_writable(orm, &[false; MAP_COUNT]));

    // Metallic connected -> ORM writable (it references metallic), normal not.
    let mut connected = [false; MAP_COUNT];
    connected[SourceMap::Metallic as usize] = true;
    assert!(spec_is_writable(orm, &connected));
    assert!(!spec_is_writable(normal, &connected));

    // A constants-only custom spec is never writable.
    let all_const = TextureSpec {
        suffix: "c".to_string(),
        channels: vec![PackedChannel::Constant(0.0); 3],
        preferred_format: ColorFormat::Rgb8,
    };
    assert!(!spec_is_writable(&all_const, &[true; MAP_COUNT]));
}

// --- map_default ------------------------------------------------------------

#[test]
fn test_map_default_constants() {
    assert_eq!(map_default(SourceMap::Albedo, SourceChannel::R), 1.0);
    assert_eq!(map_default(SourceMap::Roughness, SourceChannel::Luma), 1.0);
    assert_eq!(map_default(SourceMap::Metallic, SourceChannel::Luma), 0.0);
    assert_eq!(map_default(SourceMap::AmbientOcclusion, SourceChannel::Luma), 1.0);
    assert_eq!(map_default(SourceMap::Height, SourceChannel::Luma), 0.5);
    assert_eq!(map_default(SourceMap::Opacity, SourceChannel::A), 1.0);
    assert_eq!(map_default(SourceMap::Emission, SourceChannel::R), 0.0);
    // Normal default (0.5, 0.5, 1.0).
    assert_eq!(map_default(SourceMap::Normal, SourceChannel::R), 0.5);
    assert_eq!(map_default(SourceMap::Normal, SourceChannel::G), 0.5);
    assert_eq!(map_default(SourceMap::Normal, SourceChannel::B), 1.0);
    assert_eq!(map_default(SourceMap::Normal, SourceChannel::A), 1.0);
}

// --- pack_texture -----------------------------------------------------------

#[test]
fn test_pack_channel_order_and_constants() {
    // Godot albedo (3ch) with a distinct-per-channel albedo map.
    let specs = builtin_specs(ExportPreset::Godot, &[false; MAP_COUNT]);
    let albedo_spec = &specs[0];
    let mut maps: [Option<Arc<FloatImage>>; MAP_COUNT] = Default::default();
    maps[SourceMap::Albedo as usize] = Some(solid(2, 2, &[0.1, 0.2, 0.3]));
    let out = pack_texture(albedo_spec, &maps, 2, 2);
    assert_eq!(out.channels(), 3);
    let px = &out.as_raw()[0..3];
    assert_eq!(px, &[0.1, 0.2, 0.3]);

    // Custom spec with a constant middle channel.
    let mut slots: [(String, [String; 4]); 4] = Default::default();
    slots[0] = ("t".to_string(), ["albedo.r".to_string(), "none".to_string(), "albedo.b".to_string(), "none".to_string()]);
    let cspec = &custom_specs(&slots).unwrap()[0];
    let out = pack_texture(cspec, &maps, 2, 2);
    let px = &out.as_raw()[0..3];
    assert_eq!(px, &[0.1, 0.0, 0.3]);
}

#[test]
fn test_pack_invert_and_defaults() {
    // Unity metallic: A = 1 − roughness. Roughness = 0.25 -> smoothness 0.75.
    let specs = builtin_specs(ExportPreset::Unity, &[false; MAP_COUNT]);
    let metallic = specs.iter().find(|s| s.suffix == "metallic").unwrap();
    let mut maps: [Option<Arc<FloatImage>>; MAP_COUNT] = Default::default();
    maps[SourceMap::Metallic as usize] = Some(solid(1, 1, &[0.8]));
    maps[SourceMap::Roughness as usize] = Some(solid(1, 1, &[0.25]));
    let out = pack_texture(metallic, &maps, 1, 1);
    let px = out.as_raw();
    assert_eq!(px[0], 0.8);
    assert_eq!(px[1], 0.8);
    assert_eq!(px[2], 0.8);
    assert!((px[3] - 0.75).abs() < 1e-6, "smoothness should be 1 - 0.25");

    // ORM with nothing connected packs the fallback constants.
    let orm = builtin_specs(ExportPreset::Godot, &[false; MAP_COUNT])[1].clone();
    let empty: [Option<Arc<FloatImage>>; MAP_COUNT] = Default::default();
    let out = pack_texture(&orm, &empty, 1, 1);
    let px = out.as_raw();
    assert_eq!(px[0], 1.0); // ao default
    assert_eq!(px[1], 1.0); // roughness default
    assert_eq!(px[2], 0.0); // metallic default
}

#[test]
fn test_pack_luma_sampling() {
    // A source read as Luma on a 3-channel image is Rec.601 weighted.
    let mut slots: [(String, [String; 4]); 4] = Default::default();
    slots[0] = ("t".to_string(), ["albedo".to_string(), "none".to_string(), "none".to_string(), "none".to_string()]);
    let cspec = &custom_specs(&slots).unwrap()[0];
    let mut maps: [Option<Arc<FloatImage>>; MAP_COUNT] = Default::default();
    maps[SourceMap::Albedo as usize] = Some(solid(1, 1, &[1.0, 0.0, 0.0]));
    let out = pack_texture(cspec, &maps, 1, 1);
    assert!((out.as_raw()[0] - 0.299).abs() < 1e-6, "Rec.601 luma of pure red");
}
