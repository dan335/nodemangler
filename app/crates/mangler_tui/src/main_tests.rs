use super::*;

// ── Helper: create a temp graph file and return its path ─────────────────

/// Create a temporary graph file and return its path. The caller is
/// responsible for cleanup via `std::fs::remove_file`.
fn temp_graph_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mangle_test_{}_{}.mangle.json",
        label,
        std::process::id()
    ))
}

/// Create a temp file with an empty graph and return its path.
fn create_temp_graph(label: &str) -> PathBuf {
    let path = temp_graph_path(label);
    let _ = std::fs::remove_file(&path);
    cmd_new(path.clone()).unwrap();
    path
}

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

/// A leading colon is valid syntax: empty node ID with an explicit index.
#[test]
fn parse_slot_leading_colon_empty_node_id() {
    let (node, idx) = parse_slot(":0").unwrap();
    assert_eq!(node, "");
    assert_eq!(idx, 0);
}

/// Trailing colon leaves an empty index string — not a valid usize.
#[test]
fn parse_slot_trailing_colon_returns_err() {
    assert!(parse_slot("node:").is_err());
}

/// usize cannot represent a negative value.
#[test]
fn parse_slot_negative_index_returns_err() {
    assert!(parse_slot("node:-1").is_err());
}

/// A number too large to fit in usize must be rejected.
#[test]
fn parse_slot_overflow_index_returns_err() {
    assert!(parse_slot("node:99999999999999999999").is_err());
}

/// A bare `:` splits into node="" and index="" — the empty index fails to parse.
#[test]
fn parse_slot_only_colon_returns_err() {
    assert!(parse_slot(":").is_err());
}

// ── flatten_ops ───────────────────────────────────────────────────────────

#[test]
fn flatten_ops_returns_non_empty() {
    let all = flatten_ops(&operation_list(), "");
    assert!(!all.is_empty());
}

#[test]
fn flatten_ops_paths_contain_slash() {
    for (path, _) in flatten_ops(&operation_list(), "") {
        assert!(path.contains('/'), "expected '/' in path: {path}");
    }
}

#[test]
fn flatten_ops_prefix_prepended() {
    let all = flatten_ops(&operation_list(), "");
    let numbers: Vec<_> = all.iter().filter(|(p, _)| p.starts_with("numbers/")).collect();
    assert!(!numbers.is_empty(), "expected at least one numbers/* operation");
}

#[test]
fn flatten_ops_custom_prefix() {
    let all = flatten_ops(&operation_list(), "root");
    for (path, _) in &all {
        assert!(path.starts_with("root/"), "expected 'root/' prefix in: {path}");
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

/// An empty input slice produces an empty output.
#[test]
fn flatten_ops_empty_slice() {
    assert!(flatten_ops(&[], "").is_empty());
}

/// `Subgraph` items are silently skipped and never appear in the output.
#[test]
fn flatten_ops_subgraph_items_are_skipped() {
    let items = vec![
        OperationListItem::Subgraph,
        OperationListItem::Subgraph,
    ];
    assert!(flatten_ops(&items, "").is_empty());
}

/// A category containing no operations contributes nothing to the flat list.
#[test]
fn flatten_ops_empty_category_contributes_nothing() {
    let items = vec![OperationListItem::Category {
        name: "empty".to_string(),
        operation_list_items: vec![],
    }];
    assert!(flatten_ops(&items, "").is_empty());
}

/// A deeply nested category still produces the correct slash-joined path.
#[test]
fn flatten_ops_deep_nesting_builds_correct_path() {
    let items = vec![OperationListItem::Category {
        name: "a".to_string(),
        operation_list_items: vec![OperationListItem::Category {
            name: "b".to_string(),
            operation_list_items: vec![OperationListItem::Operation {
                operation: Operation::OpNumberMathAdd,
            }],
        }],
    }];
    let flat = flatten_ops(&items, "");
    assert_eq!(flat.len(), 1);
    assert_eq!(flat[0].0, "a/b/add");
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
    let by_name = resolve_op("OpNumberMathAdd").unwrap();
    assert_eq!(
        serde_json::to_string(&by_path).unwrap(),
        serde_json::to_string(&by_name).unwrap(),
    );
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
    assert!(resolve_op("numbers").is_err());
}

#[test]
fn resolve_op_other_categories_resolve() {
    assert!(resolve_op("logic/comparison/equal").is_ok());
    assert!(resolve_op("colors/blend/blend").is_ok());
}

#[test]
fn resolve_op_short_path_and_variant_are_equivalent() {
    let by_path = resolve_op("numbers/arithmetic/add").unwrap();
    let by_name = resolve_op("OpNumberMathAdd").unwrap();
    assert_eq!(
        serde_json::to_string(&by_path).unwrap(),
        serde_json::to_string(&by_name).unwrap(),
    );
}

// ── display_value ─────────────────────────────────────────────────────────

#[test]
fn display_value_bool_true() {
    assert!(display_value(&Value::Bool(true)).contains("true"));
}

#[test]
fn display_value_bool_false() {
    assert!(display_value(&Value::Bool(false)).contains("false"));
}

#[test]
fn display_value_integer() {
    let s = display_value(&Value::Integer(42));
    assert!(s.contains("42"), "unexpected: {s}");
}

#[test]
fn display_value_decimal() {
    let s = display_value(&Value::Decimal(1.5));
    assert!(s.contains("1.5") || s.contains("Decimal"), "unexpected: {s}");
}

#[test]
fn display_value_text() {
    let s = display_value(&Value::Text("hello".to_string()));
    assert!(s.contains("hello"), "unexpected: {s}");
}

#[test]
fn display_value_trigger() {
    let s = display_value(&Value::Trigger);
    assert!(s.contains("Trigger"), "unexpected: {s}");
}

#[test]
fn display_value_empty_text() {
    let s = display_value(&Value::Text(String::new()));
    assert!(s.contains("Text") || s.contains("\"\""), "unexpected: {s}");
}

#[test]
fn display_value_path() {
    let s = display_value(&Value::Path(PathBuf::from("/some/file.png")));
    assert!(s.contains("file.png") || s.contains("Path"), "unexpected: {s}");
}

#[test]
fn display_value_integer_min() {
    let s = display_value(&Value::Integer(i32::MIN));
    assert!(s.contains("-2147483648"), "unexpected: {s}");
}

#[test]
fn display_value_integer_max() {
    let s = display_value(&Value::Integer(i32::MAX));
    assert!(s.contains("2147483647"), "unexpected: {s}");
}

#[test]
fn display_value_negative_integer() {
    let s = display_value(&Value::Integer(-99));
    assert!(s.contains("-99"), "unexpected: {s}");
}

#[test]
fn display_value_zero_decimal() {
    let s = display_value(&Value::Decimal(0.0));
    assert!(s.contains("0") && s.contains("Decimal"), "unexpected: {s}");
}

// ── cmd_new ───────────────────────────────────────────────────────────────

#[test]
fn cmd_new_creates_valid_graph_file() {
    let path = temp_graph_path("new");
    let _ = std::fs::remove_file(&path);
    assert!(cmd_new(path.clone()).is_ok());
    assert!(path.exists());
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("\"nodes\""), "missing 'nodes' key");
    assert!(contents.contains("\"name\""), "missing 'name' key");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_uses_stem_as_graph_name() {
    let path = temp_graph_path("stem");
    let _ = std::fs::remove_file(&path);
    cmd_new(path.clone()).unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    let stem = path.file_stem().unwrap().to_str().unwrap();
    assert!(contents.contains(stem), "expected stem '{stem}' in: {contents}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_fails_if_file_already_exists() {
    let path = temp_graph_path("exists");
    std::fs::write(&path, "{}").unwrap();
    let result = cmd_new(path.clone());
    assert!(result.is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_appends_mangle_json_when_no_json_extension() {
    let base = std::env::temp_dir().join(format!(
        "mangle_test_nosuffix_{}",
        std::process::id()
    ));
    let expected = std::env::temp_dir().join(format!(
        "mangle_test_nosuffix_{}.mangle.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&expected);
    assert!(cmd_new(base).is_ok());
    assert!(expected.exists(), "file should be created with .mangle.json suffix");
    let _ = std::fs::remove_file(&expected);
}

#[test]
fn cmd_new_keeps_json_extension_unchanged() {
    let path = std::env::temp_dir().join(format!(
        "mangle_test_keepjson_{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(cmd_new(path.clone()).is_ok());
    assert!(path.exists(), "file should be created at the exact .json path");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_fails_in_nonexistent_directory() {
    let path = std::env::temp_dir().join("mangle_no_such_dir_xyz").join("graph.mangle.json");
    assert!(cmd_new(path).is_err());
}

// ── load_graph ────────────────────────────────────────────────────────────

#[test]
fn load_graph_missing_file_returns_err() {
    let path = PathBuf::from("/nonexistent/path/does/not/exist_mangle.json");
    assert!(load_graph(&path).is_err());
}

#[test]
fn load_graph_invalid_json_returns_err() {
    let path = temp_graph_path("badjson");
    std::fs::write(&path, "this is not json at all").unwrap();
    let result = load_graph(&path);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn load_graph_empty_object_returns_err() {
    let path = temp_graph_path("emptyobj");
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
    let graph = load_graph(&path).unwrap();
    let original_id = graph.id.clone();
    let original_name = graph.name.clone();
    save_graph(&graph, &path).unwrap();
    let reloaded = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(reloaded.id, original_id);
    assert_eq!(reloaded.name, original_name);
}

// ── cmd_set_input ─────────────────────────────────────────────────────────

#[test]
fn cmd_set_input_invalid_json_returns_err() {
    let path = create_temp_graph("setinput_badjson");
    let result = cmd_set_input(path.clone(), "any".to_string(), 0, "not json".to_string());
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn cmd_set_input_unknown_node_returns_err() {
    let path = create_temp_graph("setinput_nonode");
    let result = cmd_set_input(path.clone(), "ghost-node".to_string(), 0, r#"{"Integer":42}"#.to_string());
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

// ── async integration tests ───────────────────────────────────────────────

#[tokio::test]
async fn cmd_add_node_persists_to_file() {
    let path = create_temp_graph("addnode");
    let node_id = format!("test-node-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone())).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes.contains_key(&node_id));
}

#[tokio::test]
async fn cmd_add_then_remove_node_leaves_graph_empty() {
    let path = create_temp_graph("addremove");
    let node_id = format!("addremove-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone())).await.unwrap();
    cmd_remove_node(path.clone(), node_id.clone()).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!graph.nodes.contains_key(&node_id));
}

#[tokio::test]
async fn cmd_remove_node_unknown_returns_err() {
    let path = create_temp_graph("removemissing");
    let result = cmd_remove_node(path.clone(), "ghost".to_string()).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[tokio::test]
async fn cmd_set_input_on_real_node_succeeds() {
    let path = create_temp_graph("setinput_valid");
    let node_id = format!("add-node-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone())).await.unwrap();
    let result = cmd_set_input(path.clone(), node_id.clone(), 0, r#"{"Decimal":7.0}"#.to_string());
    assert!(result.is_ok());
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let stored = &graph.nodes[&node_id].inputs[0].value;
    assert!(matches!(stored, Value::Decimal(v) if (*v - 7.0).abs() < 1e-6), "unexpected: {:?}", stored);
}

#[tokio::test]
async fn cmd_connect_stores_connection_on_consumer() {
    let path = create_temp_graph("connect");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("producer".to_string())).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("consumer".to_string())).await.unwrap();
    cmd_connect(path.clone(), "producer:0".to_string(), "consumer:0".to_string()).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes["consumer"].inputs[0].connection, Some(("producer".to_string(), 0)));
}

#[tokio::test]
async fn cmd_disconnect_removes_connection() {
    let path = create_temp_graph("disconnect");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("src".to_string())).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("dst".to_string())).await.unwrap();
    cmd_connect(path.clone(), "src:0".to_string(), "dst:0".to_string()).await.unwrap();
    cmd_disconnect(path.clone(), "dst".to_string(), 0).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes["dst"].inputs[0].connection, None);
}

#[tokio::test]
async fn cmd_disconnect_unknown_node_returns_err() {
    let path = create_temp_graph("disc_nonode");
    let result = cmd_disconnect(path.clone(), "ghost".to_string(), 0).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[tokio::test]
async fn cmd_add_node_auto_id_is_unique_across_calls() {
    let path = create_temp_graph("autoid");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), None).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), None).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes.len(), 2, "expected exactly 2 distinct nodes");
}

// ── do_* function unit tests ────────────────────────────────────────────

#[tokio::test]
async fn do_add_node_invalid_op_returns_err() {
    let path = create_temp_graph("do_addnode_bad");
    let mut graph = load_graph(&path).unwrap();
    assert!(do_add_node(&mut graph, "not/a/real/op", None).await.is_err());
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

// ── show-ops ────────────────────────────────────────────────────────

#[test]
fn show_ops_group_fallback_shows_categories() {
    let text = format_show_ops_human(Some("zzz_bad_group"), None);
    assert!(text.contains("Available categories"));
    assert!(text.contains("numbers"));
    assert!(text.contains("images"));
}

#[test]
fn show_ops_group_valid_prefix() {
    let text = format_show_ops_human(Some("numbers/arithmetic"), None);
    assert!(text.contains("numbers/arithmetic/add"));
}

#[test]
fn show_ops_search() {
    let text = format_show_ops_human(None, Some("blur"));
    assert!(text.contains("blur"));
}

// ── show-types ──────────────────────────────────────────────────────

#[test]
fn show_types_human_all() {
    let text = format_show_types_human(None);
    assert!(text.contains("BlendMode") && text.contains("ColorSpace"));
}

#[test]
fn show_types_human_specific() {
    let text = format_show_types_human(Some("BlendMode"));
    assert!(text.contains("Multiply") && text.contains("Screen"));
}

// ── show-values ─────────────────────────────────────────────────────

#[test]
fn show_values_text_contains_examples() {
    let text = show_values_text();
    assert!(!text.is_empty());
    assert!(text.contains("Bool"));
    assert!(text.contains("Color"));
    assert!(text.contains("Decimal"));
}

// ── show-op ─────────────────────────────────────────────────────────

#[test]
fn show_op_human_contains_description() {
    let text = format_show_op_human("numbers/arithmetic/add").unwrap();
    assert!(text.contains("Inputs:"));
    assert!(text.contains("Outputs:"));
}

#[test]
fn show_op_unknown_returns_err() {
    assert!(format_show_op_human("not/real").is_err());
}

// ── enum_variants helper tests ──────────────────────────────────────

#[test]
fn enum_variants_all_types_resolve() {
    for name in ENUM_TYPE_NAMES { assert!(enum_variants(name).is_some(), "enum_variants({}) returned None", name); }
}

#[test]
fn enum_variants_unknown_returns_none() { assert!(enum_variants("NotAType").is_none()); }

#[test]
fn value_type_enum_name_mappings() {
    assert_eq!(value_type_enum_name(&ValueType::BlendMode), Some("BlendMode"));
    assert_eq!(value_type_enum_name(&ValueType::ColorSpace), Some("ColorSpace"));
    assert_eq!(value_type_enum_name(&ValueType::Decimal), None);
    assert_eq!(value_type_enum_name(&ValueType::Bool), None);
}

// ── collect_categories ──────────────────────────────────────────────

#[test]
fn collect_categories_returns_expected() {
    let all_ops = flatten_ops(&operation_list(), "");
    let cats = collect_categories(&all_ops);
    assert!(!cats.is_empty());
    let names: Vec<&str> = cats.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"numbers") && names.contains(&"images"));
}

#[test]
fn collect_categories_empty_input() { assert!(collect_categories(&[]).is_empty()); }

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
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("oob1".to_string())).await.unwrap();
    let result = do_set_input(&mut graph, "oob1", 999, r#"{"Integer":1}"#);
    let _ = std::fs::remove_file(&path);
    assert!(result.unwrap_err().contains("out of range"));
}

#[tokio::test]
async fn set_input_enum_error_includes_valid_values() {
    let path = create_temp_graph("setinput_enum");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "colors/blend/blend", Some("blend1".to_string())).await.unwrap();
    let blend_node = &graph.nodes["blend1"];
    let blend_idx = blend_node.inputs.iter().position(|i| matches!(i.value.value_type(), ValueType::BlendMode));
    if let Some(idx) = blend_idx {
        let err = do_set_input(&mut graph, "blend1", idx, "InvalidValue").unwrap_err();
        assert!(err.contains("BlendMode") && err.contains("Multiply"));
    }
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_missing_source_node_error() {
    let path = create_temp_graph("conn_src_miss");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("dst".to_string())).await.unwrap();
    assert!(do_connect(&mut graph, "ghost:0", "dst:0").await.unwrap_err().contains("source node"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_missing_dest_node_error() {
    let path = create_temp_graph("conn_dst_miss");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("src".to_string())).await.unwrap();
    assert!(do_connect(&mut graph, "src:0", "ghost:0").await.unwrap_err().contains("destination node"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_output_out_of_bounds_error() {
    let path = create_temp_graph("conn_out_oob");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("a".to_string())).await.unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("b".to_string())).await.unwrap();
    let err = do_connect(&mut graph, "a:999", "b:0").await.unwrap_err();
    assert!(err.contains("output index") && err.contains("out of range"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_input_out_of_bounds_error() {
    let path = create_temp_graph("conn_in_oob");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("a".to_string())).await.unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("b".to_string())).await.unwrap();
    let err = do_connect(&mut graph, "a:0", "b:999").await.unwrap_err();
    assert!(err.contains("input index") && err.contains("out of range"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn connect_valid_still_works() {
    let path = create_temp_graph("conn_valid");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("v1".to_string())).await.unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("v2".to_string())).await.unwrap();
    assert!(do_connect(&mut graph, "v1:0", "v2:0").await.is_ok());
    let _ = std::fs::remove_file(&path);
}
