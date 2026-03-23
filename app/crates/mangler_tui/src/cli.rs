//! CLI struct definitions for the `mangle` command.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Top-level CLI arguments.
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
  mangle graph.json add-node --type images/combine/blend --name \"My Blend\"
  mangle graph.json set-name --node <id> --name \"My Node\"
  mangle graph.json set-input --node <id> --input 0 --value decimal:3.14
  mangle graph.json set-input --node <id> --input 0 --value decimal:1.0 --input 1 --value decimal:2.0
  mangle graph.json set-input --node <id> --input 0 --value color:1.0,0.0,0.0,1.0
  mangle graph.json set-input --node <id> --input 0 --value blendmode:Multiply
  mangle graph.json run                         Execute and print outputs
  mangle graph.json show-output --node <id>      Run and inspect one node's output
  mangle graph.json show-output --node <id> --stats          Image statistics
  mangle graph.json show-output --node <id> --sample 0,0     Pixel at (0,0)
  mangle graph.json show-output --node <id> --save out.png   Save image to file")]
pub(crate) struct Cli {
    /// Path to the graph JSON file (required for most commands, placed before the subcommand)
    pub path: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
    /// Output results as machine-readable JSON instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,
}

/// All available subcommands.
#[derive(Subcommand)]
pub(crate) enum Commands {
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
        /// Custom display name for the node (shown instead of the operation name)
        #[arg(long)]
        name: Option<String>,
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

    /// Set or clear a custom display name for a node
    #[command(override_usage = "mangle <PATH> set-name --node <NODE> --name <NAME>")]
    SetName {
        /// ID of the target node
        #[arg(long)]
        node: String,
        /// Custom display name, or empty string to clear
        #[arg(long)]
        name: String,
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
