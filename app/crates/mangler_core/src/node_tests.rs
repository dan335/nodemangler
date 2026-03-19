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
    assert!(!node.is_busy);
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
