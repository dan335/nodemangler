use super::*;

use mangler_core::value::{Value, ValueType};

use crate::helpers::{create_temp_graph, load_graph, save_graph};

// ── cmd_new ───────────────────────────────────────────────────────────────

#[test]
fn cmd_new_creates_valid_graph_file() {
    let path = crate::helpers::temp_graph_path("new_creates");
    let _ = std::fs::remove_file(&path);
    cmd_new(path.clone(), false).unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes.is_empty());
}

#[test]
fn cmd_new_uses_stem_as_graph_name() {
    let path = crate::helpers::temp_graph_path("stem_check");
    let _ = std::fs::remove_file(&path);
    cmd_new(path.clone(), false).unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!graph.name.is_empty());
}

#[test]
fn cmd_new_fails_if_file_already_exists() {
    let path = create_temp_graph("already_exists");
    let result = cmd_new(path.clone(), false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn cmd_new_appends_mangle_json_when_no_json_extension() {
    let base = std::env::temp_dir().join(format!("mangle_test_ext_{}", std::process::id()));
    // Clean up both potential outputs.
    let expected = std::path::PathBuf::from(format!("{}.mangle.json", base.display()));
    let _ = std::fs::remove_file(&base);
    let _ = std::fs::remove_file(&expected);
    cmd_new(base.clone(), false).unwrap();
    assert!(expected.exists(), "expected {} to exist", expected.display());
    let graph = load_graph(&expected).unwrap();
    let _ = std::fs::remove_file(&expected);
    assert!(graph.nodes.is_empty());
}

#[test]
fn cmd_new_keeps_json_extension_unchanged() {
    let path = std::env::temp_dir().join(format!("mangle_test_keepext_{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    cmd_new(path.clone(), false).unwrap();
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_fails_in_nonexistent_directory() {
    let path = std::path::PathBuf::from("/no/such/directory/graph.json");
    assert!(cmd_new(path, false).is_err());
}

// ── cmd_set_input ─────────────────────────────────────────────────────────

#[test]
fn cmd_set_input_invalid_json_returns_err() {
    let path = create_temp_graph("setinput_badjson");
    let result = cmd_set_input(path.clone(), "any".to_string(), vec![0], vec!["not json".to_string()], false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn cmd_set_input_unknown_node_returns_err() {
    let path = create_temp_graph("setinput_nonode");
    let result = cmd_set_input(path.clone(), "ghost".to_string(), vec![0], vec![r#"{"Integer":1}"#.to_string()], false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

// ── async integration tests ───────────────────────────────────────────────

#[tokio::test]
async fn cmd_add_node_persists_to_file() {
    let path = create_temp_graph("addnode");
    let node_id = format!("test-node-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes.contains_key(&node_id));
}

#[tokio::test]
async fn cmd_add_then_remove_node_leaves_graph_empty() {
    let path = create_temp_graph("addremove");
    let node_id = format!("addremove-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    cmd_remove_node(path.clone(), node_id.clone(), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!graph.nodes.contains_key(&node_id));
}

#[tokio::test]
async fn cmd_remove_node_unknown_returns_err() {
    let path = create_temp_graph("removemissing");
    let result = cmd_remove_node(path.clone(), "ghost".to_string(), false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[tokio::test]
async fn cmd_set_input_on_real_node_succeeds() {
    let path = create_temp_graph("setinput_valid");
    let node_id = format!("add-node-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    let result = cmd_set_input(path.clone(), node_id.clone(), vec![0], vec![r#"{"Decimal":7.0}"#.to_string()], false);
    assert!(result.is_ok());
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let stored = &graph.nodes[&node_id].inputs[0].value;
    assert!(matches!(stored, Value::Decimal(v) if (*v - 7.0).abs() < 1e-6), "unexpected: {:?}", stored);
}

#[tokio::test]
async fn cmd_connect_stores_connection_on_consumer() {
    let path = create_temp_graph("connect");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some("producer".to_string()), None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some("consumer".to_string()), None, false).await.unwrap();
    cmd_connect(path.clone(),"producer:0".to_string(), "consumer:0".to_string(), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes["consumer"].inputs[0].connection, Some(("producer".to_string(), 0)));
}

#[tokio::test]
async fn cmd_disconnect_removes_connection() {
    let path = create_temp_graph("disconnect");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some("src".to_string()), None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some("dst".to_string()), None, false).await.unwrap();
    cmd_connect(path.clone(),"src:0".to_string(), "dst:0".to_string(), false).await.unwrap();
    cmd_disconnect(path.clone(), "dst".to_string(), 0, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes["dst"].inputs[0].connection, None);
}

#[tokio::test]
async fn cmd_disconnect_unknown_node_returns_err() {
    let path = create_temp_graph("disc_nonode");
    let result = cmd_disconnect(path.clone(), "ghost".to_string(), 0, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[tokio::test]
async fn cmd_add_node_auto_id_is_unique_across_calls() {
    let path = create_temp_graph("autoid");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), None, None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), None, None, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes.len(), 2, "expected exactly 2 distinct nodes");
}

// ── do_* function unit tests ────────────────────────────────────────────

#[tokio::test]
async fn do_add_node_invalid_op_returns_err() {
    let path = create_temp_graph("do_addnode_bad");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_add_node(&mut graph,"not/a/real/op", None, None).await.is_err());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn do_remove_node_missing_returns_err() {
    let path = create_temp_graph("do_rmnode_miss");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_remove_node(&mut graph, "ghost").await.is_err());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn do_disconnect_missing_node_returns_err() {
    let path = create_temp_graph("do_disc_miss");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_disconnect(&mut graph, "ghost", 0).await.is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn do_set_input_invalid_json_returns_err() {
    let path = create_temp_graph("do_setinput_bad");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_set_input(&mut graph, "any", 0, "not json").is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn do_set_input_missing_node_returns_err() {
    let path = create_temp_graph("do_setinput_miss");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_set_input(&mut graph, "ghost", 0, r#"{"Integer":1}"#).is_err());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn do_connect_bad_slot_returns_err() {
    let path = create_temp_graph("do_conn_bad");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_connect(&mut graph, "nocolon", "also_bad").await.is_err());
    let _ = std::fs::remove_file(&path);
}

// ── error messages ──────────────────────────────────────────────────

#[tokio::test]
async fn set_input_missing_node_error() {
    let path = create_temp_graph("setinput_miss");
    let mut graph = load_graph(&path).unwrap();
    let result = do_set_input(&mut graph, "ghost", 0, r#"{"Integer":1}"#);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn set_input_out_of_bounds_error() {
    let path = create_temp_graph("setinput_oob");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("oob1".to_string()), None).await.unwrap();
    let result = do_set_input(&mut graph, "oob1", 999, r#"{"Integer":1}"#);
    let _ = std::fs::remove_file(&path);
    assert!(result.unwrap_err().contains("out of range"));
}

#[tokio::test]
async fn set_input_enum_error_includes_valid_values() {
    let path = create_temp_graph("setinput_enum");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"colors/manipulation/blend", Some("blend1".to_string()), None).await.unwrap();
    let blend_node = &graph.nodes["blend1"];
    let blend_idx = blend_node.inputs.iter().position(|i| matches!(i.value.value_type(), ValueType::BlendMode));
    if let Some(idx) = blend_idx {
        let err = do_set_input(&mut graph, "blend1", idx, "InvalidValue").unwrap_err();
        assert!(err.contains("blendmode") && err.contains("Multiply"));
    }
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_missing_source_node_error() {
    let path = create_temp_graph("conn_src_miss");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("dst".to_string()), None).await.unwrap();
    assert!(do_connect(&mut graph, "ghost:0", "dst:0").await.unwrap_err().contains("node 'ghost' not found"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_missing_dest_node_error() {
    let path = create_temp_graph("conn_dst_miss");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("src".to_string()), None).await.unwrap();
    assert!(do_connect(&mut graph, "src:0", "ghost:0").await.unwrap_err().contains("node 'ghost' not found"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_output_out_of_bounds_error() {
    let path = create_temp_graph("conn_out_oob");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("a".to_string()), None).await.unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("b".to_string()), None).await.unwrap();
    let err = do_connect(&mut graph, "a:999", "b:0").await.unwrap_err();
    assert!(err.contains("output index") && err.contains("out of range"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_input_out_of_bounds_error() {
    let path = create_temp_graph("conn_in_oob");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("a".to_string()), None).await.unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("b".to_string()), None).await.unwrap();
    let err = do_connect(&mut graph, "a:0", "b:999").await.unwrap_err();
    assert!(err.contains("input index") && err.contains("out of range"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_valid_still_works() {
    let path = create_temp_graph("conn_valid");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("v1".to_string()), None).await.unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("v2".to_string()), None).await.unwrap();
    assert!(do_connect(&mut graph, "v1:0", "v2:0").await.is_ok());
    let _ = std::fs::remove_file(&path);
}

// ── typed value integration tests ───────────────────────────────────

/// Integration: set-input with typed value on a real node.
#[tokio::test]
async fn set_input_typed_value_decimal_on_real_node() {
    let path = create_temp_graph("typed_decimal");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("n1".to_string()), None).await.unwrap();
    let result = do_set_input(&mut graph, "n1", 0, "decimal:7.5");
    assert!(result.is_ok());
    let stored = &graph.nodes["n1"].inputs[0].value;
    assert!(matches!(stored, Value::Decimal(v) if (*v - 7.5).abs() < 1e-6));
    let _ = std::fs::remove_file(&path);
}

/// Integration: set-input with typed enum value on a real node.
#[tokio::test]
async fn set_input_typed_value_blend_mode_on_real_node() {
    let path = create_temp_graph("typed_blend");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"colors/manipulation/blend", Some("b1".to_string()), None).await.unwrap();
    let blend_node = &graph.nodes["b1"];
    let blend_idx = blend_node.inputs.iter().position(|i| matches!(i.value.value_type(), ValueType::BlendMode));
    if let Some(idx) = blend_idx {
        let result = do_set_input(&mut graph, "b1", idx, "BlendMode:Screen");
        assert!(result.is_ok());
    }
    let _ = std::fs::remove_file(&path);
}

// ── batch set-input ──────────────────────────────────────────────────

/// Batch set-input sets multiple inputs in a single load/save cycle.
#[tokio::test]
async fn cmd_set_input_batch_multiple_pairs() {
    let path = create_temp_graph("batch_multi");
    let node_id = format!("batch-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    let result = cmd_set_input(
        path.clone(),
        node_id.clone(),
        vec![0, 1],
        vec!["decimal:1.5".to_string(), "decimal:2.5".to_string()],
        false,
    );
    assert!(result.is_ok());
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let n = &graph.nodes[&node_id];
    assert!(matches!(n.inputs[0].value, Value::Decimal(v) if (v - 1.5).abs() < 1e-6));
    assert!(matches!(n.inputs[1].value, Value::Decimal(v) if (v - 2.5).abs() < 1e-6));
}

/// Mismatched --input/--value counts produce an error.
#[test]
fn cmd_set_input_batch_mismatched_counts() {
    let path = create_temp_graph("batch_mismatch");
    let result = cmd_set_input(
        path.clone(),
        "any".to_string(),
        vec![0, 1],
        vec!["decimal:1.0".to_string()],
        false,
    );
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("mismatched"));
}

/// Batch set-input fails fast: if the second pair is invalid, the first is not saved.
#[tokio::test]
async fn cmd_set_input_batch_fails_fast_on_bad_value() {
    let path = create_temp_graph("batch_failfast");
    let node_id = format!("failfast-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    // First value is valid, second is garbage — should fail and NOT save.
    let result = cmd_set_input(
        path.clone(),
        node_id.clone(),
        vec![0, 0],
        vec!["decimal:1.0".to_string(), "not_valid".to_string()],
        false,
    );
    assert!(result.is_err());
    // The graph on disk should still have the default value (not 1.0).
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let stored = &graph.nodes[&node_id].inputs[0].value;
    // Default for add input 0 is Decimal(0.0).
    assert!(matches!(stored, Value::Decimal(v) if v.abs() < 1e-6), "expected default, got {:?}", stored);
}

/// Single --input/--value pair still works (backward compat).
#[tokio::test]
async fn cmd_set_input_single_pair_backward_compat() {
    let path = create_temp_graph("batch_single");
    let node_id = format!("single-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    let result = cmd_set_input(
        path.clone(),
        node_id.clone(),
        vec![0],
        vec!["decimal:9.0".to_string()],
        false,
    );
    assert!(result.is_ok());
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(matches!(graph.nodes[&node_id].inputs[0].value, Value::Decimal(v) if (v - 9.0).abs() < 1e-6));
}

// ── set-enabled ──────────────────────────────────────────────────────

/// Disabling a node persists to file.
#[tokio::test]
async fn cmd_set_enabled_disables_node() {
    let path = create_temp_graph("set_enabled_off");
    let node_id = format!("en-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    cmd_set_enabled(path.clone(), node_id.clone(), false, false).unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!graph.nodes[&node_id].is_enabled);
}

/// Re-enabling a node persists to file.
#[tokio::test]
async fn cmd_set_enabled_re_enables_node() {
    let path = create_temp_graph("set_enabled_on");
    let node_id = format!("en2-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    cmd_set_enabled(path.clone(), node_id.clone(), false, false).unwrap();
    cmd_set_enabled(path.clone(), node_id.clone(), true, false).unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes[&node_id].is_enabled);
}

/// set-enabled on a missing node returns an error.
#[test]
fn cmd_set_enabled_missing_node_returns_err() {
    let path = create_temp_graph("set_enabled_miss");
    let result = cmd_set_enabled(path.clone(), "ghost".to_string(), false, false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

/// Nodes default to enabled (is_enabled == true).
#[tokio::test]
async fn node_defaults_to_enabled() {
    let path = create_temp_graph("default_enabled");
    let node_id = format!("def-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes[&node_id].is_enabled);
}

// ── show-output command tests ────────────────────────────────────────────

/// show-output returns an error for a nonexistent node.
#[tokio::test]
async fn show_output_node_not_found() {
    let path = create_temp_graph("so_notfound");
    let result = cmd_show_output(path.clone(), "nope".into(), None, false, vec![], None, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

/// show-output returns an error for an out-of-range output index.
#[tokio::test]
async fn show_output_index_out_of_range() {
    let path = create_temp_graph("so_oor");
    let node_id = format!("so_oor-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    let result = cmd_show_output(path.clone(), node_id, Some(99), false, vec![], None, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of range"));
}

/// show-output works for a non-image node (arithmetic add outputs a decimal).
#[tokio::test]
async fn show_output_non_image_value() {
    let path = create_temp_graph("so_nonimg");
    let node_id = format!("so_ni-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".to_string(), Some(node_id.clone()), None, false).await.unwrap();
    cmd_set_input(path.clone(), node_id.clone(), vec![0, 1], vec!["decimal:3.0".into(), "decimal:7.0".into()], false).unwrap();
    // Run show-output and check it succeeds.
    let result = cmd_show_output(path.clone(), node_id.clone(), Some(0), false, vec![], None, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

// ── Untested command handlers (smoke tests) ──────────────────────────────

/// cmd_info on an empty graph returns Ok.
#[test]
fn cmd_info_empty_graph_succeeds() {
    let path = create_temp_graph("info_empty");
    let result = cmd_info(path.clone(), None, false, false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// cmd_info on a graph with one node returns Ok.
#[tokio::test]
async fn cmd_info_single_node_succeeds() {
    let path = create_temp_graph("info_single");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("n1".into()), None, false).await.unwrap();
    let result = cmd_info(path.clone(), None, false, false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// cmd_info with compact mode returns Ok.
#[tokio::test]
async fn cmd_info_compact_mode() {
    let path = create_temp_graph("info_compact");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("c1".into()), None, false).await.unwrap();
    let result = cmd_info(path.clone(), None, true, false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// cmd_info filtering to a specific node returns Ok.
#[tokio::test]
async fn cmd_info_filter_node() {
    let path = create_temp_graph("info_filter");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("f1".into()), None, false).await.unwrap();
    let result = cmd_info(path.clone(), Some("f1".into()), false, false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// cmd_info filtering to nonexistent node returns Err.
#[test]
fn cmd_info_filter_nonexistent_returns_err() {
    let path = create_temp_graph("info_ghost");
    let result = cmd_info(path.clone(), Some("ghost".into()), false, false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

/// cmd_show_ops basic smoke test.
#[test]
fn cmd_show_ops_returns_ok() {
    assert!(cmd_show_ops(None, None, false, false).is_ok());
}

/// cmd_show_ops compact mode smoke test.
#[test]
fn cmd_show_ops_compact_returns_ok() {
    assert!(cmd_show_ops(None, None, true, false).is_ok());
}

/// cmd_show_types basic smoke test.
#[test]
fn cmd_show_types_returns_ok() {
    assert!(cmd_show_types(None, false).is_ok());
}

/// cmd_show_types for a specific type returns Ok.
#[test]
fn cmd_show_types_specific_returns_ok() {
    assert!(cmd_show_types(Some("blendmode".into()), false).is_ok());
}

/// cmd_show_values basic smoke test.
#[test]
fn cmd_show_values_returns_ok() {
    assert!(cmd_show_values(false).is_ok());
}

/// cmd_show_op for a valid op returns Ok.
#[test]
fn cmd_show_op_returns_ok() {
    assert!(cmd_show_op("numbers/arithmetic/add".into(), false).is_ok());
}

/// cmd_show_op for unknown op returns Err.
#[test]
fn cmd_show_op_unknown_returns_err() {
    assert!(cmd_show_op("no/such/op".into(), false).is_err());
}

/// cmd_run on an empty graph returns Ok.
#[tokio::test]
async fn cmd_run_empty_graph() {
    let path = create_temp_graph("run_empty");
    let result = cmd_run(path.clone(), false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// cmd_run on a graph with a node returns Ok.
#[tokio::test]
async fn cmd_run_graph_with_node() {
    let path = create_temp_graph("run_node");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("r1".into()), None, false).await.unwrap();
    cmd_set_input(path.clone(), "r1".into(), vec![0, 1], vec!["decimal:3.0".into(), "decimal:4.0".into()], false).unwrap();
    let result = cmd_run(path.clone(), false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

// ── cmd_show_output advanced paths ───────────────────────────────────────

/// show-output with no output index shows all outputs.
#[tokio::test]
async fn cmd_show_output_all_outputs() {
    let path = create_temp_graph("so_all");
    let nid = format!("so_all-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some(nid.clone()), None, false).await.unwrap();
    // output_index=None means show all.
    let result = cmd_show_output(path.clone(), nid, None, false, vec![], None, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// show-output --sample on a non-image output returns Err.
#[tokio::test]
async fn cmd_show_output_sample_non_image_returns_err() {
    let path = create_temp_graph("so_sample_ni");
    let nid = format!("so_sni-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some(nid.clone()), None, false).await.unwrap();
    let result = cmd_show_output(path.clone(), nid, Some(0), false, vec!["center".into()], None, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not an image"));
}

// ── do_* edge cases ──────────────────────────────────────────────────────

/// do_add_node with custom ID returns the same ID.
#[tokio::test]
async fn do_add_node_custom_id_matches() {
    let path = create_temp_graph("do_add_custom");
    let mut graph = load_graph(&path).unwrap();
    let id = do_add_node(&mut graph,"numbers/arithmetic/add", Some("my-custom-id".into()), None).await.unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(id, "my-custom-id");
}

/// do_add_node without ID returns a non-empty auto-generated ID.
#[tokio::test]
async fn do_add_node_auto_id_not_empty() {
    let path = create_temp_graph("do_add_auto");
    let mut graph = load_graph(&path).unwrap();
    let id = do_add_node(&mut graph,"numbers/arithmetic/add", None, None).await.unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!id.is_empty(), "auto-generated ID should not be empty");
}

/// do_connect returns a description containing "connected".
#[tokio::test]
async fn do_connect_returns_description() {
    let path = create_temp_graph("do_conn_desc");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("a".into()), None).await.unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("b".into()), None).await.unwrap();
    let msg = do_connect(&mut graph, "a:0", "b:0").await.unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(msg.contains("connected"), "expected 'connected' in: {msg}");
}

/// do_disconnect returns a description containing "disconnected".
#[tokio::test]
async fn do_disconnect_returns_description() {
    let path = create_temp_graph("do_disc_desc");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("a".into()), None).await.unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("b".into()), None).await.unwrap();
    do_connect(&mut graph, "a:0", "b:0").await.unwrap();
    let msg = do_disconnect(&mut graph, "b", 0).await.unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(msg.contains("disconnected"), "expected 'disconnected' in: {msg}");
}

// ── End-to-end workflows ─────────────────────────────────────────────────

/// Full pipeline: create graph, add 2 nodes, connect them, run.
#[tokio::test]
async fn workflow_create_add_connect_run() {
    let path = create_temp_graph("wf_full");
    // Add two add nodes.
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("a".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("b".into()), None, false).await.unwrap();
    // Set inputs on node "a".
    cmd_set_input(path.clone(), "a".into(), vec![0, 1], vec!["decimal:2.0".into(), "decimal:3.0".into()], false).unwrap();
    // Connect a:0 (output) -> b:0 (input).
    cmd_connect(path.clone(),"a:0".into(), "b:0".into(), false).await.unwrap();
    // Run the graph.
    let result = cmd_run(path.clone(), false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// Connection lifecycle: connect, disconnect, reconnect.
#[tokio::test]
async fn workflow_connect_disconnect_reconnect() {
    let path = create_temp_graph("wf_reconn");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("x".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("y".into()), None, false).await.unwrap();
    // Connect.
    cmd_connect(path.clone(),"x:0".into(), "y:0".into(), false).await.unwrap();
    let g1 = load_graph(&path).unwrap();
    assert_eq!(g1.nodes["y"].inputs[0].connection, Some(("x".into(), 0)));
    // Disconnect.
    cmd_disconnect(path.clone(), "y".into(), 0, false).await.unwrap();
    let g2 = load_graph(&path).unwrap();
    assert_eq!(g2.nodes["y"].inputs[0].connection, None);
    // Reconnect.
    cmd_connect(path.clone(),"x:0".into(), "y:0".into(), false).await.unwrap();
    let g3 = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(g3.nodes["y"].inputs[0].connection, Some(("x".into(), 0)));
}

/// Three-node chain: A -> B -> C, run and verify graph completes.
#[tokio::test]
async fn workflow_three_node_chain() {
    let path = create_temp_graph("wf_chain");
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("a".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("b".into()), None, false).await.unwrap();
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some("c".into()), None, false).await.unwrap();
    // Set initial values on node a.
    cmd_set_input(path.clone(), "a".into(), vec![0, 1], vec!["decimal:1.0".into(), "decimal:2.0".into()], false).unwrap();
    // Chain: a:0 -> b:0, b:0 -> c:0.
    cmd_connect(path.clone(),"a:0".into(), "b:0".into(), false).await.unwrap();
    cmd_connect(path.clone(),"b:0".into(), "c:0".into(), false).await.unwrap();
    // Run.
    let result = cmd_run(path.clone(), false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// Set inputs, run in-memory, and verify the computed output value.
#[tokio::test]
async fn workflow_set_input_then_run_verifies_output() {
    let path = create_temp_graph("wf_verify");
    let nid = format!("verify-{}", std::process::id());
    cmd_add_node(path.clone(),"numbers/arithmetic/add".into(), Some(nid.clone()), None, false).await.unwrap();
    cmd_set_input(path.clone(), nid.clone(), vec![0, 1], vec!["decimal:10.0".into(), "decimal:20.0".into()], false).unwrap();
    // Load and run in-memory to inspect computed outputs (saved graphs don't persist output values).
    let mut graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    graph.run().await;
    let output = &graph.nodes[&nid].outputs[0].value;
    // Add operation: 10.0 + 20.0 = 30.0
    assert!(matches!(output, Value::Decimal(v) if (*v - 30.0).abs() < 0.01), "expected 30.0, got: {:?}", output);
}

// === add_node with is_enabled and custom_name ===

/// do_add_node creates a node that is enabled by default with no custom name.
#[tokio::test]
async fn do_add_node_default_enabled_and_no_name() {
    let path = create_temp_graph("add_defaults");
    let mut graph = load_graph(&path).unwrap();
    let id = do_add_node(&mut graph,"numbers/arithmetic/add", Some("n1".into()), None).await.unwrap();
    let _ = std::fs::remove_file(&path);

    let node = graph.nodes.get(&id).unwrap();
    assert!(node.is_enabled, "node should be enabled by default");
    assert!(node.custom_name.is_none(), "node should have no custom name by default");
}

/// custom_name persists through save and reload.
#[tokio::test]
async fn custom_name_persists_through_save_reload() {
    let path = create_temp_graph("name_persist");
    let mut graph = load_graph(&path).unwrap();
    let id = do_add_node(&mut graph,"numbers/arithmetic/add", Some("named".into()), None).await.unwrap();

    // Set a custom name directly on the node and save.
    graph.nodes.get_mut(&id).unwrap().custom_name = Some("mountains image".to_string());
    save_graph(&graph, &path).unwrap();

    // Reload and verify.
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        reloaded.nodes.get(&id).unwrap().custom_name.as_deref(),
        Some("mountains image"),
    );
}

/// is_enabled persists through save and reload.
#[tokio::test]
async fn is_enabled_persists_through_save_reload() {
    let path = create_temp_graph("enabled_persist");
    let mut graph = load_graph(&path).unwrap();
    let id = do_add_node(&mut graph,"numbers/arithmetic/add", Some("dis".into()), None).await.unwrap();

    // Disable the node and save.
    graph.nodes.get_mut(&id).unwrap().is_enabled = false;
    save_graph(&graph, &path).unwrap();

    // Reload and verify.
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!reloaded.nodes.get(&id).unwrap().is_enabled);
}

/// Old save files without custom_name field load with None.
#[tokio::test]
async fn old_save_without_custom_name_loads_as_none() {
    let path = create_temp_graph("old_compat");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph,"numbers/arithmetic/add", Some("old".into()), None).await.unwrap();
    save_graph(&graph, &path).unwrap();

    // Remove the custom_name field from the JSON to simulate an old save file.
    let json_str = std::fs::read_to_string(&path).unwrap();
    let cleaned = json_str.replace(r#","custom_name":null"#, "");
    std::fs::write(&path, cleaned).unwrap();

    // Reload — should still work with custom_name defaulting to None.
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(reloaded.nodes.get("old").unwrap().custom_name.is_none());
}

// === do_add_node with custom_name ===

/// do_add_node with a custom name sets the name on the created node.
#[tokio::test]
async fn do_add_node_with_custom_name() {
    let path = create_temp_graph("add_with_name");
    let mut graph = load_graph(&path).unwrap();
    let id = do_add_node(&mut graph, "numbers/arithmetic/add", Some("n1".into()), Some("My Adder".into())).await.unwrap();
    let _ = std::fs::remove_file(&path);

    let node = graph.nodes.get(&id).unwrap();
    assert_eq!(node.custom_name.as_deref(), Some("My Adder"));
}

// === cmd_set_name ===

/// cmd_set_name sets a custom name on an existing node.
#[tokio::test]
async fn cmd_set_name_sets_custom_name() {
    let path = create_temp_graph("set_name_basic");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("n1".into()), None).await.unwrap();
    save_graph(&graph, &path).unwrap();

    cmd_set_name(path.clone(), "n1".to_string(), "My Node".to_string(), false).unwrap();
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(reloaded.nodes.get("n1").unwrap().custom_name.as_deref(), Some("My Node"));
}

/// cmd_set_name with empty string clears the custom name.
#[tokio::test]
async fn cmd_set_name_empty_clears_name() {
    let path = create_temp_graph("set_name_clear");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("n1".into()), Some("Old Name".into())).await.unwrap();
    save_graph(&graph, &path).unwrap();

    cmd_set_name(path.clone(), "n1".to_string(), "".to_string(), false).unwrap();
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(reloaded.nodes.get("n1").unwrap().custom_name.is_none());
}

/// cmd_set_name on a missing node returns an error.
#[test]
fn cmd_set_name_missing_node_returns_err() {
    let path = create_temp_graph("set_name_miss");
    let result = cmd_set_name(path.clone(), "ghost".to_string(), "Boo".to_string(), false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

// ── AI operations TUI tests ─────────────────────────────────────────────────

/// show-ops with --group ai lists AI operations.
#[test]
fn cmd_show_ops_ai_category() {
    let result = cmd_show_ops(Some("ai".to_string()), None, false, false);
    assert!(result.is_ok());
}

