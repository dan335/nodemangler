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

/// Collect writer output as a String.
fn output_to_string(buf: &Vec<u8>) -> String {
    String::from_utf8_lossy(buf).to_string()
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

// ── ReplResponse serialization ──────────────────────────────────────────

#[test]
fn repl_response_ok_serialization() {
    let resp = ReplResponse::ok(serde_json::json!({"node_id": "a1"}));
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["data"]["node_id"], "a1");
    assert!(parsed.get("error").is_none());
}

#[test]
fn repl_response_ok_message_serialization() {
    let resp = ReplResponse::ok_message("hello world");
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["data"]["message"], "hello world");
}

#[test]
fn repl_response_error_serialization() {
    let resp = ReplResponse::error("something broke");
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["error"], "something broke");
    assert!(parsed.get("data").is_none());
}

// ── emit ────────────────────────────────────────────────────────────────

#[test]
fn emit_json_mode_produces_valid_json_line() {
    let resp = ReplResponse::ok_message("test");
    let mut buf: Vec<u8> = Vec::new();
    emit(OutputMode::Json, &resp, &mut buf);
    let out = output_to_string(&buf);
    assert_eq!(out.lines().count(), 1);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
}

#[test]
fn emit_human_mode_prints_message_text() {
    let resp = ReplResponse::ok_message("hello human");
    let mut buf: Vec<u8> = Vec::new();
    emit(OutputMode::Human, &resp, &mut buf);
    let out = output_to_string(&buf);
    assert!(out.contains("hello human"));
    assert!(!out.contains("\"status\""));
}

#[test]
fn emit_human_mode_prints_error_prefix() {
    let resp = ReplResponse::error("bad input");
    let mut buf: Vec<u8> = Vec::new();
    emit(OutputMode::Human, &resp, &mut buf);
    let out = output_to_string(&buf);
    assert!(out.contains("error: bad input"));
}

#[test]
fn emit_json_mode_error_is_valid_json() {
    let resp = ReplResponse::error("oops");
    let mut buf: Vec<u8> = Vec::new();
    emit(OutputMode::Json, &resp, &mut buf);
    let out = output_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["error"], "oops");
}

// ── ReplCli parsing ─────────────────────────────────────────────────────

#[test]
fn repl_parse_exit() { assert!(matches!(ReplCli::try_parse_from(["exit"]).unwrap().command, ReplCommand::Exit)); }
#[test]
fn repl_parse_quit() { assert!(matches!(ReplCli::try_parse_from(["quit"]).unwrap().command, ReplCommand::Quit)); }
#[test]
fn repl_parse_save() { assert!(matches!(ReplCli::try_parse_from(["save"]).unwrap().command, ReplCommand::Save)); }
#[test]
fn repl_parse_help() { assert!(matches!(ReplCli::try_parse_from(["help"]).unwrap().command, ReplCommand::Help)); }
#[test]
fn repl_parse_info() { assert!(matches!(ReplCli::try_parse_from(["info"]).unwrap().command, ReplCommand::Info { .. })); }

#[test]
fn repl_parse_add_node_without_no_save() {
    let cli = ReplCli::try_parse_from(["add-node", "--type", "numbers/arithmetic/add"]).unwrap();
    match cli.command {
        ReplCommand::AddNode { op_type, id, no_save } => { assert_eq!(op_type, "numbers/arithmetic/add"); assert!(id.is_none()); assert!(!no_save); }
        _ => panic!("expected AddNode"),
    }
}

#[test]
fn repl_parse_add_node_with_no_save() {
    let cli = ReplCli::try_parse_from(["add-node", "--type", "foo", "--id", "bar", "--no-save"]).unwrap();
    match cli.command {
        ReplCommand::AddNode { op_type, id, no_save } => { assert_eq!(op_type, "foo"); assert_eq!(id, Some("bar".to_string())); assert!(no_save); }
        _ => panic!("expected AddNode"),
    }
}

#[test]
fn repl_parse_set_input() {
    let cli = ReplCli::try_parse_from(["set-input", "--node", "a1", "--input", "0", "--value", r#"{"Decimal":3.14}"#]).unwrap();
    match cli.command {
        ReplCommand::SetInput { node, input, value, no_save } => { assert_eq!(node, "a1"); assert_eq!(input, 0); assert!(value.contains("Decimal")); assert!(!no_save); }
        _ => panic!("expected SetInput"),
    }
}

#[test]
fn repl_parse_run_no_save() {
    match ReplCli::try_parse_from(["run", "--no-save"]).unwrap().command {
        ReplCommand::Run { no_save } => assert!(no_save),
        _ => panic!("expected Run"),
    }
}

#[test]
fn repl_parse_connect() {
    match ReplCli::try_parse_from(["connect", "--from", "a:0", "--to", "b:1"]).unwrap().command {
        ReplCommand::Connect { from, to, no_save } => { assert_eq!(from, "a:0"); assert_eq!(to, "b:1"); assert!(!no_save); }
        _ => panic!("expected Connect"),
    }
}

#[test]
fn repl_parse_disconnect_no_save() {
    match ReplCli::try_parse_from(["disconnect", "--node", "x", "--input", "2", "--no-save"]).unwrap().command {
        ReplCommand::Disconnect { node, input, no_save } => { assert_eq!(node, "x"); assert_eq!(input, 2); assert!(no_save); }
        _ => panic!("expected Disconnect"),
    }
}

#[test]
fn repl_parse_remove_node() {
    match ReplCli::try_parse_from(["remove-node", "--id", "foo"]).unwrap().command {
        ReplCommand::RemoveNode { id, no_save } => { assert_eq!(id, "foo"); assert!(!no_save); }
        _ => panic!("expected RemoveNode"),
    }
}

#[test]
fn repl_parse_list_ops_with_group() {
    match ReplCli::try_parse_from(["list-ops", "--group", "images"]).unwrap().command {
        ReplCommand::ListOps { group, .. } => { assert_eq!(group, Some("images".to_string())); }
        _ => panic!("expected ListOps"),
    }
}

#[test]
fn repl_parse_unknown_command_returns_err() { assert!(ReplCli::try_parse_from(["not-a-command"]).is_err()); }

#[test]
fn repl_parse_empty_returns_err() { assert!(ReplCli::try_parse_from(Vec::<String>::new()).is_err()); }

// ── shell_words splitting ───────────────────────────────────────────────

#[test]
fn shell_words_split_quoted_json() {
    let words = shell_words::split(r#"set-input --node a1 --input 0 --value '{"Decimal":3.14}'"#).unwrap();
    assert_eq!(words.len(), 7);
    assert_eq!(words[6], r#"{"Decimal":3.14}"#);
}

#[test]
fn shell_words_split_double_quoted() {
    let words = shell_words::split(r#"set-input --node a1 --input 0 --value "{\"Decimal\":3.14}""#).unwrap();
    assert_eq!(words.len(), 7);
}

#[test]
fn shell_words_split_unclosed_quote_returns_err() {
    assert!(shell_words::split("add-node --type 'unclosed").is_err());
}

// ── process_repl_line integration tests ─────────────────────────────────

#[tokio::test]
async fn repl_line_exit_returns_true() {
    let path = create_temp_graph("repl_exit");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    assert!(process_repl_line(&mut graph, &path, "exit", OutputMode::Json, &mut buf).await);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_quit_returns_true() {
    let path = create_temp_graph("repl_quit");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    assert!(process_repl_line(&mut graph, &path, "quit", OutputMode::Json, &mut buf).await);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_add_node_modifies_graph() {
    let path = create_temp_graph("repl_addnode");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let exit = process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id test1 --no-save", OutputMode::Json, &mut buf).await;
    assert!(!exit);
    assert!(graph.nodes.contains_key("test1"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_add_node_json_output() {
    let path = create_temp_graph("repl_addnode_json");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id abc --no-save", OutputMode::Json, &mut buf).await;
    let out = output_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["data"]["node_id"], "abc");
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_remove_node_modifies_graph() {
    let path = create_temp_graph("repl_rmnode");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id rm1 --no-save", OutputMode::Json, &mut buf).await;
    assert!(graph.nodes.contains_key("rm1"));
    buf.clear();
    process_repl_line(&mut graph, &path, "remove-node --id rm1 --no-save", OutputMode::Json, &mut buf).await;
    assert!(!graph.nodes.contains_key("rm1"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_remove_nonexistent_node_errors() {
    let path = create_temp_graph("repl_rmghost");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "remove-node --id ghost --no-save", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "error");
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_set_input_modifies_value() {
    let path = create_temp_graph("repl_setinput");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id si1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, r#"set-input --node si1 --input 0 --value '{"Decimal":42.0}' --no-save"#, OutputMode::Json, &mut buf).await;
    let stored = &graph.nodes["si1"].inputs[0].value;
    assert!(matches!(stored, Value::Decimal(v) if (*v - 42.0).abs() < 1e-6), "unexpected: {:?}", stored);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_connect_creates_connection() {
    let path = create_temp_graph("repl_connect");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id p1 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id c1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "connect --from p1:0 --to c1:0 --no-save", OutputMode::Json, &mut buf).await;
    assert_eq!(graph.nodes["c1"].inputs[0].connection, Some(("p1".to_string(), 0)));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_disconnect_removes_connection() {
    let path = create_temp_graph("repl_disc");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id d1 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id d2 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, "connect --from d1:0 --to d2:0 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "disconnect --node d2 --input 0 --no-save", OutputMode::Json, &mut buf).await;
    assert_eq!(graph.nodes["d2"].inputs[0].connection, None);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_info_json_has_node_count() {
    let path = create_temp_graph("repl_info_json");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id n1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["data"]["node_count"], 1);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_info_human_contains_node_id() {
    let path = create_temp_graph("repl_info_human");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id vis1 --no-save", OutputMode::Human, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info", OutputMode::Human, &mut buf).await;
    let out = output_to_string(&buf);
    assert!(out.contains("vis1"));
    assert!(out.contains("nodes: 1"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_run_json_returns_outputs() {
    let path = create_temp_graph("repl_run_json");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id r1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "run --no-save", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert!(parsed["data"]["outputs"].is_array());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_run_human_contains_output() {
    let path = create_temp_graph("repl_run_human");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id rh1 --no-save", OutputMode::Human, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "run --no-save", OutputMode::Human, &mut buf).await;
    let out = output_to_string(&buf);
    assert!(out.contains("[rh1]"));
    assert!(out.contains("out[0]"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_help_emits_help() {
    let path = create_temp_graph("repl_help");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let exit = process_repl_line(&mut graph, &path, "help", OutputMode::Human, &mut buf).await;
    assert!(!exit);
    let out = output_to_string(&buf);
    assert!(out.contains("Available commands"));
    assert!(out.contains("add-node"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_invalid_command_emits_error() {
    let path = create_temp_graph("repl_invalid");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let exit = process_repl_line(&mut graph, &path, "not-a-command", OutputMode::Json, &mut buf).await;
    assert!(!exit);
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "error");
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_empty_words_no_panic() {
    let path = create_temp_graph("repl_emptywords");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let exit = process_repl_line(&mut graph, &path, "", OutputMode::Json, &mut buf).await;
    assert!(!exit);
    let _ = std::fs::remove_file(&path);
}

// ── --no-save behavior ──────────────────────────────────────────────────

#[tokio::test]
async fn repl_no_save_does_not_write_to_disk() {
    let path = create_temp_graph("repl_nosave");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let before = std::fs::read_to_string(&path).unwrap();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id ns1 --no-save", OutputMode::Json, &mut buf).await;
    assert!(graph.nodes.contains_key("ns1"));
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(before, after, "file should not have been modified with --no-save");
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_default_save_writes_to_disk() {
    let path = create_temp_graph("repl_autosave");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let before = std::fs::read_to_string(&path).unwrap();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id as1", OutputMode::Json, &mut buf).await;
    let after = std::fs::read_to_string(&path).unwrap();
    assert_ne!(before, after, "file should have been updated without --no-save");
    assert!(after.contains("as1"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_save_command_writes_pending_changes() {
    let path = create_temp_graph("repl_save_cmd");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id sv1 --no-save", OutputMode::Json, &mut buf).await;
    let mid = std::fs::read_to_string(&path).unwrap();
    assert!(!mid.contains("sv1"));
    buf.clear();
    process_repl_line(&mut graph, &path, "save", OutputMode::Json, &mut buf).await;
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("sv1"));
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_batch_no_save_then_save_persists_all() {
    let path = create_temp_graph("repl_batch");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id b1 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id b2 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id b3 --no-save", OutputMode::Json, &mut buf).await;
    let mid = std::fs::read_to_string(&path).unwrap();
    assert!(!mid.contains("b1"));
    process_repl_line(&mut graph, &path, "save", OutputMode::Json, &mut buf).await;
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("b1") && after.contains("b2") && after.contains("b3"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_list_ops_json() {
    let path = create_temp_graph("repl_listops");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "list-ops --group numbers/arithmetic", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert!(!parsed["data"]["operations"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_list_ops_human() {
    let path = create_temp_graph("repl_listops_h");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "list-ops --group numbers/arithmetic", OutputMode::Human, &mut buf).await;
    assert!(output_to_string(&buf).contains("numbers/arithmetic/add"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_unclosed_quote_emits_error() {
    let path = create_temp_graph("repl_unclosed");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let exit = process_repl_line(&mut graph, &path, "add-node --type 'unclosed", OutputMode::Json, &mut buf).await;
    assert!(!exit);
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "error");
    let _ = std::fs::remove_file(&path);
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

#[test]
fn do_info_empty_graph() {
    let path = create_temp_graph("do_info_empty");
    let graph = load_graph(&path).unwrap();
    let resp = do_info(&graph, None, false).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(resp.status, "ok");
    assert_eq!(resp.data.as_ref().unwrap()["node_count"], 0);
}

#[test]
fn do_list_ops_no_filter() {
    let resp = do_list_ops(None, None);
    assert_eq!(resp.status, "ok");
    assert!(!resp.data.as_ref().unwrap()["operations"].as_array().unwrap().is_empty());
}

#[test]
fn do_list_ops_nonmatching_group() {
    let resp = do_list_ops(Some("zzz_nonexistent"), None);
    assert_eq!(resp.status, "ok");
    assert!(resp.data.as_ref().unwrap()["operations"].as_array().unwrap().is_empty());
    assert!(!resp.data.as_ref().unwrap()["categories"].as_array().unwrap().is_empty());
}

// ── 9A: list-ops --search and --group category fallback ─────────────

#[test]
fn list_ops_search_by_path() {
    let resp = do_list_ops(None, Some("add"));
    let ops = resp.data.as_ref().unwrap()["operations"].as_array().unwrap();
    assert!(!ops.is_empty());
    for op in ops {
        let path = op["path"].as_str().unwrap().to_lowercase();
        let variant = op["variant"].as_str().unwrap().to_lowercase();
        let desc = op["description"].as_str().unwrap().to_lowercase();
        assert!(path.contains("add") || variant.contains("add") || desc.contains("add"), "expected 'add' in: {}", op);
    }
}

#[test]
fn list_ops_search_case_insensitive() {
    let lower = do_list_ops(None, Some("add"));
    let upper = do_list_ops(None, Some("ADD"));
    assert_eq!(
        lower.data.as_ref().unwrap()["operations"].as_array().unwrap().len(),
        upper.data.as_ref().unwrap()["operations"].as_array().unwrap().len()
    );
}

#[test]
fn list_ops_search_no_match() {
    let resp = do_list_ops(None, Some("zzzzznonexistent"));
    assert!(resp.data.as_ref().unwrap()["operations"].as_array().unwrap().is_empty());
}

#[test]
fn list_ops_group_fallback_shows_categories() {
    let text = format_list_ops_human(Some("zzz_bad_group"), None);
    assert!(text.contains("Available categories"));
    assert!(text.contains("numbers"));
    assert!(text.contains("images"));
}

#[test]
fn list_ops_group_valid_prefix() {
    let resp = do_list_ops(Some("numbers/arithmetic"), None);
    let ops = resp.data.as_ref().unwrap()["operations"].as_array().unwrap();
    assert!(!ops.is_empty());
    for op in ops { assert!(op["path"].as_str().unwrap().starts_with("numbers/arithmetic")); }
}

#[test]
fn list_ops_group_and_search_combined() {
    let resp = do_list_ops(Some("numbers"), Some("add"));
    let ops = resp.data.as_ref().unwrap()["operations"].as_array().unwrap();
    assert!(!ops.is_empty());
    for op in ops { assert!(op["path"].as_str().unwrap().starts_with("numbers")); }
}

#[tokio::test]
async fn repl_line_list_ops_search() {
    let path = create_temp_graph("repl_search");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "list-ops --search blur", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert!(!parsed["data"]["operations"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_file(&path);
}

// ── 9B: list-types ──────────────────────────────────────────────────

#[test]
fn list_types_all() {
    let resp = do_list_types(None);
    assert_eq!(resp.status, "ok");
    let types = resp.data.as_ref().unwrap()["types"].as_array().unwrap();
    assert!(types.len() >= 8);
    let names: Vec<&str> = types.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(names.contains(&"BlendMode") && names.contains(&"ColorSpace") && names.contains(&"FilterType"));
}

#[test]
fn list_types_blend_mode() {
    let resp = do_list_types(Some("BlendMode"));
    assert_eq!(resp.status, "ok");
    let variants = resp.data.as_ref().unwrap()["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 17);
    let names: Vec<&str> = variants.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(names.contains(&"Multiply") && names.contains(&"Screen"));
}

#[test]
fn list_types_case_insensitive() {
    let resp = do_list_types(Some("blendmode"));
    assert_eq!(resp.status, "ok");
    assert_eq!(resp.data.as_ref().unwrap()["variants"].as_array().unwrap().len(), 17);
}

#[test]
fn list_types_unknown() {
    let resp = do_list_types(Some("NotARealType"));
    assert_eq!(resp.status, "error");
    assert!(resp.error.as_ref().unwrap().contains("unknown type"));
}

#[test]
fn list_types_color_space() { assert_eq!(do_list_types(Some("ColorSpace")).data.as_ref().unwrap()["variants"].as_array().unwrap().len(), 9); }
#[test]
fn list_types_filter_type() { assert_eq!(do_list_types(Some("FilterType")).data.as_ref().unwrap()["variants"].as_array().unwrap().len(), 5); }
#[test]
fn list_types_image_type() { assert_eq!(do_list_types(Some("ImageType")).data.as_ref().unwrap()["variants"].as_array().unwrap().len(), 13); }
#[test]
fn list_types_text_halign() { assert_eq!(do_list_types(Some("TextHAlign")).data.as_ref().unwrap()["variants"].as_array().unwrap().len(), 3); }

#[test]
fn list_types_human_all() {
    let text = format_list_types_human(None);
    assert!(text.contains("BlendMode") && text.contains("ColorSpace"));
}

#[test]
fn list_types_human_specific() {
    let text = format_list_types_human(Some("BlendMode"));
    assert!(text.contains("Multiply") && text.contains("Screen"));
}

#[tokio::test]
async fn repl_line_list_types_json() {
    let path = create_temp_graph("repl_listtypes");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "list-types BlendMode", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["data"]["variants"].as_array().unwrap().len(), 17);
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn repl_line_list_types_all() {
    let path = create_temp_graph("repl_listtypes_all");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "list-types", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert!(parsed["data"]["types"].as_array().unwrap().len() >= 8);
    let _ = std::fs::remove_file(&path);
}

// ── 9C: info --node and --compact ───────────────────────────────────

#[tokio::test]
async fn info_node_filter_shows_only_matching_node() {
    let path = create_temp_graph("info_nodefilter");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id n1 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id n2 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info --node n1", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    let nodes = parsed["data"]["nodes"].as_object().unwrap();
    assert!(nodes.contains_key("n1") && !nodes.contains_key("n2"));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_node_filter_nonexistent_errors() {
    let path = create_temp_graph("info_nodefilter_bad");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "info --node ghost", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert_eq!(parsed["status"], "error");
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_shows_description() {
    let path = create_temp_graph("info_desc");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id d1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    let node = &parsed["data"]["nodes"]["d1"];
    assert!(node["description"].is_string() && !node["description"].as_str().unwrap().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_compact_omits_description() {
    let path = create_temp_graph("info_compact");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id c1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info --compact", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert!(parsed["data"]["nodes"]["c1"].get("description").is_none());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_shows_default_values() {
    let path = create_temp_graph("info_defaults");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id dv1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert!(parsed["data"]["nodes"]["dv1"]["inputs"].as_array().unwrap()[0].get("default_value").is_some());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_compact_omits_defaults() {
    let path = create_temp_graph("info_compact_def");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id cd1 --no-save", OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info --compact", OutputMode::Json, &mut buf).await;
    let parsed: serde_json::Value = serde_json::from_str(output_to_string(&buf).trim()).unwrap();
    assert!(parsed["data"]["nodes"]["cd1"]["inputs"].as_array().unwrap()[0].get("default_value").is_none());
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_human_shows_description() {
    let path = create_temp_graph("info_human_desc");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id hd1 --no-save", OutputMode::Human, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info", OutputMode::Human, &mut buf).await;
    assert!(output_to_string(&buf).contains("\""));
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn info_human_shows_default_when_changed() {
    let path = create_temp_graph("info_human_defchg");
    let mut graph = load_graph(&path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    process_repl_line(&mut graph, &path, "add-node --type numbers/arithmetic/add --id dc1 --no-save", OutputMode::Json, &mut buf).await;
    process_repl_line(&mut graph, &path, r#"set-input --node dc1 --input 0 --value '{"Decimal":99.0}' --no-save"#, OutputMode::Json, &mut buf).await;
    buf.clear();
    process_repl_line(&mut graph, &path, "info", OutputMode::Human, &mut buf).await;
    assert!(output_to_string(&buf).contains("(default:"));
    let _ = std::fs::remove_file(&path);
}

// ── 9D: --input flag naming ─────────────────────────────────────────

#[test]
fn repl_parse_set_input_uses_input_flag() {
    match ReplCli::try_parse_from(["set-input", "--node", "n1", "--input", "2", "--value", r#"{"Integer":1}"#]).unwrap().command {
        ReplCommand::SetInput { node, input, value, .. } => { assert_eq!(node, "n1"); assert_eq!(input, 2); assert!(value.contains("Integer")); }
        _ => panic!("expected SetInput"),
    }
}

#[test]
fn repl_parse_set_input_rejects_old_index_flag() {
    assert!(ReplCli::try_parse_from(["set-input", "--node", "n1", "--index", "0", "--value", r#"{"Integer":1}"#]).is_err());
}

// ── 9E: Better error messages ───────────────────────────────────────

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
