//! `mangle` — CLI for the NodeMangler graph engine.
//!
//! Allows AI agents and terminal users to create, inspect, and execute node
//! graphs from the command line. Each command loads a graph JSON file, performs
//! one operation, saves it back, and prints a result.
//!
//! The `repl` subcommand enters an interactive loop with the graph loaded in
//! memory. Supports `--json` mode for LLM/scripted consumers (newline-delimited
//! JSON, no ANSI codes, no prompt) and `--no-save` per-command to batch
//! mutations before an explicit `save`.

use std::collections::HashMap;
use std::io::{BufRead, Write as IoWrite};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mangler_core::{
    graph::Graph, get_id, AddNodeType, GraphSaveData,
    operations::{operation_list, Operation, OperationListItem},
    value::{Value, ValueType},
};
use serde::Serialize;

// ── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "mangle", about = "CLI for the NodeMangler graph engine", after_help = "\
Examples:
  mangle graph.json new                         Create a new graph
  mangle graph.json info                        Inspect all nodes
  mangle graph.json info --node mynode          Inspect one node
  mangle show-ops                               List all operations
  mangle show-ops --search blur                 Find operations by keyword
  mangle show-ops --group images/transform      Browse a category
  mangle show-op images/combine/blend           Detailed operation info
  mangle show-types BlendMode                   List enum variants
  mangle show-values                            JSON value format reference
  mangle graph.json add-node --type images/combine/blend
  mangle graph.json set-input --node <id> --input 0 --value '{\"Decimal\":3.14}'
  mangle graph.json run                         Execute and print outputs
  mangle graph.json repl                        Interactive REPL mode
  mangle graph.json repl --json                 REPL with JSON output (for AI/scripts)

REPL mode:
  The `repl` subcommand loads a graph and enters an interactive loop.
  All commands above (except `new`) work inside the REPL without the path.
  Use `--no-save` on mutation commands to batch changes, then `save` explicitly.
  Use `--json` for newline-delimited JSON output (no ANSI codes, no prompt) —
  ideal for LLM agents and scripted pipelines.")]
struct Cli {
    /// Path to the graph JSON file (required for most commands, placed before the subcommand)
    path: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new empty graph JSON file
    #[command(override_usage = "mangle <PATH> new")]
    New,

    /// Print all nodes, inputs, outputs, and connections in a graph
    #[command(override_usage = "mangle <PATH> info [OPTIONS]")]
    Info {
        /// Show only a single node by ID
        #[arg(long)]
        node: Option<String>,
        /// Compact output: omit descriptions and default values
        #[arg(long)]
        compact: bool,
    },

    /// Show all available operation types
    ShowOps {
        /// Filter by category prefix (e.g. numbers, images/transform, colors).
        /// Shows categories with counts if no ops match.
        #[arg(long)]
        group: Option<String>,
        /// Case-insensitive substring search across path, variant, and description
        #[arg(long)]
        search: Option<String>,
    },

    /// Show enum value types and their valid variants
    ShowTypes {
        /// Type name to show variants for (e.g. BlendMode). Omit to list all types.
        type_name: Option<String>,
    },

    /// Show JSON value format reference for set-input --value
    ShowValues,

    /// Show detailed info for a single operation type (no graph file needed)
    ShowOp {
        /// Operation type: full variant name or short path (e.g. images/combine/blend)
        #[arg(id = "op_type")]
        op_type: String,
    },

    /// Add a node to a graph
    #[command(override_usage = "mangle <PATH> add-node [OPTIONS] --type <op_type>")]
    AddNode {
        /// Operation type: full variant name (OpNumberMathAdd) or short path (numbers/arithmetic/add)
        #[arg(long = "type", id = "op_type")]
        op_type: String,
        /// Node ID to assign (auto-generated if omitted)
        #[arg(long)]
        id: Option<String>,
    },

    /// Remove a node and all its connections from a graph
    #[command(override_usage = "mangle <PATH> remove-node --id <ID>")]
    RemoveNode {
        /// ID of the node to remove
        #[arg(long)]
        id: String,
    },

    /// Connect an output slot to an input slot
    #[command(override_usage = "mangle <PATH> connect --from <FROM> --to <TO>")]
    Connect {
        /// Source: <node-id>:<output-index>
        #[arg(long)]
        from: String,
        /// Destination: <node-id>:<input-index>
        #[arg(long)]
        to: String,
    },

    /// Remove the connection feeding into a specific input
    #[command(override_usage = "mangle <PATH> disconnect --node <NODE> --input <INPUT>")]
    Disconnect {
        /// ID of the node whose input should be disconnected
        #[arg(long)]
        node: String,
        /// Zero-based input index to disconnect
        #[arg(long)]
        input: usize,
    },

    /// Set a literal value on a node input
    #[command(override_usage = "mangle <PATH> set-input --node <NODE> --input <INPUT> --value <VALUE>")]
    SetInput {
        /// ID of the target node
        #[arg(long)]
        node: String,
        /// Zero-based input index
        #[arg(long)]
        input: usize,
        /// JSON-encoded Value, e.g. `{"Decimal":3.14}` or `{"Bool":true}`
        #[arg(long)]
        value: String,
    },

    /// Execute the graph and print all node output values
    #[command(override_usage = "mangle <PATH> run")]
    Run,

    /// Enter interactive REPL mode with a graph loaded in memory
    #[command(override_usage = "mangle <PATH> repl [OPTIONS]")]
    Repl {
        /// Output JSON lines instead of human-readable text (no ANSI, no prompt)
        #[arg(long)]
        json: bool,
    },
}

// ── REPL command definition ──────────────────────────────────────────────────

/// Parser for REPL input lines. Uses `no_binary_name` so clap does not expect
/// argv[0] to be the program name.
#[derive(Parser, Debug)]
#[command(name = "", no_binary_name = true, disable_help_flag = true, disable_help_subcommand = true)]
struct ReplCli {
    #[command(subcommand)]
    command: ReplCommand,
}

/// Commands available inside the REPL. Mirrors the top-level `Commands` but
/// omits `New`/`Repl` and drops the `path` argument (graph is already loaded).
/// Mutation commands accept `--no-save` to suppress auto-save.
#[derive(Subcommand, Debug)]
enum ReplCommand {
    /// Print all nodes, inputs, outputs, and connections
    Info {
        /// Show only a single node by ID
        #[arg(long)]
        node: Option<String>,
        /// Compact output: omit descriptions and default values
        #[arg(long)]
        compact: bool,
    },

    /// Show all available operation types
    ShowOps {
        /// Filter by category prefix
        #[arg(long)]
        group: Option<String>,
        /// Case-insensitive substring search
        #[arg(long)]
        search: Option<String>,
    },

    /// Show enum value types and their valid variants
    ShowTypes {
        /// Type name to show variants for
        type_name: Option<String>,
    },

    /// Show JSON value format reference for set-input --value
    ShowValues,

    /// Show detailed info for a single operation type
    ShowOp {
        /// Operation type: full variant name or short path
        #[arg(id = "op_type")]
        op_type: String,
    },

    /// Add a node to the graph
    AddNode {
        /// Operation type path or variant name
        #[arg(long = "type", id = "op_type")]
        op_type: String,
        /// Node ID to assign (auto-generated if omitted)
        #[arg(long)]
        id: Option<String>,
        /// Skip auto-save after this mutation
        #[arg(long)]
        no_save: bool,
    },

    /// Remove a node and all its connections
    RemoveNode {
        /// ID of the node to remove
        #[arg(long)]
        id: String,
        /// Skip auto-save after this mutation
        #[arg(long)]
        no_save: bool,
    },

    /// Connect an output slot to an input slot
    Connect {
        /// Source: <node-id>:<output-index>
        #[arg(long)]
        from: String,
        /// Destination: <node-id>:<input-index>
        #[arg(long)]
        to: String,
        /// Skip auto-save after this mutation
        #[arg(long)]
        no_save: bool,
    },

    /// Remove the connection feeding into a specific input
    Disconnect {
        /// ID of the node whose input should be disconnected
        #[arg(long)]
        node: String,
        /// Zero-based input index to disconnect
        #[arg(long)]
        input: usize,
        /// Skip auto-save after this mutation
        #[arg(long)]
        no_save: bool,
    },

    /// Set a literal value on a node input
    SetInput {
        /// ID of the target node
        #[arg(long)]
        node: String,
        /// Zero-based input index
        #[arg(long)]
        input: usize,
        /// JSON-encoded Value
        #[arg(long)]
        value: String,
        /// Skip auto-save after this mutation
        #[arg(long)]
        no_save: bool,
    },

    /// Execute the graph and print all node output values
    Run {
        /// Skip auto-save after execution
        #[arg(long)]
        no_save: bool,
    },

    /// Explicitly save the graph to disk
    Save,

    /// Exit the REPL
    Exit,

    /// Exit the REPL (alias for exit)
    Quit,

    /// Print available REPL commands
    Help,
}

// ── REPL response types ──────────────────────────────────────────────────────

/// Structured response from a REPL command, serializable to JSON for `--json` mode.
#[derive(Debug, Clone, Serialize)]
struct ReplResponse {
    /// `"ok"` or `"error"`.
    status: String,
    /// Response payload (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    /// Error message (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ReplResponse {
    /// Create a successful response with a data payload.
    fn ok(data: serde_json::Value) -> Self {
        Self { status: "ok".to_string(), data: Some(data), error: None }
    }

    /// Create a successful response with a simple message.
    fn ok_message(msg: impl Into<String>) -> Self {
        Self::ok(serde_json::json!({ "message": msg.into() }))
    }

    /// Create an error response.
    fn error(msg: impl Into<String>) -> Self {
        Self { status: "error".to_string(), data: None, error: Some(msg.into()) }
    }
}

/// Output mode for the REPL.
#[derive(Debug, Clone, Copy, PartialEq)]
enum OutputMode {
    /// Human-readable text with prompt.
    Human,
    /// Newline-delimited JSON, no ANSI codes or prompt.
    Json,
}

/// Write a REPL response to the given writer in the appropriate format.
fn emit(mode: OutputMode, response: &ReplResponse, writer: &mut dyn IoWrite) {
    match mode {
        OutputMode::Json => {
            let json = serde_json::to_string(response).unwrap_or_else(|_| {
                r#"{"status":"error","error":"failed to serialize response"}"#.to_string()
            });
            let _ = writeln!(writer, "{json}");
        }
        OutputMode::Human => {
            if response.status == "ok" {
                if let Some(data) = &response.data {
                    if let Some(msg) = data.get("message").and_then(|v| v.as_str()) {
                        let _ = writeln!(writer, "{msg}");
                    } else {
                        let _ = writeln!(writer, "{}", serde_json::to_string_pretty(data).unwrap_or_default());
                    }
                }
            } else if let Some(err) = &response.error {
                let _ = writeln!(writer, "error: {err}");
            }
        }
    }
    let _ = writer.flush();
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = run(cli).await;

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Dispatch the parsed CLI to the appropriate command handler.
async fn run(cli: Cli) -> Result<(), String> {
    // Extract the path, returning an error if it was not provided.
    let require_path = || -> Result<PathBuf, String> {
        cli.path.clone().ok_or_else(|| "a graph file path is required before this command (e.g. mangle graph.json <command>)".to_string())
    };

    match cli.command {
        Commands::New => cmd_new(require_path()?),
        Commands::Info { node, compact } => cmd_info(require_path()?, node, compact),
        Commands::ShowOps { group, search } => cmd_show_ops(group, search),
        Commands::ShowTypes { type_name } => cmd_show_types(type_name),
        Commands::ShowValues => { print!("{}", show_values_text()); Ok(()) }
        Commands::ShowOp { op_type } => cmd_show_op(op_type),
        Commands::AddNode { op_type, id } => cmd_add_node(require_path()?, op_type, id).await,
        Commands::RemoveNode { id } => cmd_remove_node(require_path()?, id).await,
        Commands::Connect { from, to } => cmd_connect(require_path()?, from, to).await,
        Commands::Disconnect { node, input } => cmd_disconnect(require_path()?, node, input).await,
        Commands::SetInput { node, input, value } => cmd_set_input(require_path()?, node, input, value),
        Commands::Run => cmd_run(require_path()?).await,
        Commands::Repl { json } => cmd_repl(require_path()?, json).await,
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
/// Spaces in names are replaced with underscores so paths are CLI-friendly without quoting.
fn flatten_ops(items: &[OperationListItem], prefix: &str) -> Vec<(String, Operation)> {
    let mut result = Vec::new();
    for item in items {
        match item {
            OperationListItem::Category { name, operation_list_items } => {
                let slug = name.replace(' ', "_");
                let new_prefix = if prefix.is_empty() {
                    slug
                } else {
                    format!("{prefix}/{slug}")
                };
                result.extend(flatten_ops(operation_list_items, &new_prefix));
            }
            OperationListItem::Operation { operation } => {
                let op_name = operation.settings().name.replace(' ', "_");
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

    // Try short path first (case-insensitive, spaces normalized to underscores).
    let by_path: HashMap<String, Operation> =
        all_ops.iter().map(|(p, op)| (p.to_lowercase(), op.clone())).collect();
    let normalized = type_str.to_lowercase().replace(' ', "_");
    if let Some(op) = by_path.get(&normalized) {
        return Ok(op.clone());
    }

    // Try full variant name via serde round-trip.
    let json = format!("\"{}\"", type_str);
    if let Ok(op) = serde_json::from_str::<Operation>(&json) {
        return Ok(op);
    }

    Err(format!(
        "unknown operation '{}' — run `mangle show-ops` to see all types",
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

// ── Enum type helpers ─────────────────────────────────────────────────────

/// All enum-like value types that users can set via the CLI.
const ENUM_TYPE_NAMES: &[&str] = &[
    "BlendMode", "ColorSpace", "FilterType", "ImageType",
    "ColorFormat", "NoiseWorleyDistanceFunction", "TextHAlign", "TextVAlign",
];

/// Return the valid variant names for an enum-like value type, or None if unknown.
fn enum_variants(type_name: &str) -> Option<Vec<&'static str>> {
    match type_name {
        "BlendMode" => Some(vec![
            "Over", "Lerp", "Multiply", "Screen", "Overlay", "SoftLight", "HardLight",
            "ColorDodge", "ColorBurn", "Darken", "Lighten", "Difference", "Exclusion",
            "LinearBurn", "LinearDodge", "Divide", "Subtract",
        ]),
        "ColorSpace" => Some(vec![
            "Srgb", "RgbLinear", "Hsl", "Hsv", "Lch", "Xyz", "Lab", "Yuv", "Cmyk",
        ]),
        "FilterType" => Some(vec![
            "catmullrom", "gaussian", "lanczos3", "nearest", "triangle",
        ]),
        "ImageType" => Some(vec![
            "png", "jpg", "gif", "webp", "pnm", "tiff", "tga",
            "bmp", "ico", "hdr", "exr", "ff", "qoi",
        ]),
        "ColorFormat" => Some(vec![
            "Rgba32F", "Rgb32F", "Rgba16", "Rgb16", "GrayA16", "Gray16",
            "Rgba8", "Rgb8", "GrayA8", "Gray8",
        ]),
        "NoiseWorleyDistanceFunction" => Some(vec![
            "Chebyshev", "Euclidean", "EuclideanSquared", "Manhattan", "Quadratic",
        ]),
        "TextHAlign" => Some(vec!["Left", "Center", "Right"]),
        "TextVAlign" => Some(vec!["Top", "Middle", "Bottom"]),
        _ => None,
    }
}

/// Return the enum type name for a ValueType, if it's an enum type.
fn value_type_enum_name(vt: &ValueType) -> Option<&'static str> {
    match vt {
        ValueType::BlendMode => Some("BlendMode"),
        ValueType::ColorSpace => Some("ColorSpace"),
        ValueType::FilterType => Some("FilterType"),
        ValueType::ImageType => Some("ImageType"),
        ValueType::ColorFormat => Some("ColorFormat"),
        ValueType::NoiseWorleyDistanceFunction => Some("NoiseWorleyDistanceFunction"),
        ValueType::TextHAlign => Some("TextHAlign"),
        ValueType::TextVAlign => Some("TextVAlign"),
        _ => None,
    }
}

// ── Value display ─────────────────────────────────────────────────────────────

/// Return a concise human-readable representation of a `Value`.
fn display_value(value: &Value) -> String {
    match value {
        Value::DynamicImage { data, .. } => format!("<image {}x{}>", data.width(), data.height()),
        _ => serde_json::to_string(value).unwrap_or_else(|_| format!("{:?}", value)),
    }
}

// ── Inner (do_*) functions ───────────────────────────────────────────────────
//
// These operate on an in-memory `&mut Graph` and return structured results.
// Both the top-level `cmd_*` functions and the REPL dispatcher call these.

/// Build a structured info response for a graph.
/// If `filter_node` is Some, only include that node. If `compact`, omit descriptions and defaults.
fn do_info(graph: &Graph, filter_node: Option<&str>, compact: bool) -> Result<ReplResponse, String> {
    let mut nodes = serde_json::Map::new();
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    // If filtering by node, validate it exists.
    if let Some(nid) = filter_node {
        if !graph.nodes.contains_key(nid) {
            return Err(format!("node '{nid}' not found"));
        }
    }

    for node_id in &node_ids {
        if let Some(nid) = filter_node {
            if *node_id != nid { continue; }
        }
        let node = &graph.nodes[*node_id];

        // Operation type label.
        let type_label = match &node.node_type {
            mangler_core::node_type::NodeType::Operation { operation } => {
                serde_json::to_string(operation)
                    .unwrap_or_else(|_| format!("{:?}", operation))
                    .trim_matches('"')
                    .to_string()
            }
            mangler_core::node_type::NodeType::Subgraph { path, .. } => {
                format!("subgraph({})", path.display())
            }
        };

        // Inputs.
        let inputs: Vec<serde_json::Value> = node.inputs.iter().enumerate().map(|(i, input)| {
            let mut m = serde_json::Map::new();
            m.insert("index".to_string(), serde_json::json!(i));
            m.insert("name".to_string(), serde_json::json!(input.name));
            let vt = input.value.value_type();
            m.insert("type".to_string(), serde_json::json!(format!("{:?}", vt)));
            m.insert("value".to_string(), serde_json::json!(display_value(&input.value)));
            if !compact {
                m.insert("default_value".to_string(), serde_json::json!(display_value(&input.default_value)));
                if let Some(enum_name) = value_type_enum_name(&vt) {
                    if let Some(variants) = enum_variants(enum_name) {
                        m.insert("enum_values".to_string(), serde_json::json!(variants));
                    }
                }
            }
            if let Some((src_node, src_idx)) = &input.connection {
                m.insert("connection".to_string(), serde_json::json!(format!("{}:{}", src_node, src_idx)));
            }
            serde_json::Value::Object(m)
        }).collect();

        // Outputs.
        let outputs: Vec<serde_json::Value> = node.outputs.iter().enumerate().map(|(i, output)| {
            let mut m = serde_json::Map::new();
            m.insert("index".to_string(), serde_json::json!(i));
            m.insert("name".to_string(), serde_json::json!(output.name));
            m.insert("type".to_string(), serde_json::json!(format!("{:?}", output.value.value_type())));
            m.insert("value".to_string(), serde_json::json!(display_value(&output.value)));
            if let Some(conns) = &output.connection {
                let cs: Vec<String> = conns.iter().map(|(n, idx)| format!("{}:{}", n, idx)).collect();
                m.insert("connections".to_string(), serde_json::json!(cs));
            }
            serde_json::Value::Object(m)
        }).collect();

        let mut node_map = serde_json::Map::new();
        node_map.insert("name".to_string(), serde_json::json!(node.settings.name));
        if !compact {
            node_map.insert("description".to_string(), serde_json::json!(node.settings.description));
        }
        node_map.insert("type".to_string(), serde_json::json!(type_label));
        node_map.insert("inputs".to_string(), serde_json::json!(inputs));
        node_map.insert("outputs".to_string(), serde_json::json!(outputs));
        if node.is_error {
            node_map.insert("error".to_string(), serde_json::json!(node.error_message));
        }
        nodes.insert(node_id.to_string(), serde_json::Value::Object(node_map));
    }

    Ok(ReplResponse::ok(serde_json::json!({
        "name": graph.name,
        "id": graph.id,
        "node_count": graph.nodes.len(),
        "nodes": nodes,
    })))
}

/// Format graph info as human-readable text.
/// If `filter_node` is Some, only show that node. If `compact`, omit descriptions and defaults.
fn format_info_human(graph: &Graph, filter_node: Option<&str>, compact: bool) -> Result<String, String> {
    // Validate filter node exists.
    if let Some(nid) = filter_node {
        if !graph.nodes.contains_key(nid) {
            return Err(format!("node '{nid}' not found"));
        }
    }

    let mut out = String::new();
    out.push_str(&format!("graph: {} ({})\n", graph.name, graph.id));
    out.push_str(&format!("nodes: {}\n", graph.nodes.len()));

    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    for node_id in node_ids {
        if let Some(nid) = filter_node {
            if node_id != nid { continue; }
        }
        let node = &graph.nodes[node_id];

        let type_label = match &node.node_type {
            mangler_core::node_type::NodeType::Operation { operation } => {
                serde_json::to_string(operation)
                    .unwrap_or_else(|_| format!("{:?}", operation))
                    .trim_matches('"')
                    .to_string()
            }
            mangler_core::node_type::NodeType::Subgraph { path, .. } => {
                format!("subgraph({})", path.display())
            }
        };

        out.push_str(&format!("\n  [{}] {} ({})\n", node_id, node.settings.name, type_label));

        // Show description unless compact.
        if !compact && !node.settings.description.is_empty() {
            out.push_str(&format!("    \"{}\"\n", node.settings.description));
        }

        // Show error state if present.
        if node.is_error {
            if let Some(msg) = &node.error_message {
                out.push_str(&format!("    ERROR: {}\n", msg));
            }
        }

        for (i, input) in node.inputs.iter().enumerate() {
            let vt = input.value.value_type();
            let conn = if let Some((src_node, src_idx)) = &input.connection {
                format!(" <- {}:{}", src_node, src_idx)
            } else {
                String::new()
            };

            // Build type annotation with enum variants if applicable.
            let type_str = if !compact {
                if let Some(enum_name) = value_type_enum_name(&vt) {
                    if let Some(variants) = enum_variants(enum_name) {
                        format!("{}: {}", enum_name, variants.join("|"))
                    } else {
                        format!("{:?}", vt)
                    }
                } else {
                    format!("{:?}", vt)
                }
            } else {
                format!("{:?}", vt)
            };

            // Show default value if different from current and not compact.
            let default_str = if !compact {
                let cur = display_value(&input.value);
                let def = display_value(&input.default_value);
                if cur != def {
                    format!(" (default: {})", def)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            out.push_str(&format!(
                "    in[{}] {} ({}) = {}{}{}\n",
                i, input.name, type_str, display_value(&input.value), default_str, conn
            ));
        }

        for (i, output) in node.outputs.iter().enumerate() {
            let conn = if let Some(conns) = &output.connection {
                let s: Vec<String> = conns.iter().map(|(n, idx)| format!("{}:{}", n, idx)).collect();
                format!(" -> {}", s.join(", "))
            } else {
                String::new()
            };
            out.push_str(&format!(
                "    out[{}] {} ({:?}) = {}{}\n",
                i, output.name, output.value.value_type(), display_value(&output.value), conn
            ));
        }
    }
    Ok(out)
}

/// Collect top-level categories with counts from the flattened ops list.
fn collect_categories(all_ops: &[(String, Operation)]) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (path, _) in all_ops {
        let cat = path.split('/').next().unwrap_or(path).to_string();
        *counts.entry(cat).or_insert(0) += 1;
    }
    let mut cats: Vec<(String, usize)> = counts.into_iter().collect();
    cats.sort_by(|a, b| a.0.cmp(&b.0));
    cats
}

/// Build the show-ops response data. Supports `--group` with category fallback and `--search`.
fn do_show_ops(group: Option<&str>, search: Option<&str>) -> ReplResponse {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_filter = search.unwrap_or("").to_lowercase().replace(' ', "_");

    let mut ops = Vec::new();
    for (path, op) in &all_ops {
        // Group filter.
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let description = &op.settings().description;

        // Search filter: match against path, variant, or description.
        if !search_filter.is_empty() {
            let haystack = format!("{} {} {}", path, variant, description).to_lowercase();
            if !haystack.contains(&search_filter) {
                continue;
            }
        }

        let inputs = op.create_inputs();
        let outputs = op.create_outputs();

        let in_json: Vec<serde_json::Value> = inputs.iter().map(|i| {
            let vt = i.value.value_type();
            let mut obj = serde_json::json!({
                "name": i.name,
                "type": format!("{:?}", vt),
            });
            if i.accepts_any_type {
                obj["accepts_any_type"] = serde_json::json!(true);
            } else {
                // Show other types that can connect to this input, excluding self and Trigger.
                let accepts: Vec<String> = vt.valid_conversions_from().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| format!("{:?}", t))
                    .collect();
                if !accepts.is_empty() {
                    obj["accepts"] = serde_json::json!(accepts);
                }
            }
            obj
        }).collect();

        let out_json: Vec<serde_json::Value> = outputs.iter().map(|o| {
            let vt = o.value.value_type();
            let mut obj = serde_json::json!({
                "name": o.name,
                "type": format!("{:?}", vt),
            });
            // Show types this output can convert to, excluding self and Trigger.
            let converts_to: Vec<String> = vt.valid_conversions().iter()
                .filter(|t| **t != vt && **t != ValueType::Trigger)
                .map(|t| format!("{:?}", t))
                .collect();
            if !converts_to.is_empty() {
                obj["converts_to"] = serde_json::json!(converts_to);
            }
            obj
        }).collect();

        ops.push(serde_json::json!({
            "path": path,
            "variant": variant,
            "description": description,
            "inputs": in_json,
            "outputs": out_json,
        }));
    }

    // If group was specified but no ops matched, show categories as fallback.
    if ops.is_empty() && !group_filter.is_empty() && search_filter.is_empty() {
        let cats = collect_categories(&all_ops);
        return ReplResponse::ok(serde_json::json!({
            "operations": [],
            "categories": cats.iter().map(|(name, count)| serde_json::json!({"name": name, "count": count})).collect::<Vec<_>>(),
        }));
    }

    ReplResponse::ok(serde_json::json!({ "operations": ops }))
}

/// Format show-ops as human-readable text. Supports `--group` with category fallback and `--search`.
fn format_show_ops_human(group: Option<&str>, search: Option<&str>) -> String {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_filter = search.unwrap_or("").to_lowercase().replace(' ', "_");
    let mut out = String::new();
    let mut count = 0;

    for (path, op) in &all_ops {
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let description = &op.settings().description;

        if !search_filter.is_empty() {
            let haystack = format!("{} {} {}", path, variant, description).to_lowercase();
            if !haystack.contains(&search_filter) {
                continue;
            }
        }

        let inputs = op.create_inputs();
        let outputs = op.create_outputs();

        let in_str: Vec<String> = inputs.iter()
            .map(|i| {
                let vt = i.value.value_type();
                if i.accepts_any_type {
                    format!("{}({:?}, accepts: any)", i.name, vt)
                } else {
                    let accepts: Vec<String> = vt.valid_conversions_from().iter()
                        .filter(|t| **t != vt && **t != ValueType::Trigger)
                        .map(|t| format!("{:?}", t))
                        .collect();
                    if accepts.is_empty() {
                        format!("{}({:?})", i.name, vt)
                    } else {
                        format!("{}({:?}, accepts: {})", i.name, vt, accepts.join(", "))
                    }
                }
            })
            .collect();
        let out_str: Vec<String> = outputs.iter()
            .map(|o| {
                let vt = o.value.value_type();
                let converts_to: Vec<String> = vt.valid_conversions().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| format!("{:?}", t))
                    .collect();
                if converts_to.is_empty() {
                    format!("{}({:?})", o.name, vt)
                } else {
                    format!("{}({:?}, converts to: {})", o.name, vt, converts_to.join(", "))
                }
            })
            .collect();

        out.push_str(&format!(
            "{:<45} ({})  in: [{}]  out: [{}]\n",
            path, variant, in_str.join(", "), out_str.join(", ")
        ));
        count += 1;
    }

    // If group was specified but no ops matched, show categories as fallback.
    if count == 0 && !group_filter.is_empty() && search_filter.is_empty() {
        let cats = collect_categories(&all_ops);
        out.push_str("No operations match that group. Available categories:\n");
        for (name, cnt) in &cats {
            out.push_str(&format!("  {} ({})\n", name, cnt));
        }
    }

    out
}

/// Build the show-types response.
fn do_show_types(type_name: Option<&str>) -> ReplResponse {
    match type_name {
        None => {
            // List all enum type names.
            ReplResponse::ok(serde_json::json!({ "types": ENUM_TYPE_NAMES }))
        }
        Some(name) => {
            // Case-insensitive lookup.
            let matched = ENUM_TYPE_NAMES.iter().find(|t| t.eq_ignore_ascii_case(name));
            match matched {
                Some(canonical) => {
                    let variants = enum_variants(canonical).unwrap_or_default();
                    ReplResponse::ok(serde_json::json!({
                        "type": canonical,
                        "variants": variants,
                    }))
                }
                None => {
                    ReplResponse::error(format!(
                        "unknown type '{}'. Available types: {}",
                        name,
                        ENUM_TYPE_NAMES.join(", ")
                    ))
                }
            }
        }
    }
}

/// Format show-types as human-readable text.
fn format_show_types_human(type_name: Option<&str>) -> String {
    match type_name {
        None => {
            format!("{}\n", ENUM_TYPE_NAMES.join(", "))
        }
        Some(name) => {
            let matched = ENUM_TYPE_NAMES.iter().find(|t| t.eq_ignore_ascii_case(name));
            match matched {
                Some(canonical) => {
                    let variants = enum_variants(canonical).unwrap_or_default();
                    format!("{}\n", variants.join(", "))
                }
                None => {
                    format!(
                        "unknown type '{}'. Available types: {}\n",
                        name,
                        ENUM_TYPE_NAMES.join(", ")
                    )
                }
            }
        }
    }
}

/// Add a node to an in-memory graph. Returns the node ID.
async fn do_add_node(graph: &mut Graph, op_type: &str, id: Option<String>) -> Result<String, String> {
    let operation = resolve_op(op_type)?;
    let node_id = id.unwrap_or_else(get_id);
    graph.add_node(node_id.clone(), AddNodeType::Operation(operation), glam::Vec2::ZERO).await;
    Ok(node_id)
}

/// Remove a node from an in-memory graph. Returns the removed node ID.
async fn do_remove_node(graph: &mut Graph, id: &str) -> Result<String, String> {
    if !graph.nodes.contains_key(id) {
        return Err(format!("node '{id}' not found"));
    }
    graph.remove_node(id.to_string()).await;
    Ok(id.to_string())
}

/// Connect two nodes in an in-memory graph. Returns a description string.
/// Validates that both nodes and slot indices exist before connecting (9E).
async fn do_connect(graph: &mut Graph, from: &str, to: &str) -> Result<String, String> {
    let (output_node_id, output_index) = parse_slot(from)?;
    let (input_node_id, input_index) = parse_slot(to)?;

    // Validate source node and output index.
    let src_node = graph.nodes.get(&output_node_id)
        .ok_or_else(|| format!("source node '{}' not found", output_node_id))?;
    if output_index >= src_node.outputs.len() {
        return Err(format!(
            "output index {} out of range on node '{}' (has {} outputs)",
            output_index, output_node_id, src_node.outputs.len()
        ));
    }

    // Validate destination node and input index.
    let dst_node = graph.nodes.get(&input_node_id)
        .ok_or_else(|| format!("destination node '{}' not found", input_node_id))?;
    if input_index >= dst_node.inputs.len() {
        return Err(format!(
            "input index {} out of range on node '{}' (has {} inputs)",
            input_index, input_node_id, dst_node.inputs.len()
        ));
    }

    graph.add_connection(input_node_id, input_index, output_node_id, output_index).await;
    Ok(format!("connected {from} -> {to}"))
}

/// Disconnect a node input in an in-memory graph. Returns a description string.
async fn do_disconnect(graph: &mut Graph, node: &str, input: usize) -> Result<String, String> {
    if !graph.nodes.contains_key(node) {
        return Err(format!("node '{node}' not found"));
    }
    graph.remove_connection(node.to_string(), input).await;
    Ok(format!("disconnected {node}:{input}"))
}

/// Set an input value on a node in an in-memory graph. Returns a description string.
/// Validates node exists, index is in bounds, and provides helpful error messages for
/// enum types when JSON parse fails (9E).
fn do_set_input(graph: &mut Graph, node: &str, index: usize, value: &str) -> Result<String, String> {
    // Validate node exists.
    let n = graph.nodes.get(node)
        .ok_or_else(|| format!("node '{node}' not found"))?;

    // Validate input index is in bounds.
    if index >= n.inputs.len() {
        return Err(format!(
            "input index {} out of range on node '{}' (has {} inputs)",
            index, node, n.inputs.len()
        ));
    }

    // Parse value JSON, with enhanced error message for enum types.
    let parsed: Value = serde_json::from_str(value).map_err(|e| {
        let input = &n.inputs[index];
        let vt = input.value.value_type();
        if let Some(enum_name) = value_type_enum_name(&vt) {
            if let Some(variants) = enum_variants(enum_name) {
                return format!(
                    "input '{}' (index {}) on node '{}' expects {} -- valid values: {}. JSON error: {}",
                    input.name, index, node, enum_name, variants.join(", "), e
                );
            }
        }
        format!(
            "invalid value JSON for input '{}' (index {}) on node '{}' (expects {:?}): {}",
            input.name, index, node, vt, e
        )
    })?;

    graph.set_input(node.to_string(), index, parsed);
    Ok(format!("set {node}:{index} = {value}"))
}

/// Run the graph and return output values. Reports node errors (9E).
async fn do_run(graph: &mut Graph) -> ReplResponse {
    graph.run().await;

    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    let mut outputs = Vec::new();
    let mut errors = Vec::new();
    for node_id in &node_ids {
        let node = &graph.nodes[*node_id];
        if node.is_error {
            errors.push(serde_json::json!({
                "node": node_id,
                "error": node.error_message.as_deref().unwrap_or("unknown error"),
            }));
        }
        for (i, output) in node.outputs.iter().enumerate() {
            outputs.push(serde_json::json!({
                "node": node_id,
                "index": i,
                "type": format!("{:?}", output.value.value_type()),
                "value": display_value(&output.value),
            }));
        }
    }

    let mut data = serde_json::json!({ "outputs": outputs });
    if !errors.is_empty() {
        data["errors"] = serde_json::json!(errors);
    }
    ReplResponse::ok(data)
}

/// Format run results as human-readable text. Reports node errors (9E).
fn format_run_human(graph: &Graph) -> String {
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();
    let mut out = String::new();

    // Report errors first.
    for node_id in &node_ids {
        let node = &graph.nodes[*node_id];
        if node.is_error {
            let msg = node.error_message.as_deref().unwrap_or("unknown error");
            out.push_str(&format!("[{}] ERROR: {}\n", node_id, msg));
        }
    }

    for node_id in &node_ids {
        let node = &graph.nodes[*node_id];
        for (i, output) in node.outputs.iter().enumerate() {
            out.push_str(&format!(
                "[{}] out[{}] ({:?}) = {}\n",
                node_id, i, output.value.value_type(), display_value(&output.value)
            ));
        }
    }
    out
}

// ── Top-level commands ───────────────────────────────────────────────────────

/// `mangle new <path>` — create an empty graph file.
///
/// If the path does not end in `.json`, `.mangle.json` is appended automatically.
fn cmd_new(path: PathBuf) -> Result<(), String> {
    let path = if path.extension().map_or(false, |ext| ext == "json") {
        path
    } else {
        let mut name = path.as_os_str().to_os_string();
        name.push(".mangle.json");
        PathBuf::from(name)
    };
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

/// `mangle info <path> [--node <id>] [--compact]` — print graph structure.
fn cmd_info(path: PathBuf, node: Option<String>, compact: bool) -> Result<(), String> {
    let graph = load_graph(&path)?;
    let text = format_info_human(&graph, node.as_deref(), compact)?;
    print!("{}", text);
    Ok(())
}

/// `mangle show-ops [--group <prefix>] [--search <term>]` — show available operations.
fn cmd_show_ops(group: Option<String>, search: Option<String>) -> Result<(), String> {
    print!("{}", format_show_ops_human(group.as_deref(), search.as_deref()));
    Ok(())
}

/// `mangle show-types [<type_name>]` — show enum types or their variants.
fn cmd_show_types(type_name: Option<String>) -> Result<(), String> {
    print!("{}", format_show_types_human(type_name.as_deref()));
    Ok(())
}

/// `mangle show-values` — print JSON value format reference for set-input --value.
fn show_values_text() -> &'static str {
    concat!(
        "Value formats for set-input --value:\n",
        "\n",
        "  Bool            {\"Bool\":true}\n",
        "  Integer         {\"Integer\":42}\n",
        "  Decimal         {\"Decimal\":3.14}\n",
        "  Text            {\"Text\":\"hello\"}\n",
        "  Color           {\"Color\":{\"r\":1.0,\"g\":0.0,\"b\":0.0,\"a\":1.0}}\n",
        "  Path            {\"Path\":\"path/to/file.png\"}\n",
        "  FilterType      {\"FilterType\":\"lanczos3\"}          (run `show-types FilterType` for values)\n",
        "  ImageType       {\"ImageType\":\"png\"}                (run `show-types ImageType` for values)\n",
        "  ColorFormat     {\"ColorFormat\":\"Rgba8\"}             (run `show-types ColorFormat` for values)\n",
        "  BlendMode       {\"BlendMode\":\"Multiply\"}            (run `show-types BlendMode` for values)\n",
        "  ColorSpace      {\"ColorSpace\":\"Srgb\"}               (run `show-types ColorSpace` for values)\n",
        "  NoiseWorleyDistanceFunction  {\"NoiseWorleyDistanceFunction\":\"Euclidean\"}  (run `show-types ...` for values)\n",
        "  TextHAlign      {\"TextHAlign\":\"Left\"}               (run `show-types TextHAlign` for values)\n",
        "  TextVAlign      {\"TextVAlign\":\"Top\"}                (run `show-types TextVAlign` for values)\n",
    )
}

/// `mangle show-op <type>` — show detailed info for one operation type.
fn cmd_show_op(op_type: String) -> Result<(), String> {
    print!("{}", format_show_op_human(&op_type)?);
    Ok(())
}

/// Build a structured show-op response for one operation.
///
/// Returns JSON with name, description, variant, inputs (with types, defaults,
/// enum values, and accepted types), and outputs (with types and conversion targets).
fn do_show_op(op_type: &str) -> Result<ReplResponse, String> {
    let op = resolve_op(op_type)?;
    let settings = op.settings();
    let inputs = op.create_inputs();
    let outputs = op.create_outputs();

    let variant = serde_json::to_string(&op)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    let in_json: Vec<serde_json::Value> = inputs.iter().enumerate().map(|(i, input)| {
        let vt = input.value.value_type();
        let mut obj = serde_json::json!({
            "index": i,
            "name": input.name,
            "type": format!("{:?}", vt),
            "default": display_value(&input.value),
        });
        // Add enum variant list if this is an enum type.
        if let Some(enum_name) = value_type_enum_name(&vt) {
            if let Some(variants) = enum_variants(enum_name) {
                obj["enum_values"] = serde_json::json!(variants);
            }
        }
        // Show other types that can connect to this input.
        if input.accepts_any_type {
            obj["accepts_any_type"] = serde_json::json!(true);
        } else {
            let accepts: Vec<String> = vt.valid_conversions_from().iter()
                .filter(|t| **t != vt && **t != ValueType::Trigger)
                .map(|t| format!("{:?}", t))
                .collect();
            if !accepts.is_empty() {
                obj["accepts"] = serde_json::json!(accepts);
            }
        }
        obj
    }).collect();

    let out_json: Vec<serde_json::Value> = outputs.iter().enumerate().map(|(i, output)| {
        let vt = output.value.value_type();
        let mut obj = serde_json::json!({
            "index": i,
            "name": output.name,
            "type": format!("{:?}", vt),
        });
        let converts_to: Vec<String> = vt.valid_conversions().iter()
            .filter(|t| **t != vt && **t != ValueType::Trigger)
            .map(|t| format!("{:?}", t))
            .collect();
        if !converts_to.is_empty() {
            obj["converts_to"] = serde_json::json!(converts_to);
        }
        obj
    }).collect();

    Ok(ReplResponse::ok(serde_json::json!({
        "name": settings.name,
        "description": settings.description,
        "variant": variant,
        "inputs": in_json,
        "outputs": out_json,
    })))
}

/// Format show-op as human-readable text.
fn format_show_op_human(op_type: &str) -> Result<String, String> {
    let op = resolve_op(op_type)?;
    let settings = op.settings();
    let inputs = op.create_inputs();
    let outputs = op.create_outputs();

    let variant = serde_json::to_string(&op)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    let mut out = String::new();
    out.push_str(&format!("{} ({})\n", settings.name, variant));
    if !settings.description.is_empty() {
        out.push_str(&format!("  \"{}\"\n", settings.description));
    }
    out.push_str("\n  Inputs:\n");

    for (i, input) in inputs.iter().enumerate() {
        let vt = input.value.value_type();

        // Build type string with enum variants or accepts info.
        let type_str = if let Some(enum_name) = value_type_enum_name(&vt) {
            if let Some(variants) = enum_variants(enum_name) {
                format!("{}: {}", enum_name, variants.join("|"))
            } else {
                format!("{:?}", vt)
            }
        } else {
            let mut s = format!("{:?}", vt);
            if input.accepts_any_type {
                s.push_str(", accepts: any");
            } else {
                let accepts: Vec<String> = vt.valid_conversions_from().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| format!("{:?}", t))
                    .collect();
                if !accepts.is_empty() {
                    s.push_str(&format!(", accepts: {}", accepts.join(", ")));
                }
            }
            s
        };

        out.push_str(&format!(
            "    [{}] {} ({}) = {}\n",
            i, input.name, type_str, display_value(&input.value)
        ));
    }

    out.push_str("\n  Outputs:\n");
    for (i, output) in outputs.iter().enumerate() {
        let vt = output.value.value_type();
        let converts_to: Vec<String> = vt.valid_conversions().iter()
            .filter(|t| **t != vt && **t != ValueType::Trigger)
            .map(|t| format!("{:?}", t))
            .collect();
        let conv_str = if converts_to.is_empty() {
            String::new()
        } else {
            format!(", converts to: {}", converts_to.join(", "))
        };
        out.push_str(&format!(
            "    [{}] {} ({:?}{})\n",
            i, output.name, vt, conv_str
        ));
    }

    Ok(out)
}

/// `mangle add-node <path> --type <type> [--id <id>]` — add a node to the graph.
async fn cmd_add_node(path: PathBuf, op_type: String, id: Option<String>) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let node_id = do_add_node(&mut graph, &op_type, id).await?;
    save_graph(&graph, &path)?;
    println!("{node_id}");
    Ok(())
}

/// `mangle remove-node <path> --id <id>` — remove a node and its connections.
async fn cmd_remove_node(path: PathBuf, id: String) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let removed = do_remove_node(&mut graph, &id).await?;
    save_graph(&graph, &path)?;
    println!("removed {removed}");
    Ok(())
}

/// `mangle connect <path> --from <node:out> --to <node:in>` — connect two nodes.
async fn cmd_connect(path: PathBuf, from: String, to: String) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let msg = do_connect(&mut graph, &from, &to).await?;
    save_graph(&graph, &path)?;
    println!("{msg}");
    Ok(())
}

/// `mangle disconnect <path> --node <id> --input <n>` — remove a connection.
async fn cmd_disconnect(path: PathBuf, node: String, input: usize) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let msg = do_disconnect(&mut graph, &node, input).await?;
    save_graph(&graph, &path)?;
    println!("{msg}");
    Ok(())
}

/// `mangle set-input <path> --node <id> --input <n> --value <json>` — set an input value.
fn cmd_set_input(path: PathBuf, node: String, input: usize, value: String) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let msg = do_set_input(&mut graph, &node, input, &value)?;
    save_graph(&graph, &path)?;
    println!("{msg}");
    Ok(())
}

/// `mangle run <path>` — execute the graph and print all output values.
async fn cmd_run(path: PathBuf) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    do_run(&mut graph).await;
    save_graph(&graph, &path)?;
    print!("{}", format_run_human(&graph));
    Ok(())
}

// ── REPL ─────────────────────────────────────────────────────────────────────

/// `mangle repl <path> [--json]` — enter interactive REPL mode.
async fn cmd_repl(path: PathBuf, json_mode: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let mode = if json_mode { OutputMode::Json } else { OutputMode::Human };
    let mut stdout = std::io::stdout();

    // Emit initial greeting.
    let greeting = ReplResponse::ok_message(format!("loaded graph with {} nodes", graph.nodes.len()));
    emit(mode, &greeting, &mut stdout);

    if json_mode {
        // Plain stdin loop — no rustyline, no ANSI, no prompt.
        let stdin = std::io::stdin();
        let reader = std::io::BufReader::new(stdin.lock());
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim().is_empty() { continue; }
            let should_exit = process_repl_line(&mut graph, &path, &line, mode, &mut stdout).await;
            if should_exit { break; }
        }
    } else {
        // Interactive mode with rustyline.
        let mut rl = rustyline::DefaultEditor::new().map_err(|e| e.to_string())?;
        loop {
            match rl.readline("mangler> ") {
                Ok(line) => {
                    if line.trim().is_empty() { continue; }
                    let _ = rl.add_history_entry(&line);
                    let should_exit = process_repl_line(&mut graph, &path, &line, mode, &mut stdout).await;
                    if should_exit { break; }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    writeln!(stdout, "(type 'exit' or 'quit' to leave)").unwrap_or(());
                }
                Err(rustyline::error::ReadlineError::Eof) => break,
                Err(e) => {
                    emit(mode, &ReplResponse::error(e.to_string()), &mut stdout);
                    break;
                }
            }
        }
    }
    Ok(())
}

/// Parse and execute one REPL line. Returns `true` if the REPL should exit.
async fn process_repl_line(
    graph: &mut Graph,
    path: &PathBuf,
    line: &str,
    mode: OutputMode,
    writer: &mut dyn IoWrite,
) -> bool {
    // Split the input line into shell words (handles quoted strings).
    let words = match shell_words::split(line) {
        Ok(w) => w,
        Err(e) => {
            emit(mode, &ReplResponse::error(format!("parse error: {e}")), writer);
            return false;
        }
    };

    if words.is_empty() {
        return false;
    }

    // Parse with clap.
    let parsed = match ReplCli::try_parse_from(&words) {
        Ok(cli) => cli,
        Err(e) => {
            // Render the clap error without ANSI codes for consistency.
            let msg = e.render().to_string();
            emit(mode, &ReplResponse::error(msg), writer);
            return false;
        }
    };

    match parsed.command {
        ReplCommand::Exit | ReplCommand::Quit => {
            emit(mode, &ReplResponse::ok_message("goodbye"), writer);
            return true;
        }

        ReplCommand::Help => {
            let help = concat!(
                "Available commands:\n",
                "  info [--node <id>] [--compact]          Print graph structure\n",
                "  show-ops [--group <prefix>] [--search <term>]\n",
                "  show-types [<type_name>]                Show enum types/variants\n",
                "  show-values                             Show JSON value format reference\n",
                "  show-op <type>                          Show detailed operation info\n",
                "  add-node --type <type> [--id <id>] [--no-save]\n",
                "  remove-node --id <id> [--no-save]\n",
                "  connect --from <node:out> --to <node:in> [--no-save]\n",
                "  disconnect --node <id> --input <n> [--no-save]\n",
                "  set-input --node <id> --input <n> --value <json> [--no-save]\n",
                "  run [--no-save]\n",
                "  save                                    Save graph to disk\n",
                "  exit / quit                             Leave the REPL\n",
                "  help                                    Show this help\n",
            );
            emit(mode, &ReplResponse::ok_message(help.trim_end()), writer);
        }

        ReplCommand::Save => {
            match save_graph(graph, path) {
                Ok(()) => emit(mode, &ReplResponse::ok_message(format!("saved {}", path.display())), writer),
                Err(e) => emit(mode, &ReplResponse::error(e), writer),
            }
        }

        ReplCommand::Info { node, compact } => {
            if mode == OutputMode::Json {
                match do_info(graph, node.as_deref(), compact) {
                    Ok(resp) => emit(mode, &resp, writer),
                    Err(e) => emit(mode, &ReplResponse::error(e), writer),
                }
            } else {
                match format_info_human(graph, node.as_deref(), compact) {
                    Ok(text) => { let _ = write!(writer, "{text}"); let _ = writer.flush(); }
                    Err(e) => emit(mode, &ReplResponse::error(e), writer),
                }
            }
        }

        ReplCommand::ShowOps { group, search } => {
            if mode == OutputMode::Json {
                let resp = do_show_ops(group.as_deref(), search.as_deref());
                emit(mode, &resp, writer);
            } else {
                let text = format_show_ops_human(group.as_deref(), search.as_deref());
                let _ = write!(writer, "{text}");
                let _ = writer.flush();
            }
        }

        ReplCommand::ShowTypes { type_name } => {
            if mode == OutputMode::Json {
                let resp = do_show_types(type_name.as_deref());
                emit(mode, &resp, writer);
            } else {
                let text = format_show_types_human(type_name.as_deref());
                let _ = write!(writer, "{text}");
                let _ = writer.flush();
            }
        }

        ReplCommand::ShowValues => {
            if mode == OutputMode::Json {
                emit(mode, &ReplResponse::ok_message(show_values_text().trim_end()), writer);
            } else {
                let _ = write!(writer, "{}", show_values_text());
                let _ = writer.flush();
            }
        }

        ReplCommand::ShowOp { op_type } => {
            if mode == OutputMode::Json {
                match do_show_op(&op_type) {
                    Ok(resp) => emit(mode, &resp, writer),
                    Err(e) => emit(mode, &ReplResponse::error(e), writer),
                }
            } else {
                match format_show_op_human(&op_type) {
                    Ok(text) => { let _ = write!(writer, "{text}"); let _ = writer.flush(); }
                    Err(e) => emit(mode, &ReplResponse::error(e), writer),
                }
            }
        }

        ReplCommand::AddNode { op_type, id, no_save } => {
            match do_add_node(graph, &op_type, id).await {
                Ok(node_id) => {
                    if !no_save {
                        if let Err(e) = save_graph(graph, path) {
                            emit(mode, &ReplResponse::error(e), writer);
                            return false;
                        }
                    }
                    emit(mode, &ReplResponse::ok(serde_json::json!({ "message": &node_id, "node_id": &node_id })), writer);
                }
                Err(e) => emit(mode, &ReplResponse::error(e), writer),
            }
        }

        ReplCommand::RemoveNode { id, no_save } => {
            match do_remove_node(graph, &id).await {
                Ok(removed) => {
                    if !no_save {
                        if let Err(e) = save_graph(graph, path) {
                            emit(mode, &ReplResponse::error(e), writer);
                            return false;
                        }
                    }
                    emit(mode, &ReplResponse::ok_message(format!("removed {removed}")), writer);
                }
                Err(e) => emit(mode, &ReplResponse::error(e), writer),
            }
        }

        ReplCommand::Connect { from, to, no_save } => {
            match do_connect(graph, &from, &to).await {
                Ok(msg) => {
                    if !no_save {
                        if let Err(e) = save_graph(graph, path) {
                            emit(mode, &ReplResponse::error(e), writer);
                            return false;
                        }
                    }
                    emit(mode, &ReplResponse::ok_message(msg), writer);
                }
                Err(e) => emit(mode, &ReplResponse::error(e), writer),
            }
        }

        ReplCommand::Disconnect { node, input, no_save } => {
            match do_disconnect(graph, &node, input).await {
                Ok(msg) => {
                    if !no_save {
                        if let Err(e) = save_graph(graph, path) {
                            emit(mode, &ReplResponse::error(e), writer);
                            return false;
                        }
                    }
                    emit(mode, &ReplResponse::ok_message(msg), writer);
                }
                Err(e) => emit(mode, &ReplResponse::error(e), writer),
            }
        }

        ReplCommand::SetInput { node, input, value, no_save } => {
            match do_set_input(graph, &node, input, &value) {
                Ok(msg) => {
                    if !no_save {
                        if let Err(e) = save_graph(graph, path) {
                            emit(mode, &ReplResponse::error(e), writer);
                            return false;
                        }
                    }
                    emit(mode, &ReplResponse::ok_message(msg), writer);
                }
                Err(e) => emit(mode, &ReplResponse::error(e), writer),
            }
        }

        ReplCommand::Run { no_save } => {
            let resp = do_run(graph).await;
            if !no_save {
                if let Err(e) = save_graph(graph, path) {
                    emit(mode, &ReplResponse::error(e), writer);
                    return false;
                }
            }
            if mode == OutputMode::Json {
                emit(mode, &resp, writer);
            } else {
                let text = format_run_human(graph);
                let _ = write!(writer, "{text}");
                let _ = writer.flush();
            }
        }
    }

    false
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
