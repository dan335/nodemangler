use std::collections::HashMap;

use crate::{get_id, node::Node, node_type::NodeType, operations::Operation, AddNodeType};

use super::{deserialize, serialize};

/// Serialize a fresh add-node to JSON and mutate its `node_type.Operation.operation`
/// to an unrecognized string, simulating a node saved by a newer NodeMangler
/// that introduced an `Operation` variant this build doesn't know about.
fn add_node_json_with_unknown_operation() -> serde_json::Value {
    let node = Node::new(
        get_id(),
        AddNodeType::Operation(Operation::OpNumberMathAdd),
        glam::Vec2::new(10.0, 20.0),
    );
    let mut raw = serde_json::to_value(&node).unwrap();
    raw["node_type"]["Operation"]["operation"] = serde_json::json!("OpFromTheFuture");
    raw
}

// === deserialize: tolerant fallback ===

#[test]
fn unknown_operation_becomes_placeholder_with_parsed_sockets_and_retains_raw() {
    let raw = add_node_json_with_unknown_operation();
    let mut map = serde_json::Map::new();
    map.insert("n1".to_string(), raw.clone());

    let nodes = deserialize(serde_json::Value::Object(map)).expect("deserialize should not fail");

    let node = nodes.get("n1").expect("unknown node should still be present");
    match &node.node_type {
        NodeType::Unknown { raw: stored_raw } => {
            assert_eq!(stored_raw, &raw, "raw JSON must be retained verbatim");
        }
        other => panic!("expected NodeType::Unknown, got {:?}", other),
    }
    assert!(node.is_error, "a placeholder node should start in an error state");
    // The add operation's sockets (a, b, sum) are well-formed JSON — only the
    // operation string failed to parse — so they should still recover.
    assert_eq!(node.inputs.len(), 2, "well-formed inputs should still parse");
    assert_eq!(node.outputs.len(), 1, "well-formed outputs should still parse");
}

#[test]
fn corrupted_input_value_falls_back_to_empty_inputs_but_keeps_intact_outputs() {
    let node = Node::new(
        get_id(),
        AddNodeType::Operation(Operation::OpNumberMathAdd),
        glam::Vec2::ZERO,
    );
    let mut raw = serde_json::to_value(&node).unwrap();
    // Corrupt one input's value with a Value variant tag this build doesn't
    // recognize, so `Vec<Input>` (and therefore the whole `Node`) fails to
    // parse, while `outputs` remains well-formed.
    raw["inputs"][0]["value"] = serde_json::json!({"NotARealValueType": 123});

    let mut map = serde_json::Map::new();
    map.insert("n1".to_string(), raw);

    let nodes = deserialize(serde_json::Value::Object(map)).expect("deserialize should not fail");
    let node = nodes.get("n1").unwrap();

    assert!(matches!(node.node_type, NodeType::Unknown { .. }));
    assert!(
        node.inputs.is_empty(),
        "corrupted inputs should fall back to empty rather than propagate the failure"
    );
    assert_eq!(
        node.outputs.len(),
        1,
        "outputs were well-formed independently of the corrupted input and should still parse"
    );
}

#[test]
fn known_nodes_deserialize_normally_alongside_unknown_ones() {
    let known = Node::new(
        get_id(),
        AddNodeType::Operation(Operation::OpNumberInputDecimal),
        glam::Vec2::ZERO,
    );
    let known_id = known.id.clone();
    let known_raw = serde_json::to_value(&known).unwrap();

    let unknown_raw = add_node_json_with_unknown_operation();

    let mut map = serde_json::Map::new();
    map.insert(known_id.clone(), known_raw);
    map.insert("unknown".to_string(), unknown_raw);

    let nodes = deserialize(serde_json::Value::Object(map)).unwrap();
    assert_eq!(nodes.len(), 2);
    assert!(matches!(
        nodes.get(&known_id).unwrap().node_type,
        NodeType::Operation { .. }
    ));
    assert!(matches!(
        nodes.get("unknown").unwrap().node_type,
        NodeType::Unknown { .. }
    ));
}

// === serialize: known nodes unchanged, unknown nodes patched-verbatim ===

#[test]
fn known_node_serializes_and_round_trips_normally() {
    let mut nodes = HashMap::new();
    let node = Node::new(
        get_id(),
        AddNodeType::Operation(Operation::OpNumberMathAdd),
        glam::Vec2::new(5.0, 6.0),
    );
    let id = node.id.clone();
    nodes.insert(id.clone(), node);

    let serialized = serialize(&nodes, serde_json::value::Serializer).unwrap();

    // A known node's serialized shape should parse with the *plain* Node
    // deserializer (not going through the tolerant fallback at all), proving
    // serialize() didn't wrap it in anything unusual.
    let round_tripped: HashMap<String, Node> = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped.get(&id).unwrap().settings.name, "add");
}

#[test]
fn unknown_node_serialize_patches_only_position_and_connections() {
    // Build a raw JSON for an unrecognized node, including a field this
    // build has never heard of, to prove it survives the round trip.
    let raw = {
        let mut r = add_node_json_with_unknown_operation();
        r["a_field_this_build_has_never_heard_of"] = serde_json::json!("still here");
        r
    };

    let mut map = serde_json::Map::new();
    map.insert("n1".to_string(), raw.clone());
    let mut nodes = deserialize(serde_json::Value::Object(map)).unwrap();

    // Simulate a live edit: move the node and wire up one connection.
    {
        let live = nodes.get_mut("n1").unwrap();
        live.position = glam::Vec2::new(99.0, 100.0);
        live.inputs[0].connection = Some(("other-node".to_string(), 2));
    }

    let serialized = serialize(&nodes, serde_json::value::Serializer).unwrap();
    let out = &serialized["n1"];

    // The live edits are reflected...
    assert_eq!(
        out["position"],
        serde_json::to_value(glam::Vec2::new(99.0, 100.0)).unwrap()
    );
    assert_eq!(out["inputs"][0]["connection"], serde_json::json!(["other-node", 2]));

    // ...but everything else — including the never-parsed field, and the
    // untouched socket's connection — is byte-for-byte identical to the
    // original raw JSON.
    let mut expected = raw.clone();
    expected["position"] = out["position"].clone();
    expected["inputs"][0]["connection"] = out["inputs"][0]["connection"].clone();
    assert_eq!(out, &expected);
    assert_eq!(out["a_field_this_build_has_never_heard_of"], serde_json::json!("still here"));
}
