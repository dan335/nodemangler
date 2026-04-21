//! Formatting helpers for human-readable and JSON output.

use mangler_core::{
    graph::Graph,
    operations::{operation_list, Operation},
    value::Value,
};

use crate::helpers::{
    accepted_conversions, collect_categories, enum_variants, flatten_ops, node_not_found_error,
    op_variant_name, output_conversions, resolve_enum_type_name, score_op, value_type_enum_name,
    value_type_name,
};
use crate::value_parse::display_value;

// ── Shared show-ops filtering ────────────────────────────────────────────────

/// A scored operation match from filtering/searching the operation list.
struct ScoredOp {
    score: u32,
    path: String,
    op: Operation,
}

/// Result of filtering/scoring the operation list.
struct FilterResult {
    ops: Vec<ScoredOp>,
    search_raw: String,
    group_filter: String,
    has_search: bool,
}

/// Filter and score operations by group prefix and search terms.
/// Returns the matching ops (sorted by score if searching), plus filter metadata.
fn filter_ops(group: Option<&str>, search: Option<&str>) -> FilterResult {
    let all_ops = flatten_ops(&operation_list(), "");
    let group_filter = group.unwrap_or("").to_lowercase().replace(' ', "_");
    let search_raw = search.unwrap_or("").to_string();
    let terms: Vec<String> = search_raw.split_whitespace().map(|t| t.to_lowercase()).collect();
    let has_search = !terms.is_empty();
    let mut scored_ops = Vec::new();

    for (path, op) in &all_ops {
        if !group_filter.is_empty() && !path.to_lowercase().starts_with(&group_filter) {
            continue;
        }

        let variant = op_variant_name(op);
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

        scored_ops.push(ScoredOp { score, path: path.clone(), op: op.clone() });
    }

    if has_search {
        scored_ops.sort_by(|a, b| b.score.cmp(&a.score));
    }

    FilterResult { ops: scored_ops, search_raw, group_filter, has_search }
}

/// Append "no matches" fallback text for human-readable show-ops output.
fn append_no_match_human(out: &mut String, group_filter: &str, has_search: bool, search_raw: &str) {
    if !group_filter.is_empty() && !has_search {
        let all_ops = flatten_ops(&operation_list(), "");
        let cats = collect_categories(&all_ops);
        out.push_str("No operations match that group. Available categories:\n");
        for (name, cnt) in &cats {
            out.push_str(&format!("  {} ({})\n", name, cnt));
        }
    }
    if has_search {
        out.push_str(&format!(
            "No operations match search \"{}\". Try a broader search or use --group to browse categories.\n",
            search_raw,
        ));
    }
}

// ── JSON value helper ─────────────────────────────────────────────────────────

/// Format a `Value` as a `serde_json::Value` for JSON output.
/// Images are represented as metadata objects instead of raw data.
pub(crate) fn json_value(value: &Value) -> serde_json::Value {
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

// ── Graph info formatting ─────────────────────────────────────────────────────

/// Format graph info as human-readable text.
/// If `filter_node` is Some, only show that node. If `compact`, omit descriptions and defaults.
pub(crate) fn format_info_human(graph: &Graph, filter_node: Option<&str>, compact: bool) -> Result<String, String> {
    // Validate filter node exists.
    if let Some(nid) = filter_node {
        if !graph.nodes.contains_key(nid) {
            return Err(node_not_found_error(graph, nid));
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
                op_variant_name(operation)
            }
            mangler_core::node_type::NodeType::Subgraph { path, .. } => {
                format!("subgraph({})", path.display())
            }
        };

        let disabled_tag = if !node.is_enabled { " [DISABLED]" } else { "" };
        // Show custom name as primary label if set, with operation name in parens.
        let display_name = if let Some(ref custom) = node.custom_name {
            format!("\"{}\" ({})", custom, node.settings.name)
        } else {
            node.settings.name.clone()
        };
        out.push_str(&format!("\n  [{}] {}{} ({})\n", node_id, display_name, disabled_tag, type_label));

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

/// Format graph info as a JSON value.
pub(crate) fn format_info_json(graph: &Graph, filter_node: Option<&str>) -> Result<serde_json::Value, String> {
    if let Some(nid) = filter_node {
        if !graph.nodes.contains_key(nid) {
            return Err(node_not_found_error(graph, nid));
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
                op_variant_name(operation)
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
            "custom_name": node.custom_name,
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

// ── Show-ops formatting ───────────────────────────────────────────────────────

/// Format show-ops as human-readable text. Supports `--group` with category fallback and `--search`.
pub(crate) fn format_show_ops_human(group: Option<&str>, search: Option<&str>) -> String {
    let FilterResult { ops: scored_ops, search_raw, group_filter, has_search } = filter_ops(group, search);
    let mut out = String::new();

    for sop in &scored_ops {
        let variant = op_variant_name(&sop.op);
        let inputs = sop.op.create_inputs();
        let outputs = sop.op.create_outputs();

        let in_str: Vec<String> = inputs.iter()
            .map(|i| {
                let vt = i.value.value_type();
                if i.accepts_any_type {
                    format!("{}({}, accepts: any)", i.name, value_type_name(&vt))
                } else {
                    let accepts = accepted_conversions(&vt);
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
                let converts_to = output_conversions(&vt);
                if converts_to.is_empty() {
                    format!("{}({})", o.name, value_type_name(&vt))
                } else {
                    format!("{}({}, converts to: {})", o.name, value_type_name(&vt), converts_to.join(", "))
                }
            })
            .collect();

        let score_suffix = if has_search {
            format!(" (score: {})", sop.score)
        } else {
            String::new()
        };

        out.push_str(&format!(
            "{:<45} ({})  in: [{}]  out: [{}]{}\n",
            sop.path, variant, in_str.join(", "), out_str.join(", "), score_suffix
        ));
    }

    if scored_ops.is_empty() {
        append_no_match_human(&mut out, &group_filter, has_search, &search_raw);
    }

    out
}

/// Format show-ops as a compact one-line-per-op summary (path + description).
pub(crate) fn format_show_ops_compact_human(group: Option<&str>, search: Option<&str>) -> String {
    let FilterResult { ops: scored_ops, search_raw, group_filter, has_search } = filter_ops(group, search);
    let mut out = String::new();

    for sop in &scored_ops {
        let desc = &sop.op.settings().description;
        if desc.is_empty() {
            out.push_str(&format!("{}\n", sop.path));
        } else {
            out.push_str(&format!("{:<45} {}\n", sop.path, desc));
        }
    }

    if scored_ops.is_empty() {
        append_no_match_human(&mut out, &group_filter, has_search, &search_raw);
    }

    out
}

/// Format show-ops compact as JSON: array of {path, description}.
pub(crate) fn format_show_ops_compact_json(group: Option<&str>, search: Option<&str>) -> serde_json::Value {
    let FilterResult { ops: scored_ops, search_raw, has_search, .. } = filter_ops(group, search);

    if scored_ops.is_empty() && has_search {
        return serde_json::json!({
            "matches": 0,
            "message": format!(
                "No operations match search \"{}\". Try a broader search or use --group to browse categories.",
                search_raw,
            ),
        });
    }

    let ops: Vec<serde_json::Value> = scored_ops.iter().map(|sop| {
        serde_json::json!({
            "path": sop.path,
            "description": sop.op.settings().description,
        })
    }).collect();
    serde_json::json!(ops)
}

/// Format show-ops as a JSON value.
pub(crate) fn format_show_ops_json(group: Option<&str>, search: Option<&str>) -> serde_json::Value {
    let FilterResult { ops: scored_ops, search_raw, has_search, .. } = filter_ops(group, search);

    if scored_ops.is_empty() && has_search {
        return serde_json::json!({
            "matches": 0,
            "message": format!(
                "No operations match search \"{}\". Try a broader search or use --group to browse categories.",
                search_raw,
            ),
        });
    }

    let ops: Vec<serde_json::Value> = scored_ops.iter().map(|sop| {
        let variant = op_variant_name(&sop.op);
        let description = &sop.op.settings().description;
        let inputs = sop.op.create_inputs();
        let outputs = sop.op.create_outputs();

        let in_json: Vec<serde_json::Value> = inputs.iter().map(|i| {
            let vt = i.value.value_type();
            let mut obj = serde_json::json!({
                "name": i.name,
                "type": value_type_name(&vt),
            });
            if i.accepts_any_type {
                obj["accepts"] = serde_json::json!("any");
            } else {
                let accepts = accepted_conversions(&vt);
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
            let converts_to = output_conversions(&vt);
            if !converts_to.is_empty() {
                obj["converts_to"] = serde_json::json!(converts_to);
            }
            obj
        }).collect();

        let mut op_json = serde_json::json!({
            "path": sop.path,
            "variant": variant,
            "description": description,
            "inputs": in_json,
            "outputs": out_json,
        });
        if has_search {
            op_json["score"] = serde_json::json!(sop.score);
        }
        op_json
    }).collect();

    serde_json::json!(ops)
}

// ── Show-op formatting ────────────────────────────────────────────────────────

/// Format show-op as human-readable text.
pub(crate) fn format_show_op_human(op_type: &str) -> Result<String, String> {
    let op = crate::helpers::resolve_op(op_type)?;
    let settings = op.settings();
    let inputs = op.create_inputs();
    let outputs = op.create_outputs();
    let variant = op_variant_name(&op);

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
                let accepts = accepted_conversions(&vt);
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
        let converts_to = output_conversions(&vt);
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

/// Format show-op as a JSON value.
pub(crate) fn format_show_op_json(op_type: &str) -> Result<serde_json::Value, String> {
    let op = crate::helpers::resolve_op(op_type)?;
    let settings = op.settings();
    let inputs = op.create_inputs();
    let outputs = op.create_outputs();
    let variant = op_variant_name(&op);

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
                let accepts = accepted_conversions(&vt);
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
        let converts_to = output_conversions(&vt);
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

// ── Show-types formatting ─────────────────────────────────────────────────────

/// Format show-types as human-readable text.
pub(crate) fn format_show_types_human(type_name: Option<&str>) -> String {
    use crate::helpers::ENUM_TYPE_NAMES;
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

/// Format show-types as a JSON value.
pub(crate) fn format_show_types_json(type_name: Option<&str>) -> Result<serde_json::Value, String> {
    use crate::helpers::ENUM_TYPE_NAMES;
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

// ── Show-values formatting ────────────────────────────────────────────────────

/// Return the static text for `mangle show-values`.
pub(crate) fn show_values_text() -> &'static str {
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

/// Format show-values as a JSON value.
#[allow(clippy::approx_constant)]
pub(crate) fn format_show_values_json() -> serde_json::Value {
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

// ── Run formatting ────────────────────────────────────────────────────────────

/// Format run results as human-readable text. Reports node errors.
pub(crate) fn format_run_human(graph: &Graph) -> String {
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

/// Format run results as a JSON value.
pub(crate) fn format_run_json(graph: &Graph) -> serde_json::Value {
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

// ── Show-output formatting ────────────────────────────────────────────────────

/// Format a single output's show-output result as JSON.
pub(crate) fn format_show_output_json(
    node_id: &str,
    output_index: usize,
    output_name: &str,
    value: &Value,
    stats: bool,
    samples: &[(String, u32, u32)],
    save_path: Option<&std::path::PathBuf>,
) -> Result<serde_json::Value, String> {
    use crate::image_stats::{compute_full_image_stats, sample_pixel};
    use crate::helpers::save_image_to_file;

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

            // Compute image statistics if requested (single conversion pass).
            if stats {
                let full_stats = compute_full_image_stats(data);
                let mut stats_obj = serde_json::Map::new();
                for (name, cs) in &full_stats.channels {
                    stats_obj.insert(name.to_string(), serde_json::json!({
                        "min": (cs.min * 1000.0).round() / 1000.0,
                        "max": (cs.max * 1000.0).round() / 1000.0,
                        "mean": (cs.mean * 1000.0).round() / 1000.0,
                        "stddev": (cs.stddev * 1000.0).round() / 1000.0,
                    }));
                }
                obj["stats"] = serde_json::Value::Object(stats_obj);
                obj["has_transparency"] = serde_json::json!(full_stats.has_transparency);
                obj["unique_colors"] = serde_json::json!(full_stats.unique_colors);
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
                crate::helpers::save_value_to_file(value, path)?;
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
pub(crate) fn format_show_output_human(
    node_id: &str,
    output_index: usize,
    _output_name: &str,
    value: &Value,
    stats: bool,
    samples: &[(String, u32, u32)],
    save_path: Option<&std::path::PathBuf>,
) -> Result<String, String> {
    use crate::image_stats::{compute_full_image_stats, sample_pixel};
    use crate::helpers::save_image_to_file;

    let vt = value.value_type();
    let mut out = String::new();

    match value {
        Value::Image { data, .. } => {
            let (w, h) = data.dimensions();
            out.push_str(&format!(
                "[{}] out[{}] ({}) = <image {}x{}>\n",
                node_id, output_index, value_type_name(&vt), w, h
            ));

            // Show stats (single conversion pass).
            if stats {
                let full_stats = compute_full_image_stats(data);
                for (name, cs) in &full_stats.channels {
                    out.push_str(&format!(
                        "  {}: min={:.3} max={:.3} mean={:.3} stddev={:.3}\n",
                        name, cs.min, cs.max, cs.mean, cs.stddev
                    ));
                }
                out.push_str(&format!("  has_transparency: {}\n", full_stats.has_transparency));
                out.push_str(&format!("  unique_colors: {}\n", full_stats.unique_colors));
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
                crate::helpers::save_value_to_file(value, path)?;
                out.push_str(&format!("  saved to {}\n", path.display()));
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
