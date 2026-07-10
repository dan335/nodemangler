//! Top-level command handlers and graph mutation helpers.

use std::collections::HashMap;
use std::path::PathBuf;

use mangler_core::{
    get_id, graph::Graph, AddNodeType, GraphSaveData,
    value::Value,
};

use crate::format::{
    format_info_human, format_info_json, format_run_human, format_run_json,
    format_show_op_human, format_show_op_json, format_show_ops_compact_human,
    format_show_ops_compact_json, format_show_ops_human, format_show_ops_json,
    format_show_types_human, format_show_types_json, format_show_values_json, show_values_text,
};
use crate::helpers::{
    load_graph, node_not_found_error, parse_slot, resolve_op, save_graph, value_type_enum_name,
    value_type_name, enum_variants,
};
use crate::value_parse::parse_typed_value;

// ── Graph mutation helpers ────────────────────────────────────────────────────

/// Add a node to an in-memory graph. Returns the node ID.
pub(crate) async fn do_add_node(graph: &mut Graph, op_type: &str, id: Option<String>, custom_name: Option<String>) -> Result<String, String> {
    let operation = resolve_op(op_type)?;
    let node_id = id.unwrap_or_else(get_id);
    graph.add_node(node_id.clone(), AddNodeType::Operation(operation), glam::Vec2::ZERO, true, custom_name, Vec::new()).await;
    Ok(node_id)
}

/// Remove a node from an in-memory graph. Returns the removed node ID.
pub(crate) async fn do_remove_node(graph: &mut Graph, id: &str) -> Result<String, String> {
    if !graph.nodes.contains_key(id) {
        return Err(node_not_found_error(&graph, id));
    }
    graph.remove_node(id.to_string()).await;
    Ok(id.to_string())
}

/// Connect two nodes in an in-memory graph. Returns a description string.
/// Validates that both nodes and slot indices exist before connecting.
pub(crate) async fn do_connect(graph: &mut Graph, from: &str, to: &str) -> Result<String, String> {
    let (output_node_id, output_index) = parse_slot(from)?;
    let (input_node_id, input_index) = parse_slot(to)?;

    // Validate source node and output index.
    let src_node = graph.nodes.get(&output_node_id)
        .ok_or_else(|| node_not_found_error(&graph, &output_node_id))?;
    if output_index >= src_node.outputs.len() {
        return Err(format!(
            "output index {} out of range on node '{}' (has {} outputs)",
            output_index, output_node_id, src_node.outputs.len()
        ));
    }

    // Validate destination node and input index.
    let dst_node = graph.nodes.get(&input_node_id)
        .ok_or_else(|| node_not_found_error(&graph, &input_node_id))?;
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
pub(crate) async fn do_disconnect(graph: &mut Graph, node: &str, input: usize) -> Result<String, String> {
    if !graph.nodes.contains_key(node) {
        return Err(node_not_found_error(&graph, node));
    }
    graph.remove_connection(node.to_string(), input).await;
    Ok(format!("disconnected {node}:{input}"))
}

/// Set an input value on a node in an in-memory graph. Returns a description string.
/// Validates node exists, index is in bounds, and provides helpful error messages for
/// enum types when JSON parse fails.
pub(crate) fn do_set_input(graph: &mut Graph, node: &str, index: usize, value: &str) -> Result<String, String> {
    // Validate node exists.
    let n = graph.nodes.get(node)
        .ok_or_else(|| node_not_found_error(&graph, node))?;

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

// ── Top-level commands ───────────────────────────────────────────────────────

/// `mangle new <path>` — create an empty graph file.
///
/// If the path does not end in `.json`, `.mangler.json` is appended automatically.
pub(crate) fn cmd_new(path: PathBuf, json_output: bool) -> Result<(), String> {
    let path = if path.extension().map_or(false, |ext| ext == "json") {
        path
    } else {
        let mut name = path.as_os_str().to_os_string();
        name.push(".mangler.json");
        PathBuf::from(name)
    };
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    let save_data = GraphSaveData {
        version: mangler_core::APP_VERSION.to_string(),
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
pub(crate) fn cmd_info(path: PathBuf, node: Option<String>, compact: bool, json_output: bool) -> Result<(), String> {
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
pub(crate) fn cmd_show_ops(group: Option<String>, search: Option<String>, compact: bool, json_output: bool) -> Result<(), String> {
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
pub(crate) fn cmd_show_types(type_name: Option<String>, json_output: bool) -> Result<(), String> {
    if json_output {
        let val = format_show_types_json(type_name.as_deref())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
    } else {
        print!("{}", format_show_types_human(type_name.as_deref()));
    }
    Ok(())
}

/// `mangle show-values [--json]` — print value format reference.
pub(crate) fn cmd_show_values(json_output: bool) -> Result<(), String> {
    if json_output {
        println!("{}", serde_json::to_string_pretty(&format_show_values_json()).unwrap());
    } else {
        print!("{}", show_values_text());
    }
    Ok(())
}

/// `mangle show-op <type>` — show detailed info for one operation type.
pub(crate) fn cmd_show_op(op_type: String, json_output: bool) -> Result<(), String> {
    if json_output {
        let val = format_show_op_json(&op_type)?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
    } else {
        print!("{}", format_show_op_human(&op_type)?);
    }
    Ok(())
}

/// `mangle add-node <path> --type <type> [--id <id>] [--name <name>]` — add a node to the graph.
pub(crate) async fn cmd_add_node(path: PathBuf, op_type: String, id: Option<String>, custom_name: Option<String>, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let node_id = do_add_node(&mut graph, &op_type, id, custom_name).await?;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"node_id": node_id}));
    } else {
        println!("{node_id}");
    }
    Ok(())
}

/// `mangle remove-node <path> --id <id>` — remove a node and its connections.
pub(crate) async fn cmd_remove_node(path: PathBuf, id: String, json_output: bool) -> Result<(), String> {
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
pub(crate) async fn cmd_connect(path: PathBuf, from: String, to: String, json_output: bool) -> Result<(), String> {
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
pub(crate) async fn cmd_disconnect(path: PathBuf, node: String, input: usize, json_output: bool) -> Result<(), String> {
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
pub(crate) fn cmd_set_input(path: PathBuf, node: String, inputs: Vec<usize>, values: Vec<String>, json_output: bool) -> Result<(), String> {
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

/// `mangle set-name <path> --node <id> --name <name>` — set or clear a custom display name.
///
/// An empty string clears the custom name, reverting to the operation name.
pub(crate) fn cmd_set_name(path: PathBuf, node: String, name: String, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(node_not_found_error(&graph, &node));
    }
    let n = graph.nodes.get_mut(&node).unwrap();
    let custom_name = if name.is_empty() { None } else { Some(name.clone()) };
    n.custom_name = custom_name.clone();
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::json!({"node": node, "name": custom_name}));
    } else {
        if let Some(ref n) = custom_name {
            println!("set name of {node} to \"{n}\"");
        } else {
            println!("cleared name of {node}");
        }
    }
    Ok(())
}

/// `mangle add-subgraph <path> [--id <id>] [--subgraph-file <file>]` — add a subgraph node.
///
/// If `--subgraph-file` is provided, the child graph is loaded immediately and
/// its exposed inputs/outputs surface on the new node.
pub(crate) async fn cmd_add_subgraph(
    path: PathBuf,
    id: Option<String>,
    subgraph_file: Option<PathBuf>,
    json_output: bool,
) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    let node_id = id.unwrap_or_else(get_id);

    graph.add_node(
        node_id.clone(),
        AddNodeType::Subgraph,
        glam::Vec2::ZERO,
        true,
        None,
        Vec::new(),
    ).await;

    if let Some(file) = subgraph_file.as_ref() {
        if !file.exists() {
            return Err(format!("subgraph file not found: {}", file.display()));
        }
        graph.set_subgraph_path(node_id.clone(), file.clone());
    }

    save_graph(&graph, &path)?;

    if json_output {
        let subgraph_file_str = subgraph_file.as_ref().map(|p| p.display().to_string());
        println!("{}", serde_json::json!({
            "node_id": node_id,
            "subgraph_file": subgraph_file_str,
        }));
    } else {
        println!("{node_id}");
        if let Some(file) = subgraph_file.as_ref() {
            println!("loaded subgraph from {}", file.display());
        }
    }
    Ok(())
}

/// `mangle set-subgraph-path <path> --node <id> --subgraph-file <file>` — point a
/// subgraph node at a child `.mangler.json` file and load it.
pub(crate) fn cmd_set_subgraph_path(
    path: PathBuf,
    node: String,
    subgraph_file: PathBuf,
    json_output: bool,
) -> Result<(), String> {
    if !subgraph_file.exists() {
        return Err(format!("subgraph file not found: {}", subgraph_file.display()));
    }

    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(node_not_found_error(&graph, &node));
    }

    // Verify the target is actually a subgraph node before loading.
    let is_subgraph = matches!(
        graph.nodes.get(&node).map(|n| &n.node_type),
        Some(mangler_core::node_type::NodeType::Subgraph { .. })
    );
    if !is_subgraph {
        return Err(format!(
            "node '{}' is not a subgraph node — use `add-subgraph` to create one",
            node
        ));
    }

    graph.set_subgraph_path(node.clone(), subgraph_file.clone());
    save_graph(&graph, &path)?;

    if json_output {
        println!("{}", serde_json::json!({
            "node": node,
            "subgraph_file": subgraph_file.display().to_string(),
        }));
    } else {
        println!("set subgraph path of {node} to {}", subgraph_file.display());
    }
    Ok(())
}

/// `mangle expose-input <path> --node <id> --input <n> [--expose <bool>]` —
/// mark an input as exposed (or un-exposed) for subgraph composition.
pub(crate) fn cmd_expose_input(
    path: PathBuf,
    node: String,
    input: usize,
    expose: bool,
    json_output: bool,
) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(node_not_found_error(&graph, &node));
    }
    let n = graph.nodes.get_mut(&node).unwrap();
    let input_len = n.inputs.len();
    let input_slot = n.inputs.get_mut(input).ok_or_else(|| {
        format!("input index {input} out of bounds on node '{node}' ({input_len} inputs)")
    })?;
    input_slot.is_exposed = expose;
    save_graph(&graph, &path)?;

    if json_output {
        println!("{}", serde_json::json!({
            "node": node, "input": input, "exposed": expose,
        }));
    } else {
        let verb = if expose { "exposed" } else { "un-exposed" };
        println!("{verb} {node}:{input}");
    }
    Ok(())
}

/// `mangle expose-output <path> --node <id> --output <n> [--expose <bool>]` —
/// mark an output as exposed (or un-exposed) for subgraph composition.
pub(crate) fn cmd_expose_output(
    path: PathBuf,
    node: String,
    output: usize,
    expose: bool,
    json_output: bool,
) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(node_not_found_error(&graph, &node));
    }
    let n = graph.nodes.get_mut(&node).unwrap();
    let output_len = n.outputs.len();
    let output_slot = n.outputs.get_mut(output).ok_or_else(|| {
        format!("output index {output} out of bounds on node '{node}' ({output_len} outputs)")
    })?;
    output_slot.is_exposed = expose;
    save_graph(&graph, &path)?;

    if json_output {
        println!("{}", serde_json::json!({
            "node": node, "output": output, "exposed": expose,
        }));
    } else {
        let verb = if expose { "exposed" } else { "un-exposed" };
        println!("{verb} {node} output:{output}");
    }
    Ok(())
}

/// `mangle set-enabled <path> --node <id> --enabled <bool>` — enable or disable a node.
pub(crate) fn cmd_set_enabled(path: PathBuf, node: String, enabled: bool, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    if !graph.nodes.contains_key(&node) {
        return Err(node_not_found_error(&graph, &node));
    }
    let n = graph.nodes.get_mut(&node).unwrap();
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

/// `mangle run <path>` — execute the graph and print all output values.
pub(crate) async fn cmd_run(path: PathBuf, json_output: bool) -> Result<(), String> {
    let mut graph = load_graph(&path)?;
    // Headless run: there's no user to press "save", so force every output node
    // to write regardless of its (default-off) auto-save toggle.
    graph.force_save_outputs = true;
    graph.run().await;
    save_graph(&graph, &path)?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&format_run_json(&graph)).unwrap());
    } else {
        print!("{}", format_run_human(&graph));
    }
    Ok(())
}

/// `mangle show-output <path> --node <id> [--output <n>] [--stats] [--sample <coord>...] [--save <path>]`
///
/// Runs the graph, then inspects the specified node's output(s) with optional
/// image statistics, pixel sampling, and file saving.
pub(crate) async fn cmd_show_output(
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
        return Err(node_not_found_error(&graph, &node));
    }

    // Run the graph to compute output values. Force output nodes to write too,
    // so a headless render emits its files (see cmd_run).
    graph.force_save_outputs = true;
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
                let (x, y) = crate::image_stats::resolve_sample_coord(s, w, h)?;
                Ok((s.clone(), x, y))
            }).collect::<Result<Vec<_>, String>>()?
        } else {
            if !sample_coords.is_empty() {
                return Err(format!(
                    "output {} on node '{}' is {} (not an image) — --sample is only valid for image outputs",
                    idx, node, crate::helpers::value_type_name(&value.value_type())
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
            json_results.push(crate::format::format_show_output_json(
                &node, *idx, &output.name, value, stats, &resolved_samples, save,
            )?);
        } else {
            human_output.push_str(&crate::format::format_show_output_human(
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

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
