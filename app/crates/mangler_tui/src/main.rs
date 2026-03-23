//! `mangle` — CLI for the NodeMangler graph engine.
//!
//! Allows AI agents and terminal users to create, inspect, and execute node
//! graphs from the command line. Each command loads a graph JSON file, performs
//! one operation, saves it back, and prints a result.

mod cli;
mod commands;
mod format;
mod helpers;
mod image_stats;
mod value_parse;

use clap::Parser;

use cli::{Cli, Commands};

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

    // Commands that don't require a graph file path.
    match cli.command {
        Commands::ShowOps { group, search, compact } => return commands::cmd_show_ops(group, search, compact, json),
        Commands::ShowTypes { type_name } => return commands::cmd_show_types(type_name, json),
        Commands::ShowValues => return commands::cmd_show_values(json),
        Commands::ShowOp { op_type } => return commands::cmd_show_op(op_type, json),
        _ => {}
    }

    // All remaining commands require a graph file path.
    let path = cli.path.ok_or_else(|| {
        "a graph file path is required before this command (e.g. mangle graph.json <command>)".to_string()
    })?;

    match cli.command {
        Commands::New => commands::cmd_new(path, json),
        Commands::Info { node, compact } => commands::cmd_info(path, node, compact, json),
        Commands::AddNode { op_type, id } => commands::cmd_add_node(path, op_type, id, json).await,
        Commands::RemoveNode { id } => commands::cmd_remove_node(path, id, json).await,
        Commands::Connect { from, to } => commands::cmd_connect(path, from, to, json).await,
        Commands::Disconnect { node, input } => commands::cmd_disconnect(path, node, input, json).await,
        Commands::SetInput { node, input, value } => commands::cmd_set_input(path, node, input, value, json),
        Commands::SetEnabled { node, enabled } => commands::cmd_set_enabled(path, node, enabled, json),
        Commands::Run => commands::cmd_run(path, json).await,
        Commands::ShowOutput { node, output, stats, sample, save } => {
            commands::cmd_show_output(path, node, output, stats, sample, save, json).await
        }
        // Already handled above.
        Commands::ShowOps { .. } | Commands::ShowTypes { .. } | Commands::ShowValues | Commands::ShowOp { .. } => unreachable!(),
    }
}
