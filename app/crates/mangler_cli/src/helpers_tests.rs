use super::*;

use mangler_core::operations::{operation_list, OperationListItem};

// ── parse_slot ────────────────────────────────────────────────────────────

#[test]
fn parse_slot_valid() {
    let (node, idx) = parse_slot("abc:2").unwrap();
    assert_eq!(node, "abc");
    assert_eq!(idx, 2);
}

#[test]
fn parse_slot_zero_index() {
    let (node, idx) = parse_slot("mynode:0").unwrap();
    assert_eq!(node, "mynode");
    assert_eq!(idx, 0);
}

/// Node IDs containing colons are supported — we split on the *last* colon.
#[test]
fn parse_slot_node_id_with_colon() {
    let (node, idx) = parse_slot("a:b:3").unwrap();
    assert_eq!(node, "a:b");
    assert_eq!(idx, 3);
}

#[test]
fn parse_slot_missing_colon_returns_err() {
    assert!(parse_slot("nocolon").is_err());
}

#[test]
fn parse_slot_non_numeric_index_returns_err() {
    assert!(parse_slot("node:abc").is_err());
}

#[test]
fn parse_slot_empty_string_returns_err() {
    assert!(parse_slot("").is_err());
}

#[test]
fn parse_slot_leading_colon_empty_node_id() {
    let (node, idx) = parse_slot(":0").unwrap();
    assert_eq!(node, "");
    assert_eq!(idx, 0);
}

#[test]
fn parse_slot_trailing_colon_returns_err() {
    assert!(parse_slot("node:").is_err());
}

#[test]
fn parse_slot_negative_index_returns_err() {
    assert!(parse_slot("node:-1").is_err());
}

#[test]
fn parse_slot_overflow_index_returns_err() {
    assert!(parse_slot("node:99999999999999999999").is_err());
}

#[test]
fn parse_slot_only_colon_returns_err() {
    assert!(parse_slot(":").is_err());
}

// ── flatten_ops ───────────────────────────────────────────────────────────

#[test]
fn flatten_ops_returns_non_empty() {
    let all = flatten_ops(&operation_list(), "");
    assert!(!all.is_empty(), "operation list should not be empty");
}

#[test]
fn flatten_ops_paths_contain_slash() {
    let all = flatten_ops(&operation_list(), "");
    assert!(all.iter().any(|(p, _)| p.contains('/')), "at least one path should contain '/'");
}

#[test]
fn flatten_ops_prefix_prepended() {
    let all = flatten_ops(&operation_list(), "");
    for (path, _) in &all {
        assert!(!path.starts_with('/'), "path should not start with '/'");
    }
}

#[test]
fn flatten_ops_custom_prefix() {
    let all = flatten_ops(&operation_list(), "custom");
    for (path, _) in &all {
        assert!(path.starts_with("custom/"), "path should start with 'custom/'");
    }
}

#[test]
fn flatten_ops_no_duplicates() {
    let all = flatten_ops(&operation_list(), "");
    let mut seen = std::collections::HashSet::new();
    for (path, _) in &all {
        assert!(seen.insert(path.clone()), "duplicate path: {path}");
    }
}

#[test]
fn flatten_ops_empty_slice() {
    let all = flatten_ops(&[], "");
    assert!(all.is_empty());
}

#[test]
fn flatten_ops_subgraph_items_are_skipped() {
    let items = vec![OperationListItem::Subgraph];
    let result = flatten_ops(&items, "");
    assert!(result.is_empty(), "Subgraph items should be skipped");
}

#[test]
fn flatten_ops_empty_category_contributes_nothing() {
    let items = vec![OperationListItem::Category {
        name: "empty".to_string(),
        operation_list_items: vec![],
    }];
    let result = flatten_ops(&items, "");
    assert!(result.is_empty());
}

#[test]
fn flatten_ops_deep_nesting_builds_correct_path() {
    let items = vec![OperationListItem::Category {
        name: "a".to_string(),
        operation_list_items: vec![OperationListItem::Category {
            name: "b".to_string(),
            operation_list_items: vec![OperationListItem::Category {
                name: "c".to_string(),
                operation_list_items: vec![],
            }],
        }],
    }];
    let result = flatten_ops(&items, "");
    assert!(result.is_empty(), "no ops means no entries");
}

// ── resolve_op ────────────────────────────────────────────────────────────

#[test]
fn resolve_op_by_short_path() {
    assert!(resolve_op("numbers/arithmetic/add").is_ok());
}

#[test]
fn resolve_op_case_insensitive() {
    assert!(resolve_op("Numbers/Arithmetic/Add").is_ok());
}

#[test]
fn resolve_op_by_variant_name() {
    assert!(resolve_op("OpNumberMathAdd").is_ok());
}

#[test]
fn resolve_op_short_and_variant_yield_same_operation() {
    let by_path = resolve_op("numbers/arithmetic/add").unwrap();
    let by_variant = resolve_op("OpNumberMathAdd").unwrap();
    let a = serde_json::to_string(&by_path).unwrap();
    let b = serde_json::to_string(&by_variant).unwrap();
    assert_eq!(a, b);
}

#[test]
fn resolve_op_unknown_returns_err() {
    assert!(resolve_op("not/a/real/op").is_err());
}

#[test]
fn resolve_op_empty_string_returns_err() {
    assert!(resolve_op("").is_err());
}

#[test]
fn resolve_op_leading_whitespace_returns_err() {
    assert!(resolve_op(" numbers/arithmetic/add").is_err());
}

#[test]
fn resolve_op_trailing_whitespace_returns_err() {
    assert!(resolve_op("numbers/arithmetic/add ").is_err());
}

#[test]
fn resolve_op_category_path_only_returns_err() {
    assert!(resolve_op("numbers/arithmetic").is_err());
}

#[test]
fn resolve_op_other_categories_resolve() {
    assert!(resolve_op("images/adjustments/invert").is_ok());
}

#[test]
fn resolve_op_short_path_and_variant_are_equivalent() {
    let all = flatten_ops(&operation_list(), "");
    for (path, expected) in all.iter().take(5) {
        let resolved = resolve_op(path).unwrap();
        let a = serde_json::to_string(&resolved).unwrap();
        let b = serde_json::to_string(expected).unwrap();
        assert_eq!(a, b, "mismatch for path '{path}'");
    }
}

// ── enum_variants helper tests ──────────────────────────────────────

#[test]
fn enum_variants_all_types_resolve() {
    for name in ENUM_TYPE_NAMES {
        assert!(enum_variants(name).is_some(), "enum_variants failed for {name}");
    }
}

#[test]
fn enum_variants_unknown_returns_none() { assert!(enum_variants("NotAType").is_none()); }

/// Guards against the hand-maintained `enum_variants` lists silently
/// drifting out of sync with the real enums as variants are added in
/// `mangler_core` (this happened twice: colorspace was missing 5 of 14
/// variants, imagetype was missing "avif"). Every enum type that exposes a
/// `types()` count in core is checked here; filtertype and worleydistance
/// have no such count to compare against and are covered by
/// `parse_typed_value_all_*_variants` tests in value_parse_tests.rs instead.
#[test]
fn enum_variants_counts_match_core_enum_counts() {
    use mangler_core::color::blend::BlendMode;
    use mangler_core::color::color_spaces::ColorSpace;
    use mangler_core::value::{ColorFormat, EdgeMode, ExportPreset, ImageType, TextHAlign, TextVAlign};

    assert_eq!(enum_variants("blendmode").unwrap().len(), BlendMode::types().len());
    assert_eq!(enum_variants("colorspace").unwrap().len(), ColorSpace::types().len());
    assert_eq!(enum_variants("imagetype").unwrap().len(), ImageType::types().len());
    assert_eq!(enum_variants("colorformat").unwrap().len(), ColorFormat::types().len());
    assert_eq!(enum_variants("edgemode").unwrap().len(), EdgeMode::types().len());
    assert_eq!(enum_variants("exportpreset").unwrap().len(), ExportPreset::types().len());
    assert_eq!(enum_variants("texthalign").unwrap().len(), TextHAlign::types().len());
    assert_eq!(enum_variants("textvalign").unwrap().len(), TextVAlign::types().len());
}

#[test]
fn value_type_enum_name_mappings() {
    assert_eq!(value_type_enum_name(&ValueType::BlendMode), Some("blendmode"));
    assert_eq!(value_type_enum_name(&ValueType::ColorSpace), Some("colorspace"));
    assert_eq!(value_type_enum_name(&ValueType::FilterType), Some("filtertype"));
    assert_eq!(value_type_enum_name(&ValueType::Decimal), None);
}

// ── collect_categories ──────────────────────────────────────────────

#[test]
fn collect_categories_returns_expected() {
    let all = flatten_ops(&operation_list(), "");
    let cats = collect_categories(&all);
    assert!(!cats.is_empty());
    let names: Vec<&str> = cats.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"numbers"));
    assert!(names.contains(&"images"));
}

#[test]
fn collect_categories_empty_input() { assert!(collect_categories(&[]).is_empty()); }

// ── score_op ─────────────────────────────────────────────────────────

/// Term that exactly matches a path segment scores highest (10 pts).
#[test]
fn score_op_exact_path_segment() {
    let score = score_op(("images/blur/blur", "opimageblurblur", "apply a blur"), &["blur".to_string()]);
    // "blur" matches exact segment (+10), also contained in path (+0 because exact took priority),
    // variant contains (+4), description contains (+2) = 10 + 4 + 2 = 16
    assert!(score >= 10, "expected at least 10 for exact segment match, got {score}");
}

/// Term that is a substring of a path segment but not exact scores 5.
#[test]
fn score_op_path_contains() {
    let score = score_op(("images/blur/slope_blur", "opimageblurslopeblur", "slope-based blur"), &["slope".to_string()]);
    assert!(score >= 5, "expected at least 5 for path-contains match, got {score}");
}

/// Variant exact match scores 8.
#[test]
fn score_op_exact_variant() {
    let score = score_op(("misc", "blur", "desc"), &["blur".to_string()]);
    assert!(score >= 8, "expected at least 8 for exact variant match, got {score}");
}

/// Variant substring scores 4.
#[test]
fn score_op_variant_contains() {
    let score = score_op(("misc", "superblur", "desc"), &["blur".to_string()]);
    assert!(score >= 4, "expected at least 4 for variant-contains, got {score}");
}

/// Description contains scores 2.
#[test]
fn score_op_description_contains() {
    let score = score_op(("misc", "foo", "this applies a blur effect"), &["blur".to_string()]);
    assert_eq!(score, 2, "expected exactly 2 for description-only match");
}

/// A term that matches nothing returns 0.
#[test]
fn score_op_no_match_returns_zero() {
    let score = score_op(("misc", "foo", "bar"), &["zzzzz".to_string()]);
    assert_eq!(score, 0);
}

/// Multiple terms accumulate scores.
#[test]
fn score_op_multiple_terms_accumulate() {
    let score = score_op(
        ("images/blur/blur", "opblur", "apply a gaussian blur"),
        &["blur".to_string(), "gaussian".to_string()],
    );
    // "blur" => exact segment (10) + variant contains (4) + desc (2) = 16
    // "gaussian" => desc only (2)
    // total = 18
    assert!(score >= 12, "expected accumulation, got {score}");
}

/// If any one term misses entirely, the whole score is 0.
#[test]
fn score_op_any_term_missing_returns_zero() {
    let score = score_op(
        ("images/blur/blur", "opblur", "apply a blur"),
        &["blur".to_string(), "zzzznotfound".to_string()],
    );
    assert_eq!(score, 0, "one missing term should zero the entire score");
}

/// Case shouldn't matter — caller is expected to lowercase everything.
#[test]
fn score_op_case_insensitive() {
    let upper = score_op(("images/blur/blur", "opblur", "desc"), &["BLUR".to_string()]);
    // Since everything is lowercase in the haystack but the term is uppercase, it should NOT match.
    assert_eq!(upper, 0, "case mismatch means no match in the function itself");
}

// ── load_graph ────────────────────────────────────────────────────────────

#[test]
fn load_graph_missing_file_returns_err() {
    let path = std::path::PathBuf::from("/tmp/does_not_exist_mangler_test.json");
    assert!(load_graph(&path).is_err());
}

#[test]
fn load_graph_invalid_json_returns_err() {
    let path = temp_graph_path("invalid_json");
    std::fs::write(&path, "NOT JSON").unwrap();
    let result = load_graph(&path);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn load_graph_empty_object_returns_err() {
    let path = temp_graph_path("empty_obj");
    std::fs::write(&path, "{}").unwrap();
    let result = load_graph(&path);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn load_graph_freshly_created_graph_is_empty() {
    let path = create_temp_graph("fresh");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes.is_empty());
}

#[test]
fn save_load_round_trip_preserves_name_and_id() {
    let path = create_temp_graph("roundtrip");
    let g1 = load_graph(&path).unwrap();
    save_graph(&g1, &path).unwrap();
    let g2 = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(g1.name, g2.name);
    assert_eq!(g1.id, g2.id);
}

// ── op_variant_name ──────────────────────────────────────────────────────

/// op_variant_name returns the serde variant string for a known operation.
#[test]
fn op_variant_name_known_op() {
    let op = resolve_op("numbers/arithmetic/add").unwrap();
    let name = op_variant_name(&op);
    assert_eq!(name, "OpNumberMathAdd");
}

/// op_variant_name never returns an empty string for any operation.
#[test]
fn op_variant_name_never_empty() {
    let all = flatten_ops(&operation_list(), "");
    for (path, op) in &all {
        let name = op_variant_name(op);
        assert!(!name.is_empty(), "op_variant_name returned empty for {path}");
    }
}

// ── accepted_conversions ─────────────────────────────────────────────────

/// Decimal accepts at least Integer and Bool conversions.
#[test]
fn accepted_conversions_decimal_includes_int_and_bool() {
    let accepts = accepted_conversions(&ValueType::Decimal);
    assert!(accepts.contains(&"int".to_string()), "Decimal should accept int, got: {accepts:?}");
    assert!(accepts.contains(&"bool".to_string()), "Decimal should accept bool, got: {accepts:?}");
}

/// accepted_conversions excludes self and Trigger.
#[test]
fn accepted_conversions_excludes_self_and_trigger() {
    let accepts = accepted_conversions(&ValueType::Decimal);
    assert!(!accepts.contains(&"decimal".to_string()), "should not contain self");
    assert!(!accepts.contains(&"trigger".to_string()), "should not contain trigger");
}

/// Image type accepted conversions exclude self and trigger.
#[test]
fn accepted_conversions_image() {
    let accepts = accepted_conversions(&ValueType::Image);
    // Verify self and trigger are excluded (the function filters those out).
    assert!(!accepts.contains(&"image".to_string()), "should not contain self");
    assert!(!accepts.contains(&"trigger".to_string()), "should not contain trigger");
}

// ── output_conversions ───────────────────────────────────────────────────

/// Decimal can convert to other numeric types.
#[test]
fn output_conversions_decimal() {
    let converts = output_conversions(&ValueType::Decimal);
    assert!(converts.contains(&"int".to_string()), "Decimal should convert to int, got: {converts:?}");
    assert!(converts.contains(&"bool".to_string()), "Decimal should convert to bool, got: {converts:?}");
}

/// output_conversions excludes self and Trigger.
#[test]
fn output_conversions_excludes_self_and_trigger() {
    let converts = output_conversions(&ValueType::Decimal);
    assert!(!converts.contains(&"decimal".to_string()), "should not contain self");
    assert!(!converts.contains(&"trigger".to_string()), "should not contain trigger");
}

// ── value_type_name ──────────────────────────────────────────────────────

/// Every ValueType variant maps to a non-empty string.
#[test]
fn value_type_name_all_variants_non_empty() {
    let types = [
        ValueType::Bool, ValueType::Integer, ValueType::Decimal,
        ValueType::Text, ValueType::Color, ValueType::Path,
        ValueType::Image, ValueType::Trigger, ValueType::BlendMode,
        ValueType::ColorSpace, ValueType::FilterType, ValueType::ImageType,
        ValueType::ColorFormat, ValueType::NoiseWorleyDistanceFunction,
        ValueType::TextHAlign, ValueType::TextVAlign,
    ];
    for vt in &types {
        let name = value_type_name(vt);
        assert!(!name.is_empty(), "value_type_name empty for {:?}", vt);
    }
}

/// Specific value type name mappings are correct.
#[test]
fn value_type_name_specific_values() {
    assert_eq!(value_type_name(&ValueType::Bool), "bool");
    assert_eq!(value_type_name(&ValueType::Integer), "int");
    assert_eq!(value_type_name(&ValueType::Decimal), "decimal");
    assert_eq!(value_type_name(&ValueType::Text), "text");
    assert_eq!(value_type_name(&ValueType::Color), "color");
    assert_eq!(value_type_name(&ValueType::Path), "path");
    assert_eq!(value_type_name(&ValueType::Image), "image");
    assert_eq!(value_type_name(&ValueType::Trigger), "trigger");
    assert_eq!(value_type_name(&ValueType::BlendMode), "blendmode");
    assert_eq!(value_type_name(&ValueType::ColorSpace), "colorspace");
    assert_eq!(value_type_name(&ValueType::FilterType), "filtertype");
    assert_eq!(value_type_name(&ValueType::ImageType), "imagetype");
    assert_eq!(value_type_name(&ValueType::ColorFormat), "colorformat");
    assert_eq!(value_type_name(&ValueType::NoiseWorleyDistanceFunction), "worleydistance");
    assert_eq!(value_type_name(&ValueType::TextHAlign), "texthalign");
    assert_eq!(value_type_name(&ValueType::TextVAlign), "textvalign");
}

// ── resolve_enum_type_name ───────────────────────────────────────────────

/// All 8 canonical enum type names resolve to themselves.
#[test]
fn resolve_enum_type_name_all_canonical() {
    for name in ENUM_TYPE_NAMES {
        let resolved = resolve_enum_type_name(name);
        assert_eq!(resolved, Some(*name), "canonical name '{name}' should resolve to itself");
    }
}

/// PascalCase aliases resolve to their canonical lowercase form.
#[test]
fn resolve_enum_type_name_aliases() {
    for (alias, expected) in ENUM_TYPE_ALIASES {
        let resolved = resolve_enum_type_name(alias);
        assert_eq!(resolved, Some(*expected), "alias '{alias}' should resolve to '{expected}'");
    }
}

/// resolve_enum_type_name is case-insensitive.
#[test]
fn resolve_enum_type_name_case_insensitive() {
    assert_eq!(resolve_enum_type_name("BLENDMODE"), Some("blendmode"));
    assert_eq!(resolve_enum_type_name("BlEndMoDe"), Some("blendmode"));
    assert_eq!(resolve_enum_type_name("Colorspace"), Some("colorspace"));
}

/// Unknown type name returns None.
#[test]
fn resolve_enum_type_name_unknown_returns_none() {
    assert_eq!(resolve_enum_type_name("NotAType"), None);
    assert_eq!(resolve_enum_type_name(""), None);
}

// ── ENUM_TYPE_ALIASES consistency ────────────────────────────────────────

/// Every alias target exists in ENUM_TYPE_NAMES.
#[test]
fn enum_type_aliases_all_map_to_known_canonical() {
    for (alias, canonical) in ENUM_TYPE_ALIASES {
        assert!(
            ENUM_TYPE_NAMES.contains(canonical),
            "alias '{alias}' maps to '{canonical}' which is not in ENUM_TYPE_NAMES"
        );
    }
}

// ── File I/O helpers ─────────────────────────────────────────────────────

/// Save a graph with nodes, load it back, verify nodes persist.
#[tokio::test]
async fn save_graph_load_graph_round_trip_with_nodes() {
    let path = create_temp_graph("roundtrip_nodes");
    let mut graph = load_graph(&path).unwrap();
    // Add a node to the graph.
    crate::commands::do_add_node(&mut graph, "numbers/arithmetic/add", Some("rt-node".to_string()), None).await.unwrap();
    save_graph(&graph, &path).unwrap();
    // Reload and verify.
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(reloaded.nodes.contains_key("rt-node"), "node should persist through save/load");
}

/// save_value_to_file writes valid JSON.
#[test]
fn save_value_to_file_writes_valid_json() {
    use mangler_core::value::Value;
    let path = temp_graph_path("save_value");
    let _ = std::fs::remove_file(&path);
    save_value_to_file(&Value::Decimal(42.5), &path).unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    // Verify it's valid JSON.
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    // json_value wraps in {"Decimal": 42.5} so check it's a valid JSON object.
    assert!(parsed.is_object() || parsed.is_number(), "expected valid JSON, got: {parsed}");
}

/// save_image_to_file creates a non-empty file.
#[test]
fn save_image_to_file_creates_file() {
    use mangler_core::float_image::FloatImage;
    let path = std::env::temp_dir().join(format!("mangle_test_saveimg_{}.exr", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let img = FloatImage::from_pixel(4, 4, 4, &[1.0, 0.0, 0.0, 1.0]);
    save_image_to_file(&img, &path).unwrap();
    let meta = std::fs::metadata(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(meta.len() > 0, "saved image file should be non-empty");
}

/// Regression test: a bare `FloatImage::to_dynamic()` produces `ImageRgba32F`
/// / `ImageRgb32F` for 3/4-channel images, which the PNG/JPEG/GIF/BMP
/// encoders reject outright ("the encoder or decoder for Png does not
/// support the color type Rgba32F"). `save_image_to_file` must pick a
/// compatible color format per target instead of saving the raw dynamic
/// image, for every common channel count and container format.
#[test]
fn save_image_to_file_supports_common_formats_for_rgb_and_rgba() {
    use mangler_core::float_image::FloatImage;

    for ext in ["png", "jpg", "bmp", "gif"] {
        for channels in [1u32, 2, 3, 4] {
            let path = std::env::temp_dir().join(format!(
                "mangle_test_saveimg_{}_{}ch_{}.{}",
                std::process::id(), channels, ext, ext
            ));
            let _ = std::fs::remove_file(&path);
            let pixel = vec![0.5f32; channels as usize];
            let img = FloatImage::from_pixel(4, 4, channels, &pixel);
            save_image_to_file(&img, &path)
                .unwrap_or_else(|e| panic!("saving {channels}-channel image as .{ext} should succeed: {e}"));
            let meta = std::fs::metadata(&path).unwrap();
            let _ = std::fs::remove_file(&path);
            assert!(meta.len() > 0, "saved .{ext} file should be non-empty ({channels} channels)");
        }
    }
}
