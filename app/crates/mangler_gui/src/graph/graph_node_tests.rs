use super::*;
use mangler_core::input::Input;
use mangler_core::node_settings::NodeSettings;
use mangler_core::output::Output;
use mangler_core::value::Value;

/// Helper to create a minimal GraphNode for testing connection logic.
fn make_test_node(id: &str, num_inputs: usize, num_outputs: usize) -> GraphNode {
    let inputs: Vec<Input> = (0..num_inputs)
        .map(|i| Input::new(format!("in_{}", i), Value::Integer(0), None, None))
        .collect();
    let outputs: Vec<Output> = (0..num_outputs)
        .map(|i| Output::new(format!("out_{}", i), Value::Integer(0), None))
        .collect();
    GraphNode::new(
        id.to_string(),
        Pos2::ZERO,
        NodeSettings::default(),
        inputs,
        outputs,
        false,
        None,
        true,
        None,
    )
}

#[test]
fn test_set_and_clear_input_connection() {
    // Setting an input connection stores the upstream node ID and output index,
    // and clearing it resets the field to None.
    let mut node = make_test_node("node_a", 2, 0);
    assert!(node.inputs[0].connection.is_none());

    node.set_input_connection(0, "upstream".to_string(), 1);
    assert_eq!(
        node.inputs[0].connection,
        Some(("upstream".to_string(), 1))
    );

    node.clear_input_connection(0);
    assert!(node.inputs[0].connection.is_none());
}

#[test]
fn test_set_output_connection_single() {
    // Adding a single downstream connection creates the vec with one entry.
    let mut node = make_test_node("node_a", 0, 2);
    assert!(node.outputs[0].connection.is_none());

    node.set_output_connection(0, "downstream".to_string(), 0);
    assert_eq!(
        node.outputs[0].connection,
        Some(vec![("downstream".to_string(), 0)])
    );
}

#[test]
fn test_set_output_connection_fan_out() {
    // A single output can fan out to multiple downstream inputs.
    let mut node = make_test_node("node_a", 0, 1);

    node.set_output_connection(0, "node_b".to_string(), 0);
    node.set_output_connection(0, "node_c".to_string(), 1);
    assert_eq!(
        node.outputs[0].connection,
        Some(vec![
            ("node_b".to_string(), 0),
            ("node_c".to_string(), 1),
        ])
    );
}

#[test]
fn test_clear_output_connection_removes_specific_entry() {
    // Clearing a specific downstream connection leaves the others intact.
    let mut node = make_test_node("node_a", 0, 1);

    node.set_output_connection(0, "node_b".to_string(), 0);
    node.set_output_connection(0, "node_c".to_string(), 1);
    node.set_output_connection(0, "node_d".to_string(), 2);

    node.clear_output_connection(0, "node_c", 1);
    assert_eq!(
        node.outputs[0].connection,
        Some(vec![
            ("node_b".to_string(), 0),
            ("node_d".to_string(), 2),
        ])
    );
}

#[test]
fn test_clear_output_connection_sets_none_when_last_removed() {
    // Removing the only downstream connection sets the field to None.
    let mut node = make_test_node("node_a", 0, 1);

    node.set_output_connection(0, "node_b".to_string(), 0);
    node.clear_output_connection(0, "node_b", 0);
    assert!(node.outputs[0].connection.is_none());
}

#[test]
fn test_clear_output_connection_no_op_when_not_connected() {
    // Clearing a non-existent connection is a no-op (no panic).
    let mut node = make_test_node("node_a", 0, 1);
    node.clear_output_connection(0, "nonexistent", 0);
    assert!(node.outputs[0].connection.is_none());
}

#[test]
fn test_clear_output_connection_no_op_wrong_input_index() {
    // A matching node ID but wrong input index should not remove anything.
    let mut node = make_test_node("node_a", 0, 1);
    node.set_output_connection(0, "node_b".to_string(), 0);

    node.clear_output_connection(0, "node_b", 999);
    assert_eq!(
        node.outputs[0].connection,
        Some(vec![("node_b".to_string(), 0)])
    );
}

#[test]
fn test_clear_output_connection_out_of_bounds_output_index() {
    // An out-of-bounds output index should not panic.
    let mut node = make_test_node("node_a", 0, 1);
    node.clear_output_connection(5, "node_b", 0);
}
