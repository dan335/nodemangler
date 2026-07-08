//! Tolerant (de)serialization for the `nodes` map on [`crate::GraphSaveData`]
//! (and its borrowing mirror `GraphSaveRef` in `graph.rs::save_to_file`).
//!
//! A plain `#[derive(Deserialize)]` on `HashMap<String, Node>` aborts the
//! *entire* parse the moment one node fails to deserialize — e.g. because its
//! `Operation` variant was added by a version of NodeMangler newer than this
//! build. That is unacceptable for opening a graph a colleague saved with a
//! newer app: the whole file would become unopenable. This module
//! deserializes each node independently and falls back to a verbatim
//! placeholder ([`crate::node::Node::placeholder_from_raw`]) for any node
//! that fails, so the rest of the graph still loads.
//!
//! Serialization mirrors this: normal nodes serialize through their derived
//! `Serialize` impl as usual, but [`crate::node_type::NodeType::Unknown`]
//! nodes write their original JSON back out almost byte-for-byte — only
//! `position` and the live per-socket `connection` fields are patched in, so
//! user edits (moving the node, rewiring it) still take effect while any
//! fields this build doesn't understand round-trip untouched.

use std::collections::HashMap;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serializer};
use crate::node::Node;
use crate::node_type::NodeType;

/// Deserialize a `nodes` map tolerantly: each entry is parsed as raw JSON
/// first, then as a [`Node`]; entries that fail the second step become
/// placeholder nodes (see [`Node::placeholder_from_raw`]) instead of
/// aborting the whole graph load.
pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Node>, D::Error>
where
    D: Deserializer<'de>,
{
    // First pass: parse into raw JSON values. This step can still fail (e.g.
    // the `nodes` field isn't even a JSON object) — that failure is real and
    // has no raw value to fall back to, so it propagates normally.
    let raw_nodes: HashMap<String, serde_json::Value> = HashMap::deserialize(deserializer)?;

    let mut nodes = HashMap::with_capacity(raw_nodes.len());
    for (id, raw) in raw_nodes {
        // `raw.clone()` because a failed `from_value` may partially consume
        // its input; `placeholder_from_raw` needs the pristine original.
        let node = match serde_json::from_value::<Node>(raw.clone()) {
            Ok(node) => node,
            Err(parse_error) => {
                // Malformed JSON (not just "unrecognized shape") is a real
                // corruption bug worth knowing about, even though we still
                // recover via the placeholder — surface it to stderr rather
                // than silently swallowing it.
                eprintln!(
                    "Node '{id}' failed to parse as a known node type (treating as a \
                     placeholder — likely saved by a newer NodeMangler version): {parse_error}"
                );
                Node::placeholder_from_raw(id.clone(), raw)
            }
        };
        nodes.insert(id, node);
    }
    Ok(nodes)
}

/// Serialize a `nodes` map: known node types serialize normally;
/// [`NodeType::Unknown`] nodes write their original JSON back out, patched
/// only where live editing can actually change something (position,
/// connections). See the module doc for why.
pub fn serialize<S>(nodes: &HashMap<String, Node>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(nodes.len()))?;
    for (id, node) in nodes {
        match &node.node_type {
            NodeType::Unknown { raw } => {
                let patched = patch_unknown_node(raw, node)
                    .map_err(|e| serde::ser::Error::custom(format!(
                        "failed to patch unknown node '{id}' for save: {e}"
                    )))?;
                map.serialize_entry(id, &patched)?;
            }
            _ => {
                map.serialize_entry(id, node)?;
            }
        }
    }
    map.end()
}

/// Clone `raw` and patch only the fields live editing can actually change:
/// the node's canvas position, and each socket's `connection` entry (matched
/// by index — sockets that failed to parse into [`crate::input::Input`] /
/// [`crate::output::Output`] during load have no live counterpart and are
/// left untouched). Every other field — including any this build doesn't
/// understand — survives byte-for-byte.
fn patch_unknown_node(raw: &serde_json::Value, node: &Node) -> Result<serde_json::Value, serde_json::Error> {
    let mut patched = raw.clone();

    // `serde_json::Value`'s `IndexMut<&str>` auto-vivifies a missing key as
    // `Null` before assigning, so this works whether or not `raw` already
    // had a "position" key.
    patched["position"] = serde_json::to_value(node.position)?;

    if let Some(inputs_json) = patched.get_mut("inputs").and_then(|v| v.as_array_mut()) {
        for (i, input_json) in inputs_json.iter_mut().enumerate() {
            if let Some(live_input) = node.inputs.get(i) {
                input_json["connection"] = serde_json::to_value(&live_input.connection)?;
            }
        }
    }

    if let Some(outputs_json) = patched.get_mut("outputs").and_then(|v| v.as_array_mut()) {
        for (i, output_json) in outputs_json.iter_mut().enumerate() {
            if let Some(live_output) = node.outputs.get(i) {
                output_json["connection"] = serde_json::to_value(&live_output.connection)?;
            }
        }
    }

    Ok(patched)
}

#[cfg(test)]
#[path = "saved_nodes_tests.rs"]
mod tests;
