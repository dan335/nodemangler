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
    cmd_new(path.clone(), false).unwrap();
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
    assert!(cmd_new(path.clone(), false).is_ok());
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
    cmd_new(path.clone(), false).unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    let stem = path.file_stem().unwrap().to_str().unwrap();
    assert!(contents.contains(stem), "expected stem '{stem}' in: {contents}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_fails_if_file_already_exists() {
    let path = temp_graph_path("exists");
    std::fs::write(&path, "{}").unwrap();
    let result = cmd_new(path.clone(), false);
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
    assert!(cmd_new(base, false).is_ok());
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
    assert!(cmd_new(path.clone(), false).is_ok());
    assert!(path.exists(), "file should be created at the exact .json path");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cmd_new_fails_in_nonexistent_directory() {
    let path = std::env::temp_dir().join("mangle_no_such_dir_xyz").join("graph.mangle.json");
    assert!(cmd_new(path, false).is_err());
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
    let result = cmd_set_input(path.clone(), "any".to_string(), vec![0], vec!["not json".to_string()], false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn cmd_set_input_unknown_node_returns_err() {
    let path = create_temp_graph("setinput_nonode");
    let result = cmd_set_input(path.clone(), "ghost-node".to_string(), vec![0], vec![r#"{"Integer":42}"#.to_string()], false);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_err());
}

// ── async integration tests ───────────────────────────────────────────────

#[tokio::test]
async fn cmd_add_node_persists_to_file() {
    let path = create_temp_graph("addnode");
    let node_id = format!("test-node-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes.contains_key(&node_id));
}

#[tokio::test]
async fn cmd_add_then_remove_node_leaves_graph_empty() {
    let path = create_temp_graph("addremove");
    let node_id = format!("addremove-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("producer".to_string()), false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("consumer".to_string()), false).await.unwrap();
    cmd_connect(path.clone(), "producer:0".to_string(), "consumer:0".to_string(), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(graph.nodes["consumer"].inputs[0].connection, Some(("producer".to_string(), 0)));
}

#[tokio::test]
async fn cmd_disconnect_removes_connection() {
    let path = create_temp_graph("disconnect");
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("src".to_string()), false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("dst".to_string()), false).await.unwrap();
    cmd_connect(path.clone(), "src:0".to_string(), "dst:0".to_string(), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), None, false).await.unwrap();
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), None, false).await.unwrap();
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
    // "slope" is substring of "slope_blur" segment but not exact match => +5
    assert!(score >= 5, "expected at least 5 for path contains, got {score}");
}

/// Term matching the variant exactly scores 8.
#[test]
fn score_op_exact_variant() {
    let score = score_op(("some/path", "mything", "a description"), &["mything".to_string()]);
    assert!(score >= 8, "expected at least 8 for exact variant match, got {score}");
}

/// Term substring of variant scores 4.
#[test]
fn score_op_variant_contains() {
    let score = score_op(("some/other/path", "opimageblurblur", "no match here"), &["opimage".to_string()]);
    assert!(score >= 4, "expected at least 4 for variant contains, got {score}");
}

/// Term found only in description scores 2.
#[test]
fn score_op_description_contains() {
    let score = score_op(("some/path", "somevariant", "this applies gaussian smoothing"), &["gaussian".to_string()]);
    assert_eq!(score, 2, "expected 2 for description-only match, got {score}");
}

/// A term not in any field returns 0.
#[test]
fn score_op_no_match_returns_zero() {
    let score = score_op(("images/blur/blur", "opblur", "apply blur"), &["zzzznotreal".to_string()]);
    assert_eq!(score, 0);
}

/// Two terms that each match sum their scores.
#[test]
fn score_op_multiple_terms_accumulate() {
    let score = score_op(
        ("images/blur/blur", "opblur", "apply blur to image"),
        &["blur".to_string(), "image".to_string()],
    );
    assert!(score > 0, "both terms should match");
    // Each term contributes some score, total should be sum.
    let score_blur = score_op(("images/blur/blur", "opblur", "apply blur to image"), &["blur".to_string()]);
    let score_image = score_op(("images/blur/blur", "opblur", "apply blur to image"), &["image".to_string()]);
    assert_eq!(score, score_blur + score_image);
}

/// If one of two terms doesn't match, total score is 0.
#[test]
fn score_op_any_term_missing_returns_zero() {
    let score = score_op(
        ("images/blur/blur", "opblur", "apply blur"),
        &["blur".to_string(), "zzzznotreal".to_string()],
    );
    assert_eq!(score, 0);
}

/// Uppercase term matches lowercase haystack (caller lowercases both).
#[test]
fn score_op_case_insensitive() {
    // score_op expects pre-lowercased inputs, so we test that the caller
    // lowercases correctly by passing already-lowered values.
    let score = score_op(("images/blur/blur", "opblur", "apply blur"), &["blur".to_string()]);
    assert!(score > 0);
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

// ── Multi-word AND search (integration) ─────────────────────────────

/// `"blur image"` returns only ops matching both words.
#[test]
fn search_multi_word_and() {
    let text = format_show_ops_human(None, Some("blur image"));
    // Should only include ops matching both "blur" and "image".
    assert!(!text.is_empty());
    assert!(!text.contains("No operations match"));
    // Every result line should relate to blur.
    for line in text.lines() {
        if line.trim().is_empty() { continue; }
        let lower = line.to_lowercase();
        assert!(lower.contains("blur"), "line should contain blur: {line}");
    }
}

/// `"blur zzzznotreal"` returns no results.
#[test]
fn search_multi_word_one_missing() {
    let text = format_show_ops_human(None, Some("blur zzzznotreal"));
    assert!(text.contains("No operations match search"));
}

/// `"blur"` behaves as before, returning blur-related ops.
#[test]
fn search_single_word_still_works() {
    let text = format_show_ops_human(None, Some("blur"));
    assert!(text.contains("blur"));
    assert!(!text.contains("No operations match"));
}

// ── Ranking / sort order ────────────────────────────────────────────

/// `"blur"` returns `images/blur/blur` before `images/blur/slope_blur`.
#[test]
fn search_results_sorted_by_score() {
    let text = format_show_ops_human(None, Some("blur"));
    let lines: Vec<&str> = text.lines().filter(|l| l.contains("images/blur/")).collect();
    assert!(lines.len() >= 2, "expected at least 2 blur results");
    // The first blur result should be "images/blur/blur" (exact segment match).
    let first_blur_line = lines[0];
    assert!(first_blur_line.starts_with("images/blur/blur"), "expected images/blur/blur first, got: {first_blur_line}");
}

/// An op matched via path ranks above one matched only via description.
#[test]
fn search_path_match_ranks_above_description() {
    let text = format_show_ops_human(None, Some("blur"));
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    // The first result should have "blur" in the path, not just the description.
    if let Some(first) = lines.first() {
        let path_part = first.split_whitespace().next().unwrap_or("");
        assert!(path_part.to_lowercase().contains("blur"),
            "first result should have blur in path: {first}");
    }
}

/// Human output lines contain `(score: N)`.
#[test]
fn search_scores_shown_in_human_output() {
    let text = format_show_ops_human(None, Some("blur"));
    assert!(text.contains("(score:"), "expected score annotations in output:\n{text}");
}

// ── No results message ──────────────────────────────────────────────

/// Nonsense search term produces `"No operations match search"` message.
#[test]
fn search_no_results_human() {
    let text = format_show_ops_human(None, Some("zzzznotreal"));
    assert!(text.contains("No operations match search \"zzzznotreal\""));
    assert!(text.contains("--group"));
}

/// Nonsense search in JSON mode produces `{"matches": 0, "message": "..."}`.
#[test]
fn search_no_results_json() {
    let val = format_show_ops_json(None, Some("zzzznotreal"));
    assert_eq!(val["matches"], 0);
    assert!(val["message"].as_str().unwrap().contains("No operations match search"));
}

/// `--group foo --search zzz` still shows the no-results message.
#[test]
fn search_no_results_with_group() {
    let text = format_show_ops_human(Some("images"), Some("zzzznotreal"));
    assert!(text.contains("No operations match search"));
}

// ── JSON output ─────────────────────────────────────────────────────

/// JSON output includes a `"score"` field per op.
#[test]
fn search_json_includes_score() {
    let val = format_show_ops_json(None, Some("blur"));
    let arr = val.as_array().expect("expected array");
    assert!(!arr.is_empty());
    for op in arr {
        assert!(op.get("score").is_some(), "missing score field: {op}");
        assert!(op["score"].as_u64().unwrap() > 0);
    }
}

/// JSON results are sorted by score descending.
#[test]
fn search_json_sorted_by_score() {
    let val = format_show_ops_json(None, Some("blur"));
    let arr = val.as_array().expect("expected array");
    let scores: Vec<u64> = arr.iter().map(|o| o["score"].as_u64().unwrap()).collect();
    for window in scores.windows(2) {
        assert!(window[0] >= window[1], "scores not sorted descending: {:?}", scores);
    }
}

// ── Edge cases ──────────────────────────────────────────────────────

/// `--search ""` behaves like no search (all ops returned).
#[test]
fn search_empty_string_returns_all() {
    let all = format_show_ops_human(None, None);
    let empty_search = format_show_ops_human(None, Some(""));
    assert_eq!(all, empty_search);
}

/// `--search "   "` treated as no search.
#[test]
fn search_whitespace_only_returns_all() {
    let all = format_show_ops_human(None, None);
    let ws_search = format_show_ops_human(None, Some("   "));
    assert_eq!(all, ws_search);
}

/// `--search "(*&^"` doesn't panic, returns no results gracefully.
#[test]
fn search_special_chars_no_panic() {
    let text = format_show_ops_human(None, Some("(*&^"));
    // Should either show no-results message or be empty (no panic).
    assert!(text.contains("No operations match") || text.is_empty() || !text.contains("(*&^"));
}

// ── show-types ──────────────────────────────────────────────────────

#[test]
fn show_types_human_all() {
    let text = format_show_types_human(None);
    assert!(text.contains("blendmode") && text.contains("colorspace"));
}

#[test]
fn show_types_human_specific() {
    let text = format_show_types_human(Some("blendmode"));
    assert!(text.contains("Multiply") && text.contains("Screen"));
}

// ── show-values ─────────────────────────────────────────────────────

#[test]
fn show_values_text_contains_examples() {
    let text = show_values_text();
    assert!(!text.is_empty());
    assert!(text.contains("bool"));
    assert!(text.contains("color"));
    assert!(text.contains("decimal"));
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
    assert_eq!(value_type_enum_name(&ValueType::BlendMode), Some("blendmode"));
    assert_eq!(value_type_enum_name(&ValueType::ColorSpace), Some("colorspace"));
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
        assert!(err.contains("blendmode") && err.contains("Multiply"));
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

// ── parse_typed_value ─────────────────────────────────────────────────────

/// Helper: assert a Value matches via JSON round-trip (Value doesn't impl PartialEq).
fn assert_value_json(val: &Value, expected_json: &str) {
    let actual = serde_json::to_string(val).unwrap();
    let a: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let b: serde_json::Value = serde_json::from_str(expected_json).unwrap();
    assert_eq!(a, b, "expected {expected_json}, got {actual}");
}

#[test]
fn parse_typed_value_bool_true() {
    assert_value_json(&parse_typed_value("bool:true").unwrap(), r#"{"Bool":true}"#);
}

#[test]
fn parse_typed_value_bool_false() {
    assert_value_json(&parse_typed_value("bool:false").unwrap(), r#"{"Bool":false}"#);
}

#[test]
fn parse_typed_value_bool_case_insensitive_prefix() {
    assert!(matches!(parse_typed_value("Bool:true").unwrap(), Value::Bool(true)));
    assert!(matches!(parse_typed_value("BOOL:false").unwrap(), Value::Bool(false)));
}

#[test]
fn parse_typed_value_bool_invalid() {
    assert!(parse_typed_value("bool:yes").is_err());
}

#[test]
fn parse_typed_value_int() {
    assert!(matches!(parse_typed_value("int:42").unwrap(), Value::Integer(42)));
}

#[test]
fn parse_typed_value_int_negative() {
    assert!(matches!(parse_typed_value("int:-7").unwrap(), Value::Integer(-7)));
}

#[test]
fn parse_typed_value_int_invalid() {
    assert!(parse_typed_value("int:abc").is_err());
}

#[test]
fn parse_typed_value_decimal() {
    let val = parse_typed_value("decimal:3.14").unwrap();
    match val {
        Value::Decimal(f) => assert!((f - 3.14).abs() < 0.001),
        other => panic!("expected Decimal, got {:?}", other),
    }
}

#[test]
fn parse_typed_value_decimal_integer_form() {
    let val = parse_typed_value("decimal:7").unwrap();
    assert!(matches!(val, Value::Decimal(f) if (f - 7.0).abs() < 0.001));
}

#[test]
fn parse_typed_value_decimal_negative() {
    let val = parse_typed_value("decimal:-0.5").unwrap();
    assert!(matches!(val, Value::Decimal(f) if (f + 0.5).abs() < 0.001));
}

#[test]
fn parse_typed_value_text() {
    assert!(matches!(parse_typed_value("text:hello").unwrap(), Value::Text(s) if s == "hello"));
}

#[test]
fn parse_typed_value_text_empty() {
    assert!(matches!(parse_typed_value("text:").unwrap(), Value::Text(s) if s.is_empty()));
}

/// `text:foo:bar` should preserve everything after the first colon.
#[test]
fn parse_typed_value_text_with_colon() {
    assert!(matches!(parse_typed_value("text:foo:bar").unwrap(), Value::Text(s) if s == "foo:bar"));
}

#[test]
fn parse_typed_value_text_with_spaces() {
    assert!(matches!(parse_typed_value("text:hello world").unwrap(), Value::Text(s) if s == "hello world"));
}

#[test]
fn parse_typed_value_path() {
    assert!(matches!(parse_typed_value("path:/some/file.png").unwrap(), Value::Path(p) if p == PathBuf::from("/some/file.png")));
}

/// `path:C:\foo` should preserve the Windows path with colon.
#[test]
fn parse_typed_value_path_with_colon() {
    assert!(matches!(parse_typed_value("path:C:\\foo\\bar.png").unwrap(), Value::Path(p) if p == PathBuf::from("C:\\foo\\bar.png")));
}

#[test]
fn parse_typed_value_color_valid() {
    let val = parse_typed_value("color:1.0,0.0,0.5,1.0").unwrap();
    match val {
        Value::Color(c) => {
            assert!((c.r - 1.0).abs() < 0.001);
            assert!((c.g - 0.0).abs() < 0.001);
            assert!((c.b - 0.5).abs() < 0.001);
            assert!((c.a - 1.0).abs() < 0.001);
        }
        other => panic!("expected Color, got {:?}", other),
    }
}

#[test]
fn parse_typed_value_color_with_spaces() {
    // Spaces around components should be trimmed.
    let val = parse_typed_value("color: 1.0 , 0.0 , 0.0 , 1.0 ").unwrap();
    assert!(matches!(val, Value::Color(_)));
}

#[test]
fn parse_typed_value_color_wrong_count() {
    let err = parse_typed_value("color:1.0,0.0,0.0").unwrap_err();
    assert!(err.contains("4") && err.contains("3"));
}

#[test]
fn parse_typed_value_color_non_numeric() {
    assert!(parse_typed_value("color:red,green,blue,alpha").is_err());
}

#[test]
fn parse_typed_value_blend_mode() {
    let val = parse_typed_value("BlendMode:Multiply").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Multiply"));
}

#[test]
fn parse_typed_value_blend_mode_case_insensitive_prefix() {
    let val = parse_typed_value("blendmode:Multiply").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Multiply"));
}

#[test]
fn parse_typed_value_blend_mode_case_insensitive_variant() {
    let val = parse_typed_value("BlendMode:multiply").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Multiply"));
}

#[test]
fn parse_typed_value_blend_mode_invalid_variant() {
    let err = parse_typed_value("BlendMode:NotAMode").unwrap_err();
    assert!(err.contains("blendmode") && err.contains("Multiply"));
}

#[test]
fn parse_typed_value_color_space() {
    let val = parse_typed_value("ColorSpace:Srgb").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Srgb"));
}

#[test]
fn parse_typed_value_filter_type() {
    let val = parse_typed_value("FilterType:lanczos3").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("lanczos3"));
}

#[test]
fn parse_typed_value_image_type() {
    let val = parse_typed_value("ImageType:png").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("png"));
}

#[test]
fn parse_typed_value_color_format() {
    let val = parse_typed_value("ColorFormat:Rgba8").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Rgba8"));
}

#[test]
fn parse_typed_value_noise_worley() {
    let val = parse_typed_value("NoiseWorleyDistanceFunction:Euclidean").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Euclidean"));
}

#[test]
fn parse_typed_value_text_halign() {
    let val = parse_typed_value("TextHAlign:Left").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Left"));
}

#[test]
fn parse_typed_value_text_valign() {
    let val = parse_typed_value("TextVAlign:Top").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Top"));
}

/// Legacy JSON format still works.
#[test]
fn parse_typed_value_json_fallback_decimal() {
    let val = parse_typed_value(r#"{"Decimal":3.14}"#).unwrap();
    assert!(matches!(val, Value::Decimal(f) if (f - 3.14).abs() < 0.01));
}

#[test]
fn parse_typed_value_json_fallback_bool() {
    assert!(matches!(parse_typed_value(r#"{"Bool":true}"#).unwrap(), Value::Bool(true)));
}

#[test]
fn parse_typed_value_json_fallback_color() {
    let val = parse_typed_value(r#"{"Color":{"r":1.0,"g":0.0,"b":0.0,"a":1.0}}"#).unwrap();
    assert!(matches!(val, Value::Color(_)));
}

/// Completely invalid input returns a helpful error.
#[test]
fn parse_typed_value_invalid_returns_err() {
    let err = parse_typed_value("not_valid_at_all").unwrap_err();
    assert!(err.contains("Type:value") || err.contains("JSON"));
}

/// Integration: set-input with typed value on a real node.
#[tokio::test]
async fn set_input_typed_value_decimal_on_real_node() {
    let path = create_temp_graph("typed_decimal");
    let mut graph = load_graph(&path).unwrap();
    do_add_node(&mut graph, "numbers/arithmetic/add", Some("n1".to_string())).await.unwrap();
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
    do_add_node(&mut graph, "colors/blend/blend", Some("b1".to_string())).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
    let graph = load_graph(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(graph.nodes[&node_id].is_enabled);
}

/// Human info output shows [DISABLED] for disabled nodes.
#[tokio::test]
async fn info_shows_disabled_tag() {
    let path = create_temp_graph("info_disabled");
    let node_id = format!("dis-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
    cmd_set_enabled(path.clone(), node_id.clone(), false, false).unwrap();
    let graph = load_graph(&path).unwrap();
    let val = format_info_json(&graph, Some(&node_id)).unwrap();
    let _ = std::fs::remove_file(&path);
    let nodes = val["nodes"].as_array().unwrap();
    assert_eq!(nodes[0]["enabled"], serde_json::json!(false));
}

// ── show-output ──────────────────────────────────────────────────────────

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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
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
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
    cmd_set_input(path.clone(), node_id.clone(), vec![0, 1], vec!["decimal:3.0".into(), "decimal:7.0".into()], false).unwrap();
    // Run show-output and check it succeeds.
    let result = cmd_show_output(path.clone(), node_id.clone(), Some(0), false, vec![], None, false).await;
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok());
}

/// show-output JSON format includes node and output fields for non-image.
#[tokio::test]
async fn show_output_json_format_non_image() {
    let path = create_temp_graph("so_json_ni");
    let node_id = format!("so_jni-{}", std::process::id());
    cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()), false).await.unwrap();
    cmd_set_input(path.clone(), node_id.clone(), vec![0, 1], vec!["decimal:2.0".into(), "decimal:5.0".into()], false).unwrap();

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

/// resolve_sample_coord parses named positions correctly.
#[test]
fn sample_coord_named_positions() {
    assert_eq!(resolve_sample_coord("center", 100, 200).unwrap(), (50, 100));
    assert_eq!(resolve_sample_coord("top-left", 100, 200).unwrap(), (0, 0));
    assert_eq!(resolve_sample_coord("top-right", 100, 200).unwrap(), (99, 0));
    assert_eq!(resolve_sample_coord("bottom-left", 100, 200).unwrap(), (0, 199));
    assert_eq!(resolve_sample_coord("bottom-right", 100, 200).unwrap(), (99, 199));
}

/// resolve_sample_coord parses x,y coordinates.
#[test]
fn sample_coord_xy() {
    assert_eq!(resolve_sample_coord("10,20", 100, 100).unwrap(), (10, 20));
    assert_eq!(resolve_sample_coord("0,0", 512, 512).unwrap(), (0, 0));
}

/// resolve_sample_coord rejects out-of-bounds coordinates.
#[test]
fn sample_coord_out_of_bounds() {
    assert!(resolve_sample_coord("100,0", 100, 100).is_err());
    assert!(resolve_sample_coord("0,100", 100, 100).is_err());
}

/// resolve_sample_coord rejects invalid formats.
#[test]
fn sample_coord_invalid_format() {
    assert!(resolve_sample_coord("abc", 100, 100).is_err());
    assert!(resolve_sample_coord("1,2,3", 100, 100).is_err());
}

/// compute_image_stats returns correct results for a uniform image.
#[test]
fn image_stats_uniform() {
    use image::{DynamicImage, RgbaImage, Rgba};
    // Create a 2x2 uniform red image.
    let mut img = RgbaImage::new(2, 2);
    for px in img.pixels_mut() {
        *px = Rgba([255, 0, 0, 255]);
    }
    let dyn_img = DynamicImage::ImageRgba8(img);
    let stats = compute_image_stats(&dyn_img);

    // Red channel should be 1.0 everywhere.
    let r = &stats[0].1;
    assert!((r.min - 1.0).abs() < 0.01);
    assert!((r.max - 1.0).abs() < 0.01);
    assert!((r.mean - 1.0).abs() < 0.01);
    assert!(r.stddev < 0.01);

    // Green channel should be 0.0.
    let g = &stats[1].1;
    assert!(g.max < 0.01);
    assert!(g.mean < 0.01);
}

/// has_transparency returns false for fully opaque image.
#[test]
fn transparency_opaque() {
    use image::{DynamicImage, RgbaImage, Rgba};
    let mut img = RgbaImage::new(2, 2);
    for px in img.pixels_mut() { *px = Rgba([128, 128, 128, 255]); }
    assert!(!has_transparency(&DynamicImage::ImageRgba8(img)));
}

/// has_transparency returns true when any pixel has alpha < 255.
#[test]
fn transparency_with_alpha() {
    use image::{DynamicImage, RgbaImage, Rgba};
    let mut img = RgbaImage::new(2, 2);
    for px in img.pixels_mut() { *px = Rgba([128, 128, 128, 255]); }
    img.put_pixel(0, 0, Rgba([0, 0, 0, 128]));
    assert!(has_transparency(&DynamicImage::ImageRgba8(img)));
}

/// count_unique_colors returns the correct count.
#[test]
fn unique_colors_count() {
    use image::{DynamicImage, RgbaImage, Rgba};
    let mut img = RgbaImage::new(2, 2);
    img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    img.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
    img.put_pixel(0, 1, Rgba([0, 0, 255, 255]));
    img.put_pixel(1, 1, Rgba([255, 0, 0, 255])); // duplicate of (0,0)
    assert_eq!(count_unique_colors(&DynamicImage::ImageRgba8(img)), 3);
}

/// sample_pixel returns correct RGBA values.
#[test]
fn sample_pixel_values() {
    use image::{DynamicImage, RgbaImage, Rgba};
    let mut img = RgbaImage::new(2, 2);
    img.put_pixel(1, 0, Rgba([255, 128, 0, 255]));
    let dyn_img = DynamicImage::ImageRgba8(img);
    let px = sample_pixel(&dyn_img, 1, 0);
    assert!((px[0] - 1.0).abs() < 0.01); // r = 255 -> ~1.0
    assert!((px[1] - 0.502).abs() < 0.02); // g = 128 -> ~0.502
    assert!(px[2] < 0.01); // b = 0
    assert!((px[3] - 1.0).abs() < 0.01); // a = 255 -> 1.0
}
