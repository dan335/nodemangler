//! `mangle` — CLI for the NodeMangler graph engine.
//!
//! Allows AI agents and terminal users to create, inspect, and execute node
//! graphs from the command line. Each command loads a graph JSON file, performs
//! one operation, saves it back, and prints a result.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mangler_core::{
    color::Color,
    float_image::FloatImage,
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
  mangle show-ops --compact                     One-line-per-op summary
  mangle show-ops --search blur                 Find operations by keyword
  mangle show-ops --group images/transform      Browse a category
  mangle show-op images/combine/blend           Detailed operation info
  mangle show-types blendmode                    List enum variants
  mangle show-values                            JSON value format reference
  mangle graph.json add-node --type images/combine/blend
  mangle graph.json set-input --node <id> --input 0 --value decimal:3.14
  mangle graph.json set-input --node <id> --input 0 --value decimal:1.0 --input 1 --value decimal:2.0
  mangle graph.json set-input --node <id> --input 0 --value color:1.0,0.0,0.0,1.0
  mangle graph.json set-input --node <id> --input 0 --value blendmode:Multiply
  mangle graph.json run                         Execute and print outputs
  mangle graph.json show-output --node <id>      Run and inspect one node's output
  mangle graph.json show-output --node <id> --stats          Image statistics
  mangle graph.json show-output --node <id> --sample 0,0     Pixel at (0,0)
  mangle graph.json show-output --node <id> --save out.png   Save image to file")]
struct Cli {
    /// Path to the graph JSON file (required for most commands, placed before the subcommand)
    path: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
    /// Output results as machine-readable JSON instead of human-readable text
    #[arg(long, global = true)]
    json: bool,
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
        /// One-line-per-op summary: path and brief description only (~2K tokens)
        #[arg(long)]
        compact: bool,
    },

    /// Show enum value types and their valid variants
    ShowTypes {
        /// Type name to show variants for (e.g. BlendMode). Omit to list all types.
        type_name: Option<String>,
    },

    /// Show value format reference for set-input --value (Type:value and JSON)
    ShowValues,

    /// Show detailed info for a single operation type (no graph file needed)
    ShowOp {
        /// Operation type: full variant name or short path (e.g. images/combine/blend)
        #[arg(id = "op_type")]
        op_type: String,
    },

    /// Add a node to a graph
    #[command(
        override_usage = "mangle <PATH> add-node [OPTIONS] --type <op_type>",
        after_help = "\
Examples:
  mangle g.json add-node --type images/combine/blend
  mangle g.json add-node --type numbers/arithmetic/add --id my_adder

Use `mangle show-ops` to browse available operation types.
Use `mangle show-ops --compact` for a quick summary."
    )]
    AddNode {
        /// Operation type: full variant name (OpNumberMathAdd) or short path (numbers/arithmetic/add)
        #[arg(long = "type", id = "op_type")]
        op_type: String,
        /// Node ID — used to reference this node in connect, set-input, etc. Auto-generated if omitted
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

    /// Set one or more literal values on node inputs (repeat --input/--value pairs for batch)
    #[command(override_usage = "mangle <PATH> set-input --node <NODE> --input <INPUT> --value <VALUE> [--input <INPUT> --value <VALUE> ...]")]
    SetInput {
        /// ID of the target node
        #[arg(long)]
        node: String,
        /// Zero-based input index (repeat for batch)
        #[arg(long, required = true)]
        input: Vec<usize>,
        /// Value in Type:value format (repeat for batch, paired with --input)
        #[arg(long, required = true)]
        value: Vec<String>,
    },

    /// Enable or disable a node (disabled nodes pass inputs through unchanged)
    #[command(override_usage = "mangle <PATH> set-enabled --node <NODE> --enabled <true|false>")]
    SetEnabled {
        /// ID of the target node
        #[arg(long)]
        node: String,
        /// true to enable, false to disable
        #[arg(long, num_args = 1, require_equals = false)]
        enabled: bool,
    },

    /// Execute the graph and print all node output values
    #[command(override_usage = "mangle <PATH> run")]
    Run,

    /// Run the graph and inspect a specific node's output (with optional image stats, pixel sampling, and save)
    #[command(
        override_usage = "mangle <PATH> show-output --node <NODE> [OPTIONS]",
        after_help = "\
Examples:
  mangle g.json show-output --node blur1
  mangle g.json show-output --node blur1 --output 0 --stats --json
  mangle g.json show-output --node blur1 --sample 0,0 --sample center --json
  mangle g.json show-output --node blur1 --save output.png
  mangle g.json show-output --node blur1 --stats --sample 128,128 --save out.png --json"
    )]
    ShowOutput {
        /// ID of the node to inspect
        #[arg(long)]
        node: String,
        /// Zero-based output index (default: all outputs)
        #[arg(long)]
        output: Option<usize>,
        /// Compute per-channel image statistics (min, max, mean, stddev, unique colors, transparency)
        #[arg(long)]
        stats: bool,
        /// Sample pixel values at coordinates: x,y or named positions (center, top-left, top-right, bottom-left, bottom-right)
        #[arg(long)]
        sample: Vec<String>,
        /// Save image output to a file (format from extension: .png, .jpg, .bmp, etc.) or write JSON for non-image values
        #[arg(long)]
        save: Option<PathBuf>,
    },
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let json_output = cli.json;
    let result = run(cli).await;

    if let Err(e) = result {
        if json_output {
            eprintln!("{}", serde_json::json!({"error": e}));
        } else {
            eprintln!("error: {e}");
        }
        std::process::exit(1);
    }
}

/// Dispatch the parsed CLI to the appropriate command handler.
async fn run(cli: Cli) -> Result<(), String> {
    let json = cli.json;

    // Extract the path, returning an error if it was not provided.
    let require_path = || -> Result<PathBuf, String> {
        cli.path.clone().ok_or_else(|| "a graph file path is required before this command (e.g. mangle graph.json <command>)".to_string())
    };

    match cli.command {
        Commands::New => cmd_new(require_path()?, json),
        Commands::Info { node, compact } => cmd_info(require_path()?, node, compact, json),
        Commands::ShowOps { group, search, compact } => cmd_show_ops(group, search, compact, json),
        Commands::ShowTypes { type_name } => cmd_show_types(type_name, json),
        Commands::ShowValues => cmd_show_values(json),
        Commands::ShowOp { op_type } => cmd_show_op(op_type, json),
        Commands::AddNode { op_type, id } => cmd_add_node(require_path()?, op_type, id, json).await,
        Commands::RemoveNode { id } => cmd_remove_node(require_path()?, id, json).await,
        Commands::Connect { from, to } => cmd_connect(require_path()?, from, to, json).await,
        Commands::Disconnect { node, input } => cmd_disconnect(require_path()?, node, input, json).await,
        Commands::SetInput { node, input, value } => cmd_set_input(require_path()?, node, input, value, json),
        Commands::SetEnabled { node, enabled } => cmd_set_enabled(require_path()?, node, enabled, json),
        Commands::Run => cmd_run(require_path()?, json).await,
        Commands::ShowOutput { node, output, stats, sample, save } => {
            cmd_show_output(require_path()?, node, output, stats, sample, save, json).await
        }
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

// ── Value type name helper ────────────────────────────────────────────────

/// Return the canonical lowercase CLI name for a ValueType.
fn value_type_name(vt: &ValueType) -> &'static str {
    match vt {
        ValueType::Bool => "bool",
        ValueType::Integer => "int",
        ValueType::Decimal => "decimal",
        ValueType::Text => "text",
        ValueType::Color => "color",
        ValueType::Path => "path",
        ValueType::Image => "image",
        ValueType::Trigger => "trigger",
        ValueType::BlendMode => "blendmode",
        ValueType::ColorSpace => "colorspace",
        ValueType::FilterType => "filtertype",
        ValueType::ImageType => "imagetype",
        ValueType::ColorFormat => "colorformat",
        ValueType::NoiseWorleyDistanceFunction => "worleydistance",
        ValueType::TextHAlign => "texthalign",
        ValueType::TextVAlign => "textvalign",
    }
}

// ── Enum type helpers ─────────────────────────────────────────────────────

/// All enum-like value types that users can set via the CLI.
/// Canonical lowercase enum type names shown in output.
const ENUM_TYPE_NAMES: &[&str] = &[
    "blendmode", "colorspace", "filtertype", "imagetype",
    "colorformat", "worleydistance", "texthalign", "textvalign",
];

/// Legacy PascalCase aliases accepted as input prefixes (mapped to canonical names).
const ENUM_TYPE_ALIASES: &[(&str, &str)] = &[
    ("BlendMode", "blendmode"),
    ("ColorSpace", "colorspace"),
    ("FilterType", "filtertype"),
    ("ImageType", "imagetype"),
    ("ColorFormat", "colorformat"),
    ("NoiseWorleyDistanceFunction", "worleydistance"),
    ("TextHAlign", "texthalign"),
    ("TextVAlign", "textvalign"),
];

/// Return the valid variant names for an enum-like value type, or None if unknown.
fn enum_variants(type_name: &str) -> Option<Vec<&'static str>> {
    match type_name.to_lowercase().as_str() {
        "blendmode" => Some(vec![
            "Over", "Lerp", "Multiply", "Screen", "Overlay", "SoftLight", "HardLight",
            "ColorDodge", "ColorBurn", "Darken", "Lighten", "Difference", "Exclusion",
            "LinearBurn", "LinearDodge", "Divide", "Subtract",
        ]),
        "colorspace" => Some(vec![
            "Srgb", "RgbLinear", "Hsl", "Hsv", "Lch", "Xyz", "Lab", "Yuv", "Cmyk",
        ]),
        "filtertype" => Some(vec![
            "catmullrom", "gaussian", "lanczos3", "nearest", "triangle",
        ]),
        "imagetype" => Some(vec![
            "png", "jpg", "gif", "webp", "pnm", "tiff", "tga",
            "bmp", "ico", "hdr", "exr", "ff", "qoi",
        ]),
        "colorformat" => Some(vec![
            "Rgba32F", "Rgb32F", "Rgba16", "Rgb16", "GrayA16", "Gray16",
            "Rgba8", "Rgb8", "GrayA8", "Gray8",
        ]),
        "worleydistance" | "noiseworleydistancefunction" => Some(vec![
            "Chebyshev", "Euclidean", "EuclideanSquared", "Manhattan", "Quadratic",
        ]),
        "texthalign" => Some(vec!["Left", "Center", "Right"]),
        "textvalign" => Some(vec!["Top", "Middle", "Bottom"]),
        _ => None,
    }
}

/// Return the enum type name for a ValueType, if it's an enum type.
fn value_type_enum_name(vt: &ValueType) -> Option<&'static str> {
    match vt {
        ValueType::BlendMode => Some("blendmode"),
        ValueType::ColorSpace => Some("colorspace"),
        ValueType::FilterType => Some("filtertype"),
        ValueType::ImageType => Some("imagetype"),
        ValueType::ColorFormat => Some("colorformat"),
        ValueType::NoiseWorleyDistanceFunction => Some("worleydistance"),
        ValueType::TextHAlign => Some("texthalign"),
        ValueType::TextVAlign => Some("textvalign"),
        _ => None,
    }
}

// ── Typed value parser ────────────────────────────────────────────────────────

/// Parse a `Type:value` string into a `Value`.
///
/// Supports two formats:
///   1. **Typed prefix** — `Type:value` where `Type` is a known prefix (see table below).
///      The split happens on the *first* colon, so values like `path:C:\foo` work correctly.
///   2. **JSON fallback** — any valid serde JSON representation of `Value` (e.g. `{"Decimal":3.14}`).
///
/// Type prefixes (case-insensitive for simple types, case-insensitive for enum types):
///   `bool`, `int`, `decimal`, `text`, `color` (r,g,b,a), `path`,
///   `blendmode`, `colorspace`, `filtertype`, `imagetype`, `colorformat`,
///   `worleydistance`, `texthalign`, `textvalign`.
fn parse_typed_value(s: &str) -> Result<Value, String> {
    // Try Type:value format — split on first colon.
    if let Some(colon_pos) = s.find(':') {
        let prefix = &s[..colon_pos];
        let rest = &s[colon_pos + 1..];

        // Simple types (case-insensitive prefix).
        match prefix.to_lowercase().as_str() {
            "bool" => {
                let b = rest.parse::<bool>().map_err(|_| {
                    format!("invalid bool value '{}' — expected true or false", rest)
                })?;
                return Ok(Value::Bool(b));
            }
            "int" => {
                let n = rest.parse::<i32>().map_err(|_| {
                    format!("invalid integer value '{}' — expected a 32-bit integer", rest)
                })?;
                return Ok(Value::Integer(n));
            }
            "decimal" => {
                let f = rest.parse::<f32>().map_err(|_| {
                    format!("invalid decimal value '{}' — expected a number", rest)
                })?;
                return Ok(Value::Decimal(f));
            }
            "text" => {
                return Ok(Value::Text(rest.to_string()));
            }
            "path" => {
                return Ok(Value::Path(PathBuf::from(rest)));
            }
            "color" => {
                let parts: Vec<&str> = rest.split(',').collect();
                if parts.len() != 4 {
                    return Err(format!(
                        "invalid color '{}' — expected 4 comma-separated floats (r,g,b,a), got {}",
                        rest,
                        parts.len()
                    ));
                }
                let vals: Vec<f32> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        p.trim().parse::<f32>().map_err(|_| {
                            format!("invalid color component [{}]: '{}' is not a number", i, p.trim())
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(Value::Color(Color {
                    r: vals[0],
                    g: vals[1],
                    b: vals[2],
                    a: vals[3],
                }));
            }
            _ => {}
        }

        // Enum types — match case-insensitively against canonical names and legacy aliases.
        if let Some(canonical) = resolve_enum_type_name(prefix) {
            // Validate the variant exists.
            let variants = enum_variants(canonical).unwrap_or_default();
            let matched_variant = variants.iter().find(|v| v.eq_ignore_ascii_case(rest));
            // Map canonical lowercase name to the serde PascalCase name for JSON deser.
            let serde_name = match canonical {
                "blendmode" => "BlendMode",
                "colorspace" => "ColorSpace",
                "filtertype" => "FilterType",
                "imagetype" => "ImageType",
                "colorformat" => "ColorFormat",
                "worleydistance" => "NoiseWorleyDistanceFunction",
                "texthalign" => "TextHAlign",
                "textvalign" => "TextVAlign",
                other => other,
            };
            match matched_variant {
                Some(variant) => {
                    // Deserialize via JSON: {"EnumType":"Variant"}
                    let json = format!("{{\"{serde_name}\":\"{variant}\"}}");
                    return serde_json::from_str::<Value>(&json).map_err(|e| {
                        format!("failed to parse {canonical}:{variant}: {e}")
                    });
                }
                None => {
                    return Err(format!(
                        "unknown {canonical} variant '{}' — valid values: {}",
                        rest,
                        variants.join(", ")
                    ));
                }
            }
        }
    }

    // JSON fallback.
    serde_json::from_str::<Value>(s).map_err(|e| {
        format!(
            "could not parse value '{}' — use Type:value format (e.g. decimal:3.14, bool:true, \
             color:1.0,0.0,0.0,1.0) or JSON (e.g. {{\"Decimal\":3.14}}). \
             Run `mangle show-values` for the full format reference. JSON error: {}",
            s, e
        )
    })
}

// ── Value display ─────────────────────────────────────────────────────────────

/// Return a concise human-readable representation of a `Value`.
fn display_value(value: &Value) -> String {
    match value {
        Value::Image { data, .. } => format!("<image {}x{}>", data.width(), data.height()),
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

        let disabled_tag = if !node.is_enabled { " [DISABLED]" } else { "" };
        out.push_str(&format!("\n  [{}] {}{} ({})\n", node_id, node.settings.name, disabled_tag, type_label));

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
                        value_type_name(&vt).to_string()
                    }
                } else {
                    value_type_name(&vt).to_string()
                }
            } else {
                value_type_name(&vt).to_string()
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
                "    out[{}] {} ({}) = {}{}\n",
                i, output.name, value_type_name(&output.value.value_type()), display_value(&output.value), conn
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

/// Score an operation against search terms for fuzzy ranked matching.
///
/// `haystack_parts` is `(path, variant, description)`, all lowercase.
/// `terms` are the lowercase search terms split on whitespace.
///
/// Scoring heuristic per term:
/// - Exact path segment match (term equals a `/`-delimited segment): +10 points
/// - Path contains term (substring): +5 points
/// - Variant exact match: +8 points
/// - Variant contains term: +4 points
/// - Description contains term: +2 points
///
/// If any term matches nothing, the total score is 0 (AND semantics).
fn score_op(haystack_parts: (&str, &str, &str), terms: &[String]) -> u32 {
    let (path, variant, description) = haystack_parts;
    let mut total: u32 = 0;

    for term in terms {
        let mut term_score: u32 = 0;

        // Exact path segment match (+10).
        for segment in path.split('/') {
            if segment == term {
                term_score += 10;
                break;
            }
        }

        // Path contains term (+5), only if no exact segment match.
        if term_score == 0 && path.contains(term.as_str()) {
            term_score += 5;
        }

        // Variant exact match (+8).
        if variant == term {
            term_score += 8;
        } else if variant.contains(term.as_str()) {
            // Variant contains term (+4).
            term_score += 4;
        }

        // Description contains term (+2).
        if description.contains(term.as_str()) {
            term_score += 2;
        }

        // If any term matches nothing, the whole op is excluded.
        if term_score == 0 {
            return 0;
        }

        total += term_score;
    }

    total
}

/// Format show-ops as human-readable text. Supports `--group` with category fallback and `--search`.
fn format_show_ops_human(group: Option<&str>, search: Option<&str>) -> String {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_raw = search.unwrap_or("");
    let terms: Vec<String> = search_raw.split_whitespace().map(|t| t.to_lowercase()).collect();
    let has_search = !terms.is_empty();
    let mut out = String::new();

    // Collect matching ops with scores.
    let mut scored_ops: Vec<(u32, String)> = Vec::new();

    for (path, op) in &all_ops {
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let description = &op.settings().description;

        let score = if has_search {
            let s = score_op(
                (&path.to_lowercase(), &variant.to_lowercase(), &description.to_lowercase()),
                &terms,
            );
            if s == 0 {
                continue;
            }
            s
        } else {
            0
        };

        let inputs = op.create_inputs();
        let outputs = op.create_outputs();

        let in_str: Vec<String> = inputs.iter()
            .map(|i| {
                let vt = i.value.value_type();
                if i.accepts_any_type {
                    format!("{}({}, accepts: any)", i.name, value_type_name(&vt))
                } else {
                    let accepts: Vec<String> = vt.valid_conversions_from().iter()
                        .filter(|t| **t != vt && **t != ValueType::Trigger)
                        .map(|t| value_type_name(t).to_string())
                        .collect();
                    if accepts.is_empty() {
                        format!("{}({})", i.name, value_type_name(&vt))
                    } else {
                        format!("{}({}, accepts: {})", i.name, value_type_name(&vt), accepts.join(", "))
                    }
                }
            })
            .collect();
        let out_str: Vec<String> = outputs.iter()
            .map(|o| {
                let vt = o.value.value_type();
                let converts_to: Vec<String> = vt.valid_conversions().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| value_type_name(t).to_string())
                    .collect();
                if converts_to.is_empty() {
                    format!("{}({})", o.name, value_type_name(&vt))
                } else {
                    format!("{}({}, converts to: {})", o.name, value_type_name(&vt), converts_to.join(", "))
                }
            })
            .collect();

        let score_suffix = if has_search {
            format!(" (score: {})", score)
        } else {
            String::new()
        };

        scored_ops.push((score, format!(
            "{:<45} ({})  in: [{}]  out: [{}]{}\n",
            path, variant, in_str.join(", "), out_str.join(", "), score_suffix
        )));
    }

    // Sort by score descending when searching.
    if has_search {
        scored_ops.sort_by(|a, b| b.0.cmp(&a.0));
    }

    let count = scored_ops.len();
    for (_, line) in scored_ops {
        out.push_str(&line);
    }

    // If group was specified but no ops matched, show categories as fallback.
    if count == 0 && !group_filter.is_empty() && !has_search {
        let cats = collect_categories(&all_ops);
        out.push_str("No operations match that group. Available categories:\n");
        for (name, cnt) in &cats {
            out.push_str(&format!("  {} ({})\n", name, cnt));
        }
    }

    // No results message when search matches nothing.
    if count == 0 && has_search {
        out.push_str(&format!(
            "No operations match search \"{}\". Try a broader search or use --group to browse categories.\n",
            search_raw,
        ));
    }

    out
}

/// Format show-ops as a compact one-line-per-op summary (path + description).
fn format_show_ops_compact_human(group: Option<&str>, search: Option<&str>) -> String {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_raw = search.unwrap_or("");
    let terms: Vec<String> = search_raw.split_whitespace().map(|t| t.to_lowercase()).collect();
    let has_search = !terms.is_empty();
    let mut scored_ops: Vec<(u32, String, String)> = Vec::new();

    for (path, op) in &all_ops {
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        let description = op.settings().description.clone();

        let score = if has_search {
            let s = score_op(
                (&path.to_lowercase(), &variant.to_lowercase(), &description.to_lowercase()),
                &terms,
            );
            if s == 0 { continue; }
            s
        } else {
            0
        };

        scored_ops.push((score, path.clone(), description));
    }

    if has_search {
        scored_ops.sort_by(|a, b| b.0.cmp(&a.0));
    }

    let count = scored_ops.len();
    let mut out = String::new();

    for (_, path, desc) in &scored_ops {
        if desc.is_empty() {
            out.push_str(&format!("{path}\n"));
        } else {
            out.push_str(&format!("{path:<45} {desc}\n"));
        }
    }

    if count == 0 && !group_filter.is_empty() && !has_search {
        let cats = collect_categories(&all_ops);
        out.push_str("No operations match that group. Available categories:\n");
        for (name, cnt) in &cats {
            out.push_str(&format!("  {} ({})\n", name, cnt));
        }
    }

    if count == 0 && has_search {
        out.push_str(&format!(
            "No operations match search \"{}\". Try a broader search or use --group to browse categories.\n",
            search_raw,
        ));
    }

    out
}

/// Format show-ops compact as JSON: array of {path, description}.
fn format_show_ops_compact_json(group: Option<&str>, search: Option<&str>) -> serde_json::Value {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_raw = search.unwrap_or("");
    let terms: Vec<String> = search_raw.split_whitespace().map(|t| t.to_lowercase()).collect();
    let has_search = !terms.is_empty();
    let mut scored_ops: Vec<(u32, serde_json::Value)> = Vec::new();

    for (path, op) in &all_ops {
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        let description = &op.settings().description;

        let score = if has_search {
            let s = score_op(
                (&path.to_lowercase(), &variant.to_lowercase(), &description.to_lowercase()),
                &terms,
            );
            if s == 0 { continue; }
            s
        } else {
            0
        };

        scored_ops.push((score, serde_json::json!({
            "path": path,
            "description": description,
        })));
    }

    if has_search {
        scored_ops.sort_by(|a, b| b.0.cmp(&a.0));
    }

    if scored_ops.is_empty() && has_search {
        return serde_json::json!({
            "matches": 0,
            "message": format!(
                "No operations match search \"{}\". Try a broader search or use --group to browse categories.",
                search_raw,
            ),
        });
    }

    let ops: Vec<serde_json::Value> = scored_ops.into_iter().map(|(_, v)| v).collect();
    serde_json::json!(ops)
}

/// Format show-types as human-readable text.
/// Resolve a type name to a canonical enum type name, accepting both canonical and legacy aliases.
fn resolve_enum_type_name(name: &str) -> Option<&'static str> {
    ENUM_TYPE_NAMES.iter().find(|t| t.eq_ignore_ascii_case(name)).copied()
        .or_else(|| ENUM_TYPE_ALIASES.iter().find(|(alias, _)| alias.eq_ignore_ascii_case(name)).map(|(_, canon)| *canon))
}

fn format_show_types_human(type_name: Option<&str>) -> String {
    match type_name {
        None => {
            format!("{}\n", ENUM_TYPE_NAMES.join(", "))
        }
        Some(name) => {
            match resolve_enum_type_name(name) {
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

    // Parse value using Type:value format first, then JSON fallback.
    let parsed: Value = parse_typed_value(value).map_err(|e| {
        let input = &n.inputs[index];
        let vt = input.value.value_type();
        if let Some(enum_name) = value_type_enum_name(&vt) {
            if let Some(variants) = enum_variants(enum_name) {
                return format!(
                    "input '{}' (index {}) on node '{}' expects {} -- valid values: {}. {}",
                    input.name, index, node, enum_name, variants.join(", "), e
                );
            }
        }
        format!(
            "input '{}' (index {}) on node '{}' (expects {}): {}",
            input.name, index, node, value_type_name(&vt), e
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
                "[{}] out[{}] ({}) = {}\n",
                node_id, i, value_type_name(&output.value.value_type()), display_value(&output.value)
            ));
        }
    }
    out
}

// ── JSON formatting helpers ──────────────────────────────────────────────────

/// Format a `Value` as a `serde_json::Value` for JSON output.
/// Images are represented as metadata objects instead of raw data.
fn json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::Image { data, .. } => {
            serde_json::json!({
                "type": "Image",
                "width": data.width(),
                "height": data.height()
            })
        }
        _ => serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!(format!("{:?}", value))),
    }
}

/// Format graph info as a JSON value.
fn format_info_json(graph: &Graph, filter_node: Option<&str>) -> Result<serde_json::Value, String> {
    if let Some(nid) = filter_node {
        if !graph.nodes.contains_key(nid) {
            return Err(format!("node '{nid}' not found"));
        }
    }

    let mut nodes = Vec::new();
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

        let inputs: Vec<serde_json::Value> = node.inputs.iter().enumerate().map(|(i, input)| {
            let vt = input.value.value_type();
            let mut obj = serde_json::json!({
                "index": i,
                "name": input.name,
                "type": value_type_name(&vt),
                "value": json_value(&input.value),
                "default_value": json_value(&input.default_value),
            });
            if let Some((src_node, src_idx)) = &input.connection {
                obj["connection"] = serde_json::json!({"node": src_node, "output": src_idx});
            }
            if let Some(enum_name) = value_type_enum_name(&vt) {
                obj["enum_type"] = serde_json::json!(enum_name);
                if let Some(variants) = enum_variants(enum_name) {
                    obj["enum_variants"] = serde_json::json!(variants);
                }
            }
            obj
        }).collect();

        let outputs: Vec<serde_json::Value> = node.outputs.iter().enumerate().map(|(i, output)| {
            let mut obj = serde_json::json!({
                "index": i,
                "name": output.name,
                "type": value_type_name(&output.value.value_type()),
                "value": json_value(&output.value),
            });
            if let Some(conns) = &output.connection {
                let c: Vec<serde_json::Value> = conns.iter()
                    .map(|(n, idx)| serde_json::json!({"node": n, "input": idx}))
                    .collect();
                obj["connections"] = serde_json::json!(c);
            }
            obj
        }).collect();

        let mut node_obj = serde_json::json!({
            "id": node_id,
            "name": node.settings.name,
            "type": type_label,
            "description": node.settings.description,
            "enabled": node.is_enabled,
            "inputs": inputs,
            "outputs": outputs,
        });
        if node.is_error {
            node_obj["error"] = serde_json::json!(node.error_message.as_deref().unwrap_or("unknown error"));
        }
        nodes.push(node_obj);
    }

    Ok(serde_json::json!({
        "graph_name": graph.name,
        "graph_id": graph.id,
        "node_count": graph.nodes.len(),
        "nodes": nodes,
    }))
}

/// Format show-ops as a JSON value.
fn format_show_ops_json(group: Option<&str>, search: Option<&str>) -> serde_json::Value {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_raw = search.unwrap_or("");
    let terms: Vec<String> = search_raw.split_whitespace().map(|t| t.to_lowercase()).collect();
    let has_search = !terms.is_empty();
    let mut scored_ops: Vec<(u32, serde_json::Value)> = Vec::new();

    for (path, op) in &all_ops {
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = serde_json::to_string(op)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let description = &op.settings().description;

        let score = if has_search {
            let s = score_op(
                (&path.to_lowercase(), &variant.to_lowercase(), &description.to_lowercase()),
                &terms,
            );
            if s == 0 {
                continue;
            }
            s
        } else {
            0
        };

        let inputs = op.create_inputs();
        let outputs = op.create_outputs();

        let in_json: Vec<serde_json::Value> = inputs.iter().map(|i| {
            let vt = i.value.value_type();
            let mut obj = serde_json::json!({
                "name": i.name,
                "type": value_type_name(&vt),
            });
            if i.accepts_any_type {
                obj["accepts"] = serde_json::json!("any");
            } else {
                let accepts: Vec<String> = vt.valid_conversions_from().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| value_type_name(t).to_string())
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
                "type": value_type_name(&vt),
            });
            let converts_to: Vec<String> = vt.valid_conversions().iter()
                .filter(|t| **t != vt && **t != ValueType::Trigger)
                .map(|t| value_type_name(t).to_string())
                .collect();
            if !converts_to.is_empty() {
                obj["converts_to"] = serde_json::json!(converts_to);
            }
            obj
        }).collect();

        let mut op_json = serde_json::json!({
            "path": path,
            "variant": variant,
            "description": description,
            "inputs": in_json,
            "outputs": out_json,
        });
        if has_search {
            op_json["score"] = serde_json::json!(score);
        }
        scored_ops.push((score, op_json));
    }

    // Sort by score descending when searching.
    if has_search {
        scored_ops.sort_by(|a, b| b.0.cmp(&a.0));
    }

    // No results message when search matches nothing.
    if scored_ops.is_empty() && has_search {
        return serde_json::json!({
            "matches": 0,
            "message": format!(
                "No operations match search \"{}\". Try a broader search or use --group to browse categories.",
                search_raw,
            ),
        });
    }

    let ops: Vec<serde_json::Value> = scored_ops.into_iter().map(|(_, v)| v).collect();
    serde_json::json!(ops)
}

/// Format show-op as a JSON value.
fn format_show_op_json(op_type: &str) -> Result<serde_json::Value, String> {
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
            "type": value_type_name(&vt),
            "default_value": json_value(&input.value),
        });
        if let Some(enum_name) = value_type_enum_name(&vt) {
            obj["enum_type"] = serde_json::json!(enum_name);
            if let Some(variants) = enum_variants(enum_name) {
                obj["enum_variants"] = serde_json::json!(variants);
            }
        } else {
            if input.accepts_any_type {
                obj["accepts"] = serde_json::json!("any");
            } else {
                let accepts: Vec<String> = vt.valid_conversions_from().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| value_type_name(t).to_string())
                    .collect();
                if !accepts.is_empty() {
                    obj["accepts"] = serde_json::json!(accepts);
                }
            }
        }
        obj
    }).collect();

    let out_json: Vec<serde_json::Value> = outputs.iter().enumerate().map(|(i, output)| {
        let vt = output.value.value_type();
        let mut obj = serde_json::json!({
            "index": i,
            "name": output.name,
            "type": value_type_name(&vt),
        });
        let converts_to: Vec<String> = vt.valid_conversions().iter()
            .filter(|t| **t != vt && **t != ValueType::Trigger)
            .map(|t| value_type_name(t).to_string())
            .collect();
        if !converts_to.is_empty() {
            obj["converts_to"] = serde_json::json!(converts_to);
        }
        obj
    }).collect();

    Ok(serde_json::json!({
        "name": settings.name,
        "variant": variant,
        "description": settings.description,
        "inputs": in_json,
        "outputs": out_json,
    }))
}

/// Format show-types as a JSON value.
fn format_show_types_json(type_name: Option<&str>) -> Result<serde_json::Value, String> {
    match type_name {
        None => Ok(serde_json::json!(ENUM_TYPE_NAMES)),
        Some(name) => {
            match resolve_enum_type_name(name) {
                Some(canonical) => {
                    let variants = enum_variants(canonical).unwrap_or_default();
                    Ok(serde_json::json!({
                        "type": canonical,
                        "variants": variants,
                    }))
                }
                None => Err(format!(
                    "unknown type '{}'. Available types: {}",
                    name,
                    ENUM_TYPE_NAMES.join(", ")
                ))
            }
        }
    }
}

/// Format show-values as a JSON value.
fn format_show_values_json() -> serde_json::Value {
    serde_json::json!({
        "bool": {"typed": "bool:true", "json": {"Bool": true}},
        "int": {"typed": "int:42", "json": {"Integer": 42}},
        "decimal": {"typed": "decimal:3.14", "json": {"Decimal": 3.14}},
        "text": {"typed": "text:hello", "json": {"Text": "hello"}},
        "color": {"typed": "color:1.0,0.0,0.0,1.0", "json": {"Color": {"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}}},
        "path": {"typed": "path:/some/file.png", "json": {"Path": "path/to/file.png"}},
        "filtertype": {"typed": "filtertype:lanczos3", "see": "show-types filtertype"},
        "imagetype": {"typed": "imagetype:png", "see": "show-types imagetype"},
        "colorformat": {"typed": "colorformat:Rgba8", "see": "show-types colorformat"},
        "blendmode": {"typed": "blendmode:Multiply", "see": "show-types blendmode"},
        "colorspace": {"typed": "colorspace:Srgb", "see": "show-types colorspace"},
        "worleydistance": {"typed": "worleydistance:Euclidean", "see": "show-types worleydistance"},
        "texthalign": {"typed": "texthalign:Left", "see": "show-types texthalign"},
        "textvalign": {"typed": "textvalign:Top", "see": "show-types textvalign"},
    })
}

/// Format run results as a JSON value.
fn format_run_json(graph: &Graph) -> serde_json::Value {
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();
    let mut errors = Vec::new();
    let mut outputs = Vec::new();

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
                "type": value_type_name(&output.value.value_type()),
                "value": json_value(&output.value),
            }));
        }
    }

    serde_json::json!({
        "errors": errors,
        "outputs": outputs,
    })
}

// ── Top-level commands ───────────────────────────────────────────────────────

/// `mangle new <path>` — create an empty graph file.
///
/// If the path does not end in `.json`, `.mangle.json` is appended automatically.
fn cmd_new(path: PathBuf, json_output: bool) -> Result<(), String> {
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
    let file_json = serde_json::to_string_pretty(&save_data).map_err(|e| e.to_string())?;
    std::fs::write(&path, file_json).map_err(|e| e.to_string())?;
    if json_output {
        println!("{}", serde_json::json!({
            "path": path.display().to_string(),
            "id": save_data.id,
            "name": save_data.name,
        }));
    } else {
        println!("created {}", path.display());
    }
    Ok(())
}

/// `mangle info <path> [--node <id>] [--compact]` — print graph structure.
fn cmd_info(path: PathBuf, node: Option<String>, compact: bool, json_output: bool) -> Result<(), String> {
    let graph = load_graph(&path)?;
    if json_output {
        let val = format_info_json(&graph, node.as_deref())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
    } else {
        let text = format_info_human(&graph, node.as_deref(), compact)?;
        print!("{}", text);
    }
    Ok(())
}

/// `mangle show-ops [--group <prefix>] [--search <term>] [--compact]` — show available operations.
fn cmd_show_ops(group: Option<String>, search: Option<String>, compact: bool, json_output: bool) -> Result<(), String> {
    if json_output {
        if compact {
            let val = format_show_ops_compact_json(group.as_deref(), search.as_deref());
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
        } else {
            let val = format_show_ops_json(group.as_deref(), search.as_deref());
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
        }
    } else if compact {
        print!("{}", format_show_ops_compact_human(group.as_deref(), search.as_deref()));
    } else {
        print!("{}", format_show_ops_human(group.as_deref(), search.as_deref()));
    }
    Ok(())
}

/// `mangle show-types [<type_name>]` — show enum types or their variants.
fn cmd_show_types(type_name: Option<String>, json_output: bool) -> Result<(), String> {
    if json_output {
        let val = format_show_types_json(type_name.as_deref())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
    } else {
        print!("{}", format_show_types_human(type_name.as_deref()));
    }
    Ok(())
}

/// `mangle show-values` — print value format reference for set-input --value.
fn show_values_text() -> &'static str {
    concat!(
        "Value formats for set-input --value (type:value — no quoting needed):\n",
        "\n",
        "  bool:true                             bool\n",
        "  int:42                                int\n",
        "  decimal:3.14                          decimal\n",
        "  text:hello                            text (everything after first colon)\n",
        "  color:1.0,0.0,0.0,1.0                color (r,g,b,a floats)\n",
        "  path:/some/file.png                   path (everything after first colon)\n",
        "  blendmode:Multiply                    (run `show-types blendmode` for values)\n",
        "  colorspace:Srgb                       (run `show-types colorspace` for values)\n",
        "  filtertype:lanczos3                   (run `show-types filtertype` for values)\n",
        "  imagetype:png                         (run `show-types imagetype` for values)\n",
        "  colorformat:Rgba8                     (run `show-types colorformat` for values)\n",
        "  worleydistance:Euclidean              (run `show-types worleydistance` for values)\n",
        "  texthalign:Left                       (run `show-types texthalign` for values)\n",
        "  textvalign:Top                        (run `show-types textvalign` for values)\n",
        "\n",
        "  Legacy JSON also works: {\"Decimal\":3.14}, {\"Color\":{\"r\":1,\"g\":0,\"b\":0,\"a\":1}}\n",
    )
}

/// `mangle show-values [--json]` — print value format reference.
fn cmd_show_values(json_output: bool) -> Result<(), String> {
    if json_output {
        println!("{}", serde_json::to_string_pretty(&format_show_values_json()).unwrap());
    } else {
        print!("{}", show_values_text());
    }
    Ok(())
}

/// `mangle show-op <type>` — show detailed info for one operation type.
fn cmd_show_op(op_type: String, json_output: bool) -> Result<(), String> {
    if json_output {
        let val = format_show_op_json(&op_type)?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
    } else {
        print!("{}", format_show_op_human(&op_type)?);
    }
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
                value_type_name(&vt).to_string()
            }
        } else {
            let mut s = value_type_name(&vt).to_string();
            if input.accepts_any_type {
                s.push_str(", accepts: any");
            } else {
                let accepts: Vec<String> = vt.valid_conversions_from().iter()
                    .filter(|t| **t != vt && **t != ValueType::Trigger)
                    .map(|t| value_type_name(t).to_string())
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
            .map(|t| value_type_name(t).to_string())
            .collect();
        let conv_str = if converts_to.is_empty() {
            String::new()
        } else {
            format!(", converts to: {}", converts_to.join(", "))
        };
        out.push_str(&format!(
            "    [{}] {} ({}{})\n",
            i, output.name, value_type_name(&vt), conv_str
        ));
    }

    Ok(out)
}

/// `mangle add-node <path> --type <type> [--id <id>]` — add a node to the graph.
async fn cmd_add_node(path: PathBuf, op_type: String, id: Option<String>, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let node_id = do_add_node(&mut graph, &op_type, id).await?;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"node_id": node_id}));
    } else {
        println!("{node_id}");
    }
    Ok(())
}

/// `mangle remove-node <path> --id <id>` — remove a node and its connections.
async fn cmd_remove_node(path: PathBuf, id: String, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let removed = do_remove_node(&mut graph, &id).await?;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"removed": removed}));
    } else {
        println!("removed {removed}");
    }
    Ok(())
}

/// `mangle connect <path> --from <node:out> --to <node:in>` — connect two nodes.
async fn cmd_connect(path: PathBuf, from: String, to: String, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let _msg = do_connect(&mut graph, &from, &to).await?;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"from": from, "to": to}));
    } else {
        println!("connected {from} -> {to}");
    }
    Ok(())
}

/// `mangle disconnect <path> --node <id> --input <n>` — remove a connection.
async fn cmd_disconnect(path: PathBuf, node: String, input: usize, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let _msg = do_disconnect(&mut graph, &node, input).await?;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"node": node, "input": input}));
    } else {
        println!("disconnected {node}:{input}");
    }
    Ok(())
}

/// `mangle set-input <path> --node <id> --input <n> --value <v> [...]` — set one or more input values.
///
/// Accepts repeating `--input`/`--value` pairs for batch operation with a single
/// load/save cycle. Fails fast on the first error.
fn cmd_set_input(path: PathBuf, node: String, inputs: Vec<usize>, values: Vec<String>, json_output: bool) -> Result<(), String> {
    if inputs.len() != values.len() {
        return Err(format!(
            "mismatched --input/--value counts: got {} input(s) and {} value(s) — each --input must be paired with a --value",
            inputs.len(), values.len()
        ));
    }

    let mut graph = load_graph(&path)?;

    // Apply all input/value pairs (fail fast on first error).
    let mut results: Vec<(usize, String)> = Vec::with_capacity(inputs.len());
    for (idx, val) in inputs.iter().zip(values.iter()) {
        do_set_input(&mut graph, &node, *idx, val)?;
        results.push((*idx, val.clone()));
    }

    save_graph(&graph, &path)?;

    if json_output {
        let entries: Vec<serde_json::Value> = results.iter().map(|(idx, val)| {
            let parsed_val = parse_typed_value(val).ok();
            let json_val = parsed_val
                .and_then(|v| serde_json::to_value(&v).ok())
                .unwrap_or(serde_json::json!(val));
            serde_json::json!({"input": idx, "value": json_val})
        }).collect();
        println!("{}", serde_json::json!({"node": node, "results": entries}));
    } else {
        for (idx, val) in &results {
            println!("set {node}:{idx} = {val}");
        }
    }
    Ok(())
}

/// `mangle set-enabled <path> --node <id> --enabled <bool>` — enable or disable a node.
fn cmd_set_enabled(path: PathBuf, node: String, enabled: bool, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let n = graph.nodes.get_mut(&node)
        .ok_or_else(|| format!("node '{node}' not found"))?;
    n.is_enabled = enabled;
    n.is_dirty = true;
    n.cached_input_hash = None;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"node": node, "enabled": enabled}));
    } else {
        let state = if enabled { "enabled" } else { "disabled" };
        println!("{state} {node}");
    }
    Ok(())
}

// ── Image statistics helpers ──────────────────────────────────────────────

/// Per-channel statistics for an image.
struct ChannelStats {
    min: f32,
    max: f32,
    mean: f32,
    stddev: f32,
}

/// Compute per-channel (R, G, B, A) statistics for an image.
///
/// Converts the FloatImage to RGBA f32 for uniform 4-channel analysis.
fn compute_image_stats(img: &FloatImage) -> Vec<(&'static str, ChannelStats)> {
    let dynamic = img.to_dynamic();
    let rgba = dynamic.to_rgba32f();
    let pixels: Vec<&[f32]> = rgba.as_raw().chunks(4).collect();
    let n = pixels.len() as f64;
    if n == 0.0 {
        return vec![
            ("r", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
            ("g", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
            ("b", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
            ("a", ChannelStats { min: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 }),
        ];
    }

    let mut result = Vec::with_capacity(4);
    for (ch, name) in ["r", "g", "b", "a"].iter().enumerate() {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut sum = 0.0_f64;
        for px in &pixels {
            let v = px[ch];
            if v < min { min = v; }
            if v > max { max = v; }
            sum += v as f64;
        }
        let mean = sum / n;
        let mut var_sum = 0.0_f64;
        for px in &pixels {
            let diff = px[ch] as f64 - mean;
            var_sum += diff * diff;
        }
        let stddev = (var_sum / n).sqrt();
        result.push((*name, ChannelStats {
            min,
            max,
            mean: mean as f32,
            stddev: stddev as f32,
        }));
    }
    result
}

/// Check whether an image has any transparent pixels (alpha < 1.0).
fn has_transparency(img: &FloatImage) -> bool {
    let rgba = img.to_rgba8();
    rgba.pixels().any(|p| p.0[3] < 255)
}

/// Count unique colors in an image (RGBA8).
fn count_unique_colors(img: &FloatImage) -> usize {
    let rgba = img.to_rgba8();
    let colors: HashSet<[u8; 4]> = rgba.pixels().map(|p| p.0).collect();
    colors.len()
}

/// Resolve a sample coordinate string to (x, y) given image dimensions.
/// Accepts "x,y" or named positions: center, top-left, top-right, bottom-left, bottom-right.
fn resolve_sample_coord(s: &str, w: u32, h: u32) -> Result<(u32, u32), String> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "center" => Ok((w / 2, h / 2)),
        "top_left" => Ok((0, 0)),
        "top_right" => Ok((w.saturating_sub(1), 0)),
        "bottom_left" => Ok((0, h.saturating_sub(1))),
        "bottom_right" => Ok((w.saturating_sub(1), h.saturating_sub(1))),
        _ => {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "invalid sample '{}' — expected x,y or a named position (center, top-left, top-right, bottom-left, bottom-right)",
                    s
                ));
            }
            let x: u32 = parts[0].trim().parse().map_err(|_| format!("invalid x coordinate in '{}'", s))?;
            let y: u32 = parts[1].trim().parse().map_err(|_| format!("invalid y coordinate in '{}'", s))?;
            if x >= w || y >= h {
                return Err(format!("sample ({},{}) out of bounds for {}x{} image", x, y, w, h));
            }
            Ok((x, y))
        }
    }
}

/// Sample a pixel from an image at (x, y), returning RGBA floats.
fn sample_pixel(img: &FloatImage, x: u32, y: u32) -> [f32; 4] {
    let dynamic = img.to_dynamic();
    let rgba = dynamic.to_rgba32f();
    let px = rgba.get_pixel(x, y);
    [px.0[0], px.0[1], px.0[2], px.0[3]]
}

/// Save an image to a file, inferring the format from the file extension.
///
/// Converts the FloatImage to a DynamicImage for file I/O.
fn save_image_to_file(img: &FloatImage, path: &PathBuf) -> Result<(), String> {
    let dynamic = img.to_dynamic();
    dynamic.save(path).map_err(|e| format!("failed to save image: {}", e))
}

// ── show-output formatting ───────────────────────────────────────────────

/// Format a single output's show-output result as JSON.
fn format_show_output_json(
    node_id: &str,
    output_index: usize,
    output_name: &str,
    value: &Value,
    stats: bool,
    samples: &[(String, u32, u32)],
    save_path: Option<&PathBuf>,
) -> Result<serde_json::Value, String> {
    let vt = value.value_type();
    let mut obj = serde_json::json!({
        "index": output_index,
        "name": output_name,
        "type": value_type_name(&vt),
    });

    match value {
        Value::Image { data, .. } => {
            let (w, h) = data.dimensions();
            obj["width"] = serde_json::json!(w);
            obj["height"] = serde_json::json!(h);

            // Compute image statistics if requested.
            if stats {
                let channel_stats = compute_image_stats(data);
                let mut stats_obj = serde_json::Map::new();
                for (name, cs) in &channel_stats {
                    stats_obj.insert(name.to_string(), serde_json::json!({
                        "min": (cs.min * 1000.0).round() / 1000.0,
                        "max": (cs.max * 1000.0).round() / 1000.0,
                        "mean": (cs.mean * 1000.0).round() / 1000.0,
                        "stddev": (cs.stddev * 1000.0).round() / 1000.0,
                    }));
                }
                obj["stats"] = serde_json::Value::Object(stats_obj);
                obj["has_transparency"] = serde_json::json!(has_transparency(data));
                obj["unique_colors"] = serde_json::json!(count_unique_colors(data));
            }

            // Sample pixels if requested.
            if !samples.is_empty() {
                let mut samples_obj = serde_json::Map::new();
                for (label, x, y) in samples {
                    let px = sample_pixel(data, *x, *y);
                    let rounded: Vec<f32> = px.iter().map(|v| (v * 1000.0).round() / 1000.0).collect();
                    samples_obj.insert(label.clone(), serde_json::json!(rounded));
                }
                obj["samples"] = serde_json::Value::Object(samples_obj);
            }

            // Save image if requested.
            if let Some(path) = save_path {
                save_image_to_file(data, path)?;
                obj["saved_to"] = serde_json::json!(path.display().to_string());
            }
        }
        _ => {
            obj["value"] = json_value(value);

            // Save non-image value to file as JSON if requested.
            if let Some(path) = save_path {
                let json_str = serde_json::to_string_pretty(&json_value(value))
                    .map_err(|e| format!("failed to serialize value: {}", e))?;
                std::fs::write(path, json_str).map_err(|e| format!("failed to write file: {}", e))?;
                obj["saved_to"] = serde_json::json!(path.display().to_string());
            }
        }
    }

    Ok(serde_json::json!({
        "node": node_id,
        "output": obj,
    }))
}

/// Format a single output's show-output result as human-readable text.
fn format_show_output_human(
    node_id: &str,
    output_index: usize,
    _output_name: &str,
    value: &Value,
    stats: bool,
    samples: &[(String, u32, u32)],
    save_path: Option<&PathBuf>,
) -> Result<String, String> {
    let vt = value.value_type();
    let mut out = String::new();

    match value {
        Value::Image { data, .. } => {
            let (w, h) = data.dimensions();
            out.push_str(&format!(
                "[{}] out[{}] ({}) = <image {}x{}>\n",
                node_id, output_index, value_type_name(&vt), w, h
            ));

            // Show stats.
            if stats {
                let channel_stats = compute_image_stats(data);
                for (name, cs) in &channel_stats {
                    out.push_str(&format!(
                        "  {}: min={:.3} max={:.3} mean={:.3} stddev={:.3}\n",
                        name, cs.min, cs.max, cs.mean, cs.stddev
                    ));
                }
                out.push_str(&format!("  has_transparency: {}\n", has_transparency(data)));
                out.push_str(&format!("  unique_colors: {}\n", count_unique_colors(data)));
            }

            // Show samples.
            for (label, x, y) in samples {
                let px = sample_pixel(data, *x, *y);
                out.push_str(&format!(
                    "  sample {}: [{:.3}, {:.3}, {:.3}, {:.3}]\n",
                    label, px[0], px[1], px[2], px[3]
                ));
            }

            // Save image.
            if let Some(path) = save_path {
                save_image_to_file(data, path)?;
                out.push_str(&format!("  saved to {} ({}x{})\n", path.display(), w, h));
            }
        }
        _ => {
            out.push_str(&format!(
                "[{}] out[{}] ({}) = {}\n",
                node_id, output_index, value_type_name(&vt), display_value(value)
            ));

            // Save non-image value.
            if let Some(path) = save_path {
                let json_str = serde_json::to_string_pretty(&json_value(value))
                    .map_err(|e| format!("failed to serialize value: {}", e))?;
                std::fs::write(path, json_str).map_err(|e| format!("failed to write file: {}", e))?;
                out.push_str(&format!("  saved to {}\n", path.display()));
            }
        }
    }

    Ok(out)
}

/// `mangle show-output <path> --node <id> [--output <n>] [--stats] [--sample <coord>...] [--save <path>]`
///
/// Runs the graph, then inspects the specified node's output(s) with optional
/// image statistics, pixel sampling, and file saving.
async fn cmd_show_output(
    path: PathBuf,
    node: String,
    output_index: Option<usize>,
    stats: bool,
    sample_coords: Vec<String>,
    save_path: Option<PathBuf>,
    json_output: bool,
) -> Result<(), String> {
    let mut graph = load_graph(&path)?;

    // Validate node exists before running.
    if !graph.nodes.contains_key(&node) {
        return Err(format!("node '{}' not found", node));
    }

    // Run the graph to compute output values.
    graph.run().await;
    save_graph(&graph, &path)?;

    let node_data = &graph.nodes[&node];

    // Report node errors.
    if node_data.is_error {
        let msg = node_data.error_message.as_deref().unwrap_or("unknown error");
        if json_output {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "node": node,
                "error": msg,
            })).unwrap());
        } else {
            eprintln!("[{}] ERROR: {}", node, msg);
        }
        return Ok(());
    }

    // Validate output index if specified.
    if let Some(idx) = output_index {
        if idx >= node_data.outputs.len() {
            return Err(format!(
                "output index {} out of range on node '{}' (has {} outputs)",
                idx, node, node_data.outputs.len()
            ));
        }
    }

    // Determine which outputs to show.
    let output_indices: Vec<usize> = match output_index {
        Some(idx) => vec![idx],
        None => (0..node_data.outputs.len()).collect(),
    };

    // Build results for each output.
    let mut json_results = Vec::new();
    let mut human_output = String::new();

    for idx in &output_indices {
        let output = &node_data.outputs[*idx];
        let value = &output.value;

        // Resolve sample coordinates for image outputs.
        let resolved_samples: Vec<(String, u32, u32)> = if let Value::Image { data, .. } = value {
            let (w, h) = data.dimensions();
            sample_coords.iter().map(|s| {
                let (x, y) = resolve_sample_coord(s, w, h)?;
                Ok((s.clone(), x, y))
            }).collect::<Result<Vec<_>, String>>()?
        } else {
            if !sample_coords.is_empty() {
                return Err(format!(
                    "output {} on node '{}' is {} (not an image) — --sample is only valid for image outputs",
                    idx, node, value_type_name(&value.value_type())
                ));
            }
            vec![]
        };

        // Only pass save_path for the first (or only) output to avoid overwriting.
        let save = if output_indices.len() == 1 || *idx == output_indices[0] {
            save_path.as_ref()
        } else {
            None
        };

        if json_output {
            json_results.push(format_show_output_json(
                &node, *idx, &output.name, value, stats, &resolved_samples, save,
            )?);
        } else {
            human_output.push_str(&format_show_output_human(
                &node, *idx, &output.name, value, stats, &resolved_samples, save,
            )?);
        }
    }

    if json_output {
        if json_results.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&json_results[0]).unwrap());
        } else {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "node": node,
                "outputs": json_results.iter().map(|r| r["output"].clone()).collect::<Vec<_>>(),
            })).unwrap());
        }
    } else {
        print!("{}", human_output);
    }

    Ok(())
}

/// `mangle run <path>` — execute the graph and print all output values.
async fn cmd_run(path: PathBuf, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    graph.run().await;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&format_run_json(&graph)).unwrap());
    } else {
        print!("{}", format_run_human(&graph));
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
