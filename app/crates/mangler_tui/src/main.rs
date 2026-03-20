//! `mangle` — CLI for the NodeMangler graph engine.
//!
//! Allows AI agents and terminal users to create, inspect, and execute node
//! graphs from the command line. Each command loads a graph JSON file, performs
//! one operation, saves it back, and prints a result.

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mangler_core::{
    graph::Graph, get_id, AddNodeType, GraphSaveData,
    operations::{operation_list, Operation, OperationListItem},
    value::{Value, ValueType},
};

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
  mangle graph.json run                         Execute and print outputs")]
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

// ── Formatting helpers ───────────────────────────────────────────────────────

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

/// Format run results as human-readable text. Reports node errors.
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
    graph.run().await;
    save_graph(&graph, &path)?;
    print!("{}", format_run_human(&graph));
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
