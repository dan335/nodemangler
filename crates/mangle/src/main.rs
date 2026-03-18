//! `mangle` — CLI for the NodeMangler graph engine.
//!
//! Allows AI agents and terminal users to create, inspect, and execute node
//! graphs from the command line. Each command loads a graph JSON file, performs
//! one operation, saves it back, and prints a result.

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mangler::{
    graph::Graph, get_id, AddNodeType, GraphSaveData,
    operations::{operation_list, Operation, OperationListItem},
    value::Value,
};

// ── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "mangle", about = "CLI for the NodeMangler graph engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new empty graph JSON file
    New {
        /// Path to write the graph file (e.g. graph.json)
        path: PathBuf,
    },

    /// Print all nodes, inputs, outputs, and connections in a graph
    Info {
        /// Path to the graph JSON file
        path: PathBuf,
    },

    /// List all available operation types
    ListOps {
        /// Filter by category prefix (e.g. numbers, images/transform, colors)
        #[arg(long)]
        group: Option<String>,
    },

    /// Add a node to a graph
    AddNode {
        /// Path to the graph JSON file
        path: PathBuf,
        /// Operation type: full variant name (OpNumberMathAdd) or short path (numbers/arithmetic/add)
        #[arg(long = "type", id = "op_type")]
        op_type: String,
        /// Node ID to assign (auto-generated if omitted)
        #[arg(long)]
        id: Option<String>,
    },

    /// Remove a node and all its connections from a graph
    RemoveNode {
        /// Path to the graph JSON file
        path: PathBuf,
        /// ID of the node to remove
        #[arg(long)]
        id: String,
    },

    /// Connect an output slot to an input slot
    Connect {
        /// Path to the graph JSON file
        path: PathBuf,
        /// Source: <node-id>:<output-index>
        #[arg(long)]
        from: String,
        /// Destination: <node-id>:<input-index>
        #[arg(long)]
        to: String,
    },

    /// Remove the connection feeding into a specific input
    Disconnect {
        /// Path to the graph JSON file
        path: PathBuf,
        /// ID of the node whose input should be disconnected
        #[arg(long)]
        node: String,
        /// Zero-based input index to disconnect
        #[arg(long)]
        input: usize,
    },

    /// Set a literal value on a node input
    SetInput {
        /// Path to the graph JSON file
        path: PathBuf,
        /// ID of the target node
        #[arg(long)]
        node: String,
        /// Zero-based input index
        #[arg(long)]
        index: usize,
        /// JSON-encoded Value, e.g. `{"Decimal":3.14}` or `{"Bool":true}`
        #[arg(long)]
        value: String,
    },

    /// Execute the graph and print all node output values
    Run {
        /// Path to the graph JSON file
        path: PathBuf,
    },
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { path } => cmd_new(path),
        Commands::Info { path } => cmd_info(path),
        Commands::ListOps { group } => cmd_list_ops(group),
        Commands::AddNode { path, op_type, id } => cmd_add_node(path, op_type, id).await,
        Commands::RemoveNode { path, id } => cmd_remove_node(path, id).await,
        Commands::Connect { path, from, to } => cmd_connect(path, from, to).await,
        Commands::Disconnect { path, node, input } => cmd_disconnect(path, node, input).await,
        Commands::SetInput { path, node, index, value } => cmd_set_input(path, node, index, value),
        Commands::Run { path } => cmd_run(path).await,
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// ── Graph load/save helpers ───────────────────────────────────────────────────

/// Load a graph from a JSON file with no UI channels.
fn load_graph(path: &PathBuf) -> Result<Graph, String> {
    Graph::load(path.clone(), None, None, false).map_err(|e| e.0)
}

/// Serialize a graph and write it to a JSON file.
fn save_graph(graph: &Graph, path: &PathBuf) -> Result<(), String> {
    let save_data = graph.to_save_data();
    let json = serde_json::to_string_pretty(&save_data).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ── Operation registry helpers ────────────────────────────────────────────────

/// Recursively flatten the `operation_list()` tree into `(short_path, Operation)` pairs.
///
/// The path is built by joining category names with `/`, e.g. `numbers/arithmetic/add`.
fn flatten_ops(items: &[OperationListItem], prefix: &str) -> Vec<(String, Operation)> {
    let mut result = Vec::new();
    for item in items {
        match item {
            OperationListItem::Category { name, operation_list_items } => {
                let new_prefix = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}/{name}")
                };
                result.extend(flatten_ops(operation_list_items, &new_prefix));
            }
            OperationListItem::Operation { operation } => {
                let op_name = operation.settings().name;
                let path = if prefix.is_empty() {
                    op_name
                } else {
                    format!("{prefix}/{op_name}")
                };
                result.push((path, operation.clone()));
            }
            OperationListItem::Subgraph => {}
        }
    }
    result
}

/// Resolve an operation type string to an `Operation` variant.
///
/// Accepts either the short path (`numbers/arithmetic/add`, case-insensitive)
/// or the full serde variant name (`OpNumberMathAdd`).
fn resolve_op(type_str: &str) -> Result<Operation, String> {
    let all_ops = flatten_ops(&operation_list(), "");

    // Try short path first (case-insensitive).
    let by_path: HashMap<String, Operation> =
        all_ops.iter().map(|(p, op)| (p.to_lowercase(), op.clone())).collect();
    if let Some(op) = by_path.get(&type_str.to_lowercase()) {
        return Ok(op.clone());
    }

    // Try full variant name via serde round-trip.
    let json = format!("\"{}\"", type_str);
    if let Ok(op) = serde_json::from_str::<Operation>(&json) {
        return Ok(op);
    }

    Err(format!(
        "unknown operation '{}' — run `mangle list-ops` to see all types",
        type_str
    ))
}

/// Parse a `node-id:index` slot string into `(node_id, index)`.
fn parse_slot(s: &str) -> Result<(String, usize), String> {
    // Split on the last `:` so node IDs that contain `:` still work.
    let colon = s.rfind(':').ok_or_else(|| {
        format!("expected <node-id>:<index>, got '{s}'")
    })?;
    let node_id = s[..colon].to_string();
    let index: usize = s[colon + 1..]
        .parse()
        .map_err(|_| format!("invalid index in '{s}'"))?;
    Ok((node_id, index))
}

// ── Value display ─────────────────────────────────────────────────────────────

/// Return a concise human-readable representation of a `Value`.
fn display_value(value: &Value) -> String {
    match value {
        Value::DynamicImage { data, .. } => format!("<image {}x{}>", data.width(), data.height()),
        _ => serde_json::to_string(value).unwrap_or_else(|_| format!("{:?}", value)),
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// `mangle new <path>` — create an empty graph file.
fn cmd_new(path: PathBuf) -> Result<(), String> {
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    let save_data = GraphSaveData {
        id: get_id(),
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("new graph")
            .to_string(),
        nodes: HashMap::new(),
    };
    let json = serde_json::to_string_pretty(&save_data).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    println!("created {}", path.display());
    Ok(())
}

/// `mangle info <path>` — print graph structure.
fn cmd_info(path: PathBuf) -> Result<(), String> {
    let graph = load_graph(&path)?;
    println!("graph: {} ({})", graph.name, graph.id);
    println!("nodes: {}", graph.nodes.len());

    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    for node_id in node_ids {
        let node = &graph.nodes[node_id];

        // Operation type
        let type_label = match &node.node_type {
            mangler::node_type::NodeType::Operation { operation } => {
                // Get the variant name via serde
                serde_json::to_string(operation)
                    .unwrap_or_else(|_| format!("{:?}", operation))
                    .trim_matches('"')
                    .to_string()
            }
            mangler::node_type::NodeType::Subgraph { path, .. } => {
                format!("subgraph({})", path.display())
            }
        };

        println!("\n  [{}] {} ({})", node_id, node.settings.name, type_label);

        // Inputs
        for (i, input) in node.inputs.iter().enumerate() {
            let conn = if let Some((src_node, src_idx)) = &input.connection {
                format!(" ← {}:{}", src_node, src_idx)
            } else {
                String::new()
            };
            println!(
                "    in[{}] {} ({:?}) = {}{}",
                i,
                input.name,
                input.value.value_type(),
                display_value(&input.value),
                conn
            );
        }

        // Outputs
        for (i, output) in node.outputs.iter().enumerate() {
            let conn = if let Some(conns) = &output.connection {
                let s: Vec<String> = conns.iter().map(|(n, idx)| format!("{}:{}", n, idx)).collect();
                format!(" → {}", s.join(", "))
            } else {
                String::new()
            };
            println!(
                "    out[{}] {} ({:?}) = {}{}",
                i,
                output.name,
                output.value.value_type(),
                display_value(&output.value),
                conn
            );
        }
    }
    Ok(())
}

/// `mangle list-ops [--group <prefix>]` — list available operations.
fn cmd_list_ops(group: Option<String>) -> Result<(), String> {
    let all_ops = flatten_ops(&operation_list(), "");
    let filter = group.as_deref().unwrap_or("").to_lowercase();

    for (path, op) in &all_ops {
        if !filter.is_empty() && !path.to_lowercase().starts_with(&filter) {
            continue;
        }

        // Get the serde variant name (e.g. "OpNumberMathAdd").
        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let inputs = op.create_inputs();
        let outputs = op.create_outputs();

        let in_str: Vec<String> = inputs
            .iter()
            .map(|i| format!("{}({:?})", i.name, i.value.value_type()))
            .collect();
        let out_str: Vec<String> = outputs
            .iter()
            .map(|o| format!("{}({:?})", o.name, o.value.value_type()))
            .collect();

        println!(
            "{:<45} ({})  in: [{}]  out: [{}]",
            path,
            variant,
            in_str.join(", "),
            out_str.join(", ")
        );
    }
    Ok(())
}

/// `mangle add-node <path> --type <type> [--id <id>]` — add a node to the graph.
async fn cmd_add_node(path: PathBuf, op_type: String, id: Option<String>) -> Result<(), String> {
    let operation = resolve_op(&op_type)?;
    let mut graph = load_graph(&path)?;
    let node_id = id.unwrap_or_else(get_id);
    graph
        .add_node(node_id.clone(), AddNodeType::Operation(operation), glam::Vec2::ZERO)
        .await;
    save_graph(&graph, &path)?;
    println!("{node_id}");
    Ok(())
}

/// `mangle remove-node <path> --id <id>` — remove a node and its connections.
async fn cmd_remove_node(path: PathBuf, id: String) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&id) {
        return Err(format!("node '{id}' not found"));
    }
    graph.remove_node(id.clone()).await;
    save_graph(&graph, &path)?;
    println!("removed {id}");
    Ok(())
}

/// `mangle connect <path> --from <node:out> --to <node:in>` — connect two nodes.
async fn cmd_connect(path: PathBuf, from: String, to: String) -> Result<(), String> {
    let (output_node_id, output_index) = parse_slot(&from)?;
    let (input_node_id, input_index) = parse_slot(&to)?;
    let mut graph = load_graph(&path)?;
    graph
        .add_connection(input_node_id, input_index, output_node_id, output_index)
        .await;
    save_graph(&graph, &path)?;
    println!("connected {from} → {to}");
    Ok(())
}

/// `mangle disconnect <path> --node <id> --input <n>` — remove a connection.
async fn cmd_disconnect(path: PathBuf, node: String, input: usize) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(format!("node '{node}' not found"));
    }
    graph.remove_connection(node.clone(), input).await;
    save_graph(&graph, &path)?;
    println!("disconnected {node}:{input}");
    Ok(())
}

/// `mangle set-input <path> --node <id> --index <n> --value <json>` — set an input value.
fn cmd_set_input(path: PathBuf, node: String, index: usize, value: String) -> Result<(), String> {
    let parsed: Value = serde_json::from_str(&value).map_err(|e| {
        format!("invalid value JSON: {e}. Expected e.g. {{\"Decimal\":3.14}} or {{\"Bool\":true}}")
    })?;
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(format!("node '{node}' not found"));
    }
    graph.set_input(node.clone(), index, parsed);
    save_graph(&graph, &path)?;
    println!("set {node}:{index} = {value}");
    Ok(())
}

/// `mangle run <path>` — execute the graph and print all output values.
async fn cmd_run(path: PathBuf) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    graph.run().await;
    save_graph(&graph, &path)?;

    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    for node_id in node_ids {
        let node = &graph.nodes[node_id];
        for (i, output) in node.outputs.iter().enumerate() {
            println!(
                "[{}] out[{}] ({:?}) = {}",
                node_id,
                i,
                output.value.value_type(),
                display_value(&output.value)
            );
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── flatten_ops ───────────────────────────────────────────────────────────

    #[test]
    fn flatten_ops_returns_non_empty() {
        let all = flatten_ops(&operation_list(), "");
        assert!(!all.is_empty());
    }

    #[test]
    fn flatten_ops_paths_contain_slash() {
        // Every path is category/…/name so must have at least one `/`.
        for (path, _) in flatten_ops(&operation_list(), "") {
            assert!(path.contains('/'), "expected '/' in path: {path}");
        }
    }

    #[test]
    fn flatten_ops_prefix_prepended() {
        // Paths in the "numbers" top-level category should start with "numbers/".
        let all = flatten_ops(&operation_list(), "");
        let numbers: Vec<_> = all.iter().filter(|(p, _)| p.starts_with("numbers/")).collect();
        assert!(!numbers.is_empty(), "expected at least one numbers/* operation");
    }

    #[test]
    fn flatten_ops_custom_prefix() {
        // When a non-empty prefix is supplied it appears at the start of every path.
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
        // Full serde variant name should also be accepted.
        assert!(resolve_op("OpNumberMathAdd").is_ok());
    }

    #[test]
    fn resolve_op_short_and_variant_yield_same_operation() {
        let by_path = resolve_op("numbers/arithmetic/add").unwrap();
        let by_name = resolve_op("OpNumberMathAdd").unwrap();
        // Compare via their serde representations.
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

    // ── cmd_new ───────────────────────────────────────────────────────────────

    #[test]
    fn cmd_new_creates_valid_graph_file() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_new_{}.json", std::process::id()));
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
        let path = std::env::temp_dir()
            .join(format!("mangle_test_stem_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        cmd_new(path.clone()).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        // The graph name should be derived from the file stem, e.g. "mangle_test_stem_<pid>".
        let stem = path.file_stem().unwrap().to_str().unwrap();
        assert!(contents.contains(stem), "expected stem '{stem}' in: {contents}");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cmd_new_fails_if_file_already_exists() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_exists_{}.json", std::process::id()));
        std::fs::write(&path, "{}").unwrap();

        let result = cmd_new(path.clone());
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cmd_new_fails_in_nonexistent_directory() {
        let path = std::env::temp_dir()
            .join("mangle_no_such_dir_xyz")
            .join("graph.json");
        assert!(cmd_new(path).is_err());
    }

    // ── parse_slot: further edge cases ───────────────────────────────────────

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

    // ── flatten_ops: further edge cases ──────────────────────────────────────

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

    /// A category containing no operations (or only sub-categories that are
    /// themselves empty) contributes nothing to the flat list.
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
        // The add operation's settings().name is "add".
        assert_eq!(flat[0].0, "a/b/add");
    }

    // ── resolve_op: further edge cases ───────────────────────────────────────

    /// Leading whitespace is not trimmed — the lookup must fail.
    #[test]
    fn resolve_op_leading_whitespace_returns_err() {
        assert!(resolve_op(" numbers/arithmetic/add").is_err());
    }

    /// Trailing whitespace is not trimmed — the lookup must fail.
    #[test]
    fn resolve_op_trailing_whitespace_returns_err() {
        assert!(resolve_op("numbers/arithmetic/add ").is_err());
    }

    /// A category path without a terminal operation name is not an operation.
    #[test]
    fn resolve_op_category_path_only_returns_err() {
        assert!(resolve_op("numbers/arithmetic").is_err());
        assert!(resolve_op("numbers").is_err());
    }

    /// Operations from non-numbers categories are also resolvable.
    #[test]
    fn resolve_op_other_categories_resolve() {
        assert!(resolve_op("logic/comparison/equal").is_ok());
        assert!(resolve_op("colors/blend/blend").is_ok());
    }

    /// Short path and full variant name resolve to the same operation.
    #[test]
    fn resolve_op_short_path_and_variant_are_equivalent() {
        let by_path = resolve_op("numbers/arithmetic/add").unwrap();
        let by_name = resolve_op("OpNumberMathAdd").unwrap();
        assert_eq!(
            serde_json::to_string(&by_path).unwrap(),
            serde_json::to_string(&by_name).unwrap(),
        );
    }

    // ── display_value: further edge cases ────────────────────────────────────

    #[test]
    fn display_value_trigger() {
        let s = display_value(&Value::Trigger);
        assert!(s.contains("Trigger"), "unexpected: {s}");
    }

    #[test]
    fn display_value_empty_text() {
        let s = display_value(&Value::Text(String::new()));
        // Serializes as {"Text":""} — just verify it doesn't panic and includes Text.
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

    // ── load_graph: edge cases ────────────────────────────────────────────────

    /// A path that does not exist must return an Err.
    #[test]
    fn load_graph_missing_file_returns_err() {
        let path = PathBuf::from("/nonexistent/path/does/not/exist_mangle.json");
        assert!(load_graph(&path).is_err());
    }

    /// A file containing arbitrary non-JSON text must return an Err.
    #[test]
    fn load_graph_invalid_json_returns_err() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_badjson_{}.json", std::process::id()));
        std::fs::write(&path, "this is not json at all").unwrap();
        let result = load_graph(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    /// An empty JSON object is missing required fields and must fail deserialization.
    #[test]
    fn load_graph_empty_object_returns_err() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_emptyobj_{}.json", std::process::id()));
        std::fs::write(&path, "{}").unwrap();
        let result = load_graph(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    /// A graph created by `cmd_new` must load successfully with an empty node map.
    #[test]
    fn load_graph_freshly_created_graph_is_empty() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_fresh_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();
        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(graph.nodes.is_empty());
    }

    // ── save_graph / load_graph round-trip ───────────────────────────────────

    /// Name and ID survive a save/load cycle unchanged.
    #[test]
    fn save_load_round_trip_preserves_name_and_id() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_roundtrip_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        let graph = load_graph(&path).unwrap();
        let original_id = graph.id.clone();
        let original_name = graph.name.clone();

        save_graph(&graph, &path).unwrap();
        let reloaded = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(reloaded.id, original_id);
        assert_eq!(reloaded.name, original_name);
    }

    // ── cmd_set_input: edge cases ─────────────────────────────────────────────

    /// Passing something that is not valid JSON must fail immediately, before any
    /// graph I/O occurs.
    #[test]
    fn cmd_set_input_invalid_json_returns_err() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_setinput_badjson_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();
        let result = cmd_set_input(path.clone(), "any".to_string(), 0, "not json".to_string());
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    /// Referencing a node ID that is not in the graph must return an Err.
    #[test]
    fn cmd_set_input_unknown_node_returns_err() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_setinput_nonode_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();
        let result = cmd_set_input(
            path.clone(),
            "ghost-node".to_string(),
            0,
            r#"{"Integer":42}"#.to_string(),
        );
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    // ── async integration tests ───────────────────────────────────────────────

    /// Adding a node writes it to the file; loading the file back confirms it exists.
    #[tokio::test]
    async fn cmd_add_node_persists_to_file() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_addnode_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        let node_id = format!("test-node-{}", std::process::id());
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()))
            .await
            .unwrap();

        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(graph.nodes.contains_key(&node_id));
    }

    /// Removing a node that was previously added must leave the graph empty again.
    #[tokio::test]
    async fn cmd_add_then_remove_node_leaves_graph_empty() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_addremove_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        let node_id = format!("addremove-{}", std::process::id());
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()))
            .await
            .unwrap();
        cmd_remove_node(path.clone(), node_id.clone()).await.unwrap();

        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(!graph.nodes.contains_key(&node_id));
    }

    /// Trying to remove a node that does not exist must return an Err.
    #[tokio::test]
    async fn cmd_remove_node_unknown_returns_err() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_removemissing_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();
        let result = cmd_remove_node(path.clone(), "ghost".to_string()).await;
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    /// Setting a valid input on a real node must succeed and persist.
    #[tokio::test]
    async fn cmd_set_input_on_real_node_succeeds() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_setinput_valid_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        let node_id = format!("add-node-{}", std::process::id());
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some(node_id.clone()))
            .await
            .unwrap();

        // The add node's first input is Decimal; set it to 7.0.
        let result = cmd_set_input(
            path.clone(),
            node_id.clone(),
            0,
            r#"{"Decimal":7.0}"#.to_string(),
        );
        assert!(result.is_ok());

        // Reload and verify the value was actually saved.
        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let stored = &graph.nodes[&node_id].inputs[0].value;
        assert!(
            matches!(stored, Value::Decimal(v) if (*v - 7.0).abs() < 1e-6),
            "unexpected stored value: {:?}",
            stored
        );
    }

    /// Connecting two nodes must store the connection on the consumer's input.
    #[tokio::test]
    async fn cmd_connect_stores_connection_on_consumer() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_connect_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("producer".to_string()))
            .await.unwrap();
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("consumer".to_string()))
            .await.unwrap();

        cmd_connect(path.clone(), "producer:0".to_string(), "consumer:0".to_string())
            .await.unwrap();

        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            graph.nodes["consumer"].inputs[0].connection,
            Some(("producer".to_string(), 0))
        );
    }

    /// Disconnecting removes the connection stored on the consumer input.
    #[tokio::test]
    async fn cmd_disconnect_removes_connection() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_disconnect_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("src".to_string()))
            .await.unwrap();
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), Some("dst".to_string()))
            .await.unwrap();
        cmd_connect(path.clone(), "src:0".to_string(), "dst:0".to_string())
            .await.unwrap();
        cmd_disconnect(path.clone(), "dst".to_string(), 0).await.unwrap();

        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(graph.nodes["dst"].inputs[0].connection, None);
    }

    /// Disconnecting an input on a node that does not exist must return an Err.
    #[tokio::test]
    async fn cmd_disconnect_unknown_node_returns_err() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_disc_nonode_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();
        let result = cmd_disconnect(path.clone(), "ghost".to_string(), 0).await;
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    /// Adding the same explicit node ID twice is idempotent in terms of count
    /// (the graph should contain exactly the nodes that were successfully added).
    #[tokio::test]
    async fn cmd_add_node_auto_id_is_unique_across_calls() {
        let path = std::env::temp_dir()
            .join(format!("mangle_test_autoid_{}.json", std::process::id()));
        cmd_new(path.clone()).unwrap();

        // Add two nodes without specifying IDs — both must be retained.
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), None).await.unwrap();
        cmd_add_node(path.clone(), "numbers/arithmetic/add".to_string(), None).await.unwrap();

        let graph = load_graph(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(graph.nodes.len(), 2, "expected exactly 2 distinct nodes");
    }
}
