use glam::f32::Vec2;
use crate::input::Input;
use crate::node_type::NodeType;
use crate::operations::Operation;
use crate::output::Output;
use crate::value::Value;
use crate::AddNodeType;
use super::*;

// === Helper ===

fn make_operation_node() -> Node {
    Node::new(
        "test-node".to_string(),
        AddNodeType::Operation(Operation::OpLogicBoolNot),
        Vec2::new(10.0, 20.0),
    )
}

fn make_subgraph_node() -> Node {
    Node::new(
        "sub-node".to_string(),
        AddNodeType::Subgraph,
        Vec2::new(0.0, 0.0),
    )
}

/// Build a node with a specific number of inputs and outputs for connection tests.
fn make_node_with_io(id: &str, num_inputs: usize, num_outputs: usize) -> Node {
    let mut node = make_operation_node();
    node.id = id.to_string();
    node.inputs = (0..num_inputs)
        .map(|i| Input::new(format!("in_{i}"), Value::Decimal(0.0), None, None))
        .collect();
    node.outputs = (0..num_outputs)
        .map(|i| Output::new(format!("out_{i}"), Value::Decimal(0.0), None))
        .collect();
    node
}

// === new ===

#[test]
fn test_new_operation_node() {
    let node = make_operation_node();
    assert_eq!(node.id, "test-node");
    assert_eq!(node.settings.name, "not");
    assert!(node.is_dirty);
    assert!(!node.is_error);
    assert!(node.error_message.is_none());
    assert!(node.cached_input_hash.is_none());
    assert!(node.time.is_none());
    assert_eq!(node.position, Vec2::new(10.0, 20.0));
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.outputs.len(), 1);
    assert!(matches!(node.node_type, NodeType::Operation { .. }));
}

#[test]
fn test_new_subgraph_node() {
    let node = make_subgraph_node();
    assert_eq!(node.id, "sub-node");
    assert_eq!(node.settings.name, "subgraph");
    assert!(node.is_dirty);
    assert!(node.inputs.is_empty());
    assert!(node.outputs.is_empty());
    assert!(matches!(node.node_type, NodeType::Subgraph { .. }));
}

// === PartialEq ===

#[test]
fn test_partial_eq_same_id() {
    let a = make_operation_node();
    let mut b = a.clone();
    b.is_dirty = false;
    b.position = Vec2::new(999.0, 999.0);
    assert_eq!(a, b);
}

#[test]
fn test_partial_eq_different_id() {
    let a = make_operation_node();
    let mut b = a.clone();
    b.id = "other-id".to_string();
    assert_ne!(a, b);
}

// === set_input_value ===

#[test]
fn test_set_input_value_sets_value_and_marks_dirty() {
    let mut node = make_operation_node();
    node.is_dirty = false;
    node.set_input_value(0, Value::Bool(true));
    assert!(matches!(node.inputs[0].value, Value::Bool(true)));
    assert!(node.is_dirty);
}

#[test]
fn test_set_input_value_multiple_times() {
    let mut node = make_node_with_io("n", 3, 0);
    node.set_input_value(0, Value::Decimal(1.0));
    node.set_input_value(1, Value::Decimal(2.0));
    node.set_input_value(2, Value::Decimal(3.0));
    assert!(matches!(node.inputs[0].value, Value::Decimal(v) if v == 1.0));
    assert!(matches!(node.inputs[1].value, Value::Decimal(v) if v == 2.0));
    assert!(matches!(node.inputs[2].value, Value::Decimal(v) if v == 3.0));
}

#[test]
fn test_set_input_value_last_index() {
    let mut node = make_node_with_io("n", 5, 0);
    node.set_input_value(4, Value::Decimal(42.0));
    assert!(matches!(node.inputs[4].value, Value::Decimal(v) if v == 42.0));
}

#[test]
#[should_panic(expected = "Invalid input index: 1")]
fn test_set_input_value_out_of_bounds_panics() {
    let mut node = make_operation_node(); // 1 input
    node.set_input_value(1, Value::Bool(false));
}

#[test]
#[should_panic(expected = "Invalid input index: 0")]
fn test_set_input_value_on_empty_inputs_panics() {
    let mut node = make_subgraph_node(); // 0 inputs
    node.set_input_value(0, Value::Bool(false));
}

#[test]
#[should_panic(expected = "Invalid input index: 100")]
fn test_set_input_value_way_out_of_bounds_panics() {
    let mut node = make_operation_node();
    node.set_input_value(100, Value::Bool(false));
}

// === get_input / get_inputs ===

#[test]
fn test_get_input() {
    let node = make_operation_node();
    let input = node.get_input(0);
    assert_eq!(input.name, "input");
}

#[test]
#[should_panic]
fn test_get_input_out_of_bounds_panics() {
    let node = make_operation_node();
    let _ = node.get_input(5);
}

#[test]
fn test_get_inputs() {
    let node = make_node_with_io("n", 3, 0);
    let inputs = node.get_inputs();
    assert_eq!(inputs.len(), 3);
}

// === set_input_connection / clear_input_connection ===

#[test]
fn test_set_input_connection() {
    let mut node = make_node_with_io("n", 2, 0);
    node.set_input_connection(0, "other-node".to_string(), 3);
    assert_eq!(
        node.inputs[0].connection,
        Some(("other-node".to_string(), 3))
    );
    // Other input untouched
    assert!(node.inputs[1].connection.is_none());
}

#[test]
fn test_set_input_connection_overwrite() {
    let mut node = make_node_with_io("n", 1, 0);
    node.set_input_connection(0, "a".to_string(), 0);
    node.set_input_connection(0, "b".to_string(), 1);
    assert_eq!(node.inputs[0].connection, Some(("b".to_string(), 1)));
}

#[test]
fn test_clear_input_connection() {
    let mut node = make_node_with_io("n", 1, 0);
    node.set_input_connection(0, "other".to_string(), 0);
    assert!(node.inputs[0].connection.is_some());
    node.clear_input_connection(0);
    assert!(node.inputs[0].connection.is_none());
}

#[test]
fn test_clear_input_connection_already_none() {
    let mut node = make_node_with_io("n", 1, 0);
    node.clear_input_connection(0);
    assert!(node.inputs[0].connection.is_none());
}

#[test]
#[should_panic]
fn test_set_input_connection_out_of_bounds_panics() {
    let mut node = make_node_with_io("n", 1, 0);
    node.set_input_connection(5, "x".to_string(), 0);
}

#[test]
#[should_panic]
fn test_clear_input_connection_out_of_bounds_panics() {
    let mut node = make_node_with_io("n", 1, 0);
    node.clear_input_connection(5);
}

// === set_output_connection ===

#[test]
fn test_set_output_connection_first() {
    let mut node = make_node_with_io("n", 0, 2);
    node.set_output_connection(0, "target".to_string(), 1);
    let conn = node.outputs[0].connection.as_ref().unwrap();
    assert_eq!(conn.len(), 1);
    assert_eq!(conn[0], ("target".to_string(), 1));
    assert!(node.outputs[1].connection.is_none());
}

#[test]
fn test_set_output_connection_append_multiple() {
    let mut node = make_node_with_io("n", 0, 1);
    node.set_output_connection(0, "a".to_string(), 0);
    node.set_output_connection(0, "b".to_string(), 1);
    node.set_output_connection(0, "c".to_string(), 2);
    let conn = node.outputs[0].connection.as_ref().unwrap();
    assert_eq!(conn.len(), 3);
    assert_eq!(conn[0], ("a".to_string(), 0));
    assert_eq!(conn[1], ("b".to_string(), 1));
    assert_eq!(conn[2], ("c".to_string(), 2));
}

#[test]
fn test_set_output_connection_duplicate_allowed() {
    let mut node = make_node_with_io("n", 0, 1);
    node.set_output_connection(0, "a".to_string(), 0);
    node.set_output_connection(0, "a".to_string(), 0);
    let conn = node.outputs[0].connection.as_ref().unwrap();
    assert_eq!(conn.len(), 2);
}

#[test]
fn test_set_output_connection_different_outputs() {
    let mut node = make_node_with_io("n", 0, 3);
    node.set_output_connection(0, "a".to_string(), 0);
    node.set_output_connection(1, "b".to_string(), 1);
    node.set_output_connection(2, "c".to_string(), 2);
    assert_eq!(node.outputs[0].connection.as_ref().unwrap().len(), 1);
    assert_eq!(node.outputs[1].connection.as_ref().unwrap().len(), 1);
    assert_eq!(node.outputs[2].connection.as_ref().unwrap().len(), 1);
}

#[test]
#[should_panic]
fn test_set_output_connection_out_of_bounds_panics() {
    let mut node = make_node_with_io("n", 0, 1);
    node.set_output_connection(5, "x".to_string(), 0);
}

// === custom_name ===

#[test]
fn test_custom_name_defaults_to_none() {
    let node = make_operation_node();
    assert!(node.custom_name.is_none());
}

#[test]
fn test_custom_name_defaults_to_none_subgraph() {
    let node = make_subgraph_node();
    assert!(node.custom_name.is_none());
}

#[test]
fn test_custom_name_can_be_set() {
    let mut node = make_operation_node();
    node.custom_name = Some("mountains image".to_string());
    assert_eq!(node.custom_name.as_deref(), Some("mountains image"));
}

#[test]
fn test_custom_name_can_be_cleared() {
    let mut node = make_operation_node();
    node.custom_name = Some("test".to_string());
    node.custom_name = None;
    assert!(node.custom_name.is_none());
}

#[test]
fn test_custom_name_serialization_roundtrip_with_name() {
    // Serialize a node with a custom name, then deserialize and verify it persists.
    let mut node = make_operation_node();
    node.custom_name = Some("my special node".to_string());
    let json = serde_json::to_string(&node).unwrap();
    let deserialized: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.custom_name.as_deref(), Some("my special node"));
}

#[test]
fn test_custom_name_serialization_roundtrip_without_name() {
    // Serialize a node without a custom name, then deserialize and verify it stays None.
    let node = make_operation_node();
    let json = serde_json::to_string(&node).unwrap();
    let deserialized: Node = serde_json::from_str(&json).unwrap();
    assert!(deserialized.custom_name.is_none());
}

#[test]
fn test_custom_name_serde_default_compat() {
    // Simulate loading an old save file that has no custom_name field.
    // The #[serde(default)] attribute should make it deserialize as None.
    let node = make_operation_node();
    let mut json: serde_json::Value = serde_json::to_value(&node).unwrap();
    // Remove the custom_name field to simulate an old save format.
    json.as_object_mut().unwrap().remove("custom_name");
    let deserialized: Node = serde_json::from_value(json).unwrap();
    assert!(deserialized.custom_name.is_none());
}

#[test]
fn test_custom_name_does_not_affect_equality() {
    // Node equality is based on ID only, so different custom names should still be equal.
    let mut a = make_operation_node();
    let mut b = a.clone();
    a.custom_name = Some("name A".to_string());
    b.custom_name = Some("name B".to_string());
    assert_eq!(a, b);
}

// === subgraph input forwarding ===

/// `Value::Path` must forward into a subgraph's linked input just like every
/// other value type. Regression test: the subgraph run branch used to explicitly
/// skip `Value::Path`, so after a save→reload the child graph ran with a stale
/// path until the input was manually re-edited.
#[tokio::test]
async fn test_subgraph_forwards_path_input() {
    use std::path::PathBuf;
    use tokio::sync::mpsc;
    use crate::graph::Graph;
    use crate::input::InputLink;
    use crate::{get_id, GraphChangedMessage, NodeChangedMessage};

    // Build a child graph containing a single node with a Path input we can
    // observe. The child node is an (unloaded) subgraph node so that running the
    // child graph is a safe no-op — we only care that the parent forwards its
    // Path into this input before the child runs.
    let (tx_gc, _rx_gc) = mpsc::channel::<GraphChangedMessage>(32);
    let (tx_nc, _rx_nc) = mpsc::channel::<NodeChangedMessage>(32);
    let mut child_graph = Graph::new(get_id(), tx_nc, tx_gc, true).unwrap();

    let mut child_node = make_subgraph_node();
    child_node.id = "child".to_string();
    // A target input starting with a *different* path so the changed-fingerprint
    // gate in the forwarding loop actually fires.
    child_node.inputs = vec![Input::new(
        "target_in".to_string(),
        Value::Path(PathBuf::from("old.png")),
        None,
        None,
    )];
    // The link matches on input id, so pin it to a known value.
    child_node.inputs[0].id = "target_in".to_string();
    child_graph.nodes.insert("child".to_string(), child_node);

    // Parent subgraph node whose single input is a Path linked to the child's
    // target input.
    let mut parent = make_subgraph_node();
    parent.node_type = NodeType::Subgraph {
        path: PathBuf::new(),
        graph: Some(child_graph),
        last_mtime: None,
    };
    parent.inputs = vec![Input::new(
        "path_in".to_string(),
        Value::Path(PathBuf::from("new.png")),
        None,
        Some(InputLink {
            node_id: "child".to_string(),
            input_id: "target_in".to_string(),
        }),
    )];

    parent
        .run(None, None, crate::run_context::RunContext::default())
        .await;

    // The forwarded Path should now be visible on the child graph's node.
    let NodeType::Subgraph { graph: Some(child), .. } = &parent.node_type else {
        panic!("parent lost its subgraph after running");
    };
    let forwarded = &child.nodes.get("child").unwrap().inputs[0].value;
    assert!(
        matches!(forwarded, Value::Path(p) if p == &PathBuf::from("new.png")),
        "expected forwarded Path new.png, got {:?}",
        forwarded,
    );
}
