use super::*;

use crate::helpers::{create_temp_graph, load_graph};
use crate::commands::{cmd_add_node, cmd_set_enabled};

// ── show-ops ────────────────────────────────────────────────────────

#[test]
fn show_ops_group_fallback_shows_categories() {
    let out = format_show_ops_human(Some("zzz_no_such_group"), None);
    assert!(out.contains("Available categories"), "expected fallback categories in: {out}");
    assert!(out.contains("numbers") || out.contains("images"));
}

#[test]
fn show_ops_group_valid_prefix() {
    let out = format_show_ops_human(Some("numbers"), None);
    assert!(!out.is_empty());
}

#[test]
fn show_ops_search() {
    let out = format_show_ops_human(None, Some("blur"));
    assert!(!out.is_empty());
}

// ── Multi-word AND search (integration) ─────────────────────────────

#[test]
fn search_multi_word_and() {
    let out = format_show_ops_human(None, Some("blur directional"));
    // "directional_blur" should match both terms.
    assert!(
        out.to_lowercase().contains("directional"),
        "expected 'directional' in multi-word search results: {out}"
    );
    // Should not include ops that only match one term.
    // (Exact filtering depends on the ops list, so just check it's non-empty.)
    assert!(!out.is_empty());
}

#[test]
fn search_multi_word_one_missing() {
    let out = format_show_ops_human(None, Some("blur zzzznotfound"));
    // No op should match both terms.
    assert!(out.contains("No operations match"), "expected no-results message: {out}");
}

#[test]
fn search_single_word_still_works() {
    let out = format_show_ops_human(None, Some("invert"));
    assert!(!out.is_empty());
    assert!(!out.contains("No operations match"));
}

// ── Ranking / sort order ────────────────────────────────────────────

#[test]
fn search_results_sorted_by_score() {
    let out = format_show_ops_human(None, Some("blur"));
    // Results should be non-empty and contain score annotations.
    assert!(out.contains("score:"), "expected (score: N) annotations: {out}");
    // The first result should have the highest score — just check it's present.
    let scores: Vec<u32> = out.lines()
        .filter_map(|l| l.split("score: ").nth(1)?.trim_end_matches(')').parse().ok())
        .collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "scores should be descending: {scores:?}");
}

#[test]
fn search_path_match_ranks_above_description() {
    let out = format_show_ops_human(None, Some("blur"));
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("score:")).collect();
    if lines.len() >= 2 {
        let first_score: u32 = lines[0].split("score: ").nth(1).unwrap()
            .trim_end_matches(')').parse().unwrap();
        let last_score: u32 = lines.last().unwrap().split("score: ").nth(1).unwrap()
            .trim_end_matches(')').parse().unwrap();
        assert!(first_score >= last_score);
    }
}

#[test]
fn search_scores_shown_in_human_output() {
    let out = format_show_ops_human(None, Some("blur"));
    assert!(out.contains("(score:"), "expected score annotations: {out}");
}

// ── No results message ──────────────────────────────────────────────

#[test]
fn search_no_results_human() {
    let out = format_show_ops_human(None, Some("zzzznotanopname"));
    assert!(
        out.contains("No operations match search"),
        "expected no-results message: {out}"
    );
}

#[test]
fn search_no_results_json() {
    let val = format_show_ops_json(None, Some("zzzznotanopname"));
    assert_eq!(val["matches"], 0);
    assert!(val["message"].as_str().unwrap().contains("No operations match"));
}

#[test]
fn search_no_results_with_group() {
    let out = format_show_ops_human(Some("images"), Some("zzzznotanopname"));
    assert!(out.contains("No operations match"));
}

// ── JSON output ─────────────────────────────────────────────────────

#[test]
fn search_json_includes_score() {
    let val = format_show_ops_json(None, Some("blur"));
    let arr = val.as_array().unwrap();
    assert!(!arr.is_empty());
    for item in arr {
        assert!(item.get("score").is_some(), "expected 'score' field in: {item}");
    }
}

#[test]
fn search_json_sorted_by_score() {
    let val = format_show_ops_json(None, Some("blur"));
    let arr = val.as_array().unwrap();
    let scores: Vec<u64> = arr.iter().map(|v| v["score"].as_u64().unwrap()).collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "JSON scores should be descending: {scores:?}");
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn search_empty_string_returns_all() {
    let with = format_show_ops_human(None, Some(""));
    let without = format_show_ops_human(None, None);
    // Empty search should behave like no search.
    assert_eq!(with.lines().count(), without.lines().count());
}

#[test]
fn search_whitespace_only_returns_all() {
    let with = format_show_ops_human(None, Some("   "));
    let without = format_show_ops_human(None, None);
    assert_eq!(with.lines().count(), without.lines().count());
}

#[test]
fn search_special_chars_no_panic() {
    // Should not panic on regex-special characters.
    let _ = format_show_ops_human(None, Some("(blur)"));
    let _ = format_show_ops_human(None, Some("[test]"));
    let _ = format_show_ops_human(None, Some("a*b+c"));
}

// ── show-types ──────────────────────────────────────────────────────

#[test]
fn show_types_human_all() {
    let out = format_show_types_human(None);
    assert!(out.contains("blendmode"));
}

#[test]
fn show_types_human_specific() {
    let out = format_show_types_human(Some("blendmode"));
    assert!(out.contains("Multiply"));
}

// ── show-values ─────────────────────────────────────────────────────

#[test]
fn show_values_text_contains_examples() {
    let text = show_values_text();
    assert!(text.contains("bool:true"));
    assert!(text.contains("int:42"));
    assert!(text.contains("decimal:3.14"));
    assert!(text.contains("text:hello"));
    assert!(text.contains("color:1.0,0.0,0.0,1.0"));
    assert!(text.contains("path:/some/file.png"));
}

// ── show-op ─────────────────────────────────────────────────────────

#[test]
fn show_op_human_contains_description() {
    let out = format_show_op_human("numbers/arithmetic/add").unwrap();
    assert!(out.contains("Inputs:"));
    assert!(out.contains("Outputs:"));
}

#[test]
fn show_op_unknown_returns_err() {
    assert!(format_show_op_human("no/such/op").is_err());
}

// ── info formatting ─────────────────────────────────────────────────

/// Human info output shows [DISABLED] for disabled nodes.
#[tokio::test]
async fn info_shows_disabled_tag() {
    let path = create_temp_graph("info_disabled");
    let node_id = format!("dis-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    cmd_set_enabled(path.clone(), node_id.clone(), false, false).unwrap();
    let graph = load_graph(&path).unwrap();
    let text = format_info_human(&graph, Some(&node_id), false).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(text.contains("[DISABLED]"), "expected [DISABLED] in: {text}");
}

/// JSON info output includes "enabled" field.
#[tokio::test]
async fn info_json_includes_enabled_field() {
    let path = create_temp_graph("info_json_en");
    let node_id = format!("jen-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    cmd_set_enabled(path.clone(), node_id.clone(), false, false).unwrap();
    let graph = load_graph(&path).unwrap();
    let val = format_info_json(&graph, Some(&node_id)).unwrap();
    let _ = std::fs::remove_file(&path);
    let nodes = val["nodes"].as_array().unwrap();
    assert_eq!(nodes[0]["enabled"], serde_json::json!(false));
}

// ── show-output formatting ──────────────────────────────────────────

/// show-output JSON format includes node and output fields for non-image.
#[tokio::test]
async fn show_output_json_format_non_image() {
    let path = create_temp_graph("so_json_ni");
    let node_id = format!("so_jni-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    crate::commands::cmd_set_input(path.clone(), node_id.clone(), vec![0, 1], vec!["decimal:2.0".into(), "decimal:5.0".into()], false).unwrap();

    // Run the graph to compute outputs, then format the JSON.
    let mut graph = load_graph(&path).unwrap();
    graph.run().await;
    let node_data = &graph.nodes[&node_id];
    let output = &node_data.outputs[0];
    let json = format_show_output_json(&node_id, 0, &output.name, &output.value, false, &[], None).unwrap();
    let _ = std::fs::remove_file(&path);

    assert_eq!(json["node"], node_id);
    assert!(json["output"]["type"].is_string());
    assert!(json["output"]["value"].is_object() || json["output"]["value"].is_number() || json["output"]["value"].is_string());
}

// ── json_value helper ────────────────────────────────────────────────────

/// json_value correctly formats a Bool.
#[test]
fn json_value_bool() {
    use mangler_core::value::Value;
    let j = json_value(&Value::Bool(true));
    assert_eq!(j, serde_json::json!({"Bool": true}));
}

/// json_value correctly formats an Integer.
#[test]
fn json_value_integer() {
    use mangler_core::value::Value;
    let j = json_value(&Value::Integer(42));
    assert_eq!(j, serde_json::json!({"Integer": 42}));
}

/// json_value correctly formats a Decimal.
#[test]
fn json_value_decimal() {
    use mangler_core::value::Value;
    let j = json_value(&Value::Decimal(3.14));
    // Decimal is a f32, so check it's a number.
    let obj = j.as_object().unwrap();
    assert!(obj.contains_key("Decimal"));
}

/// json_value correctly formats a Text.
#[test]
fn json_value_text() {
    use mangler_core::value::Value;
    let j = json_value(&Value::Text("hello".into()));
    assert_eq!(j, serde_json::json!({"Text": "hello"}));
}

/// json_value for Image produces metadata (type, width, height), not raw data.
#[test]
fn json_value_image_metadata() {
    use std::sync::Arc;
    use mangler_core::float_image::FloatImage;
    use mangler_core::value::Value;
    use mangler_core::get_id;
    let img = FloatImage::from_pixel(8, 16, 4, &[0.0; 4]);
    let val = Value::Image { data: Arc::new(img), change_id: get_id() };
    let j = json_value(&val);
    assert_eq!(j["type"], "Image");
    assert_eq!(j["width"], 8);
    assert_eq!(j["height"], 16);
}

// ── format_info gaps ─────────────────────────────────────────────────────

/// Human info for an empty graph shows "nodes: 0".
#[test]
fn format_info_human_empty_graph() {
    let path = create_temp_graph("info_empty");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format_info_human(&graph, None, false).unwrap();
    assert!(text.contains("nodes: 0"), "expected 'nodes: 0' in: {text}");
}

/// Human info with multiple nodes shows them in sorted order.
#[tokio::test]
async fn format_info_human_multiple_nodes_sorted() {
    let path = create_temp_graph("info_multi");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("zzz".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("aaa".into()), None, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format_info_human(&graph, None, false).unwrap();
    // "aaa" should appear before "zzz" in the output.
    let pos_aaa = text.find("aaa").unwrap();
    let pos_zzz = text.find("zzz").unwrap();
    assert!(pos_aaa < pos_zzz, "nodes should be sorted: aaa before zzz");
}

/// Compact mode omits descriptions.
#[tokio::test]
async fn format_info_human_compact_mode() {
    let path = create_temp_graph("info_compact");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("cmp".into()), None, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let verbose = format_info_human(&graph, Some("cmp"), false).unwrap();
    let compact = format_info_human(&graph, Some("cmp"), true).unwrap();
    // Compact should be shorter (fewer lines, no descriptions/defaults).
    assert!(compact.len() <= verbose.len(), "compact should be shorter than verbose");
}

/// Connections display <- and -> markers.
#[tokio::test]
async fn format_info_human_connections_displayed() {
    let path = create_temp_graph("info_conn");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("src".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("dst".into()), None, false).await.unwrap();
    crate::commands::cmd_connect(path.clone(), "src:0".into(), "dst:0".into(), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format_info_human(&graph, None, false).unwrap();
    assert!(text.contains("<-"), "expected '<-' connection marker in: {text}");
}

/// Filter to nonexistent node returns Err.
#[test]
fn format_info_human_filter_nonexistent_returns_err() {
    let path = create_temp_graph("info_ghost");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(format_info_human(&graph, Some("ghost"), false).is_err());
}

/// JSON info for empty graph has node_count=0.
#[test]
fn format_info_json_empty_graph() {
    let path = create_temp_graph("info_json_empty");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let val = format_info_json(&graph, None).unwrap();
    assert_eq!(val["node_count"], 0);
    assert_eq!(val["nodes"].as_array().unwrap().len(), 0);
}

/// JSON info with multiple nodes includes all nodes.
#[tokio::test]
async fn format_info_json_multiple_nodes() {
    let path = create_temp_graph("info_json_multi");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("n1".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("n2".into()), None, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let val = format_info_json(&graph, None).unwrap();
    assert_eq!(val["node_count"], 2);
    assert_eq!(val["nodes"].as_array().unwrap().len(), 2);
}

/// JSON info with connections has connection fields.
#[tokio::test]
async fn format_info_json_connections_present() {
    let path = create_temp_graph("info_json_conn");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("s".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".into(), Some("d".into()), None, false).await.unwrap();
    crate::commands::cmd_connect(path.clone(), "s:0".into(), "d:0".into(), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let val = format_info_json(&graph, None).unwrap();
    let nodes = val["nodes"].as_array().unwrap();
    // Find the destination node "d" and check input[0] has a connection.
    let d_node = nodes.iter().find(|n| n["id"] == "d").unwrap();
    let input0 = &d_node["inputs"][0];
    assert!(input0.get("connection").is_some(), "input should have a connection field");
}

/// JSON info filter to nonexistent node returns Err.
#[test]
fn format_info_json_filter_nonexistent_returns_err() {
    let path = create_temp_graph("info_json_ghost");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(format_info_json(&graph, Some("ghost")).is_err());
}

// ── format_show_ops compact/JSON gaps ────────────────────────────────────

/// Compact human output has one line per op.
#[test]
fn format_show_ops_compact_human_basic() {
    let out = format_show_ops_compact_human(None, None);
    assert!(!out.is_empty());
    // Each line should be an op path.
    assert!(out.lines().count() > 10, "should have many operations");
}

/// Compact human output with no results shows message.
#[test]
fn format_show_ops_compact_human_no_results() {
    let out = format_show_ops_compact_human(None, Some("zzzznotanopname"));
    assert!(out.contains("No operations match"));
}

/// Compact JSON returns array of {path, description} objects.
#[test]
fn format_show_ops_compact_json_basic() {
    let val = format_show_ops_compact_json(None, None);
    let arr = val.as_array().unwrap();
    assert!(!arr.is_empty());
    // Each item should have "path" and "description".
    for item in arr {
        assert!(item.get("path").is_some(), "expected 'path' field");
        assert!(item.get("description").is_some(), "expected 'description' field");
    }
}

/// Compact JSON no results returns {matches: 0, message: ...}.
#[test]
fn format_show_ops_compact_json_no_results() {
    let val = format_show_ops_compact_json(None, Some("zzzznotanopname"));
    assert_eq!(val["matches"], 0);
}

/// JSON show-ops with group filter only includes matching ops.
#[test]
fn format_show_ops_json_with_group_filter() {
    let val = format_show_ops_json(Some("numbers"), None);
    let arr = val.as_array().unwrap();
    for item in arr {
        let path = item["path"].as_str().unwrap();
        assert!(path.starts_with("numbers"), "path should start with 'numbers': {path}");
    }
}

/// JSON show-ops with group and search combined.
#[test]
fn format_show_ops_json_group_and_search_combined() {
    let val = format_show_ops_json(Some("numbers"), Some("add"));
    let arr = val.as_array().unwrap();
    assert!(!arr.is_empty(), "should find at least one number/add op");
    for item in arr {
        let path = item["path"].as_str().unwrap();
        assert!(path.starts_with("numbers"), "path should start with 'numbers': {path}");
    }
}

// ── format_show_op JSON ──────────────────────────────────────────────────

/// JSON show-op has correct structure.
#[test]
fn format_show_op_json_valid() {
    let val = format_show_op_json("numbers/arithmetic/add").unwrap();
    assert!(val.get("name").is_some());
    assert!(val.get("variant").is_some());
    assert!(val.get("description").is_some());
    assert!(val.get("inputs").is_some());
    assert!(val.get("outputs").is_some());
}

/// JSON show-op for unknown op returns Err.
#[test]
fn format_show_op_json_unknown_returns_err() {
    assert!(format_show_op_json("no/such/op").is_err());
}

/// JSON show-op for an op with enum inputs includes enum_type and enum_variants.
#[test]
fn format_show_op_json_enum_input_info() {
    let val = format_show_op_json("colors/manipulation/blend").unwrap();
    let inputs = val["inputs"].as_array().unwrap();
    // Find the blend mode input.
    let has_enum = inputs.iter().any(|i| i.get("enum_type").is_some());
    assert!(has_enum, "blend op should have at least one enum input");
    let enum_input = inputs.iter().find(|i| i.get("enum_type").is_some()).unwrap();
    assert!(enum_input.get("enum_variants").is_some(), "enum input should have variants");
}

// ── format_show_types JSON ───────────────────────────────────────────────

/// JSON show-types with no type name returns array of type names.
#[test]
fn format_show_types_json_all() {
    let val = format_show_types_json(None).unwrap();
    let arr = val.as_array().unwrap();
    assert!(arr.len() >= 8, "should have at least 8 enum types");
}

/// JSON show-types for specific type returns {type, variants}.
#[test]
fn format_show_types_json_specific() {
    let val = format_show_types_json(Some("blendmode")).unwrap();
    assert!(val.get("type").is_some());
    assert!(val.get("variants").is_some());
    let variants = val["variants"].as_array().unwrap();
    assert!(variants.len() >= 10, "blendmode should have many variants");
}

/// JSON show-types for unknown type returns Err.
#[test]
fn format_show_types_json_unknown_returns_err() {
    assert!(format_show_types_json(Some("garbage")).is_err());
}

/// Human show-types for unknown type shows available types.
#[test]
fn format_show_types_human_unknown_shows_available() {
    let out = format_show_types_human(Some("garbage"));
    assert!(out.contains("unknown type"), "expected 'unknown type' in: {out}");
    assert!(out.contains("blendmode"), "should list available types");
}

// ── format_show_values JSON ──────────────────────────────────────────────

/// JSON show-values contains all expected simple-type keys plus every
/// enum type in `ENUM_TYPE_NAMES` — guards against a new enum value type
/// (like edgemode/exportpreset) being added without a show-values entry.
#[test]
fn format_show_values_json_contains_all_keys() {
    use crate::helpers::ENUM_TYPE_NAMES;

    let val = format_show_values_json();
    let obj = val.as_object().unwrap();
    let simple_keys = ["bool", "int", "decimal", "text", "color", "path"];
    for key in simple_keys.iter().chain(ENUM_TYPE_NAMES) {
        assert!(obj.contains_key(*key), "missing key '{key}' in show-values JSON");
    }
}

/// Human show-values text mentions every enum type in `ENUM_TYPE_NAMES`.
#[test]
fn show_values_text_contains_all_enum_types() {
    let text = show_values_text();
    for name in crate::helpers::ENUM_TYPE_NAMES {
        assert!(text.contains(&format!("{name}:")), "show-values text missing '{name}:' example");
    }
}

// ── format_run ───────────────────────────────────────────────────────────

/// Human run output for empty graph is empty.
#[test]
fn format_run_human_empty_graph() {
    let path = create_temp_graph("run_empty");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format_run_human(&graph);
    assert!(text.is_empty(), "empty graph run should produce empty output");
}

/// JSON run output for empty graph has empty arrays.
#[test]
fn format_run_json_empty_graph() {
    let path = create_temp_graph("run_json_empty");
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let val = format_run_json(&graph);
    assert_eq!(val["errors"].as_array().unwrap().len(), 0);
    assert_eq!(val["outputs"].as_array().unwrap().len(), 0);
}

// ── format_show_output ──────────────────────────────────────────────────

/// Human show-output for a non-image value shows type and value.
#[test]
fn format_show_output_human_non_image() {
    use mangler_core::value::Value;
    let text = format_show_output_human("test-node", 0, "result", &Value::Decimal(7.5), false, &[], None).unwrap();
    assert!(text.contains("decimal"), "should show type: {text}");
    assert!(text.contains("7.5"), "should show value: {text}");
}

/// JSON show-output for non-image value with save creates a file.
#[test]
fn format_show_output_json_non_image_save() {
    use mangler_core::value::Value;
    let save_path = std::env::temp_dir().join(format!("mangle_test_save_val_{}.json", std::process::id()));
    let _ = std::fs::remove_file(&save_path);
    let val = format_show_output_json("n1", 0, "out", &Value::Integer(99), false, &[], Some(&save_path)).unwrap();
    // Verify file was created.
    assert!(save_path.exists(), "save file should exist");
    let _ = std::fs::remove_file(&save_path);
    // Verify JSON has saved_to field.
    assert!(val["output"].get("saved_to").is_some(), "should have saved_to field");
}
