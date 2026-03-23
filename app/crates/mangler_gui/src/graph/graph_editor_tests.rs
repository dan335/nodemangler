use super::*;
use egui::{Pos2, Vec2};
use mangler_core::input::Input;
use mangler_core::node_settings::NodeSettings;
use mangler_core::output::Output;
use mangler_core::value::Value;

/// Helper to add a node at a given position with no inputs/outputs.
fn add_node(editor: &mut GraphEditor, id: &str, pos: Pos2) {
    editor.add_node(
        id.to_string(),
        NodeSettings::default(),
        vec![],
        vec![],
        pos,
        false,
        None,
    );
}

/// Helper to add a node with inputs and outputs.
fn add_node_with_io(
    editor: &mut GraphEditor,
    id: &str,
    pos: Pos2,
    num_inputs: usize,
    num_outputs: usize,
) {
    let inputs: Vec<Input> = (0..num_inputs)
        .map(|i| Input::new(format!("in_{}", i), Value::Integer(0), None, None))
        .collect();
    let outputs: Vec<Output> = (0..num_outputs)
        .map(|i| Output::new(format!("out_{}", i), Value::Integer(0), None))
        .collect();
    editor.add_node(
        id.to_string(),
        NodeSettings::default(),
        inputs,
        outputs,
        pos,
        false,
        None,
    );
}

// -- apply_multi_drag tests --

#[test]
fn test_multi_drag_moves_all_selected_nodes() {
    // When multiple nodes are selected and one is dragged, all other
    // selected nodes should move by the same delta.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(200.0, 200.0));
    add_node(&mut editor, "c", Pos2::new(300.0, 300.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());
    editor.selected_node_ids.insert("c".to_string());

    let delta = Vec2::new(10.0, -5.0);
    let moved = editor.apply_multi_drag("a", delta);

    // "a" is the dragged node — it should NOT be moved by apply_multi_drag
    // (it was already moved by the node's own drag logic).
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(100.0, 100.0));

    // "b" and "c" should be moved by the delta.
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(210.0, 195.0));
    assert_eq!(editor.graph_nodes["c"].position, Pos2::new(310.0, 295.0));

    // The returned vec should contain "b" and "c" with their new positions.
    assert_eq!(moved.len(), 2);
    let moved_ids: std::collections::HashSet<String> = moved.iter().map(|(id, _)| id.clone()).collect();
    assert!(moved_ids.contains("b"));
    assert!(moved_ids.contains("c"));
}

#[test]
fn test_multi_drag_skips_dragged_node() {
    // The node that initiated the drag should not be moved a second time.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(50.0, 50.0));

    editor.selected_node_ids.insert("a".to_string());

    let delta = Vec2::new(20.0, 20.0);
    let moved = editor.apply_multi_drag("a", delta);

    // No other nodes to move — position unchanged.
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(50.0, 50.0));
    assert!(moved.is_empty());
}

#[test]
fn test_multi_drag_unselected_nodes_not_moved() {
    // Nodes that are not in the selection should not be affected by multi-drag.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(200.0, 200.0));
    add_node(&mut editor, "c", Pos2::new(300.0, 300.0));

    // Only "a" and "b" are selected; "c" is not.
    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());

    let delta = Vec2::new(15.0, -10.0);
    let moved = editor.apply_multi_drag("a", delta);

    // "b" moved, "c" untouched.
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(215.0, 190.0));
    assert_eq!(editor.graph_nodes["c"].position, Pos2::new(300.0, 300.0));

    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].0, "b");
}

#[test]
fn test_multi_drag_no_selection() {
    // If no nodes are selected, apply_multi_drag should move nothing.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));
    add_node(&mut editor, "b", Pos2::new(200.0, 200.0));

    let delta = Vec2::new(50.0, 50.0);
    let moved = editor.apply_multi_drag("a", delta);

    assert!(moved.is_empty());
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(100.0, 100.0));
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(200.0, 200.0));
}

#[test]
fn test_multi_drag_selected_node_not_in_graph() {
    // If a selected node ID doesn't exist in graph_nodes (e.g. stale selection),
    // it should be silently skipped without panicking.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 100.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("nonexistent".to_string());

    let delta = Vec2::new(10.0, 10.0);
    let moved = editor.apply_multi_drag("a", delta);

    // Only "nonexistent" would be a candidate but it doesn't exist, so nothing moves.
    assert!(moved.is_empty());
    assert_eq!(editor.graph_nodes["a"].position, Pos2::new(100.0, 100.0));
}

#[test]
fn test_multi_drag_accumulates_over_multiple_frames() {
    // Simulates multiple drag frames — delta applied repeatedly should accumulate.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(0.0, 0.0));
    add_node(&mut editor, "b", Pos2::new(100.0, 100.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());

    let delta = Vec2::new(5.0, 5.0);

    // Frame 1
    editor.apply_multi_drag("a", delta);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(105.0, 105.0));

    // Frame 2
    editor.apply_multi_drag("a", delta);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(110.0, 110.0));

    // Frame 3
    editor.apply_multi_drag("a", delta);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(115.0, 115.0));
}

#[test]
fn test_multi_drag_zero_delta() {
    // A zero delta should not change any positions.
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(100.0, 200.0));
    add_node(&mut editor, "b", Pos2::new(300.0, 400.0));

    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());

    let moved = editor.apply_multi_drag("a", Vec2::ZERO);

    // "b" is returned (it was processed) but position is unchanged.
    assert_eq!(moved.len(), 1);
    assert_eq!(editor.graph_nodes["b"].position, Pos2::new(300.0, 400.0));
}

#[test]
fn test_multi_drag_preserves_relative_positions() {
    // After a multi-drag, the relative positions between all selected nodes
    // should be preserved (same offsets as before).
    let mut editor = GraphEditor::new();
    add_node(&mut editor, "a", Pos2::new(10.0, 20.0));
    add_node(&mut editor, "b", Pos2::new(50.0, 80.0));
    add_node(&mut editor, "c", Pos2::new(100.0, 30.0));

    // Relative offsets from "a": b=(40,60), c=(90,10)
    editor.selected_node_ids.insert("a".to_string());
    editor.selected_node_ids.insert("b".to_string());
    editor.selected_node_ids.insert("c".to_string());

    let delta = Vec2::new(25.0, -15.0);

    // Simulate: "a" was already moved by its own drag logic.
    editor.graph_nodes.get_mut("a").unwrap().position += delta;
    // Now apply to others.
    editor.apply_multi_drag("a", delta);

    let a_pos = editor.graph_nodes["a"].position;
    let b_pos = editor.graph_nodes["b"].position;
    let c_pos = editor.graph_nodes["c"].position;

    // Relative offsets should be the same: b-a=(40,60), c-a=(90,10).
    let b_offset = b_pos - a_pos;
    let c_offset = c_pos - a_pos;
    assert!((b_offset.x - 40.0).abs() < 0.001);
    assert!((b_offset.y - 60.0).abs() < 0.001);
    assert!((c_offset.x - 90.0).abs() < 0.001);
    assert!((c_offset.y - 10.0).abs() < 0.001);
}
